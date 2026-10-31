// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! 1:1 CapCut AI Smart Auto-Reframe (16:9 to 9:16 Vertical Speaker Tracking).
//!
//! Computes the probability-weighted subject centroid from `Mask` frames and
//! applies zero-lag forward-backward smoothing so horizontal footage crops
//! smoothly around the active speaker or moving subject without jitter.

use crate::mask::Mask;

/// A normalized crop window inside the source frame (`0.0..=1.0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReframeWindow {
    /// Horizontal center of the crop window in source fractions (`0.0..=1.0`).
    pub center_x: f32,
    /// Vertical center of the crop window in source fractions (`0.0..=1.0`).
    pub center_y: f32,
    /// Width of the crop window as a fraction of source width (`0.0..=1.0`).
    pub width: f32,
    /// Height of the crop window as a fraction of source height (`0.0..=1.0`).
    pub height: f32,
}

/// Computes the probability-weighted center of mass `(cx, cy)` in `0.0..=1.0`
/// for the foreground subject in `mask`. Returns `(0.5, 0.5)` when empty.
pub fn subject_centroid(mask: &Mask) -> (f32, f32) {
    let w = mask.width();
    let h = mask.height();
    if w == 0 || h == 0 {
        return (0.5, 0.5);
    }

    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut total_weight = 0.0_f64;

    for y in 0..h {
        let ny = (y as f64 + 0.5) / (h as f64);
        for x in 0..w {
            let p = mask.get(x, y) as f64 / 255.0;
            if p > 0.25 {
                let nx = (x as f64 + 0.5) / (w as f64);
                sum_x += nx * p;
                sum_y += ny * p;
                total_weight += p;
            }
        }
    }

    if total_weight <= 1e-5 {
        (0.5, 0.5)
    } else {
        (
            (sum_x / total_weight).clamp(0.0, 1.0) as f32,
            (sum_y / total_weight).clamp(0.0, 1.0) as f32,
        )
    }
}

/// Computes a smooth virtual-cameraman crop trajectory converting `source_aspect`
/// (e.g. `16.0 / 9.0`) to `target_aspect` (e.g. `9.0 / 16.0`) across timed masks.
pub fn compute_reframe_trajectory(
    timed_masks: &[(f64, &Mask)],
    source_aspect: f32,
    target_aspect: f32,
    responsiveness: f32,
) -> Vec<(f64, ReframeWindow)> {
    if timed_masks.is_empty() {
        return Vec::new();
    }

    let (crop_w, crop_h) = if target_aspect < source_aspect {
        ((target_aspect / source_aspect).clamp(0.1, 1.0), 1.0_f32)
    } else {
        (1.0_f32, (source_aspect / target_aspect).clamp(0.1, 1.0))
    };

    let half_w = crop_w * 0.5;
    let half_h = crop_h * 0.5;

    let raw_centers: Vec<(f32, f32)> = timed_masks
        .iter()
        .map(|(_, m)| {
            let (cx, cy) = subject_centroid(m);
            (
                cx.clamp(half_w, 1.0 - half_w),
                cy.clamp(half_h, 1.0 - half_h),
            )
        })
        .collect();

    let alpha = responsiveness.clamp(0.02, 0.85);
    let mut forward = raw_centers.clone();
    for i in 1..forward.len() {
        forward[i].0 = forward[i - 1].0 + alpha * (raw_centers[i].0 - forward[i - 1].0);
        forward[i].1 = forward[i - 1].1 + alpha * (raw_centers[i].1 - forward[i - 1].1);
    }

    // Backward pass for zero phase lag
    let mut smoothed = forward;
    for i in (0..smoothed.len().saturating_sub(1)).rev() {
        smoothed[i].0 = smoothed[i + 1].0 + alpha * (smoothed[i].0 - smoothed[i + 1].0);
        smoothed[i].1 = smoothed[i + 1].1 + alpha * (smoothed[i].1 - smoothed[i + 1].1);
    }

    timed_masks
        .iter()
        .zip(smoothed)
        .map(|((t, _), (cx, cy))| {
            (
                *t,
                ReframeWindow {
                    center_x: cx.clamp(half_w, 1.0 - half_w),
                    center_y: cy.clamp(half_h, 1.0 - half_h),
                    width: crop_w,
                    height: crop_h,
                },
            )
        })
        .collect()
}
