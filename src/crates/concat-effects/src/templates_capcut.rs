// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Ddos-spec and Concat contributors

//! 1:1 CapCut Open Template & Viral Preset Pipeline (Zero-Cloud, 100% Free).
//!
//! Replaces CapCut's cloud-locked template ecosystem with local, self-contained
//! preset bundles combining velocity curves, GPU shaders, auto-reframe ratios,
//! and beat-synced transitions.

use serde::{Deserialize, Serialize};

use crate::speed_ramp::SpeedRampPreset;

/// Aspect ratio target for a template.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspectPreset {
    /// 9:16 Vertical (TikTok, Reels, Shorts).
    Vertical9x16,
    /// 16:9 Widescreen (YouTube, Desktop).
    Landscape16x9,
    /// 1:1 Square (Instagram Feed).
    Square1x1,
    /// 4:5 Portrait.
    Portrait4x5,
}

impl AspectPreset {
    /// Width-to-height ratio as `(width, height)`.
    pub const fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Vertical9x16 => (1080, 1920),
            Self::Landscape16x9 => (1920, 1080),
            Self::Square1x1 => (1080, 1080),
            Self::Portrait4x5 => (1080, 1350),
        }
    }

    /// Display label in the aspect ratio switcher.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vertical9x16 => "9:16 TikTok/Reels",
            Self::Landscape16x9 => "16:9 YouTube",
            Self::Square1x1 => "1:1 Square",
            Self::Portrait4x5 => "4:5 Portrait",
        }
    }
}

/// A one-click CapCut-style viral template definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ViralTemplate {
    /// Unique template identifier.
    pub id: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Category tag (`Trending`, `Velocity`, `AI Podcast`, `Aesthetic`).
    pub category: &'static str,
    /// Target aspect ratio.
    pub aspect: AspectPreset,
    /// Velocity curve preset applied to clips.
    pub speed_ramp: SpeedRampPreset,
    /// Built-in GPU effect package IDs automatically applied.
    pub effect_ids: &'static [&'static str],
    /// Whether automatic audio beat-sync cuts are enabled.
    pub auto_beat_sync: bool,
    /// Whether smart 9:16 face-tracking auto-reframe is enabled.
    pub auto_reframe: bool,
}

/// Built-in library of 100% free viral templates (unlocked from CapCut Pro).
pub const BUILTIN_TEMPLATES: &[ViralTemplate] = &[
    ViralTemplate {
        id: "template.jedag-jedug-bass",
        name: "Jedag-Jedug Bass Zoom",
        category: "Trending",
        aspect: AspectPreset::Vertical9x16,
        speed_ramp: SpeedRampPreset::JumpCut,
        effect_ids: &["concat.jedag-jedug", "concat.bloom-pulse", "concat.chromatic-aberration"],
        auto_beat_sync: true,
        auto_reframe: true,
    },
    ViralTemplate {
        id: "template.velocity-hero-edit",
        name: "Velocity Hero Montage",
        category: "Velocity",
        aspect: AspectPreset::Vertical9x16,
        speed_ramp: SpeedRampPreset::Hero,
        effect_ids: &["concat.contrast-pop", "concat.bloom-pulse"],
        auto_beat_sync: true,
        auto_reframe: true,
    },
    ViralTemplate {
        id: "template.podcast-shorts-ai",
        name: "AI Podcast Smart Shorts",
        category: "AI Podcast",
        aspect: AspectPreset::Vertical9x16,
        speed_ramp: SpeedRampPreset::Normal,
        effect_ids: &["concat.face-retouch", "concat.clean"],
        auto_beat_sync: false,
        auto_reframe: true,
    },
    ViralTemplate {
        id: "template.bullet-time-cinematic",
        name: "Bullet-Time SlowMo 4K",
        category: "Velocity",
        aspect: AspectPreset::Landscape16x9,
        speed_ramp: SpeedRampPreset::Bullet,
        effect_ids: &["concat.classic-chrome", "concat.adjust"],
        auto_beat_sync: true,
        auto_reframe: false,
    },
    ViralTemplate {
        id: "template.beauty-glow-vlog",
        name: "Beauty Glow Daily Vlog",
        category: "Aesthetic",
        aspect: AspectPreset::Vertical9x16,
        speed_ramp: SpeedRampPreset::Montage,
        effect_ids: &["concat.face-retouch", "concat.bright"],
        auto_beat_sync: false,
        auto_reframe: true,
    },
];
