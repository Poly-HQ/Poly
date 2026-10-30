<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./assets/poly-logo.svg">
  <source media="(prefers-color-scheme: light)" srcset="./assets/poly-logo-dark.svg">
  <img src="./assets/poly-logo.svg" alt="Poly Logo" height="80">
</picture>

### An enhanced, native music streaming client built with Rust and GPUI.
**A fork of [Sonora](https://github.com/sonorahq/sonora)**

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=flat-square)](./COPYING)
[![Rust: Edition 2024](https://img.shields.io/badge/Rust-Edition%202024-orange.svg?style=flat-square)](https://www.rust-lang.org/)
[![Upstream: Sonora](https://img.shields.io/badge/Upstream-Sonora-purple.svg?style=flat-square)](https://github.com/sonorahq/sonora)

<p align="center">
  <a href="#about-the-fork">About The Fork</a>
  ·
  <a href="#new-features--enhancements">New Features</a>
  ·
  <a href="#core-features">Core Features</a>
  ·
  <a href="#building--installation">Building</a>
  ·
  <a href="#licensing--legal">License</a>
</p>

</div>

---

## About The Fork

**Poly Windows** is an open-source derivative fork of **[Sonora](https://github.com/sonorahq/sonora)**, created by the Poly Project.

While upstream Sonora provides a blazing-fast, native music streaming foundation built in Rust and [GPUI](https://github.com/zed-industries/zed), **Poly Windows** elevates the desktop listening experience with:
- **Apple-grade Automix and Multiband Crossfading** with dual-deck real-time DSP filter sweeps and vocal morphing.
- **Acrylic glassmorphic aesthetics** and floating, scroll-following window controls.
- **A completely redesigned Curated Home screen** with personalized shelves and trending tracks.
- **High-resolution cover artwork decoding** across streaming providers.
- **Enhanced lyrics synchronization hierarchy** with smooth pop-in and pop-out animations.
- **AMLL-inspired Ambient Fullscreen mode** with low-overhead dynamic color blending.

All original upstream copyright notices and credits belong to the [Sonora Contributors](https://sonorahq.org). This fork is fully open-source and distributed under the **GNU General Public License version 3.0 (GPL-3.0-or-later)**. For full details on modifications and GPL compliance, see [`NOTICE.md`](./NOTICE.md).

---

> [!IMPORTANT]
> **Poly Windows is not a piracy tool.**
>
> Like upstream Sonora, Poly Windows is not a platform for obtaining, ripping, or distributing copyrighted material. We will not implement functions that export decrypted streams, DRM licenses, content keys, or convert protected streams into media files.
>
> Poly Windows requires valid user authentication and subscriptions for each connected streaming provider (Spotify, Apple Music, Subsonic, etc.). Protected content is streamed directly through official provider mechanisms.

---

## New Features & Enhancements

### 1. Studio-Grade Automix & Multiband Crossfade Engine
- **Dual-Deck Overlapping Audio Architecture**: Overlaps incoming and outgoing tracks without audio gaps or clicks.
- **Dynamic Biquad Filter Sweeps**: Frequency-swept high-pass filters (420Hz ↔ 20Hz) ensure seamless low-end bass transitions between tracks.
- **Vocal-Morphing Gain Curves**: Preserves vocals during song handoffs rather than abruptly fading the entire mix.
- **Flexible Modes**: Choose between **AutoMix** (intelligent track-length analysis), **Auto Crossfade**, or **Manual Crossfade** (1s to 12s duration). Configurable under *Settings → Playback*.

### 2. Curated Home Screen & Poly Design System
- **Curated Music Shelves**: Personalized recommendations, trending releases, and curated music sections (`crates/views/src/screens/home/curated.rs`).
- **Acrylic Frosted Glass Player Bar**: Translucent glassmorphism with subtle gradient fades, border-smoothing, and responsive layout.
- **Floating Windows Controls**: Minimalist, rounded window buttons (minimize, maximize, close) floating at the top-right that smoothly follow user scrolling.
- **Custom Poly Iconography**: Native Lucide vector icons for recommendations, charts, trending tracks, and playback indicators.

### 3. Full-Resolution Artwork Pipeline
- Replaced low-resolution thumbnails with uncompressed, maximum-resolution artwork fetchers for Spotify, Apple Music, and local libraries.

### 4. Advanced Lyrics Provider Hierarchy & Fluid Motion
- **Intelligent Lyrics Ranking**: Automatically selects the highest-granularity lyrics source:
  1. Word-by-word synced (Apple TTML)
  2. Sentence-by-sentence synced (LRC / Spotify)
  3. Plain unsynced text
- **Fluid Pop-In / Pop-Out Drawer**: Smooth animated opening and closing transitions for lyrics with zero harsh page layout stutter.

### 5. AMLL-Inspired Ambient Fullscreen Display
- Dynamic, GPU-efficient ambient backdrop that extracts color palettes from active album artwork and animates floating neon color spheres.

### 6. Looping & Infinite Smart Playback
- Flexible repeat states: Single Track Repeat, Playlist Repeat, and infinite autoplay continuation styled after your current music session.

### 7. Accelerated Windows Development Workflow
- Includes `build-fast.bat` with optimized MSVC/mold flags and incremental cache configurations for rapid iteration.

---

## Core Features (Inherited from Sonora)

- **Streaming Providers**: Spotify (via librespot), Apple Music, YouTube Music, Deezer, Subsonic/Navidrome, and Local Files.
- **Native GPUI Engine**: Zed's GPU-accelerated UI framework delivering 120fps+ smooth rendering with minimal CPU overhead.
- **Audio Pipeline**: Gapless playback, EBU R128 loudness normalization, and custom equalizer with preset memory.
- **Rich Presence & Integrations**: Discord Rich Presence, Last.fm, ListenBrainz, LibreFM, and Maloja scrobbling.
- **Localization**: Fluent i18n localization support.

---

## Building & Installation

### Prerequisites (Windows)
- **Rust Toolchain**: Rust 1.84+ (edition 2024 support, MSVC toolchain recommended)
- **C++ Build Tools**: Visual Studio C++ Build Tools (required for Widevine / native components)
- **Git**

### Rapid Build (Windows)
To build with optimizations and fast linking:
```cmd
build-fast.bat
```

### Standard Cargo Build
```sh
# Check workspace
cargo check --workspace

# Run debug build
cargo run --package sonora

# Build release binary
cargo build --release --package sonora
```
The compiled executable will be located at `target/release/sonora.exe`.

### Linux / macOS
```sh
# Nix environment (optional)
nix develop

# Standard cargo run
cargo run --locked --package sonora
```

---

## Project Structure

```
crates/
  sonora/       Binary entrypoint, window initialization, single-instance coordination
  views/        GPUI view screens, shells (Workspace, Fullscreen), and app chrome
  state/        Reactive app state, playback orchestration, and async tasks
  music/        Audio engine, DSP transition engine, providers, and models
  storage/      SQLite database for cache and persistent state
  ui/           Poly design system tokens, typography, glassmorphism, and widgets
  router/       Destination router and navigation history
  input/        Input handlers and global keyboard shortcuts
  i18n/         Fluent localization and translations
  icons/        Icon asset registry and SVG loaders
  embed/        Build-script helper for static asset bundling
  webview/      Native browser wrapper for authentication flows
  widevine/     Widevine CDM client host
```

---

## Licensing & Legal

Poly Windows is free software released under the **GNU General Public License version 3.0 (GPL-3.0-or-later)**.

- **Upstream Project:** Sonora (<https://github.com/sonorahq/sonora>), Copyright (C) Sonora Contributors.
- **Fork Project:** Poly Windows (<https://github.com/Poly-HQ/Poly>), Modifications Copyright (C) 2026 Poly Project Contributors.
- **License Terms:** See the [`COPYING`](./COPYING) and [`LICENSE`](./LICENSE) files for the full GNU General Public License v3.0 text.
- **Modification Notice:** See [`NOTICE.md`](./NOTICE.md) for detailed attribution and the itemized ledger of modifications in accordance with GPL-3.0 Section 5.
