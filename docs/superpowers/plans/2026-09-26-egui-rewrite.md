# SDA Manager 2.0 egui Rewrite Implementation Plan

> **For agentic workers:** Each task is self-contained. Read your task, the Shared Interfaces section, and the spec at `docs/superpowers/specs/2026-09-26-egui-rewrite-design.md`.

**Goal:** Replace the Slint GUI with an eframe/egui 0.36.2 app named `sda-manager`, rename the core crate to `sda-core`, trim the theme model, bundle fonts, update packaging.

**Architecture:** Two crates. `sda-core` holds all logic and tests. `sda-gui` is a plain-struct eframe `App`. `App::ui` drains one `mpsc` channel, applies shortcuts, then draws one operator screen, an immediate projector viewport, and two `egui::Window`s. Workers send `Msg` then call `ctx.request_repaint()`.

**Tech Stack:** Rust 1.98.1 stable, eframe/egui 0.36.2 with `persistence`, rfd 0.17, display-info 0.5, open 5, sys-locale 0.3, env_logger 0.11, log 0.4, anyhow 1.

## Global Constraints

- Workspace version `2.0.0`. Crates `sda-core`, `sda-gui`. Binary `sda-manager`. `update::BIN_NAME = "sda-manager"`. Release assets `sda-manager-<target>.tar.gz` for macOS and Linux, `sda-manager-<target>.zip` for Windows. `hymns.zip` unchanged.
- `library.rs` keeps `ProjectDirs::from("org", "hymnal", "HymnFinder")` so config, themes, hymns and cache load unchanged.
- egui API names below are verified against tag 0.36.2. Use them verbatim. `eframe::App::ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)` is the entry point. `App::update` does not exist. `SidePanel`/`TopBottomPanel` do not exist; use `egui::Panel::left/right/top/bottom(id)`. `ctx.style_mut` does not exist; use `ctx.all_styles_mut`.
- Projector uses `ctx.show_viewport_immediate`, not deferred. Deferred cannot borrow `App`.
- No new dependencies beyond the Tech Stack list. `slint`, `slint-build`, `font-kit` are removed.
- No stubs in the final tree. Wave 2 creates compile seams marked `// SEAM` that Wave 3 replaces. QA greps for `SEAM` and expects zero hits.
- Every task commits only its own files with `git add <paths>`. One commit per task. Prefixes: `feat`, `refactor`, `test`, `build`, `ci`, `docs`.
- Concurrent Wave 3 agents verify with `cargo check -p sda-gui`. If the only errors are in a sibling module they do not own, wait 60 s and retry up to 5 times, then report.
- Prose style in docs and comments: simple US English. No parentheses, dashes, semicolons.
- First build needs network to fetch egui 0.36.2.

## egui 0.36.2 API facts
- Panels: `egui::Panel::left(id).resizable(bool).default_size(f32).show(ui, |ui| ..)`, also `right/top/bottom`. `CentralPanel::default().show(ui, ..)`. `Ui` derefs to `Context`.
- Style: `ctx.all_styles_mut(|s| ..)`, `ctx.set_visuals(Visuals::dark())`.
- Viewports: `ViewportBuilder::default().with_title(..).with_decorations(false).with_monitor(idx)` gives borderless fullscreen on winit monitor idx. `ctx.show_viewport_immediate(ViewportId::from_hash_of("projector"), builder, |ui, class| ..)` runs on the main thread each frame; stop calling it to close. OS close: `ui.input(|i| i.viewport().close_requested())`.
- Wake from thread: `egui::Context` is Clone+Send+Sync; worker does `tx.send(msg); ctx.request_repaint();`. Drain `rx.try_recv()` at top of `ui`.
- Fonts: `FontDefinitions::default()`, `fonts.font_data.insert(name, Arc::new(FontData::from_static(bytes)))`, `fonts.families.insert(FontFamily::Name(name.into()), vec![name])`, `ctx.set_fonts(fonts)`. `FontId::new(size, family)`.
- Layout: `LayoutJob::simple(text, font_id, color, wrap_width)`, `job.halign = Align::Center`, `painter.layout_job(job) -> Arc<Galley>`, `galley.size()`, `painter.galley(pos, galley, color)`. With halign Center the galley x origin is the center. `painter.rect_filled(rect, 0.0, color)`.
- List: `ScrollArea::vertical().auto_shrink([false,false]).vertical_scroll_offset(f32).show_rows(ui, row_h, total, |ui, range| ..)`; `ui.selectable_label(sel, text)`.
- Input: `ui.input(|i| i.key_pressed(Key::X))`, `ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::X))`, `ui.memory(|m| m.focused())`, `ui.ctx().wants_keyboard_input()`. `TextEdit::singleline(&mut s).desired_width(f32::INFINITY).hint_text(..).id(Id::new("search"))`, `resp.request_focus()`.
- Widgets: `ui.color_edit_button_srgba(&mut Color32)`, `ComboBox::from_id_salt(..).selected_text(..).show_ui(ui, |ui| ui.selectable_value(&mut v, x, "l"))`, `ProgressBar::new(f).show_percentage().text(..)`, `Slider::new(&mut f, a..=b)`, `Frame::new().fill(c).inner_margin(12.0).corner_radius(8.0)`, `egui::Window::new(t).open(&mut bool).resizable(true).show(ctx, ..)`.
- eframe: `NativeOptions { viewport, centered: true, persist_window: true, ..Default::default() }`, `run_native(name, opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))`, `eframe::icon_data::from_png_bytes`.

## File Structure Map

Deleted: `crates/hymnal-gui/**`, `assets/icon-{control,download,library,settings,themes}.svg`.
Renamed: `crates/hymnal-core/` → `crates/sda-core/`; tests import `sda_core::`.
Modified core: `Cargo.toml`, `crates/sda-core/Cargo.toml`, `src/lib.rs` (`pub mod fit;`), `src/update.rs` (BIN_NAME), `src/theme.rs`, `src/i18n.rs`, `tests/theme_test.rs`, `tests/i18n_test.rs`, new `tests/fit_test.rs`, new `src/fit.rs`.
New GUI `crates/sda-gui/`: `Cargo.toml`, `build.rs`, `src/main.rs`, `app.rs`, `core.rs`, `style.rs`, `slide.rs`, `shortcuts.rs`, `search_panel.rs`, `preview.rs`, `live.rs`, `projector.rs`, `settings.rs`, `theme_editor.rs`, `videos.rs`.
New assets: `assets/fonts/{Inter,NotoSerif,Lora,RobotoSlab}-{Regular,Bold}.ttf`, `assets/fonts/OFL-{Inter,NotoSerif,Lora,RobotoSlab}.txt`.
Packaging: `.github/workflows/release.yml`, `install.sh`, `install.ps1`, `README.md`, `CLAUDE.md`.

## Shared Interfaces (fixed names, use verbatim)

```rust
// sda-core theme.rs
pub enum Font { Inter, NotoSerif, Lora, RobotoSlab }
impl Font { pub fn all() -> [Font; 4]; pub fn label(self) -> &'static str; }
pub enum FontSize { Auto, Pt(f32) }
pub enum HAlign { Left, Center, Right }   // unchanged
pub enum VAlign { Top, Middle, Bottom }   // unchanged
pub struct Theme { pub name: String, pub font: Font, pub size: FontSize, pub bold: bool,
  pub text_color: Rgba, pub bg_color: Rgba, pub h_align: HAlign, pub v_align: VAlign,
  pub line_spacing: f32, pub margin: f32, pub footer: bool }
impl Theme { pub fn cache_key(&self) -> u64; to_json; from_json; is_builtin_default }
// store module unchanged: list_themes, load_theme, save_theme, delete_theme

// sda-core fit.rs
pub fn fit_size(measure: impl FnMut(f32) -> (f32, f32), bounds: (f32, f32), min: f32, max: f32) -> f32;

// sda-gui app.rs
pub enum Msg { Index(Searcher), Synced(Result<Searcher, String>), Reindexed(Result<Searcher, String>),
  Update(String), Download(DownloadEvent) }
pub struct App { pub core: Core, pub ui: UiState, pub projector: Option<ProjectorState>,
  pub rx: Receiver<Msg>, pub tx: Sender<Msg>, pub displays: Vec<Display>, pub status: Status }
pub struct UiState { pub query: String, pub last_query: Option<String>, pub hits: Vec<usize>,
  pub selected: Option<usize>, pub scroll_to_selected: bool, pub preview_slide: usize,
  pub focus_search: bool, pub settings_open: bool, pub videos_open: bool,
  pub draft: Theme, pub draft_index: usize, pub fit: slide::FitCache, pub video: VideoState,
  pub library_status: String }
pub struct Status { pub library: String, pub sync: String, pub update: String, pub syncing: bool }
pub struct ProjectorState { pub display: usize }
pub struct VideoState { pub url: String, pub dir: PathBuf, pub title: String, pub percent: f32,
  pub speed: String, pub eta: String, pub phase: VideoPhase, pub message: String }
pub enum VideoPhase { Idle, Resolving, Downloading, Done, Failed }
impl App {
  pub fn new(cc: &eframe::CreationContext) -> Self;
  pub fn selected_entry(&self) -> Option<&HymnEntry>;
  pub fn move_selection(&mut self, delta: i32);
  pub fn go_live(&mut self, slide: usize);
  pub fn live_next(&mut self); pub fn live_prev(&mut self); pub fn live_blank(&mut self);
  pub fn live_jump(&mut self, n: usize);
  pub fn open_projector(&mut self, display: usize); pub fn stop_projector(&mut self);
  pub fn run_search(&mut self);
}

// sda-gui core.rs
pub struct Core { pub cfg: Config, pub cfg_path: Option<PathBuf>, pub language: Language,
  pub strings: Strings, pub searcher: Option<Searcher>, pub present: PresentationState,
  pub themes: Vec<Theme>, pub active_theme: Theme }
impl Core {
  pub fn load() -> Core; pub fn save_cfg(&self);
  pub fn set_language(&mut self, lang: Language);
  pub fn reload_themes(&mut self); pub fn set_active_theme(&mut self, t: Theme);
  pub fn spawn_boot(&self, tx: Sender<Msg>, ctx: egui::Context);
  pub fn spawn_force_sync(&self, tx: Sender<Msg>, ctx: egui::Context);
  pub fn spawn_reindex(&self, tx: Sender<Msg>, ctx: egui::Context);
  pub fn spawn_download(&self, url: String, dir: PathBuf, tx: Sender<Msg>, ctx: egui::Context);
}

// sda-gui style.rs
pub fn install(ctx: &egui::Context);
pub fn font_family(font: Font, bold: bool) -> egui::FontFamily;
pub fn color32(c: Rgba) -> egui::Color32;
pub fn rgba(c: egui::Color32) -> Rgba;
pub const ACCENT: egui::Color32;

// sda-gui slide.rs
pub struct FitCache { .. }  // Default
pub fn paint(painter: &egui::Painter, rect: egui::Rect, theme: &Theme, text: &str,
  footer: Option<&str>, cache: &mut FitCache);
pub fn footer_text(number: Option<&str>, title: &str) -> String;   // "150 · Title"

// sda-gui shortcuts.rs
pub fn handle(ui: &egui::Ui, app: &mut App);

// sda-gui projector.rs
pub struct Display { pub index: usize, pub label: String, pub is_primary: bool }
pub fn list_displays() -> Vec<Display>;
pub fn default_display(displays: &[Display]) -> usize;
pub fn show(ctx: &egui::Context, app: &mut App);

// panels: pub fn show(ui: &mut egui::Ui, app: &mut App)  -> search_panel, preview, live, theme_editor
// windows: pub fn show(ctx: &egui::Context, app: &mut App) -> settings, videos
```

## i18n::Strings field set after Task 3

Removed: `nav_library, nav_downloader, nav_settings, nav_themes, nav_control, downloader_heading, control_heading, control_start, control_stop, control_blank, control_prev, control_next, control_next_label, control_live, control_output, control_theme, library_project, theme_weight_label`.

Added (en / it / ro):

| field | en | it | ro |
|---|---|---|---|
| preview | Preview | Anteprima | Previzualizare |
| live | Live | In onda | În direct |
| go_live | Go Live | Manda in onda | Trimite în direct |
| next_label | Next | Prossima | Următoarea |
| videos | Videos | Video | Videoclipuri |
| settings | Settings | Impostazioni | Setări |
| display | Display | Schermo | Ecran |
| projector_on | Projector on | Accendi proiettore | Pornește proiectorul |
| projector_off | Projector off | Spegni proiettore | Oprește proiectorul |
| blank | Blank | Schermo nero | Ecran negru |
| stop | Stop | Ferma | Oprește |
| nothing_live | Nothing is live. Select a hymn and press Go Live. | Niente in onda. Scegli un inno e premi Manda in onda. | Nimic în direct. Alege un imn și apasă Trimite în direct. |
| theme_label | Theme | Tema | Temă |
| theme_bold_label | Bold | Grassetto | Aldin |
| theme_auto_size | Auto | Automatica | Automată |
| theme_valign_label | Vertical | Verticale | Vertical |
| theme_line_spacing_label | Line spacing | Interlinea | Spațiere rânduri |
| theme_margin_label | Margin | Margine | Margine |
| theme_footer_label | Footer with number and title | Piè di pagina con numero e titolo | Subsol cu număr și titlu |
| download_button | Download | Scarica | Descarcă |
| remove | Remove | Rimuovi | Elimină |
| active_suffix | (active) | (attivo) | (activ) |

All other fields keep their current values. Total field count 70.

---
## Wave 0

### Task 0: Rename crates and remove Slint GUI
Files: rename `crates/hymnal-core` → `crates/sda-core`; modify `Cargo.toml`, `crates/sda-core/Cargo.toml`, `crates/sda-core/src/update.rs`, all `crates/sda-core/tests/*.rs`; delete `crates/hymnal-gui/`, five `assets/icon-*.svg`.
- `git mv crates/hymnal-core crates/sda-core && git rm -rq crates/hymnal-gui assets/icon-control.svg assets/icon-download.svg assets/icon-library.svg assets/icon-settings.svg assets/icon-themes.svg`
- `Cargo.toml`: `members = ["crates/sda-core"]`, `version = "2.0.0"`, profile comment says egui.
- `crates/sda-core/Cargo.toml`: `name = "sda-core"`.
- `update.rs`: `const BIN_NAME: &str = "sda-manager";`, comment `sda-manager-<target>/sda-manager`.
- `grep -rl hymnal_core crates/sda-core/tests | xargs sed -i '' 's/hymnal_core/sda_core/g'`
- Verify: `cargo test -p sda-core` all ok; `grep -rn "hymnal-gui\|hymnal_core\|hymnal-core" crates Cargo.toml` empty.
- Commit: `refactor: rename hymnal-core to sda-core, drop Slint GUI, version 2.0.0`

## Wave 1 (parallel, disjoint files)

### Task 1: Flat theme model with legacy JSON compat
Files: `crates/sda-core/src/theme.rs`, `crates/sda-core/tests/theme_test.rs`. Keep `Rgba`, `HAlign`, `VAlign`, `store` unchanged. Keep the three `rgba_*` tests.

Tests to add in `theme_test.rs`:
```rust
use sda_core::theme::{Font, FontSize, HAlign, Rgba, Theme, VAlign};

#[test]
fn theme_json_round_trips() {
    let t = Theme::default();
    assert_eq!(Theme::from_json(&t.to_json().unwrap()).unwrap(), t);
}
#[test]
fn default_theme_matches_spec() {
    let t = Theme::default();
    assert_eq!(t.name, "Default"); assert_eq!(t.font, Font::Inter); assert_eq!(t.size, FontSize::Auto);
    assert!(t.bold); assert_eq!(t.text_color, Rgba::new(255, 255, 255, 255));
    assert_eq!(t.bg_color, Rgba::new(11, 31, 58, 255)); assert_eq!(t.h_align, HAlign::Center);
    assert_eq!(t.v_align, VAlign::Middle); assert_eq!(t.line_spacing, 1.3); assert_eq!(t.margin, 0.06); assert!(!t.footer);
}
#[test]
fn custom_theme_round_trips_all_fields() {
    let t = Theme { name: "Christmas".into(), font: Font::Lora, size: FontSize::Pt(48.0), bold: false,
        text_color: Rgba::new(1, 2, 3, 255), bg_color: Rgba::new(4, 5, 6, 255), h_align: HAlign::Left,
        v_align: VAlign::Bottom, line_spacing: 1.1, margin: 0.1, footer: true };
    assert_eq!(Theme::from_json(&t.to_json().unwrap()).unwrap(), t);
}
const OLD_JSON: &str = r##"{
  "name": "Old",
  "text": {"font_family":"Georgia","font_size_pt":48.0,"font_weight":700,
    "color":{"r":255,"g":255,"b":255,"a":255},"h_align":"Center","v_align":"Middle","line_spacing":1.3,
    "shadow":{"enabled":false,"color":{"r":0,"g":0,"b":0,"a":180},"blur":4.0,"dx":2.0,"dy":2.0},
    "outline":{"enabled":false,"color":{"r":0,"g":0,"b":0,"a":255},"width":1.0}},
  "background": {"kind":{"Solid":{"color":{"r":11,"g":31,"b":58,"a":255}}},"image_fit":"Cover",
    "overlay_color":{"r":0,"g":0,"b":0,"a":255},"overlay_opacity":0.0},
  "layout": {"margin":48.0,"max_text_width":0.8},
  "footer": {"show":true,"content":"HymnNumberTitle"}
}"##;
#[test]
fn old_nested_json_loads_with_defaults_and_footer_show() {
    let t = Theme::from_json(OLD_JSON).unwrap();
    assert_eq!(t.name, "Old"); assert_eq!(t.font, Font::Inter); assert_eq!(t.size, FontSize::Auto);
    assert!(t.footer, "old footer.show must map to the new bool");
}
#[test]
fn missing_fields_take_defaults() {
    let t = Theme::from_json(r#"{"name":"Bare"}"#).unwrap();
    assert_eq!(t.name, "Bare"); assert_eq!(t.bg_color, Theme::default().bg_color);
}
#[test]
fn cache_key_changes_with_any_field() {
    let a = Theme::default(); let mut b = a.clone(); b.margin = 0.07;
    assert_ne!(a.cache_key(), b.cache_key()); assert_eq!(a.cache_key(), Theme::default().cache_key());
}
#[test]
fn font_labels_and_all() { assert_eq!(Font::all().len(), 4); assert_eq!(Font::NotoSerif.label(), "Noto Serif"); }
```

Implementation for `theme.rs` (replace Shadow, Outline, TextStyle, Background, ImageFit, BackgroundStyle, LayoutStyle, FooterContent, FooterStyle, Theme):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Font { Inter, NotoSerif, Lora, RobotoSlab }
impl Font {
    pub fn all() -> [Font; 4] { [Font::Inter, Font::NotoSerif, Font::Lora, Font::RobotoSlab] }
    pub fn label(self) -> &'static str {
        match self { Font::Inter => "Inter", Font::NotoSerif => "Noto Serif", Font::Lora => "Lora", Font::RobotoSlab => "Roboto Slab" }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum FontSize { #[default] Auto, Pt(f32) }

/// Old files stored `footer` as `{show, content}`. Accept a bool or that object.
fn footer_compat<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    #[derive(Deserialize)] #[serde(untagged)]
    enum F { Flag(bool), Old { show: bool } }
    Ok(match F::deserialize(d)? { F::Flag(b) => b, F::Old { show } => show })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub name: String, pub font: Font, pub size: FontSize, pub bold: bool,
    pub text_color: Rgba, pub bg_color: Rgba, pub h_align: HAlign, pub v_align: VAlign,
    pub line_spacing: f32, pub margin: f32,
    #[serde(deserialize_with = "footer_compat")] pub footer: bool,
}
impl Default for Theme { fn default() -> Self { Theme { name: "Default".into(), font: Font::Inter, size: FontSize::Auto, bold: true,
    text_color: Rgba::new(255,255,255,255), bg_color: Rgba::new(11,31,58,255), h_align: HAlign::Center, v_align: VAlign::Middle,
    line_spacing: 1.3, margin: 0.06, footer: false } } }
impl Theme {
    pub fn to_json(&self) -> anyhow::Result<String> { Ok(serde_json::to_string_pretty(self)?) }
    pub fn from_json(text: &str) -> anyhow::Result<Theme> { Ok(serde_json::from_str(text)?) }
    pub fn is_builtin_default(&self) -> bool { self.name == "Default" }
    /// Stable hash of every field, used by the GUI layout cache.
    pub fn cache_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.name.hash(&mut h); self.font.hash(&mut h); self.bold.hash(&mut h);
        match self.size { FontSize::Auto => 0u32.hash(&mut h), FontSize::Pt(p) => p.to_bits().hash(&mut h) }
        (self.text_color.r, self.text_color.g, self.text_color.b, self.text_color.a).hash(&mut h);
        (self.bg_color.r, self.bg_color.g, self.bg_color.b, self.bg_color.a).hash(&mut h);
        (self.h_align as u8, self.v_align as u8).hash(&mut h);
        self.line_spacing.to_bits().hash(&mut h); self.margin.to_bits().hash(&mut h); self.footer.hash(&mut h);
        h.finish()
    }
}
```
Verify: `cargo test -p sda-core --test theme_test --test theme_store_test` all pass. Commit: `feat(core): flat theme model with bundled fonts and legacy JSON compat`.

### Task 2: fit.rs
Files: create `crates/sda-core/src/fit.rs`, `crates/sda-core/tests/fit_test.rs`; add `pub mod fit;` to `lib.rs`.
```rust
// tests/fit_test.rs
use sda_core::fit::fit_size;
fn linear(pt: f32) -> (f32, f32) { (pt * 10.0, pt * 2.0) }
#[test] fn converges_to_largest_size_that_fits() { let pt = fit_size(linear, (1000.0, 300.0), 8.0, 400.0); assert!(pt <= 100.0 && pt > 99.0, "got {pt}"); }
#[test] fn clamps_to_max_when_everything_fits() { assert_eq!(fit_size(linear, (1e9, 1e9), 8.0, 120.0), 120.0); }
#[test] fn returns_min_when_nothing_fits() { assert_eq!(fit_size(linear, (1.0, 1.0), 8.0, 120.0), 8.0); }
#[test] fn calls_measure_at_most_twenty_times() { let mut n = 0; fit_size(|pt| { n += 1; linear(pt) }, (1000.0, 300.0), 8.0, 400.0); assert!(n <= 20, "measure called {n} times"); }
```
```rust
// src/fit.rs
//! Pure binary search for the largest font size whose measured text fits.
pub fn fit_size(mut measure: impl FnMut(f32) -> (f32, f32), bounds: (f32, f32), min: f32, max: f32) -> f32 {
    let fits = |m: (f32, f32)| m.0 <= bounds.0 && m.1 <= bounds.1;
    if fits(measure(max)) { return max; }
    if !fits(measure(min)) { return min; }
    let (mut lo, mut hi) = (min, max);
    while hi - lo > 0.5 { let mid = (lo + hi) / 2.0; if fits(measure(mid)) { lo = mid } else { hi = mid } }
    lo
}
```
Verify: 4 passed. Commit: `feat(core): fit_size binary search for auto font size`.

### Task 3: i18n Strings for the new UI
Files: `crates/sda-core/src/i18n.rs`, `crates/sda-core/tests/i18n_test.rs`. Apply the field table in Shared section. Update struct, `as_fields`, `en`, `it`, `ro`. Keep both existing tests; add:
```rust
#[test]
fn italian_and_romanian_are_translated_not_copied() {
    let en = Strings::for_language(Language::En);
    for lang in [Language::It, Language::Ro] {
        let s = Strings::for_language(lang);
        assert_ne!(s.go_live, en.go_live, "{lang:?}"); assert_ne!(s.projector_on, en.projector_on, "{lang:?}");
        assert_ne!(s.nothing_live, en.nothing_live, "{lang:?}"); assert_ne!(s.theme_footer_label, en.theme_footer_label, "{lang:?}");
    }
}
#[test]
fn field_count_matches_struct() { assert_eq!(Strings::for_language(Language::En).as_fields().len(), 70); }
```
Verify: `cargo test -p sda-core --test i18n_test` 4 passed. Commit: `feat(core): i18n strings for the single-screen egui UI`.

### Task 4: Bundle OFL fonts
Files: `assets/fonts/{Inter,NotoSerif,Lora,RobotoSlab}-{Regular,Bold}.ttf`, `assets/fonts/OFL-{Inter,NotoSerif,Lora,RobotoSlab}.txt`.
```bash
R=/Users/abelor/projects/sda_manager/assets/fonts; mkdir -p $R; cd /tmp && rm -rf fontwork && mkdir fontwork && cd fontwork
curl -fL -o inter.zip https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip && unzip -q inter.zip -d inter
cp inter/extras/ttf/Inter-Regular.ttf $R/Inter-Regular.ttf; cp inter/extras/ttf/Inter-Bold.ttf $R/Inter-Bold.ttf; cp inter/LICENSE.txt $R/OFL-Inter.txt
N=https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts/NotoSerif/unhinted/ttf
curl -fL -o $R/NotoSerif-Regular.ttf $N/NotoSerif-Regular.ttf; curl -fL -o $R/NotoSerif-Bold.ttf $N/NotoSerif-Bold.ttf
curl -fL -o $R/OFL-NotoSerif.txt https://raw.githubusercontent.com/google/fonts/main/ofl/notoserif/OFL.txt
L=https://raw.githubusercontent.com/cyrealtype/Lora-Cyrillic/master
curl -fL -o $R/Lora-Regular.ttf $L/fonts/ttf/Lora-Regular.ttf; curl -fL -o $R/Lora-Bold.ttf $L/fonts/ttf/Lora-Bold.ttf; curl -fL -o $R/OFL-Lora.txt $L/OFL.txt
S=https://raw.githubusercontent.com/googlefonts/robotoslab/main/fonts/static
curl -fL -o $R/RobotoSlab-Regular.ttf $S/RobotoSlab-Regular.ttf; curl -fL -o $R/RobotoSlab-Bold.ttf $S/RobotoSlab-Bold.ttf
curl -fL -o $R/LICENSE-RobotoSlab-Apache-2.0.txt https://raw.githubusercontent.com/google/fonts/main/ofl/robotoslab/OFL.txt
```
If Inter zip layout differs: `unzip -l inter.zip | grep -E "Inter-(Regular|Bold)\.ttf"`. Fallback for any dead TTF URL, Google Fonts CSS API serves static TTF to curl:
```bash
fam="Roboto Slab"; css=$(curl -fsL -A "curl" "https://fonts.googleapis.com/css2?family=${fam// /+}:wght@400;700")
echo "$css" | grep -oE "https://fonts.gstatic.com/[^)]+\.ttf"   # 400 then 700
```
License fallback: `https://cdn.jsdelivr.net/npm/@fontsource/<slug>@5/LICENSE`.
Verify: `file assets/fonts/*.ttf` all TrueType; `wc -c` each > 50000; `ls assets/fonts | wc -l` = 12. Commit: `build: bundle Inter, Noto Serif, Lora, Roboto Slab under OFL`.

---
## Wave 2

### Task 5: sda-gui crate scaffold with boot flow, layout, slide painter, shortcuts, projector, seams
Files: create `crates/sda-gui/{Cargo.toml,build.rs}`, `src/{main,app,core,style,slide,shortcuts,projector}.rs`, seams `src/{search_panel,preview,live,settings,theme_editor,videos}.rs`. Modify workspace `Cargo.toml` members.

`Cargo.toml`:
```toml
[package]
name = "sda-gui"
edition.workspace = true
version.workspace = true
[[bin]]
name = "sda-manager"
path = "src/main.rs"
[dependencies]
sda-core = { path = "../sda-core" }
eframe = { version = "0.36.2", features = ["persistence"] }
egui = "0.36.2"
display-info = "0.5"
rfd = "0.17"
open = "5"
sys-locale = "0.3"
anyhow = { workspace = true }
log = "0.4"
env_logger = "0.11"
[target.'cfg(windows)'.build-dependencies]
winresource = "0.1"
```
`build.rs`: winresource block under `#[cfg(windows)]` setting `assets/icon.ico`, plus `println!("cargo:rerun-if-changed=../../assets/icon.ico")`. No slint.

`main.rs`:
```rust
#![windows_subsystem = "windows"]
mod app; mod core; mod live; mod preview; mod projector; mod search_panel; mod settings;
mod shortcuts; mod slide; mod style; mod theme_editor; mod videos;
use std::sync::Arc;
fn main() -> eframe::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("sda_gui=info,sda_core=info")).init();
    log::info!("starting SDA Manager {}", env!("CARGO_PKG_VERSION"));
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon.png")).expect("icon.png");
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("SDA Manager")
            .with_inner_size([1100.0, 700.0]).with_min_inner_size([900.0, 600.0]).with_icon(Arc::new(icon)),
        centered: true, persist_window: true, ..Default::default()
    };
    eframe::run_native("SDA Manager", opts, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
```

`core.rs`: `Core::load()` reads config via `library::config_path` + `Config::load`, calls `refresh::register_default_library(&mut cfg)`, language from `cfg.language` via `Language::from_code` else `sys_locale::get_locale` via `Language::from_locale` else En, `strings = Strings::for_language`, themes via `store::list_themes(themes_dir)`, active theme by `cfg.active_theme` via `store::load_theme` else `Theme::default()`. `save_cfg` is the only writer. `set_language` sets language, strings, cfg.language, saves. `set_active_theme` sets active_theme, cfg.active_theme, saves. `reload_themes` rereads. Workers:
```rust
pub fn spawn_boot(&self, tx: Sender<Msg>, ctx: egui::Context) {
    let cfg = self.cfg.clone(); let s = self.strings.clone();
    std::thread::spawn(move || {
        let send = |m: Msg| { let _ = tx.send(m); ctx.request_repaint(); };
        if refresh::default_library_present() {
            send(Msg::Index(Searcher::new(refresh::load_local(cfg.clone()))));
            if refresh::sync_default(&cfg) == Some(SyncOutcome::Downloaded) {
                send(Msg::Index(Searcher::new(refresh::load_local(cfg))));
            }
        } else { send(Msg::Index(Searcher::new(refresh::load_library(cfg, false)))); }
        let msg = match update::check_and_stage_update() {
            Ok(UpdateOutcome::UpToDate) => s.update_up_to_date,
            Ok(UpdateOutcome::Updated { version }) => s.update_staged_fmt.replace("{}", &version),
            Ok(UpdateOutcome::CheckUnavailable) => s.update_unavailable,
            Err(e) => { log::warn!("update check failed: {e}"); s.update_failed }
        };
        send(Msg::Update(msg));
    });
}
```
`spawn_force_sync`: `refresh::force_clean()` then `Msg::Synced(Ok(Searcher::new(refresh::load_library(cfg, true))))` else `Err(format!("force clean failed: {e}"))`. `spawn_reindex`: `Msg::Reindexed(Ok(Searcher::new(refresh::load_local(cfg))))`. `spawn_download`: `downloader::download(&url, &dir, |ev| { let _ = tx.send(Msg::Download(ev)); ctx.request_repaint(); })`.

`style.rs`:
```rust
pub const ACCENT: Color32 = Color32::from_rgb(79, 140, 255);
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let all: [(&str, &'static [u8]); 8] = [
        ("Inter", include_bytes!("../../../assets/fonts/Inter-Regular.ttf")),
        ("Inter Bold", include_bytes!("../../../assets/fonts/Inter-Bold.ttf")),
        ("Noto Serif", include_bytes!("../../../assets/fonts/NotoSerif-Regular.ttf")),
        ("Noto Serif Bold", include_bytes!("../../../assets/fonts/NotoSerif-Bold.ttf")),
        ("Lora", include_bytes!("../../../assets/fonts/Lora-Regular.ttf")),
        ("Lora Bold", include_bytes!("../../../assets/fonts/Lora-Bold.ttf")),
        ("Roboto Slab", include_bytes!("../../../assets/fonts/RobotoSlab-Regular.ttf")),
        ("Roboto Slab Bold", include_bytes!("../../../assets/fonts/RobotoSlab-Bold.ttf")),
    ];
    for (name, bytes) in all {
        fonts.font_data.insert(name.into(), Arc::new(FontData::from_static(bytes)));
        fonts.families.insert(FontFamily::Name(name.into()), vec![name.into()]);
    }
    fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "Inter".into());
    ctx.set_fonts(fonts);
    ctx.set_visuals(Visuals::dark());
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(12.0, 12.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        for w in [&mut s.visuals.widgets.noninteractive, &mut s.visuals.widgets.inactive, &mut s.visuals.widgets.hovered, &mut s.visuals.widgets.active] { w.corner_radius = 8.into(); }
        s.visuals.selection.bg_fill = ACCENT; s.visuals.hyperlink_color = ACCENT;
    });
}
pub fn font_family(font: Font, bold: bool) -> FontFamily {
    let base = font.label();
    FontFamily::Name(if bold { format!("{base} Bold").into() } else { base.into() })
}
pub fn color32(c: Rgba) -> Color32 { Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a) }
pub fn rgba(c: Color32) -> Rgba { Rgba::new(c.r(), c.g(), c.b(), 255) }
#[cfg(test)]
mod tests {
    #[test]
    fn fonts_install_without_panic() {
        let ctx = egui::Context::default(); super::install(&ctx);
        let _ = ctx.run(Default::default(), |ctx| { egui::CentralPanel::default().show(ctx, |ui| {
            for f in sda_core::theme::Font::all() { ui.label(egui::RichText::new("Plecaţi-vă").family(super::font_family(f, true))); } }); });
    }
}
```
(If `CentralPanel::show` needs a `Ui` in 0.36, use `ctx.run(.., |ctx| { let mut ui = ...` per compiler guidance, or `egui::Area`. Adjust to compile.)

`slide.rs`:
```rust
#[derive(Default)]
pub struct FitCache { theme_key: u64, sizes: HashMap<(u32, u32, String), f32> }
pub fn footer_text(number: Option<&str>, title: &str) -> String {
    match number { Some(n) => format!("{n} · {title}"), None => title.to_string() }
}
pub fn paint(painter: &Painter, rect: Rect, theme: &Theme, text: &str, footer: Option<&str>, cache: &mut FitCache) {
    painter.rect_filled(rect, 0.0, style::color32(theme.bg_color));
    let short = rect.width().min(rect.height()); let m = short * theme.margin;
    let footer_h = if theme.footer && footer.is_some() { short * 0.05 } else { 0.0 };
    let area = Rect::from_min_max(rect.min + vec2(m, m), rect.max - vec2(m, m + footer_h));
    let fam = style::font_family(theme.font, theme.bold); let color = style::color32(theme.text_color);
    let halign = match theme.h_align { HAlign::Left => Align::LEFT, HAlign::Center => Align::Center, HAlign::Right => Align::RIGHT };
    let job_at = |pt: f32| {
        let mut job = LayoutJob::simple(text.to_owned(), FontId::new(pt, fam.clone()), color, area.width());
        job.halign = halign;
        for s in &mut job.sections { s.format.line_height = Some(pt * theme.line_spacing); }
        job
    };
    let pt = match theme.size {
        FontSize::Pt(p) => p,
        FontSize::Auto => {
            if cache.theme_key != theme.cache_key() || cache.sizes.len() > 64 { cache.sizes.clear(); cache.theme_key = theme.cache_key(); }
            let key = (area.width() as u32, area.height() as u32, text.to_owned());
            *cache.sizes.entry(key).or_insert_with(|| fit_size(|p| { let g = painter.layout_job(job_at(p)); (g.size().x, g.size().y) }, (area.width(), area.height()), 8.0, short * 0.5))
        }
    };
    let galley = painter.layout_job(job_at(pt)); let size = galley.size();
    let x = match theme.h_align { HAlign::Left => area.left(), HAlign::Center => area.center().x, HAlign::Right => area.right() };
    let y = match theme.v_align { VAlign::Top => area.top(), VAlign::Middle => area.center().y - size.y / 2.0, VAlign::Bottom => area.bottom() - size.y };
    painter.galley(pos2(x, y), galley, color);
    if let (true, Some(f)) = (theme.footer, footer) {
        let fpt = (short * 0.025).max(14.0);
        let mut job = LayoutJob::simple(f.to_owned(), FontId::new(fpt, fam), color.gamma_multiply(0.8), area.width());
        job.halign = Align::Center; let g = painter.layout_job(job);
        painter.galley(pos2(area.center().x, rect.bottom() - m - g.size().y), g, color);
    }
}
```
If `TextFormat.line_height` does not exist in 0.36, drop those two lines and note it in the commit.

`projector.rs`:
```rust
pub struct Display { pub index: usize, pub label: String, pub is_primary: bool }
pub fn list_displays() -> Vec<Display> {
    match display_info::DisplayInfo::all() {
        Ok(l) if !l.is_empty() => l.into_iter().enumerate().map(|(i, d)| Display { index: i, label: format!("{} ({}x{})", d.name, d.width, d.height), is_primary: d.is_primary }).collect(),
        other => { if let Err(e) = other { log::warn!("display enumeration failed: {e}"); } vec![Display { index: 0, label: "Primary".into(), is_primary: true }] }
    }
}
pub fn default_display(displays: &[Display]) -> usize { displays.iter().find(|d| !d.is_primary).or_else(|| displays.first()).map(|d| d.index).unwrap_or(0) }
pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(display) = app.projector.as_ref().map(|p| p.display) else { return };
    let builder = egui::ViewportBuilder::default().with_title("Projector").with_decorations(false).with_monitor(display);
    let mut close = false;
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("projector"), builder, |ui, _class| {
        if ui.input(|i| i.viewport().close_requested()) { close = true; }
        shortcuts::handle(ui, app);
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(egui::Color32::BLACK)).show(ui, |ui| {
            let rect = ui.max_rect();
            if app.core.present.blank { ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK); return; }
            match app.core.present.current_slide() {
                Some(text) => { let f = slide::footer_text(app.core.present.number.as_deref(), &app.core.present.title);
                    let text = text.to_owned(); slide::paint(ui.painter(), rect, &app.core.active_theme, &text, Some(&f), &mut app.ui.fit); }
                None => { ui.painter().rect_filled(rect, 0.0, style::color32(app.core.active_theme.bg_color)); }
            }
        });
    });
    if close { app.projector = None; }
}
```
Fallback if `Frame::NONE` is not a const in 0.36: `egui::Frame::new().fill(..)`.

`shortcuts.rs`:
```rust
pub fn handle(ui: &egui::Ui, app: &mut App) {
    let search_id = egui::Id::new("search");
    let search_focused = ui.memory(|m| m.focused()) == Some(search_id);
    let typing = ui.ctx().wants_keyboard_input();
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::K)) { app.ui.focus_search = true; }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) { app.stop_projector(); }
    if search_focused || !typing {
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) { app.move_selection(1); }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) { app.move_selection(-1); }
    }
    if search_focused && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)) { let s = app.ui.preview_slide; app.go_live(s); }
    if !typing {
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) || i.consume_key(egui::Modifiers::NONE, egui::Key::Space)) { app.live_next(); }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)) { app.live_prev(); }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::B)) { app.live_blank(); }
        let nums = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7, egui::Key::Num8, egui::Key::Num9];
        for (n, k) in nums.into_iter().enumerate() { if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k)) { app.live_jump(n + 1); } }
    }
}
```

`app.rs`: define all Shared Interface types. `App::new(cc)`: `style::install(&cc.egui_ctx)`, `Core::load()`, `mpsc::channel`, `displays = projector::list_displays()`, `status.library = strings.status_loading`, `status.update = strings.update_checking`, `ui.focus_search = true`, `ui.draft = core.active_theme.clone()`, `ui.draft_index = index of active in themes else 0`, `ui.video.dir = library::downloads_dir(&core.cfg)`, `core.spawn_boot(tx.clone(), cc.egui_ctx.clone())`. Methods:
```rust
pub fn selected_entry(&self) -> Option<&HymnEntry> { let idx = *self.ui.hits.get(self.ui.selected?)?; self.core.searcher.as_ref()?.entry(idx) }
pub fn move_selection(&mut self, delta: i32) {
    if self.ui.hits.is_empty() { return; }
    let cur = self.ui.selected.unwrap_or(0) as i32;
    let next = (cur + delta).clamp(0, self.ui.hits.len() as i32 - 1) as usize;
    self.ui.selected = Some(next); self.ui.preview_slide = 0; self.ui.scroll_to_selected = true;
}
pub fn run_search(&mut self) {
    let Some(s) = self.core.searcher.as_mut() else { return };
    self.ui.hits = s.search(&self.ui.query).iter().map(|h| h.index).collect();
    self.ui.last_query = Some(self.ui.query.clone());
    self.ui.selected = if self.ui.hits.is_empty() { None } else { Some(0) };
    self.ui.preview_slide = 0; self.ui.scroll_to_selected = true;
}
pub fn go_live(&mut self, slide: usize) {
    let Some(e) = self.selected_entry().cloned() else { return };
    self.core.present.load_hymn(e.number, e.title, e.slides);
    self.core.present.index = slide.min(self.core.present.slide_count().saturating_sub(1));
}
pub fn live_next(&mut self) { self.core.present.next(); }
pub fn live_prev(&mut self) { self.core.present.prev(); }
pub fn live_blank(&mut self) { self.core.present.toggle_blank(); }
pub fn live_jump(&mut self, n: usize) { if n >= 1 && n <= self.core.present.slide_count() { self.core.present.index = n - 1; } }
pub fn open_projector(&mut self, display: usize) { self.projector = Some(ProjectorState { display }); self.core.cfg.output_display = Some(display as i32); self.core.save_cfg(); }
pub fn stop_projector(&mut self) { self.projector = None; }
fn drain(&mut self) {
    while let Ok(m) = self.rx.try_recv() {
        match m {
            Msg::Index(s) => { self.core.searcher = Some(s); self.status.library = self.core.strings.status_library_ready.clone(); self.ui.last_query = None; }
            Msg::Synced(Ok(s)) => { self.status.syncing = false; self.status.sync = self.core.strings.status_synced_fmt.replace("{}", &s.len().to_string()); self.core.searcher = Some(s); self.ui.last_query = None; }
            Msg::Synced(Err(e)) => { self.status.syncing = false; self.status.sync = self.core.strings.status_sync_failed_fmt.replace("{}", &e); }
            Msg::Reindexed(Ok(s)) => { self.ui.library_status = self.core.strings.status_indexed_fmt.replacen("{}", &s.len().to_string(), 1); self.core.searcher = Some(s); self.ui.last_query = None; }
            Msg::Reindexed(Err(e)) => { self.ui.library_status = self.core.strings.status_indexing_failed_fmt.replacen("{}", &e, 1); }
            Msg::Update(s) => self.status.update = s,
            Msg::Download(ev) => self.apply_download(ev),
        }
    }
}
fn apply_download(&mut self, ev: DownloadEvent) {
    let v = &mut self.ui.video;
    match ev {
        DownloadEvent::Resolving => v.phase = VideoPhase::Resolving,
        DownloadEvent::Title(t) => v.title = t,
        DownloadEvent::Progress(p) => { v.phase = VideoPhase::Downloading; v.percent = p.percent; v.speed = p.speed; v.eta = p.eta; }
        DownloadEvent::Done { .. } => { v.phase = VideoPhase::Done; v.percent = 100.0; }
        DownloadEvent::Failed { message } => { v.phase = VideoPhase::Failed; v.message = message; }
    }
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain();
        if self.ui.last_query.as_deref() != Some(self.ui.query.as_str()) { self.run_search(); }
        shortcuts::handle(ui, self);
        let s = self.core.strings.clone();
        egui::Panel::top("toolbar").show(ui, |ui| ui.horizontal(|ui| {
            ui.heading(&s.app_title);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(&s.settings).clicked() { self.ui.settings_open = true; }
                if ui.button(&s.videos).clicked() { self.ui.videos_open = true; }
            });
        }));
        egui::Panel::bottom("status").show(ui, |ui| ui.horizontal(|ui| {
            ui.label(&self.status.library); ui.separator(); ui.label(&self.status.update);
            if self.projector.is_some() { ui.separator(); ui.colored_label(style::ACCENT, &s.live); }
        }));
        egui::Panel::left("search").resizable(true).default_size(340.0).show(ui, |ui| search_panel::show(ui, self));
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                let w = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0; let h = ui.available_height();
                ui.allocate_ui_with_layout(egui::vec2(w, h), egui::Layout::top_down(egui::Align::Min), |ui| preview::show(ui, self));
                ui.allocate_ui_with_layout(egui::vec2(w, h), egui::Layout::top_down(egui::Align::Min), |ui| live::show(ui, self));
            });
        });
        settings::show(ui.ctx(), self); videos::show(ui.ctx(), self); projector::show(ui.ctx(), self);
    }
}
```
Seams: `search_panel.rs`, `preview.rs`, `live.rs`, `theme_editor.rs` each `pub fn show(ui: &mut egui::Ui, app: &mut App) { // SEAM\n let _ = app; ui.label("SEAM"); }`. `settings.rs` and `videos.rs`: `pub fn show(ctx: &egui::Context, app: &mut App) { // SEAM\n if !app.ui.settings_open { return; } egui::Window::new("Settings").open(&mut app.ui.settings_open).show(ctx, |ui| { ui.label("SEAM"); }); }` with the videos flag for videos.

Verify: `cargo test -p sda-gui` 1 passed; `cargo build -p sda-gui` Finished; `timeout 15 cargo run -p sda-gui 2>&1 | grep -E "starting SDA Manager|total .* hymns"` shows both. Commit: `feat(gui): eframe scaffold with boot flow, slide painter, shortcuts, projector viewport`.

---
## Wave 3 (parallel, one file each). Replace exactly one seam file. Do not edit any other file. If a needed field or method is missing, stop and report. Remove the `// SEAM` marker.

### Task 6: search_panel.rs
```rust
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let s = app.core.strings.clone();
    let resp = ui.add(egui::TextEdit::singleline(&mut app.ui.query).desired_width(f32::INFINITY).hint_text(&s.search_placeholder).id(egui::Id::new("search")));
    if app.ui.focus_search { resp.request_focus(); app.ui.focus_search = false; }
    ui.horizontal(|ui| {
        let has = app.selected_entry().is_some();
        if ui.add_enabled(has, egui::Button::new(&s.go_live).fill(style::ACCENT)).clicked() { let p = app.ui.preview_slide; app.go_live(p); }
        if ui.add_enabled(has, egui::Button::new(&s.open_in_powerpoint)).clicked() {
            if let Some(e) = app.selected_entry() { if let Err(err) = open::that(&e.path) { log::warn!("open failed: {err}"); } } }
        if ui.add_enabled(has, egui::Button::new(&s.reveal_in_folder)).clicked() {
            if let Some(p) = app.selected_entry().and_then(|e| e.path.parent().map(|p| p.to_path_buf())) { let _ = open::that(p); } }
    });
    ui.separator();
    let row_h = ui.text_style_height(&egui::TextStyle::Body) + 8.0;
    let total = app.ui.hits.len();
    let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
    if app.ui.scroll_to_selected {
        if let Some(sel) = app.ui.selected {
            area = area.vertical_scroll_offset((sel as f32 * (row_h + ui.spacing().item_spacing.y) - ui.available_height() / 2.0).max(0.0));
        }
        app.ui.scroll_to_selected = false;
    }
    area.show_rows(ui, row_h, total, |ui, range| {
        let mut clicked = None; let mut double = None;
        for row in range {
            let idx = app.ui.hits[row];
            let label = app.core.searcher.as_ref().and_then(|s| s.label(idx)).unwrap_or("").to_owned();
            let r = ui.selectable_label(app.ui.selected == Some(row), label);
            if r.clicked() { clicked = Some(row); }
            if r.double_clicked() { double = Some(row); }
        }
        if let Some(row) = clicked { app.ui.selected = Some(row); app.ui.preview_slide = 0; }
        if let Some(row) = double { app.ui.selected = Some(row); app.go_live(0); }
    });
}
```
Verify: `cargo check -p sda-gui`; run: type `150`, first row highlights, Down moves and scrolls, Cmd/Ctrl+K refocuses, double click goes live. Commit: `feat(gui): search panel with virtualized results and actions`.

### Task 7: preview.rs
```rust
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let s = app.core.strings.clone();
    ui.heading(&s.preview);
    let Some(e) = app.selected_entry().cloned() else { return };
    let count = e.slides.len();
    if count == 0 { ui.label(s.slide_zero_fmt.replace("{}", &e.title)); return; }
    app.ui.preview_slide = app.ui.preview_slide.min(count - 1);
    let idx = app.ui.preview_slide;
    let w = ui.available_width(); let h = w * 9.0 / 16.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let footer = slide::footer_text(e.number.as_deref(), &e.title);
    slide::paint(ui.painter(), rect, &app.core.active_theme, &e.slides[idx], Some(&footer), &mut app.ui.fit);
    ui.horizontal(|ui| {
        if ui.add_enabled(idx > 0, egui::Button::new(&s.prev_slide)).clicked() { app.ui.preview_slide -= 1; }
        let titled = match &e.number { Some(n) => format!("{n}. {}", e.title), None => e.title.clone() };
        ui.label(s.slide_counter_fmt.replacen("{}", &titled, 1).replacen("{}", &(idx + 1).to_string(), 1).replacen("{}", &count.to_string(), 1));
        if ui.add_enabled(idx + 1 < count, egui::Button::new(&s.next_slide)).clicked() { app.ui.preview_slide += 1; }
        if ui.add(egui::Button::new(&s.go_live).fill(style::ACCENT)).clicked() { app.go_live(idx); }
    });
}
```
Commit: `feat(gui): preview card`.

### Task 8: live.rs
```rust
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let s = app.core.strings.clone();
    ui.horizontal(|ui| { ui.heading(&s.live); if app.projector.is_some() { ui.colored_label(style::ACCENT, "●"); } });
    if app.core.present.slide_count() == 0 { ui.label(&s.nothing_live); } else {
        let w = ui.available_width(); let h = w * 9.0 / 16.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
        let footer = slide::footer_text(app.core.present.number.as_deref(), &app.core.present.title);
        let text = app.core.present.current_slide().unwrap_or("").to_owned();
        if app.core.present.blank { ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK); }
        else { slide::paint(ui.painter(), rect, &app.core.active_theme, &text, Some(&footer), &mut app.ui.fit); }
        let p = &app.core.present;
        ui.label(format!("{}  {}/{}", p.title, p.index + 1, p.slide_count()));
        if let Some(n) = p.next_slide() { ui.small(format!("{}: {}", s.next_label, n.lines().next().unwrap_or(""))); }
    }
    ui.horizontal(|ui| {
        if ui.button("◀").clicked() { app.live_prev(); }
        if ui.button("▶").clicked() { app.live_next(); }
        let blank = app.core.present.blank;
        if ui.selectable_label(blank, format!("{} (B)", s.blank)).clicked() { app.live_blank(); }
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(&s.display);
        let mut sel = app.projector.as_ref().map(|p| p.display)
            .or(app.core.cfg.output_display.map(|i| i as usize)).unwrap_or_else(|| projector::default_display(&app.displays));
        egui::ComboBox::from_id_salt("display").selected_text(app.displays.get(sel).map(|d| d.label.as_str()).unwrap_or("?"))
            .show_ui(ui, |ui| { for d in &app.displays { ui.selectable_value(&mut sel, d.index, &d.label); } });
        match app.projector.as_ref().map(|p| p.display) {
            None => { if ui.add(egui::Button::new(&s.projector_on).fill(style::ACCENT)).clicked() { app.open_projector(sel); } }
            Some(cur) => { if ui.button(&s.projector_off).clicked() { app.stop_projector(); } else if sel != cur { app.open_projector(sel); } }
        }
    });
}
```
Commit: `feat(gui): live card and projector controls`.

### Task 9: settings.rs
```rust
pub fn show(ctx: &egui::Context, app: &mut App) {
    if !app.ui.settings_open { return; }
    let s = app.core.strings.clone(); let mut open = app.ui.settings_open;
    egui::Window::new(&s.settings).open(&mut open).resizable(true).default_size([720.0, 600.0]).show(ctx, |ui| {
      egui::ScrollArea::vertical().show(ui, |ui| {
        ui.label(format!("{}{}", s.version_prefix, env!("CARGO_PKG_VERSION"))); ui.label(&app.status.update);
        ui.separator(); ui.heading(&s.language_heading);
        let mut lang = app.core.language;
        ui.horizontal(|ui| { for l in Language::all() { ui.selectable_value(&mut lang, l, l.label()); } });
        if lang != app.core.language { app.core.set_language(lang); }
        ui.separator(); ui.heading(&s.library_heading); ui.label(&s.force_sync_description);
        let label = if app.status.syncing { &s.syncing_button } else { &s.force_sync_button };
        if ui.add_enabled(!app.status.syncing, egui::Button::new(label)).clicked() {
            app.status.syncing = true; app.status.sync = s.status_re_cloning.clone();
            app.core.spawn_force_sync(app.tx.clone(), ctx.clone());
        }
        ui.label(&app.status.sync);
        ui.separator(); ui.heading(&s.user_libraries_heading); ui.label(&s.user_libraries_description);
        let mut changed = false; let mut remove: Option<String> = None;
        for lib in app.core.cfg.libraries.iter_mut() {
            ui.horizontal(|ui| {
                if ui.checkbox(&mut lib.enabled, &lib.name).changed() { changed = true; }
                if !library::library_available(&lib.path) { ui.weak(&s.library_unavailable_suffix); }
                ui.weak(&lib.path);
                if !lib.managed_by_git && ui.small_button(&s.remove).clicked() { remove = Some(lib.path.clone()); }
            });
        }
        if let Some(p) = remove { library::remove_user_library(&mut app.core.cfg, &p); changed = true; }
        if ui.button(&s.add_folder_button).clicked() {
            if let Some(f) = rfd::FileDialog::new().pick_folder() {
                match library::add_user_library(&mut app.core.cfg, &f) { Ok(()) => changed = true, Err(e) => app.ui.library_status = e.to_string() }
            }
        }
        if changed { app.core.save_cfg(); app.ui.library_status = s.status_indexing.clone(); app.core.spawn_reindex(app.tx.clone(), ctx.clone()); }
        ui.label(&app.ui.library_status);
        ui.separator(); ui.heading(&s.themes_heading); theme_editor::show(ui, app);
      });
    });
    app.ui.settings_open = open;
}
```
Commit: `feat(gui): settings window`.

### Task 10: theme_editor.rs
```rust
pub fn show(ui: &mut egui::Ui, app: &mut App) {
    let s = app.core.strings.clone();
    egui::ScrollArea::horizontal().id_salt("thumbs").show(ui, |ui| ui.horizontal(|ui| {
        let active = app.core.active_theme.name.clone();
        for (i, t) in app.core.themes.clone().iter().enumerate() {
            let (rect, r) = ui.allocate_exact_size(egui::vec2(160.0, 90.0), egui::Sense::click());
            slide::paint(ui.painter(), rect, t, &s.theme_preview_sample, None, &mut app.ui.fit);
            if app.ui.draft_index == i { ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(2.0, style::ACCENT), egui::StrokeKind::Outside); }
            let name = if t.name == active { format!("{} {}", t.name, s.active_suffix) } else { t.name.clone() };
            ui.painter().text(rect.left_bottom() + egui::vec2(6.0, -4.0), egui::Align2::LEFT_BOTTOM, name, egui::FontId::proportional(12.0), egui::Color32::WHITE);
            if r.clicked() { app.ui.draft = t.clone(); app.ui.draft_index = i; }
        }
        if ui.button(&s.theme_new).clicked() { app.ui.draft = Theme { name: "Custom".into(), ..Theme::default() }; app.ui.draft_index = usize::MAX; }
    }));
    let d = &mut app.ui.draft;
    egui::Grid::new("theme_form").num_columns(4).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label(&s.theme_name_label); ui.text_edit_singleline(&mut d.name);
        ui.label(&s.theme_font_label);
        egui::ComboBox::from_id_salt("font").selected_text(d.font.label()).show_ui(ui, |ui| { for f in Font::all() { ui.selectable_value(&mut d.font, f, f.label()); } });
        ui.end_row();
        ui.label(&s.theme_size_label);
        ui.horizontal(|ui| {
            let mut auto = d.size == FontSize::Auto;
            if ui.checkbox(&mut auto, &s.theme_auto_size).changed() { d.size = if auto { FontSize::Auto } else { FontSize::Pt(44.0) }; }
            if let FontSize::Pt(p) = &mut d.size { ui.add(egui::Slider::new(p, 12.0..=160.0)); }
        });
        ui.label(&s.theme_bold_label); ui.checkbox(&mut d.bold, ""); ui.end_row();
        ui.label(&s.theme_text_color_label); let mut c = style::color32(d.text_color); if ui.color_edit_button_srgba(&mut c).changed() { d.text_color = style::rgba(c); }
        ui.label(&s.theme_bg_color_label); let mut b = style::color32(d.bg_color); if ui.color_edit_button_srgba(&mut b).changed() { d.bg_color = style::rgba(b); }
        ui.end_row();
        ui.label(&s.theme_align_label);
        ui.horizontal(|ui| { for (a, l) in [(HAlign::Left, "⬅"), (HAlign::Center, "⬌"), (HAlign::Right, "➡")] { ui.selectable_value(&mut d.h_align, a, l); } });
        ui.label(&s.theme_valign_label);
        ui.horizontal(|ui| { for (a, l) in [(VAlign::Top, "⬆"), (VAlign::Middle, "⬍"), (VAlign::Bottom, "⬇")] { ui.selectable_value(&mut d.v_align, a, l); } });
        ui.end_row();
        ui.label(&s.theme_line_spacing_label); ui.add(egui::Slider::new(&mut d.line_spacing, 1.0..=2.0));
        ui.label(&s.theme_margin_label); ui.add(egui::Slider::new(&mut d.margin, 0.02..=0.2)); ui.end_row();
        ui.label(&s.theme_footer_label); ui.checkbox(&mut d.footer, ""); ui.end_row();
    });
    let w = ui.available_width().min(640.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0), egui::Sense::hover());
    slide::paint(ui.painter(), rect, &app.ui.draft, &s.theme_preview_sample, Some("150 · Plecaţi-vă"), &mut app.ui.fit);
    ui.horizontal(|ui| {
        let builtin = app.ui.draft.is_builtin_default();
        if ui.add_enabled(!builtin, egui::Button::new(&s.theme_save)).clicked() {
            if app.ui.draft.name.trim().is_empty() { app.ui.draft.name = "Custom".into(); }
            if let Some(dir) = library::themes_dir() { if let Err(e) = store::save_theme(&dir, &app.ui.draft) { log::warn!("save theme: {e}"); } }
            app.core.reload_themes();
            app.ui.draft_index = app.core.themes.iter().position(|t| t.name == app.ui.draft.name).unwrap_or(0);
            if app.core.active_theme.name == app.ui.draft.name { let d = app.ui.draft.clone(); app.core.set_active_theme(d); }
        }
        if ui.add_enabled(!builtin, egui::Button::new(&s.theme_delete)).clicked() {
            let name = app.ui.draft.name.clone();
            if let Some(dir) = library::themes_dir() { let _ = store::delete_theme(&dir, &name); }
            app.core.reload_themes();
            if app.core.active_theme.name == name { app.core.set_active_theme(Theme::default()); }
            app.ui.draft = Theme::default(); app.ui.draft_index = 0;
        }
        if ui.add(egui::Button::new(&s.theme_set_active).fill(style::ACCENT)).clicked() { let d = app.ui.draft.clone(); app.core.set_active_theme(d); }
    });
}
```
Fallback: if `StrokeKind` is absent, `rect_stroke(rect, 8.0, stroke)`. Commit: `feat(gui): theme editor with thumbnails, form and live preview`.

### Task 11: videos.rs
```rust
pub fn show(ctx: &egui::Context, app: &mut App) {
    if !app.ui.videos_open { return; }
    let s = app.core.strings.clone(); let mut open = app.ui.videos_open;
    egui::Window::new(&s.videos).open(&mut open).resizable(true).default_size([520.0, 260.0]).show(ctx, |ui| {
        ui.label(&s.downloader_subtitle);
        ui.add(egui::TextEdit::singleline(&mut app.ui.video.url).desired_width(f32::INFINITY).hint_text(&s.downloader_url_placeholder));
        ui.horizontal(|ui| {
            ui.weak(app.ui.video.dir.display().to_string());
            if ui.button(&s.choose).clicked() {
                if let Some(f) = rfd::FileDialog::new().set_directory(&app.ui.video.dir).pick_folder() {
                    app.ui.video.dir = f.clone(); app.core.cfg.download_dir = Some(f.to_string_lossy().to_string()); app.core.save_cfg();
                }
            }
        });
        let busy = matches!(app.ui.video.phase, VideoPhase::Resolving | VideoPhase::Downloading);
        ui.horizontal(|ui| {
            if ui.add_enabled(!busy && !app.ui.video.url.trim().is_empty(), egui::Button::new(&s.download_button).fill(style::ACCENT)).clicked() {
                let v = &mut app.ui.video; v.phase = VideoPhase::Resolving; v.percent = 0.0; v.title.clear(); v.message.clear();
                let (url, dir) = (v.url.clone(), v.dir.clone());
                app.core.spawn_download(url, dir, app.tx.clone(), ctx.clone());
            }
            if ui.button(&s.reveal_in_folder).clicked() { let _ = open::that(&app.ui.video.dir); }
        });
        let v = &app.ui.video;
        if !v.title.is_empty() { ui.label(&v.title); }
        match v.phase {
            VideoPhase::Idle => {}
            VideoPhase::Resolving => { ui.spinner(); ui.label(&s.setting_up_downloader); }
            VideoPhase::Downloading => { ui.add(egui::ProgressBar::new(v.percent / 100.0).show_percentage().text(format!("{}  {}", v.speed, v.eta))); }
            VideoPhase::Done => { ui.colored_label(style::ACCENT, &s.download_complete); }
            VideoPhase::Failed => { ui.colored_label(egui::Color32::LIGHT_RED, format!("{}{}", s.download_failed_prefix, v.message)); }
        }
    });
    app.ui.videos_open = open;
}
```
Commit: `feat(gui): videos window`.

## Wave 4 (parallel, disjoint files)

### Task 12: release.yml
Replace the `Install Linux GUI dependencies` step packages with `libxkbcommon-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libgtk-3-dev`, comment `eframe plus rfd GTK dialogs`. `sed -i '' 's/-p hymnal-gui/-p sda-gui/; s/hymnal-gui/sda-manager/g' .github/workflows/release.yml`. Verify `grep -c hymnal` = 0, `grep -c sda-manager` >= 6. Commit: `ci: build and package sda-manager assets`.

### Task 13: install.sh and install.ps1
`sed -i '' 's/hymnal-gui/sda-manager/g; s/Hymn Finder/SDA Manager/g' install.sh install.ps1`. Verify `sh -n install.sh`; `grep -c hymnal` = 0 for both. Commit: `build: installers fetch sda-manager assets`.

### Task 14: README.md and CLAUDE.md
README: `hymnal-gui`→`sda-manager`, `hymnal-core`→`sda-core`, `RUST_LOG=sda_gui=debug,sda_core=debug`, Slint sentence → `written in Rust with eframe/egui`, themes bullet → `solid colors and four bundled fonts`, Using it steps → Search, Preview, Go Live, Projector on in the Live card, Settings for themes and folders. Remove Cross compilation section. CLAUDE.md: rewrite What this is and Architecture per spec: module list, `Msg` drain in `App::ui`, immediate projector viewport, `slide::paint` rule, keyboard table, commands `cargo build --release -p sda-gui`, binary `target/release/sda-manager`, `cargo test -p sda-core`. Verify `grep -c "hymnal-gui\|hymnal-core\|Slint" README.md CLAUDE.md` = 0. Commit: `docs: README and CLAUDE.md for the egui app`.

## Wave 5

### Task 15: Full verification
- `grep -rn "SEAM" crates/sda-gui/src` empty.
- `grep -rn "hymnal-gui\|hymnal_core\|hymnal-core\|slint" --include=*.rs --include=*.toml --include=*.yml --include=*.sh --include=*.ps1 --include=*.md . | grep -v target` empty.
- `cargo test --workspace` all ok. `cargo clippy --workspace` warning count recorded.
- `cargo build --release -p sda-gui && ls -la target/release/sda-manager`.
- Force sync against local server: zip assets/920 to /tmp/hz/hymns.zip, `python3 -m http.server 8765` in tmux, point `hymns_url` in config at it, run with RUST_LOG debug, expect `sync ok` and `total 9xx hymns indexed`. Restore config, kill tmux.
- Manual scenarios 1 to 10 (boot, search, Enter go live, projector keys, OS close, settings language/sync/folders, theme save/set active/delete, old theme compat, videos, relaunch geometry). Fixes go in `fix(gui): <what>` commits.

## Risks
1. Immediate viewport runs projector frames on main thread. Fine for one static slide.
2. `with_monitor(idx)` assumes display-info order equals winit order. Invalid index is ignored with a warning. Fallback: sort both by primary first then position.
3. `footer_compat` untagged enum. If `null` appears add a `Null` variant → false.
4. `TextFormat.line_height` may not exist in 0.36. Drop line spacing if so, note in commit.
5. `Frame::NONE`, `StrokeKind::Outside` names. Fallbacks `Frame::new()`, 3-arg `rect_stroke`.
6. FitCache bounded at 64 entries shared by projector, preview, live, editor.
7. First build needs network.
8. Wave 3 agents must not edit `app.rs`. Report missing fields.
9. Linux CI deps are a best guess. Run workflow on `v2.0.0-rc1` first.
10. Font URLs may move. Google Fonts CSS fallback provided.
