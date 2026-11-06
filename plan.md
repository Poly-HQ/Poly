# Plan — Thai font, wrong motion artwork, frozen motion artwork

Three separate bugs with three separate root causes. Each one is traced to a specific line below.

---

## 1. Thai lyrics still show heads (looped glyphs)

### Root cause (in GPUI, not in Poly)
`gpui_windows/src/direct_write.rs` → `generate_font_fallbacks` (line 388):

```rust
let font_set = &system_font_collection.GetFontSet()?;      // only looks in SYSTEM fonts
for family_name in fallback_list {
    let Some(fonts) = font_set.GetMatchingFonts(&family_name, ...) else { continue };
    let Ok(font_face) = fonts.GetFontFaceReference(0) else { continue };   // skips embedded fonts
    ...
    builder.AddMapping(&ranges, &[family_name], None /* = system collection */, ...);
}
```

- Our `NotoSansThai-*.ttf` files are added with `add_fonts`, so they go into `custom_font_collection`. They are not in the system collection.
- That means `"Noto Sans Thai"` in `SCRIPTS` gets **silently skipped**. The first fallback that matches is `"Leelawadee UI"` (system font, looped), and it wins.
- Adding more font files or reordering `SCRIPTS` can't fix this. The fallback builder never sees embedded fonts.
- The same bug also breaks `"Inter"` as a fallback when a custom UI font is chosen.

### Fix (recommended): patch `generate_font_fallbacks`
```diff
 fn generate_font_fallbacks(
     fallbacks: &FontFallbacks,
     factory: &IDWriteFactory5,
+    custom_font_collection: &IDWriteFontCollection1,
     system_font_collection: &IDWriteFontCollection1,
 ) -> Result<Option<IDWriteFontFallback>> {
     ...
-    let font_set = &system_font_collection.GetFontSet()?;
+    let sets = [
+        (custom_font_collection.GetFontSet()?, custom_font_collection),
+        (system_font_collection.GetFontSet()?, system_font_collection),
+    ];
     for family_name in fallback_list {
-        let Some(fonts) = font_set.GetMatchingFonts(...) else { continue };
+        let Some((fonts, collection)) = sets.iter().find_map(|(set, collection)| {
+            let fonts = set.GetMatchingFonts(&family_name, ...).ok()?;
+            (fonts.GetFontCount() > 0).then_some((fonts, *collection))
+        }) else { continue };
         ...
-        builder.AddMapping(&ranges, &[family_name.as_ptr()], None, None, None, 1.0)?;
+        builder.AddMapping(&ranges, &[family_name.as_ptr()], Some(&collection.cast()?), None, None, 1.0)?;
     }
```
`make_font_from_font_collection` passes `custom_font_collection` through (one extra argument at the single call site, line 494).

With this change, Thai codepoints map to the embedded **Noto Sans Thai** before Leelawadee. DirectWrite then picks the SemiBold/Bold static faces we already embed.

### How to ship the GPUI patch
GPUI is a git dependency: `sonorahq/gpui @ 5ff26c7`. Two options:

| Option | How | Trade-off |
|---|---|---|
| **A. Fork (recommended)** | Fork `sonorahq/gpui` under your account, commit the patch on top of `5ff26c7`, then change `rev`/`git` for `gpui` + `gpui_platform` in the root `Cargo.toml` | Clean and reproducible in CI. Needs one GitHub fork. |
| B. Vendor + `[patch]` | Copy the GPUI checkout to `vendor/gpui`, add `[patch."https://github.com/sonorahq/gpui"]` entries for **all** gpui crates (patching only `gpui_windows` would pull in two copies of `gpui` and break the types) | Works offline, but adds a large vendored tree to the repo. |

Rejected alternatives:
- Splitting every Thai run into a separate `StyledText` font run in the app. This touches every text site and the lyrics width measurement (`lyrics_wrap_rows` shapes whole lines), so it's fragile.
- Installing the font per-user into Windows. It changes the user's machine.

---

## 2. Wrong motion artwork ("Just Keep Watching" got `greedy`)

### Root cause
`crates/music/src/artwork.rs` → `search_apple_motion` (lines 460-483) calls `closest(...)`. That function's own doc says it returns *"the candidate whose name matches exactly, **or failing that the first one**"*. So when no album matches `F1 The Album`, it returns the first Tate McRae album that has a video (`greedy - Single`). The `matched.len() == 1` fallback is just as loose.

### Fix
- Add a strict matcher `motion_match(name, wanted)`. It accepts only `compared(searchable(name)) == compared(wanted)` or `extends(...)`, which are the same normalizers the songs branch already uses. With it, `"greedy - Single"` matches title `"greedy"`, but nothing matches `"F1 The Album"`.
- Use it instead of `closest` for album candidates, matching against the album name first and the title second.
- Remove the `matched.len() == 1` fallback.
- Log every rejection: `[motion-artwork] Rejected album candidate 'greedy - Single' (wanted 'F1 The Album' / 'Just Keep Watching')`.
- Delete the stale disk cache entry? Not needed. The cache is keyed by playlist URL, and the wrong URL will no longer be resolved.

Result: tracks with no real motion cover keep the static cover. No borrowed video.

---

## 3. Decoded motion artwork doesn't move

### What we know
- The decode works: `Decoded 75 frames successfully`.
- `Artwork::render` already gives `img` an element ID, so GPUI's built-in frame loop *should* run.
- GPUI only advances frames inside `img`'s `request_layout`, and only when **all** of these hold (`elements/img.rs` lines 321-390): `frame_count > 1`, `!cx.reduce_motion()`, `window.is_window_active()`, and the element is laid out again every frame.

### Competing hypotheses
| # | Hypothesis | Fastest check |
|---|---|---|
| H1 | Media Foundation returns the same picture 75 times (wrong sample handling), so it "plays" but every frame looks identical | Log a checksum of frames 0 / 37 / 74 in `decode_mp4_file_inner` |
| H2 | `cx.reduce_motion()` is true (Windows "Animation effects" off, or the app setting), which freezes every animated image | Log `cx.reduce_motion()` once when the motion image is served |
| H3 | The fullscreen cover isn't laid out again each frame (the `.layer_scale` / `.motion` wrappers or the view not being redrawn), so `frame_index` stays at 0 | Log `frame_count` + a counter in the `ImageSource::Custom` closure: one line means no re-layout |

### Fix
1. **Diagnostics first.** Add the H1/H2/H3 logs (`[motion-artwork]` prefix, same style as the existing ones) so the first run shows which hypothesis is true.
2. **Our own player, so we don't depend on GPUI's `img` loop.** Add a `MotionArtwork` element in `crates/ui/src/artwork.rs` that the fullscreen view uses when the URL is a `file://…mp4`:
   - Split the decoded `RenderImage` into one single-frame `Arc<RenderImage>` per frame, cached once per URL.
   - `FullscreenView` holds a `motion_frame: usize` and a `Task` that ticks every `frame.delay()` (40 ms ≈ 25 fps, clamped 20-100 ms), advances `motion_frame`, and calls `cx.notify()`.
   - It renders `img(ImageSource::Render(frames[motion_frame]))`. A static texture per frame means the GPU atlas uploads each frame once, after which drawing is just a quad, so it's cheap on an iGPU.
   - The tick pauses when fullscreen is closed or the window loses focus, and is dropped on track change.
   - This handles H2 and H3 directly. H1 is caught by the diagnostic, and if it fires the fix goes in the decoder (re-read the sample buffer per `ReadSample` and check `MF_SOURCE_READERF_STREAMTICK`).
3. Keep `max_frames = 75` (3 s loop at 25 fps, about 30 MB of BGRA at 320²). If the loop looks too short, raise it to 150 with `target_edge` 320 (about 60 MB).

---

## Files touched
| File | Change |
|---|---|
| GPUI fork → `crates/gpui_windows/src/direct_write.rs` | Fallback lookup includes the custom collection |
| `Cargo.toml` (root) | Point `gpui` / `gpui_platform` at the fork rev |
| `crates/music/src/artwork.rs` | Strict `motion_match`, drop the loose fallbacks, rejection logs |
| `crates/ui/src/artwork.rs` | Frame-checksum + reduce-motion logs, frame splitter, `MotionArtwork` element |
| `crates/views/src/shells/fullscreen.rs` | Frame ticker task + use `MotionArtwork` for `.mp4` covers |

## Verification (you run these, per execution policy)
1. `cargo build` → run Poly.
2. **Thai:** play the T!NE track, open fullscreen lyrics. Glyphs should be loopless Noto Sans Thai (compare ช, ง, ม, อ). Mixed lines like `I told that bitch ว่าถ้าเป็น` should show Latin in Inter and Thai in Noto.
3. **Wrong artwork:** play *Just Keep Watching*. The log should show `Rejected album candidate 'greedy - Single'…` and the cover stays static.
4. **Motion:** play a track whose own album has motion (e.g. *greedy*). The log should show three different frame checksums and the cover should loop smoothly in fullscreen.
5. Commit in three separate commits: font, matching, player.

## Open questions
1. **GPUI patch delivery:** Option A (fork, recommended) or Option B (vendor + `[patch]`)? If A, give me the fork URL once it exists, or tell me to do it via `gh repo fork`.
2. **Motion loop length:** keep 75 frames (~3 s) or go to 150 (~6 s, about 2× memory)?
