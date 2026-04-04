//! Built-in curve presets for PumpControl.

use crate::dsp::envelope::{CurveData, CurvePoint};

pub struct Preset {
    pub name: &'static str,
    pub category: &'static str,
    pub curve: CurveData,
}

pub fn all_presets() -> Vec<Preset> {
    vec![
        // ── Hardstyle ──────────────────────────────────────────────────────
        Preset {
            name: "Hardstyle Pump",
            category: "Hardstyle",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.010, y: 0.0, tension: 0.8 },
                    CurvePoint { x: 0.050, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.600, y: 0.85, tension: -0.3 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },
        Preset {
            name: "Reverse Pump",
            category: "Hardstyle",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.400, y: 0.15, tension: 0.3 },
                    CurvePoint { x: 0.950, y: 1.0, tension: -0.8 },
                    CurvePoint { x: 0.990, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 1.000, y: 0.0, tension: 0.0 },
                ],
            },
        },
        Preset {
            name: "Kick Space",
            category: "Hardstyle",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.005, y: 0.0, tension: 0.9 },
                    CurvePoint { x: 0.200, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.400, y: 0.9, tension: -0.5 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },
        Preset {
            name: "Rawstyle Crush",
            category: "Hardstyle",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.008, y: 0.0, tension: 0.95 },
                    CurvePoint { x: 0.030, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.100, y: 0.5, tension: -0.6 },
                    CurvePoint { x: 0.250, y: 0.3, tension: 0.4 },
                    CurvePoint { x: 0.500, y: 0.7, tension: -0.2 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },

        // ── Hardcore ───────────────────────────────────────────────────────
        Preset {
            name: "Hardcore Pump",
            category: "Hardcore",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.005, y: 0.0, tension: 0.9 },
                    CurvePoint { x: 0.020, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.300, y: 0.9, tension: -0.5 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },
        Preset {
            name: "Hardcore Stutter",
            category: "Hardcore",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.010, y: 0.0, tension: 0.5 },
                    CurvePoint { x: 0.200, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.250, y: 1.0, tension: 0.5 },
                    CurvePoint { x: 0.260, y: 0.0, tension: 0.5 },
                    CurvePoint { x: 0.450, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.500, y: 1.0, tension: 0.5 },
                    CurvePoint { x: 0.510, y: 0.0, tension: 0.5 },
                    CurvePoint { x: 0.700, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.750, y: 1.0, tension: -0.3 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },
        Preset {
            name: "Gabber Smash",
            category: "Hardcore",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.003, y: 0.0, tension: 0.95 },
                    CurvePoint { x: 0.010, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.150, y: 1.0, tension: -0.7 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },

        // ── Frenchcore ─────────────────────────────────────────────────────
        Preset {
            name: "Frenchcore Pump",
            category: "Frenchcore",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.006, y: 0.0, tension: 0.9 },
                    CurvePoint { x: 0.025, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.200, y: 0.95, tension: -0.6 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },
        Preset {
            name: "Snare Duck",
            category: "Frenchcore",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.020, y: 0.3, tension: 0.6 },
                    CurvePoint { x: 0.080, y: 0.3, tension: 0.0 },
                    CurvePoint { x: 0.300, y: 0.9, tension: -0.4 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },

        // ── Utility ────────────────────────────────────────────────────────
        Preset {
            name: "Sine LFO",
            category: "Utility",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: -0.6 },
                    CurvePoint { x: 0.250, y: 0.0, tension: -0.6 },
                    CurvePoint { x: 0.500, y: 1.0, tension: -0.6 },
                    CurvePoint { x: 0.750, y: 0.0, tension: -0.6 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },
        Preset {
            name: "Trance Gate",
            category: "Utility",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.240, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.250, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.490, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.500, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.740, y: 1.0, tension: 0.0 },
                    CurvePoint { x: 0.750, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 0.990, y: 0.0, tension: 0.0 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },
        Preset {
            name: "Slow Breathe",
            category: "Utility",
            curve: CurveData {
                points: vec![
                    CurvePoint { x: 0.000, y: 1.0, tension: -0.4 },
                    CurvePoint { x: 0.500, y: 0.3, tension: -0.4 },
                    CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
                ],
            },
        },
    ]
}

pub fn load_preset(name: &str) -> Option<CurveData> {
    all_presets()
        .into_iter()
        .find(|p| p.name == name)
        .map(|p| p.curve)
}
