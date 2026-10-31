// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! 1:1 CapCut Auto Beat-Sync & Smart Rhythm Engine (100% Free, Offline).
//!
//! Detects musical onsets, bass kicks and rhythmic peaks directly from decoded
//! PCM audio samples to place yellow CapCut-style beat markers on the timeline
//! for automatic "Jedag-Jedug" and velocity cutting.

/// A detected musical beat or onset marker on the timeline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeatMarker {
    /// Timestamp in seconds from the start of the audio track.
    pub time_sec: f64,
    /// Normalized onset strength (`0.0..=1.0`).
    pub strength: f32,
    /// True if this beat is a major bar downbeat / bass drop.
    pub is_downbeat: bool,
}

/// Sensitivity mode matching CapCut's "Auto Beat" 1 & 2 presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeatPreset {
    /// Beat 1: Major downbeats and primary bass kicks only (slower cuts).
    MainBeats,
    /// Beat 2: Every snare, hi-hat accent and kick (fast Jedag-Jedug cuts).
    FastRhythm,
}

/// Detects beat markers from mono PCM `samples` (`-1.0..=1.0`) at `sample_rate` Hz.
pub fn detect_audio_beats(
    samples: &[f32],
    sample_rate: u32,
    preset: BeatPreset,
) -> Vec<BeatMarker> {
    if samples.is_empty() || sample_rate == 0 {
        return Vec::new();
    }

    // 20 ms analysis window
    let win_len = ((sample_rate as usize) / 50).max(64);
    let num_wins = samples.len() / win_len;
    if num_wins < 4 {
        return Vec::new();
    }

    // Compute RMS energy per window
    let mut energies = Vec::with_capacity(num_wins);
    for i in 0..num_wins {
        let slice = &samples[i * win_len..(i + 1) * win_len];
        let sum_sq: f32 = slice.iter().map(|&s| s * s).sum();
        energies.push((sum_sq / (win_len as f32)).sqrt());
    }

    // Positive first-order energy flux (onset strength)
    let mut flux = vec![0.0_f32; num_wins];
    for i in 1..num_wins {
        let diff = energies[i] - energies[i - 1];
        if diff > 0.0 {
            flux[i] = diff;
        }
    }

    let max_flux = flux.iter().copied().fold(0.0_f32, f32::max);
    if max_flux <= 1e-6 {
        return Vec::new();
    }

    let (threshold_mult, min_gap_sec) = match preset {
        BeatPreset::MainBeats => (1.65_f32, 0.38_f64),
        BeatPreset::FastRhythm => (1.25_f32, 0.18_f64),
    };

    let local_radius = 20usize; // ~400ms local context window
    let mut markers = Vec::new();
    let mut last_beat_time = -10.0_f64;
    let mut beat_counter = 0usize;

    for i in 1..(num_wins - 1) {
        if flux[i] <= flux[i - 1] || flux[i] < flux[i + 1] {
            continue;
        }
        let start = i.saturating_sub(local_radius);
        let end = (i + local_radius + 1).min(num_wins);
        let local_mean: f32 = flux[start..end].iter().sum::<f32>() / ((end - start) as f32);

        if flux[i] > (local_mean * threshold_mult).max(max_flux * 0.08) {
            let time_sec = (i * win_len) as f64 / (sample_rate as f64);
            if time_sec - last_beat_time >= min_gap_sec {
                let strength = (flux[i] / max_flux).clamp(0.0, 1.0);
                let is_downbeat = beat_counter % 4 == 0 || strength > 0.78;
                markers.push(BeatMarker {
                    time_sec,
                    strength,
                    is_downbeat,
                });
                last_beat_time = time_sec;
                beat_counter += 1;
            }
        }
    }

    markers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_periodic_impulses_as_beats() {
        let sr = 16000u32;
        let mut pcm = vec![0.001_f32; sr as usize * 3];
        // Inject impulses at 0.5s, 1.0s, 1.5s, 2.0s, 2.5s
        for beat in 1..=5 {
            let idx = (beat * (sr as usize)) / 2;
            for k in 0..320 {
                if idx + k < pcm.len() {
                    pcm[idx + k] = 0.9;
                }
            }
        }
        let beats = detect_audio_beats(&pcm, sr, BeatPreset::MainBeats);
        assert!(beats.len() >= 4);
    }
}
