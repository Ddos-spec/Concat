// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! Where a clip changes shot: the cuts a "split by scene" command splits at.
//!
//! Entirely on this machine and without a model. Each frame is boiled down to
//! a colour histogram, and a cut is where one frame's histogram is far from
//! the one before it. A histogram rather than a pixel difference because a
//! camera move or an actor crossing the frame shifts every pixel and leaves
//! the colours alone, while a real cut changes the colours wholesale.
//!
//! The detector is fed frames one at a time, so the caller decides how they
//! are sampled: every frame for exactness, every few for speed on a long clip.

use concat_core::frame::{BYTES_PER_PIXEL, Frame};

/// Buckets per colour channel.
const BINS: usize = 16;
/// Pixels looked at along each axis at most; a large frame is sampled, not read whole.
const GRID: u32 = 96;

/// One detected cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cut {
    /// Index, in the order frames were pushed, of the first frame of the new shot.
    pub index: usize,
    /// How far the colours moved, `0.0..=1.0`. Higher is a harder cut.
    pub score: f32,
}

/// How eagerly a cut is called.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// The histogram distance, `0.0..=1.0`, past which two neighbours are a cut.
    pub threshold: f32,
    /// The fewest frames a shot may last; a flash or a dropped frame is not a shot.
    pub min_shot: usize,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            threshold: 0.4,
            min_shot: 5,
        }
    }
}

type Histogram = [f32; BINS * 3];

/// Finds cuts in a stream of frames.
pub struct Detector {
    settings: Settings,
    previous: Option<Histogram>,
    index: usize,
    last_cut: Option<usize>,
}

impl Detector {
    /// A detector with nothing seen yet.
    pub fn new(settings: Settings) -> Detector {
        Detector {
            settings,
            previous: None,
            index: 0,
            last_cut: None,
        }
    }

    /// Feeds the next frame. `Some` when it opens a new shot.
    pub fn push(&mut self, frame: &Frame) -> Option<Cut> {
        let here = histogram(frame);
        let index = self.index;
        self.index += 1;
        let previous = self.previous.replace(here)?;
        let score = distance(&previous, &here);
        let far_enough = self
            .last_cut
            .is_none_or(|last| index - last >= self.settings.min_shot);
        if score >= self.settings.threshold && far_enough {
            self.last_cut = Some(index);
            return Some(Cut { index, score });
        }
        None
    }
}

/// Every cut in `frames`, in order.
pub fn detect<'a>(frames: impl IntoIterator<Item = &'a Frame>, settings: Settings) -> Vec<Cut> {
    let mut detector = Detector::new(settings);
    frames
        .into_iter()
        .filter_map(|frame| detector.push(frame))
        .collect()
}

fn histogram(frame: &Frame) -> Histogram {
    // Each pixel is shared between the two nearest buckets by distance, so a
    // slow fade moves the histogram a little at a time instead of jumping a
    // whole bucket when one channel crosses a boundary.
    let mut counts = [0f32; BINS * 3];
    let (w, h) = (frame.width(), frame.height());
    if w == 0 || h == 0 || frame.bytes_per_pixel() != BYTES_PER_PIXEL {
        return [0.0; BINS * 3];
    }
    let (step_x, step_y) = ((w / GRID).max(1), (h / GRID).max(1));
    let pixels = frame.pixels();
    let mut taken = 0u32;
    let mut y = 0;
    while y < h {
        let mut x = 0;
        while x < w {
            let at = (y as usize * w as usize + x as usize) * BYTES_PER_PIXEL;
            for channel in 0..3 {
                let place = (pixels[at + channel] as f32 + 0.5) * BINS as f32 / 256.0 - 0.5;
                let low = place.floor().max(0.0);
                let weight_high = (place - low).clamp(0.0, 1.0);
                let low = low as usize;
                let high = (low + 1).min(BINS - 1);
                counts[channel * BINS + low] += 1.0 - weight_high;
                counts[channel * BINS + high] += weight_high;
            }
            taken += 1;
            x += step_x;
        }
        y += step_y;
    }
    let mut out = [0.0f32; BINS * 3];
    if taken > 0 {
        for (slot, count) in out.iter_mut().zip(counts) {
            *slot = count / taken as f32;
        }
    }
    out
}

/// Half the L1 distance, averaged over the three channels: `0` for the same
/// colours, `1` for none in common.
fn distance(a: &Histogram, b: &Histogram) -> f32 {
    let total: f32 = a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum();
    total / 2.0 / 3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(rgb: [u8; 3]) -> Frame {
        let mut frame = Frame::black(64, 36);
        frame.fill([rgb[0], rgb[1], rgb[2], 255]);
        frame
    }

    #[test]
    fn a_hard_change_of_colours_is_one_cut() {
        let mut frames: Vec<Frame> = (0..8).map(|_| solid([10, 10, 10])).collect();
        frames.extend((0..8).map(|_| solid([240, 230, 220])));
        let cuts = detect(&frames, Settings::default());
        assert_eq!(cuts.len(), 1);
        assert_eq!(cuts[0].index, 8);
        assert!(cuts[0].score > 0.9);
    }

    #[test]
    fn a_still_shot_has_no_cuts() {
        let frames: Vec<Frame> = (0..20).map(|_| solid([90, 120, 60])).collect();
        assert!(detect(&frames, Settings::default()).is_empty());
    }

    #[test]
    fn a_slow_fade_is_not_a_cut() {
        let frames: Vec<Frame> = (0..40u8)
            .map(|i| solid([i * 4, i * 4, i * 4]))
            .collect();
        assert!(detect(&frames, Settings::default()).is_empty());
    }

    #[test]
    fn a_one_frame_flash_is_not_a_shot() {
        let mut frames: Vec<Frame> = (0..6).map(|_| solid([20, 20, 20])).collect();
        frames.push(solid([255, 255, 255]));
        frames.extend((0..6).map(|_| solid([20, 20, 20])));
        let cuts = detect(&frames, Settings::default());
        // The flash opens a "shot" at 6 and the return at 7 is inside its minimum.
        assert_eq!(cuts.iter().map(|c| c.index).collect::<Vec<_>>(), vec![6]);
    }

    #[test]
    fn equal_brightness_but_different_hue_is_still_a_cut() {
        let mut frames: Vec<Frame> = (0..6).map(|_| solid([200, 0, 0])).collect();
        frames.extend((0..6).map(|_| solid([0, 200, 0])));
        assert_eq!(detect(&frames, Settings::default()).len(), 1);
    }
}
