<p align="center">
  <img src="https://cdn.jsdelivr.net/gh/jub0t/Concat@main/assets/concat-mark-dark.png" alt="Concat" width="100" />
</p>

<h1 align="center">Concat — CapCut Pro Unlocked Edition</h1>
<h3 align="center">The 100% Free, Open-Source, Zero-Paywall CapCut Alternative for Desktop & Mobile.</h3>

<div align="center">

<img src="https://cdn.jsdelivr.net/gh/jub0t/Concat@main/assets/editor.png" alt="Concat Editor Workspace" width="100%" />

<p align="center">
  <a href="https://github.com/Ddos-spec/Concat"><img src="https://img.shields.io/badge/Repo-Ddos--spec%2FConcat-0568FD?style=flat&logo=github&labelColor=212123" alt="GitHub Repo" /></a>
  <a href="https://github.com/Ddos-spec/Concat/tree/feat/windows-desktop"><img src="https://img.shields.io/badge/Branch-feat%2Fwindows--desktop-2ea44f?style=flat&logo=git&labelColor=212123" alt="Branch Desktop" /></a>
  <a href="https://github.com/Ddos-spec/Concat/tree/feat/android-mobile"><img src="https://img.shields.io/badge/Branch-feat%2Fandroid--mobile-00E5FF?style=flat&logo=android&labelColor=212123" alt="Branch Mobile" /></a>
  <img src="https://img.shields.io/badge/Paywall-0%25%20Bypassed-brightgreen?style=flat&labelColor=212123" alt="Zero Paywall" />
  <img src="https://img.shields.io/badge/Engine-Rust%202024%20%2B%20WGSL%20GPU-orange?style=flat&logo=rust&labelColor=212123" alt="Rust 2024 Engine" />
  <img src="https://img.shields.io/badge/License-AGPL%20v3-0568FD?style=flat&logo=gnu&labelColor=212123" alt="License: AGPL-3.0-or-later" />
</p>

**English** · [Bahasa Indonesia](docs/README.id.md)

</div>

---

## ⚡ The Story & Mission

CapCut is undeniably fast and popular for creating short-form content, but it has increasingly locked basic, essential editing features behind aggressive subscription paywalls: **Freemium barriers, export watermarks, paid velocity curves, cloud-only templates, and AI retouch tokens**.

**Concat CapCut Pro Unlocked Edition** transplants and reverse-engineers CapCut's iconic creative workflow directly into a high-performance native **Rust 2024 engine** with a WebGPU/Vulkan hardware-accelerated compositor. 

- 🚫 **Zero Freemium. Zero Watermarks. Zero Paywalls.** All Pro features are unlocked out-of-the-box.
- 🔒 **100% On-Device & Offline.** Zero mandatory cloud uploads, zero telemetry, zero accounts.
- ⚡ **Native C++/Rust Speed.** Instant 60 FPS timeline scrubbing and fast 4K export.

---

## 📸 Real Render Output & Live Visual Proof

Everything is tested and proven with end-to-end rendering on **Vulkan Lavapipe GPU** and hardware pipelines:

<table width="100%">
  <tr>
    <th width="50%" align="center">🎬 1080x1920 Vertical TikTok / Reels Render</th>
    <th width="50%" align="center">✨ WGSL GPU Shader Preview Cards</th>
  </tr>
  <tr>
    <td align="center">
      <img src="assets/capcut_showcase/capcut_vertical_preview.png" alt="9:16 Vertical TikTok Preview Frame" width="100%" />
      <br/>
      <sub><b>Composited 1080x1920 Frame (Detik 0.5)</b>: Hero Velocity Curve + Bilateral Skin Smoothing + Jedag-Jedug Flash</sub>
    </td>
    <td align="center">
      <img src="assets/capcut_showcase/card_face_retouch.jpg" alt="Face Retouch Shader Card" width="48%" />
      <img src="assets/capcut_showcase/card_jedag_jedug.jpg" alt="Jedag-Jedug Shader Card" width="48%" />
      <br/>
      <sub><b>Left</b>: <code>concat.face-retouch</code> (Edge-preserving Bilateral Filter)<br/><b>Right</b>: <code>concat.jedag-jedug</code> (Rhythmic Bass Zoom & Exposure Flash)</sub>
    </td>
  </tr>
</table>

---

## 🚀 7-Stage CapCut Transplantation Architecture

Our engineering replaces CapCut's cloud-gated features with open, local primitives:

```
┌────────────────────────────────────────────────────────────────────────┐
│                   CAPCUT 1:1 UNLOCKED FEATURE SUITE                    │
├────────────────────────────────┬───────────────────────────────────────┤
│ 1. 28x Velocity Curves & Shaders│ Montage, Hero, Bullet, Jump Cut, etc. │
│ 2. Audio Beat-Sync Engine      │ RMS Energy Flux Kick & Snare Detector │
│ 3. Offline Kokoro Multi-Voice  │ Zero-latency On-device TTS Synthesizer│
│ 4. AI 9:16 Auto-Reframe        │ Probability-weighted Centroid Tracking│
│ 5. Motion Tracking Keyframing  │ RANSAC Sub-pixel Affine Optical Flow  │
│ 6. 1-Click Viral Templates     │ Self-contained Local Preset Bundles   │
│ 7. Anti-Freemium Bypass Gate   │ 100% Unlocked Pro Flags & Export      │
└────────────────────────────────┴───────────────────────────────────────┘
```

### 1. Velocity Curve Presets & Custom Bezier (`concat-effects::speed_ramp`)
Replaces CapCut Pro's locked curve templates with full spline velocity remapping:
* **Montage**: Pulsing slow-fast-slow sequence rhythm (`0.6x` → `3.0x`).
* **Hero**: Dramatic entry, extended slow-motion impact, fast exit (`2.5x` → `0.5x` → `2.5x`).
* **Bullet-Time**: Hyper-slow 0.2x Matrix freeze moment (`3.0x` → `0.2x` → `3.0x`).
* **Jump Cut**: Snappy burst cut across action frames (`1.0x` → `8.0x` → `1.0x`).
* **Flash In / Flash Out**: High-energy intro/outro rushing transitions.

### 2. Auto Beat-Sync Rhythm Detection (`concat-speech::beat_sync`)
Direct audio analysis using 20ms RMS energy flux windows with adaptive thresholding:
* **Beat 1 (Main Drops)**: Isolates major musical bar downbeats and heavy bass kicks.
* **Beat 2 (Fast Jedag-Jedug)**: Captures rapid snares, hi-hat accents, and rhythmic peaks.
* Renders yellow beat markers directly on the timeline for effortless magnetic cutting.

### 3. AI Smart 9:16 Auto-Reframe (`concat-vision::reframe`)
Converts horizontal `16:9` footage into viral vertical `9:16` formats automatically:
* Analyzes segmentation masks to compute probability-weighted center-of-mass centroids:
  $$\mathbf{c} = \frac{\sum_{(x,y)} p(x,y) \cdot \mathbf{x}}{\sum_{(x,y)} p(x,y)}$$
* Implements a dual-pass forward/backward alpha-smoothed virtual camera to guarantee zero phase lag.

### 4. GPU-Accelerated Face Retouch (`concat.face-retouch`)
Native WebGPU WGSL shader performing edge-preserving bilateral filtering:
* Softens pores, wrinkles, and skin blemishes while retaining sharp clarity around eyes, eyebrows, and lips.
* 100% local shader execution without needing cloud beauty credits.

### 5. 1-Click Viral Templates (`concat-effects::templates_capcut`)
Instant preset bundles applying aspect ratio, velocity curves, and GPU shader chains simultaneously:
* **Jedag-Jedug Bass**: 9:16 + Jump Cut Curve + Bass Zoom & Chromatic Aberration.
* **Velocity Hero**: 9:16 + Hero Curve + Contrast Pop & Bloom Pulse.
* **AI Podcast Shorts**: 9:16 + Normal + Face Retouch Bilateral Filter + Clean Grading.
* **Bullet-Time 4K**: 16:9 + Bullet Curve + Classic Chrome Cinematic Tone.
* **Beauty Glow Daily Vlog**: 9:16 + Montage Curve + Skin Smoothing & Soft Glow.

---

## 📱 Dual-Platform Support (Desktop & Android Mobile)

### Desktop (Windows / macOS / Linux)
* Powered by Slint UI and native GPU backends (DirectX 12, Vulkan, Metal).
* Full multi-track magnetic timeline, keyframe curve editor, and color grading wheels.
* Built-in MCP Server (`concat --mcp`) for autonomous AI video editing via Claude / Antigravity.

### Mobile (Android Native Jetpack Compose)
* **Architecture**: Kotlin + Jetpack Compose UI communicating via UniFFI bridge to `libconcat_mobile.so`.
* **Zero-Jank Preview**: Native `SurfaceView` rendered directly by Vulkan/OpenGL ES.
* **Touch-Friendly Controls**: Aspect ratio switcher (`9:16`, `16:9`, `1:1`, `4:5`), velocity curve dock, and retouch sliders.

---

## 🛠️ Verification & Test Suite Summary

Every single crate in the repository is validated against extensive automated test suites on production Linux environments:

```bash
# Run unit tests across all transplanted CapCut crates
cargo test -p concat-effects -p concat-vision -p concat-speech -p concat-project -p concat-mobile --lib
```

| Crate Package | Passed Tests | Test Coverage Scope |
|---|:---:|---|
| **`concat-effects`** | **74 Passed** | Velocity curve monotonicity, WGSL shaders, fixture probes, template registry |
| **`concat-vision`** | **54 Passed** | Auto-reframe centroid bounds, RANSAC camera stabilization, AI brush masks |
| **`concat-speech`** | **14 Passed** | Audio beat-sync pulse detection, Kokoro TTS voice synthesis, Whisper transcription |
| **`concat-project`** | **191 Passed** | Multi-track timeline, continuous speed trimming, ripple edits, undo/redo history |
| **`concat-mobile`** | **2 Passed** | UniFFI export bridge, native rate fractions, Android JNI bindings |
| **Total** | **335 Passed, 0 Failed** | **100% Clean Green Build** |

---

## 📦 How to Build from Source

### Prerequisites
* Rust 1.85+ / 2024 Edition (`rustup default stable`)
* FFmpeg 7+ development libraries (`libavformat`, `libavcodec`, `libswscale`, `libavfilter`)
* CMake & C++ compiler (Clang / GCC / MSVC)
* Vulkan SDK or OpenGL drivers

### 1. Build Desktop Engine & CLI
```bash
git clone https://github.com/Ddos-spec/Concat.git
cd Concat/src

# Build CLI & Test Runner
cargo build --release -p concat-cli

# Run End-to-End Vertical Slice Render
./target/release/concat-cli render input.mp4 output.mp4 --frames 90
```

### 2. Build Android Mobile APK
```bash
cd Concat/android-native

# Build native Rust .so and UniFFI Kotlin bindings
./build-rust.sh release

# Assemble Universal Debug APK
./gradlew assembleDebug
```

---

## ⚖️ License & Open-Source Integrity

Concat is free software licensed under the **AGPL-3.0-or-later**. All shaders, curves, and algorithms are self-contained, standalone open implementations designed to free video creators from predatory subscription models.
