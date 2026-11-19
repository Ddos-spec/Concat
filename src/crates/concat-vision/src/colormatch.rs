// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! Colour match: make one clip look lit and graded like another.
//!
//! Reinhard's transfer, on this machine and without a model. Both pictures go
//! to CIE Lab, where lightness and the two colour axes are nearly independent;
//! the clip's mean and spread in each axis are then moved onto the reference's.
//! Working in Lab rather than RGB is what keeps a warm reference from tinting
//! the shadows green: the axes move one at a time.
//!
//! Two steps so the cost of reading the reference is paid once for a whole
//! clip: [`Stats::of`] reads a picture, [`Transfer::between`] relates two
//! readings, and [`Transfer::apply`] regrades any frame of the clip.

use concat_core::frame::{BYTES_PER_PIXEL, Frame};

/// Pixels read along each axis at most when measuring a picture.
const GRID: u32 = 128;

/// The mean and spread of a picture in Lab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    /// Mean of L, a and b.
    pub mean: [f32; 3],
    /// Standard deviation of L, a and b.
    pub spread: [f32; 3],
}

impl Stats {
    /// Measures `frame`. `None` for an empty or non-eight-bit picture.
    pub fn of(frame: &Frame) -> Option<Stats> {
        let (w, h) = (frame.width(), frame.height());
        if w == 0 || h == 0 || frame.bytes_per_pixel() != BYTES_PER_PIXEL {
            return None;
        }
        let (sx, sy) = ((w / GRID).max(1), (h / GRID).max(1));
        let px = frame.pixels();
        let (mut sum, mut sum_sq, mut n) = ([0f64; 3], [0f64; 3], 0f64);
        let mut y = 0;
        while y < h {
            let mut x = 0;
            while x < w {
                let at = (y as usize * w as usize + x as usize) * BYTES_PER_PIXEL;
                let lab = to_lab([px[at], px[at + 1], px[at + 2]]);
                for c in 0..3 {
                    sum[c] += lab[c] as f64;
                    sum_sq[c] += (lab[c] as f64) * (lab[c] as f64);
                }
                n += 1.0;
                x += sx;
            }
            y += sy;
        }
        let mut mean = [0f32; 3];
        let mut spread = [0f32; 3];
        for c in 0..3 {
            let m = sum[c] / n;
            mean[c] = m as f32;
            spread[c] = ((sum_sq[c] / n - m * m).max(0.0)).sqrt() as f32;
        }
        Some(Stats { mean, spread })
    }
}

/// How to move one picture's colours onto another's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transfer {
    gain: [f32; 3],
    offset: [f32; 3],
    /// How much of the match is applied, `0.0..=1.0`.
    strength: f32,
}

impl Transfer {
    /// The grade that takes a picture measured as `source` to look like `reference`.
    pub fn between(source: &Stats, reference: &Stats) -> Transfer {
        let mut gain = [1f32; 3];
        let mut offset = [0f32; 3];
        for c in 0..3 {
            // A flat source has no spread to scale; shift it, never divide by ~0.
            gain[c] = if source.spread[c] > 1e-3 {
                (reference.spread[c] / source.spread[c]).clamp(0.25, 4.0)
            } else {
                1.0
            };
            offset[c] = reference.mean[c] - source.mean[c] * gain[c];
        }
        Transfer {
            gain,
            offset,
            strength: 1.0,
        }
    }

    /// The same grade at a fraction of its strength: the "intensity" slider.
    pub fn with_strength(self, strength: f32) -> Transfer {
        Transfer {
            strength: strength.clamp(0.0, 1.0),
            ..self
        }
    }

    /// Regrades `frame` in place. Alpha is left alone.
    pub fn apply(&self, frame: &mut Frame) {
        if frame.bytes_per_pixel() != BYTES_PER_PIXEL {
            return;
        }
        for px in frame.pixels_mut().chunks_exact_mut(BYTES_PER_PIXEL) {
            let lab = to_lab([px[0], px[1], px[2]]);
            let mut out = [0f32; 3];
            for c in 0..3 {
                let graded = lab[c] * self.gain[c] + self.offset[c];
                out[c] = lab[c] + (graded - lab[c]) * self.strength;
            }
            let rgb = to_rgb(out);
            px[0] = rgb[0];
            px[1] = rgb[1];
            px[2] = rgb[2];
        }
    }
}

const WHITE: [f32; 3] = [0.950_47, 1.0, 1.088_83];

fn srgb_to_linear(v: u8) -> f32 {
    let c = v as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let v = if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (v * 255.0 + 0.5) as u8
}

fn f(t: f32) -> f32 {
    if t > 0.008_856 {
        t.cbrt()
    } else {
        7.787 * t + 16.0 / 116.0
    }
}

fn f_inv(t: f32) -> f32 {
    let cubed = t * t * t;
    if cubed > 0.008_856 {
        cubed
    } else {
        (t - 16.0 / 116.0) / 7.787
    }
}

fn to_lab(rgb: [u8; 3]) -> [f32; 3] {
    let (r, g, b) = (
        srgb_to_linear(rgb[0]),
        srgb_to_linear(rgb[1]),
        srgb_to_linear(rgb[2]),
    );
    let x = 0.412_456_4 * r + 0.357_576_1 * g + 0.180_437_5 * b;
    let y = 0.212_672_9 * r + 0.715_152_2 * g + 0.072_175_0 * b;
    let z = 0.019_333_9 * r + 0.119_192_0 * g + 0.950_304_1 * b;
    let (fx, fy, fz) = (f(x / WHITE[0]), f(y / WHITE[1]), f(z / WHITE[2]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn to_rgb(lab: [f32; 3]) -> [u8; 3] {
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = fy + lab[1] / 500.0;
    let fz = fy - lab[2] / 200.0;
    let (x, y, z) = (
        f_inv(fx) * WHITE[0],
        f_inv(fy) * WHITE[1],
        f_inv(fz) * WHITE[2],
    );
    let r = 3.240_454_2 * x - 1.537_138_5 * y - 0.498_531_4 * z;
    let g = -0.969_266 * x + 1.876_010_8 * y + 0.041_556 * z;
    let b = 0.055_643_4 * x - 0.204_025_9 * y + 1.057_225_2 * z;
    [linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame with a horizontal ramp so there is spread to measure.
    fn ramp(lo: u8, hi: u8, tint: [i16; 3]) -> Frame {
        let (w, h) = (64u32, 32u32);
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..h {
            for x in 0..w {
                let v = lo as i32 + (hi as i32 - lo as i32) * x as i32 / (w as i32 - 1);
                for t in tint {
                    px.push((v + t as i32).clamp(0, 255) as u8);
                }
                px.push(255);
            }
        }
        Frame::from_rgba(w, h, px).expect("sized")
    }

    #[test]
    fn matching_a_picture_to_itself_changes_nothing() {
        let frame = ramp(20, 220, [0, 5, -5]);
        let stats = Stats::of(&frame).unwrap();
        let mut copy = Frame::from_rgba(64, 32, frame.pixels().to_vec()).unwrap();
        Transfer::between(&stats, &stats).apply(&mut copy);
        let worst = frame
            .pixels()
            .iter()
            .zip(copy.pixels())
            .map(|(a, b)| (*a as i16 - *b as i16).abs())
            .max()
            .unwrap();
        assert!(worst <= 1, "round trip drifted by {worst}");
    }

    #[test]
    fn a_dark_clip_takes_on_a_bright_reference() {
        let dark = ramp(5, 90, [0, 0, 0]);
        let bright = ramp(120, 250, [0, 0, 0]);
        let (sd, sb) = (Stats::of(&dark).unwrap(), Stats::of(&bright).unwrap());
        let mut graded = Frame::from_rgba(64, 32, dark.pixels().to_vec()).unwrap();
        Transfer::between(&sd, &sb).apply(&mut graded);
        let after = Stats::of(&graded).unwrap();
        assert!(
            (after.mean[0] - sb.mean[0]).abs() < 3.0,
            "lightness {} vs {}",
            after.mean[0],
            sb.mean[0]
        );
    }

    #[test]
    fn a_warm_reference_warms_a_neutral_clip() {
        let neutral = ramp(40, 200, [0, 0, 0]);
        let warm = ramp(40, 200, [25, 5, -25]);
        let (sn, sw) = (Stats::of(&neutral).unwrap(), Stats::of(&warm).unwrap());
        let mut graded = Frame::from_rgba(64, 32, neutral.pixels().to_vec()).unwrap();
        Transfer::between(&sn, &sw).apply(&mut graded);
        let after = Stats::of(&graded).unwrap();
        assert!(after.mean[2] > sn.mean[2] + 5.0, "b axis did not move warm");
    }

    #[test]
    fn zero_strength_leaves_the_picture_alone() {
        let a = ramp(5, 90, [0, 0, 0]);
        let b = ramp(120, 250, [10, 0, 0]);
        let t = Transfer::between(&Stats::of(&a).unwrap(), &Stats::of(&b).unwrap()).with_strength(0.0);
        let mut c = Frame::from_rgba(64, 32, a.pixels().to_vec()).unwrap();
        t.apply(&mut c);
        let worst = a
            .pixels()
            .iter()
            .zip(c.pixels())
            .map(|(x, y)| (*x as i16 - *y as i16).abs())
            .max()
            .unwrap();
        assert!(worst <= 1);
    }
}
