# SDA Manager 3.0: Iced rewrite design

Date: 2026-10-18. Status: approved by the owner in chat, all sections.

## 1. Goal

Replace the egui GUI with an Iced 0.14 desktop app, add a GPU renderer for the
projected output, add Bible verse presenting, keep every current feature, and
run on macOS and Windows. The owner approved: parser based rendering as the
default with an opt in "show the PPTX as is" mode, a full GPU scene for the
projector, theme wins over PPTX styling with a per property Inherit choice,
tabs with a persistent Live strip, and the three crate split below.

Feature list the app must deliver:

1. List, browse, fuzzy search hymns by number, title, filename, and lyrics.
2. User folders of PPTX files included in search.
3. Preview every slide, then Go Live to a projector window on a chosen display.
4. Settings, three UI languages, fully customizable presentation styling.
5. Opt in as-is presenting of the original PPTX through a converter when one is installed.
6. Video downloader tab with quality choice and a chosen save folder.
7. Saved favorite themes: font, background, opacity, slide transitions, animated backgrounds including looping video and gif.
8. Parser keeps PPTX colors, sizes, bold, alignment, box position, and slide background when the theme says Inherit.
9. History of last opened hymns by number and title.
10. Bible: present one verse or a range, browse, search, switch language and version, themed like hymns.
11. Modern, clean UI. One dark palette now, built so a second palette is one file.

## 2. Crates

```
crates/
  sda-core/    logic, no UI, no GPU. Tested with fixtures.
  sda-render/  wgpu scene renderer, no Iced. Tested headless with golden PNGs.
  sda-app/     Iced daemon, binary `sda-manager`. Tested with iced_test.
```

`sda-gui` is deleted. `sda-core` keeps its modules and gains new ones. Binary
name, config paths, themes dir, hymns sync, and the update check are unchanged
so existing installs upgrade in place.

Tech stack: Rust 1.88 or newer, edition 2024 for the two new crates. Iced
0.14.0 with features `wgpu`, `image`, `tokio`, `advanced`. `iced_test` for
headless UI tests. `wgpu` 27, `cosmic-text` 0.15 to match Iced. `display-info`
0.5, `rfd` 0.17, `open` 5, `sys-locale` 0.3, `hayro` for PDF raster, `image`
0.25 for stills. Existing core deps stay. No GStreamer. No git2.

## 3. Data model, `sda-core`

### 3.1 Styled hymn

```rust
pub struct Hymn { pub number: Option<String>, pub title: String, pub slides: Vec<Slide>,
                  pub path: PathBuf, pub library: String, pub mtime: i64 }
pub struct Slide { pub background: Option<Rgba>, pub boxes: Vec<TextBox>, pub notes: Option<String> }
pub struct TextBox { pub frame: Option<Frame>, pub paragraphs: Vec<Paragraph> }
pub struct Frame { pub x: f32, pub y: f32, pub w: f32, pub h: f32 }   // fractions of the slide, 0..1
pub struct Paragraph { pub align: Option<HAlign>, pub line_spacing: Option<f32>, pub runs: Vec<Run> }
pub struct Run { pub text: String, pub color: Option<Rgba>, pub size_pt: Option<f32>,
                 pub bold: Option<bool>, pub italic: Option<bool>, pub underline: Option<bool>,
                 pub font: Option<String> }
impl Slide { pub fn plain_text(&self) -> String }   // paragraphs joined by '\n', chrome removed
```

`HymnEntry` is replaced by `Hymn`. The parser in `pptx.rs` resolves colors:
`srgbClr` direct, `schemeClr` through the slide master `clrMap` and the theme
`clrScheme`, then applies `lumMod` and `lumOff`. Sizes come from `sz` in
hundredths of a point. Frames come from `a:xfrm` divided by `p:sldSz`. Slide
background comes from the slide `p:bg`, else the layout, else the master. Chrome
lines, the footer "IMNURI CREȘTINE" and the "N/M" counter, are dropped from
boxes as today. Speaker notes come from `ppt/notesSlides/notesSlideN.xml`.
`CACHE_VERSION` bumps to 8.

### 3.2 Theme

```rust
pub enum Prop<T> { Inherit, Set(T) }
pub struct Theme { pub name: String, pub text: TextStyle, pub background: Background,
                   pub transition: Transition, pub footer: bool, pub verses: VerseLayout,
                   pub favorite: bool }
pub struct TextStyle { pub font: Prop<Font>, pub size: Prop<TextSize>, pub bold: Prop<bool>,
    pub color: Prop<Rgba>, pub h_align: Prop<HAlign>, pub v_align: VAlign,
    pub line_spacing: Prop<f32>, pub margin: Prop<f32>, pub opacity: f32,
    pub shadow: Option<Shadow>, pub outline: Option<Outline> }
pub enum Font { Bundled(BundledFont), System(String) }
pub enum BundledFont { Inter, NotoSerif, Lora, RobotoSlab }
pub enum TextSize { Auto, Pt(f32) }
pub struct Shadow { pub color: Rgba, pub offset: (f32, f32), pub blur: f32 }
pub struct Outline { pub color: Rgba, pub width: f32 }
pub enum Background {
    Solid(Rgba),
    Gradient { stops: Vec<(f32, Rgba)>, angle: f32, drift: f32 },
    Particles { base: Rgba, color: Rgba, count: u32, speed: f32 },
    Image(PathBuf),
    Video { path: PathBuf },        // gif too
}
pub struct Transition { pub kind: TransitionKind, pub duration_ms: u32, pub easing: Easing }
pub enum TransitionKind { None, Fade, Slide(Direction), Scale, Blur }
pub enum Easing { Linear, EaseInOut, EaseOut }
pub struct VerseLayout { pub per_slide: u8, pub show_reference: bool, pub show_numbers: bool,
                         pub reference_at: RefPos }
pub enum RefPos { Top, Bottom }
```

Background opacity is `background_opacity: f32` on `Theme`, blended over black.
The Default theme ships with font, size, bold, color, h_align, line_spacing,
and margin on Inherit, v_align Middle, black solid background, Fade 400 ms
EaseInOut, footer on, verses per_slide 1 with reference at Bottom.

Fallbacks when a property is Inherit and the PPTX has no value: Inter, Auto,
not bold, white, Center, line spacing 1.1, margin 0.06 of the short side.

Old 2.0 theme JSON loads through a `#[serde(from = "ThemeV2")]` adapter that
maps flat fields into `Set` values. `theme::store` keeps `list_themes`,
`load_theme`, `save_theme`, `delete_theme`.

### 3.3 Resolve

```rust
pub struct ResolvedSlide { pub background: ResolvedBackground, pub boxes: Vec<ResolvedBox>,
                           pub footer: Option<String> }
pub struct ResolvedBox { pub frame: Frame, pub v_align: VAlign, pub lines: Vec<ResolvedLine> }
pub struct ResolvedLine { pub align: HAlign, pub spacing: f32, pub runs: Vec<ResolvedRun> }
pub struct ResolvedRun { pub text: String, pub font: Font, pub size: TextSize, pub bold: bool,
                         pub italic: bool, pub color: Rgba, pub opacity: f32,
                         pub shadow: Option<Shadow>, pub outline: Option<Outline> }
pub fn resolve(theme: &Theme, content: &SlideContent) -> ResolvedSlide
```

`resolve` is pure and lives in `sda-core::resolve`. A slide with no frames
gets one box covering the slide minus margin. Verses produce one box with the
verse lines and the reference as a separate small line at `reference_at`.

### 3.4 Presentation

```rust
pub enum SlideContent { Styled(Slide), Verses { reference: String, verses: Vec<(u32, String)> }, Bitmap(PathBuf) }
pub struct Presentation { pub title: String, pub subtitle: Option<String>, pub slides: Vec<SlideContent>,
                          pub index: usize, pub blank: bool, pub source: Source }
pub enum Source { Hymn(PathBuf), Bible { version: String, reference: String }, None }
```

`present.rs` keeps `next`, `prev`, `toggle_blank`, `jump`, and adds
`load(Presentation)`. The Live strip and projector read only this struct.

### 3.5 Bible

```rust
pub struct Book { pub abbrev: String, pub name: String, pub chapters: Vec<Vec<String>> }
pub struct Bible { pub code: String, pub books: Vec<Book> }   // 66 books
pub struct VersionInfo { pub code: String, pub language: String, pub name: String,
                         pub abbreviation: String, pub public_domain: Option<bool> }
pub fn load(path: &Path) -> Result<Bible>
pub fn index_url() -> String             // pinned commit 0e8fd65a89727f5b73e9f88edb220da8e0d5daba
pub fn version_url(code: &str) -> String
pub struct Reference { pub book: usize, pub chapter: u32, pub verses: Option<(u32, u32)> }
pub fn parse_reference(input: &str, lang: Language) -> Option<Reference>
pub fn book_name(book: usize, lang: Language) -> &'static str
pub fn search(bible: &Bible, query: &str, limit: usize) -> Vec<(Reference, String)>
```

Book name tables for en, it, ro with common aliases live in `bible/names.rs`.
Reference parsing accepts `:` or `,` between chapter and verse, `-` for ranges,
and matches book names and aliases after folding. `search` folds both sides
and does a substring scan over all verses, which takes under 20 ms on 31k
verses. Downloaded files live in `data_dir/bibles/<code>.json` with an
`<code>.etag` stamp, through the existing `sync` module generalized to take a
URL, a destination file, and a stamp. `public_domain` is a small built in list
of known PD codes, `None` when unknown.

### 3.6 Persistence

Config TOML keeps all current keys and adds `bible_version: Option<String>`.
New JSON files in the data dir, each a `Vec` capped at 50 and newest first:
`history.json` with path, number, title, time. `downloads.json` with title,
path, time. `overrides.json` is a map from hymn path to `{ as_is: bool }`.
`window.json` stores operator window size and position, written on move and
resize debounced 500 ms. All through one `store::json_load` and
`store::json_save` pair with atomic rename.

### 3.7 Other core changes

`search.rs` adds two rules with tests: a query in double quotes forces an exact
phrase, and an exact number match ranks above everything else. `history.rs`
with `record` and `recent`. `asis.rs` per section 6. `downloader.rs` gains
`probe` per section 7. `i18n.rs` gains the strings listed in the plan for
Bible, Themes, Settings sections, and the Live strip. `sync.rs` generalizes to
`sync_file(url, dest, stamp) -> Result<SyncOutcome>` and `sync_hymns` calls it.

## 4. Renderer, `sda-render`

No Iced dependency. Public API:

```rust
pub struct Scene { pub size: (u32, u32), pub background: BackgroundState, pub background_opacity: f32,
                   pub current: Option<Layer>, pub outgoing: Option<(Layer, f32)>,
                   pub transition: Transition, pub blank: bool, pub now_secs: f32 }
pub enum Layer { Text(Arc<ResolvedSlide>), Bitmap(Arc<RgbaImage>) }
pub enum BackgroundState { Solid(Rgba), Gradient {..}, Particles {..}, Image(Arc<RgbaImage>),
                           Video(Arc<FrameSource>) }
pub struct Renderer { .. }
impl Renderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self;
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene);
    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView, viewport: Rect<u32>);
}
pub struct FrameSource { .. }   // double buffered RGBA frames from the ffmpeg decoder
pub fn spawn_decoder(ffmpeg: &Path, media: &Path, size: (u32, u32)) -> Result<Arc<FrameSource>>
pub mod headless { pub fn device() -> Option<(wgpu::Device, wgpu::Queue)>;
                   pub fn render_to_png(scene: &Scene) -> Option<RgbaImage> }
```

Passes, bottom to top:

1. Background. One WGSL shader per kind. Gradient and Particles take
   `now_secs` as a uniform. Particles are point sprites seeded by a hash in the
   shader, no CPU list. Image and Video sample a texture. Video uploads the
   newest decoded frame per `prepare`.
2. Text. Each `ResolvedSlide` is shaped with cosmic-text and rasterized into a
   per slide offscreen RGBA texture at scene size. Auto size runs `fit_size`
   over cosmic-text measurement per box. Shadow and outline are extra glyph
   draws. Textures are cached by hash of the resolved slide and size, 16
   entries, LRU.
3. Composite. Draws outgoing then current with per layer uniforms: alpha for
   Fade, offset for Slide, scale for Scale, a 9 tap blur for Blur. Progress
   is the eased `outgoing.1`. `blank` draws nothing above the background.

Fonts: the four bundled families are embedded. System fonts load through
cosmic-text's `fontdb` once at renderer start on a worker thread. A missing
system font falls back to Inter.

Video decoding runs `ffmpeg -stream_loop -1 -i <media> -f rawvideo -pix_fmt rgba -s WxH -`
and reads frames on a thread into the `FrameSource` double buffer. Gif uses
the same path. Missing ffmpeg means a still from the `image` crate if it can
decode the file, else solid black and an app status message.

Time comes from the scene, never from the clock inside the renderer, so a
test can render progress 0.5 of a transition deterministically.

## 5. App, `sda-app`

### 5.1 Structure

```
src/main.rs        iced::daemon, logging, fonts, settings
src/app.rs         App, Msg, update, view(window_id) routing, subscriptions
src/state/         live.rs, present.rs, bible.rs, videos.rs, themes.rs, settings.rs
src/view/          shell.rs (frame + tabs), live_strip.rs, present.rs, bible.rs, videos.rs,
                   themes.rs, theme_form.rs, settings.rs, screen_picker.rs, projector.rs
src/widgets/       scene_view.rs (shader widget over sda-render), card.rs, chip.rs, segmented.rs
src/ui_theme.rs    palette, container and button styles
src/shortcuts.rs   event filter
src/displays.rs    display-info wrapper
src/workers.rs     Task builders for boot, sync, reindex, probe, download, asis, bibles
```

`App` is one struct owned by Iced. `Core` from today moves into
`sda-app::state::core` and keeps `load`, `save_cfg`, `set_language`, themes
handling. No `Rc<RefCell>`.

### 5.2 Operator window

Top row: app title left, tab bar center, status chip right with library
count, sync state, update available. Tabs: Present, Bible, Videos, Themes,
Settings. Below: the active tab. Bottom: the Live strip, 88 px, always
present.

Live strip: live thumbnail 160 px wide rendered by `scene_view` or "Nothing
live", title and subtitle, `n / total`, prev, next, blank, stop, display
picker, projector toggle. Toggle disabled with a tooltip when one display
exists.

Present tab: left 320 px search box and results, with a Recent section on an
empty query. Center: Preview card, slide thumbnails strip, Go Live, and an
Original toggle shown only when a converter exists. Right 280 px: theme quick
picker with rendered thumbnails and a notes pane.

Bible tab: top row with version picker and a reference field. Typing a
reference jumps to it. Left: book list, chapter grid. Center: verse list with
checkboxes to pick one or many, Go Live with the selection. Right: full text
search field and results. A Manage versions button opens a modal listing
versions grouped by language with Download and Remove, a PD or Restricted
hint, and download progress.

Videos tab: URL field, Fetch, then title, thumbnail, quality radio rows from
`probe`, folder picker, Download, progress with speed and ETA, Reveal. Past
downloads list below.

Themes tab: left theme cards with rendered thumbnails, favorite star, New,
Duplicate, Delete. Right: a live preview card looping the transition every 3
s, then the form grouped as Text, Background, Transition, Verses, Footer.
Every text property has an Inherit checkbox. Background kind is a segmented
control that swaps the form below. Image and Video get a file picker.

Settings tab: section list left. General: language, check updates. Library:
Force sync, user folders table with add, remove, enable, status. Display:
screen picker drawn at real positions, Identify, default display. Presenting:
converter detection with path and Detect again, Bible default version. About:
version, licenses, data folder with Reveal.

### 5.3 Visual style

Dark neutral palette through `Theme::custom_with_fn`, one accent, 8 px radius,
soft shadows on cards, 12 px spacing grid, Inter for UI. All in `ui_theme.rs`.
A second palette later is one more function.

### 5.4 Keyboard

`event::listen_with` filtering `Status::Ignored`. Cmd/Ctrl+K search, Up Down
move selection, Enter go live, Left Right Space live prev next, B blank, Esc
stop projector, 1 to 9 jump, Ctrl+1 to Ctrl+5 tabs, Ctrl+O projector toggle.
The projector window forwards the same keys. Shortcuts never fire while a
text field has focus.

### 5.5 Projector window and displays

`displays.rs` wraps `display-info` into `Display { index, label, rect, is_primary }`.
Open: `window::open` with `decorations: false`, `resizable: false`,
`position: Specific(rect.origin + 1 px)`, `size: rect.size`, `level: AlwaysOnTop`,
`exit_on_close_request: false`, then `window::set_mode(id, Fullscreen)`. The
projector view is one `scene_view` filling the window with the live scene.
`window::frames()` subscription runs only while a transition is in progress or
the background is animated. `close_events` clears the projector state. Esc
in the projector closes it. Identify opens one small window per display for 3
s with the display number. Open failures show a status message and reset the
toggle.

### 5.6 Workers and errors

Every long task runs through `Task::perform` or `Task::run` and sends one
`Msg` variant: `Indexed`, `Synced`, `Reindexed`, `UpdateChecked`,
`VideoProbed`, `VideoProgress`, `AsIsRendered`, `BiblesListed`,
`BibleDownloaded`, `BibleLoaded`, `DisplaysListed`, `FontsListed`. Failures
carry `Result<T, String>` and land in the relevant status line. No panic on a
user path. If wgpu device creation fails, the app opens with a banner "GPU
rendering unavailable", text previews, and a disabled projector toggle.

## 6. As-is pipeline, `sda-core::asis`

`detect() -> Option<Converter>` checks LibreOffice at standard paths and on
`PATH`, then PowerPoint on Windows through the registry and on macOS at
`/Applications/Microsoft PowerPoint.app`. Cached in memory, shown in Settings.

`render(converter, pptx, out_dir, size) -> Result<Vec<PathBuf>>`. LibreOffice:
`soffice --headless --convert-to pdf --outdir <tmp> <pptx>`, then `hayro`
rasterizes each page to `out_dir/<n>.png`. PowerPoint: a shipped PowerShell
script on Windows and a shipped `osascript` on macOS export each slide to PNG.
A `.done` marker written last makes the cache atomic. Cache dir is
`cache_dir/asis/<hash of path, mtime, size>/`.

The Original toggle renders on a worker with a spinner on the Preview card,
then `SlideContent::Bitmap` frames replace the styled slides for that hymn.
Transitions still apply. Text styling, background, and footer are off. The
choice is saved per hymn in `overrides.json`. If `hayro` output is wrong on
the fixtures, `pdfium-render` with its bundled binary replaces it behind the
same `render` signature.

## 7. Library, search, history, videos

Index, search, fold, sync, refresh stay. The cache stores `Hymn`. The
`Searcher` indexes plain text only. User folders keep the current config
entries and reindex worker.

`history::record(path, number, title)` on Go Live, deduplicated by path,
capped at 50. `recent(10)` feeds the Present tab.

`downloader::probe(url) -> Result<Probe>` runs `yt-dlp -J` and returns title,
thumbnail URL, duration, and `Vec<Quality { label, height, format_id, size }>`
deduplicated by height. `download(url, format_id, dir)` passes
`-f <format_id>+bestaudio/best`. Tool bootstrap unchanged.

## 8. Packaging

`release.yml` keeps the three native jobs plus `hymns.zip`. Linux deps change
to what Iced and rfd need. Binary name stays `sda-manager`. `install.sh`,
`install.ps1`, `update.rs` unchanged. Windows icon embed through `build.rs`
moves to `sda-app`. CI runs `cargo test --workspace` on all three OSes. Golden
render tests use a software adapter when no GPU exists and skip with a clear
message when none is available.

## 9. Testing

Core: fixture tests for styled parsing with hand read expected values,
`resolve` Inherit versus Set per property, history, `asis::detect` with a fake
`PATH`, `probe` parsing from a saved `yt-dlp -J` JSON, search rules, Bible
loader on a small fixture, reference parsing in three languages, Bible search.

Render: headless goldens for solid plus centered auto text, each transition at
progress 0.5, gradient at 0 s and 2 s, particles at 0 s, bitmap layer, blank,
footer, a verses slide. Tolerance 2 per channel and 0.1 percent differing
pixels. `UPDATE_GOLDENS=1` regenerates.

App: `iced_test` simulator per view. Search filters and selects, Enter goes
live and the strip shows the title, tab switch keeps the strip, Inherit
checkbox flips `Prop`, Settings add folder emits the message, Videos quality
radio sets the format id, Bible reference field jumps, verse checkboxes build
the selection. Shortcuts tested by feeding events to the filter function.

Manual QA on macOS by the agent after each UI task: build, launch, screenshot
every tab and the strip, open the projector, run a transition, set gradient
and video backgrounds, download a short video, switch language, force sync
against a local server, present a verse range, switch Bible version. Windows
is verified through CI only and listed as such in the final report.

Done when: all tests green on three OSes, clippy clean, release workflow
produces three archives plus `hymns.zip`, every numbered feature in section 1
maps to a test or a QA screenshot.
