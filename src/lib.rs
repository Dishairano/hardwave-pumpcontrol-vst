//! Hardwave PumpControl — sidechain / volume shaper VST3/CLAP plugin.
//!
//! Signal chain:
//!   Input Gain → Trigger Engine (phase 0..1) → Curve Evaluate → Gain
//!   Optional multiband: 3-band crossover split, per-band depth control.
//!   → Dry/Wet Mix → Output Gain → Output

use crossbeam_channel::{Sender, Receiver};
use nih_plug::prelude::*;
use parking_lot::Mutex;
use std::num::NonZeroU32;
use std::sync::Arc;

mod auth;
mod dsp;
mod editor;
mod params;
mod presets;
mod protocol;

use dsp::{Crossover3Band, PeakMeter, TriggerEngine};
use dsp::trigger::{SyncRate, TriggerMode};
use params::{PumpControlParams, TriggerModeParam};
use protocol::PumpPacket;

struct HardwavePumpControl {
    params: Arc<PumpControlParams>,

    trigger: TriggerEngine,
    crossover: Crossover3Band,
    input_meter: PeakMeter,
    output_meter: PeakMeter,

    // Editor communication.
    editor_packet_tx: Sender<PumpPacket>,
    editor_packet_rx: Arc<Mutex<Receiver<PumpPacket>>>,
    update_counter: u32,

    // Cache for metering sent to UI.
    last_phase: f32,
    last_gain: f32,

    sample_rate: f32,
}

impl Default for HardwavePumpControl {
    fn default() -> Self {
        let sr = 44100.0;
        let (pkt_tx, pkt_rx) = crossbeam_channel::bounded(4);
        Self {
            params: Arc::new(PumpControlParams::default()),
            trigger: TriggerEngine::new(sr),
            crossover: Crossover3Band::new(sr),
            input_meter: PeakMeter::new(sr),
            output_meter: PeakMeter::new(sr),
            editor_packet_tx: pkt_tx,
            editor_packet_rx: Arc::new(Mutex::new(pkt_rx)),
            update_counter: 0,
            last_phase: 0.0,
            last_gain: 1.0,
            sample_rate: sr,
        }
    }
}

impl Plugin for HardwavePumpControl {
    const NAME: &'static str = "Hardwave PumpControl";
    const VENDOR: &'static str = "Hardwave Studios";
    const URL: &'static str = "https://hardwavestudios.com";
    const EMAIL: &'static str = "hello@hardwavestudios.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[new_nonzero_u32(2)], // Sidechain input
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let token = auth::load_token();
        Some(Box::new(editor::PumpEditor::new(
            Arc::clone(&self.params),
            Arc::clone(&self.editor_packet_rx),
            token,
        )))
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let sr = buffer_config.sample_rate;
        self.sample_rate = sr;
        self.trigger.set_sample_rate(sr);
        self.crossover.set_sample_rate(sr);
        self.input_meter.set_sample_rate(sr);
        self.output_meter.set_sample_rate(sr);
        true
    }

    fn reset(&mut self) {
        self.trigger.reset();
        self.crossover.reset();
        self.input_meter.reset();
        self.output_meter.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // Read param values.
        let p = &self.params;
        let enabled = p.enabled.value();
        let input_gain_db = p.input_gain.value();
        let output_gain_db = p.output_gain.value();
        let mix = p.mix.value();
        let depth = p.depth.value();

        let trigger_mode = match p.trigger_mode.value() {
            TriggerModeParam::Internal => TriggerMode::Internal,
            TriggerModeParam::Sidechain => TriggerMode::Sidechain,
            TriggerModeParam::Midi => TriggerMode::Midi,
        };
        let sync_rate = SyncRate::from_index(p.sync_rate.value());
        let rate_hz = p.rate_hz.value();
        let phase_offset = p.phase_offset.value();

        let sc_threshold_db = p.sc_threshold.value();
        let sc_attack = p.sc_attack.value();
        let sc_release = p.sc_release.value();

        let multiband = p.multiband.value();
        let xover_low = p.xover_low.value();
        let xover_high = p.xover_high.value();
        let depth_low = p.depth_low.value();
        let depth_mid = p.depth_mid.value();
        let depth_high = p.depth_high.value();

        // Snapshot for editor.
        let pkt_snapshot = editor::snapshot_params(p);
        let _ = p;

        let input_gain = db_to_linear(input_gain_db);
        let output_gain = db_to_linear(output_gain_db);
        let sc_threshold_lin = db_to_linear(sc_threshold_db);

        // Update sidechain envelope params.
        self.trigger.set_sidechain_params(sc_attack, sc_release);

        // Update crossover freqs.
        self.crossover.set_freqs(xover_low, xover_high);

        // Read transport.
        let transport = context.transport();
        self.trigger.set_transport(
            transport.tempo,
            transport.playing,
            transport.pos_beats(),
        );

        // Process MIDI events (note-on triggers).
        while let Some(event) = context.next_event() {
            if let NoteEvent::NoteOn { .. } = event {
                self.trigger.midi_note_on();
            }
        }

        // Take a snapshot of the curve (avoid holding the lock per-sample).
        let curve = self.params.curve_data.lock().clone();

        // Get sidechain aux input slices (if available).
        let sc_slices: Option<(&[f32], &[f32])> = if !aux.inputs.is_empty()
            && aux.inputs[0].channels() >= 2
        {
            let slices = aux.inputs[0].as_slice_immutable();
            Some((slices[0], slices[1]))
        } else {
            None
        };

        for (sample_idx, mut frame) in buffer.iter_samples().enumerate() {
            let num_channels = frame.len();
            if num_channels < 2 {
                continue;
            }

            let dry_l = *frame.get_mut(0).unwrap();
            let dry_r = *frame.get_mut(1).unwrap();

            // Input gain.
            let mut l = dry_l * input_gain;
            let mut r = dry_r * input_gain;

            // Input metering.
            self.input_meter.process(l, r);

            if enabled {
                // Get sidechain level for envelope follower.
                let sc_level = if let Some((sc_l_buf, sc_r_buf)) = sc_slices {
                    let sc_l = sc_l_buf.get(sample_idx).copied().unwrap_or(0.0);
                    let sc_r = sc_r_buf.get(sample_idx).copied().unwrap_or(0.0);
                    sc_l.abs().max(sc_r.abs())
                } else {
                    0.0
                };

                // Advance trigger, get phase.
                let phase = self.trigger.tick(
                    trigger_mode,
                    sync_rate,
                    rate_hz,
                    phase_offset,
                    sc_level,
                    sc_threshold_lin,
                );

                // Evaluate curve at current phase.
                let curve_gain = curve.evaluate(phase);

                if multiband {
                    // Split into 3 bands.
                    let (low, mid, high) = self.crossover.process(l, r);

                    // Apply per-band depth-scaled gain.
                    let gain_low = 1.0 - depth_low * depth * (1.0 - curve_gain);
                    let gain_mid = 1.0 - depth_mid * depth * (1.0 - curve_gain);
                    let gain_high = 1.0 - depth_high * depth * (1.0 - curve_gain);

                    l = low.0 * gain_low + mid.0 * gain_mid + high.0 * gain_high;
                    r = low.1 * gain_low + mid.1 * gain_mid + high.1 * gain_high;
                } else {
                    // Single-band: apply global depth.
                    let gain = 1.0 - depth * (1.0 - curve_gain);
                    l *= gain;
                    r *= gain;
                }

                self.last_phase = phase;
                self.last_gain = curve_gain;

                // Dry/wet mix.
                l = dry_l * input_gain * (1.0 - mix) + l * mix;
                r = dry_r * input_gain * (1.0 - mix) + r * mix;
            }

            // Output gain.
            l *= output_gain;
            r *= output_gain;

            // Output metering.
            self.output_meter.process(l, r);

            // Write output.
            *frame.get_mut(0).unwrap() = l;
            *frame.get_mut(1).unwrap() = r;
        }

        // Send state packet to editor (~60 fps).
        self.update_counter += 1;
        if self.update_counter >= 4 {
            self.update_counter = 0;

            let mut packet = pkt_snapshot;
            let (il, ir) = self.input_meter.peak_db();
            let (ol, or) = self.output_meter.peak_db();
            packet.input_peak_l = il;
            packet.input_peak_r = ir;
            packet.output_peak_l = ol;
            packet.output_peak_r = or;
            packet.current_phase = self.last_phase;
            packet.current_gain = self.last_gain;

            let _ = self.editor_packet_tx.try_send(packet);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for HardwavePumpControl {
    const CLAP_ID: &'static str = "com.hardwavestudios.pumpcontrol";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Drawable volume shaper with tempo sync and multiband processing");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = Some("https://hardwavestudios.com/support");
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Utility,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for HardwavePumpControl {
    const VST3_CLASS_ID: [u8; 16] = *b"HWPumpCtrl_v001\0";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Tools,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(HardwavePumpControl);
nih_export_vst3!(HardwavePumpControl);

#[inline(always)]
fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}
