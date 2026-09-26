<div align="center">

# SDA Manager

**Find a hymn and put it on the screen in seconds.**

A free, cross-platform desktop app for projecting hymns during a service.
Search a hymnal by number, title, or a half-remembered line of the lyrics,
then project it full screen on the sanctuary display with your own styling.
No PowerPoint and no fiddling in the middle of a service.

Runs on **macOS**, **Windows**, and **Linux**. Available in **English**,
**Italian**, and **Romanian**.

[Download](https://github.com/AbelHristodor/sda_manager/releases) ·
[Install](#install) · [Features](#features) · [For developers](#for-developers)

</div>

---

## Why

Getting a hymn on the projector often means hunting through folders of
PowerPoint files while the congregation waits. SDA Manager replaces that with
one search box. Type `plecati`, press Enter, and *Plecaţi-vă* is on the wall.

It ships with a full hymnal that stays current on its own. You can add your own
folders of slides too.

## Features

- **Instant fuzzy search.** Search by number, title, filename, or any lyric
  line. Search ignores accents, so `plecati` finds *Plecaţi-vă*.
- **Built-in projector.** The app draws the slides itself on a second display.
  It needs no PowerPoint and works the same on every OS.
- **Preview and Live.** Check a slide in the Preview card, then send it to the
  Live card and the projector.
- **Custom themes.** Solid colors and four bundled fonts: Inter, Noto Serif,
  Lora, and Roboto Slab. Pick auto or fixed text size, bold, alignment, line
  spacing, margin, and footer.
- **Your own libraries.** Add any folder of `.pptx` files next to the built-in
  hymnal.
- **Video downloader.** Paste a YouTube link and save the video to a folder.
- **Multilingual UI.** English, Italian, and Romanian.
- **Keyboard first.** Every live action has a key. See the table below.
- **Always up to date.** The hymnal and the app update from the latest release.

## Install

Prebuilt apps are on the
[Releases](https://github.com/AbelHristodor/sda_manager/releases) page for
macOS on Apple Silicon, Windows x86_64, and Linux x86_64.

**macOS and Linux.** Paste into a terminal:

```sh
curl -fsSL https://raw.githubusercontent.com/AbelHristodor/sda_manager/main/install.sh | sh
```

The macOS app is unsigned. The first time, right click it and choose **Open**.
It installs `sda-manager` to `~/.local/bin`. Set `BIN_DIR` to change that.

**Windows.** Paste into PowerShell:

```powershell
irm https://raw.githubusercontent.com/AbelHristodor/sda_manager/main/install.ps1 | iex
```

It installs to `%LOCALAPPDATA%\Programs\sda-manager`, adds a Desktop shortcut,
and puts `sda-manager` on your `PATH`.

## Using it

1. **Search.** Type in the search panel on the left. Up and Down move the
   selection.
2. **Preview.** The selected hymn shows in the Preview card.
3. **Go Live.** Press Enter or click **Go Live**. The slide moves to the Live
   card.
4. **Projector.** Turn the projector on in the Live card and pick a display.
5. **Settings.** Open Settings to edit themes, add folders, change language,
   and sync the hymnal.

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

---

## For developers

SDA Manager is written in Rust with eframe/egui. It reads the text out of
`.pptx` files and draws the slides itself.

### Workspace layout

```
crates/
  sda-core/   # logic: pptx text, index, fuzzy search, config, sync,
              # themes, presentation state, text fit. Fully tested.
  sda-gui/    # egui app, binary `sda-manager`
```

### Building from source

```sh
cargo build --release -p sda-gui      # binary at target/release/sda-manager
cargo run --release -p sda-gui        # build and run
```

Use `--release` for daily use. Search feels faster. Set
`RUST_LOG=sda_manager=debug,sda_core=debug` to log indexing, sync, and search.

### How libraries work

A library is a folder of `.pptx` files. Each folder is crawled recursively.
Lock files and other file types are skipped. The default library is the
`hymns.zip` asset of the latest release. The app downloads it on first run and
again only when its ETag changes. A cached index means later launches only
parse files that changed.

### Configuration

Config is a TOML file in the OS config directory:

- **macOS:** `~/Library/Application Support/org.hymnal.HymnFinder/config.toml`
- **Windows:** `%APPDATA%\hymnal\HymnFinder\config\config.toml`
- **Linux:** `~/.config/HymnFinder/config.toml`

The default library lives in `…/org.hymnal.HymnFinder/hymns` with the last
ETag in `hymns.etag`. Themes live in `…/themes/*.json`. The index cache is
`…/org.hymnal.HymnFinder/index.bin` in the OS cache dir.

Point `hymns_url` at your own `hymns.zip`, or add libraries directly:

```toml
hymns_url = "https://example.com/hymns.zip"

[[libraries]]
name = "Imnuri Creștine"
path = "/path/to/hymns"
enabled = true
managed_by_git = false
```

### Tests

```sh
cargo test -p sda-core
```

Tests cover text extraction, accent folding, ranking, cache invalidation, zip
sync against a local HTTP stub, themes, presentation state, and text fit. They
use real `.pptx` fixtures in `crates/sda-core/tests/fixtures/`. `sda-gui` has
one unit test for fonts. The rest of the GUI is checked by hand.

### Fonts

Inter, Noto Serif, and Lora use the SIL Open Font License. Roboto Slab uses the
Apache 2.0 license.

### Releases

Run the **Release** workflow from the Actions tab with a tag like `v2.0.0`. It
publishes one archive per platform plus `hymns.zip` built from `assets/920`.
Running apps pick up the new `hymns.zip` on their next launch.
