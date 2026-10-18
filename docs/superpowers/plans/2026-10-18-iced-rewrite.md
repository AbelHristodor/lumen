# SDA Manager 3.0 Iced Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Read your task, the Shared Interfaces section, and the spec.

**Goal:** Replace the egui GUI with an Iced 0.14 app plus a wgpu renderer, add Bible presenting, keep every feature, run on macOS and Windows.

**Architecture:** Three crates. `sda-core` holds logic and data with no UI. `sda-render` draws a `Scene` into any wgpu texture with no Iced dependency. `sda-app` is one `iced::daemon` with an operator window and a projector window. Workers send one `Msg` each.

**Tech Stack:** Rust 1.88+, Iced 0.14.0 (`wgpu`, `image`, `tokio`, `advanced`), `iced_test` 0.14.0, `wgpu` 27, `cosmic-text` 0.15, `display-info` 0.5, `rfd` 0.17, `hayro` 0.8, `image` 0.25, existing core deps.

**Spec:** `docs/superpowers/specs/2026-10-18-iced-rewrite-design.md`

## Global Constraints

- Workspace version `3.0.0`. Crates `sda-core`, `sda-render`, `sda-app`. Binary `sda-manager`. `update::BIN_NAME` stays `sda-manager`.
- `library.rs` keeps `ProjectDirs::from("org", "hymnal", "HymnFinder")`. Config, themes, hymns, cache paths unchanged.
- Iced API names below are verified against tag 0.14.0. Use them verbatim. `iced::daemon(boot, update, view)`. `view(&State, window::Id)`. `window::open(Settings) -> (Id, Task<Id>)`. `window::set_mode(id, Mode::Fullscreen)`. `window::frames()`. `event::listen_with(fn(Event, Status, Id) -> Option<Msg>)`. `window::close_events()`. `iced::widget::shader(program)` with `shader::Program` and `shader::Primitive`. `Theme::custom_with_fn`. `iced::font::load`. `iced::keyboard::on_key_press` does not exist. `iced::Panel` does not exist, Iced uses `row`, `column`, `container`.
- No GStreamer, no git2, no AGPL crates. Dependencies limited to the Tech Stack list plus what is already in `sda-core/Cargo.toml`.
- Prose in docs, comments, and UI strings: simple US English. No parentheses, dashes, semicolons.
- `cargo test --workspace` green and `cargo clippy --workspace --all-targets` zero warnings after every task. Commit after every task with a conventional commit prefix: `feat`, `refactor`, `test`, `build`, `ci`, `docs`, `fix`.
- A new `i18n::Strings` field must be added to En, It, and Ro in the same task or the crate does not compile.
- Every task commits only its own files with `git add <paths>`.
- Concurrent agents in the same wave touch disjoint files. If `cargo check` fails only in a sibling's file, wait 60 s and retry up to 5 times, then report.

## File Structure Map

Deleted in Task 0: `crates/sda-gui/**`.

`crates/sda-core/src/`:
- Modify: `model.rs` (Hymn, Slide, TextBox, Frame, Paragraph, Run, SearchHit), `pptx.rs` (styled extract), `index.rs` (Hymn cache, CACHE_VERSION 8), `search.rs` (Hymn, quotes, number first), `theme.rs` (new Theme), `present.rs` (Presentation), `library.rs` (bible_version, bibles_dir, history and overrides paths), `sync.rs` (sync_file), `refresh.rs` (Hymn), `downloader.rs` (probe, format id), `i18n.rs` (new strings), `lib.rs` (new mods).
- Create: `resolve.rs`, `store.rs`, `history.rs`, `overrides.rs`, `asis.rs`, `bible/mod.rs`, `bible/names.rs`, `bible/reference.rs`.
- Tests: `tests/pptx_test.rs`, `tests/index_test.rs`, `tests/search_test.rs`, `tests/theme_test.rs`, `tests/present_test.rs`, `tests/refresh_test.rs`, `tests/sync_test.rs`, `tests/downloader_test.rs` modified. Create `tests/resolve_test.rs`, `tests/store_test.rs`, `tests/history_test.rs`, `tests/asis_test.rs`, `tests/bible_test.rs`, `tests/fixtures/bible_mini.json`, `tests/fixtures/ytdlp_probe.json`.

`crates/sda-render/`: `Cargo.toml`, `src/lib.rs`, `src/scene.rs`, `src/text.rs`, `src/background.rs`, `src/composite.rs`, `src/video.rs`, `src/headless.rs`, `shaders/solid.wgsl`, `shaders/gradient.wgsl`, `shaders/particles.wgsl`, `shaders/image.wgsl`, `shaders/composite.wgsl`, `tests/golden_test.rs`, `tests/golden/*.png`.

`crates/sda-app/`: `Cargo.toml`, `build.rs`, `src/main.rs`, `src/app.rs`, `src/msg.rs`, `src/state/mod.rs`, `src/state/core.rs`, `src/state/live.rs`, `src/state/present.rs`, `src/state/bible.rs`, `src/state/videos.rs`, `src/state/themes.rs`, `src/state/settings.rs`, `src/view/mod.rs`, `src/view/shell.rs`, `src/view/live_strip.rs`, `src/view/present.rs`, `src/view/bible.rs`, `src/view/videos.rs`, `src/view/themes.rs`, `src/view/theme_form.rs`, `src/view/settings.rs`, `src/view/screen_picker.rs`, `src/view/projector.rs`, `src/widgets/mod.rs`, `src/widgets/scene_view.rs`, `src/widgets/card.rs`, `src/widgets/segmented.rs`, `src/ui_theme.rs`, `src/shortcuts.rs`, `src/displays.rs`, `src/workers.rs`, `tests/views_test.rs`.

Packaging: `Cargo.toml`, `.github/workflows/release.yml`, `README.md`, `CLAUDE.md`.

## Shared Interfaces (fixed names, use verbatim)

```rust
// sda-core model.rs
pub struct Hymn { pub number: Option<String>, pub title: String, pub slides: Vec<Slide>,
                  pub path: PathBuf, pub library: String, pub mtime: i64 }
pub struct Slide { pub background: Option<Rgba>, pub boxes: Vec<TextBox>, pub notes: Option<String> }
pub struct TextBox { pub frame: Option<Frame>, pub paragraphs: Vec<Paragraph> }
pub struct Frame { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }
pub struct Paragraph { pub align: Option<HAlign>, pub line_spacing: Option<f32>, pub runs: Vec<Run> }
pub struct Run { pub text: String, pub color: Option<Rgba>, pub size_pt: Option<f32>, pub bold: Option<bool>,
                 pub italic: Option<bool>, pub underline: Option<bool>, pub font: Option<String> }
impl Slide { pub fn plain_text(&self) -> String; }
impl Hymn { pub fn plain_slides(&self) -> Vec<String>; }
pub enum MatchField { Number, Title, Filename, Body }
pub struct SearchHit<'a> { pub index: usize, pub entry: &'a Hymn, pub score: u32, pub field: MatchField }

// sda-core theme.rs
pub struct Rgba { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }   // unchanged, plus to_hex, from_hex
pub enum HAlign { Left, Center, Right }
pub enum VAlign { Top, Middle, Bottom }
pub enum Prop<T> { Inherit, Set(T) }
impl<T: Clone> Prop<T> { pub fn resolve(&self, inherited: Option<T>, fallback: T) -> T; }
pub enum BundledFont { Inter, NotoSerif, Lora, RobotoSlab }
impl BundledFont { pub fn all() -> [BundledFont; 4]; pub fn label(self) -> &'static str; pub fn family(self) -> &'static str; }
pub enum Font { Bundled(BundledFont), System(String) }
impl Font { pub fn family(&self) -> &str; }
pub enum TextSize { Auto, Pt(f32) }
pub struct Shadow { pub color: Rgba, pub offset: (f32, f32), pub blur: f32 }
pub struct Outline { pub color: Rgba, pub width: f32 }
pub struct TextStyle { pub font: Prop<Font>, pub size: Prop<TextSize>, pub bold: Prop<bool>, pub color: Prop<Rgba>,
    pub h_align: Prop<HAlign>, pub v_align: VAlign, pub line_spacing: Prop<f32>, pub margin: Prop<f32>,
    pub opacity: f32, pub shadow: Option<Shadow>, pub outline: Option<Outline> }
pub enum Background { Solid(Rgba), Gradient { stops: Vec<(f32, Rgba)>, angle: f32, drift: f32 },
    Particles { base: Rgba, color: Rgba, count: u32, speed: f32 }, Image(PathBuf), Video { path: PathBuf } }
impl Background { pub fn kind_label(&self) -> &'static str; pub fn is_animated(&self) -> bool; }
pub enum Direction { Left, Right, Up, Down }
pub enum TransitionKind { None, Fade, Slide(Direction), Scale, Blur }
pub enum Easing { Linear, EaseInOut, EaseOut }
impl Easing { pub fn apply(self, t: f32) -> f32; }
pub struct Transition { pub kind: TransitionKind, pub duration_ms: u32, pub easing: Easing }
pub enum RefPos { Top, Bottom }
pub struct VerseLayout { pub per_slide: u8, pub show_reference: bool, pub show_numbers: bool, pub reference_at: RefPos }
pub struct Theme { pub name: String, pub text: TextStyle, pub background: Background, pub background_opacity: f32,
    pub transition: Transition, pub footer: bool, pub verses: VerseLayout, pub favorite: bool }
impl Theme { pub fn to_json(&self) -> Result<String>; pub fn from_json(s: &str) -> Result<Theme>;
             pub fn is_builtin_default(&self) -> bool; pub fn cache_key(&self) -> u64; }
pub mod store { pub fn list_themes(dir: &Path) -> Vec<Theme>; pub fn load_theme(dir: &Path, name: &str) -> Result<Theme>;
                pub fn save_theme(dir: &Path, t: &Theme) -> Result<()>; pub fn delete_theme(dir: &Path, name: &str) -> Result<()>; }

// sda-core resolve.rs
pub enum SlideContent { Styled(Slide), Verses { reference: String, verses: Vec<(u32, String)> }, Bitmap(PathBuf) }
pub struct ResolvedRun { pub text: String, pub font: Font, pub size: TextSize, pub bold: bool, pub italic: bool,
                         pub color: Rgba, pub opacity: f32, pub shadow: Option<Shadow>, pub outline: Option<Outline> }
pub struct ResolvedLine { pub align: HAlign, pub spacing: f32, pub runs: Vec<ResolvedRun> }
pub struct ResolvedBox { pub frame: Frame, pub v_align: VAlign, pub lines: Vec<ResolvedLine> }
pub struct ResolvedSlide { pub background: Rgba, pub boxes: Vec<ResolvedBox>, pub footer: Option<String> }
pub fn resolve(theme: &Theme, content: &SlideContent, footer: Option<&str>) -> Option<ResolvedSlide>  // None for Bitmap
pub const FALLBACK_COLOR: Rgba; pub const FALLBACK_SPACING: f32; pub const FALLBACK_MARGIN: f32;

// sda-core present.rs
pub enum Source { Hymn(PathBuf), Bible { version: String, reference: String }, None }
pub struct Presentation { pub title: String, pub subtitle: Option<String>, pub slides: Vec<SlideContent>,
                          pub index: usize, pub blank: bool, pub source: Source }
impl Presentation { pub fn load(&mut self, title: String, subtitle: Option<String>, slides: Vec<SlideContent>, source: Source);
    pub fn slide_count(&self) -> usize; pub fn current(&self) -> Option<&SlideContent>; pub fn next(&mut self);
    pub fn prev(&mut self); pub fn jump(&mut self, n: usize); pub fn toggle_blank(&mut self); pub fn is_loaded(&self) -> bool; }

// sda-core bible
pub struct Book { pub abbrev: String, pub name: String, pub chapters: Vec<Vec<String>> }
pub struct Bible { pub code: String, pub books: Vec<Book> }
pub struct VersionInfo { pub code: String, pub language: String, pub name: String, pub abbreviation: String, pub public_domain: Option<bool> }
pub const PIN: &str = "0e8fd65a89727f5b73e9f88edb220da8e0d5daba";
pub fn index_url() -> String; pub fn version_url(code: &str) -> String;
pub fn parse_index(json: &str) -> Result<Vec<VersionInfo>>;
pub fn load(path: &Path) -> Result<Bible>;
pub fn public_domain(code: &str) -> Option<bool>;
pub struct Reference { pub book: usize, pub chapter: u32, pub verses: Option<(u32, u32)> }
pub fn parse_reference(input: &str, lang: Language) -> Option<Reference>;
pub fn book_name(book: usize, lang: Language) -> &'static str;
pub fn format_reference(r: &Reference, lang: Language) -> String;   // "John 3:16-18"
pub fn verses(bible: &Bible, r: &Reference) -> Vec<(u32, String)>;
pub fn search(bible: &Bible, query: &str, limit: usize) -> Vec<(Reference, String)>;
pub fn verse_slides(verses: &[(u32, String)], reference: &str, per_slide: u8) -> Vec<SlideContent>;

// sda-core store.rs
pub fn json_load<T: DeserializeOwned + Default>(path: &Path) -> T;
pub fn json_save<T: Serialize>(path: &Path, value: &T) -> Result<()>;   // write tmp then rename

// sda-core history.rs
pub struct HistoryEntry { pub path: PathBuf, pub number: Option<String>, pub title: String, pub opened_at: i64 }
pub fn record(list: &mut Vec<HistoryEntry>, entry: HistoryEntry);   // dedupe by path, newest first, cap 50
pub fn recent(list: &[HistoryEntry], n: usize) -> &[HistoryEntry];

// sda-core overrides.rs
pub struct Overrides { pub as_is: BTreeMap<PathBuf, bool> }

// sda-core asis.rs
pub enum Converter { LibreOffice(PathBuf), PowerPoint(PathBuf) }
pub fn detect() -> Option<Converter>;
pub fn detect_in(path_var: &str, candidates: &[PathBuf]) -> Option<Converter>;
pub fn cache_dir_for(root: &Path, pptx: &Path, mtime: i64, size: (u32, u32)) -> PathBuf;
pub fn cached(dir: &Path) -> Option<Vec<PathBuf>>;   // Some when `.done` exists
pub fn render(conv: &Converter, pptx: &Path, out_dir: &Path, size: (u32, u32)) -> Result<Vec<PathBuf>>;

// sda-core downloader.rs additions
pub struct Quality { pub label: String, pub height: u32, pub format_id: String, pub size_bytes: Option<u64> }
pub struct Probe { pub title: String, pub thumbnail: Option<String>, pub duration_secs: Option<u64>, pub qualities: Vec<Quality> }
pub fn parse_probe(json: &str) -> Result<Probe>;
pub fn probe(url: &str) -> Result<Probe>;
pub fn start_download(url: &str, format_id: Option<&str>, dir: &Path, on_event: impl FnMut(DownloadEvent)) -> Result<()>;
pub struct DownloadRecord { pub title: String, pub path: PathBuf, pub at: i64 }

// sda-core sync.rs
pub enum SyncOutcome { Downloaded, Unchanged }
pub fn sync_file(url: &str, dest: &Path, stamp: &Path) -> Result<SyncOutcome>;   // GET to dest.part, rename
pub fn sync_hymns(url: &str, dest_dir: &Path, stamp: &Path) -> Result<SyncOutcome>;   // zip, unchanged behavior

// sda-core library.rs additions
pub fn bibles_dir() -> Option<PathBuf>; pub fn history_path() -> Option<PathBuf>; pub fn downloads_path() -> Option<PathBuf>;
pub fn overrides_path() -> Option<PathBuf>; pub fn window_path() -> Option<PathBuf>; pub fn asis_cache_dir() -> Option<PathBuf>;
// Config gains: #[serde(default)] pub bible_version: Option<String>

// sda-render
pub struct Scene { pub size: (u32, u32), pub background: BackgroundState, pub background_opacity: f32,
                   pub current: Option<Layer>, pub outgoing: Option<(Layer, f32)>, pub transition: Transition,
                   pub blank: bool, pub now_secs: f32 }
pub enum Layer { Text(Arc<ResolvedSlide>), Bitmap(Arc<RgbaImage>) }
pub enum BackgroundState { Solid(Rgba), Gradient { stops: Vec<(f32, Rgba)>, angle: f32, drift: f32 },
    Particles { base: Rgba, color: Rgba, count: u32, speed: f32 }, Image(Arc<RgbaImage>), Video(Arc<FrameSource>) }
pub struct Renderer;  impl Renderer { pub fn new(&Device, &Queue, TextureFormat) -> Self;
    pub fn prepare(&mut self, &Device, &Queue, &Scene); pub fn render(&self, &mut CommandEncoder, &TextureView, Viewport); }
pub struct Viewport { pub x: u32, pub y: u32, pub w: u32, pub h: u32 }
pub struct FrameSource;  impl FrameSource { pub fn latest(&self) -> Option<Arc<RgbaImage>>; pub fn size(&self) -> (u32, u32); }
pub fn spawn_decoder(ffmpeg: &Path, media: &Path, size: (u32, u32)) -> Result<Arc<FrameSource>>;
pub mod headless { pub fn device() -> Option<(Device, Queue)>; pub fn render_to_image(scene: &Scene) -> Option<RgbaImage>; }
pub fn fonts() -> &'static [(&'static str, &'static [u8])];   // the 8 bundled ttf files

// sda-app msg.rs
pub enum Msg { Tab(Tab), Search(String), Select(usize), MoveSelection(i32), PreviewSlide(usize), GoLive, GoLiveAt(usize),
    LiveNext, LivePrev, LiveBlank, LiveJump(usize), LiveStop, ProjectorToggle, ProjectorDisplay(usize), ProjectorClosed(window::Id),
    Tick(Instant), Indexed(Result<Searcher, String>), Synced(Result<Searcher, String>), Reindexed(Result<Searcher, String>),
    UpdateChecked(String), ForceSync, Bible(BibleMsg), Videos(VideosMsg), Themes(ThemesMsg), Settings(SettingsMsg),
    AsIsToggle(bool), AsIsRendered(Result<(PathBuf, Vec<PathBuf>), String>), DisplaysListed(Vec<Display>),
    FontsListed(Vec<String>), WindowMoved(Point), WindowResized(Size), SaveWindow, Identify, IdentifyDone, Noop }
pub enum Tab { Present, Bible, Videos, Themes, Settings }
```

---

## Wave 0: workspace prep (one agent)

### Task 0: Workspace version, remove the egui crate, core deps

**Files:**
- Modify: `Cargo.toml` (version `3.0.0`, members `["crates/sda-core"]`), `crates/sda-core/Cargo.toml` (add `hayro = "0.8"`, `image = { version = "0.25", default-features = false, features = ["png", "jpeg", "gif"] }`, `sha2 = "0.10"`)
- Delete: `crates/sda-gui/**`

- [ ] `git rm -r crates/sda-gui`. Set `[workspace.package] version = "3.0.0"` and `members = ["crates/sda-core"]`.
- [ ] Add the deps above to `sda-core`. Run `cargo test --workspace`. Expect green.
- [ ] Commit: `build: remove the egui app, bump to 3.0.0, add hayro, image, sha2 to core`

The egui app is gone from here on. The Iced app arrives in Wave 3. Every task in between keeps `cargo test --workspace` green.

## Wave 1: core (parallel, disjoint files)

Tasks 1 to 6 run in parallel. Task 1 changes `model.rs`, which Tasks 2 and 4 read but do not edit. Task 1 lands first, the others start after `model.rs` is committed.

### Task 1: Styled hymn model and parser

**Files:**
- Modify: `crates/sda-core/src/model.rs`, `crates/sda-core/src/pptx.rs`, `crates/sda-core/src/index.rs`, `crates/sda-core/src/search.rs`, `crates/sda-core/src/refresh.rs`
- Test: `crates/sda-core/tests/pptx_test.rs`, `tests/index_test.rs`, `tests/search_test.rs`, `tests/refresh_test.rs`

**Interfaces:** Produces `Hymn`, `Slide`, `TextBox`, `Frame`, `Paragraph`, `Run` per Shared Interfaces. `pptx::extract(path) -> Result<ParsedPptx>` where `ParsedPptx { number, title, slides: Vec<Slide> }`. `index::refresh_index(root, lib, cached: &[Hymn]) -> Vec<Hymn>`. `CACHE_VERSION = 8`. `Searcher::new(Vec<Hymn>)`, `Searcher::entry(i) -> Option<&Hymn>`, `Searcher::search(&mut self, q) -> Vec<SearchHit>` unchanged in shape.

- [ ] **Step 1: Write the failing tests** in `tests/pptx_test.rs`. Keep existing tests but change `parsed.slides[i]` string asserts to `parsed.slides[i].plain_text()`. Add:

```rust
#[test]
fn slide_runs_keep_color_size_and_bold() {
    let p = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    let slide = &p.slides[1];
    let runs: Vec<&Run> = slide.boxes[0].paragraphs[0].runs.iter().collect();
    assert_eq!(runs[0].text, "1");
    assert_eq!(runs[0].color, Some(Rgba::new(0xFF, 0xDD, 0x4B, 255)));
    assert_eq!(runs[0].size_pt, Some(40.0));
    assert_eq!(runs[0].bold, Some(true));
    assert_eq!(runs[0].font.as_deref(), Some("Georgia"));
    let white = runs.iter().find(|r| r.text.contains("Cerul")).unwrap();
    assert_eq!(white.color, Some(Rgba::new(255, 255, 255, 255)));   // schemeClr bg1 -> lt1 FFFFFF
    assert_eq!(white.bold, None);
}

#[test]
fn slide_background_comes_from_master() {
    let p = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    assert_eq!(p.slides[1].background, Some(Rgba::new(0, 0, 0, 255)));   // tx1 -> dk1 000000
}

#[test]
fn text_box_frame_is_fraction_of_slide() {
    let p = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    let f = p.slides[1].boxes[0].frame.unwrap();
    assert!((f.x - 838200.0 / 9144000.0).abs() < 1e-4);
    assert!((f.y - 417255.0 / 6858000.0).abs() < 1e-4);
    assert!((f.w - 8305800.0 / 9144000.0).abs() < 1e-4);
}

#[test]
fn chrome_boxes_are_dropped() {
    let p = extract(Path::new("tests/fixtures/150.pptx")).unwrap();
    let all = p.slides[1].plain_text();
    assert!(!all.contains("IMNURI"));
    assert!(!all.contains("150/920"));
    assert_eq!(p.slides[1].boxes.len(), 1);
}

#[test]
fn lum_mod_darkens_scheme_color() {
    assert_eq!(pptx::apply_lum(Rgba::new(255, 255, 255, 255), Some(50000), None), Rgba::new(128, 128, 128, 255));
}
```

- [ ] **Step 2: Run** `cargo test -p sda-core --test pptx_test`. Expect compile failure: no `Run`, `Slide` fields.

- [ ] **Step 3: Model.** Replace `HymnEntry` in `model.rs` with the Shared Interfaces types. All derive `Debug, Clone, PartialEq, Serialize, Deserialize`. `Frame` and `Run` also `Default`. Implement:

```rust
impl Slide {
    pub fn plain_text(&self) -> String {
        self.boxes.iter().flat_map(|b| b.paragraphs.iter())
            .map(|p| p.runs.iter().map(|r| r.text.as_str()).collect::<String>().trim().to_string())
            .filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n")
    }
}
impl Hymn { pub fn plain_slides(&self) -> Vec<String> { self.slides.iter().map(Slide::plain_text).collect() } }
```

Keep `MatchField` and `SearchHit` with `entry: &'a Hymn`.

- [ ] **Step 4: Parser.** In `pptx.rs` add `pub fn apply_lum(c: Rgba, lum_mod: Option<u32>, lum_off: Option<u32>) -> Rgba` using HSL: convert to HSL, `l = l * lum_mod/100000 + lum_off/100000`, clamp, back to RGB. Add a `ColorScheme` struct built once per file: parse `ppt/theme/theme1.xml` `a:clrScheme` into a map name to Rgba, parse the first slide master `p:clrMap` attributes into a map `bg1 -> lt1` and so on. `fn scheme_color(name: &str) -> Option<Rgba>` looks up the clrMap then the scheme. Parse `p:sldSz` from `ppt/presentation.xml` into `(cx, cy)`.

  Rewrite `slide_text_lines` into `fn parse_slide(xml: &str, scheme: &ColorScheme, slide_size: (f32, f32)) -> Result<Vec<TextBox>>` with a quick-xml state machine: on `p:sp` start a `TextBox`; `a:xfrm` with `a:off` and `a:ext` set `frame` as fractions; `a:p` starts a `Paragraph`; `a:pPr algn` sets `align` (`l`, `ctr`, `r`); `a:lnSpc/a:spcPct val` sets `line_spacing = val / 100000`; `a:r` starts a `Run`; `a:rPr` attributes `sz` (`sz / 100` as pt), `b`, `i`, `u`; inside `a:rPr`, `a:solidFill/a:srgbClr val` sets color, `a:schemeClr val` with nested `a:lumMod`, `a:lumOff` sets color through `scheme_color` then `apply_lum`; `a:latin typeface` sets `font`; `a:t` text appends to the run. On `p:sp` end push the box if it has any non empty run. Drop a whole box when every paragraph's joined text matches `is_chrome`. Empty paragraphs are dropped.

  Slide background: parse `p:bg//a:solidFill` in `slideN.xml`, else in the layout referenced by `slideN.xml.rels`, else in `slideMaster1.xml`. Resolve scheme colors the same way.

  Notes: if `ppt/notesSlides/notesSlideN.xml` exists where N matches the slide's rels target, collect its `a:t` text into `notes`.

  `extract` returns `ParsedPptx { number, title, slides: Vec<Slide> }` with the title picked from the plain text of the first slide as today.

- [ ] **Step 5: Index.** In `index.rs` replace `HymnEntry` with `Hymn`, build `Hymn { slides: parsed.slides, .. }`, bump `CACHE_VERSION` to 8.

- [ ] **Step 6: Search.** In `search.rs` the indexed body is `fold(&entry.plain_slides().join("\n"))`. Add two ranking rules with tests in `tests/search_test.rs`: `quoted_query_requires_exact_phrase` asserts that `"\"se vor topi\""` matches hymn 150 and that `"\"topi se vor\""` matches nothing, implemented by stripping the quotes and requiring `body_folded.contains(&folded_query)` for every item; `exact_number_ranks_first` asserts that for query `1` the first hit has number `1` even though hymn 150 and 664b contain the digit, implemented by adding a bonus of `10_000` when `number_str == folded_query`. In `refresh.rs` rename types.

- [ ] **Step 7: Run** `cargo test -p sda-core`. Fix the other test files: `HymnEntry` literals become `Hymn` with `slides: vec![Slide { background: None, boxes: vec![TextBox { frame: None, paragraphs: vec![Paragraph { align: None, line_spacing: None, runs: vec![Run { text: "...".into(), ..Default::default() }] }] }], notes: None }]`. Add a test helper `fn hymn(number, title, lines: &[&str]) -> Hymn` at the top of `search_test.rs` and `index_test.rs`. Expect all pass.

- [ ] **Step 8: Commit:** `feat(core): styled pptx model with colors, sizes, frames, backgrounds, notes`

### Task 2: Theme model with Prop, resolve, Presentation

**Files:**
- Modify: `crates/sda-core/src/theme.rs`, `crates/sda-core/src/present.rs`, `crates/sda-core/src/lib.rs` (`pub mod resolve;`)
- Create: `crates/sda-core/src/resolve.rs`
- Test: `tests/theme_test.rs`, `tests/present_test.rs`, create `tests/resolve_test.rs`

**Interfaces:** Produces every theme, resolve, and present type from Shared Interfaces.

- [ ] **Step 1: Tests.** Replace `tests/theme_test.rs` content with:

```rust
use sda_core::theme::*;

#[test]
fn default_theme_inherits_text_and_has_black_fade() {
    let t = Theme::default();
    assert_eq!(t.name, "Default");
    assert!(matches!(t.text.font, Prop::Inherit));
    assert!(matches!(t.text.size, Prop::Inherit));
    assert!(matches!(t.text.color, Prop::Inherit));
    assert_eq!(t.background, Background::Solid(Rgba::new(0, 0, 0, 255)));
    assert_eq!(t.transition.kind, TransitionKind::Fade);
    assert_eq!(t.transition.duration_ms, 400);
    assert!(t.footer);
    assert_eq!(t.verses.per_slide, 1);
}

#[test]
fn v2_flat_theme_json_loads_as_set_values() {
    let old = r##"{"name":"Old","font":"Lora","size":{"Pt":48.0},"bold":true,"text_color":{"r":1,"g":2,"b":3,"a":255},
        "bg_color":{"r":9,"g":9,"b":9,"a":255},"h_align":"Left","v_align":"Top","line_spacing":1.3,"margin":0.1,"footer":false}"##;
    let t = Theme::from_json(old).unwrap();
    assert_eq!(t.text.font, Prop::Set(Font::Bundled(BundledFont::Lora)));
    assert_eq!(t.text.size, Prop::Set(TextSize::Pt(48.0)));
    assert_eq!(t.text.color, Prop::Set(Rgba::new(1, 2, 3, 255)));
    assert_eq!(t.background, Background::Solid(Rgba::new(9, 9, 9, 255)));
    assert!(!t.footer);
}

#[test]
fn theme_json_round_trips_every_background() {
    for bg in [Background::Solid(Rgba::new(1, 2, 3, 255)),
               Background::Gradient { stops: vec![(0.0, Rgba::new(0,0,0,255)), (1.0, Rgba::new(9,9,9,255))], angle: 45.0, drift: 0.2 },
               Background::Particles { base: Rgba::new(0,0,0,255), color: Rgba::new(255,255,255,255), count: 80, speed: 0.5 },
               Background::Image("/x.png".into()), Background::Video { path: "/x.mp4".into() }] {
        let t = Theme { background: bg, ..Theme::default() };
        assert_eq!(Theme::from_json(&t.to_json().unwrap()).unwrap(), t);
    }
}

#[test]
fn prop_resolve_prefers_set_then_inherited_then_fallback() {
    assert_eq!(Prop::Set(3).resolve(Some(2), 1), 3);
    assert_eq!(Prop::Inherit.resolve(Some(2), 1), 2);
    assert_eq!(Prop::<i32>::Inherit.resolve(None, 1), 1);
}

#[test]
fn easing_endpoints() {
    for e in [Easing::Linear, Easing::EaseInOut, Easing::EaseOut] {
        assert!((e.apply(0.0)).abs() < 1e-6);
        assert!((e.apply(1.0) - 1.0).abs() < 1e-6);
    }
}

#[test]
fn cache_key_changes_with_background_and_text() {
    let a = Theme::default();
    let mut b = a.clone(); b.text.opacity = 0.5;
    let mut c = a.clone(); c.background = Background::Solid(Rgba::new(1,1,1,255));
    assert_ne!(a.cache_key(), b.cache_key());
    assert_ne!(a.cache_key(), c.cache_key());
}
```

  Keep the existing `store` tests, updated to the new struct. Create `tests/resolve_test.rs`:

```rust
use sda_core::model::*; use sda_core::theme::*; use sda_core::resolve::*;

fn slide(color: Option<Rgba>, size: Option<f32>, align: Option<HAlign>) -> SlideContent {
    SlideContent::Styled(Slide { background: Some(Rgba::new(10, 10, 10, 255)), notes: None, boxes: vec![TextBox {
        frame: Some(Frame { x: 0.1, y: 0.1, w: 0.8, h: 0.3 }),
        paragraphs: vec![Paragraph { align, line_spacing: None, runs: vec![Run { text: "Hi".into(), color, size_pt: size, ..Default::default() }] }] }] })
}

#[test]
fn inherit_takes_pptx_values() {
    let r = resolve(&Theme::default(), &slide(Some(Rgba::new(1,2,3,255)), Some(40.0), Some(HAlign::Left)), None).unwrap();
    let run = &r.boxes[0].lines[0].runs[0];
    assert_eq!(run.color, Rgba::new(1,2,3,255));
    assert_eq!(run.size, TextSize::Pt(40.0));
    assert_eq!(r.boxes[0].lines[0].align, HAlign::Left);
    assert_eq!(r.boxes[0].frame, Frame { x: 0.1, y: 0.1, w: 0.8, h: 0.3 });
}

#[test]
fn inherit_falls_back_when_pptx_is_silent() {
    let r = resolve(&Theme::default(), &slide(None, None, None), None).unwrap();
    let run = &r.boxes[0].lines[0].runs[0];
    assert_eq!(run.color, FALLBACK_COLOR);
    assert_eq!(run.size, TextSize::Auto);
    assert_eq!(r.boxes[0].lines[0].align, HAlign::Center);
}

#[test]
fn set_overrides_pptx() {
    let mut t = Theme::default();
    t.text.color = Prop::Set(Rgba::new(9,9,9,255));
    t.text.h_align = Prop::Set(HAlign::Right);
    t.text.margin = Prop::Set(0.2);
    let r = resolve(&t, &slide(Some(Rgba::new(1,2,3,255)), None, Some(HAlign::Left)), None).unwrap();
    assert_eq!(r.boxes[0].lines[0].runs[0].color, Rgba::new(9,9,9,255));
    assert_eq!(r.boxes[0].lines[0].align, HAlign::Right);
    assert_eq!(r.boxes[0].frame, Frame { x: 0.2, y: 0.2, w: 0.6, h: 0.6 });   // Set margin replaces pptx frame
}

#[test]
fn verses_make_one_box_plus_reference_line() {
    let c = SlideContent::Verses { reference: "John 3:16".into(), verses: vec![(16, "For God".into())] };
    let r = resolve(&Theme::default(), &c, None).unwrap();
    assert_eq!(r.boxes.len(), 1);
    let last = r.boxes[0].lines.last().unwrap();
    assert_eq!(last.runs[0].text, "John 3:16");
    assert!(r.boxes[0].lines[0].runs[0].text.starts_with("16 "));
}

#[test]
fn bitmap_resolves_to_none_and_footer_passes_through() {
    assert!(resolve(&Theme::default(), &SlideContent::Bitmap("/a.png".into()), None).is_none());
    let r = resolve(&Theme::default(), &slide(None, None, None), Some("150 · Title")).unwrap();
    assert_eq!(r.footer.as_deref(), Some("150 · Title"));
}
```

  In `tests/present_test.rs` rewrite for `Presentation::load(title, subtitle, slides, source)`, `current()`, `jump(n)` clamped and 1 based, `is_loaded()`.

- [ ] **Step 2: Run** `cargo test -p sda-core --test theme_test --test resolve_test --test present_test`. Expect compile failures.

- [ ] **Step 3: Theme.** Implement every type in Shared Interfaces in `theme.rs`. Derives: `Debug, Clone, PartialEq, Serialize, Deserialize`; `Copy` on enums without data and on `Rgba`, `Shadow`, `Outline`, `Transition`, `VerseLayout`; `Hash` where fields allow it, hashing `f32` by `to_bits`. `Prop<T>` serializes as `"Inherit"` or `{"Set": ...}` through default derive. `Theme` has `#[serde(default, from = "ThemeV2OrV3")]` where a private untagged enum tries the new shape first then the old flat shape and maps flat fields into `Set`. `Theme::default()` per spec section 3.2. `cache_key` hashes the whole theme with `DefaultHasher`. `Easing::apply`: Linear `t`, EaseInOut `0.5 - 0.5 * cos(PI t)`, EaseOut `1 - (1-t)^3`. `impl Default for Transition` returns Fade, 400, EaseInOut. `Background::is_animated` true for Gradient with `drift > 0`, Particles, Video. Keep `store` functions, `file_stem`, and the Default name reservation.

- [ ] **Step 4: Resolve.** `resolve.rs` per spec section 3.3. Constants `FALLBACK_COLOR = Rgba::new(255,255,255,255)`, `FALLBACK_SPACING = 1.1`, `FALLBACK_MARGIN = 0.06`. For Styled: background is `Set` only through `Background::Solid`, otherwise `slide.background` or black, this field only matters for the thumbnail fallback. Each `TextBox` becomes a `ResolvedBox`: if `theme.text.margin` is `Set(m)` or the box has no frame, frame is `Frame { x: m, y: m, w: 1-2m, h: 1-2m }` with `m` from `margin.resolve(None, FALLBACK_MARGIN)`, else the pptx frame. Each paragraph becomes one `ResolvedLine` with `align = theme.text.h_align.resolve(p.align, HAlign::Center)`, `spacing = line_spacing.resolve(p.line_spacing, FALLBACK_SPACING)`. Each run resolves font through `font.resolve(run.font.clone().map(Font::System), Font::Bundled(BundledFont::Inter))`, size through `size.resolve(run.size_pt.map(TextSize::Pt), TextSize::Auto)`, bold, color, and copies opacity, shadow, outline from the theme. For Verses: one box with the margin frame, one line per verse with text `"{n} {text}"` when `show_numbers` else `text`, aligned per theme, then if `show_reference` a final line with the reference at 60 percent size when size is `Pt`, else Auto, placed last for `RefPos::Bottom` and first for `Top`. Bitmap returns `None`.

- [ ] **Step 5: Present.** Rewrite `present.rs` to `Presentation` with the Shared Interfaces methods. `jump(n)` sets `index = n - 1` when `1 <= n <= slide_count()`.

- [ ] **Step 6: Run** `cargo test -p sda-core`. Expect pass. Clippy clean.
- [ ] **Step 7: Commit:** `feat(core): theme with inherit props, pure resolve, presentation state`

### Task 3: Bible module

**Files:**
- Create: `crates/sda-core/src/bible/mod.rs`, `bible/names.rs`, `bible/reference.rs`, `tests/bible_test.rs`, `tests/fixtures/bible_mini.json`
- Modify: `crates/sda-core/src/lib.rs` (`pub mod bible;`)

**Interfaces:** Produces everything under `// sda-core bible` in Shared Interfaces. Depends on `resolve::SlideContent` from Task 2. If Task 2 is not merged yet, define `verse_slides` last and run `cargo check` after Task 2 lands.

- [ ] **Step 1: Fixture.** Write `tests/fixtures/bible_mini.json`: a 66 element array where each book is `{"abbrev": "<abbrev>", "name": "<name>", "chapters": [[]]}` except Genesis (`gn`) with chapter 1 verses `["In the beginning God created the heaven and the earth.", "And the earth was without form"]` and John (`jo`, index 42) with chapter 3 holding 18 verses where verse 16 is `"For God so loved the world"`, verse 17 `"For God sent not his Son"`, verse 18 `"He that believeth"`, the other verses `"v"`. Use the real abbrev list from the repo order: `gn ex lv nm dt js jz rt 1sm 2sm 1kgs 2kgs 1ch 2ch ezr ne et job ps prv ec so is jr lm ez dn ho jl am ob jn mi na hk zp hg zc ml mt mk lk jo act rm 1co 2co gl eph ph cl 1ts 2ts 1tm 2tm tt phm hb jm 1pe 2pe 1jo 2jo 3jo jd re`. Note the real repo uses `jó` for Job and `atos` for Acts. Match by position, never by abbrev.

- [ ] **Step 2: Tests** `tests/bible_test.rs`:

```rust
use sda_core::bible::*; use sda_core::i18n::Language; use std::path::Path;

fn mini() -> Bible { load(Path::new("tests/fixtures/bible_mini.json")).unwrap() }

#[test] fn loads_66_books() { assert_eq!(mini().books.len(), 66); assert_eq!(mini().code, "bible_mini"); }

#[test] fn parses_references_in_three_languages() {
    let r = parse_reference("John 3:16-18", Language::En).unwrap();
    assert_eq!((r.book, r.chapter, r.verses), (42, 3, Some((16, 18))));
    assert_eq!(parse_reference("Ioan 3,16", Language::Ro).unwrap().verses, Some((16, 16)));
    assert_eq!(parse_reference("Giovanni 3", Language::It).unwrap().verses, None);
    assert_eq!(parse_reference("1 Sam 2:1", Language::En).unwrap().book, 8);
    assert_eq!(parse_reference("gen 1:1", Language::En).unwrap().book, 0);
    assert!(parse_reference("Nowhere 1:1", Language::En).is_none());
}

#[test] fn formats_reference_localized() {
    let r = Reference { book: 42, chapter: 3, verses: Some((16, 18)) };
    assert_eq!(format_reference(&r, Language::En), "John 3:16-18");
    assert_eq!(format_reference(&r, Language::Ro), "Ioan 3:16-18");
    assert_eq!(format_reference(&Reference { book: 0, chapter: 1, verses: None }, Language::It), "Genesi 1");
}

#[test] fn verses_returns_range_clamped() {
    let v = verses(&mini(), &Reference { book: 42, chapter: 3, verses: Some((17, 99)) });
    assert_eq!(v.len(), 2); assert_eq!(v[0].0, 17);
}

#[test] fn search_folds_accents_and_limits() {
    let hits = search(&mini(), "god so LOVED", 10);
    assert_eq!(hits.len(), 1); assert_eq!(hits[0].0.verses, Some((16, 16)));
    assert_eq!(search(&mini(), "v", 3).len(), 3);
}

#[test] fn verse_slides_groups_per_slide() {
    let v = vec![(16, "a".to_string()), (17, "b".to_string()), (18, "c".to_string())];
    let s = verse_slides(&v, "John 3:16-18", 2);
    assert_eq!(s.len(), 2);
    match &s[1] { sda_core::resolve::SlideContent::Verses { verses, reference } => { assert_eq!(verses.len(), 1); assert_eq!(reference, "John 3:16-18"); }, _ => panic!() }
}

#[test] fn index_parses_and_urls_are_pinned() {
    let json = r#"[{"language":"English","versions":[{"name":"King James Version","abbreviation":"kjv"}]},{"language":"Romanian","versions":[{"name":"VDC","abbreviation":"vdc"}]}]"#;
    let v = parse_index(json).unwrap();
    assert_eq!(v[0].code, "en_kjv"); assert_eq!(v[0].public_domain, Some(true));
    assert_eq!(v[1].code, "ro_vdc"); assert_eq!(v[1].public_domain, Some(false));
    assert!(version_url("en_kjv").contains(PIN));
}
```

- [ ] **Step 3: Run**, expect compile failure.

- [ ] **Step 4: Implement.** `mod.rs`: `Book`, `Bible`, `load` reads the file, strips a UTF-8 BOM, `serde_json::from_str::<Vec<Book>>`, `code` is the file stem. `parse_index` maps the repo index shape: language display name to a code prefix through a table `("English","en"), ("Romanian","ro"), ("Italian","it"), ("Portuguese","pt"), ("Spanish","es"), ("German","de"), ("French","fr")`, other languages lower cased first two letters. `public_domain`: `Some(true)` for `en_kjv en_kjva en_asv en_darby en_dra en_ylt98 en_gnv en_web en_bsb la_vulg cs_bkr hu_hunk eo_esp tl_abtag`, `Some(false)` for `en_niv en_esv en_nlt en_nasb en_nkjv en_csb en_amp en_gnt en_cev it_irb20 ro_vdc es_rvr1960 de_sch1951`, else `None`. `index_url` and `version_url` use `https://raw.githubusercontent.com/thiagobodruk/bible/{PIN}/json/`. `verses` clamps the range to the chapter length. `search` folds query and verses with `fold::fold`, scans all verses, stops at `limit`. `verse_slides` chunks by `per_slide.max(1)`.

  `names.rs`: three `[&'static str; 66]` arrays `EN`, `IT`, `RO` with canonical names, and `fn aliases(lang) -> &'static [(&'static str, usize)]` with folded lower case aliases: for En include three letter abbreviations and common forms like `gen`, `ex`, `ps`, `psalm`, `song`, `jn`, `jhn`, `rev`; for Ro `geneza`, `exod`, `psalmi`, `ioan`, `apocalipsa`; for It `genesi`, `esodo`, `salmi`, `giovanni`, `apocalisse`. Matching strips dots and folds.

  `reference.rs`: `parse_reference` lower cases and folds the input, splits the leading book part from the first digit that starts the chapter, allowing a leading `1 `, `2 `, `3 ` or `i `, `ii `, `iii ` book prefix. Book matching: exact canonical name, then alias, then unique prefix of the canonical name in the given language, then any language. Chapter and verses parsed from `C`, `C:V`, `C,V`, `C:V-W`, `C:V–W`. `format_reference` prints `Name C` or `Name C:V` or `Name C:V-W`.

- [ ] **Step 5: Run** tests, expect pass. Clippy clean.
- [ ] **Step 6: Commit:** `feat(core): bible loader, localized references, verse search and slides`

### Task 4: store, history, overrides, downloads record, library paths, sync_file

**Files:**
- Create: `crates/sda-core/src/store.rs`, `history.rs`, `overrides.rs`, `tests/store_test.rs`, `tests/history_test.rs`
- Modify: `crates/sda-core/src/library.rs`, `sync.rs`, `refresh.rs` (call `sync_hymns`), `lib.rs`
- Test: `tests/sync_test.rs`

**Interfaces:** Produces `store`, `history`, `overrides`, `library` additions, `sync_file`, `sync_hymns` per Shared Interfaces.

- [ ] **Step 1: Tests.** `tests/store_test.rs`:

```rust
#[test] fn json_round_trip_and_missing_is_default() {
    let dir = tempfile::tempdir().unwrap(); let p = dir.path().join("x.json");
    assert_eq!(sda_core::store::json_load::<Vec<u32>>(&p), Vec::<u32>::new());
    sda_core::store::json_save(&p, &vec![1u32, 2]).unwrap();
    assert_eq!(sda_core::store::json_load::<Vec<u32>>(&p), vec![1, 2]);
    assert!(!dir.path().join("x.json.tmp").exists());
}
```

  `tests/history_test.rs`:

```rust
use sda_core::history::*;
fn e(p: &str, t: i64) -> HistoryEntry { HistoryEntry { path: p.into(), number: None, title: p.into(), opened_at: t } }
#[test] fn record_dedupes_by_path_newest_first_and_caps() {
    let mut l = Vec::new();
    for i in 0..60 { record(&mut l, e(&format!("/{i}"), i)); }
    assert_eq!(l.len(), 50); assert_eq!(l[0].path.to_str(), Some("/59"));
    record(&mut l, e("/30", 100));
    assert_eq!(l[0].path.to_str(), Some("/30")); assert_eq!(l.iter().filter(|x| x.path.to_str() == Some("/30")).count(), 1);
    assert_eq!(recent(&l, 3).len(), 3);
}
```

  In `tests/sync_test.rs` add `sync_file_downloads_then_unchanged` using the existing TCP stub: first call returns `Downloaded` and the file bytes equal the body, second call with the same ETag returns `Unchanged`, deleting the dest and calling again returns `Downloaded`. Also add in `library` inline tests: `config_without_bible_version_parses` and `bible_version_round_trips`.

- [ ] **Step 2: Run**, expect compile failures.
- [ ] **Step 3: Implement.** `store.rs`: `json_load` reads and deserializes, returns `T::default()` on any error with a `log::warn!`. `json_save` writes `path.with_extension("json.tmp")` then `fs::rename`. Create the parent dir. `history.rs` per interfaces. `overrides.rs`: `#[derive(Default, Serialize, Deserialize)] pub struct Overrides { pub as_is: BTreeMap<PathBuf, bool> }`. `library.rs`: add `bible_version` field, `bibles_dir = data_dir/bibles`, `history_path = data_dir/history.json`, `downloads_path = data_dir/downloads.json`, `overrides_path = data_dir/overrides.json`, `window_path = data_dir/window.json`, `asis_cache_dir = cache_dir/asis`. `sync.rs`: extract the HEAD, stamp compare, GET to `.part`, rename, write stamp into `sync_file`. `sync_hymns` calls `sync_file(url, dest_dir.with_extension("zip"), stamp)` and when `Downloaded` extracts into `tmp`, swaps dirs, removes the zip. Keep `extract_zip`.
- [ ] **Step 4: Run** `cargo test -p sda-core`, expect pass.
- [ ] **Step 5: Commit:** `feat(core): json store, history, overrides, bible paths, generic file sync`

### Task 5: as-is converter detection and render

**Files:**
- Create: `crates/sda-core/src/asis.rs`, `tests/asis_test.rs`
- Modify: `lib.rs`

**Interfaces:** Produces `asis` per Shared Interfaces.

- [ ] **Step 1: Tests:**

```rust
use sda_core::asis::*; use std::path::{Path, PathBuf};
#[test] fn detect_finds_soffice_on_path_var() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join(if cfg!(windows) { "soffice.exe" } else { "soffice" });
    std::fs::write(&exe, "").unwrap();
    #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap(); }
    assert!(matches!(detect_in(dir.path().to_str().unwrap(), &[]), Some(Converter::LibreOffice(p)) if p == exe));
    assert!(detect_in("", &[]).is_none());
}
#[test] fn detect_prefers_candidates_that_exist() {
    let dir = tempfile::tempdir().unwrap(); let app = dir.path().join("Microsoft PowerPoint.app"); std::fs::create_dir(&app).unwrap();
    assert!(matches!(detect_in("", &[app.clone()]), Some(Converter::PowerPoint(p)) if p == app));
}
#[test] fn cache_dir_changes_with_mtime_and_size() {
    let a = cache_dir_for(Path::new("/c"), Path::new("/h/1.pptx"), 1, (1920, 1080));
    assert_ne!(a, cache_dir_for(Path::new("/c"), Path::new("/h/1.pptx"), 2, (1920, 1080)));
    assert_ne!(a, cache_dir_for(Path::new("/c"), Path::new("/h/1.pptx"), 1, (1280, 720)));
    assert!(a.starts_with("/c"));
}
#[test] fn cached_requires_done_marker_and_sorts_numerically() {
    let dir = tempfile::tempdir().unwrap();
    for n in [10, 2, 1] { std::fs::write(dir.path().join(format!("{n}.png")), "").unwrap(); }
    assert!(cached(dir.path()).is_none());
    std::fs::write(dir.path().join(".done"), "").unwrap();
    let v = cached(dir.path()).unwrap();
    assert_eq!(v.iter().map(|p| p.file_name().unwrap().to_str().unwrap()).collect::<Vec<_>>(), ["1.png", "2.png", "10.png"]);
}
```

- [ ] **Step 2: Run**, expect compile failure.
- [ ] **Step 3: Implement.** `detect_in(path_var, candidates)`: candidates that exist first, a path ending in `.app` or containing `POWERPNT` is `PowerPoint`, else `LibreOffice`; then each `path_var` entry joined with `soffice` or `soffice.exe` that exists. `detect()` calls it with `std::env::var("PATH")` and OS candidates: macOS `/Applications/LibreOffice.app/Contents/MacOS/soffice`, `/Applications/Microsoft PowerPoint.app`; Windows `C:\Program Files\LibreOffice\program\soffice.exe`, `C:\Program Files\Microsoft Office\root\Office16\POWERPNT.EXE`; Linux `/usr/bin/soffice`. `cache_dir_for` hashes path, mtime, size with `sha2` and returns `root/<hex16>`. `cached` lists `*.png` sorted by numeric stem when `.done` exists. `render`: LibreOffice runs `soffice --headless --convert-to pdf --outdir <out_dir> <pptx>`, then opens the PDF with `hayro::Pdf` and renders each page with `hayro::render` at a scale so the width equals `size.0`, saving `out_dir/<n>.png` with `image`. PowerPoint on macOS runs `osascript -e` with a script that tells `application "Microsoft PowerPoint"` to open the file and `save as` each slide as PNG into `out_dir`; on Windows runs `powershell -NoProfile -Command` with COM `New-Object -ComObject PowerPoint.Application`, `Presentations.Open`, `Slides | ForEach Export`. Both write `.done` last. Return the sorted list. Use `hayro`'s documented API from its docs.rs page; if the API differs from `hayro::Pdf` and `hayro::render`, adapt and note it in the commit body.
- [ ] **Step 4: Run** tests, expect pass. Then a manual check on this Mac: `cargo run -p sda-core --example asis_probe -- crates/sda-core/tests/fixtures/150.pptx /tmp/asis` where the example calls `detect` and `render` and prints the paths. Create `crates/sda-core/examples/asis_probe.rs` for this and keep it. Expect PNG files when PowerPoint is installed.
- [ ] **Step 5: Commit:** `feat(core): detect LibreOffice or PowerPoint and render pptx slides to png`

### Task 6: downloader probe and quality, i18n strings

**Files:**
- Modify: `crates/sda-core/src/downloader.rs`, `crates/sda-core/src/i18n.rs`
- Test: `tests/downloader_test.rs`, create `tests/fixtures/ytdlp_probe.json`

**Interfaces:** Produces `Quality`, `Probe`, `parse_probe`, `probe`, `start_download(url, format_id, dir, on_event)`, `DownloadRecord`. Produces the new `Strings` fields listed below.

- [ ] **Step 1: Fixture.** `tests/fixtures/ytdlp_probe.json` is a trimmed `yt-dlp -J` output: `{"title":"Sample","thumbnail":"https://x/t.jpg","duration":125,"formats":[{"format_id":"18","height":360,"vcodec":"avc1","acodec":"mp4a","filesize":1000},{"format_id":"136","height":720,"vcodec":"avc1","acodec":"none","filesize":5000},{"format_id":"137","height":1080,"vcodec":"avc1","acodec":"none","filesize_approx":9000},{"format_id":"140","height":null,"vcodec":"none","acodec":"mp4a"},{"format_id":"299","height":1080,"vcodec":"avc1","acodec":"none","filesize":9500}]}`.
- [ ] **Step 2: Tests:**

```rust
#[test] fn parse_probe_dedupes_by_height_and_sorts_desc() {
    let p = sda_core::downloader::parse_probe(include_str!("fixtures/ytdlp_probe.json")).unwrap();
    assert_eq!(p.title, "Sample"); assert_eq!(p.duration_secs, Some(125));
    let h: Vec<u32> = p.qualities.iter().map(|q| q.height).collect();
    assert_eq!(h, [1080, 720, 360]);
    assert_eq!(p.qualities[0].format_id, "299");   // largest file wins at equal height
    assert_eq!(p.qualities[0].label, "1080p");
}
```

- [ ] **Step 3: Implement.** `parse_probe` with `serde_json::Value`: keep formats with `vcodec != "none"` and a numeric `height`, group by height keeping the one with the largest `filesize` or `filesize_approx`, sort descending, label `"{height}p"`. `probe(url)` resolves tools as `start_download` does and runs `yt-dlp -J --no-warnings <url>`. `start_download` gains `format_id: Option<&str>`; when `Some` pass `-f <id>+bestaudio/best` else keep today's default. `DownloadRecord` derives serde.
- [ ] **Step 4: i18n.** Add to `Strings` for En, It, Ro: `tab_present, tab_bible, tab_videos, tab_themes, tab_settings, recent, nothing_live, go_live, original, original_hint, notes, bible_reference_hint, bible_version, bible_manage, bible_download, bible_remove, bible_public_domain, bible_restricted, bible_search_hint, bible_books, bible_chapters, bible_verses, bible_present_selected, videos_fetch, videos_quality, videos_folder, videos_download, videos_reveal, videos_history, theme_new, theme_duplicate, theme_delete, theme_favorite, theme_inherit, theme_text, theme_background, theme_transition, theme_verses, theme_footer, theme_opacity, theme_shadow, theme_outline, theme_bg_solid, theme_bg_gradient, theme_bg_particles, theme_bg_image, theme_bg_video, theme_duration, theme_easing, theme_per_slide, theme_show_reference, theme_show_numbers, settings_general, settings_library, settings_display, settings_presenting, settings_about, identify, default_display, converter_found, converter_missing, detect_again, data_folder, gpu_unavailable, projector_failed, status_rendering`. Values: plain US English, Italian, Romanian. Keep every existing field. Delete no existing field in this task.
- [ ] **Step 5: Run** `cargo test -p sda-core`. Expect pass.
- [ ] **Step 6: Commit:** `feat(core): yt-dlp probe with quality list, i18n strings for the new UI`

## Wave 1 gate

- [ ] `cargo test --workspace` green, `cargo clippy --workspace --all-targets` clean.

## Wave 2: renderer (one agent, sequential tasks)

### Task 7: `sda-render` crate, headless device, solid background, golden harness

**Files:**
- Create: `crates/sda-render/Cargo.toml`, `src/lib.rs`, `src/scene.rs`, `src/headless.rs`, `src/background.rs`, `shaders/solid.wgsl`, `tests/golden_test.rs`, `tests/golden/solid_black.png`
- Modify: `Cargo.toml` members

**Interfaces:** Produces `Scene`, `Layer`, `BackgroundState`, `Viewport`, `Renderer::new/prepare/render`, `headless::device`, `headless::render_to_image`. Text and transitions come in Tasks 8 and 9, so in this task `current` and `outgoing` are ignored by `render`.

- [ ] **Step 1: Cargo.toml**

```toml
[package]
name = "sda-render"
edition = "2024"
version.workspace = true

[dependencies]
sda-core = { path = "../sda-core" }
wgpu = { version = "27", default-features = false, features = ["std", "wgsl", "metal", "dx12", "vulkan"] }
cosmic-text = "0.15"
image = { version = "0.25", default-features = false, features = ["png", "jpeg", "gif"] }
bytemuck = { version = "1", features = ["derive"] }
pollster = "0.4"
log = "0.4"
anyhow = { workspace = true }

[dev-dependencies]
tempfile = "3"
```

  Add `"crates/sda-render"` to workspace members.

- [ ] **Step 2: Golden harness** in `tests/golden_test.rs`:

```rust
use sda_render::*; use std::path::Path;

fn check(name: &str, scene: &Scene) {
    let Some(img) = headless::render_to_image(scene) else { eprintln!("no GPU adapter, skipping {name}"); return };
    let path = Path::new("tests/golden").join(format!("{name}.png"));
    if std::env::var("UPDATE_GOLDENS").is_ok() || !path.exists() { img.save(&path).unwrap(); return; }
    let want = image::open(&path).unwrap().to_rgba8();
    assert_eq!(want.dimensions(), img.dimensions(), "{name} size");
    let bad = want.pixels().zip(img.pixels()).filter(|(a, b)| a.0.iter().zip(b.0.iter()).any(|(x, y)| x.abs_diff(*y) > 2)).count();
    let budget = (want.width() * want.height()) as usize / 1000;
    assert!(bad <= budget, "{name}: {bad} pixels differ, budget {budget}");
}

fn base() -> Scene {
    Scene { size: (320, 180), background: BackgroundState::Solid(sda_core::theme::Rgba::new(0, 0, 0, 255)), background_opacity: 1.0,
            current: None, outgoing: None, transition: sda_core::theme::Transition::default(), blank: false, now_secs: 0.0 }
}

#[test] fn solid_black() { check("solid_black", &base()); }
#[test] fn solid_half_opacity_red() {
    let mut s = base(); s.background = BackgroundState::Solid(sda_core::theme::Rgba::new(255, 0, 0, 255)); s.background_opacity = 0.5;
    let img = headless::render_to_image(&s); let Some(img) = img else { return };
    let p = img.get_pixel(10, 10); assert!(p[0] > 120 && p[0] < 136 && p[1] == 0, "{p:?}");
    check("solid_half_red", &s);
}
```

- [ ] **Step 3: Run** `cargo test -p sda-render`, expect compile failure.

- [ ] **Step 4: Implement.** `scene.rs` holds the Scene types. `Transition` needs `Default` in `sda-core`, add `impl Default for Transition` returning Fade 400 EaseInOut if Task 2 did not. `headless.rs`: `device()` creates `wgpu::Instance::default()`, requests any adapter with `request_adapter(&RequestAdapterOptions { compatible_surface: None, force_fallback_adapter: false, ..})` using `pollster::block_on`, then `request_device`. `render_to_image(scene)`: device, `Renderer::new(.., TextureFormat::Rgba8UnormSrgb)`, a texture of `scene.size` with `RENDER_ATTACHMENT | COPY_SRC`, `prepare`, `render`, copy to a buffer with 256 byte row padding, map, unpad, return `RgbaImage`. `background.rs`: a `SolidPipeline` with `solid.wgsl` drawing a fullscreen triangle with a uniform `color: vec4<f32>` where alpha is `background_opacity` and the pass clears to black. `Renderer` holds the pipelines and a `Viewport`. `render` sets the viewport and scissor to `Viewport`.

- [ ] **Step 5: Run** tests twice. First run writes goldens, second run compares. Commit goldens.
- [ ] **Step 6: Commit:** `feat(render): wgpu renderer crate with headless goldens and solid background`

### Task 8: Text layer with cosmic-text, auto fit, shadow and outline, footer

**Files:**
- Create: `crates/sda-render/src/text.rs`, `crates/sda-render/src/fonts.rs`, `tests/golden/text_center_auto.png`, `tests/golden/text_left_pt_shadow.png`, `tests/golden/text_footer.png`, `tests/golden/verses.png`
- Modify: `src/lib.rs`, `src/scene.rs`, `tests/golden_test.rs`

**Interfaces:** Produces `fonts()` and the internal `TextRaster` that turns a `ResolvedSlide` into an RGBA texture. `Layer::Text` now renders.

- [ ] **Step 1: Copy the 8 ttf files** from `assets/fonts/` into `crates/sda-render/fonts/` with `git mv` so the GUI crate no longer owns them, and `fonts.rs` returns `include_bytes!` for each as `(family, bytes)`.
- [ ] **Step 2: Tests.** Add to `golden_test.rs`:

```rust
fn styled(text: &str, align: HAlign, size: TextSize, shadow: Option<Shadow>) -> Arc<ResolvedSlide> {
    Arc::new(ResolvedSlide { background: Rgba::new(0,0,0,255), footer: None, boxes: vec![ResolvedBox {
        frame: Frame { x: 0.06, y: 0.06, w: 0.88, h: 0.88 }, v_align: VAlign::Middle,
        lines: text.lines().map(|l| ResolvedLine { align, spacing: 1.1, runs: vec![ResolvedRun { text: l.into(),
            font: Font::Bundled(BundledFont::Inter), size, bold: false, italic: false, color: Rgba::new(255,255,255,255), opacity: 1.0, shadow, outline: None }] }).collect() }] })
}
#[test] fn text_center_auto() { let mut s = base(); s.current = Some(Layer::Text(styled("Cerul, pământul\nse vor topi", HAlign::Center, TextSize::Auto, None))); check("text_center_auto", &s); }
#[test] fn text_left_pt_shadow() { let mut s = base(); s.current = Some(Layer::Text(styled("Plecaţi-vă", HAlign::Left, TextSize::Pt(24.0), Some(Shadow { color: Rgba::new(0,0,0,255), offset: (2.0, 2.0), blur: 0.0 })))); check("text_left_pt_shadow", &s); }
#[test] fn text_footer() { let mut s = base(); let mut r = (*styled("A", HAlign::Center, TextSize::Pt(30.0), None)).clone(); r.footer = Some("150 · Cerul".into()); s.current = Some(Layer::Text(Arc::new(r))); check("text_footer", &s); }
#[test] fn auto_text_fills_box_without_overflow() {
    let mut s = base(); s.current = Some(Layer::Text(styled("Short", HAlign::Center, TextSize::Auto, None)));
    let Some(img) = headless::render_to_image(&s) else { return };
    let white_rows: Vec<u32> = (0..180).filter(|y| (0..320).any(|x| img.get_pixel(x, *y)[0] > 200)).collect();
    assert!(white_rows.len() > 60, "auto size should be large, got {} rows", white_rows.len());
    assert!(*white_rows.first().unwrap() >= 10 && *white_rows.last().unwrap() <= 170);
}
```

- [ ] **Step 3: Implement** `text.rs`. `TextRaster { font_system: cosmic_text::FontSystem, swash: SwashCache }` built with `FontSystem::new_with_fonts(fonts().iter().map(|(_, b)| Source::Binary(Arc::new(b.to_vec()))))` plus system fonts through `FontSystem::new()` merged by `db_mut().load_font_data` for each bundled font. `fn rasterize(&mut self, slide: &ResolvedSlide, size: (u32, u32)) -> RgbaImage`: for each box compute pixel rect from frame. For a box whose runs all have `TextSize::Auto`, call `sda_core::fit::fit_size(measure, (w, h), 12.0, 400.0)` where `measure(pt)` shapes the lines in a `Buffer` with `Metrics::new(pt, pt * spacing)` and `set_size(Some(w), None)` then returns `(max line width, total height)`. Then shape at the chosen size, align each line per `align` inside the box, vertically place per `v_align`, and draw glyphs with `swash.get_image` into the `RgbaImage` compositing with alpha times run opacity. Shadow draws the same glyphs first with offset and color. Outline draws the glyphs at 8 offsets of `width` in the outline color before the fill. Footer uses `Pt(size.1 as f32 * 0.03)` in the bottom right with 2 percent margin, white at 70 percent opacity. Cache in `Renderer`: `HashMap<u64, wgpu::Texture>` keyed by hash of `Arc::as_ptr` plus size, with an LRU `VecDeque` capped at 16. A `LayerPipeline` with `image.wgsl` draws a textured quad with uniforms `alpha, offset, scale`. In `render`, after the background, draw `current` with alpha 1.

- [ ] **Step 4: Run** tests, write goldens, run again, inspect `tests/golden/*.png` by opening them. Confirm text is centered and the auto size fills the box.
- [ ] **Step 5: Commit:** `feat(render): text layer with cosmic-text, auto fit, shadow, outline, footer`

### Task 9: Transitions, bitmap layer, blank, image background

**Files:**
- Create: `crates/sda-render/src/composite.rs`, `shaders/composite.wgsl`, `shaders/image.wgsl`, goldens `fade_half.png`, `slide_left_half.png`, `scale_half.png`, `blur_half.png`, `bitmap.png`, `blank.png`, `image_bg.png`
- Modify: `src/lib.rs`, `tests/golden_test.rs`

- [ ] **Step 1: Tests:**

```rust
fn two(kind: TransitionKind) -> Scene {
    let mut s = base(); s.current = Some(Layer::Text(styled("B", HAlign::Center, TextSize::Pt(80.0), None)));
    s.outgoing = Some((Layer::Text(styled("A", HAlign::Center, TextSize::Pt(80.0), None)), 0.5));
    s.transition = Transition { kind, duration_ms: 400, easing: Easing::Linear }; s
}
#[test] fn fade_half() { check("fade_half", &two(TransitionKind::Fade)); }
#[test] fn slide_left_half() { check("slide_left_half", &two(TransitionKind::Slide(Direction::Left))); }
#[test] fn scale_half() { check("scale_half", &two(TransitionKind::Scale)); }
#[test] fn blur_half() { check("blur_half", &two(TransitionKind::Blur)); }
#[test] fn blank_hides_text() { let mut s = two(TransitionKind::Fade); s.blank = true; let Some(img) = headless::render_to_image(&s) else { return }; assert!(img.pixels().all(|p| p[0] < 3)); check("blank", &s); }
#[test] fn bitmap_layer_covers_output() {
    let mut bmp = image::RgbaImage::new(64, 36); for p in bmp.pixels_mut() { *p = image::Rgba([0, 200, 0, 255]); }
    let mut s = base(); s.current = Some(Layer::Bitmap(Arc::new(bmp)));
    let Some(img) = headless::render_to_image(&s) else { return }; assert!(img.get_pixel(160, 90)[1] > 180); check("bitmap", &s);
}
#[test] fn image_background() { let mut bmp = image::RgbaImage::new(8, 8); for (x, _, p) in bmp.enumerate_pixels_mut() { *p = image::Rgba([x as u8 * 30, 0, 0, 255]); } let mut s = base(); s.background = BackgroundState::Image(Arc::new(bmp)); check("image_bg", &s); }
```

- [ ] **Step 2: Implement.** `composite.rs`: the `LayerPipeline` grows uniforms `alpha: f32, offset: vec2<f32>, scale: f32, blur: f32`. `composite.wgsl` samples 9 taps along both axes when `blur > 0`. Per kind, with eased `p = easing.apply(progress)`: Fade outgoing `alpha = 1 - p`, current `alpha = p`. Slide(dir) outgoing offset moves out by `p` in `dir`, current moves in from the opposite side by `1 - p`. Scale outgoing `scale = 1 + 0.2p, alpha = 1 - p`, current `scale = 0.8 + 0.2p, alpha = p`. Blur outgoing `blur = p * 8 px, alpha = 1 - p`, current `blur = (1 - p) * 8, alpha = p`. None draws current only. Blank skips both layers. Bitmap layers upload the `RgbaImage` to a texture cached by `Arc::as_ptr` and draw with `scale` to fit cover. Image background uploads once and draws under the layers with `background_opacity`.
- [ ] **Step 3: Run**, goldens, inspect the half way PNGs by eye. Commit: `feat(render): transitions, bitmap layer, blank, image background`

### Task 10: Gradient and particle shaders, video decoder

**Files:**
- Create: `shaders/gradient.wgsl`, `shaders/particles.wgsl`, `src/video.rs`, goldens `gradient_t0.png`, `gradient_t2.png`, `particles_t0.png`
- Modify: `src/background.rs`, `src/lib.rs`, `tests/golden_test.rs`

- [ ] **Step 1: Tests:**

```rust
#[test] fn gradient_drifts_with_time() {
    let g = BackgroundState::Gradient { stops: vec![(0.0, Rgba::new(20, 0, 60, 255)), (1.0, Rgba::new(0, 120, 200, 255))], angle: 30.0, drift: 0.3 };
    let mut a = base(); a.background = g.clone(); check("gradient_t0", &a);
    let mut b = a.clone(); b.now_secs = 2.0; check("gradient_t2", &b);
    let (Some(ia), Some(ib)) = (headless::render_to_image(&a), headless::render_to_image(&b)) else { return };
    assert_ne!(ia.get_pixel(50, 50), ib.get_pixel(50, 50));
}
#[test] fn particles_draw_bright_points() {
    let mut s = base(); s.background = BackgroundState::Particles { base: Rgba::new(0,0,20,255), color: Rgba::new(255,255,255,255), count: 120, speed: 0.4 };
    let Some(img) = headless::render_to_image(&s) else { return };
    assert!(img.pixels().filter(|p| p[0] > 180).count() > 50); check("particles_t0", &s);
}
#[test] fn decoder_fails_cleanly_without_ffmpeg() {
    assert!(spawn_decoder(std::path::Path::new("/nonexistent/ffmpeg"), std::path::Path::new("/x.mp4"), (64, 36)).is_err());
}
```

- [ ] **Step 2: Implement.** `gradient.wgsl`: uniforms `stops[8]: vec4` colors, `positions: vec4<f32>` x2, `count, angle, phase`. Fragment projects uv onto the angle axis, adds `phase = drift * now_secs` wrapped with a smooth triangle wave so the gradient slides back and forth, and interpolates stops. `particles.wgsl`: a vertex shader with `count * 6` vertices, instance id hashed to a seed, position `fract(seed.xy + vec2(0, speed * now_secs * k))`, size 2 to 5 px, soft circle in the fragment, additive blend over the base color pass. `video.rs`: `FrameSource { latest: Mutex<Option<Arc<RgbaImage>>>, size }`. `spawn_decoder` checks the ffmpeg path exists, spawns `ffmpeg -loglevel error -stream_loop -1 -i <media> -f rawvideo -pix_fmt rgba -s WxH -r 30 -` with stdout piped, a thread reads exactly `W*H*4` bytes per frame into a fresh `RgbaImage` and stores it. Process is killed on `Drop`. `BackgroundState::Video` uploads `latest()` in `prepare` when the Arc pointer changed.
- [ ] **Step 3: Run**, goldens, inspect. Then a manual check: `cargo run -p sda-render --example video_probe -- <path to an mp4 or gif> /tmp/frame.png` with an example that spawns the decoder with the app's ffmpeg from `sda_core::downloader` tool resolution, waits 500 ms, saves the latest frame. Keep the example. Use any short mp4 on this Mac or download one with yt-dlp in the next wave.
- [ ] **Step 4: Commit:** `feat(render): animated gradient and particle backgrounds, ffmpeg video frames`

## Wave 2 gate

- [ ] `cargo test --workspace` green, clippy clean, all goldens committed and inspected by eye.

## Wave 3: app scaffold (one agent)

### Task 11: `sda-app` daemon with shell, tabs, Live strip, scene widget, shortcuts, workers

**Files:**
- Create: `crates/sda-app/Cargo.toml`, `build.rs`, `src/main.rs`, `src/app.rs`, `src/msg.rs`, `src/state/mod.rs`, `src/state/core.rs`, `src/state/live.rs`, `src/view/mod.rs`, `src/view/shell.rs`, `src/view/live_strip.rs`, `src/view/placeholder.rs`, `src/widgets/mod.rs`, `src/widgets/scene_view.rs`, `src/widgets/card.rs`, `src/widgets/segmented.rs`, `src/ui_theme.rs`, `src/shortcuts.rs`, `src/displays.rs`, `src/workers.rs`, `tests/views_test.rs`
- Modify: `Cargo.toml` members, `assets/icon.png` stays, move `crates/sda-gui/build.rs` logic here (it was deleted in Task 0, re-create it: on Windows `winresource::WindowsResource::new().set_icon("../../assets/icon.ico").compile()`).

**Interfaces:** Produces `App`, `Msg`, `Tab`, `App::for_test() -> App` (Core with `cfg_path: None`, themes dir and data paths pointed at a fresh `tempfile::tempdir` kept alive in a `static OnceLock`, a `Searcher` over the four fixture hymns parsed from `../sda-core/tests/fixtures`, `gpu_ok: false`, `displays` one fake entry), `App::for_test_with_dirs(themes_dir: PathBuf) -> App`, `App::view_operator(&self) -> Element<Msg>`, `App::update(&mut self, Msg) -> Task<Msg>`, `state::core::Core`, `state::live::LiveState`, `widgets::scene_view::scene_view(scene: Arc<Scene>) -> Element`, `ui_theme::theme() -> iced::Theme`, `ui_theme::card`, `ui_theme::accent_button`, `shortcuts::filter(event, status, window_id, in_projector: bool) -> Option<Msg>`, `displays::list() -> Vec<Display>`, `workers::boot(core) -> Task<Msg>`, `workers::force_sync`, `workers::reindex`. Views for the other tabs are `view::placeholder::show(title)` until Wave 4 replaces them. Each Wave 4 task replaces one `placeholder` call in `shell.rs`, so `shell.rs` has one line per tab:

```rust
Tab::Present => view::present::show(app),
Tab::Bible => view::bible::show(app),
Tab::Videos => view::videos::show(app),
Tab::Themes => view::themes::show(app),
Tab::Settings => view::settings::show(app),
```

  In this task the five modules exist and each `show` returns `placeholder::show(&app.core.strings.tab_present)` and so on. Each Wave 4 task owns exactly its `view/<tab>.rs` and `state/<tab>.rs` and its `Msg` sub enum file `msg/<tab>.rs`. Declare the sub enums now as `pub enum BibleMsg { Noop }`, `VideosMsg { Noop }`, `ThemesMsg { Noop, SetActive(String) }`, `SettingsMsg { Noop, Detected(Option<Converter>) }` in `src/msg/{bible,videos,themes,settings}.rs` and `update` forwards each to `state::<tab>::update(app, msg) -> Task<Msg>`. In this task `state::settings::update` handles `Detected` by setting `app.core.converter`, and `state::themes::update` handles `SetActive` by calling `core.set_active_theme` and setting `app.live.theme`. Wave 4 fills in the rest.

- [ ] **Step 1: Cargo.toml**

```toml
[package]
name = "sda-app"
edition = "2024"
version.workspace = true

[[bin]]
name = "sda-manager"
path = "src/main.rs"

[dependencies]
sda-core = { path = "../sda-core" }
sda-render = { path = "../sda-render" }
iced = { version = "0.14", features = ["wgpu", "image", "tokio", "advanced", "lazy"] }
wgpu = { version = "27", default-features = false }
display-info = "0.5"
rfd = "0.17"
open = "5"
sys-locale = "0.3"
anyhow = { workspace = true }
log = "0.4"
env_logger = "0.11"
image = { version = "0.25", default-features = false, features = ["png", "jpeg"] }

[dev-dependencies]
iced_test = "0.14"

[target.'cfg(windows)'.build-dependencies]
winresource = "0.1"
```

- [ ] **Step 2: main.rs**

```rust
#![windows_subsystem = "windows"]
mod app; mod msg; mod state; mod view; mod widgets; mod ui_theme; mod shortcuts; mod displays; mod workers;

fn main() -> iced::Result {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("sda_manager=info,sda_core=info,sda_render=info")).init();
    let mut daemon = iced::daemon(app::App::boot, app::App::update, app::App::view)
        .title(app::App::title).subscription(app::App::subscription).theme(|_, _| ui_theme::theme())
        .default_font(iced::Font::with_name("Inter"));
    for (_, bytes) in sda_render::fonts() { daemon = daemon.font(*bytes); }
    daemon.run()
}
```

  Note `#![windows_subsystem = "windows"]` goes on the binary crate. Logs go to stderr on macOS.

- [ ] **Step 3: app.rs.** `App { core: Core, tab: Tab, live: LiveState, present: state::present::PresentState, bible, videos, themes, settings, operator: window::Id, projector: Option<window::Id>, displays: Vec<Display>, status: Status, gpu_ok: bool }`. `boot() -> (App, Task<Msg>)` loads `Core`, opens the operator window with `window::Settings { size: saved or 1280x800, position: saved or Centered, min_size: Some(960x640), ..Default::default() }`, chains `workers::boot(&core)` and `Task::perform(async { displays::list() }, Msg::DisplaysListed)`. `update` matches every `Msg` in `msg.rs`; tab sub messages forward. `view(&self, id)` returns `view::projector::show(self)` when `Some(id) == self.projector` else `view::shell::show(self)`. `subscription`: `Subscription::batch([event::listen_with(|e, s, id| shortcuts::filter(e, s, id)).map(..), window::close_events().map(Msg::ProjectorClosed), frames when self.live.animating()])`. `title(&self, id)` returns `"SDA Manager"` or `"SDA Manager Projector"`.

- [ ] **Step 4: state/core.rs** is today's `sda-gui/src/core.rs` moved: `Core { cfg, cfg_path, language, strings, searcher: Option<Searcher>, themes: Vec<Theme>, active_theme: Theme, history: Vec<HistoryEntry>, overrides: Overrides, downloads: Vec<DownloadRecord>, converter: Option<Converter> }` plus `paths: Paths { themes_dir, history, overrides, downloads, window, bibles_dir, asis_cache }` filled from `sda_core::library` in `load` and from a tempdir in `for_test`, with `load`, `save_cfg`, `save_history`, `save_overrides`, `save_downloads`, `set_language`, `reload_themes`, `set_active_theme`. All JSON through `store::json_load` and `json_save`. No worker spawning here, that is `workers.rs`.

- [ ] **Step 5: state/live.rs.** `LiveState { present: Presentation, theme: Theme, started: Option<Instant>, outgoing: Option<SlideContent>, ffmpeg: Option<PathBuf>, video: Option<Arc<FrameSource>>, bitmaps: HashMap<PathBuf, Arc<RgbaImage>> }`. `fn scene(&self, size: (u32, u32), now: Instant) -> Arc<Scene>` builds the scene: background from `theme.background` using `video` or decoded `Image` cached in `bitmaps`, `current` from `resolve(&theme, slide, footer)` wrapped in `Arc` and cached by `(index, theme.cache_key())`, `outgoing` with progress `elapsed / duration` while `started` is younger than the duration, `blank`, `now_secs` since app start. `fn animating(&self) -> bool` is true while a transition runs or `theme.background.is_animated()`. `go(presentation)`, `next`, `prev`, `jump`, `blank`, `stop` set `outgoing` to the previous slide and `started = Some(now)`.

- [ ] **Step 6: widgets/scene_view.rs.** The shader wrapper:

```rust
pub fn scene_view<'a, M: 'a>(scene: Arc<Scene>) -> iced::Element<'a, M> { iced::widget::shader(SceneProgram { scene }).width(Fill).height(Fill).into() }
struct SceneProgram { scene: Arc<Scene> }
impl<M> shader::Program<M> for SceneProgram { type State = (); type Primitive = ScenePrimitive;
    fn draw(&self, _: &(), _: mouse::Cursor, _: Rectangle) -> ScenePrimitive { ScenePrimitive { scene: self.scene.clone() } } }
#[derive(Debug)] struct ScenePrimitive { scene: Arc<Scene> }
struct ScenePipeline { renderer: sda_render::Renderer }
impl shader::Pipeline for ScenePipeline { fn new(device, queue, format) -> Self { Self { renderer: Renderer::new(device, queue, format) } } fn trim(&mut self) {} }
impl shader::Primitive for ScenePrimitive { type Pipeline = ScenePipeline;
    fn prepare(&self, p: &mut ScenePipeline, device, queue, bounds: &Rectangle, viewport: &Viewport) {
        let scale = viewport.scale_factor() as f32; let mut scene = (*self.scene).clone();
        scene.size = ((bounds.width * scale) as u32, (bounds.height * scale) as u32); p.renderer.prepare(device, queue, &scene); }
    fn draw(&self, _: &ScenePipeline, _: &mut wgpu::RenderPass) -> bool { false }
    fn render(&self, p: &ScenePipeline, encoder, target, clip: &Rectangle<u32>) {
        p.renderer.render(encoder, target, sda_render::Viewport { x: clip.x, y: clip.y, w: clip.width, h: clip.height }); } }
```

  `Scene` must derive `Clone` and `Debug`. Verify the exact `Primitive` trait signatures against `iced_wgpu` 0.14 docs on docs.rs before writing. Shaders need the sRGB format Iced gives, pass it through.

- [ ] **Step 7: ui_theme.rs.** `theme()` returns `Theme::custom_with_fn("SDA", Palette { background: 0x14161A, text: 0xE6E8EC, primary: 0x4F8CFF, success: 0x3DD68C, warning: 0xF5B83D, danger: 0xF06A6A }, palette::Extended::generate)`. `card(theme) -> container::Style` with `background 0x1C1F26`, `border radius 8`, `shadow` offset 0 4 blur 16 color black 40 percent. `accent_button(theme, status) -> button::Style` primary fill, radius 8, lighter on hover. `ghost_button`. `chip`. Constants `SPACING = 12.0`, `RADIUS = 8.0`.

- [ ] **Step 8: shell.rs and live_strip.rs.** `shell::show(app)`: `column![top_bar, content.height(Fill), live_strip::show(app)]`. Top bar: `row![text("SDA Manager").size(18), horizontal_space(), segmented tabs, horizontal_space(), status chip]` padded 12. Tabs are five `button`s styled as a segmented control with the active one filled, each emitting `Msg::Tab(t)`. Live strip per spec 5.2: thumbnail is `container(scene_view(app.live.scene((160, 90), now))).width(160).height(90)` when `app.gpu_ok && app.live.present.is_loaded()`, else `text(nothing_live)`. Buttons emit `LivePrev, LiveNext, LiveBlank, LiveStop`. Display picker is `pick_list(displays labels, selected, Msg::ProjectorDisplay)`. Projector toggle is `toggler` or a button emitting `ProjectorToggle`, disabled when `app.displays.len() < 2`.

- [ ] **Step 9: shortcuts.rs.**

```rust
pub fn filter(event: Event, status: event::Status, _id: window::Id) -> Option<Msg> {
    if status == event::Status::Captured { return None; }
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else { return None };
    let cmd = modifiers.command();
    Some(match key.as_ref() {
        Key::Character("k") if cmd => Msg::FocusSearch,
        Key::Character("o") if cmd => Msg::ProjectorToggle,
        Key::Character(d @ ("1" | "2" | "3" | "4" | "5")) if cmd => Msg::Tab(Tab::nth(d.parse::<usize>().unwrap() - 1)),
        Key::Character(d) if !cmd && d.len() == 1 && d.as_bytes()[0].is_ascii_digit() && d != "0" => Msg::LiveJump(d.parse().unwrap()),
        Key::Character("b") | Key::Character("B") if !cmd => Msg::LiveBlank,
        Key::Named(Named::ArrowRight) | Key::Named(Named::Space) => Msg::LiveNext,
        Key::Named(Named::ArrowLeft) => Msg::LivePrev,
        Key::Named(Named::ArrowUp) => Msg::MoveSelection(-1),
        Key::Named(Named::ArrowDown) => Msg::MoveSelection(1),
        Key::Named(Named::Enter) => Msg::GoLive,
        Key::Named(Named::Escape) => Msg::LiveStop,
        _ => return None,
    })
}
```

  Add `Msg::FocusSearch` to `msg.rs`. Search focus uses `text_input::focus(SEARCH_ID)` with a `widget::Id` constant in `view/present.rs`. Up and Down only act when the Present tab is active, enforce that in `update`.

- [ ] **Step 10: displays.rs** wraps `display_info::DisplayInfo::all()` into `Display { index, label: format!("Display {} {}x{}", i + 1, w, h), rect: (x, y, w, h) in logical points, scale, is_primary }`. `default_display(&[Display]) -> Option<usize>` first non primary.

- [ ] **Step 11: workers.rs.** `boot(core: &Core) -> Task<Msg>`: `Task::perform(async move { refresh::load_local(..) }, Msg::Indexed).chain(Task::perform(sync..., Msg::Synced)).chain(update check)`. Use `Task::future` with `tokio::task::spawn_blocking` for the blocking core calls. Same for `force_sync` and `reindex`. Add `fonts_list() -> Task<Msg>` that builds a `cosmic_text::FontSystem::new()` on a blocking thread and returns the sorted unique family names as `Msg::FontsListed`, stored in `app.themes.fonts`. Add `cosmic-text = "0.15"` to `sda-app` for this one call. Add `converter_detect() -> Task<Msg>` returning `Msg::Settings(SettingsMsg::Detected(asis::detect()))` and run both from `boot`.

- [ ] **Step 12: Tests** `tests/views_test.rs` using `iced_test`. The app crate is a binary, so make `src/lib.rs` export the modules and have `main.rs` call `sda_app::run()`. Tests:

```rust
use iced_test::{Simulator, selector}; use sda_app::{App, Msg, Tab};
fn app() -> App { App::for_test() }   // Core with cfg_path None, Searcher over the four fixture hymns, gpu_ok false
#[test] fn tab_buttons_switch_and_strip_stays() {
    let mut app = app(); let mut ui = Simulator::new(app.view_operator());
    ui.click(&*app.core.strings.tab_videos).unwrap();
    for m in ui.into_messages() { let _ = app.update(m); }
    assert_eq!(app.tab, Tab::Videos);
    let mut ui = Simulator::new(app.view_operator()); assert!(ui.find(&*app.core.strings.nothing_live).is_ok());
}
#[test] fn shortcut_filter_ignores_captured_and_maps_keys() {
    use iced::{event, keyboard::{self, key::Named, Key}, window};
    let press = |k: Key| iced::Event::Keyboard(keyboard::Event::KeyPressed { key: k.clone(), modified_key: k, physical_key: keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified), location: keyboard::Location::Standard, modifiers: keyboard::Modifiers::default(), text: None, repeat: false });
    assert!(matches!(sda_app::shortcuts::filter(press(Key::Named(Named::ArrowRight)), event::Status::Ignored, window::Id::unique()), Some(Msg::LiveNext)));
    assert!(sda_app::shortcuts::filter(press(Key::Named(Named::ArrowRight)), event::Status::Captured, window::Id::unique()).is_none());
    assert!(matches!(sda_app::shortcuts::filter(press(Key::Character("3".into())), event::Status::Ignored, window::Id::unique()), Some(Msg::LiveJump(3))));
}
```

  `App::for_test()` and `view_operator(&self) -> Element<Msg>` are `pub`. Adjust the `KeyPressed` literal to the exact 0.14 field set from docs.rs.

- [ ] **Step 13: Run** `cargo run -p sda-app` and confirm the window opens with the top bar, placeholder tabs, and the Live strip. Take a screenshot with `screencapture -x /tmp/shell.png` and open it. `cargo test -p sda-app` green. Clippy clean.
- [ ] **Step 14: Commit:** `feat(app): iced daemon shell with tabs, live strip, scene widget, shortcuts, workers`

## Wave 3 gate

- [ ] The window opens, tabs switch, the strip is visible, tests green, clippy clean.

## Wave 4: tabs (parallel, disjoint files)

Each task owns `src/state/<tab>.rs`, `src/view/<tab>.rs`, `src/msg/<tab>.rs`, and may append tests to `tests/views_test.rs` inside a module named after the tab. Nobody edits `app.rs`, `shell.rs`, or `msg.rs`. If you need a top level `Msg` that does not exist, use your tab's sub enum and handle it in your own `update`. Each `update(app: &mut App, msg: XMsg) -> Task<Msg>` has full access to `app.core` and `app.live`.

### Task 12: Present tab

**Files:** `src/state/present.rs`, `src/view/present.rs`, `src/msg/present.rs` (new, `PresentMsg`), tests module `present`.

**Interfaces:** `PresentState { query, hits: Vec<usize>, selected: Option<usize>, preview_slide: usize, scroll: scrollable::Id, as_is_busy: bool }`. `pub const SEARCH_ID: &str = "search"`. `state::present::run_search(app)`, `select(app, i)`, `move_selection(app, d)`, `go_live(app, slide)` implement `Msg::Search`, `Select`, `MoveSelection`, `GoLive`, `GoLiveAt`, `PreviewSlide`, `AsIsToggle`, `AsIsRendered`, `FocusSearch`. `app.rs` already routes those top level messages to these functions by name, written in Task 11 as calls to `state::present::*`, so keep the names.

- [ ] **Step 1: Tests:**

```rust
mod present {
    use super::*;
    #[test] fn typing_filters_and_selects_first() {
        let mut app = app(); let mut ui = Simulator::new(app.view_operator());
        ui.click(selector::id(iced::widget::Id::new(sda_app::state::present::SEARCH_ID))).unwrap(); ui.typewrite("664");
        for m in ui.into_messages() { let _ = app.update(m); }
        assert_eq!(app.present.hits.len(), 1); assert_eq!(app.present.selected, Some(0));
        let mut ui = Simulator::new(app.view_operator()); assert!(ui.find("664b").is_ok());
    }
    #[test] fn enter_goes_live_and_strip_shows_title() {
        let mut app = app(); let _ = app.update(Msg::Search("150".into())); let _ = app.update(Msg::GoLive);
        assert!(app.live.present.is_loaded()); assert_eq!(app.live.present.index, 0);
        let mut ui = Simulator::new(app.view_operator()); assert!(ui.find("150").is_ok());
        assert_eq!(app.core.history[0].number.as_deref(), Some("150"));
    }
    #[test] fn recent_shows_on_empty_query() {
        let mut app = app(); let _ = app.update(Msg::Search("150".into())); let _ = app.update(Msg::GoLive); let _ = app.update(Msg::Search(String::new()));
        let mut ui = Simulator::new(app.view_operator()); assert!(ui.find(&*app.core.strings.recent).is_ok());
    }
}
```

- [ ] **Step 2: Implement** per spec 5.2 Present tab. Search box `text_input(hint, &query).id(SEARCH_ID).on_input(Msg::Search)`. Results in a `scrollable` of `button`s styled as rows: number bold, title, library chip. `MoveSelection` scrolls with `scrollable::snap_to` to keep the selection visible. Preview card: `card(scene_view(preview_scene))` where `preview_scene` builds a `Scene` from the selected hymn at `preview_slide` with `app.live.theme`, no transition. Below it a horizontal `scrollable` of slide thumbnails each a small `scene_view` sized 128x72 wrapped in a button `PreviewSlide(i)`, limit to the first 12 and show `+N` when more. Go Live button `accent_button` emits `GoLiveAt(preview_slide)`. Original toggle `toggler(label, overrides.as_is.get(path), AsIsToggle)` only when `app.core.converter.is_some()`. Right column: theme quick picker as a vertical list of theme names each with a 96x54 `scene_view` thumbnail of sample text, clicking emits `ThemesMsg::SetActive(name)` through `Msg::Themes`. Notes pane shows `slide.notes` or nothing. `go_live` builds `Presentation` with `SlideContent::Styled` per slide, or `Bitmap` paths when the override is on and `asis::cached` is `Some`, else spawns `workers::asis_render` and sets `as_is_busy`. Records history and saves it. Hymn title is `"{number} {title}"` and the footer per slide is `format!("{number} · {title}")`, or just the title when there is no number.
- [ ] **Step 3: Run** app, type, arrow, Enter, confirm Preview and Live strip. Screenshot `/tmp/present.png`. Tests green.
- [ ] **Step 4: Commit:** `feat(app): present tab with search, recent, preview, slide strip, go live`

### Task 13: Bible tab

**Files:** `src/state/bible.rs`, `src/view/bible.rs`, `src/msg/bible.rs`, tests module `bible`.

**Interfaces:** `BibleMsg { Reference(String), Version(String), Book(usize), Chapter(u32), ToggleVerse(u32), SelectRange(u32, u32), ClearSelection, Search(String), Present, ManageOpen, ManageClose, Download(String), Remove(String), IndexLoaded(Result<Vec<VersionInfo>, String>), Downloaded(Result<String, String>), Loaded(Result<Arc<Bible>, String>) }`. `BibleState { reference_input, version: Option<String>, bible: Option<Arc<Bible>>, book: usize, chapter: u32, selected: BTreeSet<u32>, query, results, manage_open, index: Vec<VersionInfo>, installed: Vec<String>, busy: Option<String>, error: Option<String> }`.

- [ ] **Step 1: Tests:** `reference_field_jumps_to_chapter` types `John 3:16-18` into the reference input and asserts `book == 42`, `chapter == 3`, `selected == {16,17,18}` with `App::for_test()` preloaded with `bible_mini.json` through `app.bible.bible = Some(..)`. `present_builds_verse_slides` sets a selection and sends `BibleMsg::Present`, asserts `app.live.present.slides.len()` equals `ceil(3 / per_slide)` and `Source::Bible`. `version_pick_list_lists_installed` finds the version abbreviation text.
- [ ] **Step 2: Implement** per spec 5.2 Bible tab. Version `pick_list` over `installed` codes showing `VersionInfo.abbreviation` upper case. Reference `text_input` with `on_submit`. Left: `scrollable` book list of `button`s with `book_name(i, lang)`, chapter grid of 6 columns. Center: verse rows each `checkbox(format!("{n} {text}"))` with `ToggleVerse`, Shift click range through `SelectRange` when the simulator supports modifiers, else a `Select to here` button on the row after the first selected. Header shows `format_reference` of the selection and `Present` accent button. Right: search `text_input` plus results as buttons that jump and select. Manage modal through `iced::widget::stack` and `opaque` overlay listing `index` grouped by language, each row with Download or Remove and a PD or Restricted chip from `public_domain`. `workers`: `bible_index(core)` downloads `index_url()` through `sync_file` into `bibles_dir/index.json` then `parse_index`; `bible_download(code)` through `sync_file(version_url(code), bibles_dir/<code>.json, <code>.etag)`; `bible_load(code)` through `bible::load`. On tab open with `bible == None`, load `cfg.bible_version` if installed. On version change save `cfg.bible_version`. `Present` calls `app.live.go(Presentation { title: format_reference(..), subtitle: Some(version abbr), slides: verse_slides(.., app.live.theme.verses.per_slide), source: Source::Bible {..} })`.
- [ ] **Step 3: Manual:** open Manage, download `en_kjv` and `ro_vdc`, present `Ioan 3:16-18` in Romanian, screenshot `/tmp/bible.png`.
- [ ] **Step 4: Commit:** `feat(app): bible tab with versions, browsing, reference jump, search, present`

### Task 14: Videos tab

**Files:** `src/state/videos.rs`, `src/view/videos.rs`, `src/msg/videos.rs`, tests module `videos`.

**Interfaces:** `VideosMsg { Url(String), Fetch, Probed(Result<Probe, String>), Quality(String), ChooseFolder, FolderChosen(Option<PathBuf>), Download, Event(DownloadEvent), Reveal(PathBuf) }`. `VideosState { url, probe: Option<Probe>, format_id: Option<String>, dir: PathBuf, phase: Phase, percent, speed, eta, title, message }`, `Phase { Idle, Probing, Ready, Downloading, Done(PathBuf), Failed }`.

- [ ] **Step 1: Tests:** `quality_radio_sets_format_id` loads `parse_probe(include_str!("../../sda-core/tests/fixtures/ytdlp_probe.json"))` into state, renders, clicks `"720p"`, asserts `format_id == Some("136")`. `download_button_disabled_until_probed`.
- [ ] **Step 2: Implement** per spec 5.2 Videos tab. `workers::probe(url)` and `workers::download(url, format_id, dir)` where download uses `Task::run(stream)` with an `mpsc` channel fed by `start_download`'s callback on a blocking thread. On `Done` push a `DownloadRecord` and save. Folder picker through `rfd::AsyncFileDialog::new().pick_folder()`. History list below from `core.downloads` with Reveal buttons using `open::that`.
- [ ] **Step 3: Manual:** download a short public video at 360p into `/tmp/vid`, confirm progress and Reveal. Screenshot `/tmp/videos.png`.
- [ ] **Step 4: Commit:** `feat(app): videos tab with probe, quality choice, folder, progress, history`

### Task 15: Themes tab

**Files:** `src/state/themes.rs`, `src/view/themes.rs`, `src/view/theme_form.rs`, `src/msg/themes.rs`, tests module `themes`.

**Interfaces:** `ThemesMsg { Select(usize), New, Duplicate, Delete, Save, SetActive(String), Favorite(bool), Draft(DraftEdit), PickImage, PickVideo, FilePicked(Option<PathBuf>), Tick }`. `DraftEdit` is an enum with one variant per form control: `Name(String), FontInherit(bool), Font(Font), SizeInherit(bool), SizeAuto(bool), SizePt(f32), BoldInherit(bool), Bold(bool), ColorInherit(bool), Color(Rgba), HAlignInherit(bool), HAlign(HAlign), VAlign(VAlign), SpacingInherit(bool), Spacing(f32), MarginInherit(bool), Margin(f32), Opacity(f32), ShadowOn(bool), ShadowColor(Rgba), ShadowOffset(f32), OutlineOn(bool), OutlineColor(Rgba), OutlineWidth(f32), BgKind(BgKind), BgSolid(Rgba), BgStop(usize, Rgba), BgAngle(f32), BgDrift(f32), BgParticleColor(Rgba), BgParticleCount(u32), BgParticleSpeed(f32), BgOpacity(f32), TransitionKind(TransitionKind), Duration(u32), Easing(Easing), PerSlide(u8), ShowReference(bool), ShowNumbers(bool), RefPos(RefPos), Footer(bool) }`. `ThemesState { draft: Theme, draft_index: usize, fonts: Vec<String>, preview_started: Instant, preview_flip: bool }`.

- [ ] **Step 1: Tests:** `inherit_checkbox_flips_prop` renders, clicks the Inherit checkbox next to the color control, asserts `draft.text.color` toggles between `Inherit` and `Set(..)`. `save_persists_theme_file` with `core.themes_dir` pointed at a tempdir through `App::for_test_with_dirs`.
- [ ] **Step 2: Implement** per spec 5.2 Themes tab. Color control: a `text_input` with hex plus a 24x24 swatch, since Iced has no color picker. Font `pick_list` lists bundled labels then system family names from `app.themes.fonts`. Background kind through `widgets::segmented`. Preview card at the top: `scene_view` of the draft with sample text `"Cerul, pământul se vor topi!"` and an `outgoing` sample so the transition loops every 3 s driven by `Msg::Tick` while the tab is open. Save writes through `store::save_theme`, reloads `core.themes`. SetActive updates `core.active_theme`, `cfg.active_theme`, `app.live.theme`, saves cfg. Favorite sorts favorites first. Delete refuses Default.
- [ ] **Step 3: Manual:** create a theme with a gradient background and Slide transition, set active, go live, see the transition. Screenshot `/tmp/themes.png`.
- [ ] **Step 4: Commit:** `feat(app): themes tab with live preview, inherit controls, backgrounds, favorites`

### Task 16: Settings tab and screen picker

**Files:** `src/state/settings.rs`, `src/view/settings.rs`, `src/view/screen_picker.rs`, `src/msg/settings.rs`, tests module `settings`.

**Interfaces:** `SettingsMsg { Section(Section), Language(Language), CheckUpdate, ForceSync, AddFolder, FolderPicked(Option<PathBuf>), RemoveFolder(String), EnableFolder(String, bool), DefaultDisplay(usize), Identify, DetectConverter, Detected(Option<Converter>), BibleDefault(String), RevealData }`. `Section { General, Library, Display, Presenting, About }`. `screen_picker::show(displays, selected, on_pick) -> Element` draws each display as a `container` positioned by its rect scaled to fit 360x200 using `canvas`, feature `canvas` must be added to the Iced features in `Cargo.toml` by this task, with a number label and a highlight on the selected one.

- [ ] **Step 1: Tests:** `add_folder_button_emits_message` clicks the add button and asserts `SettingsMsg::AddFolder` in the messages. `language_pick_updates_strings` sends `Language(It)` and asserts `app.core.strings.tab_settings == "Impostazioni"`.
- [ ] **Step 2: Implement** per spec 5.2 Settings tab. Library folders table rows: name, path truncated, enabled `toggler`, status chip `available` or `missing`, Remove. Force sync shows `status.sync`. Display section: `screen_picker`, `Identify` emits top level `Msg::Identify`, default display `pick_list`. Presenting: converter row with `Detected` text from `Converter` path and Detect again, Bible default version `pick_list` over installed codes. About: version from `env!("CARGO_PKG_VERSION")`, fonts licenses text, data folder path with Reveal.
- [ ] **Step 3: Manual:** switch language to Romanian, add a folder, see reindex count. Screenshot `/tmp/settings.png`.
- [ ] **Step 4: Commit:** `feat(app): settings tab with library folders, screen picker, presenting, about`

## Wave 4 gate

- [ ] `cargo test --workspace` green, clippy clean, five screenshots exist and look like the spec.

## Wave 5: integration (one agent, sequential)

### Task 17: Projector window, displays, identify, frames subscription

**Files:** `src/view/projector.rs`, `src/state/projector.rs` (new), modify `app.rs` for `ProjectorToggle`, `ProjectorDisplay`, `ProjectorClosed`, `Identify`, `IdentifyDone`, `Tick`.

- [ ] **Step 1: Implement** per spec 5.5. `state::projector::open(app, display) -> Task<Msg>` builds the `window::Settings`, `let (id, task) = window::open(settings); app.projector = Some(id); task.then(move |_| window::set_mode(id, Mode::Fullscreen)).discard()`. Save `cfg.output_display`. `close(app)` is `window::close(id)`. `view::projector::show(app)` is `container(scene_view(app.live.scene(size, now))).width(Fill).height(Fill)` with mouse cursor hidden through `mouse_area` or `window::change_icon` is not needed, use `iced::widget::mouse_area(..).interaction(mouse::Interaction::None)`. `Identify` opens one window per display sized 240x160 at `rect.origin + (40, 40)`, `decorations: false`, `level: AlwaysOnTop`, showing `text(index + 1).size(96)`, stores the ids in `app.identify: Vec<window::Id>`, and `Task::perform(tokio::time::sleep(3 s), |_| Msg::IdentifyDone)` closes them. `ProjectorClosed(id)` clears state when `id` matches. `Tick(now)` from `window::frames()` stores `app.now` and ends the transition when elapsed exceeds the duration, so the subscription stops.
- [ ] **Step 2: Manual on this Mac:** with one display, use `Identify` to see a numbered window. Then temporarily set the toggle enabled for one display in a debug env var `SDA_ALLOW_SINGLE_DISPLAY=1` read in `live_strip.rs` so the projector can open fullscreen on the only screen for testing, and keep that env var in the code. Open the projector, press Right, see the Fade. Press Esc, see it close. Screenshot the projector with `screencapture -x /tmp/projector.png`.
- [ ] **Step 3: Commit:** `feat(app): projector window on a chosen display, identify, frame driven transitions`

### Task 18: As-is integration, video background wiring, GPU failure banner

**Files:** modify `src/state/live.rs`, `src/state/present.rs`, `src/workers.rs`, `src/view/shell.rs`.

- [ ] **Step 1:** `workers::asis_render(converter, pptx, mtime, size) -> Task<Msg>` returns `AsIsRendered(Ok((pptx, pngs)))`. `AsIsRendered` in `state::present` loads the PNGs into `app.live.bitmaps` as `Arc<RgbaImage>` and if the live presentation is that hymn swaps its slides to `Bitmap`. Preview shows a `ProgressBar` indeterminate replacement text `status_rendering` while `as_is_busy`.
- [ ] **Step 2:** On `SetActive` and on boot, if `theme.background` is `Video`, resolve ffmpeg through `sda_core::downloader` tool lookup and `spawn_decoder` at 1920x1080, store in `app.live.video`. On a theme change drop the old one. Missing ffmpeg sets `status.library` to a message and uses a still.
- [ ] **Step 3:** `gpu_ok` is set false when `sda_render::headless::device()` returns `None` at boot, which is a cheap probe run once on a worker. When false, `shell.rs` shows a top banner with `gpu_unavailable`, previews render the slide text as plain Iced `text`, the projector toggle is disabled.
- [ ] **Step 4:** Manual: toggle Original on hymn 150 with PowerPoint installed, see bitmap slides in Preview and Live. Set a theme with a `Video` background using a short mp4 from the Videos tab, go live, see it loop in the projector. Commit: `feat(app): as-is slides, video backgrounds, gpu fallback banner`

### Task 19: Window persistence, boot order, status chip, polish pass

**Files:** modify `app.rs`, `state/core.rs`, `view/shell.rs`.

- [ ] **Step 1:** `window::events()` subscription filtered to `Moved` and `Resized` for the operator id updates `app.window_geom` and starts a 500 ms `Task::perform(sleep)` that sends `SaveWindow`, which writes `window.json` through `store::json_save`. Boot reads it.
- [ ] **Step 2:** Status chip shows `status.library` and a dot colored by state: accent while syncing, success when ready, danger on error. Update text when a new version exists with a button that opens the release page through `open::that`.
- [ ] **Step 3:** Polish: consistent 12 px padding, all buttons through `ui_theme`, every `text_input` has a hint, focus ring color accent, empty states have one sentence of guidance in all three languages. Walk every tab and fix anything misaligned. Screenshot every tab again into `docs/screenshots/*.png` and commit them.
- [ ] **Step 4:** Commit: `feat(app): window geometry persistence, status chip, visual polish`

## Wave 6: packaging, docs, QA (one agent)

### Task 20: release.yml, installers, CLAUDE.md, README

- [ ] `release.yml`: Linux deps `libxkbcommon-dev libwayland-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libgtk-3-dev libasound2-dev`. Build `-p sda-app`. Add a `test` job that runs `cargo test --workspace` on all three OSes before `build`. Keep `hymns.zip`.
- [ ] `CLAUDE.md`: rewrite What this is, Commands, Architecture for three crates, module lists, data flow, keyboard table, tests, Bible notes, as-is notes. `README.md`: features list with Bible and animated backgrounds, screenshots from `docs/screenshots`, config example with `bible_version`, Bible data source note with the license warning per version.
- [ ] `grep -rn "egui\|eframe\|sda-gui\|sda_gui" --include='*.rs' --include='*.toml' --include='*.yml' --include='*.md' . | grep -v target | grep -v docs/superpowers` is empty.
- [ ] Commit: `ci: test and package the iced app, docs for 3.0`

### Task 21: Full verification and QA report

- [ ] `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo build --release -p sda-app`, `ls -la target/release/sda-manager`.
- [ ] Local sync QA: zip `assets/920` to `/tmp/hz/hymns.zip`, `python3 -m http.server 8765`, point `hymns_url` at it, run with `RUST_LOG=debug`, expect `sync ok: Downloaded` then `Unchanged`. Restore `hymns_url` to the default afterwards, verify with `head -1` of the config.
- [ ] Manual scenarios, each with a screenshot in `/tmp/qa/`: boot shows library count. Search `plecati` finds Plecaţi-vă. Enter goes live. Projector opens with the env var, Right and Left transition, B blanks, Esc closes. Settings language to It and Ro, every tab readable. Add a folder, remove it. Theme with gradient, particles, image, video backgrounds, each seen live. Theme saved and favorited survives relaunch. Bible: download KJV, present John 3:16-18, switch to a second version. Videos: probe, pick 360p, download, Reveal. Original toggle renders bitmaps. Relaunch restores window geometry. History shows the last hymns on empty search.
- [ ] Push a tag `v3.0.0-rc1` and run the Release workflow. Record the three archive names and sizes.
- [ ] Write the final report: what passed, what was fixed, what is unverified on Windows.
