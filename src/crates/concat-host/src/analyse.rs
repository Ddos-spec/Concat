// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! The questions an editor asks of a clip's pictures, answered on this machine:
//! where does the shot change, how is it graded, where did that thing go.
//!
//! Each reads the media small (the answers do not need full size and a long
//! clip is read in a fraction of its running time) and hands the frames to the
//! matching piece of `concat-vision`. None of them needs a model or a network.

use concat_core::frame::Frame;
use concat_core::time::Rational;
use concat_media::{DecodeOptions, Decoder, FrameSource};
use concat_vision::colormatch::{Stats, Transfer};
use concat_vision::follow::Follower;
use concat_vision::scene::{Detector, Settings};

/// The longest side the analysis reads a picture at.
const SIDE: u32 = 256;

fn open(path: &str, from: f64) -> Result<Decoder, String> {
    let probe = Decoder::open(path, &DecodeOptions::default().limited_to(1))
        .map_err(|error| format!("{path}: {error}"))?;
    let (w, h) = (probe.width().max(1), probe.height().max(1));
    let scale = (SIDE as f64 / w.max(h) as f64).min(1.0);
    let (w, h) = (
        ((w as f64 * scale).round() as u32).max(2) & !1,
        ((h as f64 * scale).round() as u32).max(2) & !1,
    );
    let options = DecodeOptions::default()
        .starting_at(Rational::approximate(from.max(0.0)).unwrap_or(Rational::ZERO))
        .scaled_to(w, h);
    Decoder::open(path, &options).map_err(|error| format!("{path}: {error}"))
}

fn at(decoder: &Decoder, fallback: f64) -> f64 {
    decoder
        .position()
        .map(|position| position.as_f64())
        .unwrap_or(fallback)
}

/// The times, in seconds into `path`, at which a new shot begins.
pub fn scenes(path: &str, settings: Settings) -> Result<Vec<f64>, String> {
    scenes_between(path, 0.0, f64::INFINITY, settings)
}

/// [`scenes`] over only the stretch `from..to` seconds of `path`: what a clip
/// that shows part of its media needs.
pub fn scenes_between(
    path: &str,
    from: f64,
    to: f64,
    settings: Settings,
) -> Result<Vec<f64>, String> {
    let mut decoder = open(path, from)?;
    let mut detector = Detector::new(settings);
    let mut cuts = Vec::new();
    let mut clock = from.max(0.0);
    while clock < to {
        let Some(frame) = decoder.next_frame().map_err(|error| error.to_string())? else {
            break;
        };
        clock = at(&decoder, clock + 1.0 / 30.0);
        if detector.push(&frame).is_some() && clock < to {
            cuts.push(clock);
        }
    }
    Ok(cuts)
}

/// [`scenes`] with the two knobs as plain numbers: how far the colours must
/// move for a cut (`0.0..=1.0`) and the fewest frames a shot may last.
pub fn scene_cuts(path: &str, threshold: f32, min_shot: usize) -> Result<Vec<f64>, String> {
    scenes(
        path,
        Settings {
            threshold: threshold.clamp(0.05, 0.95),
            min_shot: min_shot.max(1),
        },
    )
}

/// The mean and spread of `path`'s picture at `seconds`, in Lab.
pub fn colour_numbers(path: &str, seconds: f64) -> Result<([f32; 3], [f32; 3]), String> {
    let stats = colour_of(path, seconds)?;
    Ok((stats.mean, stats.spread))
}

/// How the picture at `seconds` into `path` is graded.
pub fn colour_of(path: &str, seconds: f64) -> Result<Stats, String> {
    let mut decoder = open(path, seconds)?;
    let frame = decoder
        .next_frame()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("{path}: no picture at {seconds}s"))?;
    Stats::of(&frame).ok_or_else(|| format!("{path}: the picture could not be measured"))
}

/// The grade that makes the clip at `source` look like `reference`, each read
/// at the given time.
pub fn colour_match(
    source: (&str, f64),
    reference: (&str, f64),
) -> Result<Transfer, String> {
    Ok(Transfer::between(
        &colour_of(source.0, source.1)?,
        &colour_of(reference.0, reference.1)?,
    ))
}

/// One step of a followed point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    /// Seconds into the media.
    pub seconds: f64,
    /// Where the point is, `0.0..=1.0` across and down the picture.
    pub x: f32,
    /// See `x`.
    pub y: f32,
}

/// Follows the point `start` (`0.0..=1.0` across and down) from `from` to `to`
/// seconds. The path ends where the point was lost.
pub fn follow(path: &str, from: f64, to: f64, start: (f32, f32)) -> Result<Vec<Fix>, String> {
    let mut decoder = open(path, from)?;
    let (w, h) = (decoder.width() as f32, decoder.height() as f32);
    let Some(first) = decoder.next_frame().map_err(|error| error.to_string())? else {
        return Ok(Vec::new());
    };
    let mut clock = at(&decoder, from);
    let mut follower = Follower::start(&first, (start.0 * w, start.1 * h));
    let mut path_out = vec![Fix {
        seconds: clock,
        x: start.0,
        y: start.1,
    }];
    while clock < to {
        let Some(frame): Option<Frame> = decoder.next_frame().map_err(|error| error.to_string())?
        else {
            break;
        };
        clock = at(&decoder, clock + 1.0 / 30.0);
        match follower.advance(&frame) {
            Some((x, y)) => path_out.push(Fix {
                seconds: clock,
                x: x / w,
                y: y / h,
            }),
            None => break,
        }
    }
    Ok(path_out)
}
