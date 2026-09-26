# CLAUDE.md

This file guides Claude Code when working in this repository.

## What this is

SDA Manager 2.0.0 is a desktop app for projecting hymns. It is written in Rust
with eframe/egui 0.36. There is one operator screen. The search panel is on the
left. The Preview card and the Live card are on the right. Settings and Videos
open as egui windows. The projector is an immediate egui viewport shown
fullscreen on a chosen display.

Hymns are `.pptx` files. Search matches number, title, filename, and lyrics. It
is fuzzy and ignores accents, so `plecati` finds *Plecaţi-vă*. The hymnal comes
from the `hymns.zip` asset of the latest GitHub release with ETag sync. The app
also checks GitHub Releases for new versions.

## Commands

```sh
cargo build --release -p sda-gui      # -> target/release/sda-manager
cargo run -p sda-gui                  # run in debug
cargo test -p sda-core                # all core tests
cargo test -p sda-core search         # one test file
cargo test -p sda-core -- matches_by_number   # one test by name
cargo test -p sda-gui                 # the fonts unit test
RUST_LOG=sda_manager=debug,sda_core=debug cargo run -p sda-gui   # verbose logs
```

The binary log target is `sda_manager`. Run `cargo clippy` for lints.

### Releases and distribution

- Releases are built by `.github/workflows/release.yml` by manual trigger only.
  It takes a `version` input like `v2.0.0`. It builds natively on `macos-14`,
  `windows-latest`, and `ubuntu-latest`. It publishes a GitHub Release with one archive per target.
- `install.sh` (macOS/Linux) and `install.ps1` (Windows) are one-liner installers
  that fetch the latest `sda-manager` asset. The README has the commands.
- The `hymns` job zips `assets/920` into `hymns.zip`. The app downloads it on
  first run and whenever its ETag changes. Hymn edits reach users on the next
  release. A push alone does nothing.

## Architecture

Two crates. Logic lives in the core. The GUI stays thin.

- **`crates/sda-core`**: no UI, unit tested. Modules: `pptx`, `model`, `index`,
  `search`, `fold`, `library`, `sync`, `refresh`, `downloader`, `update`,
  `i18n`, `theme`, `present`, `fit`.
- **`crates/sda-gui`**: the egui app, binary `sda-manager`. `build.rs` embeds
  `assets/icon.ico` on Windows. `#![windows_subsystem = "windows"]` hides the
  console in release builds on Windows. Modules:

```
main.rs          eframe setup, logging, window options
app.rs           App, Msg, UiState, App::ui: drain, shortcuts, layout
core.rs          Core: config, strings, themes, worker spawns, save_cfg
search_panel.rs  search box and virtualized results
preview.rs       Preview card
live.rs          Live card and projector controls
projector.rs     display list and the projector viewport
slide.rs         slide::paint, layout, footer, FitCache
settings.rs      Settings window
theme_editor.rs  theme thumbnails, form, live preview
videos.rs        Videos window
shortcuts.rs     key handling
style.rs         visuals, spacing, bundled fonts
```

Config is loaded once in `Core::load` and written only by `Core::save_cfg`.

### Data flow and threading

Workers never touch the UI. Each worker sends one `app::Msg` over an
`mpsc` channel and calls `ctx.request_repaint()`. Messages are `Index`,
`Synced`, `Reindexed`, `Update`, and `Download`. Workers build the `Searcher`
themselves, so the UI thread only swaps it in.

- Boot, `Core::spawn_boot`: load the local index, then sync `hymns.zip`, then
  reindex only if the zip changed, then check for updates.
- Force sync, `Core::spawn_force_sync`: sends `Msg::Synced`.
- Library folder edits, `Core::spawn_reindex`: sends `Msg::Reindexed`.
- Downloads, `Core::spawn_download`: sends `Msg::Download` events.

`App::ui` calls `drain` first on every frame to apply pending messages.

### UI conventions

- Search runs only when the query changed. Results use `ScrollArea::show_rows`.
- `slide::paint` is the only place slide text is drawn. Preview, Live, the
  projector, and the theme editor all call it. Auto size uses `sda_core::fit`
  and a `FitCache` bounded at 64 entries.
- The selected hymn drives Preview. Go Live copies it into the presentation
  state that Live and the projector show.
- The projector runs through `ctx.show_viewport_immediate`, so its frames run
  on the main thread. Closing it from the OS clears `app.projector`. Keys
  pressed in the projector go through the same `shortcuts::handle`.
- Settings and Videos are `egui::Window`s. eframe persists window geometry.

### Keyboard

| Key | Action | Scope |
|---|---|---|
| Cmd/Ctrl+K | focus search | always |
| Up / Down | move selection | search focused or list |
| Enter | go live with previewed slide | search focused |
| Left / Right, Space | live prev / next | not in a text field |
| B | blank toggle | not in a text field |
| Esc | stop projector | always |
| 1..9 | live jump to slide N | not in a text field |

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
- Themes in `theme.rs` are JSON files. Fonts are Inter, Noto Serif, Lora, and
  Roboto Slab. Roboto Slab is Apache 2.0 and the others are OFL. A theme sets a
  solid background color, auto or fixed size, bold, alignment, line spacing,
  margin, and footer.
- Config paths are unchanged from 1.x. Config is TOML in the OS config dir.
- `i18n.rs` has a `Strings` struct per language, En, It, and Ro. A new UI
  string fails to compile until all three languages have it.
- `downloader.rs` finds `yt-dlp` and `ffmpeg` on `PATH` or in the data
  `tools/` dir, else downloads them.

### Tests

Core tests live in `crates/sda-core/tests/` and use real `.pptx` fixtures in
`tests/fixtures/`: `001.pptx`, `150.pptx`, `356.pptx`, and `664b.pptx`. The
last one covers letter suffixes. `sync` is tested against a local
`TcpListener` stub. `downloader` parsing runs without network. Self update is
not tested. `sda-gui` has one unit test for fonts.
