# Video Presenting, Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open one video file in the Videos tab, preview it with play, pause, seek, step, volume, and present it on the projector with sound.

**Architecture:** A new `sda-media` crate runs two `ffmpeg` children per file, one for RGBA frames and one for f32 PCM, both started at the same `-ss`. A `Player` paces frames against the audio clock through the existing `FrameSource`. The renderer gains `Layer::Video` drawn with `contain` over black. `LiveState` holds the live `Player` and maps next, prev, blank onto seek and pause.

**Tech Stack:** Rust 2024, Iced 0.14, wgpu 27, ffmpeg CLI from `sda_core::downloader`, rodio 0.23 with `default-features = false, features = ["playback"]`.

**Spec:** `docs/superpowers/specs/2026-10-20-video-presenting-design.md`

## Global Constraints

- Rust 1.88 or newer. `sda-app`, `sda-render`, and the new `sda-media` use edition 2024.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass. `cargo fmt --all -- --check` must pass.
- Every UI string goes into `sda_core::i18n::Strings` for En, It, and Ro. Use the helper described in Task 6 to add them, it keeps the field count test in sync.
- No new runtime dependency except `libasound2` on Linux. ffmpeg stays a child process.
- Tests never open an audio device and never run the real ffmpeg. `Sink::open` returns `None` when `LUMEN_NO_AUDIO` is set and every test that builds a `Player` sets it. Process tests run only on unix through a stub script.
- Phase 1 decisions: end of file goes black, blank pauses sound and picture, step is 10 s, no loop.
- Commit after each task with the repo style: `feat(scope): what`, `test(scope): what`, `docs: what`.
- Prose in comments and docs: simple sentences, no parentheses, no dashes, no semicolons.

---

## File map

Create
- `crates/sda-core/src/media.rs` pure helpers: file filter, listing, time format, duration parse.
- `crates/sda-core/tests/media_test.rs` and `tests/fixtures/ffmpeg_info.txt`.
- `crates/sda-media/Cargo.toml`, `src/lib.rs`, `src/probe.rs`, `src/decode.rs`, `src/audio.rs`, `src/clock.rs`, `src/player.rs`.
- `crates/sda-media/tests/fixtures/fake_ffmpeg.sh` and `tests/player_test.rs`.
- `crates/sda-render/tests/golden/video_contain.png` written on first run.

Modify
- `Cargo.toml` workspace members.
- `crates/sda-core/src/lib.rs` add `pub mod media;`.
- `crates/sda-core/src/resolve.rs` add `SlideContent::Video(PathBuf)`.
- `crates/sda-core/src/present.rs` add `Source::Video(PathBuf)`.
- `crates/sda-core/src/library.rs` add `video_dir` and `video_volume` to `Config`.
- `crates/sda-render/src/scene.rs` add `Layer::Video(Arc<FrameSource>)`.
- `crates/sda-render/src/composite.rs` add `contain`.
- `crates/sda-render/src/lib.rs` draw `Layer::Video`.
- `crates/sda-render/tests/golden_test.rs` one golden.
- `crates/sda-app/Cargo.toml` depend on `sda-media`, deb and rpm depends.
- `crates/sda-app/src/state/live.rs` player field and video paths.
- `crates/sda-app/src/msg/videos.rs`, `state/videos.rs`, `view/videos.rs` the presenting UI.
- `crates/sda-app/src/workers.rs` probe and open workers.
- `crates/sda-app/src/app.rs` end of file on tick, Clear on Videos.
- `crates/sda-core/src/i18n.rs` strings.
- `CHANGELOG.md`, `CLAUDE.md`.

---

### Task 1: Core media helpers

**Files:**
- Create: `crates/sda-core/src/media.rs`
- Create: `crates/sda-core/tests/media_test.rs`
- Create: `crates/sda-core/tests/fixtures/ffmpeg_info.txt`
- Modify: `crates/sda-core/src/lib.rs`

**Interfaces:**
- Produces: `media::is_video_file(&Path) -> bool`, `media::list_videos(&Path) -> Vec<PathBuf>`, `media::format_time(Duration) -> String`, `media::parse_duration(&str) -> Option<Duration>`, `media::parse_size(&str) -> Option<(u32, u32)>`, `media::STEP: Duration`.

- [ ] **Step 1: Write the failing tests**

`crates/sda-core/tests/fixtures/ffmpeg_info.txt`, a saved `ffmpeg -i clip.mp4` stderr:

```
ffmpeg version 7.1 Copyright (c) 2000-2024 the FFmpeg developers
Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'clip.mp4':
  Metadata:
    major_brand     : isom
  Duration: 00:03:10.52, start: 0.000000, bitrate: 2514 kb/s
  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], 2380 kb/s, 29.97 fps, 29.97 tbr, 30k tbn (default)
  Stream #0:1[0x2](und): Audio: aac (LC) (mp4a / 0x6134706D), 48000 Hz, stereo, fltp, 128 kb/s (default)
At least one output file must be specified
```

`crates/sda-core/tests/media_test.rs`:

```rust
use sda_core::media::*;
use std::path::Path;
use std::time::Duration;

#[test]
fn video_files_by_extension_any_case() {
    for ok in ["a.mp4", "b.MKV", "c.mov", "d.webm", "e.avi", "f.m4v"] {
        assert!(is_video_file(Path::new(ok)), "{ok}");
    }
    for no in ["a.mp3", "b.png", "c.pptx", "noext", ".mp4.part"] {
        assert!(!is_video_file(Path::new(no)), "{no}");
    }
}

#[test]
fn list_videos_sorts_by_name_and_skips_others() {
    let dir = tempfile::tempdir().unwrap();
    for f in ["zeta.mp4", "Alpha.mkv", "notes.txt", "beta.MOV"] {
        std::fs::write(dir.path().join(f), b"").unwrap();
    }
    std::fs::create_dir(dir.path().join("sub.mp4")).unwrap();
    let names: Vec<String> = list_videos(dir.path())
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["Alpha.mkv", "beta.MOV", "zeta.mp4"]);
    assert!(list_videos(Path::new("/definitely/missing")).is_empty());
}

#[test]
fn format_time_reads_like_a_player() {
    assert_eq!(format_time(Duration::ZERO), "0:00");
    assert_eq!(format_time(Duration::from_secs(42)), "0:42");
    assert_eq!(format_time(Duration::from_secs(190)), "3:10");
    assert_eq!(format_time(Duration::from_secs(3600 + 65)), "1:01:05");
    assert_eq!(format_time(Duration::from_millis(59_999)), "0:59");
}

#[test]
fn parse_duration_and_size_from_ffmpeg_stderr() {
    let info = include_str!("fixtures/ffmpeg_info.txt");
    let d = parse_duration(info).expect("duration line");
    assert!((d.as_secs_f64() - 190.52).abs() < 1e-3, "{d:?}");
    assert_eq!(parse_size(info), Some((1920, 1080)));
    assert_eq!(parse_duration("no duration here"), None);
    assert_eq!(parse_size("Video: h264, yuv420p, 2380 kb/s"), None);
    assert_eq!(parse_duration("  Duration: N/A, start: 0"), None);
}

#[test]
fn step_is_ten_seconds() {
    assert_eq!(STEP, Duration::from_secs(10));
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p sda-core --test media_test`
Expected: compile error, `sda_core::media` does not exist.

- [ ] **Step 3: Write the module**

`crates/sda-core/src/media.rs`:

```rust
//! Pure helpers for presenting video files. No process, no I/O beyond a
//! directory read. The player itself lives in the `sda-media` crate.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// How far one step moves, the arrow keys and the step buttons.
pub const STEP: Duration = Duration::from_secs(10);

const EXTENSIONS: [&str; 6] = ["mp4", "mkv", "mov", "webm", "avi", "m4v"];

/// True for the container extensions ffmpeg plays everywhere, any case.
pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| EXTENSIONS.contains(&e.as_str()))
}

/// Video files directly in `dir`, sorted by name without case. A missing
/// or unreadable dir gives an empty list.
pub fn list_videos(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_video_file(p))
        .collect();
    out.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    out
}

/// `m:ss`, or `h:mm:ss` from one hour. Whole seconds, rounded down.
pub fn format_time(d: Duration) -> String {
    let s = d.as_secs();
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m}:{sec:02}")
    }
}

/// The `Duration: hh:mm:ss.cc` line ffmpeg prints for an input.
pub fn parse_duration(stderr: &str) -> Option<Duration> {
    let line = stderr.lines().find(|l| l.trim_start().starts_with("Duration:"))?;
    let value = line.trim_start().strip_prefix("Duration:")?.trim();
    let value = value.split(',').next()?.trim();
    let mut parts = value.split(':');
    let h: u64 = parts.next()?.parse().ok()?;
    let m: u64 = parts.next()?.parse().ok()?;
    let s: f64 = parts.next()?.parse().ok()?;
    Some(Duration::from_secs_f64((h * 3600 + m * 60) as f64 + s))
}

/// The first `WxH` of the first video stream line, the coded picture size.
pub fn parse_size(stderr: &str) -> Option<(u32, u32)> {
    let line = stderr.lines().find(|l| l.contains("Video:"))?;
    line.split([' ', ','])
        .filter_map(|tok| {
            let (w, h) = tok.split_once('x')?;
            Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?))
        })
        .find(|(w, h)| *w >= 16 && *h >= 16)
}
```

Add to `crates/sda-core/src/lib.rs` next to the other modules, in alphabetical order:

```rust
pub mod media;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sda-core --test media_test`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/sda-core/src/media.rs crates/sda-core/src/lib.rs crates/sda-core/tests/media_test.rs crates/sda-core/tests/fixtures/ffmpeg_info.txt
git commit -m "feat(core): media helpers for video files, time, and ffmpeg info"
```

---

### Task 2: Core model, a video slide and source

**Files:**
- Modify: `crates/sda-core/src/resolve.rs:14-21`
- Modify: `crates/sda-core/src/present.rs:9-18`
- Modify: `crates/sda-core/src/library.rs:28-60`
- Test: `crates/sda-core/tests/resolve_test.rs`, `crates/sda-core/tests/present_test.rs` if present, else add the test to `resolve_test.rs`

**Interfaces:**
- Produces: `SlideContent::Video(PathBuf)`, `Source::Video(PathBuf)`, `Config.video_dir: Option<String>`, `Config.video_volume: f32` default `1.0`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/sda-core/tests/resolve_test.rs`:

```rust
#[test]
fn a_video_slide_resolves_to_nothing_like_a_bitmap() {
    let c = SlideContent::Video(std::path::PathBuf::from("/clips/intro.mp4"));
    assert!(resolve(&Theme::default(), &c, None).is_none());
}
```

Append to `crates/sda-core/tests/library_test.rs` if it exists, otherwise create it with these imports:

```rust
use sda_core::library::Config;

#[test]
fn config_video_fields_default_and_round_trip() {
    let c = Config::default();
    assert_eq!(c.video_dir, None);
    assert_eq!(c.video_volume, 1.0);
    let toml = toml::to_string(&Config {
        video_dir: Some("/clips".into()),
        video_volume: 0.4,
        ..Config::default()
    })
    .unwrap();
    let back: Config = toml::from_str(&toml).unwrap();
    assert_eq!(back.video_dir.as_deref(), Some("/clips"));
    assert_eq!(back.video_volume, 0.4);
    // An old config without the keys still loads.
    let old: Config = toml::from_str("hymns_url = \"x\"\nlibraries = []\n").unwrap();
    assert_eq!(old.video_volume, 1.0);
}
```

Check `toml` is a dependency of `sda-core` with `grep toml crates/sda-core/Cargo.toml`. It is, `Config::save` writes TOML.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p sda-core --test resolve_test --test library_test`
Expected: compile errors on `SlideContent::Video` and `video_dir`.

- [ ] **Step 3: Add the variants and fields**

`crates/sda-core/src/resolve.rs`, the enum:

```rust
/// What a presentation slide holds before resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum SlideContent {
    Styled(Slide),
    Verses {
        reference: String,
        verses: Vec<(u32, String)>,
    },
    Bitmap(PathBuf),
    /// A video file. Drawn by the player, no theme applies.
    Video(PathBuf),
}
```

and in `resolve`, the first match arm:

```rust
        SlideContent::Bitmap(_) | SlideContent::Video(_) => None,
```

`crates/sda-core/src/present.rs`:

```rust
pub enum Source {
    Hymn(PathBuf),
    Bible {
        version: String,
        reference: String,
    },
    Video(PathBuf),
    #[default]
    None,
}
```

`crates/sda-core/src/library.rs`, inside `Config` after `bible_version`:

```rust
    /// Folder the Videos tab lists. `None` => the download folder.
    #[serde(default)]
    pub video_dir: Option<String>,
    /// Playback volume, 0.0 to 1.0.
    #[serde(default = "full_volume")]
    pub video_volume: f32,
```

add near the other default fns:

```rust
fn full_volume() -> f32 {
    1.0
}
```

and in `impl Default for Config` add `video_dir: None, video_volume: 1.0,`.

- [ ] **Step 4: Fix exhaustive matches and run the whole workspace build**

Run: `cargo build --workspace 2>&1 | grep -E "^error" -A 8`

Every `match` on `SlideContent` with no wildcard fails now. Expected places and the arm to add:
- `crates/sda-app/src/state/bible.rs:~346`, the `flat_map` over slides, has `_ => Vec::new()`, fine.
- `crates/sda-app/src/view/bible.rs:~348`, has `Some(_) => ...`, fine.
- `crates/sda-core/src/bible/mod.rs` builds `Verses` only, fine.
- `crates/sda-app/src/state/live.rs:272` is an `if let`, fine.

If the build is clean, nothing else to touch. If an error names a file, add `SlideContent::Video(_) => ...` with the same behaviour as `Bitmap`.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p sda-core`
Expected: all green, including the two new tests.

- [ ] **Step 6: Commit**

```bash
git add crates/sda-core
git commit -m "feat(core): video slide content, video source, video config"
```

---

### Task 3: Renderer draws a video layer with contain

**Files:**
- Modify: `crates/sda-render/src/scene.rs:33-36`
- Modify: `crates/sda-render/src/composite.rs:59-73`
- Modify: `crates/sda-render/src/lib.rs` at `prepare`, `queue_draw`, `texture`
- Test: `crates/sda-render/tests/golden_test.rs`

**Interfaces:**
- Consumes: `FrameSource::new(size)`, `FrameSource::push(RgbaImage)`, `FrameSource::latest()` from `crates/sda-render/src/video.rs`.
- Produces: `Layer::Video(Arc<FrameSource>)`, `composite::contain(tex, out) -> [f32; 2]`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/sda-render/tests/golden_test.rs`:

```rust
/// A 4:3 frame on a 16:9 scene draws letterboxed: black at the sides,
/// the frame's color in the middle, nothing stretched.
#[test]
fn video_contain() {
    use sda_render::FrameSource;
    let src = FrameSource::new((120, 90));
    src.push(image::RgbaImage::from_pixel(120, 90, image::Rgba([0, 160, 255, 255])));
    let mut s = base();
    s.current = Some(Layer::Video(src));
    let Some(img) = headless::render_to_image(&s) else {
        return;
    };
    // Scene is 320 by 180. A 4:3 frame fitted by height is 240 wide, so
    // columns 0..40 and 280..320 stay black and the middle is blue.
    assert_eq!(img.get_pixel(10, 90)[2], 0, "left band black");
    assert_eq!(img.get_pixel(310, 90)[2], 0, "right band black");
    let mid = img.get_pixel(160, 90);
    assert!(mid[2] > 200 && mid[1] > 120, "frame color in the middle {mid:?}");
    check("video_contain", &s);
}

#[test]
fn video_layer_without_a_frame_draws_nothing() {
    use sda_render::FrameSource;
    let mut s = base();
    s.current = Some(Layer::Video(FrameSource::new((120, 90))));
    let Some(img) = headless::render_to_image(&s) else {
        return;
    };
    assert!((0..320).step_by(16).all(|x| img.get_pixel(x, 90)[2] == 0));
}
```

And a unit test for `contain` inside `crates/sda-render/src/composite.rs` in the existing `mod tests`:

```rust
    #[test]
    fn contain_fits_inside_and_cover_fills() {
        // 4:3 into 16:9: width shrinks to 0.75, height stays.
        let c = contain((400, 300), (1600, 900));
        assert!((c[0] - 0.75).abs() < 1e-5 && (c[1] - 1.0).abs() < 1e-5);
        // 21:9 into 16:9: height shrinks.
        let c = contain((2100, 900), (1600, 900));
        assert!((c[0] - 1.0).abs() < 1e-5 && (c[1] - 16.0 / 21.0).abs() < 1e-5);
        // Same aspect: no change.
        assert_eq!(contain((16, 9), (1920, 1080)), [1.0, 1.0]);
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p sda-render`
Expected: compile errors, `Layer::Video` and `contain` missing.

- [ ] **Step 3: Add the layer and contain**

`crates/sda-render/src/scene.rs`:

```rust
#[derive(Clone, Debug)]
pub enum Layer {
    Text(Arc<ResolvedSlide>),
    Bitmap(Arc<RgbaImage>),
    /// Live frames from a player. Drawn letterboxed over the background.
    Video(Arc<FrameSource>),
}
```

Add `use crate::video::FrameSource;` at the top of `scene.rs` if not imported.

`crates/sda-render/src/composite.rs` after `cover`:

```rust
/// Scale factors that fit `tex` inside `out` with its aspect kept. The
/// shorter side stays at 1.0, the other shrinks. The opposite of `cover`.
pub fn contain(tex: (u32, u32), out: (u32, u32)) -> [f32; 2] {
    let ta = tex.0 as f32 / tex.1.max(1) as f32;
    let oa = out.0 as f32 / out.1.max(1) as f32;
    if ta > oa {
        [1.0, oa / ta]
    } else {
        [ta / oa, 1.0]
    }
}
```

`crates/sda-render/src/lib.rs`. In `prepare`, replace the block that queues `scene.current` with one that unwraps a video layer to its latest frame first:

```rust
            if let Some(layer) = &scene.current {
                match layer {
                    Layer::Video(src) => {
                        if let Some(frame) = src.latest() {
                            let key = self.queue_draw(
                                device,
                                queue,
                                &mut p,
                                composite::CURRENT,
                                &Layer::Bitmap(frame),
                                cur_params,
                                composite::Fit::Contain,
                                scene.size,
                            );
                            if let Some(old) = p.video_key.replace(key)
                                && old != key
                            {
                                self.stale.push(old);
                            }
                        }
                    }
                    other => {
                        self.queue_draw(
                            device,
                            queue,
                            &mut p,
                            composite::CURRENT,
                            other,
                            cur_params,
                            composite::Fit::Cover,
                            scene.size,
                        );
                    }
                }
            }
```

`queue_draw` scales bitmaps with `cover` today. `LayerParams` is a
`bytemuck::Pod` uniform, so the choice cannot ride on it. Add an enum to
`composite.rs`:

```rust
/// How a bitmap meets the output: fill it and crop, or fit inside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Cover,
    Contain,
}
```

Give `queue_draw` one more argument, `fit: composite::Fit`, after
`params`. Pass `composite::Fit::Cover` at the outgoing call, the existing
current call, and inside `background_image`. Pass `composite::Fit::Contain`
from the video arm above, as already written. Then the scaling in
`queue_draw` becomes:

```rust
        let params = match (layer, fit) {
            (Layer::Text(_), _) | (Layer::Video(_), _) => params,
            (Layer::Bitmap(img), composite::Fit::Cover) => {
                composite::scaled(params, composite::cover(img.dimensions(), size))
            }
            (Layer::Bitmap(img), composite::Fit::Contain) => {
                composite::scaled(params, composite::contain(img.dimensions(), size))
            }
        };
```

In `texture`, the `match layer` for the hash and for the upload must handle `Layer::Video`. It never reaches there because `prepare` unwraps it, so:

```rust
            Layer::Video(src) => (Arc::as_ptr(src) as usize).hash(&mut h),
```

for the hash, and for the upload arm:

```rust
                Layer::Video(_) => unreachable!("video layers are unwrapped in prepare"),
```

Also the `video_key` eviction in `prepare` currently only clears when the background stops being video. Extend the condition so a key whose current layer stopped being a video also releases its frame:

```rust
        let shows_video = matches!(scene.background, BackgroundState::Video(_))
            || matches!(scene.current, Some(Layer::Video(_)));
        if !shows_video && let Some(old) = p.video_key.take() {
            self.stale.push(old);
        }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sda-render`
Expected: all green. `video_contain.png` is written on the first run. Open it and confirm a blue 4:3 block centered on black. Then run again to confirm it matches.

- [ ] **Step 5: Commit**

```bash
git add crates/sda-render
git commit -m "feat(render): video layer drawn letterboxed with contain"
```

---

### Task 4: The `sda-media` crate, probe and the fake ffmpeg

**Files:**
- Modify: `Cargo.toml` workspace members
- Create: `crates/sda-media/Cargo.toml`
- Create: `crates/sda-media/src/lib.rs`
- Create: `crates/sda-media/src/probe.rs`
- Create: `crates/sda-media/tests/fixtures/fake_ffmpeg.sh`
- Create: `crates/sda-media/tests/player_test.rs`

**Interfaces:**
- Consumes: `sda_core::media::{parse_duration, parse_size}`, `sda_render::video::hide_console` which must become `pub` in `crates/sda-render/src/video.rs` and be re-exported as `sda_render::hide_console`.
- Produces: `sda_media::probe::Info { duration: Duration, size: (u32, u32), has_audio: bool }`, `sda_media::probe::probe(ffmpeg: &Path, file: &Path) -> anyhow::Result<Info>`.

- [ ] **Step 1: Create the crate**

`Cargo.toml` at the root, members:

```toml
members = ["crates/sda-core", "crates/sda-render", "crates/sda-media", "crates/sda-app"]
```

`crates/sda-media/Cargo.toml`:

```toml
[package]
name = "sda-media"
edition = "2024"
version.workspace = true

[dependencies]
sda-core = { path = "../sda-core" }
sda-render = { path = "../sda-render" }
rodio = { version = "0.23", default-features = false, features = ["playback"] }
image = { version = "0.25", default-features = false }
log = "0.4"
anyhow = { workspace = true }

[dev-dependencies]
tempfile = "3"
```

`crates/sda-media/src/lib.rs`:

```rust
//! Video playback for presenting. Two ffmpeg children decode one file,
//! pictures as raw RGBA and sound as f32 PCM, both started at the same
//! time offset. The player paces pictures against the sound and feeds
//! frames into a `sda_render::FrameSource`. No Iced, no GPU here.

pub mod audio;
pub mod clock;
pub mod decode;
pub mod player;
pub mod probe;

pub use player::{Player, State};
pub use probe::Info;
```

Create empty `audio.rs`, `clock.rs`, `decode.rs`, `player.rs` with only a module doc line each so the crate compiles. Tasks 5 and 6 fill them.

In `crates/sda-render/src/video.rs` change `pub(crate) fn hide_console` to `pub fn hide_console` and add `pub use video::hide_console;` next to the existing `pub use video::...` line in `crates/sda-render/src/lib.rs`. Find it with `grep -n "pub use video" crates/sda-render/src/lib.rs`.

- [ ] **Step 2: Write the fake ffmpeg**

`crates/sda-media/tests/fixtures/fake_ffmpeg.sh`, then `chmod +x` it:

```sh
#!/bin/sh
# Stands in for ffmpeg in tests. Reads the arguments Lumen passes and
# answers like ffmpeg would, without decoding anything.
#
#   fake_ffmpeg.sh -i FILE                        info on stderr, exit 1
#   fake_ffmpeg.sh ... -f rawvideo ... -s WxH ... -   N RGBA frames on stdout
#   fake_ffmpeg.sh ... -f f32le ... -              N seconds of stereo PCM
#
# FAKE_FRAMES sets the frame count, default 6. FAKE_SECONDS the audio
# length, default 2. FAKE_FAIL=1 exits 1 with an error line at once.

if [ "$FAKE_FAIL" = "1" ]; then
  echo "clip.mp4: Invalid data found when processing input" >&2
  exit 1
fi

size=""
fmt=""
prev=""
for a in "$@"; do
  case "$prev" in
    -s) size="$a" ;;
    -f) fmt="$a" ;;
  esac
  prev="$a"
done

if [ -z "$fmt" ]; then
  cat >&2 <<INFO
Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'clip.mp4':
  Duration: 00:00:02.00, start: 0.000000, bitrate: 100 kb/s
  Stream #0:0: Video: h264, yuv420p, 160x90, 30 fps
  Stream #0:1: Audio: aac, 48000 Hz, stereo, fltp
At least one output file must be specified
INFO
  exit 1
fi

if [ "$fmt" = "rawvideo" ]; then
  w=${size%x*}
  h=${size#*x}
  n=${FAKE_FRAMES:-6}
  i=0
  while [ "$i" -lt "$n" ]; do
    # One frame of solid color, the red channel counts the frame.
    head -c $((w * h * 4)) /dev/zero | tr '\0' "$(printf '\\%03o' $((i + 1)))"
    i=$((i + 1))
  done
  exit 0
fi

if [ "$fmt" = "f32le" ]; then
  n=${FAKE_SECONDS:-2}
  head -c $((48000 * 2 * 4 * n)) /dev/zero
  exit 0
fi
echo "fake_ffmpeg: unknown output $fmt" >&2
exit 2
```

- [ ] **Step 3: Write the failing probe test**

`crates/sda-media/tests/player_test.rs`:

```rust
#![cfg(unix)]
//! Runs the fake ffmpeg script, never the real one. Unix only, the stub
//! is a shell script.

use std::path::PathBuf;
use std::time::Duration;

fn fake_ffmpeg() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fake_ffmpeg.sh"
    ))
}

#[test]
fn probe_reads_duration_size_and_audio() {
    let info = sda_media::probe::probe(&fake_ffmpeg(), std::path::Path::new("clip.mp4")).unwrap();
    assert_eq!(info.duration, Duration::from_secs(2));
    assert_eq!(info.size, (160, 90));
    assert!(info.has_audio);
}

#[test]
fn probe_fails_on_a_broken_file() {
    // SAFETY: tests in this file run on one thread per process start, the
    // variable is read by the child only.
    unsafe { std::env::set_var("FAKE_FAIL", "1") };
    let r = sda_media::probe::probe(&fake_ffmpeg(), std::path::Path::new("bad.mp4"));
    unsafe { std::env::remove_var("FAKE_FAIL") };
    let e = r.unwrap_err().to_string();
    assert!(e.contains("Invalid data"), "{e}");
}
```

The tests share one process and `FAKE_FAIL` is global, so every run of this crate's tests uses `--test-threads=1`.

- [ ] **Step 4: Run the test to see it fail**

Run: `cargo test -p sda-media --test player_test -- --test-threads=1`
Expected: compile error, `probe::probe` missing.

- [ ] **Step 5: Write probe**

`crates/sda-media/src/probe.rs`:

```rust
//! What a file is, from `ffmpeg -i file`. ffmpeg prints the streams on
//! stderr and exits 1 because no output was given. That is the only
//! probe this app needs, so no ffprobe.

use sda_core::media::{parse_duration, parse_size};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Info {
    pub duration: Duration,
    pub size: (u32, u32),
    pub has_audio: bool,
}

/// Run `ffmpeg -i file` and read the stream lines. Fails when ffmpeg is
/// missing or the file has no video stream, with ffmpeg's own words.
pub fn probe(ffmpeg: &Path, file: &Path) -> anyhow::Result<Info> {
    let mut cmd = Command::new(ffmpeg);
    sda_render::hide_console(&mut cmd);
    let out = cmd
        .args(["-hide_banner", "-i"])
        .arg(file)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()?;
    let text = String::from_utf8_lossy(&out.stderr);
    let Some(duration) = parse_duration(&text) else {
        let reason = text
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty() && !l.contains("At least one output"))
            .unwrap_or("no duration in ffmpeg output")
            .trim()
            .to_string();
        anyhow::bail!("{reason}");
    };
    let size = parse_size(&text).ok_or_else(|| anyhow::anyhow!("no video stream"))?;
    Ok(Info {
        duration,
        size,
        has_audio: text.contains("Audio:"),
    })
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p sda-media --test player_test -- --test-threads=1`
Expected: 2 passed.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/sda-media crates/sda-render/src/video.rs crates/sda-render/src/lib.rs
git commit -m "feat(media): new crate with an ffmpeg probe and a fake ffmpeg for tests"
```

---

### Task 5: Decoding, the clock, and the audio sink

**Files:**
- Create: `crates/sda-media/src/decode.rs`
- Create: `crates/sda-media/src/clock.rs`
- Create: `crates/sda-media/src/audio.rs`
- Test: `crates/sda-media/tests/player_test.rs`, unit tests inside `clock.rs` and `audio.rs`

**Interfaces:**
- Consumes: `sda_render::FrameSource`, `sda_render::hide_console`.
- Produces:
  - `decode::Timed { at: Duration, frame: RgbaImage }`
  - `decode::spawn_video(ffmpeg, file, start: Duration, out: (u32,u32)) -> Result<(Child, Receiver<Timed>)>` where `out` is the picture size from `fit_size`
  - `decode::spawn_audio(ffmpeg, file, start) -> Result<(Child, Receiver<Vec<f32>>)>` chunks of interleaved stereo f32 at 48 kHz
  - `decode::fit_size(src: (u32,u32), max: (u32,u32)) -> (u32,u32)` even numbers
  - `decode::FPS: u32 = 30`, `audio::RATE: u32 = 48_000`, `audio::CHANNELS: u16 = 2`
  - `clock::Clock` with `fn now(&self) -> Duration`, `fn pause(&self)`, `fn resume(&self)`, `fn paused(&self) -> bool`, two impls `WallClock` and `SampleClock`
  - `audio::Sink` with `Sink::open(start: Duration) -> Option<Sink>`, `Sink::feed(&self, chunk: Vec<f32>)`, `Sink::queued(&self) -> usize`, `Sink::set_volume(&self, v: f32)`, `Sink::clear(&self)`, `Sink::clock(&self) -> Arc<SampleClock>`, `Sink::pause(&self)`, `Sink::play(&self)`

- [ ] **Step 1: Write the failing tests**

Unit tests at the bottom of `crates/sda-media/src/clock.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn wall_clock_runs_and_pauses() {
        let c = WallClock::new(Duration::from_secs(5));
        sleep(Duration::from_millis(30));
        assert!(c.now() >= Duration::from_millis(5_030));
        c.pause();
        let at = c.now();
        sleep(Duration::from_millis(20));
        assert_eq!(c.now(), at);
        c.resume();
        sleep(Duration::from_millis(20));
        assert!(c.now() > at);
    }

    #[test]
    fn sample_clock_counts_samples_from_its_offset() {
        let c = SampleClock::new(Duration::from_secs(1), 48_000, 2);
        c.advance(48_000 * 2);
        assert_eq!(c.now(), Duration::from_secs(2));
        c.pause();
        c.advance(48_000);
        assert_eq!(c.now(), Duration::from_secs(2), "paused clocks ignore samples");
    }
}
```

Unit test at the bottom of `crates/sda-media/src/decode.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_size_keeps_aspect_and_evenness() {
        assert_eq!(fit_size((3840, 2160), (1920, 1080)), (1920, 1080));
        assert_eq!(fit_size((1280, 720), (1920, 1080)), (1280, 720), "never upscales");
        assert_eq!(fit_size((1920, 1440), (1920, 1080)), (1440, 1080));
        assert_eq!(fit_size((641, 361), (1920, 1080)), (640, 360), "even sizes for yuv");
    }
}
```

Append to `crates/sda-media/tests/player_test.rs`:

```rust
#[test]
fn spawn_video_delivers_timed_frames_in_order() {
    use sda_media::decode::{FPS, spawn_video};
    unsafe { std::env::set_var("FAKE_FRAMES", "4") };
    let (mut child, rx) = spawn_video(
        &fake_ffmpeg(),
        std::path::Path::new("clip.mp4"),
        Duration::from_secs(1),
        (160, 90),
    )
    .unwrap();
    let frames: Vec<_> = rx.iter().collect();
    unsafe { std::env::remove_var("FAKE_FRAMES") };
    let _ = child.wait();
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0].at, Duration::from_secs(1));
    assert_eq!(frames[1].at, Duration::from_secs(1) + Duration::from_secs(1) / FPS);
    assert_eq!(frames[0].frame.dimensions(), (160, 90));
    assert_eq!(frames[3].frame.get_pixel(0, 0)[0], 4, "the stub counts frames in red");
}

#[test]
fn spawn_audio_delivers_pcm_chunks() {
    use sda_media::audio::{CHANNELS, RATE};
    use sda_media::decode::spawn_audio;
    unsafe { std::env::set_var("FAKE_SECONDS", "1") };
    let (mut child, rx) = spawn_audio(
        &fake_ffmpeg(),
        std::path::Path::new("clip.mp4"),
        Duration::ZERO,
    )
    .unwrap();
    let total: usize = rx.iter().map(|c| c.len()).sum();
    unsafe { std::env::remove_var("FAKE_SECONDS") };
    let _ = child.wait();
    assert_eq!(total, RATE as usize * CHANNELS as usize);
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p sda-media -- --test-threads=1`
Expected: compile errors for the missing items.

- [ ] **Step 3: Write clock.rs**

```rust
//! The playback clock the picture follows. With sound it counts the
//! samples the device pulled, so picture and sound cannot drift. Without
//! sound it is the wall clock since play, so a silent box still plays.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub trait Clock: Send + Sync {
    /// Position in the file.
    fn now(&self) -> Duration;
    fn pause(&self);
    fn resume(&self);
    fn paused(&self) -> bool;
}

/// Wall clock from `start`, frozen while paused.
pub struct WallClock {
    start: Duration,
    state: Mutex<(Instant, Duration, bool)>,
}

impl WallClock {
    pub fn new(start: Duration) -> Self {
        WallClock {
            start,
            state: Mutex::new((Instant::now(), Duration::ZERO, false)),
        }
    }
}

impl Clock for WallClock {
    fn now(&self) -> Duration {
        let (since, banked, paused) = *self.state.lock().unwrap_or_else(|e| e.into_inner());
        let run = if paused { Duration::ZERO } else { since.elapsed() };
        self.start + banked + run
    }

    fn pause(&self) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !st.2 {
            st.1 += st.0.elapsed();
            st.2 = true;
        }
    }

    fn resume(&self) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.2 {
            st.0 = Instant::now();
            st.2 = false;
        }
    }

    fn paused(&self) -> bool {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).2
    }
}

/// Counts interleaved samples the audio device consumed.
pub struct SampleClock {
    start: Duration,
    rate: u32,
    channels: u16,
    samples: AtomicU64,
    paused: AtomicBool,
}

impl SampleClock {
    pub fn new(start: Duration, rate: u32, channels: u16) -> Self {
        SampleClock {
            start,
            rate,
            channels,
            samples: AtomicU64::new(0),
            paused: AtomicBool::new(false),
        }
    }

    /// The device pulled `n` interleaved samples. Ignored while paused,
    /// rodio keeps pulling silence then.
    pub fn advance(&self, n: u64) {
        if !self.paused.load(Ordering::Relaxed) {
            self.samples.fetch_add(n, Ordering::Relaxed);
        }
    }
}

impl Clock for SampleClock {
    fn now(&self) -> Duration {
        let frames = self.samples.load(Ordering::Relaxed) / u64::from(self.channels);
        self.start + Duration::from_secs_f64(frames as f64 / f64::from(self.rate))
    }

    fn pause(&self) {
        self.paused.store(true, Ordering::Relaxed);
    }

    fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }
}
```

- [ ] **Step 4: Write decode.rs**

```rust
//! The two ffmpeg children for one file. Each writes to a pipe, a thread
//! reads the pipe into a bounded channel. The channel's back pressure
//! throttles ffmpeg, so there is no `-re` and no burst after a pause.
//! Dropping the receiver ends the thread and the child dies on its next
//! write, the player also kills it outright on seek and stop.

use crate::audio::{CHANNELS, RATE};
use image::RgbaImage;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::Duration;

/// Constant output rate, so frame k sits at `start + k / FPS`.
pub const FPS: u32 = 30;
/// Frames waiting in the channel before ffmpeg blocks. A third of a second.
const QUEUE: usize = 10;
/// PCM chunk the audio thread hands over, 20 ms of stereo.
const CHUNK_SAMPLES: usize = (RATE as usize / 50) * CHANNELS as usize;

/// One decoded picture and its time in the file.
pub struct Timed {
    pub at: Duration,
    pub frame: RgbaImage,
}

/// `src` fitted inside `max`, aspect kept, never upscaled, both sides
/// even because ffmpeg's scaler wants that for yuv sources.
pub fn fit_size(src: (u32, u32), max: (u32, u32)) -> (u32, u32) {
    let (sw, sh) = (src.0.max(1) as f32, src.1.max(1) as f32);
    let k = (max.0 as f32 / sw).min(max.1 as f32 / sh).min(1.0);
    let even = |v: f32| ((v.round() as u32) / 2 * 2).max(2);
    (even(sw * k), even(sh * k))
}

fn seek_args(start: Duration) -> [String; 2] {
    ["-ss".into(), format!("{:.3}", start.as_secs_f64())]
}

/// Start ffmpeg decoding pictures from `start` at `out` pixels. Frames
/// arrive through the receiver with their time, `QUEUE` ahead at most.
pub fn spawn_video(
    ffmpeg: &Path,
    file: &Path,
    start: Duration,
    out: (u32, u32),
) -> anyhow::Result<(Child, Receiver<Timed>)> {
    let (w, h) = out;
    let mut cmd = Command::new(ffmpeg);
    sda_render::hide_console(&mut cmd);
    let mut child = cmd
        .args(["-hide_banner", "-loglevel", "error"])
        .args(seek_args(start))
        .arg("-i")
        .arg(file)
        .args(["-an", "-fps_mode", "cfr", "-r", &FPS.to_string()])
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{w}x{h}"), "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout not piped"))?;
    let (tx, rx): (SyncSender<Timed>, Receiver<Timed>) = sync_channel(QUEUE);
    std::thread::Builder::new()
        .name("video frames".into())
        .spawn(move || {
            let bytes = (w * h * 4) as usize;
            let mut k: u32 = 0;
            loop {
                let mut buf = vec![0u8; bytes];
                if stdout.read_exact(&mut buf).is_err() {
                    break;
                }
                let Some(frame) = RgbaImage::from_raw(w, h, buf) else { break };
                let at = start + Duration::from_secs(1) * k / FPS;
                if tx.send(Timed { at, frame }).is_err() {
                    break;
                }
                k += 1;
            }
        })?;
    Ok((child, rx))
}

/// Start ffmpeg decoding sound from `start` as interleaved stereo f32 at
/// `RATE`. Chunks of `CHUNK_SAMPLES` arrive through the receiver.
pub fn spawn_audio(
    ffmpeg: &Path,
    file: &Path,
    start: Duration,
) -> anyhow::Result<(Child, Receiver<Vec<f32>>)> {
    let mut cmd = Command::new(ffmpeg);
    sda_render::hide_console(&mut cmd);
    let mut child = cmd
        .args(["-hide_banner", "-loglevel", "error"])
        .args(seek_args(start))
        .arg("-i")
        .arg(file)
        .args(["-vn", "-f", "f32le", "-ac", &CHANNELS.to_string(), "-ar", &RATE.to_string(), "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("ffmpeg stdout not piped"))?;
    // Half a second of chunks ahead. Bigger means a longer pause lag.
    let (tx, rx) = sync_channel::<Vec<f32>>(25);
    std::thread::Builder::new()
        .name("audio pcm".into())
        .spawn(move || {
            let mut buf = vec![0u8; CHUNK_SAMPLES * 4];
            loop {
                let mut filled = 0;
                while filled < buf.len() {
                    match stdout.read(&mut buf[filled..]) {
                        Ok(0) => break,
                        Ok(n) => filled += n,
                        Err(_) => break,
                    }
                }
                if filled == 0 {
                    break;
                }
                let samples: Vec<f32> = buf[..filled - filled % 4]
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                    .collect();
                if tx.send(samples).is_err() || filled < buf.len() {
                    break;
                }
            }
        })?;
    Ok((child, rx))
}
```

- [ ] **Step 5: Write audio.rs**

The device side. A `Source` for rodio that drains a shared queue and
counts what it handed out, that count is the clock. When no device opens,
`Sink::open` returns `None` and the player falls back to `WallClock`.

```rust
//! Sound output through rodio. `Sink::open` returns `None` when the box
//! has no output device, the player then runs on the wall clock, muted.

use crate::clock::SampleClock;
use rodio::Source;
use rodio::stream::{DeviceSinkBuilder, MixerDeviceSink};
use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const RATE: u32 = 48_000;
pub const CHANNELS: u16 = 2;

/// Shared between the feeder and the device callback.
#[derive(Default)]
struct Queue {
    samples: VecDeque<f32>,
}

/// A rodio source that plays whatever `feed` put in and silence when the
/// queue runs dry, so the stream never ends on its own.
struct Pcm {
    queue: Arc<Mutex<Queue>>,
    clock: Arc<SampleClock>,
}

impl Iterator for Pcm {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.clock.paused() {
            return Some(0.0);
        }
        let sample = self
            .queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .samples
            .pop_front()
            .unwrap_or(0.0);
        self.clock.advance(1);
        Some(sample)
    }
}

impl Source for Pcm {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(CHANNELS).expect("two channels")
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(RATE).expect("48 kHz")
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub struct Sink {
    _stream: MixerDeviceSink,
    player: rodio::Player,
    queue: Arc<Mutex<Queue>>,
    clock: Arc<SampleClock>,
}

impl Sink {
    /// Open the default output device and start a silent stream whose
    /// clock begins at `start`. `None` when there is no usable device.
    pub fn open(start: Duration) -> Option<Sink> {
        // Tests and headless boxes opt out, the player then runs on the
        // wall clock.
        if std::env::var_os("LUMEN_NO_AUDIO").is_some() {
            return None;
        }
        let stream = match DeviceSinkBuilder::open_default_sink() {
            Ok(s) => s,
            Err(e) => {
                log::warn!("no audio output: {e}");
                return None;
            }
        };
        let queue = Arc::new(Mutex::new(Queue::default()));
        let clock = Arc::new(SampleClock::new(start, RATE, CHANNELS));
        let player = rodio::Player::connect_new(stream.mixer());
        player.append(Pcm {
            queue: queue.clone(),
            clock: clock.clone(),
        });
        Some(Sink {
            _stream: stream,
            player,
            queue,
            clock,
        })
    }

    pub fn feed(&self, chunk: Vec<f32>) {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .samples
            .extend(chunk);
    }

    /// Samples waiting to play. The feeder stops pulling from ffmpeg when
    /// this grows past a few hundred milliseconds.
    pub fn queued(&self) -> usize {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .samples
            .len()
    }

    pub fn clear(&self) {
        self.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .samples
            .clear();
    }

    pub fn set_volume(&self, v: f32) {
        self.player.set_volume(v.clamp(0.0, 1.0));
    }

    pub fn pause(&self) {
        self.clock.pause();
    }

    pub fn play(&self) {
        self.clock.resume();
    }

    pub fn clock(&self) -> Arc<SampleClock> {
        self.clock.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_source_plays_the_queue_then_silence_and_counts() {
        let queue = Arc::new(Mutex::new(Queue::default()));
        let clock = Arc::new(SampleClock::new(Duration::ZERO, RATE, CHANNELS));
        queue.lock().unwrap().samples.extend([0.5, -0.5]);
        let mut pcm = Pcm {
            queue,
            clock: clock.clone(),
        };
        assert_eq!(pcm.next(), Some(0.5));
        assert_eq!(pcm.next(), Some(-0.5));
        assert_eq!(pcm.next(), Some(0.0), "silence when dry");
        assert_eq!(clock.now(), Duration::from_secs_f64(3.0 / 2.0 / RATE as f64));
        clock.pause();
        assert_eq!(pcm.next(), Some(0.0));
        assert_eq!(clock.now(), Duration::from_secs_f64(3.0 / 2.0 / RATE as f64));
    }
}
```

Pause keeps the stream open and plays silence, so resume has no device
latency and the clock stays exact. A `Sink` is only created inside the
player, never in tests, so no test opens a device.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p sda-media -- --test-threads=1`
Expected: unit tests in `clock`, `decode`, `audio` pass, plus 4 in `player_test`.

- [ ] **Step 7: Commit**

```bash
git add crates/sda-media
git commit -m "feat(media): ffmpeg video and audio pipes, sample and wall clocks, rodio sink"
```

---

### Task 6: The Player

**Files:**
- Create: `crates/sda-media/src/player.rs`
- Test: `crates/sda-media/tests/player_test.rs`

**Interfaces:**
- Consumes: everything from Tasks 4 and 5, `sda_render::FrameSource`.
- Produces:

```rust
pub enum State { Playing, Paused, Ended }
pub struct Player { .. }
impl Player {
    pub fn open(ffmpeg: &Path, file: &Path, max: (u32, u32)) -> anyhow::Result<Player>;
    pub fn play(&self);
    pub fn pause(&self);
    pub fn toggle(&self);
    pub fn seek(&self, to: Duration);
    pub fn step(&self, by: i64);            // seconds, negative goes back
    pub fn set_volume(&self, v: f32);
    pub fn volume(&self) -> f32;
    pub fn muted(&self) -> bool;            // no audio device or no audio stream
    pub fn position(&self) -> Duration;
    pub fn duration(&self) -> Duration;
    pub fn state(&self) -> State;
    pub fn frames(&self) -> Arc<FrameSource>;
    pub fn file(&self) -> &Path;
    pub fn tick(&self);                     // move frames up to the clock, detect the end
}
```

`tick` is called by the app once per rendered frame. It keeps the player
free of threads that touch the frame source, which keeps the tests
deterministic.

- [ ] **Step 1: Write the failing tests**

Append to `crates/sda-media/tests/player_test.rs`:

```rust
use sda_media::{Player, State};

fn open_player() -> Player {
    unsafe { std::env::set_var("LUMEN_NO_AUDIO", "1") };
    Player::open(&fake_ffmpeg(), std::path::Path::new("clip.mp4"), (320, 180)).unwrap()
}

#[test]
fn open_starts_paused_at_zero_with_the_probe_info() {
    unsafe { std::env::set_var("FAKE_FRAMES", "6") };
    let p = open_player();
    unsafe { std::env::remove_var("FAKE_FRAMES") };
    assert_eq!(p.state(), State::Paused);
    assert_eq!(p.position(), Duration::ZERO);
    assert_eq!(p.duration(), Duration::from_secs(2));
    assert_eq!(p.frames().size(), (160, 90));
    // The first frame shows while paused so the preview is not black.
    for _ in 0..50 {
        p.tick();
        if p.frames().latest().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(p.frames().latest().is_some(), "first frame after open");
}

#[test]
fn play_advances_frames_with_the_clock_and_ends() {
    unsafe { std::env::set_var("FAKE_FRAMES", "6") };
    unsafe { std::env::set_var("FAKE_SECONDS", "0") };
    let p = open_player();
    unsafe { std::env::remove_var("FAKE_FRAMES") };
    unsafe { std::env::remove_var("FAKE_SECONDS") };
    // No device under `LUMEN_NO_AUDIO`, so the wall clock drives it.
    assert!(p.muted());
    p.play();
    assert_eq!(p.state(), State::Playing);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while p.state() == State::Playing && std::time::Instant::now() < deadline {
        p.tick();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(p.state(), State::Ended, "six frames at 30 fps end in 0.2 s");
    assert!(p.position() >= Duration::from_millis(190));
    assert!(p.frames().latest().is_none(), "ended means black");
}

#[test]
fn pause_freezes_position_and_seek_restarts_at_the_new_time() {
    unsafe { std::env::set_var("FAKE_FRAMES", "60") };
    let p = open_player();
    p.play();
    std::thread::sleep(Duration::from_millis(50));
    p.tick();
    p.pause();
    let at = p.position();
    std::thread::sleep(Duration::from_millis(50));
    p.tick();
    assert_eq!(p.position(), at);
    p.seek(Duration::from_millis(1500));
    assert_eq!(p.state(), State::Paused, "seek keeps the paused state");
    assert_eq!(p.position(), Duration::from_millis(1500));
    for _ in 0..50 {
        p.tick();
        if p.frames().latest().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let f = p.frames().latest().expect("a frame at the seek point");
    assert_eq!(f.get_pixel(0, 0)[0], 1, "the first frame after a restart");
    p.step(-10);
    assert_eq!(p.position(), Duration::ZERO, "clamped at the start");
    p.step(10);
    assert_eq!(p.position(), Duration::from_secs(2), "clamped at the end");
    unsafe { std::env::remove_var("FAKE_FRAMES") };
}

#[test]
fn volume_is_clamped_and_remembered() {
    let p = open_player();
    p.set_volume(1.7);
    assert_eq!(p.volume(), 1.0);
    p.set_volume(0.25);
    assert_eq!(p.volume(), 0.25);
}

#[test]
fn open_fails_with_ffmpegs_words() {
    unsafe { std::env::set_var("FAKE_FAIL", "1") };
    let r = Player::open(&fake_ffmpeg(), std::path::Path::new("bad.mp4"), (320, 180));
    unsafe { std::env::remove_var("FAKE_FAIL") };
    assert!(r.unwrap_err().to_string().contains("Invalid data"));
}
```

Every player test runs under `LUMEN_NO_AUDIO`, so `muted()` is always
true there and the wall clock drives playback. The rule for real use is
the same path: no device, or an audio stream that closes before its first
chunk, means wall clock and muted.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p sda-media -- --test-threads=1`
Expected: compile errors, `Player` missing.

- [ ] **Step 3: Write player.rs**

```rust
//! One file, two ffmpeg children, one clock. `open` probes and starts a
//! paused segment at zero. `seek` kills the segment and starts another at
//! the new time. `tick` moves frames into the `FrameSource` up to the
//! clock and notices the end. All methods take `&self`, the state lives
//! behind one mutex so the app can hold the player in an `Arc`.

use crate::audio::Sink;
use crate::clock::{Clock, WallClock};
use crate::decode::{self, Timed};
use crate::probe::{self, Info};
use sda_render::FrameSource;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Playing,
    Paused,
    Ended,
}

/// The running children and channels for one start offset.
struct Segment {
    start: Duration,
    video: Child,
    frames: Receiver<Timed>,
    /// Next frame pulled from the channel but not yet due.
    pending: Option<Timed>,
    audio: Option<(Child, Receiver<Vec<f32>>)>,
    sink: Option<Sink>,
    clock: Arc<dyn Clock>,
    /// True once the frame channel closed, the picture is over.
    frames_done: bool,
}

struct Inner {
    segment: Option<Segment>,
    state: State,
    volume: f32,
    /// Where a paused or ended player sits. A playing one asks its clock.
    frozen: Duration,
}

pub struct Player {
    ffmpeg: PathBuf,
    file: PathBuf,
    info: Info,
    out: (u32, u32),
    frames: Arc<FrameSource>,
    inner: Mutex<Inner>,
}

/// Samples kept ready in the sink before the feeder waits. 300 ms.
const AHEAD: usize = (crate::audio::RATE as usize * 3 / 10) * crate::audio::CHANNELS as usize;

impl Player {
    /// Probe `file`, then start a paused segment at zero so the first frame
    /// shows. `max` is the largest picture the output needs.
    pub fn open(ffmpeg: &Path, file: &Path, max: (u32, u32)) -> anyhow::Result<Player> {
        let info = probe::probe(ffmpeg, file)?;
        let out = decode::fit_size(info.size, max);
        let player = Player {
            ffmpeg: ffmpeg.to_path_buf(),
            file: file.to_path_buf(),
            info,
            out,
            frames: FrameSource::new(out),
            inner: Mutex::new(Inner {
                segment: None,
                state: State::Paused,
                volume: 1.0,
                frozen: Duration::ZERO,
            }),
        };
        player.restart(Duration::ZERO)?;
        Ok(player)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Kill the running segment and start one at `start`. The new clock
    /// is paused when the player is paused.
    fn restart(&self, start: Duration) -> anyhow::Result<()> {
        let mut inner = self.lock();
        if let Some(mut old) = inner.segment.take() {
            let _ = old.video.kill();
            let _ = old.video.wait();
            if let Some((mut a, _)) = old.audio.take() {
                let _ = a.kill();
                let _ = a.wait();
            }
        }
        let (video, frames) = decode::spawn_video(&self.ffmpeg, &self.file, start, self.out)?;
        let mut audio = None;
        let mut sink = None;
        if self.info.has_audio {
            match decode::spawn_audio(&self.ffmpeg, &self.file, start) {
                Ok((child, rx)) => {
                    // A stream that ends before one chunk has no sound to
                    // follow. Same for a box without an output device.
                    match rx.recv_timeout(Duration::from_millis(500)) {
                        Ok(first) => {
                            if let Some(s) = Sink::open(start) {
                                s.set_volume(inner.volume);
                                s.feed(first);
                                sink = Some(s);
                                audio = Some((child, rx));
                            }
                        }
                        Err(_) => {
                            let mut child = child;
                            let _ = child.kill();
                            let _ = child.wait();
                        }
                    }
                }
                Err(e) => log::warn!("audio decode failed, playing muted: {e}"),
            }
        }
        let clock: Arc<dyn Clock> = match &sink {
            Some(s) => s.clock(),
            None => Arc::new(WallClock::new(start)),
        };
        if inner.state != State::Playing {
            clock.pause();
            if let Some(s) = &sink {
                s.pause();
            }
        }
        inner.frozen = start;
        if inner.state == State::Ended {
            inner.state = State::Paused;
        }
        inner.segment = Some(Segment {
            start,
            video,
            frames,
            pending: None,
            audio,
            sink,
            clock,
            frames_done: false,
        });
        self.frames.push_none();
        Ok(())
    }

    pub fn play(&self) {
        let mut inner = self.lock();
        match inner.state {
            State::Playing => {}
            State::Paused => {
                inner.state = State::Playing;
                if let Some(seg) = &inner.segment {
                    seg.clock.resume();
                    if let Some(s) = &seg.sink {
                        s.play();
                    }
                }
            }
            State::Ended => {
                drop(inner);
                let _ = self.restart(Duration::ZERO);
                self.play();
            }
        }
    }

    pub fn pause(&self) {
        let mut inner = self.lock();
        if inner.state != State::Playing {
            return;
        }
        if let Some(seg) = &inner.segment {
            inner.frozen = seg.clock.now().min(self.info.duration);
            seg.clock.pause();
            if let Some(s) = &seg.sink {
                s.pause();
            }
        }
        inner.state = State::Paused;
    }

    pub fn toggle(&self) {
        if self.state() == State::Playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Restart at `to`, clamped to the file. Paused stays paused and shows
    /// the frame at `to`.
    pub fn seek(&self, to: Duration) {
        let to = to.min(self.info.duration);
        if let Err(e) = self.restart(to) {
            log::warn!("seek failed: {e}");
        }
    }

    /// `by` seconds from the current position, negative goes back.
    pub fn step(&self, by: i64) {
        let now = self.position().as_secs_f64();
        let to = (now + by as f64).clamp(0.0, self.info.duration.as_secs_f64());
        self.seek(Duration::from_secs_f64(to));
    }

    pub fn set_volume(&self, v: f32) {
        let v = v.clamp(0.0, 1.0);
        let inner = self.lock();
        if let Some(s) = inner.segment.as_ref().and_then(|s| s.sink.as_ref()) {
            s.set_volume(v);
        }
        drop(inner);
        self.lock().volume = v;
    }

    pub fn volume(&self) -> f32 {
        self.lock().volume
    }

    /// No sound plays: the file has none, or no device opened.
    pub fn muted(&self) -> bool {
        self.lock()
            .segment
            .as_ref()
            .is_none_or(|s| s.sink.is_none())
    }

    pub fn position(&self) -> Duration {
        let inner = self.lock();
        match (&inner.segment, inner.state) {
            (Some(seg), State::Playing) => seg.clock.now().min(self.info.duration),
            _ => inner.frozen,
        }
    }

    pub fn duration(&self) -> Duration {
        self.info.duration
    }

    pub fn state(&self) -> State {
        self.lock().state
    }

    pub fn frames(&self) -> Arc<FrameSource> {
        self.frames.clone()
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Feed the sink, show the newest frame that is due, and end when the
    /// pictures ran out. Call once per rendered frame.
    pub fn tick(&self) {
        let mut inner = self.lock();
        let state = inner.state;
        let Some(seg) = inner.segment.as_mut() else { return };

        // Sound first, so the clock never starves.
        if let (Some((_, rx)), Some(sink)) = (&seg.audio, &seg.sink) {
            while sink.queued() < AHEAD {
                match rx.try_recv() {
                    Ok(chunk) => sink.feed(chunk),
                    Err(_) => break,
                }
            }
        }

        // While paused, show one frame at the segment start and stop.
        let target = if state == State::Playing {
            seg.clock.now()
        } else {
            seg.start
        };
        let mut show: Option<Timed> = None;
        loop {
            let next = match seg.pending.take() {
                Some(t) => Some(t),
                None => match seg.frames.try_recv() {
                    Ok(t) => Some(t),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => {
                        seg.frames_done = true;
                        None
                    }
                },
            };
            match next {
                Some(t) if t.at <= target || show.is_none() && state != State::Playing => {
                    show = Some(t);
                    if state != State::Playing {
                        break;
                    }
                }
                Some(t) => {
                    seg.pending = Some(t);
                    break;
                }
                None => break,
            }
        }
        if let Some(t) = show {
            self.frames.push(t.frame);
        }

        // The end: the frame channel closed and nothing waits to be shown.
        if state == State::Playing && seg.frames_done && seg.pending.is_none() {
            inner.frozen = self.info.duration;
            inner.state = State::Ended;
            if let Some(seg) = inner.segment.as_mut() {
                seg.clock.pause();
                if let Some(s) = &seg.sink {
                    s.pause();
                    s.clear();
                }
            }
            self.frames.push_none();
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let mut inner = self.lock();
        if let Some(mut seg) = inner.segment.take() {
            let _ = seg.video.kill();
            let _ = seg.video.wait();
            if let Some((mut a, _)) = seg.audio.take() {
                let _ = a.kill();
                let _ = a.wait();
            }
        }
    }
}
```

`FrameSource` needs one new method in `crates/sda-render/src/video.rs`,
next to `push`:

```rust
    /// Forget the current frame, so the next draw shows nothing.
    pub fn push_none(&self) {
        *self.latest.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
```

A note on the paused frame rule in `tick`: when paused, the loop takes
the first frame it can get and stops, so a seek shows its target frame
without playing on. When playing, it drains every frame whose time is at
or under the clock and shows the last one, which drops frames when the
machine falls behind instead of slowing the picture.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sda-media -- --test-threads=1`
Expected: all green, 9 in `player_test` plus the unit tests. If
`play_advances_frames_with_the_clock_and_ends` is slow or flaky, check
that `FAKE_SECONDS=0` made `spawn_audio` close its channel before the
500 ms `recv_timeout`, the fake writes zero bytes and exits.

- [ ] **Step 5: Run clippy on the crate**

Run: `cargo clippy -p sda-media --all-targets -- -D warnings`
Expected: clean. `is_none_or` needs Rust 1.82, fine on 1.88.

- [ ] **Step 6: Commit**

```bash
git add crates/sda-media crates/sda-render/src/video.rs
git commit -m "feat(media): player with play, pause, seek, step, volume, and end of file"
```

---

### Task 7: Live state plays video

**Files:**
- Modify: `crates/sda-app/Cargo.toml`
- Modify: `crates/sda-app/src/state/live.rs`
- Modify: `crates/sda-app/src/app.rs` at `Msg::Tick`
- Test: `crates/sda-app/tests/views_test.rs`

**Interfaces:**
- Consumes: `sda_media::{Player, State}`, `SlideContent::Video`, `Source::Video`, `Layer::Video`.
- Produces: `LiveState.player: Option<Arc<Player>>`, `LiveState::go_video(player: Arc<Player>, title: String)`, `LiveState::video_playing() -> bool`.

- [ ] **Step 1: Add the dependency**

In `crates/sda-app/Cargo.toml` under `[dependencies]` after `sda-render`:

```toml
sda-media = { path = "../sda-media" }
```

- [ ] **Step 2: Write the failing tests**

Append to `crates/sda-app/tests/views_test.rs`. These tests cannot open a
real player without ffmpeg, so they test the wiring that does not need
one, and the one place that does is `#[cfg(unix)]` with the fake script.

```rust
#[test]
fn a_video_presentation_has_one_slide_and_no_theme_footer() {
    use sda_core::present::{Presentation, Source};
    use sda_core::resolve::SlideContent;
    let mut app = app();
    let path = std::path::PathBuf::from("/clips/intro.mp4");
    app.live.go(Presentation {
        title: "intro.mp4".into(),
        subtitle: None,
        slides: vec![SlideContent::Video(path.clone())],
        index: 0,
        blank: false,
        source: Source::Video(path),
    });
    assert!(app.live.present.is_loaded());
    assert_eq!(app.live.present.slide_count(), 1);
    // No player yet, so the scene draws nothing in the current slot and
    // a black background whatever the theme says.
    let scene = app.live.scene((160, 90), std::time::Instant::now());
    assert!(scene.current.is_none());
    assert!(matches!(
        scene.background,
        sda_render::BackgroundState::Solid(c) if c == sda_core::theme::Rgba::BLACK
    ));
}

#[cfg(unix)]
#[test]
fn live_video_seeks_on_next_and_prev_and_pauses_on_blank() {
    use sda_media::{Player, State};
    use std::sync::Arc;
    use std::time::Duration;
    let fake = std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../sda-media/tests/fixtures/fake_ffmpeg.sh"
    ));
    unsafe { std::env::set_var("LUMEN_NO_AUDIO", "1") };
    unsafe { std::env::set_var("FAKE_FRAMES", "60") };
    let player = Arc::new(
        Player::open(&fake, std::path::Path::new("clip.mp4"), (320, 180)).unwrap(),
    );
    unsafe { std::env::remove_var("FAKE_FRAMES") };
    let mut app = app();
    app.live.go_video(player.clone(), "clip.mp4".into());
    assert_eq!(player.state(), State::Playing, "going live starts playback");
    assert!(app.live.animating(), "a playing video keeps the frame clock on");
    let _ = app.update(Msg::LiveNext);
    assert_eq!(player.position(), Duration::from_secs(2), "10 s forward clamps to the end");
    let _ = app.update(Msg::LivePrev);
    assert_eq!(player.position(), Duration::ZERO);
    let _ = app.update(Msg::LiveBlank);
    assert!(app.live.present.blank);
    assert_eq!(player.state(), State::Paused, "blank pauses the sound too");
    let _ = app.update(Msg::LiveBlank);
    assert_eq!(player.state(), State::Playing);
    let scene = app.live.scene((160, 90), std::time::Instant::now());
    assert!(matches!(scene.current, Some(sda_render::Layer::Video(_))));
    let _ = app.update(Msg::LiveStop);
    assert!(app.live.player.is_none(), "stop drops the player");
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p sda-app --test views_test video`
Expected: compile errors, `go_video` and `player` missing. The first test
also fails on the background assertion until the video path exists.

- [ ] **Step 4: Wire the live state**

`crates/sda-app/src/state/live.rs`. Add the field and the imports:

```rust
use sda_media::{Player, State as PlayState};
```

In `LiveState`:

```rust
    /// The video that is live. Set by `go_video`, dropped by `stop` and by
    /// any `go` of other content, which kills its ffmpeg children.
    pub player: Option<Arc<Player>>,
```

Initialize `player: None` in `new`. Then add after `go`:

```rust
    /// Put `player` live and start it. One video slide, no theme.
    pub fn go_video(&mut self, player: Arc<Player>, title: String) {
        let path = player.file().to_path_buf();
        self.go(Presentation {
            title,
            subtitle: None,
            slides: vec![SlideContent::Video(path.clone())],
            index: 0,
            blank: false,
            source: Source::Video(path),
        });
        player.play();
        self.player = Some(player);
    }

    /// True while the live video plays. The frame clock must run then.
    pub fn video_playing(&self) -> bool {
        self.player
            .as_ref()
            .is_some_and(|p| p.state() == PlayState::Playing)
    }

    /// Per frame: pull frames up to the clock. The projector goes black at
    /// the end on its own because the player clears its frame source.
    pub fn tick_video(&self) {
        if let Some(p) = &self.player {
            p.tick();
        }
    }
```

Add `use sda_core::present::Source;` to the imports.

In `go`, drop a previous player first, as the first line:

```rust
        self.player = None;
```

In `next` and `prev`, branch before anything else:

```rust
        if let Some(p) = &self.player {
            p.step(10);
            return;
        }
```

with `p.step(-10)` in `prev`. In `blank`, after `toggle_blank()`:

```rust
        if let Some(p) = &self.player {
            if self.present.blank {
                p.pause();
            } else {
                p.play();
            }
        }
```

In `stop`, before the early return check is fine, add `self.player = None;`
as the first line.

In `animating`:

```rust
        self.started.is_some() || self.theme.background.is_animated() || self.video_playing()
```

In `layer`, before the bitmap check:

```rust
        if let SlideContent::Video(_) = content {
            return self.player.as_ref().map(|p| Layer::Video(p.frames()));
        }
```

In `scene`, the background and opacity ignore the theme for a video:

```rust
        let is_video = matches!(self.present.current(), Some(SlideContent::Video(_)));
        let background = if is_video {
            BackgroundState::Solid(Rgba::BLACK)
        } else {
            self.background()
        };
        let background_opacity = if is_video { 1.0 } else { self.theme.background_opacity };
```

and use `background` and `background_opacity` in the `Scene` literal. Also
the footer: `footer()` is only read through `resolve` for text slides, and
a video never resolves, so nothing to do there.

In `crates/sda-app/src/app.rs` at `Msg::Tick`:

```rust
            Msg::Tick(now) => {
                self.live.tick_video();
                projector::tick(self, now);
            }
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p sda-app --test views_test`
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add crates/sda-app/Cargo.toml Cargo.lock crates/sda-app/src/state/live.rs crates/sda-app/src/app.rs crates/sda-app/tests/views_test.rs
git commit -m "feat(app): live state plays a video, arrows seek, blank pauses"
```

---

### Task 8: Videos tab, open a file, preview, transport, present

**Files:**
- Modify: `crates/sda-app/src/msg/videos.rs`
- Modify: `crates/sda-app/src/state/videos.rs`
- Modify: `crates/sda-app/src/view/videos.rs`
- Modify: `crates/sda-app/src/workers.rs`
- Modify: `crates/sda-app/src/app.rs` at `subscription`, `Msg::Tick`, `Msg::Clear`
- Modify: `crates/sda-app/src/shortcuts.rs` scope of `Action::Present`
- Modify: `crates/sda-core/src/i18n.rs` through the helper below
- Modify: `crates/sda-core/tests/i18n_test.rs` field count, the helper does it
- Test: `crates/sda-app/tests/videos_test.rs`

**Interfaces:**
- Consumes: `sda_media::Player`, `LiveState::go_video`, `sda_core::media::{STEP, format_time}`, `sda_core::downloader::ensure_ffmpeg`.
- Produces: `VideosMsg::{OpenFile, FileChosen(Option<PathBuf>), Opened(Result<Arc<Player>, String>), Toggle, Seek(f32), Step(i64), Volume(f32), Present}`, `Msg::LiveSpace`, `shortcuts::Action::Space`, `VideosState::{picked, player, opening, error, volume}`, `workers::open_video(file: PathBuf, max: (u32, u32)) -> Task<Msg>`, `videos::preview_playing(app) -> bool`, `videos::tick(app)`.

- [ ] **Step 1: Add the strings**

Save this helper as `/tmp/addstr.py`. It inserts a field, its `as_fields`
entry, and the three translations after `history_empty`, and bumps the
field count in `i18n_test.rs`.

```python
import sys, json, re
p='crates/sda-core/src/i18n.rs'
s=open(p).read()
items=json.load(open(sys.argv[1]))
def ins(anchor, text):
    global s
    assert s.count(anchor)==1,(anchor,s.count(anchor))
    s=s.replace(anchor, anchor+text)
ins("    pub history_empty: String,\n", ''.join(f'    pub {n}: String,\n' for n,_,_,_ in items))
ins("            &self.history_empty,\n", ''.join(f'            &self.{n},\n' for n,_,_,_ in items))
for i, marker in [(1,'history_empty: "No hymns opened yet".into(),\n'),(2,'history_empty: "Nessun inno aperto".into(),\n'),(3,'history_empty: "Niciun imn deschis".into(),\n')]:
    ins("            "+marker, ''.join(f'            {it[0]}: {json.dumps(it[i], ensure_ascii=False)}.into(),\n' for it in items))
open(p,'w').write(s)
t='crates/sda-core/tests/i18n_test.rs'
ts=open(t).read()
m=re.search(r'as_fields\(\)\.len\(\), (\d+)\)', ts)
ts=ts.replace(m.group(0), f'as_fields().len(), {int(m.group(1))+len(items)})')
open(t,'w').write(ts)
```

Strings file `/tmp/s_video.json`:

```json
[
["videos_open_file","Open file…","Apri file…","Deschide fișier…"],
["videos_pick_hint","Open a video file to preview it.","Apri un file video per vederlo in anteprima.","Deschide un fișier video pentru previzualizare."],
["videos_opening","Opening…","Apertura…","Se deschide…"],
["play","Play","Riproduci","Redă"],
["pause","Pause","Pausa","Pauză"],
["volume","Volume","Volume","Volum"],
["videos_muted_hint","No audio output, playing without sound.","Nessuna uscita audio, riproduzione senza suono.","Fără ieșire audio, redare fără sunet."],
["videos_ended","Ended","Terminato","Încheiat"],
["sc_space","Next slide, or play and pause a live video","Diapositiva successiva, o riproduci e metti in pausa un video in onda","Diapozitivul următor, sau redă și pune pauză unui video în direct"]
]
```

Run: `python3 /tmp/addstr.py /tmp/s_video.json && cargo test -p sda-core --test i18n_test`
Expected: tests pass with the field count raised by 9.

- [ ] **Step 2: Write the failing tests**

`crates/sda-app/tests/videos_test.rs`:

```rust
use iced_test::Simulator;
use sda_app::msg::VideosMsg;
use sda_app::{App, Msg, Tab};
use std::path::PathBuf;

fn app() -> App {
    let mut app = App::for_test();
    let _ = app.update(Msg::Tab(Tab::Videos));
    app
}

#[test]
fn empty_tab_offers_open_file_and_no_transport() {
    let app = app();
    let mut ui = Simulator::new(app.view_operator());
    let s = &app.core.strings;
    assert!(ui.find(&*s.videos_open_file).is_ok());
    assert!(ui.find(&*s.videos_pick_hint).is_ok());
    assert!(ui.find(&*s.play).is_err());
    assert!(ui.find(&*s.videos_download.to_uppercase()).is_ok());
}

#[test]
fn choosing_a_file_marks_it_opening_and_a_failure_shows_the_reason() {
    let mut app = app();
    let path = PathBuf::from("/clips/intro.mp4");
    let _ = app.update(Msg::Videos(VideosMsg::FileChosen(Some(path.clone()))));
    assert_eq!(app.videos.picked.as_deref(), Some(path.as_path()));
    assert!(app.videos.opening);
    let _ = app.update(Msg::Videos(VideosMsg::Opened(Err("Invalid data".into()))));
    assert!(!app.videos.opening);
    assert_eq!(app.videos.error.as_deref(), Some("Invalid data"));
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find("Invalid data").is_ok());
    assert!(ui.find("intro.mp4").is_ok());
}

#[test]
fn volume_is_saved_to_config() {
    let mut app = app();
    let _ = app.update(Msg::Videos(VideosMsg::Volume(0.3)));
    assert_eq!(app.videos.volume, 0.3);
    assert_eq!(app.core.cfg.video_volume, 0.3);
}

#[cfg(unix)]
mod with_fake_ffmpeg {
    use super::*;
    use sda_media::{Player, State};
    use std::sync::Arc;
    use std::time::Duration;

    fn fake() -> PathBuf {
        PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sda-media/tests/fixtures/fake_ffmpeg.sh"
        ))
    }

    fn opened_app() -> (App, Arc<Player>) {
        unsafe { std::env::set_var("LUMEN_NO_AUDIO", "1") };
        unsafe { std::env::set_var("FAKE_FRAMES", "60") };
        let player = Arc::new(
            Player::open(&fake(), std::path::Path::new("clip.mp4"), (320, 180)).unwrap(),
        );
        unsafe { std::env::remove_var("FAKE_FRAMES") };
        let mut app = app();
        let _ = app.update(Msg::Videos(VideosMsg::FileChosen(Some(PathBuf::from("clip.mp4")))));
        let _ = app.update(Msg::Videos(VideosMsg::Opened(Ok(player.clone()))));
        (app, player)
    }

    #[test]
    fn transport_drives_the_preview_player() {
        let (mut app, player) = opened_app();
        assert!(!app.videos.opening);
        assert_eq!(player.state(), State::Paused);
        let _ = app.update(Msg::Videos(VideosMsg::Toggle));
        assert_eq!(player.state(), State::Playing);
        assert!(sda_app::state::videos::preview_playing(&app));
        let _ = app.update(Msg::Videos(VideosMsg::Toggle));
        assert_eq!(player.state(), State::Paused);
        let _ = app.update(Msg::Videos(VideosMsg::Seek(1.5)));
        assert_eq!(player.position(), Duration::from_millis(1500));
        let _ = app.update(Msg::Videos(VideosMsg::Step(-10)));
        assert_eq!(player.position(), Duration::ZERO);
        let _ = app.update(Msg::Videos(VideosMsg::Volume(0.5)));
        assert_eq!(player.volume(), 0.5);
        let mut ui = Simulator::new(app.view_operator());
        assert!(ui.find("0:00 / 0:02").is_ok());
    }

    #[test]
    fn present_puts_the_same_player_live_and_enter_does_too() {
        let (mut app, player) = opened_app();
        let _ = app.update(Msg::Videos(VideosMsg::Present));
        assert!(Arc::ptr_eq(app.live.player.as_ref().unwrap(), &player));
        assert_eq!(player.state(), State::Playing);
        assert_eq!(app.live.present.title, "clip.mp4");
        assert!(matches!(
            app.live.present.source,
            sda_core::present::Source::Video(ref p) if p.ends_with("clip.mp4")
        ));
        // Enter on the Videos tab is the same action.
        use iced::keyboard::{self, Key, key::Named};
        let enter = iced::Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(Named::Enter),
            modified_key: Key::Named(Named::Enter),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        });
        assert!(matches!(
            sda_app::shortcuts::filter(enter, iced::event::Status::Ignored, iced::window::Id::unique(), Tab::Videos),
            Some(Msg::Videos(VideosMsg::Present))
        ));
        // Space pauses the live video, a second Space resumes it.
        let _ = app.update(Msg::LiveSpace);
        assert_eq!(player.state(), State::Paused);
        let _ = app.update(Msg::LiveSpace);
        assert_eq!(player.state(), State::Playing);
    }
}
```

`sda-media` must be a dev dependency of `sda-app` for this test. It is
already a dependency from Task 7, so nothing to add.

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p sda-app --test videos_test`
Expected: compile errors on the new messages and fields.

- [ ] **Step 4: Messages and state**

`crates/sda-app/src/msg/videos.rs`, add to the enum:

```rust
    /// Open the file picker for a video to present.
    OpenFile,
    FileChosen(Option<PathBuf>),
    /// The player for the picked file, or why it could not open.
    Opened(Result<Arc<sda_media::Player>, String>),
    /// Play or pause the preview player, which is the live one once presented.
    Toggle,
    /// Seek to a position in seconds, from the slider.
    Seek(f32),
    /// Step by seconds, the arrows and the step buttons.
    Step(i64),
    Volume(f32),
    /// Send the preview player live.
    Present,
```

Add `use std::sync::Arc;` at the top. `Msg` derives `Debug`, so
`sda_media::Player` needs `Debug`. Add to `crates/sda-media/src/player.rs`:

```rust
impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Player({})", self.file.display())
    }
}
```

`crates/sda-app/src/state/videos.rs`. Fields on `VideosState`:

```rust
    /// The file chosen to present.
    pub picked: Option<PathBuf>,
    /// The preview player. `Present` hands this same `Arc` to the live state.
    pub player: Option<Arc<sda_media::Player>>,
    pub opening: bool,
    /// Why the last open failed, shown under the picker.
    pub error: Option<String>,
    /// Playback volume, 0.0 to 1.0, from the config.
    pub volume: f32,
```

`VideosState` derives `Default`, which gives `volume` 0.0. Replace the
derive with a manual `impl Default` that sets `volume: 1.0` and every other
field to its default. Where the app loads config into state at boot, find
it with `grep -n "download_dir" crates/sda-app/src/app.rs crates/sda-app/src/state/videos.rs`,
set `app.videos.volume = app.core.cfg.video_volume` in `App::new` right
after `videos: videos::VideosState::default(),` is built, through a
`let mut app = App { .. }; app.videos.volume = app.core.cfg.video_volume; app`.

Add `use std::sync::Arc;` and the handlers inside `update`, in the match:

```rust
        VideosMsg::OpenFile => {
            let start = v.picked.as_ref().and_then(|p| p.parent().map(Path::to_path_buf))
                .unwrap_or_else(|| dir(app));
            return Task::perform(
                async move {
                    rfd::AsyncFileDialog::new()
                        .set_directory(start)
                        .add_filter("Video", &["mp4", "mkv", "mov", "webm", "avi", "m4v"])
                        .pick_file()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                |p| Msg::Videos(VideosMsg::FileChosen(p)),
            );
        }
        VideosMsg::FileChosen(None) => {}
        VideosMsg::FileChosen(Some(path)) => {
            v.picked = Some(path.clone());
            v.player = None;
            v.error = None;
            v.opening = true;
            return crate::workers::open_video(path, projector_size(app));
        }
        VideosMsg::Opened(result) => {
            v.opening = false;
            match result {
                Ok(player) => {
                    player.set_volume(v.volume);
                    v.player = Some(player);
                }
                Err(e) => v.error = Some(e),
            }
        }
        VideosMsg::Toggle => {
            if let Some(p) = &v.player {
                p.toggle();
            }
        }
        VideosMsg::Seek(secs) => {
            if let Some(p) = &v.player {
                p.seek(std::time::Duration::from_secs_f32(secs.max(0.0)));
            }
        }
        VideosMsg::Step(by) => {
            if let Some(p) = &v.player {
                p.step(by);
            }
        }
        VideosMsg::Volume(vol) => {
            v.volume = vol.clamp(0.0, 1.0);
            if let Some(p) = &v.player {
                p.set_volume(v.volume);
            }
            app.core.cfg.video_volume = v.volume;
            app.core.save_cfg();
        }
        VideosMsg::Present => {
            if let Some(p) = v.player.clone() {
                let title = p
                    .file()
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                app.live.go_video(p, title);
            }
        }
```

`v` is the existing `let v = &mut app.videos;` at the top of `update`. The
`Present` arm borrows `app.live` while `v` is alive, so take
`let player = v.player.clone();` before touching `app.live`, as written.

Two helpers at module level:

```rust
/// The largest picture the live output needs, the chosen display or 1080p.
fn projector_size(app: &App) -> (u32, u32) {
    app.projector_display
        .and_then(|i| app.displays.get(i))
        .map(|d| {
            let (_, _, w, h) = d.rect;
            ((w as f32 * d.scale) as u32, (h as f32 * d.scale) as u32)
        })
        .unwrap_or((1920, 1080))
}

/// True while the preview plays and is not the live player, which the
/// live state already keeps the frame clock on for.
pub fn preview_playing(app: &App) -> bool {
    app.videos
        .player
        .as_ref()
        .is_some_and(|p| p.state() == sda_media::State::Playing)
}

/// Per frame. The live state ticks the same player when it is live, a
/// second tick in one frame is harmless.
pub fn tick(app: &App) {
    if let Some(p) = &app.videos.player {
        p.tick();
    }
}
```

`crates/sda-app/src/workers.rs`:

```rust
/// Open `file` in a player sized for `max`. Downloads ffmpeg first when it
/// is missing, the same way the downloader does.
pub fn open_video(file: PathBuf, max: (u32, u32)) -> Task<Msg> {
    Task::perform(
        blocking(move || {
            let ffmpeg = downloader::ensure_ffmpeg().map_err(|e| e.to_string())?;
            sda_media::Player::open(&ffmpeg, &file, max)
                .map(std::sync::Arc::new)
                .map_err(|e| e.to_string())
        }),
        |r| Msg::Videos(crate::msg::VideosMsg::Opened(r.and_then(|inner| inner))),
    )
}
```

`crates/sda-app/src/app.rs`:
- `subscription`: `if self.live.animating() || state::videos::preview_playing(self)`.
- `Msg::Tick`: call `state::videos::tick(self);` next to `self.live.tick_video();`.

`crates/sda-app/src/shortcuts.rs`: rename `Scope::PresentAndBible` to
`Scope::Content` covering `Tab::Present | Tab::Bible | Tab::Videos`, and
in `message`:

```rust
        Action::Present if tab == Tab::Bible => Msg::Bible(BibleMsg::Present),
        Action::Present if tab == Tab::Videos => Msg::Videos(VideosMsg::Present),
        Action::Present => Msg::GoLive,
```

with `use crate::msg::VideosMsg;` added. Update the test
`enter_presents_on_present_and_bible_only` in `views_test.rs`: `Tab::Videos`
moves from the ignoring list to its own assertion on
`Msg::Videos(VideosMsg::Present)`.

Space must pause a live video and still advance a hymn. Split it from
`Action::Next`: add `Action::Space` to the enum, change the Space row of
`TABLE` to `sc(Action::Space, KeyDef::Named(Named::Space), false, Scope::Everywhere)`,
map it in `message` to a new `Msg::LiveSpace`, give it `label` `&s.sc_space`,
and add `Msg::LiveSpace` to `msg.rs` next to `LiveNext` with the doc
"Space. Pauses a live video, else the next slide." In `app.rs`:

```rust
            Msg::LiveSpace => match &self.live.player {
                Some(p) => p.toggle(),
                None => self.live.next(),
            },
```

The Next button keeps `Action::Next`, whose first key is now → alone. The
sheet gains a Space row from the table.

- [ ] **Step 5: The view**

Replace `show` in `crates/sda-app/src/view/videos.rs` and add the two
new panes. The downloader card and `history` stay as they are, they move
to the right column.

```rust
use crate::shortcuts::Action;
use crate::widgets::hint::labeled;
use crate::widgets::icon::{Icon, icon};
use crate::widgets::scene_view::{key, scene_view};
use iced::widget::slider;
use sda_core::media::format_time;
use sda_media::State;

const PICKER_W: f32 = 260.0;
const SIDE_W: f32 = 340.0;

pub fn show(app: &App) -> Element<'_, Msg> {
    let side = column![downloader(app), history(app)]
        .spacing(ui_theme::SPACING)
        .width(SIDE_W)
        .height(Fill);
    row![picker(app), preview(app), side]
        .spacing(ui_theme::SPACING)
        .padding([0, ui_theme::SPACING as u16])
        .height(Fill)
        .into()
}

/// The picked file and the Open button. Phase 2 grows this into a folder
/// list with thumbnails.
fn picker(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let v = &app.videos;
    let open = button(labeled(Some(Icon::Search), &s.videos_open_file, 13.0, None))
        .padding([6, 12])
        .style(ui_theme::accent_button)
        .on_press(Msg::Videos(VideosMsg::OpenFile));
    let mut col = column![ui_theme::section_label(&s.tab_videos), open].spacing(ui_theme::SPACING);
    match &v.picked {
        Some(p) => {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            col = col.push(text(name).size(13));
            if v.opening {
                col = col.push(text(&s.videos_opening).size(12).color(ui_theme::MUTED));
            }
        }
        None => col = col.push(text(&s.videos_pick_hint).size(13).color(ui_theme::MUTED)),
    }
    if let Some(e) = &v.error {
        let danger = ui_theme::theme().palette().danger;
        col = col.push(text(e.as_str()).size(12).color(danger));
    }
    card(col).width(PICKER_W).height(Fill).into()
}

/// The preview frame with the transport under it.
fn preview(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let v = &app.videos;
    let frame: Element<'_, Msg> = match (&v.player, app.gpu_ok) {
        (Some(p), true) => {
            let scene = std::sync::Arc::new(sda_render::Scene {
                size: (1280, 720),
                background: sda_render::BackgroundState::Solid(sda_core::theme::Rgba::BLACK),
                background_opacity: 1.0,
                current: Some(sda_render::Layer::Video(p.frames())),
                outgoing: None,
                transition: sda_core::theme::Transition::default(),
                blank: false,
                now_secs: 0.0,
            });
            scene_view(key("video_preview"), scene)
        }
        (Some(_), false) => text(&s.gpu_unavailable).size(13).center().into(),
        (None, _) => text(&s.videos_pick_hint).size(13).color(ui_theme::MUTED).center().into(),
    };
    let frame = container(frame)
        .width(Fill)
        .height(Fill)
        .style(ui_theme::thumb_frame);
    let mut col = column![ui_theme::section_label(&s.preview), frame].spacing(ui_theme::SPACING);
    if let Some(p) = &v.player {
        col = col.push(transport(app, p));
    }
    card(col).width(Fill).height(Fill).into()
}

fn transport<'a>(app: &'a App, p: &'a sda_media::Player) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let pos = p.position();
    let dur = p.duration();
    let playing = p.state() == State::Playing;
    let seek = slider(0.0..=dur.as_secs_f32().max(0.01), pos.as_secs_f32(), |x| {
        Msg::Videos(VideosMsg::Seek(x))
    })
    .step(0.1);
    let time = text(format!("{} / {}", format_time(pos), format_time(dur)))
        .size(12)
        .color(ui_theme::MUTED)
        .wrapping(text::Wrapping::None);
    let ghost = |body: Element<'a, Msg>, msg: Msg| {
        button(body)
            .padding([6, 10])
            .style(ui_theme::ghost_button)
            .on_press(msg)
    };
    let back = ghost(icon(Icon::SkipBack, 14.0).into(), Msg::Videos(VideosMsg::Step(-10)));
    let fwd = ghost(icon(Icon::SkipForward, 14.0).into(), Msg::Videos(VideosMsg::Step(10)));
    let (play_icon, play_label) = if playing {
        (Icon::Square, s.pause.as_str())
    } else {
        (Icon::Play, s.play.as_str())
    };
    let toggle = ghost(
        labeled(Some(play_icon), play_label, 13.0, None),
        Msg::Videos(VideosMsg::Toggle),
    );
    let volume = slider(0.0..=1.0, app.videos.volume, |x| Msg::Videos(VideosMsg::Volume(x)))
        .step(0.05)
        .width(100);
    let mut vol_row = row![text(&s.volume).size(12).color(ui_theme::MUTED), volume]
        .spacing(6)
        .align_y(Alignment::Center);
    if p.muted() {
        vol_row = vol_row.push(
            iced::widget::tooltip(
                icon(Icon::EyeOff, 13.0).color(ui_theme::MUTED),
                container(text(&s.videos_muted_hint).size(12)).padding(6).style(ui_theme::card),
                iced::widget::tooltip::Position::Top,
            ),
        );
    }
    let state_hint: Element<'a, Msg> = if p.state() == State::Ended {
        text(&s.videos_ended).size(12).color(ui_theme::MUTED).into()
    } else {
        space::horizontal().into()
    };
    let present = button(labeled(Some(Icon::Play), &s.bible_present_selected, 13.0, Some(Action::Present)))
        .padding([6, 14])
        .style(ui_theme::accent_button)
        .on_press(Msg::Videos(VideosMsg::Present));
    column![
        row![time, seek].spacing(ui_theme::SPACING).align_y(Alignment::Center),
        row![back, toggle, fwd, state_hint, vol_row, present]
            .spacing(ui_theme::SPACING / 2.0)
            .align_y(Alignment::Center),
    ]
    .spacing(ui_theme::SPACING / 2.0)
    .into()
}
```

`bible_present_selected` reads "Present" in every language, reuse it.
Rename the downloader card title: in `downloader`, replace
`ui_theme::section_label(&s.videos)` with
`ui_theme::section_label(&s.videos_download)` and give the card
`.height(Shrink)` instead of `Fill` so history takes the rest. Import
`Shrink` from `iced`.

- [ ] **Step 6: Run the tests, then clippy and fmt**

Run: `cargo test -p sda-app --test videos_test --test views_test`
Expected: green.

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: clean. Fix anything it names.

- [ ] **Step 7: Run the app and use it**

Run: `cargo run -p sda-app`. On the Videos tab: Open file, pick an mp4
with sound. Expected: the first frame shows paused, the time reads
`0:00 / m:ss`. Play, the picture moves with sound. Drag the slider, the
picture jumps and sound follows within a second. Step back and forward.
Volume slider changes loudness. Present, the live strip shows the same
picture, the projector shows it letterboxed over black, Space pauses and
→ ← step 10 s, B blanks and pauses, ⌘. stops and the sound stops. Let a
clip end, the projector goes black and the transport reads Ended.

- [ ] **Step 8: Commit**

```bash
git add crates/sda-app crates/sda-core crates/sda-media
git commit -m "feat(videos): open a file, preview with transport, present it"
```

---

### Task 9: Packaging and docs

**Files:**
- Modify: `crates/sda-app/Cargo.toml` deb and rpm metadata
- Modify: `CHANGELOG.md`, `CLAUDE.md`

- [ ] **Step 1: Linux runtime dependency**

In `crates/sda-app/Cargo.toml` find `[package.metadata.deb]` and
`[package.metadata.generate-rpm]` with `grep -n "metadata" crates/sda-app/Cargo.toml`.
Add to the deb section:

```toml
depends = "$auto, libasound2 | libasound2t64"
```

If a `depends` line exists, append `, libasound2 | libasound2t64` to it.
Add to the rpm section:

```toml
[package.metadata.generate-rpm.requires]
alsa-lib = "*"
```

Run: `cargo build -p sda-app` to be sure the manifest still parses.

- [ ] **Step 2: Changelog**

Add a `### Video` block to the `## v2.1.0` section of `CHANGELOG.md`
after `### Keyboard and icons`:

```markdown
### Video

- Videos tab: open a video file, preview it with play, pause, a seek bar, 10 s steps, and volume, then present it. Sound plays through the default output. On the projector the picture is letterboxed over black, no theme applies. Space pauses, → ← step 10 s, B blanks and pauses. The file ends to a black screen.
- The downloader is one card on the right of the Videos tab.
- Linux packages now depend on `libasound2` for sound output.
```

- [ ] **Step 3: CLAUDE.md**

In the Architecture list add a crate after `sda-render`:

```markdown
- **`crates/sda-media`**: video playback for presenting. `Player` runs two
  `ffmpeg` children per file, raw RGBA frames and f32 PCM, both from the
  same `-ss`. Frames are paced against the audio clock, a `SampleClock`
  fed by the rodio source, or a `WallClock` when there is no sound. Seek
  restarts both children. `tick` runs once per rendered frame from
  `Msg::Tick`. Tests run a fake ffmpeg shell script and never open a
  device, so they are unix only.
```

Add `sda-media` to the Commands block: `cargo test -p sda-media -- --test-threads=1`.
In the `sda-app` module list under `workers.rs` add `open_video`. In UI
conventions add: "A video slide is `SlideContent::Video`. `LiveState`
keeps the `Player`, `next` and `prev` seek 10 s, `blank` pauses, the
scene background is black whatever the theme says." In the Keyboard
table note Enter presents on Videos too and Space toggles a live video.

- [ ] **Step 4: Full verification**

Run: `cargo test --workspace --no-fail-fast 2>&1 | grep -E "test result|FAILED"`
Expected: every line reads `0 failed`.

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add crates/sda-app/Cargo.toml CHANGELOG.md CLAUDE.md
git commit -m "docs: video presenting in the changelog and the architecture notes, alsa depends"
```

---

## Out of scope for this plan, phase 2 and 3

Folder listing with thumbnails and a remembered folder, the videos history
card, Esc clearing the picked file, `[` `]` stepping the preview on the
Videos tab, screenshots. Each starts from the spec section with its name.
