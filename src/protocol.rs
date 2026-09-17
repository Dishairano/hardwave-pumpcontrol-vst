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
///
/// This is the whole protocol. `editor.rs` deserializes into it, so a message
/// the webview sends that is not listed here fails to parse instead of being
/// silently ignored, and a field renamed on one side breaks the build on the
/// other. It used to be matched by hand on a `serde_json::Value`, which meant
/// three of these variants were missing and nobody noticed.
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

    #[serde(rename = "resize")]
    Resize { width: u32, height: u32 },

    #[serde(rename = "save_token")]
    SaveToken { token: String },

    #[serde(rename = "clear_token")]
    ClearToken,
}

#[cfg(test)]
mod tests {
    use super::UiMessage;

    /// Exactly the payloads the shipped webview sends, taken from the served
    /// index.html. A rename on either side fails here instead of in a DAW.
    #[test]
    fn parses_every_message_the_ui_sends() {
        let cases = [
            r#"{"type":"set_param","id":"depth","value":0.75}"#,
            r#"{"type":"set_curve","points":[{"x":0,"y":1,"tension":0},{"x":0.6,"y":0.85,"tension":-0.3}]}"#,
            r#"{"type":"load_preset","name":"Hardstyle"}"#,
            r#"{"type":"save_token","token":"pending"}"#,
            r#"{"type":"clear_token"}"#,
            r#"{"type":"release_focus"}"#,
            r#"{"type":"resize","width":900,"height":600}"#,
        ];
        for raw in cases {
            serde_json::from_str::<UiMessage>(raw).unwrap_or_else(|e| panic!("{raw}: {e}"));
        }
    }

    #[test]
    fn refuses_what_it_does_not_know() {
        assert!(serde_json::from_str::<UiMessage>(r#"{"type":"set_parm","id":"a","value":1}"#).is_err());
        assert!(serde_json::from_str::<UiMessage>(r#"{"type":"set_param","id":"a"}"#).is_err());
    }
}
