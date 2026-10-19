# Bible selection, verse styling, Themes editor layout: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Click and drag verse selection, one Bible search bar with a text mode and a slide preview, verses packed into as few slides as fit with per slide references, an independent reference text style, a two column Themes editor whose preview shows verse edits, and a History card on the Present tab.

**Architecture:** Core gains `Theme.reference: TextStyle`, `VerseLayout.min_size`, a two box verse resolve, space separated reference parsing, and `pack_verses` with a heuristic `fits`. The app tracks keyboard modifiers, rewires the Bible tab around `Press`, `Enter`, `Release`, `Input`, and `TextMode` messages, adds a preview pane, splits the Themes form into two columns with a Hymn | Bible sample, and swaps the Notes card for History.

**Tech Stack:** Rust 1.88, Iced 0.14, iced_test, serde, cosmic-text through sda-render. Edition 2024 in `sda-app` and `sda-render`.

**Spec:** `docs/superpowers/specs/2026-10-19-bible-and-themes-ux-design.md`

## Global Constraints

- Every new UI string goes in `Strings` for En, It, and Ro in `crates/sda-core/src/i18n.rs`, and in the `as_fields` list, or the i18n test fails.
- Theme JSON written by 2.0.1 must still load. New fields take defaults.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass at the end of every task.
- `cargo test --workspace` must pass at the end of every task. Render goldens skip without a GPU.
- Run `cargo fmt --all` before every commit.
- No `unwrap` on user data in app code. Tests may unwrap.
- Keep the one dark palette in `ui_theme.rs`. Reuse `card`, `segmented`, `ghost_button`, `list_row`, `thumb_frame`.
- The verse packing heuristic is marked with a `ponytail:` comment naming the cosmic-text measure as the upgrade.

---

## File map

| File | Change |
|---|---|
| `crates/sda-core/src/theme.rs` | `Theme.reference`, `VerseLayout.min_size`, delete `RefPos`, `per_slide`, `reference_at`. Hashes. `ThemeV3` defaults. |
| `crates/sda-core/src/model.rs` | `Frame::cut`. |
| `crates/sda-core/src/resolve.rs` | Two box `resolve_verses`, `REF_BAND`, `verse_box_pt`, `resolve_reference_style`. |
| `crates/sda-core/src/bible/reference.rs` | Space separated chapter and verse. |
| `crates/sda-core/src/bible/mod.rs` | `pack_verses`, `fits_heuristic`, new `verse_slides`. |
| `crates/sda-core/src/i18n.rs` | Strings added and removed. |
| `crates/sda-core/tests/{theme,resolve,bible}_test.rs` | Tests. |
| `crates/sda-render/tests/golden_test.rs` | `verses_reference_band` golden. |
| `crates/sda-app/src/msg.rs` | `Msg::Modifiers`. |
| `crates/sda-app/src/app.rs` | `modifiers` field, `Msg::Modifiers` arm. |
| `crates/sda-app/src/shortcuts.rs` | Emit `Msg::Modifiers`. |
| `crates/sda-app/src/msg/bible.rs` | New message set. |
| `crates/sda-app/src/state/bible.rs` | New state and update. |
| `crates/sda-app/src/view/bible.rs` | Row selection, one bar, preview pane. |
| `crates/sda-app/src/msg/themes.rs` | `DraftEdit::Reference`, `MinSize`, `ColorField::Ref*`, `ThemesMsg::Sample`. |
| `crates/sda-app/src/state/themes.rs` | Reference stash, `Sample`, Bible sample, `apply` split. |
| `crates/sda-app/src/view/themes.rs` | Two columns, sample toggle, responsive preview. |
| `crates/sda-app/src/view/theme_form.rs` | `text_style_section`, Reference section, verses section. |
| `crates/sda-app/src/view/present.rs` | History card, Recent block removed. |
| `crates/sda-app/src/state/present.rs` | `RECENT_COUNT` removed, `select_history`. |
| `crates/sda-app/tests/{bible,themes,present}_test.rs` | Tests. |
| `CLAUDE.md`, `CHANGELOG.md` | Docs. |

---

### Task 1: Theme model, reference style and min size

**Files:**
- Modify: `crates/sda-core/src/theme.rs:380-405`, `407-448`, `474-493`
- Modify: `crates/sda-core/src/resolve.rs:148-194` to compile only, full change in Task 2
- Modify: `crates/sda-app/src/msg/themes.rs:3`, `81-85`
- Modify: `crates/sda-app/src/state/themes.rs:393-396`
- Modify: `crates/sda-app/src/view/theme_form.rs:18-21`, `668-704`
- Modify: `crates/sda-core/src/i18n.rs` strings `theme_per_slide`, `reference_position` become `theme_min_size`
- Modify: `crates/sda-core/tests/resolve_test.rs:121-137`
- Test: `crates/sda-core/tests/theme_test.rs`

**Interfaces:**
- Produces: `Theme.reference: TextStyle`, `VerseLayout { min_size: f32, show_reference: bool, show_numbers: bool }`, `VerseLayout::default().min_size == 28.0`, `Theme::default().reference.v_align == VAlign::Bottom`, `DraftEdit::MinSize(f32)`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/sda-core/tests/theme_test.rs`:

```rust
#[test]
fn reference_style_defaults_to_bottom_and_inherit() {
    let t = Theme::default();
    assert_eq!(t.reference.v_align, VAlign::Bottom);
    assert_eq!(t.reference.font, Prop::Inherit);
    assert_eq!(t.reference.size, Prop::Inherit);
    assert_eq!(t.verses.min_size, 28.0);
}

#[test]
fn v3_theme_without_reference_and_with_per_slide_loads() {
    let old = r##"{"name":"Three","text":{},"verses":{"per_slide":2,"show_reference":false,"reference_at":"Top"}}"##;
    let t = Theme::from_json(old).unwrap();
    assert_eq!(t.name, "Three");
    assert!(!t.verses.show_reference);
    assert_eq!(t.verses.min_size, 28.0);
    assert_eq!(t.reference, TextStyle { v_align: VAlign::Bottom, ..TextStyle::default() });
}

#[test]
fn cache_key_changes_with_reference_and_min_size() {
    let a = Theme::default();
    let mut b = a.clone();
    b.reference.color = Prop::Set(Rgba::new(1, 2, 3, 255));
    assert_ne!(a.cache_key(), b.cache_key());
    let mut c = a.clone();
    c.verses.min_size = 40.0;
    assert_ne!(a.cache_key(), c.cache_key());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sda-core --test theme_test`
Expected: compile errors on `reference` and `min_size`.

- [ ] **Step 3: Change the model**

In `crates/sda-core/src/theme.rs` replace lines 380 to 405 with:

```rust
/// How Bible verses are split and labeled. Verses pack into a slide while
/// they fit at `min_size` points, see `bible::pack_verses`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VerseLayout {
    pub min_size: f32,
    pub show_reference: bool,
    pub show_numbers: bool,
}

impl Default for VerseLayout {
    fn default() -> Self {
        VerseLayout {
            min_size: 28.0,
            show_reference: true,
            show_numbers: true,
        }
    }
}

impl Hash for VerseLayout {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.min_size.to_bits().hash(h);
        self.show_reference.hash(h);
        self.show_numbers.hash(h);
    }
}
```

`Eq` is dropped from `VerseLayout` because of the `f32`. Remove the `RefPos` enum entirely.

In `Theme` add `pub reference: TextStyle,` after `text`. In `Default for Theme` add `reference: reference_default(),`. In `Hash for Theme` add `self.reference.hash(h);` after `self.text.hash(h);`. Add:

```rust
/// The reference sits under the verses unless a theme moves it.
fn reference_default() -> TextStyle {
    TextStyle {
        v_align: VAlign::Bottom,
        ..TextStyle::default()
    }
}
```

In `ThemeV3` add `#[serde(default = "reference_default")] reference: TextStyle,` after `text`, and copy it in the `From<ThemeV2OrV3>` conversion that builds `Theme` from `ThemeV3`. Open the `impl From<ThemeV2OrV3> for Theme` block below line 530 and set `reference: v.reference` for V3 and `reference: reference_default()` for V2.

- [ ] **Step 4: Fix compile sites**

`crates/sda-core/src/resolve.rs`: remove `RefPos` from the import and replace the `match theme.verses.reference_at` block in `resolve_verses` with `lines.push(r);`. Task 2 rewrites this function.

`crates/sda-core/tests/resolve_test.rs`: in `verses_reference_at_top_with_small_fixed_size_and_no_numbers` delete the line `t.verses.reference_at = RefPos::Top;`, rename the test to `verses_reference_small_fixed_size_and_no_numbers`, and swap the two assertions so the verse line is `lines[0]` and the reference is `lines[1]`. Task 2 rewrites this test again.

`crates/sda-app/src/msg/themes.rs`: remove `RefPos` from the import. Replace `PerSlide(u8),` with `MinSize(f32),` and delete `RefPos(RefPos),`.

`crates/sda-app/src/state/themes.rs` in `apply`: replace `DraftEdit::PerSlide(n) => d.verses.per_slide = n.max(1),` with `DraftEdit::MinSize(v) => d.verses.min_size = v.clamp(16.0, 60.0),` and delete the `RefPos` arm.

`crates/sda-core/src/i18n.rs`: rename field `theme_per_slide` to `theme_min_size` in the struct, in `as_fields`, and in the three language blocks. Values: En `"Smallest text size"`, It `"Dimensione minima del testo"`, Ro `"Dimensiunea minimă a textului"`. Delete `reference_position` from the struct, from `as_fields`, and from the three blocks.

`crates/sda-app/src/view/theme_form.rs`: remove `RefPos` from the import. Replace `verses_section` with:

```rust
fn verses_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let v = d.verses;
    section(
        &s.theme_verses,
        vec![
            field(
                &s.theme_min_size,
                slide(
                    16.0..=60.0,
                    1.0,
                    v.min_size,
                    format!("{:.0} pt", v.min_size),
                    DraftEdit::MinSize,
                ),
            ),
            check(
                &s.theme_show_reference,
                v.show_reference,
                DraftEdit::ShowReference,
            ),
            check(
                &s.theme_show_numbers,
                v.show_numbers,
                DraftEdit::ShowNumbers,
            ),
        ],
    )
}
```

`crates/sda-app/src/state/bible.rs:276-280`: replace `let per_slide = app.live.theme.verses.per_slide;` and the `verse_slides(&verses, &reference, per_slide)` call with `verse_slides(&verses, &reference, 1)` for now. Task 4 replaces this.

`crates/sda-app/tests/bible_test.rs:45-59`: in `present_builds_verse_slides` replace the two `per_slide` lines with `assert_eq!(app.live.present.slides.len(), 3);`. Task 4 changes it again.

- [ ] **Step 5: Run the workspace tests**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS. The three new theme tests pass.

- [ ] **Step 6: Commit**

```bash
cargo fmt --all
git add -A crates
git commit -m "feat(core): reference text style and min verse size on themes"
```

---

### Task 2: Two box verse resolve

**Files:**
- Modify: `crates/sda-core/src/model.rs:40-45`
- Modify: `crates/sda-core/src/resolve.rs:59-62`, `148-194`
- Test: `crates/sda-core/tests/resolve_test.rs`

**Interfaces:**
- Produces: `pub const REF_BAND: f32 = 0.12;`, `Frame::cut(self, side: VAlign, band: f32) -> (Frame, Frame)` returning `(strip, rest)`, `pub fn verse_frame(theme: &Theme) -> Frame`, `pub fn verse_box_pt(theme: &Theme) -> (f32, f32)`, `pub fn reference_style(theme: &Theme) -> TextStyle` with every `Prop` resolved to `Set`.

- [ ] **Step 1: Write the failing tests**

Replace `verses_make_one_box_plus_reference_line`, `verses_reference_small_fixed_size_and_no_numbers`, and `verses_without_reference_have_only_verse_lines` in `crates/sda-core/tests/resolve_test.rs` with:

```rust
fn verses_content() -> SlideContent {
    SlideContent::Verses {
        reference: "John 3:16".into(),
        verses: vec![(16, "For God".into())],
    }
}

#[test]
fn verses_make_a_verse_box_and_a_reference_box() {
    let r = resolve(&Theme::default(), &verses_content(), None).unwrap();
    assert_eq!(r.boxes.len(), 2);
    assert!(r.boxes[0].lines[0].runs[0].text.starts_with("16 "));
    assert_eq!(r.boxes[1].lines[0].runs[0].text, "John 3:16");
    assert_eq!(r.boxes[1].v_align, VAlign::Bottom);
}

#[test]
fn reference_band_does_not_overlap_the_verses() {
    for side in [VAlign::Top, VAlign::Bottom] {
        let mut t = Theme::default();
        t.reference.v_align = side;
        let r = resolve(&t, &verses_content(), None).unwrap();
        let v = r.boxes[0].frame;
        let rf = r.boxes[1].frame;
        assert!((rf.h - REF_BAND).abs() < 1e-5, "{side:?} band height");
        let v_bottom = v.y + v.h;
        let rf_bottom = rf.y + rf.h;
        assert!(
            v_bottom <= rf.y + 1e-5 || rf_bottom <= v.y + 1e-5,
            "{side:?} frames overlap: verses {v:?} reference {rf:?}"
        );
    }
}

#[test]
fn reference_middle_keeps_the_whole_frame() {
    let mut t = Theme::default();
    t.reference.v_align = VAlign::Middle;
    let r = resolve(&t, &verses_content(), None).unwrap();
    assert_eq!(r.boxes[0].frame, r.boxes[1].frame);
}

#[test]
fn reference_inherits_from_verse_text_at_sixty_percent() {
    let mut t = Theme::default();
    t.text.size = Prop::Set(TextSize::Pt(50.0));
    t.text.color = Prop::Set(Rgba::new(1, 2, 3, 255));
    t.text.font = Prop::Set(Font::Bundled(BundledFont::Lora));
    t.verses.show_numbers = false;
    let r = resolve(&t, &verses_content(), None).unwrap();
    let verse = &r.boxes[0].lines[0].runs[0];
    let reference = &r.boxes[1].lines[0].runs[0];
    assert_eq!(verse.text, "For God");
    assert_eq!(verse.size, TextSize::Pt(50.0));
    assert!(matches!(reference.size, TextSize::Pt(p) if (p - 30.0).abs() < 1e-4));
    assert_eq!(reference.color, Rgba::new(1, 2, 3, 255));
    assert_eq!(reference.font, Font::Bundled(BundledFont::Lora));
}

#[test]
fn reference_set_values_win_over_verse_text() {
    let mut t = Theme::default();
    t.text.color = Prop::Set(Rgba::new(1, 2, 3, 255));
    t.reference.color = Prop::Set(Rgba::new(9, 9, 9, 255));
    t.reference.h_align = Prop::Set(HAlign::Right);
    let r = resolve(&t, &verses_content(), None).unwrap();
    let line = &r.boxes[1].lines[0];
    assert_eq!(line.runs[0].color, Rgba::new(9, 9, 9, 255));
    assert_eq!(line.align, HAlign::Right);
}

#[test]
fn verses_without_reference_have_one_box() {
    let mut t = Theme::default();
    t.verses.show_reference = false;
    let c = SlideContent::Verses {
        reference: "John 3:16".into(),
        verses: vec![(16, "a".into()), (17, "b".into())],
    };
    let r = resolve(&t, &c, None).unwrap();
    assert_eq!(r.boxes.len(), 1);
    assert_eq!(r.boxes[0].lines.len(), 2);
    assert_eq!(r.boxes[0].lines[1].runs[0].text, "17 b");
    let full = Frame { x: 0.06, y: 0.06, w: 0.88, h: 0.88 };
    assert_eq!(r.boxes[0].frame, full);
}

#[test]
fn frame_cut_splits_a_band_off_one_side() {
    let f = Frame { x: 0.1, y: 0.1, w: 0.8, h: 0.8 };
    let (strip, rest) = f.cut(VAlign::Top, 0.2);
    assert_eq!(strip, Frame { x: 0.1, y: 0.1, w: 0.8, h: 0.2 });
    assert!((rest.y - 0.3).abs() < 1e-6 && (rest.h - 0.6).abs() < 1e-6);
    let (strip, rest) = f.cut(VAlign::Bottom, 0.2);
    assert!((strip.y - 0.7).abs() < 1e-6 && (strip.h - 0.2).abs() < 1e-6);
    assert_eq!(rest, Frame { x: 0.1, y: 0.1, w: 0.8, h: 0.6 });
    let (strip, rest) = f.cut(VAlign::Middle, 0.2);
    assert_eq!(strip, f);
    assert_eq!(rest, f);
}

#[test]
fn verse_box_pt_is_the_verse_frame_on_a_720_slide() {
    let t = Theme::default();
    let (w, h) = verse_box_pt(&t);
    assert!((w - 0.88 * 720.0).abs() < 1e-3);
    assert!((h - (0.88 - REF_BAND) * 405.0).abs() < 1e-3);
}
```

Add `use sda_core::model::Frame;` only if `model::*` is not already imported. It is, so skip it.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sda-core --test resolve_test`
Expected: compile errors on `REF_BAND`, `cut`, `verse_box_pt`.

- [ ] **Step 3: Add `Frame::cut`**

In `crates/sda-core/src/model.rs` after the `Frame` struct add:

```rust
impl Frame {
    /// Split a horizontal band of height `band` off `side`. Returns the
    /// band and what is left. `Middle` returns the whole frame twice, so a
    /// centered reference overlays the verses.
    pub fn cut(self, side: crate::theme::VAlign, band: f32) -> (Frame, Frame) {
        use crate::theme::VAlign;
        let band = band.min(self.h);
        match side {
            VAlign::Top => (
                Frame { h: band, ..self },
                Frame {
                    y: self.y + band,
                    h: self.h - band,
                    ..self
                },
            ),
            VAlign::Bottom => (
                Frame {
                    y: self.y + self.h - band,
                    h: band,
                    ..self
                },
                Frame {
                    h: self.h - band,
                    ..self
                },
            ),
            VAlign::Middle => (self, self),
        }
    }
}
```

`Frame` must derive `PartialEq` and `Copy`. Check line 38 and add them if missing.

- [ ] **Step 4: Rewrite `resolve_verses`**

In `crates/sda-core/src/resolve.rs` replace `const REFERENCE_SCALE` with:

```rust
pub const REFERENCE_SCALE: f32 = 0.6;
/// Slide height fraction the reference strip takes above or below the verses.
pub const REF_BAND: f32 = 0.12;
/// A 16 by 9 slide measured in points, the unit `TextSize::Pt` uses.
pub const SLIDE_PT: (f32, f32) = (720.0, 405.0);
```

Replace `resolve_verses` with:

```rust
/// The reference style with every `Prop` resolved. `Inherit` takes the
/// verse text value, size at `REFERENCE_SCALE` of the verse size.
pub fn reference_style(theme: &Theme) -> TextStyle {
    let t = &theme.text;
    let r = &theme.reference;
    let verse_size = t.size.resolve(None, TextSize::Auto);
    let small = match verse_size {
        TextSize::Pt(p) => TextSize::Pt(p * REFERENCE_SCALE),
        TextSize::Auto => TextSize::Auto,
    };
    TextStyle {
        font: Prop::Set(r.font.resolve(
            Some(t.font.resolve(None, Font::Bundled(BundledFont::Inter))),
            Font::Bundled(BundledFont::Inter),
        )),
        size: Prop::Set(r.size.resolve(Some(small), TextSize::Auto)),
        bold: Prop::Set(r.bold.resolve(Some(t.bold.resolve(None, false)), false)),
        color: Prop::Set(r.color.resolve(
            Some(t.color.resolve(None, FALLBACK_COLOR)),
            FALLBACK_COLOR,
        )),
        h_align: Prop::Set(r.h_align.resolve(
            Some(t.h_align.resolve(None, HAlign::Center)),
            HAlign::Center,
        )),
        v_align: r.v_align,
        line_spacing: Prop::Set(r.line_spacing.resolve(
            Some(t.line_spacing.resolve(None, FALLBACK_SPACING)),
            FALLBACK_SPACING,
        )),
        margin: Prop::Set(r.margin.resolve(
            Some(t.margin.resolve(None, FALLBACK_MARGIN)),
            FALLBACK_MARGIN,
        )),
        opacity: r.opacity,
        shadow: r.shadow,
        outline: r.outline,
    }
}

/// The frame the verses fill after the reference band is taken off.
pub fn verse_frame(theme: &Theme) -> Frame {
    let full = margin_frame(&theme.text);
    if !theme.verses.show_reference {
        return full;
    }
    full.cut(theme.reference.v_align, REF_BAND).1
}

/// `verse_frame` in points on a 720 by 405 slide, for `bible::fits_heuristic`.
pub fn verse_box_pt(theme: &Theme) -> (f32, f32) {
    let f = verse_frame(theme);
    (f.w * SLIDE_PT.0, f.h * SLIDE_PT.1)
}

fn plain_run(text: &TextStyle, s: String) -> ResolvedRun {
    ResolvedRun {
        text: s,
        font: text.font.resolve(None, Font::Bundled(BundledFont::Inter)),
        size: text.size.resolve(None, TextSize::Auto),
        bold: text.bold.resolve(None, false),
        italic: false,
        color: text.color.resolve(None, FALLBACK_COLOR),
        opacity: text.opacity,
        shadow: text.shadow,
        outline: text.outline,
    }
}

fn plain_line(text: &TextStyle, r: ResolvedRun) -> ResolvedLine {
    ResolvedLine {
        align: text.h_align.resolve(None, HAlign::Center),
        spacing: text.line_spacing.resolve(None, FALLBACK_SPACING),
        runs: vec![r],
    }
}

fn resolve_verses(theme: &Theme, reference: &str, verses: &[(u32, String)]) -> Vec<ResolvedBox> {
    let text = &theme.text;
    let lines = verses
        .iter()
        .map(|(n, v)| {
            let s = if theme.verses.show_numbers {
                format!("{n} {v}")
            } else {
                v.clone()
            };
            plain_line(text, plain_run(text, s))
        })
        .collect();
    let mut boxes = vec![ResolvedBox {
        frame: verse_frame(theme),
        v_align: text.v_align,
        lines,
    }];
    if theme.verses.show_reference {
        let style = reference_style(theme);
        let frame = margin_frame(&style).cut(style.v_align, REF_BAND).0;
        boxes.push(ResolvedBox {
            frame,
            v_align: style.v_align,
            lines: vec![plain_line(&style, plain_run(&style, reference.to_string()))],
        });
    }
    boxes
}
```

In `resolve` change the `Verses` arm to `boxes: resolve_verses(theme, reference, verses),`.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p sda-core --test resolve_test`
Expected: PASS, nine resolve tests.

- [ ] **Step 6: Run the workspace and clippy, then commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

```bash
cargo fmt --all
git add -A crates
git commit -m "feat(core): verse slides resolve to a verse box and a styled reference box"
```

---

### Task 3: Space separated references

**Files:**
- Modify: `crates/sda-core/src/bible/reference.rs:122-140`
- Test: `crates/sda-core/tests/bible_test.rs`

**Interfaces:**
- Produces: `parse_reference("mat 10 1", Language::En) == Some(Reference { book: 39, chapter: 10, verses: Some((1, 1)) })`.

- [ ] **Step 1: Write the failing test**

Append to `crates/sda-core/tests/bible_test.rs`:

```rust
#[test]
fn parses_space_separated_chapter_and_verses() {
    let r = |s: &str| parse_reference(s, Language::En);
    let mat = |chapter, verses| Some(Reference { book: 39, chapter, verses });
    assert_eq!(r("mat 10"), mat(10, None));
    assert_eq!(r("mat 10 1"), mat(10, Some((1, 1))));
    assert_eq!(r("matthew 10 1-5"), mat(10, Some((1, 5))));
    assert_eq!(r("mat 10:1"), mat(10, Some((1, 1))));
    assert_eq!(r("mat 10 1 5"), None);
    assert_eq!(
        r("1 sam 2 1"),
        Some(Reference { book: 8, chapter: 2, verses: Some((1, 1)) })
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sda-core --test bible_test parses_space_separated`
Expected: FAIL. `mat 10 1` parses as chapter 101 today.

- [ ] **Step 3: Accept whitespace as the separator**

Replace `parse_chapter_verses` in `crates/sda-core/src/bible/reference.rs` with:

```rust
/// Parse `3`, `3:16`, `3,16`, `3 16`, `3:16-18`, `3 16-18`, or `3:16–18`.
/// More than one space separated group after the chapter is an error.
fn parse_chapter_verses(rest: &str) -> Option<(u32, Option<(u32, u32)>)> {
    let compact: String = rest.chars().filter(|c| !c.is_whitespace()).collect();
    let (chapter_s, verse_s): (&str, Option<&str>) = match compact.split_once([':', ',']) {
        Some((c, v)) => (c, Some(v)),
        None => {
            let words: Vec<&str> = rest.split_whitespace().collect();
            match words.as_slice() {
                [c] => (c, None),
                [c, v] => (c, Some(v)),
                _ => return None,
            }
        }
    };
    let chapter: u32 = chapter_s.parse().ok().filter(|c| *c > 0)?;
    let verses = match verse_s {
        None | Some("") => None,
        Some(v) => {
            let (a, b) = v.split_once(['-', '–']).unwrap_or((v, v));
            let a: u32 = a.parse().ok().filter(|n| *n > 0)?;
            let b: u32 = b.parse().ok().filter(|n| *n > 0)?;
            Some((a.min(b), a.max(b)))
        }
    };
    Some((chapter, verses))
}
```

`compact` is still needed for the colon branch because `3 : 16` must work. The borrow of `compact` in the first arm and of `rest` in the second both live long enough since both are locals of the function.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sda-core --test bible_test`
Expected: PASS including `parses_references_in_three_languages`.

- [ ] **Step 5: Commit**

```bash
cargo fmt --all
git add crates/sda-core
git commit -m "feat(core): references accept a space between chapter and verse"
```

---

### Task 4: Verse packing

**Files:**
- Modify: `crates/sda-core/src/bible/mod.rs:217-227`
- Modify: `crates/sda-app/src/state/bible.rs:261-284`
- Modify: `crates/sda-app/tests/bible_test.rs:44-59`
- Test: `crates/sda-core/tests/bible_test.rs:76-93`

**Interfaces:**
- Produces:
  - `pub fn pack_verses(verses: &[(u32, String)], fits: impl Fn(&[(u32, String)]) -> bool) -> Vec<Vec<(u32, String)>>`
  - `pub fn fits_heuristic(theme: &Theme, box_pt: (f32, f32)) -> impl Fn(&[(u32, String)]) -> bool`
  - `pub fn verse_slides(r: &Reference, groups: &[Vec<(u32, String)>], lang: Language) -> Vec<SlideContent>`

- [ ] **Step 1: Write the failing tests**

Replace `verse_slides_groups_per_slide` in `crates/sda-core/tests/bible_test.rs` with:

```rust
fn vs(items: &[(u32, &str)]) -> Vec<(u32, String)> {
    items.iter().map(|(n, s)| (*n, s.to_string())).collect()
}

#[test]
fn pack_verses_is_greedy_and_never_drops_a_verse() {
    let v = vs(&[(1, "a"), (2, "b"), (3, "c"), (4, "d"), (5, "e")]);
    let two_at_most = |g: &[(u32, String)]| g.len() <= 2;
    let packed = pack_verses(&v, two_at_most);
    assert_eq!(packed.len(), 3);
    assert_eq!(packed[0].len(), 2);
    assert_eq!(packed[2][0].0, 5);
    let nothing_fits = |_: &[(u32, String)]| false;
    let packed = pack_verses(&v, nothing_fits);
    assert_eq!(packed.len(), 5, "a verse that never fits still gets a slide");
    let all_fit = |_: &[(u32, String)]| true;
    assert_eq!(pack_verses(&v, all_fit).len(), 1);
    assert!(pack_verses(&[], all_fit).is_empty());
}

#[test]
fn fits_heuristic_tiny_box_one_per_slide_huge_box_all_together() {
    let t = Theme::default();
    let v = vs(&[
        (1, "In the beginning God created the heaven and the earth."),
        (2, "And the earth was without form, and void; and darkness was upon the face of the deep."),
        (3, "And God said, Let there be light: and there was light."),
    ]);
    let tiny = pack_verses(&v, fits_heuristic(&t, (100.0, 30.0)));
    assert_eq!(tiny.len(), 3);
    let huge = pack_verses(&v, fits_heuristic(&t, (5000.0, 5000.0)));
    assert_eq!(huge.len(), 1);
    let real = pack_verses(&v, fits_heuristic(&t, sda_core::resolve::verse_box_pt(&t)));
    assert!(real.len() >= 1 && real.len() <= 3);
}

#[test]
fn verse_slides_carry_their_own_reference() {
    let r = Reference { book: 42, chapter: 3, verses: Some((16, 21)) };
    let groups = vec![
        vs(&[(16, "a"), (17, "b"), (18, "c")]),
        vs(&[(19, "d"), (20, "e"), (21, "f")]),
        vs(&[(22, "g")]),
    ];
    let s = verse_slides(&r, &groups, Language::En);
    assert_eq!(s.len(), 3);
    let refs: Vec<&str> = s
        .iter()
        .map(|c| match c {
            sda_core::resolve::SlideContent::Verses { reference, .. } => reference.as_str(),
            _ => panic!(),
        })
        .collect();
    assert_eq!(refs, ["John 3:16-18", "John 3:19-21", "John 3:22"]);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sda-core --test bible_test`
Expected: compile errors on `pack_verses`, `fits_heuristic`, new `verse_slides` signature.

- [ ] **Step 3: Implement packing**

Replace `verse_slides` in `crates/sda-core/src/bible/mod.rs` with:

```rust
/// Greedy grouping. Adds verses to the current slide while `fits` holds,
/// then starts a new slide. A verse that does not fit alone still takes one
/// slide and shrinks at render time.
pub fn pack_verses(
    verses: &[(u32, String)],
    fits: impl Fn(&[(u32, String)]) -> bool,
) -> Vec<Vec<(u32, String)>> {
    let mut groups: Vec<Vec<(u32, String)>> = Vec::new();
    let mut current: Vec<(u32, String)> = Vec::new();
    for v in verses {
        current.push(v.clone());
        if current.len() > 1 && !fits(&current) {
            let last = current.pop().expect("just pushed");
            groups.push(std::mem::take(&mut current));
            current.push(last);
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }
    groups
}

/// Character width as a fraction of the font size. Latin text averages
/// about half the em.
const CHAR_EM: f32 = 0.5;
/// Characters a verse number and its space take in front of the text.
const NUMBER_CHARS: usize = 3;

/// Estimates whether a group of verses fits a box of `box_pt` points at the
/// theme's smallest verse size. `box_pt` comes from `resolve::verse_box_pt`.
// ponytail: character count heuristic. Upgrade is a cosmic-text measure
// from sda-render passed as the closure at the call site.
pub fn fits_heuristic(theme: &Theme, box_pt: (f32, f32)) -> impl Fn(&[(u32, String)]) -> bool {
    let size = theme.verses.min_size.max(1.0);
    let spacing = theme.text.line_spacing.resolve(None, FALLBACK_SPACING);
    let numbers = theme.verses.show_numbers;
    let chars_per_line = ((box_pt.0 / (CHAR_EM * size)).floor() as usize).max(1);
    let line_h = size * spacing;
    move |group: &[(u32, String)]| {
        let lines: usize = group
            .iter()
            .map(|(_, v)| {
                let chars = v.chars().count() + if numbers { NUMBER_CHARS } else { 0 };
                chars.div_ceil(chars_per_line).max(1)
            })
            .sum();
        lines as f32 * line_h <= box_pt.1
    }
}

/// One `SlideContent::Verses` per group. Each slide's reference is its own
/// range, so slide two of John 3:16-21 reads `John 3:19-21`.
pub fn verse_slides(r: &Reference, groups: &[Vec<(u32, String)>], lang: Language) -> Vec<SlideContent> {
    groups
        .iter()
        .filter(|g| !g.is_empty())
        .map(|g| {
            let first = g.first().map(|v| v.0).unwrap_or(1);
            let last = g.last().map(|v| v.0).unwrap_or(first);
            let own = Reference {
                book: r.book,
                chapter: r.chapter,
                verses: Some((first, last)),
            };
            SlideContent::Verses {
                reference: format_reference(&own, lang),
                verses: g.clone(),
            }
        })
        .collect()
}
```

Add to the imports at the top of `bible/mod.rs`: `use crate::i18n::Language;`, `use crate::resolve::FALLBACK_SPACING;`, `use crate::theme::Theme;`.

- [ ] **Step 4: Run the core tests**

Run: `cargo test -p sda-core --test bible_test`
Expected: PASS.

- [ ] **Step 5: Use it in the app**

In `crates/sda-app/src/state/bible.rs` replace `present` with:

```rust
fn present(app: &mut App) {
    let Some(b) = app.bible.bible.clone() else {
        return;
    };
    let lang = app.core.language;
    let r = app.bible.reference();
    let verses: Vec<(u32, String)> = bible::verses(&b, &r)
        .into_iter()
        .filter(|(n, _)| app.bible.selected.is_empty() || app.bible.selected.contains(n))
        .collect();
    if verses.is_empty() {
        return;
    }
    let reference = format_reference(&r, lang);
    let version = app.bible.version.clone().unwrap_or_else(|| b.code.clone());
    let theme = &app.live.theme;
    let groups = pack_verses(&verses, fits_heuristic(theme, verse_box_pt(theme)));
    app.live.go(Presentation {
        title: reference.clone(),
        subtitle: Some(app.bible.abbreviation(&version)),
        slides: verse_slides(&r, &groups, lang),
        source: Source::Bible { version, reference },
        ..Presentation::default()
    });
}
```

Update the imports: add `fits_heuristic, pack_verses` to the `sda_core::bible` list and `use sda_core::resolve::verse_box_pt;`.

In `crates/sda-app/tests/bible_test.rs` replace `present_builds_verse_slides` with:

```rust
#[test]
fn present_builds_verse_slides() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    app.bible.selected = BTreeSet::from([16, 17, 18]);
    let _ = app.update(Msg::Bible(BibleMsg::Present));
    let n = app.live.present.slides.len();
    assert!((1..=3).contains(&n), "packed into {n} slides");
    assert_eq!(app.live.present.title, "John 3:16-18");
    assert!(matches!(
        &app.live.present.source,
        Source::Bible { version, reference }
            if version == "bible_mini" && reference == "John 3:16-18"
    ));
    app.live.theme.verses.min_size = 60.0;
    app.live.theme.text.margin = Prop::Set(0.45);
    let _ = app.update(Msg::Bible(BibleMsg::Present));
    assert_eq!(app.live.present.slides.len(), 3, "a tiny box gives one verse per slide");
}
```

Add `use sda_core::theme::Prop;` to the test imports.

- [ ] **Step 6: Run the workspace and clippy, then commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

```bash
cargo fmt --all
git add -A crates
git commit -m "feat: pack verses into as few slides as fit, one reference per slide"
```

---

### Task 5: Modifier tracking and click, shift click, drag selection

**Files:**
- Modify: `crates/sda-app/src/msg.rs` top level `Msg` enum
- Modify: `crates/sda-app/src/app.rs:69-98`, `100-125`, the `update` match
- Modify: `crates/sda-app/src/shortcuts.rs:11-17`
- Modify: `crates/sda-app/src/msg/bible.rs`
- Modify: `crates/sda-app/src/state/bible.rs:16-53`, `286-324`
- Modify: `crates/sda-app/src/view/bible.rs:243-283`
- Modify: `crates/sda-core/src/i18n.rs` remove `select_to_here`
- Test: `crates/sda-app/tests/bible_test.rs`

**Interfaces:**
- Produces: `Msg::Modifiers(iced::keyboard::Modifiers)`, `App.modifiers: iced::keyboard::Modifiers`, `BibleMsg::Press(u32)`, `BibleMsg::Enter(u32)`, `BibleMsg::Release`, `BibleState.anchor: Option<u32>`, `BibleState.dragging: bool`.
- Removes: `BibleMsg::ToggleVerse`, `BibleMsg::SelectRange`, string `select_to_here`.

- [ ] **Step 1: Write the failing tests**

Replace `toggle_and_range_select_verses` in `crates/sda-app/tests/bible_test.rs` with:

```rust
fn press(app: &mut App, n: u32) {
    let _ = app.update(Msg::Bible(BibleMsg::Press(n)));
    let _ = app.update(Msg::Bible(BibleMsg::Release));
}

#[test]
fn click_toggles_shift_click_ranges() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    press(&mut app, 2);
    assert_eq!(app.bible.selected, BTreeSet::from([2]));
    press(&mut app, 4);
    assert_eq!(app.bible.selected, BTreeSet::from([2, 4]));
    press(&mut app, 2);
    assert_eq!(app.bible.selected, BTreeSet::from([4]), "second click deselects");
    assert_eq!(app.bible.anchor, Some(2));

    let _ = app.update(Msg::Modifiers(Modifiers::SHIFT));
    press(&mut app, 6);
    assert_eq!(app.bible.selected, BTreeSet::from([2, 3, 4, 5, 6]));
    assert_eq!(app.bible.anchor, Some(2), "shift keeps the anchor");
    let _ = app.update(Msg::Modifiers(Modifiers::empty()));

    let _ = app.update(Msg::Bible(BibleMsg::ClearSelection));
    assert!(app.bible.selected.is_empty());
    let _ = app.update(Msg::Bible(BibleMsg::Book(0)));
    assert_eq!(app.bible.chapter, 1);
    assert_eq!(app.bible.anchor, None);
}

#[test]
fn drag_selects_a_range() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    let _ = app.update(Msg::Bible(BibleMsg::Press(3)));
    assert!(app.bible.dragging);
    let _ = app.update(Msg::Bible(BibleMsg::Enter(5)));
    assert_eq!(app.bible.selected, BTreeSet::from([3, 4, 5]));
    let _ = app.update(Msg::Bible(BibleMsg::Enter(1)));
    assert_eq!(app.bible.selected, BTreeSet::from([1, 2, 3]), "drag upward replaces the drag range");
    let _ = app.update(Msg::Bible(BibleMsg::Release));
    assert!(!app.bible.dragging);
    let _ = app.update(Msg::Bible(BibleMsg::Enter(9)));
    assert_eq!(app.bible.selected, BTreeSet::from([1, 2, 3]), "hover after release does nothing");
}

#[test]
fn verse_row_click_selects_without_a_checkbox() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find("Select to here").is_err());
    ui.click("For God so loved the world").expect("verse row");
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.bible.selected, BTreeSet::from([16]));
}
```

Add `use iced::keyboard::Modifiers;` to the imports. Check the verse 16 text of `bible_mini.json` and use its first words in the click if they differ.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sda-app --test bible_test`
Expected: compile errors on `Press`, `Enter`, `Release`, `Msg::Modifiers`, `anchor`.

- [ ] **Step 3: Track modifiers**

`crates/sda-app/src/msg.rs`: add to the `Msg` enum `/// Shift, Ctrl, Alt, Cmd state. Updated on every change, captured or not.` `Modifiers(iced::keyboard::Modifiers),`.

`crates/sda-app/src/app.rs`: add `pub modifiers: iced::keyboard::Modifiers,` to `App` and `modifiers: iced::keyboard::Modifiers::empty(),` in `App::new`. In `update` add `Msg::Modifiers(m) => self.modifiers = m,`.

`crates/sda-app/src/shortcuts.rs`: at the top of `filter`, before the captured check:

```rust
    if let Event::Keyboard(keyboard::Event::ModifiersChanged(m)) = event {
        return Some(Msg::Modifiers(m));
    }
```

- [ ] **Step 4: New messages and state**

`crates/sda-app/src/msg/bible.rs`: replace `ToggleVerse(u32),` and `SelectRange(u32, u32),` with:

```rust
    /// Mouse down on a verse row. Toggles it, or with Shift selects from
    /// the anchor to it.
    Press(u32),
    /// The pointer entered a verse row. Extends a drag.
    Enter(u32),
    /// Mouse up anywhere on the list. Ends a drag.
    Release,
```

`crates/sda-app/src/state/bible.rs`: add to `BibleState` after `selected`:

```rust
    /// Last verse clicked without Shift. Shift click and drag range from it.
    pub anchor: Option<u32>,
    /// True between `Press` and `Release`. `Enter` extends the range then.
    pub dragging: bool,
```

Initialize `anchor: None, dragging: false,` in `Default`. In `jump` and in the `Book` and `Chapter` arms set `app.bible.anchor = None;` wherever `selected` is cleared or replaced. Add a method:

```rust
    /// Replace the drag range. Verses selected before the drag began stay.
    fn drag_to(&mut self, n: u32) {
        let Some(a) = self.anchor else { return };
        let range = a.min(n)..=a.max(n);
        self.selected.retain(|v| !self.drag_range.contains(v));
        self.selected.extend(range.clone());
        self.drag_range = range;
    }
```

That needs `pub drag_range: std::ops::RangeInclusive<u32>` on the state, default `1..=0`. Reset it in `Press` to `n..=n`.

Replace the `ToggleVerse` and `SelectRange` arms in `update` with:

```rust
        BibleMsg::Press(n) => {
            if app.modifiers.shift() && let Some(a) = app.bible.anchor {
                app.bible.selected.extend(a.min(n)..=a.max(n));
            } else {
                if !app.bible.selected.remove(&n) {
                    app.bible.selected.insert(n);
                }
                app.bible.anchor = Some(n);
                app.bible.drag_range = n..=n;
                app.bible.dragging = true;
            }
        }
        BibleMsg::Enter(n) => {
            if app.bible.dragging {
                app.bible.drag_to(n);
            }
        }
        BibleMsg::Release => app.bible.dragging = false,
```

A drag that starts on a selected verse first deselects it on `Press`, then `drag_to` selects the range including it. That is the expected drag feel.

- [ ] **Step 5: Rows without a checkbox**

In `crates/sda-app/src/view/bible.rs` replace the `rows` construction in `verses_pane` with:

```rust
    let rows = Column::with_children(st.chapter_verses().into_iter().map(|(n, verse)| {
        let selected = st.selected.contains(&n);
        let line = row![
            text(n.to_string())
                .size(12)
                .font(ui_theme::BOLD)
                .color(ui_theme::MUTED)
                .width(26),
            text(verse).size(14).width(Fill),
        ]
        .spacing(4)
        .align_y(Alignment::Start);
        mouse_area(
            container(line)
                .padding([4, 8])
                .width(Fill)
                .style(ui_theme::verse_row(selected)),
        )
        .interaction(iced::mouse::Interaction::Pointer)
        .on_press(b(BibleMsg::Press(n)))
        .on_enter(b(BibleMsg::Enter(n)))
        .on_release(b(BibleMsg::Release))
        .into()
    }))
    .spacing(2);
```

Delete `let last = st.selected.last().copied();`. Remove `checkbox` from the import list.

In `crates/sda-app/src/ui_theme.rs` add next to `list_row`:

```rust
/// A verse row. Selected rows get the accent tint, others are flat.
pub fn verse_row(selected: bool) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: selected.then(|| ACCENT.scale_alpha(0.22).into()),
        border: Border::default().rounded(RADIUS),
        ..container::Style::default()
    }
}
```

Check how `list_row` tints its selected state and copy that color instead of `ACCENT.scale_alpha(0.22)` if it differs, so both lists look the same.

Wrap the `scrollable` of the verse list in a `mouse_area(...).on_release(b(BibleMsg::Release))` so a release on the gap between rows also ends the drag.

`crates/sda-core/src/i18n.rs`: delete `select_to_here` from the struct, `as_fields`, and the three blocks.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p sda-app --test bible_test && cargo test -p sda-core --test i18n_test`
Expected: PASS.

- [ ] **Step 7: Run the workspace and clippy, then commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

```bash
cargo fmt --all
git add -A crates
git commit -m "feat(bible): click, shift click, and drag select verses"
```

---

### Task 6: One search bar, text mode, and the preview pane

**Files:**
- Modify: `crates/sda-app/src/msg/bible.rs`
- Modify: `crates/sda-app/src/state/bible.rs`
- Modify: `crates/sda-app/src/view/bible.rs:17-20`, `41-107`, `213-344`
- Modify: `crates/sda-core/src/i18n.rs`
- Test: `crates/sda-app/tests/bible_test.rs`

**Interfaces:**
- Produces: `BibleMsg::Input(String)`, `BibleMsg::Submit`, `BibleMsg::TextMode(bool)`, `BibleMsg::PreviewSlide(usize)`, `BibleState.text_mode: bool`, `BibleState.preview_slide: usize`, `pub fn preview_slides(app: &App) -> Vec<SlideContent>` in `state/bible.rs`, view const `INPUT_ID: &str = "bible-input"`.
- Removes: `BibleMsg::Reference`, `BibleMsg::Search`, `BibleState.query`, view consts `REFERENCE_ID` and `SEARCH_ID`.
- Strings added: `bible_mode_reference` En `Ref` It `Rif` Ro `Ref`, `bible_mode_text` En `Text` It `Testo` Ro `Text`, `bible_input_hint` En `Type a reference like John 3:16 or mat 10 1` It `Scrivi un riferimento come Giovanni 3:16 o mat 10 1` Ro `Scrie o referință precum Ioan 3:16 sau mat 10 1`, `bible_preview_empty` En `Pick a chapter to preview` It `Scegli un capitolo per l'anteprima` Ro `Alege un capitol pentru previzualizare`. Removed: `bible_reference_hint`.

- [ ] **Step 1: Write the failing tests**

In `crates/sda-app/tests/bible_test.rs` replace `reference_field_jumps_to_chapter` and `search_result_jumps_and_selects` with:

```rust
#[test]
fn input_jumps_on_short_reference() {
    let mut app = app();
    let mut ui = Simulator::new(app.view_operator());
    ui.click(selector::id(sda_app::view::bible::INPUT_ID))
        .expect("input");
    ui.typewrite("joh 3 16-18");
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.bible.book, 42);
    assert_eq!(app.bible.chapter, 3);
    assert_eq!(app.bible.selected, BTreeSet::from([16, 17, 18]));
    assert_eq!(app.bible.preview_slide, 0);
}

#[test]
fn text_mode_searches_and_a_result_jumps() {
    let mut app = app();
    let _ = app.update(Msg::Bible(BibleMsg::TextMode(true)));
    let _ = app.update(Msg::Bible(BibleMsg::Input("so loved".into())));
    assert_eq!(app.bible.results.len(), 1);
    let mut ui = Simulator::new(app.view_operator());
    ui.click("John 3:16").expect("search result button");
    for m in ui.into_messages() {
        let _ = app.update(m);
    }
    assert_eq!(app.bible.book, 42);
    assert_eq!(app.bible.chapter, 3);
    assert_eq!(app.bible.selected, BTreeSet::from([16]));
    let _ = app.update(Msg::Bible(BibleMsg::TextMode(false)));
    assert!(app.bible.results.is_empty(), "leaving text mode clears results");
}

#[test]
fn submit_in_text_mode_jumps_to_the_first_result() {
    let mut app = app();
    let _ = app.update(Msg::Bible(BibleMsg::TextMode(true)));
    let _ = app.update(Msg::Bible(BibleMsg::Input("so loved".into())));
    let _ = app.update(Msg::Bible(BibleMsg::Submit));
    assert_eq!(app.bible.selected, BTreeSet::from([16]));
}

#[test]
fn preview_slides_follow_the_selection() {
    let mut app = app();
    app.bible.book = 42;
    app.bible.chapter = 3;
    assert!(!sda_app::state::bible::preview_slides(&app).is_empty(), "whole chapter when nothing is selected");
    press(&mut app, 16);
    let slides = sda_app::state::bible::preview_slides(&app);
    assert_eq!(slides.len(), 1);
    match &slides[0] {
        sda_core::resolve::SlideContent::Verses { reference, verses } => {
            assert_eq!(reference, "John 3:16");
            assert_eq!(verses.len(), 1);
        }
        _ => panic!("verses slide"),
    }
    let _ = app.update(Msg::Bible(BibleMsg::PreviewSlide(5)));
    assert_eq!(app.bible.preview_slide, 0, "clamped to the slide count");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sda-app --test bible_test`
Expected: compile errors on `INPUT_ID`, `TextMode`, `Input`, `Submit`, `PreviewSlide`, `preview_slides`.

- [ ] **Step 3: Messages and state**

`crates/sda-app/src/msg/bible.rs`: replace `Reference(String),` and `Search(String),` with:

```rust
    /// Typed into the one bar. A reference in Ref mode, a query in Text mode.
    Input(String),
    /// Enter in the bar. In Text mode jumps to the first result.
    Submit,
    TextMode(bool),
    /// Which packed slide the preview pane shows.
    PreviewSlide(usize),
```

`crates/sda-app/src/state/bible.rs`: rename `reference_input` to `input` throughout the file. Delete `query`. Add `pub text_mode: bool,` and `pub preview_slide: usize,` with defaults `false` and `0`. Replace `run_search` with:

```rust
fn run_search(app: &mut App) {
    app.bible.results = match &app.bible.bible {
        Some(b) if app.bible.text_mode && !app.bible.input.trim().is_empty() => {
            bible::search(b, &app.bible.input, SEARCH_LIMIT)
        }
        _ => Vec::new(),
    };
}
```

Add after `present`:

```rust
/// The packed slides the preview pane shows: the selection, or the whole
/// chapter when nothing is selected. Resolved under the live theme.
pub fn preview_slides(app: &App) -> Vec<SlideContent> {
    let Some(b) = &app.bible.bible else {
        return Vec::new();
    };
    let r = app.bible.reference();
    let verses: Vec<(u32, String)> = bible::verses(b, &r)
        .into_iter()
        .filter(|(n, _)| app.bible.selected.is_empty() || app.bible.selected.contains(n))
        .collect();
    let theme = &app.live.theme;
    let groups = pack_verses(&verses, fits_heuristic(theme, verse_box_pt(theme)));
    verse_slides(&r, &groups, app.core.language)
}
```

Import `SlideContent` from `sda_core::resolve`. `present` now reuses `preview_slides` for its `slides` so the preview and the projector always agree:

```rust
    let slides = preview_slides(app);
    if slides.is_empty() {
        return;
    }
    app.live.go(Presentation { title: reference.clone(), subtitle: ..., slides, source: ..., ..Presentation::default() });
```

Keep the `reference` and `version` lines from Task 5 as they are.

Update arms: replace `BibleMsg::Reference(text)` and `BibleMsg::Search(q)` with:

```rust
        BibleMsg::Input(text) => {
            app.bible.input = text;
            if app.bible.text_mode {
                run_search(app);
            } else if let Some(r) = parse_reference(&app.bible.input, app.core.language) {
                app.bible.jump(r);
            }
        }
        BibleMsg::Submit => {
            if app.bible.text_mode && let Some((r, _)) = app.bible.results.first().copied() {
                app.bible.jump(r);
            }
        }
        BibleMsg::TextMode(on) => {
            app.bible.text_mode = on;
            app.bible.input.clear();
            run_search(app);
        }
        BibleMsg::PreviewSlide(i) => {
            let n = preview_slides(app).len();
            app.bible.preview_slide = i.min(n.saturating_sub(1));
        }
```

`results` holds `(Reference, String)`, so `first().copied()` does not compile. Use `app.bible.results.first().map(|(r, _)| *r)`.

Set `app.bible.preview_slide = 0;` inside `jump`, in the `Book`, `Chapter`, `Press`, `ClearSelection` arms, and in `drag_to`. Easiest: a `fn selection_changed(&mut self)` on `BibleState` that resets `preview_slide`, called from each.

Search result click: in the view the result button sends `b(BibleMsg::Input(label))` today. That breaks in text mode. Send a dedicated jump instead: add `BibleMsg::Jump(Reference)` to the enum with the arm `BibleMsg::Jump(r) => app.bible.jump(r),` and have result buttons send `b(BibleMsg::Jump(*r))`. `Reference` is `Copy`.

- [ ] **Step 4: The view**

In `crates/sda-app/src/view/bible.rs`:

Replace the two id consts with `pub const INPUT_ID: &str = "bible-input";`.

`show`: the body row becomes `row![books_pane(app), verses_pane(app), preview_pane(app)]`.

`top_row`: replace the `reference` input and the `manage` button so the row reads `row![picker, input, mode, manage]`:

```rust
    let hint = if st.text_mode {
        &s.bible_search_hint
    } else {
        &s.bible_input_hint
    };
    let input = text_input(hint, &st.input)
        .id(INPUT_ID)
        .on_input(|t| b(BibleMsg::Input(t)))
        .on_submit(b(BibleMsg::Submit))
        .style(ui_theme::input)
        .padding([6, 10])
        .size(14)
        .width(Fill);
    let mode = segmented_on_card(
        [
            (s.bible_mode_reference.as_str(), false),
            (s.bible_mode_text.as_str(), true),
        ],
        st.text_mode,
        |on| b(BibleMsg::TextMode(on)),
    );
```

Import `crate::widgets::segmented::segmented_on_card`.

Delete `search_pane`. Add:

```rust
const PREVIEW_WIDTH: f32 = 320.0;
const PREVIEW_H: f32 = (PREVIEW_WIDTH - 2.0 * ui_theme::SPACING) * 9.0 / 16.0;

fn preview_pane(app: &App) -> Element<'_, Msg> {
    let s = &app.core.strings;
    let st = &app.bible;
    let lang = app.core.language;
    let slides = crate::state::bible::preview_slides(app);
    let count = slides.len();
    let i = st.preview_slide.min(count.saturating_sub(1));
    let w = PREVIEW_WIDTH - 2.0 * ui_theme::SPACING;
    let frame: Element<'_, Msg> = match slides.get(i) {
        None => center(text(&s.bible_preview_empty).size(13).color(ui_theme::MUTED)).into(),
        Some(content) if app.gpu_ok => {
            match crate::state::bible::preview_scene(app, content, (w as u32 * 2, PREVIEW_H as u32 * 2)) {
                Some(scene) => scene_view(key("bible_preview"), scene),
                None => space::horizontal().into(),
            }
        }
        Some(SlideContent::Verses { verses, .. }) => {
            let body: String = verses.iter().map(|(n, v)| format!("{n} {v}")).collect::<Vec<_>>().join("\n");
            container(text(body).size(12).center()).padding(ui_theme::SPACING).into()
        }
        Some(_) => space::horizontal().into(),
    };
    let frame = container(frame)
        .width(w)
        .height(PREVIEW_H)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(ui_theme::thumb_frame);
    let nav = |label: &'static str, target: Option<usize>| {
        button(text(label).size(13))
            .padding([4, 10])
            .style(ui_theme::ghost_button)
            .on_press_maybe(target.map(|t| b(BibleMsg::PreviewSlide(t))))
    };
    let counter = row![
        nav("‹", (i > 0).then(|| i - 1)),
        container(text(format!("{} / {}", i + 1, count.max(1))).size(12))
            .padding([3, 8])
            .style(ui_theme::chip),
        nav("›", (i + 1 < count).then_some(i + 1)),
    ]
    .spacing(4)
    .align_y(Alignment::Center);
    let clear = button(text(&s.clear_selection).size(13))
        .padding([6, 12])
        .style(ui_theme::ghost_button)
        .on_press_maybe((!st.selected.is_empty()).then_some(b(BibleMsg::ClearSelection)));
    let present = button(text(&s.bible_present_selected).size(13))
        .padding([6, 14])
        .style(ui_theme::accent_button)
        .on_press_maybe((count > 0).then_some(b(BibleMsg::Present)));
    let actions = row![counter, space::horizontal(), clear, present]
        .spacing(ui_theme::SPACING / 2.0)
        .align_y(Alignment::Center);
    let mut col = column![heading(&s.preview), frame, actions].spacing(ui_theme::SPACING);
    if st.text_mode {
        let results = Column::with_children(st.results.iter().map(|(r, verse)| {
            button(
                column![
                    text(format_reference(r, lang)).size(12).font(ui_theme::BOLD),
                    text(verse.as_str()).size(12).color(ui_theme::MUTED),
                ]
                .spacing(2),
            )
            .padding([6, 8])
            .width(Fill)
            .style(ui_theme::ghost_button)
            .on_press(b(BibleMsg::Jump(*r)))
            .into()
        }))
        .spacing(4);
        let list: Element<'_, Msg> = if st.results.is_empty() {
            center(text(&s.bible_search_empty).size(13).color(ui_theme::MUTED).center())
                .padding(ui_theme::SPACING)
                .into()
        } else {
            scrollable(gutter(results)).direction(thin_scrollbar()).height(Fill).into()
        };
        col = col.push(list);
    }
    card(col).width(PREVIEW_WIDTH).height(Fill).into()
}
```

Remove `clear` and `present` from `verses_pane`'s header, the preview pane owns them now. Import `crate::widgets::scene_view::{key, scene_view}` and `sda_core::resolve::SlideContent`.

Add to `crates/sda-app/src/state/bible.rs`:

```rust
/// A static scene of one verses slide under the live theme, same shape the
/// Present preview uses.
pub fn preview_scene(app: &App, content: &SlideContent, size: (u32, u32)) -> Option<Arc<sda_render::Scene>> {
    let theme = &app.live.theme;
    let footer = theme.footer.then(|| app.bible.version.as_deref().map(|v| app.bible.abbreviation(v)).unwrap_or_default());
    let resolved = Arc::new(sda_core::resolve::resolve(theme, content, footer.as_deref())?);
    Some(crate::state::present::still(theme, &app.live.bitmaps, sda_render::Layer::Text(resolved), size))
}
```

Make `still` in `crates/sda-app/src/state/present.rs` `pub(crate)`.

`crates/sda-core/src/i18n.rs`: add the four strings and remove `bible_reference_hint`, each in the struct, `as_fields`, and all three languages.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p sda-app --test bible_test && cargo test -p sda-core --test i18n_test`
Expected: PASS. If the `scene_view` key collides with another on screen, rename it. Only `bible_preview` is new.

- [ ] **Step 6: Run the workspace and clippy, then commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

```bash
cargo fmt --all
git add -A crates
git commit -m "feat(bible): one search bar with a text mode and a slide preview pane"
```

---

### Task 7: Themes editor, two columns, sample toggle, Reference section

**Files:**
- Modify: `crates/sda-app/src/msg/themes.rs`
- Modify: `crates/sda-app/src/state/themes.rs:32-98`, `140-199`, `269-399`, `418-446`, and the `update` match near line 640
- Modify: `crates/sda-app/src/view/themes.rs:24-39`, `279-369`
- Modify: `crates/sda-app/src/view/theme_form.rs:49-168`, `273-486`, `714-725`
- Modify: `crates/sda-core/src/i18n.rs`
- Test: `crates/sda-app/tests/themes_test.rs`

**Interfaces:**
- Produces: `DraftEdit::Reference(Box<DraftEdit>)`, `ColorField::RefText`, `ColorField::RefShadow`, `ColorField::RefOutline`, `ThemesMsg::Sample(Sample)`, `pub enum Sample { Hymn, Bible }` in `msg/themes.rs`, `ThemesState.sample: Sample`, `ThemesState.ref_stash: InheritStash`.
- Strings added: `theme_reference` En `Reference` It `Riferimento` Ro `Referință`, `theme_sample_hymn` En `Hymn` It `Inno` Ro `Imn`, `theme_sample_bible` En `Bible` It `Bibbia` Ro `Biblie`, `theme_preview_verse_1` En `For God so loved the world, that he gave his only begotten Son` It `Perché Dio ha tanto amato il mondo, che ha dato il suo unigenito Figlio` Ro `Fiindcă atât de mult a iubit Dumnezeu lumea, că a dat pe singurul Lui Fiu`, `theme_preview_verse_2` En `that whosoever believeth in him should not perish, but have everlasting life.` It `affinché chiunque crede in lui non perisca, ma abbia vita eterna.` Ro `pentru ca oricine crede în El să nu piară, ci să aibă viața veșnică.`, `theme_preview_reference` En `John 3:16` It `Giovanni 3:16` Ro `Ioan 3:16`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/sda-app/tests/themes_test.rs`:

```rust
#[test]
fn reference_edits_target_the_reference_style() {
    let mut app = themes_app();
    send(&mut app, ThemesMsg::Draft(DraftEdit::Reference(Box::new(DraftEdit::Color(Rgba::new(9, 8, 7, 255))))));
    assert_eq!(app.themes.draft.reference.color, Prop::Set(Rgba::new(9, 8, 7, 255)));
    assert_eq!(app.themes.draft.text.color, Prop::Inherit, "verse text untouched");
    send(&mut app, ThemesMsg::Draft(DraftEdit::Reference(Box::new(DraftEdit::VAlign(VAlign::Top)))));
    assert_eq!(app.themes.draft.reference.v_align, VAlign::Top);
}

#[test]
fn verse_edits_flip_the_preview_sample_to_bible() {
    let mut app = themes_app();
    assert_eq!(app.themes.sample, Sample::Hymn);
    send(&mut app, ThemesMsg::Draft(DraftEdit::MinSize(30.0)));
    assert_eq!(app.themes.sample, Sample::Bible);
    assert!(matches!(
        app.themes.preview.present.slides.first(),
        Some(sda_core::resolve::SlideContent::Verses { .. })
    ));
    send(&mut app, ThemesMsg::Sample(Sample::Hymn));
    assert_eq!(app.themes.sample, Sample::Hymn);
    assert!(matches!(
        app.themes.preview.present.slides.first(),
        Some(sda_core::resolve::SlideContent::Styled(_))
    ));
}

#[test]
fn reference_section_shows_in_the_form() {
    let app = themes_app();
    let mut ui = tall(&app);
    assert!(ui.find("Reference").is_ok());
    assert!(ui.find("Smallest text size").is_ok());
    assert!(ui.find("Per slide").is_err());
}
```

Add `use sda_app::msg::themes::Sample;` and `use sda_core::theme::VAlign;` to the imports.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sda-app --test themes_test`
Expected: compile errors on `Reference`, `Sample`.

- [ ] **Step 3: Messages**

`crates/sda-app/src/msg/themes.rs`:

```rust
/// What the preview plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sample {
    Hymn,
    Bible,
}
```

Add `RefText, RefShadow, RefOutline,` to `ColorField` and in `ColorField::edit`:

```rust
            ColorField::RefText => DraftEdit::Reference(Box::new(DraftEdit::Color(c))),
            ColorField::RefShadow => DraftEdit::Reference(Box::new(DraftEdit::ShadowColor(c))),
            ColorField::RefOutline => DraftEdit::Reference(Box::new(DraftEdit::OutlineColor(c))),
```

Add to `DraftEdit`: `/// A text edit applied to the reference style instead of the verse text.` `Reference(Box<DraftEdit>),`. Add to `ThemesMsg`: `Sample(Sample),`.

- [ ] **Step 4: State**

`crates/sda-app/src/state/themes.rs`:

Add `pub sample: Sample,` and `pub ref_stash: InheritStash,` to `ThemesState`, defaults `Sample::Hymn` and `InheritStash::default()`.

Split `apply`. The text arms from `FontInherit` through `OutlineWidth` move into:

```rust
/// Apply a text edit to one `TextStyle`. Returns false when `e` is not a
/// text edit, so the caller handles it.
fn apply_text(t: &mut TextStyle, stash: &mut InheritStash, hex_edit: &mut Option<(ColorField, String)>, e: &DraftEdit) -> bool {
    match e {
        DraftEdit::FontInherit(i) => toggle(&mut t.font, &mut stash.font, *i, Font::Bundled(BundledFont::Inter)),
        DraftEdit::Font(f) => t.font = Prop::Set(f.clone()),
        // ... every text arm from the old apply, with `*` on Copy payloads ...
        DraftEdit::OutlineWidth(w) => { if let Some(o) = &mut t.outline { o.width = *w; } }
        _ => return false,
    }
    true
}
```

Where the old arms set `st.hex_edit = None` they now set `*hex_edit = None`. Then `apply` becomes:

```rust
pub fn apply(st: &mut ThemesState, e: DraftEdit) {
    if let DraftEdit::Reference(inner) = e {
        let d = &mut st.draft;
        apply_text(&mut d.reference, &mut st.ref_stash, &mut st.hex_edit, &inner);
        st.sample = Sample::Bible;
        return;
    }
    if apply_text(&mut st.draft.text, &mut st.stash, &mut st.hex_edit, &e) {
        return;
    }
    let d = &mut st.draft;
    match e {
        DraftEdit::Name(n) => d.name = n,
        // background, transition, footer arms unchanged
        DraftEdit::MinSize(v) => { d.verses.min_size = v.clamp(16.0, 60.0); st.sample = Sample::Bible; }
        DraftEdit::ShowReference(b) => { d.verses.show_reference = b; st.sample = Sample::Bible; }
        DraftEdit::ShowNumbers(b) => { d.verses.show_numbers = b; st.sample = Sample::Bible; }
        DraftEdit::Footer(b) => d.footer = b,
        _ => {}
    }
}
```

The `_ => {}` arm catches the text edits already handled. Keep it last.

Sample selection. Change `SampleKey` to carry the kind: `type SampleKey = (Sample, Option<(PathBuf, usize)>);` default `(Sample::Hymn, None)`. Add:

```rust
fn bible_sample(s: &Strings) -> SlideContent {
    SlideContent::Verses {
        reference: s.theme_preview_reference.clone(),
        verses: vec![(16, s.theme_preview_verse_1.clone()), (17, s.theme_preview_verse_2.clone())],
    }
}
```

In `sample(app)`: when `app.themes.sample == Sample::Bible` return `(bible_sample(s), s.theme_preview_reference.clone(), (Sample::Bible, None))` before the hymn branch. Wrap the two existing return keys as `(Sample::Hymn, Some(..))` and `(Sample::Hymn, None)`.

In `sample_presentation(app)`: when `Sample::Bible`, build a presentation with two slides, `bible_sample(s)` and a second `SlideContent::Verses` holding only verse 17 with reference `s.theme_preview_reference`, title `s.theme_preview_reference`, `Source::None`. The two slides let the transition loop as the hymn sample does.

`thumb_for` and `sync_preview` keep using `sample(app)`. Theme list thumbnails use the hymn sample always: in `sync_preview` and `thumb_for` call a new `fn hymn_sample(app)` that is the old `sample` body, and use `sample(app)` only for the preview player. Simplest: rename the old function `hymn_sample`, write `sample` as the switch on `app.themes.sample`, and change the two thumbnail call sites to `hymn_sample`.

In `update` add `ThemesMsg::Sample(k) => { app.themes.sample = k; }` and fall through to the existing `sync_preview` call that every draft edit already triggers. Check how `ThemesMsg::Draft` returns `sync_preview(app)` and do the same.

- [ ] **Step 5: Form**

`crates/sda-app/src/view/theme_form.rs`:

Generalize `text_section` into

```rust
fn text_style_section<'a>(
    app: &'a App,
    title: &'a str,
    t: &'a TextStyle,
    file: Inherited,
    colors: (ColorField, ColorField, ColorField),
    wrap: fn(DraftEdit) -> DraftEdit,
    name_row: Option<Element<'a, Msg>>,
) -> Element<'a, Msg>
```

Every `DraftEdit::X` the old body passed as a `fn(...) -> DraftEdit` becomes a closure `move |v| wrap(DraftEdit::X(v))`. For that, change the `f: fn(bool) -> DraftEdit` parameters of `toggle_box`, `inherit_row`, and `slide` to `impl Fn(..) -> DraftEdit + 'a`. `color_field` takes the `ColorField` from `colors`: `.0` text, `.1` shadow, `.2` outline. `name_row` is pushed first when `Some`.

Then:

```rust
fn text_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let name = field(&s.theme_name_label, text_input(s.theme_name_label.as_str(), &d.name).on_input(|n| edit(DraftEdit::Name(n))).style(ui_theme::input).size(13).padding(6).width(260));
    text_style_section(app, &s.theme_text, &d.text, inherited(app), (ColorField::Text, ColorField::Shadow, ColorField::Outline), |e| e, Some(name))
}

fn reference_section<'a>(app: &'a App, d: &'a Theme) -> Element<'a, Msg> {
    let s = &app.core.strings;
    let resolved = sda_core::resolve::reference_style(d);
    let file = Inherited {
        font: Some(resolved.font.resolve(None, Font::Bundled(BundledFont::Inter)).family().to_string()),
        size: match resolved.size.resolve(None, TextSize::Auto) { TextSize::Pt(p) => Some(format!("{p:.0}pt")), TextSize::Auto => Some(s.theme_auto_size.clone()) },
        bold: Some(if resolved.bold.resolve(None, false) { "✓" } else { "✗" }.to_string()),
        color: Some(resolved.color.resolve(None, Rgba::WHITE).to_hex()),
        h_align: None,
        spacing: Some(format!("{:.2}", resolved.line_spacing.resolve(None, 1.1))),
    };
    text_style_section(app, &s.theme_reference, &d.reference, file, (ColorField::RefText, ColorField::RefShadow, ColorField::RefOutline), |e| DraftEdit::Reference(Box::new(e)), None)
}
```

`show` returns the two columns instead of one:

```rust
/// The left and right form columns. The preview sits above the left one.
pub fn columns(app: &App) -> (Element<'_, Msg>, Element<'_, Msg>) {
    let d = &app.themes.draft;
    let left = column![text_section(app, d), reference_section(app, d)].spacing(ui_theme::SPACING).into();
    let right = column![background_section(app, d), transition_section(app, d), verses_section(app, d), footer_section(app, d)].spacing(ui_theme::SPACING).into();
    (left, right)
}
```

Delete `show`.

`crates/sda-app/src/view/themes.rs`: replace `PREVIEW_W` and `PREVIEW_H` with `const PREVIEW_MAX_H: f32 = 540.0;`. `preview(app)` becomes `preview(app, w: f32)` with `h = (w * 9.0 / 16.0).min(PREVIEW_MAX_H)` and the frame `.width(w).height(h)`; the scene size is `(w as u32 * 2, h as u32 * 2)`. Add the sample toggle to the toolbar column as its first item:

```rust
        segmented(
            [(s.theme_sample_hymn.as_str(), Sample::Hymn), (s.theme_sample_bible.as_str(), Sample::Bible)],
            app.themes.sample,
            |k| Msg::Themes(ThemesMsg::Sample(k)),
        ),
```

Move the toolbar under the frame instead of beside it, as a `row` with the toggle, Save, Set active, Favorite, so the frame takes the full column width. Replace `editor`:

```rust
fn editor(app: &App) -> Element<'_, Msg> {
    let (left, right) = theme_form::columns(app);
    let scroll = |body: Element<'_, Msg>| {
        scrollable(container(body).padding(iced::Padding::ZERO.right(SCROLL_GUTTER)))
            .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(6).margin(2).scroller_width(6)))
            .height(Fill)
            .width(Fill)
    };
    let left_col = responsive(move |avail| {
        let w = avail.width - 2.0 * ui_theme::SPACING - SCROLL_GUTTER;
        scroll(column![preview(app, w), left].spacing(ui_theme::SPACING).into()).into()
    });
    row![left_col, scroll(right)].spacing(ui_theme::SPACING).height(Fill).width(Fill).into()
}
```

`responsive` captures `left` by move, so build `left` inside the closure by calling `theme_form::columns(app).0` there, and `right` outside. Import `iced::widget::responsive` and `crate::widgets::segmented::segmented`.

`crates/sda-core/src/i18n.rs`: add the seven strings in the struct, `as_fields`, and all three languages.

- [ ] **Step 6: Run the tests**

Run: `cargo test -p sda-app --test themes_test && cargo test -p sda-core --test i18n_test`
Expected: PASS. `inherit_checkbox_flips_prop` clicks `COLOR_INHERIT_ID`, which must stay on the verse text color row only. Pass `Some(COLOR_INHERIT_ID)` for the text section and `None` for the reference section through a new `color_id: Option<&'static str>` parameter of `text_style_section`.

- [ ] **Step 7: Run the app and look**

Run: `cargo run -p sda-app`, open Themes. Check: two columns, the preview fills the left column, the Hymn | Bible toggle switches the sample, dragging Smallest text size flips to Bible, the Reference section edits move the reference. Fix layout gaps before committing.

- [ ] **Step 8: Run the workspace and clippy, then commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

```bash
cargo fmt --all
git add -A crates
git commit -m "feat(themes): two column editor, bible sample, independent reference style"
```

---

### Task 8: History card on the Present tab

**Files:**
- Modify: `crates/sda-app/src/view/present.rs:1-3`, `21-37`, `51-108`, `317-382`
- Modify: `crates/sda-app/src/state/present.rs:29`, add `select_history`
- Modify: `crates/sda-app/src/msg/present.rs`
- Modify: `crates/sda-core/src/i18n.rs`
- Test: `crates/sda-app/tests/present_test.rs`

**Interfaces:**
- Produces: `PresentMsg::OpenHistory(PathBuf)`, `pub fn select_history(app: &mut App, path: &Path)` in `state/present.rs`, `pub fn relative_time(s: &Strings, then: i64, now: i64) -> String` in `state/present.rs`.
- Removes: `RECENT_COUNT`, strings `notes`, `no_notes`, `recent`.
- Strings added: `history` En `History` It `Cronologia` Ro `Istoric`, `time_just_now` En `Just now` It `Adesso` Ro `Chiar acum`, `time_minutes_ago_fmt` En `{} min ago` It `{} min fa` Ro `{} min în urmă`, `time_hours_ago_fmt` En `{} h ago` It `{} h fa` Ro `{} h în urmă`, `time_yesterday` En `Yesterday` It `Ieri` Ro `Ieri`, `history_empty` En `No hymns opened yet` It `Nessun inno aperto` Ro `Niciun imn deschis`.

- [ ] **Step 1: Write the failing tests**

Open `crates/sda-app/tests/present_test.rs` and read its helper that builds an app with an indexed fixture library. Reuse it. Append:

```rust
#[test]
fn relative_time_buckets() {
    use sda_app::state::present::relative_time;
    use sda_core::i18n::{Language, Strings};
    let s = Strings::for_language(Language::En);
    let now = 1_000_000;
    assert_eq!(relative_time(&s, now - 10, now), "Just now");
    assert_eq!(relative_time(&s, now - 5 * 60, now), "5 min ago");
    assert_eq!(relative_time(&s, now - 3 * 3600, now), "3 h ago");
    assert_eq!(relative_time(&s, now - 30 * 3600, now), "Yesterday");
    assert!(relative_time(&s, now - 10 * 86400, now).contains('-'), "older is a date");
}

#[test]
fn history_card_lists_and_opens_hymns() {
    let mut app = indexed_app();
    let _ = app.update(Msg::Search("1".into()));
    let _ = app.update(Msg::GoLive);
    assert!(!app.core.history.is_empty());
    let path = app.core.history[0].path.clone();
    let _ = app.update(Msg::Search("zzzz".into()));
    assert!(app.present.hits.is_empty());
    let mut ui = Simulator::new(app.view_operator());
    assert!(ui.find("History").is_ok());
    assert!(ui.find("Notes").is_err());
    assert!(ui.find("Recent").is_err());
    drop(ui);
    let _ = app.update(Msg::Present(PresentMsg::OpenHistory(path.clone())));
    let (_, hymn) = sda_app::state::present::selected_hymn(&app).expect("selected");
    assert_eq!(hymn.path, path);
}
```

Replace `indexed_app()` with the name of the existing helper in that file.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sda-app --test present_test`
Expected: compile errors on `relative_time`, `OpenHistory`.

- [ ] **Step 3: State and message**

`crates/sda-app/src/msg/present.rs`: add `/// A History card row was clicked.` `OpenHistory(PathBuf),` and `use std::path::PathBuf;`.

`crates/sda-app/src/state/present.rs`: delete `RECENT_COUNT`. Add:

```rust
/// Select the hymn at `path`. When the current hits do not hold it, search
/// its number so it does, then select the hit.
pub fn select_history(app: &mut App, path: &Path) {
    if let Some(pos) = hit_position(app, path) {
        select(app, pos);
        return;
    }
    let number = app
        .core
        .history
        .iter()
        .find(|e| e.path == path)
        .and_then(|e| e.number.clone())
        .unwrap_or_default();
    app.present.query = number;
    run_search(app);
    if let Some(pos) = hit_position(app, path) {
        select(app, pos);
    }
}

/// `then` as a short relative label against `now`, both Unix seconds.
pub fn relative_time(s: &Strings, then: i64, now: i64) -> String {
    let secs = (now - then).max(0);
    match secs {
        0..=59 => s.time_just_now.clone(),
        60..=3599 => s.time_minutes_ago_fmt.replace("{}", &(secs / 60).to_string()),
        3600..=86_399 => s.time_hours_ago_fmt.replace("{}", &(secs / 3600).to_string()),
        86_400..=172_799 => s.time_yesterday.clone(),
        _ => {
            let days = then / 86_400;
            let (y, m, d) = civil_from_days(days);
            format!("{y:04}-{m:02}-{d:02}")
        }
    }
}

/// Howard Hinnant's days to civil date, proleptic Gregorian.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
```

Import `Strings` from `sda_core::i18n`. In `update` for `PresentMsg` add `PresentMsg::OpenHistory(p) => select_history(app, &p),`. Check `run_search` is `pub` already. It is.

Add the `history_empty` string. The `fmt` strings end in `_fmt` so the i18n placeholder test can cover them; add `assert!(s.time_minutes_ago_fmt.contains("{}"))` and the same for hours to `format_strings_keep_their_placeholders` in `crates/sda-core/tests/i18n_test.rs`.

- [ ] **Step 4: View**

`crates/sda-app/src/view/present.rs`:

Delete the Recent block in `search_panel` (lines 67 to 81) and the `history` and `RECENT_COUNT` imports. Keep `heading`.

Replace the notes card in `side_panel` with:

```rust
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut rows = column![].spacing(2);
    if app.core.history.is_empty() {
        rows = rows.push(text(&s.history_empty).size(13).color(ui_theme::MUTED));
    }
    for e in &app.core.history {
        let indexed = app.core.searcher.as_ref().is_some_and(|sr| sr.has_path(&e.path));
        let color = if indexed { ui_theme::TEXT } else { ui_theme::MUTED };
        let line = row![
            text(e.number.clone().unwrap_or_default()).size(13).font(ui_theme::BOLD).color(color).width(44),
            text(shorten(&e.title, 22)).size(13).color(color).width(Fill).wrapping(text::Wrapping::None),
            text(relative_time(s, e.opened_at, now)).size(11).color(ui_theme::MUTED),
        ]
        .spacing(4)
        .align_y(Alignment::Center);
        rows = rows.push(
            button(line)
                .height(ROW_H)
                .padding([0, 8])
                .width(Fill)
                .style(ui_theme::list_row(false))
                .on_press_maybe(indexed.then(|| Msg::Present(PresentMsg::OpenHistory(e.path.clone())))),
        );
    }
    let history_card = card(
        column![ui_theme::section_label(&s.history), scrollable(rows).height(Fill)].spacing(SPACING),
    )
    .width(Fill)
    .height(Fill);
    column![theme_card, history_card].spacing(SPACING).height(Fill).into()
```

Import `crate::msg::PresentMsg` and `crate::state::present::relative_time`. `ui_theme::TEXT` is the default text color; if the palette names it differently use that name. `Searcher::has_path` does not exist. Check `sda_core::search::Searcher` for a lookup by path and add `pub fn has_path(&self, path: &Path) -> bool { self.entries().iter().any(|h| h.path == path) }` next to `entry` if there is none, using whatever the entries accessor is called.

Update the module doc comment on line 1 to say `theme quick picker and history`.

`crates/sda-core/src/i18n.rs`: add the six strings, remove `notes`, `no_notes`, `recent`.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p sda-app --test present_test --test views_test && cargo test -p sda-core --test i18n_test`
Expected: PASS. `views_test` may assert the word Notes or Recent, update those assertions to History.

- [ ] **Step 6: Run the workspace and clippy, then commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

```bash
cargo fmt --all
git add -A crates
git commit -m "feat(present): history card replaces notes, recent block folded into it"
```

---

### Task 9: Render golden for the reference band

**Files:**
- Test: `crates/sda-render/tests/golden_test.rs`
- Create: `crates/sda-render/tests/golden/verses_reference_band.png` on first run

- [ ] **Step 1: Write the test**

Append to `crates/sda-render/tests/golden_test.rs`:

```rust
#[test]
fn verses_reference_band() {
    use sda_core::resolve::{SlideContent, resolve};
    use sda_core::theme::{Prop, Theme};
    let mut t = Theme::default();
    t.text.size = Prop::Set(TextSize::Pt(40.0));
    t.reference.v_align = VAlign::Top;
    t.reference.h_align = Prop::Set(HAlign::Right);
    t.reference.color = Prop::Set(Rgba::new(255, 80, 80, 255));
    t.reference.font = Prop::Set(Font::Bundled(BundledFont::Lora));
    let content = SlideContent::Verses {
        reference: "John 3:16".into(),
        verses: vec![(16, "For God so loved the world".into())],
    };
    let slide = Arc::new(resolve(&t, &content, None).unwrap());
    assert_eq!(slide.boxes.len(), 2);
    let mut s = base();
    s.current = Some(Layer::Text(slide));
    let Some(img) = headless::render_to_image(&s) else { return };
    let top_right = img.get_pixel(300, 12);
    assert!(top_right[0] > 150 && top_right[1] < 120, "red reference in the top band: {top_right:?}");
    check("verses_reference_band", &s);
}
```

The pixel probe may land between glyphs. If it does, scan `(260..320, 4..20)` for any pixel with `r > 150 && g < 120` instead of one point.

- [ ] **Step 2: Run it**

Run: `cargo test -p sda-render verses_reference_band`
Expected: PASS and the PNG is written. Open `tests/golden/verses_reference_band.png`, confirm the red reference sits top right and the verse sits below it. Rerun, PASS against the saved golden.

- [ ] **Step 3: Commit**

```bash
cargo fmt --all
git add crates/sda-render
git commit -m "test(render): golden for the styled reference band"
```

---

### Task 10: Docs and changelog

**Files:**
- Modify: `CLAUDE.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Update CLAUDE.md**

In the Conventions bullet about themes replace `footer, VerseLayout, and favorite` with `footer, VerseLayout, a second TextStyle for the Bible reference, and favorite`. In the Bible bullets replace the `verse_slides` sentence with: `pack_verses groups a selection into as few slides as fit under fits_heuristic, a character count estimate of the verse box at the theme's min_size. Each slide carries its own reference range.` In the Keyboard section add a line after the table: `In the Bible tab a click toggles a verse, Shift click selects from the last clicked verse, and dragging selects a range. shortcuts::filter forwards ModifiersChanged as Msg::Modifiers so views know about Shift.` In the UI conventions add: `The Themes editor shows a Hymn or Bible sample. Verse and reference edits switch it to Bible.`

- [ ] **Step 2: Changelog**

Add a `## 2.1.0` section at the top of `CHANGELOG.md`:

```markdown
## 2.1.0

- Bible: click a verse to select it, Shift click or drag for a range. The checkbox and "Select to here" are gone.
- Bible: one search bar. `mat 10 1` opens Matthew 10:1, `matthew 10 1-5` selects 1 to 5. A Ref | Text toggle switches to text search.
- Bible: a preview pane shows the slides before they go to the projector.
- Verses pack into as few slides as fit. Every slide shows its own verse range. "Per slide" became "Smallest text size".
- Themes: the reference on verse slides has its own font, size, color, alignment, and position.
- Themes: two column editor, the preview fills the column, a Hymn | Bible sample toggle.
- Present: the Notes card became a History card.
```

Do not bump `Cargo.toml` versions. The release workflow owns that.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md CHANGELOG.md
git commit -m "docs: 2.1.0 notes for bible selection, verse packing, and the themes editor"
```

---

## Self review

Spec coverage: section 2 is Task 1, section 3 is Task 2, section 4 is Task 4, section 5.1 and 5.2 are Tasks 5 and 6, section 5.3 is Task 3, sections 5.4 and 5.5 are Task 6, section 6 is Task 7, section 7 is Task 8, section 8 is spread over Tasks 1, 5, 6, 7, 8, section 9 is the test steps of every task plus Task 9.

Type consistency: `pack_verses` and `fits_heuristic` take `&[(u32, String)]` everywhere. `verse_slides(&Reference, &[Vec<(u32, String)>], Language)` in Tasks 4 and 6. `preview_slides(&App) -> Vec<SlideContent>` in Task 6 only. `DraftEdit::Reference(Box<DraftEdit>)` in Task 7 tests, messages, state, and form. `Sample` lives in `msg/themes.rs` and is re-exported by nothing, so tests import `sda_app::msg::themes::Sample`.
