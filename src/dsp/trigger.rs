/// Trigger modes for the volume shaper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    Internal = 0,
    Sidechain = 1,
    Midi = 2,
}

/// Musical sync rates (note divisions).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncRate {
    Sixteenth,
    EighthTriplet,
    Eighth,
    DottedEighth,
    QuarterTriplet,
    Quarter,
    DottedQuarter,
    HalfTriplet,
    Half,
    DottedHalf,
    Whole,
    TwoBars,
    FourBars,
    Free,
}

impl SyncRate {
    /// Returns the number of beats (quarter notes) per cycle.
    pub fn to_beats(self) -> f64 {
        match self {
            Self::Sixteenth => 0.25,
            Self::EighthTriplet => 1.0 / 3.0,
            Self::Eighth => 0.5,
            Self::DottedEighth => 0.75,
            Self::QuarterTriplet => 2.0 / 3.0,
            Self::Quarter => 1.0,
            Self::DottedQuarter => 1.5,
            Self::HalfTriplet => 4.0 / 3.0,
            Self::Half => 2.0,
            Self::DottedHalf => 3.0,
            Self::Whole => 4.0,
            Self::TwoBars => 8.0,
            Self::FourBars => 16.0,
            Self::Free => 1.0, // unused in free mode
        }
    }

    /// Convert an integer param value (0..13) to SyncRate.
    pub fn from_index(i: i32) -> Self {
        match i {
            0 => Self::Sixteenth,
            1 => Self::EighthTriplet,
            2 => Self::Eighth,
            3 => Self::DottedEighth,
            4 => Self::QuarterTriplet,
            5 => Self::Quarter,
            6 => Self::DottedQuarter,
            7 => Self::HalfTriplet,
            8 => Self::Half,
            9 => Self::DottedHalf,
            10 => Self::Whole,
            11 => Self::TwoBars,
            12 => Self::FourBars,
            13 => Self::Free,
            _ => Self::Quarter,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Sixteenth => "1/16",
            Self::EighthTriplet => "1/8T",
            Self::Eighth => "1/8",
            Self::DottedEighth => "1/8D",
            Self::QuarterTriplet => "1/4T",
            Self::Quarter => "1/4",
            Self::DottedQuarter => "1/4D",
            Self::HalfTriplet => "1/2T",
            Self::Half => "1/2",
            Self::DottedHalf => "1/2D",
            Self::Whole => "1/1",
            Self::TwoBars => "2 bars",
            Self::FourBars => "4 bars",
            Self::Free => "Free",
        }
    }
}

pub struct TriggerEngine {
    phase: f64,
    sample_rate: f64,

    // Transport state
    tempo: f64,
    playing: bool,
    pos_beats: f64,

    // Sidechain envelope follower
    sc_env: f32,
    sc_attack_coeff: f32,
    sc_release_coeff: f32,
    sc_was_above: bool,

    // MIDI
    midi_retrigger: bool,
}

impl TriggerEngine {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            phase: 0.0,
            sample_rate: sample_rate as f64,
            tempo: 150.0,
            playing: false,
            pos_beats: 0.0,
            sc_env: 0.0,
            sc_attack_coeff: 0.0,
            sc_release_coeff: 0.0,
            sc_was_above: false,
            midi_retrigger: false,
        }
    }

    pub fn set_sample_rate(&mut self, sr: f32) {
        self.sample_rate = sr as f64;
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.sc_env = 0.0;
        self.sc_was_above = false;
        self.midi_retrigger = false;
    }

    pub fn set_transport(&mut self, tempo: Option<f64>, playing: bool, pos_beats: Option<f64>) {
        self.tempo = tempo.unwrap_or(150.0);
        self.playing = playing;
        if let Some(pb) = pos_beats {
            self.pos_beats = pb;
        }
    }

    pub fn set_sidechain_params(&mut self, attack_ms: f32, release_ms: f32) {
        let sr = self.sample_rate as f32;
        self.sc_attack_coeff = (-1.0 / (attack_ms * 0.001 * sr)).exp();
        self.sc_release_coeff = (-1.0 / (release_ms * 0.001 * sr)).exp();
    }

    pub fn midi_note_on(&mut self) {
        self.midi_retrigger = true;
    }

    /// Advance one sample. Returns current phase (0.0..1.0).
    #[inline]
    pub fn tick(
        &mut self,
        mode: TriggerMode,
        sync_rate: SyncRate,
        rate_hz: f32,
        phase_offset: f32,
        sc_level: f32,
        sc_threshold_lin: f32,
    ) -> f32 {
        // Transport stopped: hold phase at its last position. Matches the
        // convention used by ShaperBox / LFOTool / Kickstart — a paused DAW
        // freezes the playhead in every trigger mode, including Free.
        if !self.playing {
            let out = (self.phase as f32 + phase_offset) % 1.0;
            return out.clamp(0.0, 1.0);
        }

        match mode {
            TriggerMode::Internal => {
                if sync_rate == SyncRate::Free {
                    self.phase += rate_hz as f64 / self.sample_rate;
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                    }
                } else {
                    let beats_per_cycle = sync_rate.to_beats();
                    self.phase = (self.pos_beats % beats_per_cycle) / beats_per_cycle;
                    // Advance pos_beats by 1 sample
                    self.pos_beats += self.tempo / 60.0 / self.sample_rate;
                }
            }
            TriggerMode::Sidechain => {
                // Envelope follower
                let coeff = if sc_level > self.sc_env {
                    self.sc_attack_coeff
                } else {
                    self.sc_release_coeff
                };
                self.sc_env = coeff * self.sc_env + (1.0 - coeff) * sc_level;

                let above = self.sc_env > sc_threshold_lin;
                if above && !self.sc_was_above {
                    // Rising edge: retrigger
                    self.phase = 0.0;
                }
                self.sc_was_above = above;

                // Advance phase
                self.advance_phase(sync_rate, rate_hz);
            }
            TriggerMode::Midi => {
                if self.midi_retrigger {
                    self.phase = 0.0;
                    self.midi_retrigger = false;
                }
                self.advance_phase(sync_rate, rate_hz);
            }
        }

        // Apply phase offset and wrap.
        let out = (self.phase as f32 + phase_offset) % 1.0;
        out.clamp(0.0, 1.0)
    }

    #[inline]
    fn advance_phase(&mut self, sync_rate: SyncRate, rate_hz: f32) {
        if sync_rate == SyncRate::Free {
            self.phase += rate_hz as f64 / self.sample_rate;
        } else {
            let beats_per_cycle = sync_rate.to_beats();
            self.phase += self.tempo / 60.0 / self.sample_rate / beats_per_cycle;
        }
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
    }
}
