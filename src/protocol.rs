//! Rust <-> JS packet definitions for the PumpControl webview UI.

use serde::{Deserialize, Serialize};

use crate::dsp::envelope::CurvePoint;

/// Full state packet pushed to the webview at ~60 fps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpPacket {
    // ── Global ──────────────────────────────────────────────────────────────
    pub enabled: bool,
    pub input_gain: f32,
    pub output_gain: f32,
    pub mix: f32,
    pub depth: f32,

    // ── Trigger ─────────────────────────────────────────────────────────────
    pub trigger_mode: i32,
    pub sync_rate: i32,
    pub rate_hz: f32,
    pub phase_offset: f32,

    // ── Sidechain ───────────────────────────────────────────────────────────
    pub sc_threshold: f32,
    pub sc_attack: f32,
    pub sc_release: f32,

    // ── Multiband ───────────────────────────────────────────────────────────
    pub multiband: bool,
    pub xover_low: f32,
    pub xover_high: f32,
    pub depth_low: f32,
    pub depth_mid: f32,
    pub depth_high: f32,

    // ── Curve ───────────────────────────────────────────────────────────────
    pub curve_points: Vec<CurvePoint>,

    // ── Metering (read-only, pushed from DSP) ───────────────────────────────
    pub input_peak_l: f32,
    pub input_peak_r: f32,
    pub output_peak_l: f32,
    pub output_peak_r: f32,
    pub current_phase: f32,
    pub current_gain: f32,
}

/// JS -> Rust messages from the webview.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum UiMessage {
    #[serde(rename = "set_param")]
    SetParam { id: String, value: f64 },

    #[serde(rename = "set_curve")]
    SetCurve { points: Vec<CurvePoint> },

    #[serde(rename = "load_preset")]
    LoadPreset { name: String },

    #[serde(rename = "release_focus")]
    ReleaseFocus,
}
