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
    pub id: String,
    /// Display name.
    pub name: String,
    /// Category tag (`Trending`, `Velocity`, `AI Podcast`, `Aesthetic`).
    pub category: String,
    /// Target aspect ratio.
    pub aspect: AspectPreset,
    /// Velocity curve preset applied to clips.
    pub speed_ramp: SpeedRampPreset,
    /// Built-in GPU effect package IDs automatically applied.
    pub effect_ids: Vec<String>,
    /// Whether automatic audio beat-sync cuts are enabled.
    pub auto_beat_sync: bool,
    /// Whether smart 9:16 face-tracking auto-reframe is enabled.
    pub auto_reframe: bool,
}

impl ViralTemplate {
    /// Helper to construct a static template.
    pub fn new(
        id: &str,
        name: &str,
        category: &str,
        aspect: AspectPreset,
        speed_ramp: SpeedRampPreset,
        effect_ids: &[&str],
        auto_beat_sync: bool,
        auto_reframe: bool,
    ) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            category: category.to_owned(),
            aspect,
            speed_ramp,
            effect_ids: effect_ids.iter().map(|&s| s.to_owned()).collect(),
            auto_beat_sync,
            auto_reframe,
        }
    }
}

/// Built-in library of 100% free viral templates (unlocked from CapCut Pro).
pub fn builtin_templates() -> Vec<ViralTemplate> {
    vec![
        ViralTemplate::new(
            "template.jedag-jedug-bass",
            "Jedag-Jedug Bass Zoom",
            "Trending",
            AspectPreset::Vertical9x16,
            SpeedRampPreset::JumpCut,
            &["concat.jedag-jedug", "concat.bloom-pulse", "concat.chromatic-aberration"],
            true,
            true,
        ),
        ViralTemplate::new(
            "template.velocity-hero-edit",
            "Velocity Hero Montage",
            "Velocity",
            AspectPreset::Vertical9x16,
            SpeedRampPreset::Hero,
            &["concat.contrast-pop", "concat.bloom-pulse"],
            true,
            true,
        ),
        ViralTemplate::new(
            "template.podcast-shorts-ai",
            "AI Podcast Smart Shorts",
            "AI Podcast",
            AspectPreset::Vertical9x16,
            SpeedRampPreset::Normal,
            &["concat.face-retouch", "concat.clean"],
            false,
            true,
        ),
        ViralTemplate::new(
            "template.bullet-time-cinematic",
            "Bullet-Time SlowMo 4K",
            "Velocity",
            AspectPreset::Landscape16x9,
            SpeedRampPreset::Bullet,
            &["concat.classic-chrome", "concat.adjust"],
            true,
            false,
        ),
        ViralTemplate::new(
            "template.beauty-glow-vlog",
            "Beauty Glow Daily Vlog",
            "Aesthetic",
            AspectPreset::Vertical9x16,
            SpeedRampPreset::Montage,
            &["concat.face-retouch", "concat.bright"],
            false,
            true,
        ),
    ]
}


/// Finds a built-in viral template by ID or display name.
pub fn find_template(query: &str) -> Option<ViralTemplate> {
    let q = query.trim();
    builtin_templates()
        .into_iter()
        .find(|t| t.id.eq_ignore_ascii_case(q) || t.name.eq_ignore_ascii_case(q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Catalogue;

    #[test]
    fn every_builtin_template_references_existing_gpu_effects() {
        let catalogue = Catalogue::builtin();
        let templates = builtin_templates();
        assert!(!templates.is_empty());
        for tpl in &templates {
            let (w, h) = tpl.aspect.dimensions();
            assert!(w > 0 && h > 0);
            assert!(!tpl.aspect.label().is_empty());
            for effect_id in &tpl.effect_ids {
                assert!(
                    catalogue.get(effect_id).is_some(),
                    "Template {} references missing effect {}",
                    tpl.id,
                    effect_id
                );
            }
        }
        assert!(find_template("template.jedag-jedug-bass").is_some());
    }
}
