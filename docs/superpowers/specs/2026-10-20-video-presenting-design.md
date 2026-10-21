# Video presenting

Status: approved 2026-10-20, open questions settled. Nothing here is built yet.

## Goal

Present a video file on the projector the same way hymns and verses are
presented. The operator browses a folder or opens one file, sees a preview
with transport controls, and sends it live. Live controls: play, pause,
seek to a time, step back and forward, volume. No theme applies. The
YouTube downloader stays, folded into one small card.

## What exists

- `sda-render::video` already runs `ffmpeg` as a child process and reads
  raw RGBA frames from its stdout into a `FrameSource`. The renderer uploads
  `latest()` once per frame. It is video only, loops forever, is paced by
  `-re`, and ffmpeg stretches the clip to the target size. It serves video
  backgrounds for themes.
- `sda-core::downloader::ensure_ffmpeg` finds or downloads a static ffmpeg
  into the tools dir on every OS. The binary is a separate process, so its
  GPL build never links into Lumen.
- `Presentation` holds slides. `Source` is `Hymn`, `Bible`, or `None`.
  `LiveState` builds a `Scene` for the Live strip and the projector.
  `animating()` turns the frame clock on.
- `composite::cover` scales a bitmap to fill the output. No `contain`.
- The Videos tab is the downloader.

## Research: how to decode and play across macOS, Windows, Linux

Checked on 2026-10-20. Versions from crates.io.

| Option | New runtime dependency | A/V clock | Seek | Verdict |
|---|---|---|---|---|
| A. ffmpeg CLI for pictures and sound, rodio for output | `libasound2` on Linux only | rodio `Player::get_pos` | restart ffmpeg at `-ss` | **pick** |
| B. `ffmpeg-sidecar` 2.7 | same as A | timestamps are computed as frame/fps, wrong on variable frame rate | restart | no gain over A, we already ship ffmpeg |
| C. GStreamer via `iced_video_player` 0.6 | full GStreamer runtime per OS, plugin dirs, framework bundling on macOS | pipeline | native | heavy installers, gstreamer 0.25 needs Rust 1.92 |
| D. libmpv via `libmpv2` 6 | libmpv plus its own ffmpeg per OS, GPL by default | mpv | native | render API is OpenGL only, no wgpu path |
| E. `ffmpeg-next` 9 static | none at run time | our code | native | bindgen, msys2 on Windows, two macOS builds and lipo, LGPL relink duty |

Option A wins on every axis that matters here. It reuses the decoder and
the download path we have, adds one pure Rust crate for sound, and keeps
ffmpeg out of our binary.

Facts behind A:

- `rodio` 0.23, MIT or Apache 2.0, MSRV 1.87. `Player` has `append`,
  `pause`, `play`, `set_volume`, `clear`, and `get_pos`. Use
  `default-features = false, features = ["playback"]` so symphonia and its
  decoders stay out. We feed f32 PCM from ffmpeg, no decoding in Rust.
- `cpal` underneath uses CoreAudio, WASAPI, and ALSA. On Linux the `alsa`
  crate is not optional. CI already installs `libasound2-dev`. The deb and
  rpm need a runtime depends on `libasound2`. PulseAudio and PipeWire are
  reached through their ALSA plugins, which every desktop has.
- `get_pos` counts samples pulled from the source. It does not include
  device latency, a few tens of milliseconds, which is below what an eye
  notices against the picture.
- ffmpeg can write two outputs from one process, but on Windows file
  descriptors above 2 are not usable, so that needs named pipes or local
  sockets. Two ffmpeg processes sharing the same `-ss` are simpler and
  behave the same on every OS. Audio decode is cheap next to video, so the
  second process costs little.
- `-f rawvideo` carries no timestamps. With `-fps_mode cfr -r N` frame k
  sits at `ss + k / N`. That is enough for sync against the audio clock.
- `-re` paces by wall clock and bursts after a pause. The app paces
  instead: no `-re`, a bounded frame queue, pipe back pressure throttles
  ffmpeg, and the presenter picks the frame whose time is at or under the
  audio clock.
- `-ss` before `-i` lands on the exact time after decoding forward from the
  nearest keyframe. A seek costs one process restart, well under a second
  on a 1080p file.
- Duration comes from `ffmpeg -i file` on stderr, the `Duration: hh:mm:ss.cc`
  line. No ffprobe needed.

## Architecture

### New crate `sda-media`

Owns the player. Depends on `sda-render` for `FrameSource` and on `rodio`.
Keeping it out of `sda-render` keeps the renderer GPU only and keeps the
golden tests free of audio devices.

```
sda-media/src/
  lib.rs       Player: open, play, pause, seek, set_volume, state, position
  probe.rs     duration and size of a file by parsing ffmpeg stderr
  decode.rs    spawn the video and the audio ffmpeg processes for a file
               starting at t, bounded frame queue, PCM ring buffer
  clock.rs     the playback clock: rodio position when sound is on,
               wall clock since play when the file has no audio or no
               device opened, so the picture never freezes on a silent box
  thumb.rs     one frame at 10 percent into the file, 320 px wide, cached
               by path and mtime under the data dir
```

`Player` holds a `FrameSource`, a rodio `Player`, the two children, the
start offset of the current segment, and `State { Idle, Playing, Paused,
Ended }`. The presenter reads `frame_source()` each tick, which already
holds the frame matching the clock. Dropping the `Player` kills both
children, the same pattern `FrameSource` uses today.

Public surface, all synchronous and cheap, workers call the slow parts:

```rust
pub struct Player { .. }
impl Player {
    pub fn open(ffmpeg: &Path, file: &Path, max_size: (u32, u32)) -> Result<Player>;
    pub fn play(&self);
    pub fn pause(&self);
    pub fn toggle(&self);
    pub fn seek(&self, to: Duration);        // restarts both children at `to`
    pub fn step(&self, by: Duration);         // seek relative, clamped
    pub fn set_volume(&self, v: f32);         // 0.0 to 1.0, also muted flag
    pub fn position(&self) -> Duration;
    pub fn duration(&self) -> Duration;
    pub fn state(&self) -> State;
    pub fn frames(&self) -> Arc<FrameSource>;
}
```

`open` scales the picture in ffmpeg with
`scale=w:h:force_original_aspect_ratio=decrease` to the projector size, so
a 4K file never pushes 4K through the pipe, and the frame keeps its aspect.

### Core model, `sda-core`

- `SlideContent::Video(PathBuf)`. `resolve` returns `None` for it like
  `Bitmap`.
- `Source::Video(PathBuf)`.
- `Presentation` with one video slide. `next`, `prev`, `jump` stay no ops
  at one slide. Blank keeps its meaning, a black projector.
- `media.rs`: `is_video_file(path)` by extension, mp4 mkv mov webm avi m4v,
  `list_videos(dir)` sorted by name, `format_time(Duration)` as `m:ss` or
  `h:mm:ss`. Pure and unit tested.
- `Config` gains `video_dir: Option<PathBuf>` and `video_volume: f32`.

### Renderer, `sda-render`

- `Layer::Video(Arc<FrameSource>)`. Drawn with the bitmap pipeline and a
  new `composite::contain`, six lines next to `cover`, over the solid black
  background. The video key eviction that backgrounds use applies here
  too, so textures of old frames are trimmed.
- One golden test with a synthetic frame source, no ffmpeg.

### App, `sda-app`

`LiveState`

- `player: Option<Arc<sda_media::Player>>`, set by `go` when the source is
  a video, dropped by `stop` and by any other `go`.
- `scene()` emits `Layer::Video` for a video slide. Background is black,
  theme ignored. Footer off.
- `animating()` is true while `player.state() == Playing`. `Tick` keeps
  the frame clock running and ends it at `Ended`.
- `next` and `prev` on a video seek by 10 s. `blank` pauses the sound and
  blanks the picture. So the Live strip and the → ← Space B keys work
  with no new messages.

New messages, `VideosMsg`

```
Folder(PathBuf) FolderChosen OpenFile FileChosen Pick(PathBuf)
Toggle Seek(Duration) Step(i32) Volume(f32) Present Ended
```

The preview `Player` in the Videos tab is the one that goes live.
`Present` moves it into `LiveState`, so there is no second decode and the
live picture starts where the preview was. A new preview `Player` opens
when the operator picks another file.

Videos tab layout, same shape as Bible

```
+------------------+--------------------------------+------------------+
| FOLDER   [cog]   | PREVIEW                        | DOWNLOAD         |
| path chip        | [frame, contain, 16:9]         | URL field        |
| [Open file…]     | 0:42 ----o-------------- 3:10  | [Fetch] [Save]   |
| list of videos   | [|◁ -10s] [▷ Play] [+10s ▷|]   | small progress   |
| with thumbnails  | volume slider   [Present ⏎]    |                  |
|                  |                                | HISTORY          |
|                  |                                | recent videos    |
+------------------+--------------------------------+------------------+
```

The download card keeps today's downloader with fewer words. A finished
download lands in the folder list and is selected.

Shortcuts, from the one table

- Enter presents the previewed video on the Videos tab, same `Action::Present`.
- Space toggles play and pause when a video is live, else next slide.
  `Action::Next` already routes to `LiveState::next`, which decides.
- → ← step 10 s when a video is live.
- Esc clears the picked file.
- `[` `]` step the preview 10 s, the Bible preview keys reused by scope.

Workers

- `workers::probe_video(path)` runs ffmpeg for duration and size, ends in
  `VideosMsg::Probed`.
- `workers::thumbnail(path)` ends in `VideosMsg::Thumb`. One at a time,
  newest requested first, so a folder of 200 files does not spawn 200
  processes.
- `Player::open` runs on a blocking thread too, ends in `VideosMsg::Opened`.

### Failure handling

- No ffmpeg yet: the tab shows the same "setting up" line the downloader
  shows and `ensure_ffmpeg` runs. Nothing else works until it lands.
- No audio device, or rodio fails to open one: the player logs it and
  plays on the wall clock, muted. The UI shows a muted icon with a tooltip.
- End of file: state `Ended`, the projector goes black, the preview shows
  the first frame again with the position at the end. No loop.
- ffmpeg exits early with an error: state `Ended`, the error line from
  stderr shows under the preview, the projector shows black.
- A file that is not a video: `open` fails on probe, the row shows the
  error, nothing goes live.
- Seek past the end clamps to the end. Seek while paused stays paused and
  shows the new frame.

### Packaging

- `crates/sda-app/Cargo.toml` deb and rpm metadata: depends on
  `libasound2 | libasound2t64` and `alsa-lib`. AppImage and tar.gz rely
  on the host, every desktop has ALSA.
- macOS and Windows: nothing new.
- Build: nothing new, `libasound2-dev` is already in the Linux CI step.
- Licenses: rodio MIT or Apache 2.0, cpal Apache 2.0. ffmpeg stays a
  process.

### Testing

- `sda-core`: file filter, listing order, time formatting, duration
  parsing from a saved ffmpeg stderr fixture.
- `sda-media`: the clock against a fake audio position, the frame picker
  against a queue of timed frames, and `open` against a stub `ffmpeg`
  script in `tests/fixtures` that writes a few RGBA frames and a few PCM
  samples. Audio output is behind a trait so tests never open a device.
- `sda-render`: one golden, `video_contain`, with a hand made frame.
- `sda-app`: Simulator tests feed `VideosMsg` and check the presentation,
  the live strip labels, and that → seeks while a video is live.

## Phases

1. Core model, `sda-media` with play, pause, seek, volume, `Layer::Video`
   with contain, `LiveState` wiring, a bare Videos tab with Open file and
   the transport. A video plays on the projector with sound.
2. Folder browsing with thumbnails, the download card, history, the
   remembered folder and volume.
3. Polish: end of file behaviour, muted fallback UI, the shortcut sheet
   entries, screenshots and docs.

## Decisions

Settled with the owner on 2026-10-20.

1. End of file goes black. No loop.
2. Blank pauses the sound as well as the picture.
3. Step size for → ← and the step buttons is 10 s.
4. The downloader stays in the Videos tab as one card. A finished download
   lands in the folder list and is selected.
