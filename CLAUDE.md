# CLAUDE.md

This file guides Claude Code when working in this repository.

## What this is

Lumen 2.0.1 is a desktop app for projecting hymns and Bible verses. It is
written in Rust with Iced 0.14 and a wgpu renderer. There is one operator
window with five tabs: Present, Bible, Videos, Themes, Settings. A Live strip
stays visible across tabs and shows what the projector shows. The projector is
a second Iced window shown fullscreen on a chosen display.

Hymns are `.pptx` files. Search matches number, title, filename, and lyrics. It
is fuzzy and ignores accents, so `plecati` finds *Plecaţi-vă*. The hymnal comes
from the `hymns.zip` asset of the latest GitHub release with ETag sync. Bibles
come from the thiagobodruk/bible repository pinned at one commit. The app also
checks GitHub Releases for new versions.

## Commands

```sh
cargo build --release -p sda-app      # -> target/release/lumen
cargo run -p sda-app                  # run in debug
cargo test --workspace                # everything
cargo test -p sda-core                # core tests
cargo test -p sda-core search         # one test file
cargo test -p sda-core -- matches_by_number   # one test by name
cargo test -p sda-render              # headless goldens, skip without a GPU
UPDATE_GOLDENS=1 cargo test -p sda-render     # rewrite the golden PNGs
cargo test -p sda-app                 # iced_test simulator tests
cargo clippy --workspace --all-targets -- -D warnings
RUST_LOG=sda_app=debug,sda_core=debug,sda_render=debug cargo run -p sda-app
```

Log targets are `sda_app`, `sda_core`, and `sda_render`. The default filter
is `info` for all three. Rust 1.88 or newer. `sda-app` and `sda-render` use
edition 2024 and let chains.

### Releases and distribution

- Releases are built by `.github/workflows/release.yml` by manual trigger only.
  It takes a `version` input like `v2.0.1` and a `publish` flag. Off means a
  dry run that keeps the packages as workflow artifacts. A `test` job runs
  `cargo test --workspace` and clippy on `macos-14`, `windows-latest`, and
  `ubuntu-latest`, and fails when the tag is not the Cargo version. Release
  builds use fat LTO and opt-level "s" for size. Packaging jobs:
  - `macos`: builds both Apple targets, then `packaging/macos/package.sh`
    makes `lumen-<v>-macos-universal.dmg` with an ad hoc signed Lumen.app.
  - `windows`: WiX 5 builds `lumen-<v>-windows-x64.msi` from
    `packaging/windows/lumen.wxs`, a per user install in
    `%LOCALAPPDATA%\Programs\Lumen`, plus `lumen-<v>-windows-x64-portable.zip`.
  - `linux` on x86_64 and aarch64: `.deb` by cargo-deb, `.rpm` by
    cargo-generate-rpm, both from `[package.metadata.*]` in
    `crates/sda-app/Cargo.toml`, an `.AppImage` by appimagetool, and a flat
    `.tar.gz`. `packaging/linux/lumen.desktop` is the menu entry.
  - `release` adds `SHA256SUMS.txt` and takes the notes from the matching
    `CHANGELOG.md` section.
- Self update in `update.rs` downloads only the portable files, matched by
  `portable_asset`. The macOS app, AppImage, deb, rpm, or a folder it cannot
  write report `UpdateOutcome::Available` and the chip opens the release
  page. `update` also skips major version bumps.
- Linux CI installs `libxkbcommon-dev libwayland-dev libxcb-render0-dev
  libxcb-shape0-dev libxcb-xfixes0-dev libgtk-3-dev libasound2-dev`.
- `install.sh` for macOS and Linux and `install.ps1` for Windows are one line
  installers. `install.sh` copies Lumen.app from the dmg on macOS and the
  portable binary plus a menu entry on Linux. `install.ps1` runs the msi.
- The `hymns` job zips `assets/920` into `hymns.zip`. The app downloads it on
  first run and whenever its ETag changes. Hymn edits reach users on the next
  release. A push alone does nothing.
- v1.x shipped the binary `hymnal-gui` from the repo `sda_manager`. GitHub
  redirects the old repo. v1.x cannot self update to 2.x, so users reinstall
  once. The installers remove the v1.x binary, folder, and PATH entry. Config
  paths, themes dir, and hymns sync are unchanged from 1.x.

## Architecture

Three crates. Logic lives in the core. The renderer knows nothing about Iced.
The app stays thin.

- **`crates/sda-core`**: no UI, no GPU, unit tested. Modules: `asis`, `bible`
  with `names` and `reference`, `downloader`, `fit`, `fold`, `history`, `i18n`,
  `index`, `library`, `model`, `overrides`, `pptx`, `present`, `refresh`,
  `resolve`, `search`, `store`, `sync`, `theme`, `update`.
- **`crates/sda-render`**: wgpu scene renderer, tested headless with golden
  PNGs. Modules: `background`, `composite`, `fonts`, `headless`, `layer`,
  `scene`, `text`, `video`. `Renderer` owns every pipeline. `prepare` uploads
  one scene under a caller chosen key and `render` draws it into a viewport
  with a scissor. `trim` runs at the end of every frame and drops scenes not
  prepared for two frames.
- **`crates/sda-app`**: the Iced daemon, binary `lumen`. `build.rs`
  embeds `assets/icon.ico` on Windows. `#![windows_subsystem = "windows"]`
  hides the console on Windows. Modules:

```
main.rs          calls sda_app::run
lib.rs           run: env_logger, iced::daemon, bundled fonts
app.rs           App, boot, update, view(window_id), subscription, theme
msg.rs           Msg, Tab, Handoff, BgTarget, BackgroundAsset
msg/             bible.rs, present.rs, settings.rs, themes.rs, videos.rs
state/           core.rs, live.rs, present.rs, bible.rs, videos.rs,
                 themes.rs, settings.rs, projector.rs
view/            shell.rs, live_strip.rs, present.rs, bible.rs, videos.rs,
                 themes.rs, theme_form.rs, settings.rs, screen_picker.rs,
                 projector.rs, placeholder.rs
widgets/         scene_view.rs, card.rs, segmented.rs
ui_theme.rs      palette, container and button styles
shortcuts.rs     event filter
displays.rs      display-info wrapper
workers.rs       Task builders for boot, sync, reindex, update, gpu probe,
                 backgrounds, converter detect, fonts list
```

`Core` in `state/core.rs` holds config, strings, searcher, themes, history,
overrides, download records, and the detected converter. `Paths` lists the
small JSON state files: `history.json`, `overrides.json`, `downloads.json`,
`window.json`, the `bibles` dir, and the `asis` cache dir. Config is loaded
once in `Core::load` and written only by `Core::save_cfg`. The other files are
written by `save_history`, `save_overrides`, and `save_downloads`.

### Data flow and threading

Workers never touch the UI. Every function in `workers.rs` returns an
`iced::Task` that runs the blocking core call on a tokio blocking thread and
ends in exactly one `Msg`. A panic inside becomes an `Err` string. Results
that are not `Clone`, like `Searcher`, travel in `Handoff<T>` and `update`
takes them out once.

Top level messages in `msg.rs` include `Indexed`, `Synced`, `Reindexed`,
`UpdateChecked`, `GpuProbed`, `BackgroundReady`, `AsIsRendered`,
`DisplaysListed`, `FontsListed`, `Tick`, `ProjectorClosed`, `WindowMoved`,
`WindowResized`, and `SaveWindow`. Tab messages live in sub enums:
`PresentMsg`, `BibleMsg`, `VideosMsg`, `ThemesMsg`, `SettingsMsg`.

- Boot, `workers::boot`: index from disk, then sync `hymns.zip` if it changed,
  then check for updates. On a first run the library downloads before indexing.
- Force sync, `workers::force_sync`: ends in `Msg::Synced`. `Ok(None)` means
  the zip was already current.
- Library folder edits, `workers::reindex`: ends in `Msg::Reindexed`.
- `workers::gpu_probe` calls `sda_render::headless::device()` and ends in
  `Msg::GpuProbed(false)` when there is no wgpu adapter. The app shows a
  fallback banner.
- `workers::background` decodes an image or starts an ffmpeg decoder for a
  video background and ends in `Msg::BackgroundReady` for either the live
  state or the Themes tab preview.
- Downloads, Bible index and version downloads, and as-is conversion send
  their own tab messages.

`App::subscription` listens to `shortcuts::filter`, window geometry events,
and `window::close_events`. It adds `window::frames` as `Msg::Tick` only while
`live.animating()` is true, so idle frames cost nothing.

### UI conventions

- Search runs only when the query changed.
- `scene_view` is the only widget that draws a slide. The Present preview,
  the thumbnails, the Live strip, the Themes preview, and the projector all
  call it with a `Scene` from `sda-render`. Every call site on one screen
  passes a different stable key, because Iced prepares all scenes before it
  renders any.
- A slide is resolved by `sda_core::resolve` from a `Theme` plus a
  `SlideContent`. `SlideContent` is `Styled` for parsed pptx, `Verses` for
  Bible verses, or `Bitmap` for as-is pages. Theme properties are `Prop<T>`.
  `Set` wins, `Inherit` takes the pptx value when the file has one, and the
  fallback otherwise.
- Auto text size uses `sda_core::fit`. Text is rasterized by `sda-render::text`
  with cosmic-text at the target size, so size is part of the texture key.
- `LiveState` holds the `Presentation`, the theme, the outgoing slide, and the
  transition clock. The Live strip and the projector both build their `Scene`
  from it.
- The projector is a second Iced window with no decorations. A close request
  only comes from the OS and `Msg::ProjectorClosed` clears the state.
- One dark palette lives in `ui_theme.rs`. A second palette is one file.
- `i18n::Strings` has every UI string. Tabs take their label from it.
- The Themes editor shows a Hymn or Bible sample. Verse and reference edits
  switch it to Bible.

### Keyboard

`shortcuts::filter` runs on every event the widgets did not capture, so
nothing fires while a text field has focus. Modifier changes and left button
releases are forwarded before the captured check, as `Msg::Modifiers` and
`BibleMsg::Release`.

| Key | Action |
|---|---|
| Cmd/Ctrl+K | focus search |
| Cmd/Ctrl+O | toggle projector |
| Cmd/Ctrl+1..5 | switch tab |
| Up / Down | move selection |
| Enter | go live with the previewed slide |
| Left / Right, Space | live prev / next |
| B | blank toggle |
| 1..9 | live jump to slide N |
| Esc | stop the projector |

In the Bible tab a click toggles a verse, Shift click selects from the last
clicked verse, and dragging selects a range. `shortcuts::filter` forwards
`ModifiersChanged` as `Msg::Modifiers` so views know about Shift.

### Conventions

- Hymn numbers come from the filename stem, not slide text. The number is an
  `Option<String>` because some hymns have a letter suffix like `664b`.
  `search::number_sort_key` sorts `664` before `664a` before `665`.
- Accent folding in `fold.rs` runs on both indexed text and the query. Add new
  characters to its table.
- Ranking in `search.rs` adds fuzzy score with bonuses for exact, prefix, and
  substring matches, plus a small field weight. Number beats title, then
  filename, then body.
- The index cache in `index.rs` is bincode with a `CACHE_VERSION`. Bump it when
  the parser or entry layout changes. Unchanged files are reused by path and
  mtime. The rest parse in parallel.
- Themes in `theme.rs` are JSON files in the themes dir. A theme has a name,
  `TextStyle`, `Background`, `background_opacity`, `Transition`, `footer`,
  `VerseLayout`, a second `TextStyle` for the Bible reference, and `favorite`. Backgrounds are `Solid`, `Gradient`,
  `Particles`, `Image`, or `Video`. Transitions are `None`, `Fade`, `Slide`,
  `Scale`, or `Blur` with an easing. Fonts are Inter, Noto Serif, Lora, and
  Roboto Slab. Roboto Slab is Apache 2.0 and the others are OFL. A theme can
  also name a system font.
- `history.rs` keeps the last 50 opened hymns, newest first, one entry per
  path. `store.rs` loads small JSON files without failing and saves through a
  temp file and a rename.
- `overrides.rs` keeps per hymn choices outside the hymn files. Today that is
  the as-is flag per path.
- Config paths are unchanged from 1.x. Config is TOML in the OS config dir.
  `Config` has `hymns_url`, `libraries`, `download_dir`, `language`,
  `active_theme`, `output_display`, and `bible_version`.
- `i18n.rs` has a `Strings` struct per language, En, It, and Ro. A new UI
  string fails to compile until all three languages have it.
- `downloader.rs` finds `yt-dlp` and `ffmpeg` on `PATH` or in the data
  `tools/` dir, else downloads them. Video backgrounds decode through the same
  `ffmpeg`.

### Bible data and license

- `bible/mod.rs` loads the JSON files of the thiagobodruk/bible repository.
  Every download URL points at the commit in `bible::PIN`. Files live in the
  `bibles` dir as `<code>.json` with a `<code>.etag` stamp through the `sync`
  module.
- `bible::public_domain(code)` is a small built in list. `Some(true)` for known
  public domain versions like `en_kjv`, `Some(false)` for known restricted
  ones like `en_niv`, `None` when unknown. The copyright of each version
  belongs to its owner. The app only marks what it knows. Do not present an
  unknown version as free.
- `bible/names.rs` has book names and aliases for en, it, and ro.
  `parse_reference` accepts `:`, `,`, or a space as in `mat 10 1` between
  chapter and verse and `-` for ranges, after folding. `search` folds both
  sides and scans all verses.
  `pack_verses` groups a selection into as few slides as fit under
  `fits_heuristic`, a character count estimate of the verse box at the theme's
  `min_size`. Each slide carries its own reference range.

### As-is pipeline

- `asis::detect` finds LibreOffice or PowerPoint on `PATH` or in well known
  install paths and returns a `Converter`. `workers::converter_detect` runs it
  and Settings shows the result.
- When a hymn has the as-is flag, the app converts the pptx to one PNG per
  slide and presents them as `SlideContent::Bitmap`. Output is cached in the
  `asis` cache dir under a content hash. The toggle is per hymn and lives in
  `overrides.json`.
- Without a converter the toggle is off and the parser based rendering is
  used.

### Tests

- Core tests live in `crates/sda-core/tests/` and use real `.pptx` fixtures in
  `tests/fixtures/`: `001.pptx`, `150.pptx`, `356.pptx`, and `664b.pptx`. The
  last one covers letter suffixes. `bible_mini.json` is a small Bible fixture.
  `ytdlp_probe.json` is a saved `yt-dlp -J` output. `sync` is tested against
  a local `TcpListener` stub. `asis::detect` is tested with a fake `PATH`.
  Self update is not tested.
- Render tests live in `crates/sda-render/tests/golden_test.rs`. Each test
  renders a 320 by 180 scene through `headless::render_to_image` and compares
  it to `tests/golden/<name>.png`. Tolerance is 2 per channel and one pixel
  per thousand. When `headless::device()` returns `None` the test prints
  `no GPU adapter, skipping` and returns, so Linux and Windows CI pass without
  a GPU. `UPDATE_GOLDENS=1` rewrites the PNGs. A missing PNG is written on
  first run.
- App tests live in `crates/sda-app/tests/` and use the `iced_test`
  `Simulator` on `App::for_test()`. They click and type into views, collect
  the messages, and feed them back through `App::update`. Shortcuts are tested
  by feeding events to `shortcuts::filter`.
