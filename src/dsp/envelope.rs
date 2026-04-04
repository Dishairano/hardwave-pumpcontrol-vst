use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    pub x: f32, // 0.0..1.0 (phase position)
    pub y: f32, // 0.0..1.0 (1.0 = full volume, 0.0 = silent)
    pub tension: f32, // -1.0..1.0 (0 = linear, + = convex, - = concave)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurveData {
    pub points: Vec<CurvePoint>,
}

impl Default for CurveData {
    fn default() -> Self {
        Self::default_pump()
    }
}

impl CurveData {
    /// Default: classic pump curve (fast drop, slow rise).
    pub fn default_pump() -> Self {
        Self {
            points: vec![
                CurvePoint { x: 0.000, y: 1.0, tension: 0.0 },
                CurvePoint { x: 0.010, y: 0.0, tension: 0.8 },
                CurvePoint { x: 0.050, y: 0.0, tension: 0.0 },
                CurvePoint { x: 0.600, y: 0.85, tension: -0.3 },
                CurvePoint { x: 1.000, y: 1.0, tension: 0.0 },
            ],
        }
    }

    /// Evaluate the curve at a given phase (0.0..1.0). Returns amplitude (0.0..1.0).
    #[inline]
    pub fn evaluate(&self, phase: f32) -> f32 {
        let pts = &self.points;
        if pts.is_empty() {
            return 1.0;
        }
        if pts.len() == 1 {
            return pts[0].y;
        }

        let phase = phase.clamp(0.0, 1.0);

        // Find the segment containing `phase` via binary search.
        let mut lo = 0;
        let mut hi = pts.len() - 1;
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            if pts[mid].x <= phase {
                lo = mid;
            } else {
                hi = mid;
            }
        }

        let p0 = &pts[lo];
        let p1 = &pts[hi];

        let dx = p1.x - p0.x;
        if dx < 1e-8 {
            return p1.y;
        }

        // t = normalized position within this segment (0..1).
        let t = ((phase - p0.x) / dx).clamp(0.0, 1.0);

        // Apply tension to create curved interpolation.
        let t_adj = apply_tension(t, p0.tension);

        // Lerp between y values.
        p0.y + (p1.y - p0.y) * t_adj
    }
}

/// Apply tension to a linear t value.
/// tension > 0 → convex (fast start, slow end)
/// tension < 0 → concave (slow start, fast end)
/// tension == 0 → linear
#[inline]
fn apply_tension(t: f32, tension: f32) -> f32 {
    if tension.abs() < 0.001 {
        return t;
    }
    if tension >= 0.0 {
        t.powf(1.0 + tension * 3.0)
    } else {
        1.0 - (1.0 - t).powf(1.0 + tension.abs() * 3.0)
    }
}
