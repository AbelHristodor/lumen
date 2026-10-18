//! Themes tab state. Owned by the Themes tab task. Holds the draft being
//! edited, a `LiveState` that plays the preview, and the thumbnail cache.

use crate::app::App;
use crate::msg::themes::{BgKind, ColorField, DraftEdit};
use crate::msg::{BgTarget, Msg, Tab, ThemesMsg};
use crate::state::core::Core;
use crate::state::live::LiveState;
use crate::workers;
use iced::Task;
use sda_core::i18n::{Language, Strings};
use sda_core::model::{Paragraph, Run, Slide, TextBox};
use sda_core::present::{Presentation, Source};
use sda_core::resolve::{ResolvedSlide, SlideContent, resolve};
use sda_core::theme::{
    Background, BundledFont, Font, HAlign, Outline, Prop, Rgba, Shadow, TextSize, Theme, store,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Widget id of the Inherit checkbox next to the text color control.
pub const COLOR_INHERIT_ID: &str = "theme-inherit-color";
/// Sample line shown as the current preview slide.
pub const PREVIEW_SAMPLE: &str = "Cerul, pământul se vor topi!";
/// How often the preview swaps its two sample slides.
pub const PREVIEW_PERIOD: Duration = Duration::from_secs(3);
const TICK: Duration = Duration::from_millis(16);

/// The last Set value of each inheritable text property. Restored when the
/// user turns Inherit back off, so a toggle does not lose the value.
#[derive(Default)]
pub struct InheritStash {
    pub font: Option<Font>,
    pub size: Option<TextSize>,
    pub bold: Option<bool>,
    pub color: Option<Rgba>,
    pub h_align: Option<HAlign>,
    pub line_spacing: Option<f32>,
    pub margin: Option<f32>,
}

pub struct ThemesState {
    pub draft: Theme,
    /// Position of the draft in `sorted_themes`.
    pub draft_index: usize,
    /// Font family names found on this machine, for the font picker.
    pub fonts: Vec<String>,
    /// When the preview last swapped its sample slides.
    pub preview_started: Instant,
    pub preview_flip: bool,
    /// Never reset. Drives animated thumbnails so they do not jump.
    pub clock: Instant,
    pub stash: InheritStash,
    /// Plays the draft with two sample slides so the transition loops.
    pub preview: LiveState,
    /// The color field being typed into and its raw text.
    pub hex_edit: Option<(ColorField, String)>,
    /// True while the Tick chain runs.
    pub ticking: bool,
    /// Last store error, shown under the toolbar.
    pub error: Option<String>,
    /// Resolved sample slide per theme cache key, for the list thumbnails.
    pub thumbs: HashMap<u64, Arc<ResolvedSlide>>,
}

impl Default for ThemesState {
    fn default() -> Self {
        let mut preview = LiveState::new(Theme::default());
        preview.go(sample_presentation(&Strings::for_language(Language::En)));
        ThemesState {
            draft: Theme::default(),
            draft_index: 0,
            fonts: Vec::new(),
            preview_started: Instant::now(),
            preview_flip: false,
            clock: Instant::now(),
            stash: InheritStash::default(),
            preview,
            hex_edit: None,
            ticking: false,
            error: None,
            thumbs: HashMap::new(),
        }
    }
}

/// Every loaded theme with favorites first. Order is otherwise the one on
/// disk, so Default stays ahead of the rest.
pub fn sorted_themes(core: &Core) -> Vec<&Theme> {
    let mut v: Vec<&Theme> = core.themes.iter().collect();
    v.sort_by_key(|t| !t.favorite);
    v
}

fn sample_slide(text: &str) -> SlideContent {
    SlideContent::Styled(Slide {
        background: None,
        boxes: vec![TextBox {
            frame: None,
            paragraphs: vec![Paragraph {
                align: None,
                line_spacing: None,
                runs: vec![Run {
                    text: text.to_string(),
                    ..Run::default()
                }],
            }],
        }],
        notes: None,
    })
}

fn sample_presentation(s: &Strings) -> Presentation {
    let mut p = Presentation::default();
    p.load(
        s.theme_preview_sample.clone(),
        None,
        vec![
            sample_slide(PREVIEW_SAMPLE),
            sample_slide(&s.theme_preview_sample),
        ],
        Source::None,
    );
    p
}

/// The sample slide resolved under `theme`, for a thumbnail.
pub fn thumb_slide(theme: &Theme, s: &Strings) -> Option<ResolvedSlide> {
    let footer = theme.footer.then_some(s.theme_preview_sample.as_str());
    resolve(theme, &sample_slide(PREVIEW_SAMPLE), footer)
}

/// A name no loaded theme uses, built from `base` with a counter.
fn unique_name(core: &Core, base: &str) -> String {
    let taken = |n: &str| core.themes.iter().any(|t| t.name == n);
    if !taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|i| format!("{base} {i}"))
        .find(|n| !taken(n))
        .unwrap_or_else(|| base.to_string())
}

/// The first color of a background, used when switching kinds.
fn base_color(bg: &Background) -> Rgba {
    match bg {
        Background::Solid(c) => *c,
        Background::Gradient { stops, .. } => stops.first().map_or(Rgba::BLACK, |s| s.1),
        Background::Particles { base, .. } => *base,
        Background::Image(_) | Background::Video { .. } => Rgba::BLACK,
    }
}

fn background_of_kind(kind: BgKind, current: &Background) -> Background {
    let base = base_color(current);
    let path = match current {
        Background::Image(p) | Background::Video { path: p } => p.clone(),
        _ => PathBuf::new(),
    };
    match kind {
        BgKind::Solid => Background::Solid(base),
        BgKind::Gradient => Background::Gradient {
            stops: vec![(0.0, base), (1.0, Rgba::new(0x0B, 0x1F, 0x3A, 255))],
            angle: 45.0,
            drift: 0.0,
        },
        BgKind::Particles => Background::Particles {
            base,
            color: Rgba::WHITE,
            count: 120,
            speed: 1.0,
        },
        BgKind::Image => Background::Image(path),
        BgKind::Video => Background::Video { path },
    }
}

pub fn bg_kind(bg: &Background) -> BgKind {
    match bg {
        Background::Solid(_) => BgKind::Solid,
        Background::Gradient { .. } => BgKind::Gradient,
        Background::Particles { .. } => BgKind::Particles,
        Background::Image(_) => BgKind::Image,
        Background::Video { .. } => BgKind::Video,
    }
}

/// Flip Inherit on a property. The Set value goes into `stash` and comes
/// back on the next flip. `fallback` is used only when nothing was stashed.
fn toggle<T: Clone>(p: &mut Prop<T>, stash: &mut Option<T>, inherit: bool, fallback: T) {
    if inherit {
        if let Prop::Set(v) = std::mem::replace(p, Prop::Inherit) {
            *stash = Some(v);
        }
    } else if matches!(p, Prop::Inherit) {
        *p = Prop::Set(stash.take().unwrap_or(fallback));
    }
}

/// Apply one form edit to the draft.
pub fn apply(st: &mut ThemesState, e: DraftEdit) {
    let d = &mut st.draft;
    let t = &mut d.text;
    let stash = &mut st.stash;
    match e {
        DraftEdit::Name(n) => d.name = n,
        DraftEdit::FontInherit(i) => toggle(
            &mut t.font,
            &mut stash.font,
            i,
            Font::Bundled(BundledFont::Inter),
        ),
        DraftEdit::Font(f) => t.font = Prop::Set(f),
        DraftEdit::SizeInherit(i) => toggle(&mut t.size, &mut stash.size, i, TextSize::Auto),
        DraftEdit::SizeAuto(auto) => {
            t.size = Prop::Set(if auto {
                TextSize::Auto
            } else {
                TextSize::Pt(48.0)
            })
        }
        DraftEdit::SizePt(pt) => t.size = Prop::Set(TextSize::Pt(pt)),
        DraftEdit::BoldInherit(i) => toggle(&mut t.bold, &mut stash.bold, i, false),
        DraftEdit::Bold(b) => t.bold = Prop::Set(b),
        DraftEdit::ColorInherit(i) => {
            toggle(&mut t.color, &mut stash.color, i, Rgba::WHITE);
            st.hex_edit = None;
        }
        DraftEdit::Color(c) => t.color = Prop::Set(c),
        DraftEdit::HAlignInherit(i) => {
            toggle(&mut t.h_align, &mut stash.h_align, i, HAlign::Center)
        }
        DraftEdit::HAlign(a) => t.h_align = Prop::Set(a),
        DraftEdit::VAlign(a) => t.v_align = a,
        DraftEdit::SpacingInherit(i) => {
            toggle(&mut t.line_spacing, &mut stash.line_spacing, i, 1.1)
        }
        DraftEdit::Spacing(v) => t.line_spacing = Prop::Set(v),
        DraftEdit::MarginInherit(i) => toggle(&mut t.margin, &mut stash.margin, i, 0.06),
        DraftEdit::Margin(v) => t.margin = Prop::Set(v),
        DraftEdit::Opacity(v) => t.opacity = v,
        DraftEdit::ShadowOn(on) => {
            t.shadow = on.then_some(Shadow {
                color: Rgba::BLACK,
                offset: (2.0, 2.0),
                blur: 4.0,
            });
            st.hex_edit = None;
        }
        DraftEdit::ShadowColor(c) => {
            if let Some(sh) = &mut t.shadow {
                sh.color = c;
            }
        }
        DraftEdit::ShadowOffset(o) => {
            if let Some(sh) = &mut t.shadow {
                sh.offset = (o, o);
            }
        }
        DraftEdit::OutlineOn(on) => {
            t.outline = on.then_some(Outline {
                color: Rgba::BLACK,
                width: 2.0,
            });
            st.hex_edit = None;
        }
        DraftEdit::OutlineColor(c) => {
            if let Some(o) = &mut t.outline {
                o.color = c;
            }
        }
        DraftEdit::OutlineWidth(w) => {
            if let Some(o) = &mut t.outline {
                o.width = w;
            }
        }
        DraftEdit::BgKind(k) => {
            if bg_kind(&d.background) != k {
                d.background = background_of_kind(k, &d.background);
                st.hex_edit = None;
            }
        }
        DraftEdit::BgSolid(c) => match &mut d.background {
            Background::Solid(s) => *s = c,
            Background::Particles { base, .. } => *base = c,
            _ => {}
        },
        DraftEdit::BgStop(i, c) => {
            if let Background::Gradient { stops, .. } = &mut d.background
                && let Some(stop) = stops.get_mut(i)
            {
                stop.1 = c;
            }
        }
        DraftEdit::BgAngle(a) => {
            if let Background::Gradient { angle, .. } = &mut d.background {
                *angle = a;
            }
        }
        DraftEdit::BgDrift(v) => {
            if let Background::Gradient { drift, .. } = &mut d.background {
                *drift = v;
            }
        }
        DraftEdit::BgParticleColor(c) => {
            if let Background::Particles { color, .. } = &mut d.background {
                *color = c;
            }
        }
        DraftEdit::BgParticleCount(n) => {
            if let Background::Particles { count, .. } = &mut d.background {
                *count = n;
            }
        }
        DraftEdit::BgParticleSpeed(v) => {
            if let Background::Particles { speed, .. } = &mut d.background {
                *speed = v;
            }
        }
        DraftEdit::BgOpacity(v) => d.background_opacity = v,
        DraftEdit::TransitionKind(k) => d.transition.kind = k,
        DraftEdit::Duration(ms) => d.transition.duration_ms = ms,
        DraftEdit::Easing(e) => d.transition.easing = e,
        DraftEdit::PerSlide(n) => d.verses.per_slide = n.max(1),
        DraftEdit::ShowReference(b) => d.verses.show_reference = b,
        DraftEdit::ShowNumbers(b) => d.verses.show_numbers = b,
        DraftEdit::RefPos(p) => d.verses.reference_at = p,
        DraftEdit::Footer(b) => d.footer = b,
    }
}

/// What the preview plays. An image or video background with no file yet
/// shows black instead of a failed load.
fn preview_theme(draft: &Theme) -> Theme {
    let mut t = draft.clone();
    let empty = match &t.background {
        Background::Image(p) | Background::Video { path: p } => p.as_os_str().is_empty(),
        _ => false,
    };
    if empty {
        t.background = Background::Solid(Rgba::BLACK);
    }
    t
}

/// Keep the preview player and the thumbnail cache in step with the draft
/// and the loaded themes. Returns the background worker when the preview
/// needs a file loaded.
fn sync_preview(app: &mut App) -> Task<Msg> {
    let s = &app.core.strings;
    let t = &mut app.themes;
    let shown = preview_theme(&t.draft);
    if t.preview.theme != shown {
        t.preview.set_theme(shown);
    }
    t.preview.adopt_video(&app.live);
    let keys: Vec<u64> = app.core.themes.iter().map(Theme::cache_key).collect();
    t.thumbs.retain(|k, _| keys.contains(k));
    for theme in &app.core.themes {
        let key = theme.cache_key();
        if !t.thumbs.contains_key(&key)
            && let Some(slide) = thumb_slide(theme, s)
        {
            t.thumbs.insert(key, Arc::new(slide));
        }
    }
    workers::background(BgTarget::ThemePreview, &t.preview)
}

/// Make `theme` the live one and start its background worker if needed.
fn apply_live(app: &mut App, theme: Theme) -> Task<Msg> {
    app.core.set_active_theme(theme.clone());
    app.live.set_theme(theme);
    app.live.adopt_video(&app.themes.preview);
    app.status.notice = None;
    workers::background(BgTarget::Live, &app.live)
}

fn tick_later() -> Task<Msg> {
    // The sleep is built inside the future so no runtime is needed until
    // Iced polls it.
    Task::perform(async { tokio::time::sleep(TICK).await }, |_| {
        Msg::Themes(ThemesMsg::Tick)
    })
}

/// The Themes tab was shown. Starts the preview loop.
pub fn opened(app: &mut App) -> Task<Msg> {
    ensure_ticking(app)
}

/// Start the preview loop if the tab is open and it is not running.
fn ensure_ticking(app: &mut App) -> Task<Msg> {
    if app.tab != Tab::Themes || app.themes.ticking {
        return Task::none();
    }
    app.themes.ticking = true;
    app.themes.preview_started = Instant::now();
    tick_later()
}

fn tick(app: &mut App) -> Task<Msg> {
    let t = &mut app.themes;
    if app.tab != Tab::Themes {
        t.ticking = false;
        return Task::none();
    }
    if t.preview_started.elapsed() >= PREVIEW_PERIOD {
        t.preview_started = Instant::now();
        t.preview_flip = !t.preview_flip;
        if t.preview_flip {
            t.preview.next();
        } else {
            t.preview.prev();
        }
    }
    t.ticking = true;
    tick_later()
}

/// Reread themes from disk and load the one called `name` into the form,
/// or the first one if it is gone.
fn reload_and_select(app: &mut App, name: &str) {
    app.core.reload_themes();
    let sorted = sorted_themes(&app.core);
    let i = sorted.iter().position(|t| t.name == name).unwrap_or(0);
    let theme = sorted.get(i).cloned().cloned().unwrap_or_default();
    app.themes.draft = theme;
    app.themes.draft_index = i;
    app.themes.hex_edit = None;
    app.themes.stash = InheritStash::default();
}

/// Write `theme` to disk and select it.
fn store_and_select(app: &mut App, theme: Theme) {
    match store::save_theme(&app.core.paths.themes_dir, &theme) {
        Ok(()) => {
            app.themes.error = None;
            reload_and_select(app, &theme.name);
        }
        Err(e) => app.themes.error = Some(e.to_string()),
    }
}

/// Same rule as `theme::store` uses for the file name, so two names that
/// map to one file are caught before the write.
fn file_stem(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// True when saving `draft` under its name would land on another theme's
/// file. `old` is the name the draft had when it was loaded.
fn name_collides(core: &Core, draft: &Theme, old: Option<&str>) -> bool {
    if old == Some(draft.name.as_str()) {
        return false;
    }
    let stem = file_stem(&draft.name);
    let taken_by_other = core
        .themes
        .iter()
        .filter(|t| Some(t.name.as_str()) != old)
        .any(|t| t.name == draft.name || file_stem(&t.name) == stem);
    let old_stem = old.map(file_stem);
    let on_disk = old_stem.as_deref() != Some(stem.as_str())
        && core.paths.themes_dir.join(format!("{stem}.json")).exists();
    taken_by_other || on_disk
}

/// Save the draft. A rename removes the old file. Saving the active theme
/// applies it to the projector.
fn save_draft(app: &mut App) -> Task<Msg> {
    let draft = app.themes.draft.clone();
    if draft.name.trim().is_empty() {
        app.themes.error = Some(app.core.strings.theme_name_label.clone());
        return Task::none();
    }
    let old = sorted_themes(&app.core)
        .get(app.themes.draft_index)
        .map(|t| t.name.clone());
    if name_collides(&app.core, &draft, old.as_deref()) {
        app.themes.error = Some(app.core.strings.theme_name_exists.clone());
        return Task::none();
    }
    let was_active = app.core.active_theme.name == draft.name
        || old.as_deref() == Some(app.core.active_theme.name.as_str());
    if let Err(e) = store::save_theme(&app.core.paths.themes_dir, &draft) {
        app.themes.error = Some(e.to_string());
        return Task::none();
    }
    if let Some(old) = old
        && old != draft.name
        && let Err(e) = store::delete_theme(&app.core.paths.themes_dir, &old)
    {
        log::warn!("removing renamed theme {old} failed: {e}");
    }
    let task = if was_active {
        apply_live(app, draft.clone())
    } else {
        Task::none()
    };
    app.themes.error = None;
    reload_and_select(app, &draft.name);
    task
}

fn pick_file(label: &str, exts: &[&str]) -> Task<Msg> {
    let label = label.to_string();
    let exts: Vec<String> = exts.iter().map(|e| e.to_string()).collect();
    Task::perform(
        async move {
            rfd::AsyncFileDialog::new()
                .add_filter(label, &exts)
                .pick_file()
                .await
                .map(|h| h.path().to_path_buf())
        },
        |p| Msg::Themes(ThemesMsg::FilePicked(p)),
    )
}

pub fn update(app: &mut App, msg: ThemesMsg) -> Task<Msg> {
    let task = match msg {
        ThemesMsg::Noop => Task::none(),
        ThemesMsg::Tick => return tick(app),
        ThemesMsg::Select(i) => {
            if let Some(t) = sorted_themes(&app.core).get(i).cloned().cloned() {
                app.themes.draft = t;
                app.themes.draft_index = i;
                app.themes.hex_edit = None;
                app.themes.stash = InheritStash::default();
                app.themes.error = None;
            }
            Task::none()
        }
        ThemesMsg::New => {
            let name = unique_name(&app.core, &app.core.strings.theme_new);
            store_and_select(
                app,
                Theme {
                    name,
                    ..Theme::default()
                },
            );
            Task::none()
        }
        ThemesMsg::Duplicate => {
            let mut t = app.themes.draft.clone();
            t.name = unique_name(&app.core, &t.name);
            t.favorite = false;
            store_and_select(app, t);
            Task::none()
        }
        ThemesMsg::Delete => {
            let name = app.themes.draft.name.clone();
            match store::delete_theme(&app.core.paths.themes_dir, &name) {
                Err(e) => {
                    app.themes.error = Some(e.to_string());
                    Task::none()
                }
                Ok(()) => {
                    let task = if app.core.active_theme.name == name {
                        apply_live(app, Theme::default())
                    } else {
                        Task::none()
                    };
                    app.themes.error = None;
                    reload_and_select(app, "Default");
                    task
                }
            }
        }
        ThemesMsg::Save => save_draft(app),
        ThemesMsg::SetActive(name) => match app.core.theme_named(&name).cloned() {
            Some(theme) => apply_live(app, theme),
            None => Task::none(),
        },
        ThemesMsg::Favorite(f) => {
            app.themes.draft.favorite = f;
            if app.themes.draft.is_builtin_default() {
                Task::none()
            } else {
                save_draft(app)
            }
        }
        ThemesMsg::Draft(e) => {
            apply(&mut app.themes, e);
            Task::none()
        }
        ThemesMsg::Hex(field, text) => {
            if let Some(c) = Rgba::from_hex(text.trim()) {
                apply(&mut app.themes, field.edit(c));
            }
            app.themes.hex_edit = Some((field, text));
            Task::none()
        }
        ThemesMsg::PickImage => {
            apply(&mut app.themes, DraftEdit::BgKind(BgKind::Image));
            pick_file(&app.core.strings.theme_bg_image, &["png", "jpg", "jpeg"])
        }
        ThemesMsg::PickVideo => {
            apply(&mut app.themes, DraftEdit::BgKind(BgKind::Video));
            pick_file(
                &app.core.strings.theme_bg_video,
                &["mp4", "mov", "mkv", "webm"],
            )
        }
        ThemesMsg::FilePicked(Some(p)) => {
            match &mut app.themes.draft.background {
                Background::Image(path) | Background::Video { path } => *path = p,
                _ => {}
            }
            Task::none()
        }
        ThemesMsg::FilePicked(None) => Task::none(),
    };
    let preview = sync_preview(app);
    Task::batch([task, preview, ensure_ticking(app)])
}
