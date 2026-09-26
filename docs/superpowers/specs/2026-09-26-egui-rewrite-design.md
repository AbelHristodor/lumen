# SDA Manager 2.0: egui rewrite

Date: 2026-09-26

## Goal

Replace the Slint GUI with an egui/eframe application built for live use.
One operator screen, a second fullscreen projector window, the same core
logic. Performance and a clean usable UI are the priorities. No customers
exist yet, so nothing needs to stay compatible except the on-disk hymn
library, config and theme files, which load unchanged.

## Workspace

```
crates/
  sda-core/   renamed from hymnal-core; logic, tested headlessly
  sda-gui/    new eframe app, binary `sda-manager`
assets/
  920/        hymn .pptx
  fonts/      bundled OFL fonts
  icon.png    window icon
```

Version 2.0.0. `update::BIN_NAME = "sda-manager"`. Installers, release
workflow and self-update use the new asset names
`sda-manager-<target>.{tar.gz,zip}`.

## Dependencies

sda-gui: `eframe`, `egui` (default features, `persistence` on),
`display-info`, `rfd`, `open`, `sys-locale`, `env_logger`, `log`,
`anyhow`, `sda-core`. Removed: `slint`, `slint-build`, `font-kit`,
`winresource` stays for the Windows icon.

## Core changes

### theme.rs

```rust
pub enum Font { Inter, NotoSerif, Lora, RobotoSlab }   // bundled
pub enum FontSize { Auto, Pt(f32) }
pub enum HAlign { Left, Center, Right }
pub enum VAlign { Top, Middle, Bottom }
pub struct Theme {
    name: String,
    font: Font,
    size: FontSize,
    bold: bool,
    text_color: Rgba,
    bg_color: Rgba,
    h_align: HAlign,
    v_align: VAlign,
    line_spacing: f32,   // multiplier
    margin: f32,         // fraction of the shorter side, 0.02..0.2
    footer: bool,        // "150 · Title" bottom line
}
```

Serde: `#[serde(default)]` on the struct so old JSON files with the
previous nested layout load as defaults for missing fields. Old nested
fields (`text`, `background`, `layout`) are ignored. `store` module
unchanged. Default theme: Inter, Auto, bold, white on `#0B1F3A`, center,
middle, 1.3, 0.06, footer off.

### fit.rs (new, pure)

`pub fn fit_size(measure: impl FnMut(f32) -> (f32, f32), bounds: (f32, f32), min: f32, max: f32) -> f32`
Binary search over point sizes until the measured galley fits `bounds`.
Tested with a synthetic linear `measure`.

### Everything else

`pptx`, `index`, `search`, `fold`, `library`, `sync`, `refresh`,
`downloader`, `update`, `i18n`, `present` unchanged except the crate rename.
`i18n::Strings` loses fields that no longer exist in the UI and gains the
new labels (Preview, Live, Go Live, Next, Videos, Settings, Display,
Projector on, Projector off, Blank). Adding a label stays a compile error
until all three languages supply it.

## GUI architecture

### State

```rust
struct App {
    core: Core,          // cfg, strings, searcher, present, theme list, active theme
    ui: UiState,         // query, hits: Vec<usize>, selected, preview_slide, open windows, draft theme
    projector: Option<ProjectorState>,   // viewport id, display index
    rx: Receiver<Msg>, tx: Sender<Msg>,
    displays: Vec<Display>,
    status: Status,      // library, sync, update messages
}
```

Plain fields. Workers get `Sender<Msg>` and an `egui::Context` clone and
call `request_repaint()` after `send`. `App::update` drains `rx` first,
then draws.

### Modules

```
main.rs        eframe boot, NativeOptions, icon, fonts, run_native
app.rs         App, Msg, update(): drain, shortcuts, layout of panels
core.rs        Core: load config, boot worker, force sync, reindex, save
search_panel.rs   search box + virtualized results
preview.rs     Preview card
live.rs        Live card + projector controls
projector.rs   deferred viewport, display list, open/close
slide.rs       slide::paint + galley layout + footer
settings.rs    Settings window: language, libraries, sync, update, themes
theme_editor.rs  thumbnails, form, live preview, save/delete/set active
videos.rs      Videos window
shortcuts.rs   key handling table
style.rs       dark visuals, spacing, fonts install
```

### Rendering rules

- Search runs only when `ui.query` differs from `ui.last_query`.
- Result rows use `ScrollArea::show_rows`; the selected row calls
  `scroll_to_me` when selection changed by keyboard.
- `slide::paint(painter, rect, theme, text, footer)` is the only place
  slide text is drawn. Auto size uses `fit.rs` with `Fonts::layout_job`
  as the measure. Result is cached per `(rect size, theme hash, text)` in
  `UiState` so the projector lays out once per slide change.
- Dark visuals, one accent color, 12 px spacing, 8 px rounding.

### Projector

`ctx.show_viewport_deferred(id, builder, |ctx, class| ...)`. Builder:
no decorations, title "Projector", position from `display-info` for the
chosen display, then `ViewportCommand::Fullscreen(true)` on first frame.
Closed with `ViewportCommand::Close`. If the user closes it from the OS,
`ctx.input(|i| i.viewport().close_requested())` clears `projector`.
The viewport paints black when `present.blank`, else `slide::paint` over
the full screen. Keys pressed while the projector has focus are read from
that viewport's input and applied to the same handlers.

### Go-live model

`ui.selected` drives Preview. `Go Live` copies the selected hymn into
`core.present` at slide 0 or the previewed slide. Live controls mutate
`core.present` only. Preview never touches the wall.

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

### Windows

Settings and Videos are `egui::Window`s, resizable, closable, remembered
by eframe persistence. Theme editor is a section of Settings with a
thumbnail strip, a two-column form and a live preview using `slide::paint`.

### Boot

1. `Core::load` reads config, registers the default library, picks
   language, loads the active theme.
2. Spawn boot worker: `load_local` → `Msg::Index`, `sync_default` →
   second `Msg::Index` on `Downloaded`, `check_and_stage_update` →
   `Msg::Update`.
3. First frame installs fonts and visuals, focuses the search box.

## Fonts

`assets/fonts/{Inter,NotoSerif,Lora,RobotoSlab}-{Regular,Bold}.ttf`,
OFL licenses copied next to them. Registered as named families
`"Inter"`, `"Inter Bold"`, etc. UI uses Inter as the proportional family.

## Release

Workflow builds `sda-manager` for the three targets, packages as
`sda-manager-<target>`, plus `hymns.zip`. `install.sh` and `install.ps1`
updated. Windows icon via `winresource` in `sda-gui/build.rs`.

## Testing

Core suite unchanged plus: theme JSON migration from the old nested
format, `fit_size` convergence, `Strings` completeness. GUI verified by
running the release binary through every flow.

## Out of scope

Gradient or image backgrounds, text shadow or outline, system font
enumeration, playlists, web build.
