//! DAW-exposed parameters for Hardwave PumpControl.

use nih_plug::prelude::*;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::dsp::envelope::CurveData;

/// Trigger mode exposed to the DAW.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum TriggerModeParam {
    #[name = "Internal"]
    Internal,
    #[name = "Sidechain"]
    Sidechain,
    #[name = "MIDI"]
    Midi,
}

#[derive(Params)]
pub struct PumpControlParams {
    // ── Global ─────────────────────────────────────────────────────────────
    #[id = "enabled"]
    pub enabled: BoolParam,

    #[id = "input_gain"]
    pub input_gain: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,

    #[id = "mix"]
    pub mix: FloatParam,

    #[id = "depth"]
    pub depth: FloatParam,

    // ── Trigger ────────────────────────────────────────────────────────────
    #[id = "trigger_mode"]
    pub trigger_mode: EnumParam<TriggerModeParam>,

    #[id = "sync_rate"]
    pub sync_rate: IntParam,

    #[id = "rate_hz"]
    pub rate_hz: FloatParam,

    #[id = "phase_offset"]
    pub phase_offset: FloatParam,

    // ── Sidechain ──────────────────────────────────────────────────────────
    #[id = "sc_threshold"]
    pub sc_threshold: FloatParam,

    #[id = "sc_attack"]
    pub sc_attack: FloatParam,

    #[id = "sc_release"]
    pub sc_release: FloatParam,

    // ── Multiband ──────────────────────────────────────────────────────────
    #[id = "multiband"]
    pub multiband: BoolParam,

    #[id = "xover_low"]
    pub xover_low: FloatParam,

    #[id = "xover_high"]
    pub xover_high: FloatParam,

    #[id = "depth_low"]
    pub depth_low: FloatParam,

    #[id = "depth_mid"]
    pub depth_mid: FloatParam,

    #[id = "depth_high"]
    pub depth_high: FloatParam,

    // ── Curve (persisted, not a DAW knob) ──────────────────────────────────
    #[persist = "curve_data"]
    pub curve_data: Arc<Mutex<CurveData>>,
}

impl Default for PumpControlParams {
    fn default() -> Self {
        Self {
            enabled: BoolParam::new("Enabled", true),

            input_gain: FloatParam::new(
                "Input Gain",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit(" dB"),

            output_gain: FloatParam::new(
                "Output Gain",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit(" dB"),

            mix: FloatParam::new("Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            depth: FloatParam::new("Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            trigger_mode: EnumParam::new("Trigger", TriggerModeParam::Internal),

            sync_rate: IntParam::new("Rate", 5, IntRange::Linear { min: 0, max: 13 }),

            rate_hz: FloatParam::new(
                "Rate Hz",
                4.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 20.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" Hz"),

            phase_offset: FloatParam::new(
                "Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            sc_threshold: FloatParam::new(
                "SC Thresh",
                -20.0,
                FloatRange::Linear { min: -60.0, max: 0.0 },
            )
            .with_unit(" dB"),

            sc_attack: FloatParam::new(
                "SC Attack",
                1.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" ms"),

            sc_release: FloatParam::new(
                "SC Release",
                100.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" ms"),

            multiband: BoolParam::new("Multiband", false),

            xover_low: FloatParam::new(
                "Xover Low",
                200.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" Hz"),

            xover_high: FloatParam::new(
                "Xover High",
                5000.0,
                FloatRange::Skewed {
                    min: 1000.0,
                    max: 16000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz"),

            depth_low: FloatParam::new("Low Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            depth_mid: FloatParam::new("Mid Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            depth_high: FloatParam::new("High Depth", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            curve_data: Arc::new(Mutex::new(CurveData::default())),
        }
    }
}
