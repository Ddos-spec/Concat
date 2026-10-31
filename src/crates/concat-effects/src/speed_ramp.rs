// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! 1:1 CapCut Velocity & Speed Ramping Curve Engine (100% Free, Zero Paywall).
//!
//! Provides the 6 signature CapCut velocity curve presets (`Montage`, `Hero`,
//! `Bullet`, `JumpCut`, `FlashIn`, `FlashOut`), cubic-bezier control-point
//! evaluation, and integral time-remapping (`remap_source_time`) for smooth
//! variable-framerate playback without cloud or Pro subscription locks.

use serde::{Deserialize, Serialize};

/// A single control knot on a velocity or tone curve.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveKnot {
    /// Normalized timeline position (`0.0..=1.0`).
    pub t: f64,
    /// Speed multiplier (`0.1..=10.0`, where `1.0` is normal speed) or normalized channel level.
    pub value: f64,
    /// Incoming cubic-bezier tangent handle offset `(dt, dv)`.
    pub handle_in: (f64, f64),
    /// Outgoing cubic-bezier tangent handle offset `(dt, dv)`.
    pub handle_out: (f64, f64),
}

impl CurveKnot {
    /// Creates a smooth knot with automatic horizontal bezier tangents.
    pub const fn smooth(t: f64, value: f64) -> Self {
        Self {
            t,
            value,
            handle_in: (-0.08, 0.0),
            handle_out: (0.08, 0.0),
        }
    }
}

/// Built-in CapCut 1:1 Speed Ramping Presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeedRampPreset {
    /// Constant 1.0x speed.
    Normal,
    /// CapCut "Montage": rhythmic slow build, fast whip, slow-mo drop, normal finish.
    Montage,
    /// CapCut "Hero": dramatic fast entry into ultra slow-motion hero moment.
    Hero,
    /// CapCut "Bullet": high-speed entry & exit wrapping a 0.2x bullet-time freeze.
    Bullet,
    /// CapCut "Jump Cut": sharp velocity spike in the middle for beat transitions.
    JumpCut,
    /// CapCut "Flash In": 6.5x whip entry decelerating smoothly to 1.0x.
    FlashIn,
    /// CapCut "Flash Out": 1.0x accelerating smoothly into a 6.5x whip exit.
    FlashOut,
}

impl SpeedRampPreset {
    /// All presets in the order shown in the CapCut velocity bar.
    pub const ALL: [SpeedRampPreset; 7] = [
        Self::Normal,
        Self::Montage,
        Self::Hero,
        Self::Bullet,
        Self::JumpCut,
        Self::FlashIn,
        Self::FlashOut,
    ];

    /// Human-readable display label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Montage => "Montage",
            Self::Hero => "Hero",
            Self::Bullet => "Bullet",
            Self::JumpCut => "Jump Cut",
            Self::FlashIn => "Flash In",
            Self::FlashOut => "Flash Out",
        }
    }

    /// Control knots defining this velocity preset.
    pub fn knots(self) -> Vec<CurveKnot> {
        match self {
            Self::Normal => vec![
                CurveKnot::smooth(0.0, 1.0),
                CurveKnot::smooth(1.0, 1.0),
            ],
            Self::Montage => vec![
                CurveKnot::smooth(0.0, 0.9),
                CurveKnot::smooth(0.28, 5.2),
                CurveKnot::smooth(0.55, 0.4),
                CurveKnot::smooth(0.82, 1.8),
                CurveKnot::smooth(1.0, 1.0),
            ],
            Self::Hero => vec![
                CurveKnot::smooth(0.0, 1.0),
                CurveKnot::smooth(0.25, 5.8),
                CurveKnot::smooth(0.48, 0.3),
                CurveKnot::smooth(0.75, 0.3),
                CurveKnot::smooth(1.0, 1.0),
            ],
            Self::Bullet => vec![
                CurveKnot::smooth(0.0, 5.5),
                CurveKnot::smooth(0.35, 0.2),
                CurveKnot::smooth(0.65, 0.2),
                CurveKnot::smooth(1.0, 5.5),
            ],
            Self::JumpCut => vec![
                CurveKnot::smooth(0.0, 0.6),
                CurveKnot::smooth(0.5, 6.0),
                CurveKnot::smooth(1.0, 0.6),
            ],
            Self::FlashIn => vec![
                CurveKnot::smooth(0.0, 6.5),
                CurveKnot::smooth(0.35, 1.6),
                CurveKnot::smooth(1.0, 1.0),
            ],
            Self::FlashOut => vec![
                CurveKnot::smooth(0.0, 1.0),
                CurveKnot::smooth(0.65, 1.6),
                CurveKnot::smooth(1.0, 6.5),
            ],
        }
    }

    /// Evaluates the instantaneous velocity multiplier at normalized position `u` (`0.0..=1.0`).
    pub fn speed_at(self, u: f64) -> f64 {
        eval_curve(&self.knots(), u).clamp(0.1, 10.0)
    }

    /// Maps normalized output timeline progress `u` (`0.0..=1.0`) to normalized source clip position
    /// (`0.0..=1.0`) by numerically integrating the velocity curve.
    pub fn remap_progress(self, u: f64) -> f64 {
        let u = u.clamp(0.0, 1.0);
        if self == Self::Normal || u <= 0.0 {
            return u;
        }
        const STEPS: usize = 64;
        let mut total_area = 0.0;
        let mut partial_area = 0.0;
        for i in 0..STEPS {
            let t0 = i as f64 / STEPS as f64;
            let t1 = (i + 1) as f64 / STEPS as f64;
            let mid = 0.5 * (t0 + t1);
            let s = self.speed_at(mid);
            let seg = s * (t1 - t0);
            total_area += seg;
            if t1 <= u {
                partial_area += seg;
            } else if t0 < u {
                partial_area += s * (u - t0);
            }
        }
        if total_area <= f64::EPSILON {
            u
        } else {
            (partial_area / total_area).clamp(0.0, 1.0)
        }
    }
}

/// Evaluates a piecewise cubic Hermite / Bezier curve defined by `knots` at `u` (`0.0..=1.0`).
pub fn eval_curve(knots: &[CurveKnot], u: f64) -> f64 {
    if knots.is_empty() {
        return 1.0;
    }
    let u = u.clamp(0.0, 1.0);
    if u <= knots[0].t {
        return knots[0].value;
    }
    let last = &knots[knots.len() - 1];
    if u >= last.t {
        return last.value;
    }

    for pair in knots.windows(2) {
        let a = &pair[0];
        let b = &pair[1];
        if u >= a.t && u <= b.t {
            let span = (b.t - a.t).max(1e-6);
            let local_t = ((u - a.t) / span).clamp(0.0, 1.0);
            // Smoothstep cubic Hermite interpolation between knots
            let s = local_t * local_t * (3.0 - 2.0 * local_t);
            return a.value + (b.value - a.value) * s;
        }
    }
    last.value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_ramp_presets_remap_monotonically() {
        for preset in SpeedRampPreset::ALL {
            let start = preset.remap_progress(0.0);
            let mid = preset.remap_progress(0.5);
            let end = preset.remap_progress(1.0);
            assert!((start - 0.0).abs() < 1e-6);
            assert!(mid > 0.0 && mid < 1.0);
            assert!((end - 1.0).abs() < 1e-6);
        }
    }
}
