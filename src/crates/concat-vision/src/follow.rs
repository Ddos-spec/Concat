// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! Motion tracking: follow a point on a moving subject so a title, a sticker
//! or a blur can ride along with it.
//!
//! The same pyramidal Lucas-Kanade flow that stabilisation measures camera
//! motion with, pointed at one spot instead of a field of corners. No model
//! and no network: it needs only that the spot has some texture to hold on to.
//! When the flow loses the point (it leaves the frame, is covered, or sits on
//! a flat patch) the follower says so rather than guessing, and the caller
//! decides whether to stop the track there or to start a new one.

use concat_core::frame::Frame;

use crate::stabilize::track::{Gray, Pyramid};

/// Follows one point from frame to frame.
pub struct Follower {
    last: Pyramid,
    at: (f32, f32),
}

impl Follower {
    /// Starts following `at` (full-size pixels) on `first`.
    pub fn start(first: &Frame, at: (f32, f32)) -> Follower {
        Follower {
            last: Pyramid::of(Gray::of_frame(first)),
            at,
        }
    }

    /// Where the point is now.
    pub fn position(&self) -> (f32, f32) {
        self.at
    }

    /// Advances to `next`. `None` when the point was lost; the follower keeps
    /// its last position and the frame it was on, so a later frame may still
    /// be tried.
    pub fn advance(&mut self, next: &Frame) -> Option<(f32, f32)> {
        let pyramid = Pyramid::of(Gray::of_frame(next));
        let moved = self.last.flow(&pyramid, self.at)?;
        let base = pyramid.base();
        let inside = moved.0 >= 0.0
            && moved.1 >= 0.0
            && moved.0 <= (base.width - 1) as f32
            && moved.1 <= (base.height - 1) as f32;
        if !inside {
            return None;
        }
        self.last = pyramid;
        self.at = moved;
        Some(moved)
    }
}

/// The path of `start` across `frames`, the first of which it starts on. One
/// entry per frame; `None` from the frame the point was lost onward.
pub fn follow(frames: &[Frame], start: (f32, f32)) -> Vec<Option<(f32, f32)>> {
    let Some(first) = frames.first() else {
        return Vec::new();
    };
    let mut follower = Follower::start(first, start);
    let mut path = vec![Some(start)];
    let mut lost = false;
    for frame in &frames[1..] {
        if lost {
            path.push(None);
            continue;
        }
        match follower.advance(frame) {
            Some(p) => path.push(Some(p)),
            None => {
                lost = true;
                path.push(None);
            }
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A smooth, textured picture shifted by `(dx, dy)`.
    fn scene(dx: f32, dy: f32) -> Frame {
        let (w, h) = (160u32, 120u32);
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let (u, v) = (x as f32 - dx, y as f32 - dy);
                let value = 128.0
                    + 50.0 * (u * 0.21).sin() * (v * 0.17).cos()
                    + 40.0 * (u * 0.07 + v * 0.11).sin();
                let g = value.clamp(0.0, 255.0) as u8;
                px.extend_from_slice(&[g, g, g, 255]);
            }
        }
        Frame::from_rgba(w, h, px).expect("sized")
    }

    #[test]
    fn a_point_rides_a_steady_drift() {
        let frames: Vec<Frame> = (0..6).map(|i| scene(i as f32 * 2.0, i as f32 * 1.0)).collect();
        let path = follow(&frames, (80.0, 60.0));
        let last = path[5].expect("kept hold");
        assert!((last.0 - 90.0).abs() < 1.0, "x {}", last.0);
        assert!((last.1 - 65.0).abs() < 1.0, "y {}", last.1);
    }

    #[test]
    fn a_point_that_leaves_the_frame_is_reported_lost() {
        let frames: Vec<Frame> = (0..12).map(|i| scene(i as f32 * 8.0, 0.0)).collect();
        let path = follow(&frames, (140.0, 60.0));
        assert!(path.iter().any(|p| p.is_none()), "never reported lost");
        let first_lost = path.iter().position(|p| p.is_none()).unwrap();
        assert!(path[first_lost..].iter().all(|p| p.is_none()));
    }

    #[test]
    fn no_frames_no_path() {
        assert!(follow(&[], (0.0, 0.0)).is_empty());
    }
}
