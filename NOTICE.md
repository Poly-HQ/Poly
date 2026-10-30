# Poly Windows - Sonora Fork Notice

This project is **Poly Windows**, an open-source music streaming application that is a derivative work and fork of **[Sonora](https://github.com/sonorahq/sonora)**.

---

## 1. Upstream Project & Copyright Notice

- **Original Project:** Sonora
- **Original Authors:** Sonora Contributors (<https://sonorahq.org>, <https://github.com/sonorahq/sonora>)
- **Original License:** GNU General Public License version 3.0 or later (GPL-3.0-or-later)
- **Upstream Repository:** <https://github.com/sonorahq/sonora>

The original copyright and all upstream notices remain the property of the Sonora Contributors and are preserved intact in accordance with Section 4 and Section 5 of the GNU General Public License version 3.0.

---

## 2. Fork Attribution & Modifications Notice (GPL-3.0 Section 5 Compliance)

In compliance with **Section 5(a), 5(b), and 5(c) of the GNU General Public License v3.0**:

- **Derivative Project:** Poly Windows (Sonora Fork)
- **Fork Maintainers:** Poly Project Contributors (<https://github.com/Poly-HQ/Poly>)
- **Date of Modifications:** October 2026
- **License:** GNU General Public License version 3.0 or later (`GPL-3.0-or-later`)

This program is free software: you can redistribute it and/or modify it under the terms of the **GNU General Public License** as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but **WITHOUT ANY WARRANTY**; without even the implied warranty of **MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE**. See the [`COPYING`](./COPYING) file or <https://www.gnu.org/licenses/> for full details.

---

## 3. Summary of Modifications and Added Features

The following primary architectural enhancements and features were introduced in this fork:

1. **Apple-Grade Automix & Advanced Multiband Crossfade Engine (`crates/music/src/transition.rs`):**
   - Implemented high-performance DSP transition engine with silence trimming, onset cushioning, and multiband crossover filtering.
   - Dual-deck overlapping playback system (`crates/music/src/spotify/playback.rs` and `crates/music/src/spotify/sink.rs`) supporting seamless intro/outro mixing.
   - Dynamic Biquad high-pass filter frequency sweeps and vocal-morph curves.
   - Configurable modes: AutoMix, Auto Crossfade, and Manual Crossfade with customizable duration.

2. **Curated Home Screen & Poly UI Overhaul:**
   - Added curated music recommendations and section layouts (`crates/views/src/screens/home/curated.rs`, `crates/views/src/screens/home/mod.rs`).
   - Integrated custom SVG icon pack (`poly-logo.svg`, `flame.svg`, `sparkles.svg`, `trending-up.svg`, `trophy.svg`, `mic-2.svg`, `bookmark.svg`, `more-horizontal.svg`).
   - Replaced default title bar with floating, scroll-following rounded window controls with transparent background.
   - Redesigned acrylic glassmorphic player control bar with gradient fades and border refinements.
   - Streamlined sidebar navigation matching Poly ecosystem specifications.

3. **High-Resolution Artwork Loader:**
   - Overhauled album and artist thumbnail resolution pipeline across Spotify and provider modules to fetch uncompressed/maximum-resolution cover art instead of low-resolution thumbnails.

4. **Hierarchical Lyrics Provider Algorithm & Smooth Pop Animations:**
   - Implemented multi-tier lyrics selection ranking: Word-by-word synced (TTML) > Sentence-by-sentence synced (LRC) > Unsynced lyrics.
   - Smooth pop-in and pop-out animations for lyrics drawer transitions with fluid page resizing.

5. **AMLL-Inspired Ambient Fullscreen Display:**
   - Low-overhead GPU ambient canvas featuring floating neon color blobs synchronized with album artwork palettes.

6. **Looping & Infinite Smart Autoplay:**
   - Enhanced repeat modes (single song repeat, playlist loop) and infinite style playback continuation.

7. **Windows Fast Build Tooling:**
   - Added `build-fast.bat` for optimized incremental developer compilation.

---

## 4. Itemized Ledger of Modified & Added Files

### Added Files (New in Poly Windows)
- `crates/music/src/transition.rs` - Multiband DSP transition, Biquad filter sweeps, and Automix engine.
- `crates/views/src/screens/home/curated.rs` - Curated home recommendations and section models.
- `build-fast.bat` - Optimized Windows build script.
- `assets/icons/lucide/poly-logo.svg` - Poly brand vector icon.
- `assets/poly-logo.svg` - Poly brand white wordmark vector logo (P logo with "oly").
- `assets/poly-logo-dark.svg` - Poly brand dark wordmark vector logo for light themes.
- `assets/icons/lucide/flame.svg` - Trending indicator icon.
- `assets/icons/lucide/sparkles.svg` - Curated / AI recommendation icon.
- `assets/icons/lucide/trending-up.svg` - Charts indicator icon.
- `assets/icons/lucide/trophy.svg` - Top picks icon.
- `assets/icons/lucide/mic-2.svg` - Lyrics / vocal track icon.
- `assets/icons/lucide/bookmark.svg` - Saved / favorites icon.
- `assets/icons/lucide/more-horizontal.svg` - Context menu icon.
- `assets/windows/poly.ico` - Poly Windows multi-resolution application icon.
- `NOTICE.md` - GPL-3.0 compliance notice and file modification ledger.
- `LICENSE` - GPL-3.0 license declaration for Poly Windows.

### Modified Files (from Upstream Sonora)
- `Cargo.toml` - Updated repository URL to the Poly fork.
- `.github/readme-icon.svg` - Updated readme header logo to Poly.
- `assets/icon.svg` - Master SVG icon updated with white Poly logo vector.
- `assets/tray/sonora.png` - System tray icon updated with Poly icon.
- `assets/windows/sonora.ico` - Windows executable icon updated with Poly icon.
- `assets/i18n/en-US/main.ftl` - Added localization strings for transitions, settings, and curated UI.
- `crates/music/src/audio.rs` - Added license attribution header.
- `crates/music/src/engine.rs` - Integrated multi-track fading and transition triggers.
- `crates/music/src/lib.rs` - Exported transition models and audio routing traits.
- `crates/music/src/lyrics/mod.rs` - Added lyrics ranking hierarchy.
- `crates/music/src/models.rs` - Extended track and genre models for curated shelves.
- `crates/music/src/scrobble/listenbrainz.rs` - Scrobble client branding updated to Poly.
- `crates/music/src/spotify/albums.rs` - High-resolution artwork URL resolution.
- `crates/music/src/spotify/artists.rs` - High-resolution artist image resolution.
- `crates/music/src/spotify/collection.rs` - High-resolution cover artwork resolution.
- `crates/music/src/spotify/pathfinder/album.rs` - Max-resolution artwork paths.
- `crates/music/src/spotify/pathfinder/artist.rs` - Max-resolution artist artwork paths.
- `crates/music/src/spotify/pathfinder/browse.rs` - Category and browse image enhancements.
- `crates/music/src/spotify/pathfinder/library.rs` - Library album image enhancements.
- `crates/music/src/spotify/pathfinder/search.rs` - Search result thumbnail quality upgrades.
- `crates/music/src/spotify/playback.rs` - Dual-deck playback architecture for seamless transitions.
- `crates/music/src/spotify/sink.rs` - Real-time DSP audio sample blending and filter modulation.
- `crates/music/src/subsonic/client.rs` - High-res cover resolution for Subsonic.
- `crates/music/src/youtube/genres.rs` - Artwork path refinement for YouTube Music.
- `crates/sonora/build.rs` - Windows executable metadata updated to Poly.
- `crates/sonora/src/actions.rs` - Application menu branding updated to Poly.
- `crates/sonora/src/main.rs` - Window title, app_id, bounds, and transition state initialization.
- `crates/sonora/src/tray/native.rs` - Tray tooltip updated to Poly.
- `crates/sonora/src/tray/sni.rs` - Tray title and tooltip updated to Poly.
- `crates/state/src/discord.rs` - Discord presence links and branding updated to Poly.
- `crates/state/src/lib.rs` - Re-exported transition and lyrics state components.
- `crates/state/src/lyrics.rs` - Provider scoring algorithm based on sync granularity.
- `crates/state/src/playback.rs` - State orchestration for dual-deck fading and countdown hooks.
- `crates/state/src/queue.rs` - Infinite autoplay continuation and repeat logic.
- `crates/state/src/remote/mod.rs` - OS media overlay display name updated to Poly.
- `crates/state/src/lib.rs` - Re-exported transition and lyrics state components.
- `crates/state/src/lyrics.rs` - Provider scoring algorithm based on sync granularity.
- `crates/state/src/playback.rs` - State orchestration for dual-deck fading and countdown hooks.
- `crates/state/src/queue.rs` - Infinite autoplay continuation and repeat logic.
- `crates/state/src/settings.rs` - Transition settings storage (mode, duration, toggle).
- `crates/state/src/window_shape.rs` - Floating window bounds and shape adjustments.
- `crates/ui/examples/icons.rs` - Registered new SVG icon entries.
- `crates/ui/src/artwork.rs` - Artwork caching and high-resolution image decoding.
- `crates/ui/src/controls.rs` - Control button styling and hover state refinements.
- `crates/ui/src/glass.rs` - Acrylic glassmorphism shader adjustments.
- `crates/ui/src/motion.rs` - Smooth spring easing curves for modal and drawer animations.
- `crates/ui/src/palette.rs` - Accent color palette refinement and dark mode tokens.
- `crates/ui/src/theme.rs` - Theme token additions for acrylic containers.
- `crates/views/src/chrome/mod.rs` - Exported updated chrome components.
- `crates/views/src/chrome/player_bar.rs` - Redesigned acrylic glass player bar with fades.
- `crates/views/src/chrome/sidebar_left.rs` - Streamlined navigation with Poly branding.
- `crates/views/src/chrome/sidebar_right.rs` - Pop-in/pop-out animated lyrics drawer.
- `crates/views/src/chrome/title_bar.rs` - Minimalist top bar delegating to floating controls.
- `crates/views/src/root.rs` - Root container layout and ambient background binding.
- `crates/views/src/screens/artist.rs` - Artwork header quality adjustments.
- `crates/views/src/screens/detail.rs` - Album view layout refinements.
- `crates/views/src/screens/home/mod.rs` - Overhauled home screen view with curated shelves.
- `crates/views/src/screens/login.rs` - Sign-in screen branding updated to Poly.
- `crates/views/src/screens/search.rs` - High-res search card grid.
- `crates/views/src/screens/settings.rs` - Transition, Crossfade, and Automix settings interface.
- `crates/views/src/shared/adaptive.rs` - Artwork color sampling improvements.
- `crates/views/src/shared/ambient.rs` - Apple Music-inspired ambient background renderer.
- `crates/views/src/shared/menus/context.rs` - Context menu styling and action entries.
- `crates/views/src/shared/picks.rs` - Pick card presentation refinements.
- `crates/views/src/shared/shelves.rs` - Curated horizontal shelf carousel.
- `crates/views/src/shared/transport.rs` - Playback transport control button updates.
- `crates/views/src/shells/fullscreen.rs` - Fullscreen ambient stage adjustments.
- `crates/views/src/shells/workspace.rs` - Floating window control positioning and animated layout resize.
