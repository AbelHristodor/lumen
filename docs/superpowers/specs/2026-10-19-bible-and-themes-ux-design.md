# Lumen 2.1: Bible selection, verse styling, Themes editor layout

Date: 2026-10-19. Status: approved by the owner in chat, all sections.

## 1. Goal

Six changes to the operator window:

1. Themes editor in two columns. Verse and reference edits show in the preview.
2. The reference on a verse slide has its own `TextStyle`, independent of the verse text.
3. Bible tab: click, shift click, and drag select verses. No checkbox, no "Select until here".
4. Bible tab: one search bar. `mat 10 1` is Matthew 10:1. `matthew 10 1-5` selects 1 to 5. A toggle switches the bar to text search. The right pane becomes a slide preview.
5. Verses pack into as few slides as fit. Each slide shows its own `X-Y` reference.
6. Present tab: the Notes card becomes a History card.

Nothing else moves. Config paths, theme file names, and the hymn pipeline are unchanged.

## 2. Theme model, `sda-core/src/theme.rs`

```rust
pub struct Theme {
    pub name: String,
    pub text: TextStyle,
    pub reference: TextStyle,   // new
    pub background: Background,
    pub background_opacity: f32,
    pub transition: Transition,
    pub footer: bool,
    pub verses: VerseLayout,
    pub favorite: bool,
}

pub struct VerseLayout {
    pub min_size: f32,          // points, default 28. Replaces per_slide.
    pub show_reference: bool,
    pub show_numbers: bool,
}
```

- `reference` defaults to `TextStyle::default()` with `v_align: VAlign::Bottom`.
- `RefPos` and `reference_at` are deleted. Position comes from `reference.v_align` and `reference.h_align`.
- `per_slide` is deleted. Files that still have it load because the struct is `serde(default)` and unknown fields are ignored. `ThemeV3` gains `reference` with a default and `VerseLayout` gains `min_size` with a default.
- `Theme::hash` and `VerseLayout::hash` include the new fields. `min_size` hashes through `to_bits`.
- `DraftEdit` gains `Reference(TextEdit)` where `TextEdit` is the existing set of text field edits, and `MinSize(f32)`. `PerSlide` and `RefPos` are removed.

## 3. Resolve, `sda-core/src/resolve.rs`

`resolve_verses` returns two boxes.

```rust
pub const REF_BAND: f32 = 0.12;   // slide height fraction the reference strip takes
```

- Verse box: the margin frame of `theme.text`. When `show_reference` is on and `reference.v_align` is Top or Bottom, the frame loses `REF_BAND` on that side.
- Reference box: the margin frame of `theme.reference`, cut to the `REF_BAND` strip at the top or bottom for Top or Bottom. Middle keeps the whole margin frame and overlays the verses.
- Reference runs resolve every `Prop` with `inherited` set to the resolved value of the same property in `theme.text`. Size inherits at 0.6 of the verse size, Auto stays Auto. Opacity, shadow, and outline come from `theme.reference` directly.
- `show_reference` off yields one box, as today.
- `Frame` gets a small helper `cut(&self, side: VAlign, band: f32) -> (Frame, Frame)` returning the strip and the rest. Unit tested.

## 4. Verse packing, `sda-core/src/bible/mod.rs`

```rust
/// Greedy. Adds verses while `fits` holds, then starts a new slide. A verse
/// that does not fit alone still takes one slide.
pub fn pack_verses(
    verses: &[(u32, String)],
    fits: impl Fn(&[(u32, String)]) -> bool,
) -> Vec<Vec<(u32, String)>>;

/// Heuristic measure. Box in points on a 720 by 405 slide.
pub fn fits_heuristic(theme: &Theme, box_pt: (f32, f32)) -> impl Fn(&[(u32, String)]) -> bool;

/// One `SlideContent::Verses` per group. `reference` is the group's own
/// range, so slide two of John 3:16-21 reads `John 3:19-21`.
pub fn verse_slides(bible_ref: &Reference, groups: &[Vec<(u32, String)>], lang: Language) -> Vec<SlideContent>;
```

Heuristic, marked with a `ponytail:` comment naming the cosmic-text measure as the upgrade path:

- `chars_per_line = box_w / (0.5 * min_size)`.
- `lines(verse) = ceil(chars / chars_per_line)`. Numbers on add two characters per verse.
- `line_h = min_size * line_spacing` where `line_spacing` is the resolved verse spacing.
- A group fits when `sum(lines) * line_h <= box_h`.

`box_pt` comes from the verse box frame of `resolve_verses` times 720 by 405. The app computes it once per present through a new `resolve::verse_box_pt(theme) -> (f32, f32)`.

`verse_slides` is deleted in its current form. Callers: `state/bible.rs::present` only.

## 5. Bible tab, `sda-app`

### 5.1 State

```rust
pub struct BibleState {
    pub reference_input: String,     // the one bar
    pub text_mode: bool,             // false: reference, true: text search
    pub anchor: Option<u32>,         // last clicked verse, for shift and drag
    pub dragging: bool,
    // existing: version, bible, book, chapter, selected, results, manage_open,
    // index, installed, busy, error
}
```

`query` is removed. In text mode `reference_input` is the search text.

App gains `modifiers: keyboard::Modifiers`, kept current by `Msg::Modifiers` which `shortcuts::filter` emits on `keyboard::Event::ModifiersChanged` regardless of capture status.

### 5.2 Messages

```rust
pub enum BibleMsg {
    Opened, Input(String), Submit, TextMode(bool), Version(String), Book(usize), Chapter(u32),
    Press(u32), Enter(u32), Release,
    ClearSelection, Present, ManageOpen, ManageClose, Download(String), Remove(String),
    IndexLoaded(..), Downloaded(..), Loaded(..),
}
```

`Reference`, `ToggleVerse`, `SelectRange`, and `Search` are removed.

- `Input(t)`: store. Reference mode: `parse_reference` and jump on success. Text mode: run `bible::search`.
- `Submit`: text mode and a result list, jump to the first result.
- `Press(n)`: Shift held and an anchor exists, select the range anchor to `n` and keep the anchor. Else toggle `n`, set anchor to `n`, set `dragging`.
- `Enter(n)`: while `dragging`, select the range anchor to `n`.
- `Release`: clear `dragging`.
- A new book or chapter clears selection and anchor.

### 5.3 Reference parsing, `bible/reference.rs`

`parse_chapter_verses` accepts whitespace as the chapter to verse separator when no `:` or `,` is present. Inputs and results:

| Input | Result |
|---|---|
| `mat 10` | Matthew 10 |
| `mat 10 1` | Matthew 10:1 |
| `matthew 10 1-5` | Matthew 10:1-5 |
| `mat 10:1` | Matthew 10:1 |
| `1 sam 2 1` | 1 Samuel 2:1 |
| `mat 10 1 5` | None |

A book prefix of `1`, `2`, or `3` is handled first by `split_book` as today, so the chapter and verse split sees only `C`, `C V`, `C V-W` and the colon forms.

### 5.4 View

Top row: version picker, the bar, a `Text` toggle styled as `segmented_on_card` with `Ref | Text`, then Manage.

Body: `books_pane` unchanged. `verses_pane` rows are `mouse_area` over the whole row with `on_press(Press(n))`, `on_enter(Enter(n))`, `on_release(Release)`. Selected rows use `ui_theme::list_row(true)`. The checkbox and the "Select until here" button are removed. `interaction(mouse::Interaction::Pointer)`.

Right pane, 320 wide, `preview_pane`:

- A 16 by 9 `scene_view` with key `bible_preview`. The scene is built the same way `state::present::slide_scene` builds the Present preview: `resolve` under `app.live.theme`, static background. It shows slide `preview_slide` of the packed selection. `preview_slide` is a new `BibleState` field, reset to 0 on every selection change. With nothing selected it shows the whole chapter packed, slide one.
- Under it a row: `< 2 / 3 >` chip with prev and next, `Clear`, `Present`.
- In text mode a scrollable results list under the row. A result click jumps to the verse and selects it.

### 5.5 Present

`present` packs the selected verses with `fits_heuristic`, builds slides through `verse_slides`, keeps the presentation title as the full range, and calls `live.go`.

## 6. Themes editor, `sda-app/src/view/themes.rs` and `theme_form.rs`

Layout:

```
[ theme list 300 ] [ left column Fill ] [ right column Fill ]
                    preview card         background section
                    text section         transition section
                    reference section    verses section
                                         footer section
```

Both columns are independent `scrollable`s with the thin scrollbar. The preview frame uses `responsive` to take the column width at 16 by 9, capped at 540 high.

Preview sample: `ThemesState` gains `sample: Sample` with `Hymn | Bible`. The preview card shows a `Hymn | Bible` segmented control. `sample()` returns a two verse `SlideContent::Verses` for `Bible`, text from two new strings `theme_preview_verse_1` and `theme_preview_verse_2` with reference `theme_preview_reference`. Any `DraftEdit::Reference(..)`, `MinSize`, `ShowReference`, `ShowNumbers` sets `sample = Bible`. Theme list thumbnails keep the hymn sample.

Form: `text_section` is generalized to `text_style_section(app, title, style, inherited, wrap)` where `wrap` maps a `TextEdit` into a `DraftEdit`. The Text and Reference sections are two calls. The Reference section's `inherited` values are the resolved verse text values, so the Inherit toggles read correctly. Verses section drops Per slide and Position, gains a `Smallest text size` slider 16 to 60.

## 7. Present tab, `sda-app/src/view/present.rs`

`side_panel` keeps the theme card. The notes card becomes a history card: heading `History`, a scrollable list of `core.history`, each row number, title, and a relative time `3 min ago`, `2 h ago`, `Yesterday`, else date. Click calls `Msg::Select` after `st::hit_position` finds the hymn in the current hits, else sets the query to the hymn number and selects the first hit. Entries whose path is not indexed render muted and do nothing. The Recent block inside the search results is removed along with `RECENT_COUNT`. `no_notes` and `notes` strings are removed, `history` and the relative time strings are added.

## 8. Strings, `sda-core/src/i18n.rs`

Add in En, It, Ro: `theme_reference`, `theme_min_size`, `theme_sample_hymn`, `theme_sample_bible`, `theme_preview_verse_1`, `theme_preview_verse_2`, `theme_preview_reference`, `bible_mode_reference`, `bible_mode_text`, `bible_input_hint`, `bible_search_hint` reused, `history`, `time_just_now`, `time_minutes_ago`, `time_hours_ago`, `time_yesterday`. Remove `select_to_here`, `theme_per_slide`, `reference_position`, `notes`, `no_notes`.

## 9. Tests

- `sda-core/tests/bible.rs`: the parse table in 5.3. `pack_verses` with a `fits` that counts verses: one long verse alone, three short verses together, a split in the middle. `fits_heuristic` on a tiny box returns one verse per slide, on a huge box all verses. `verse_slides` reference text per slide.
- `sda-core/tests/theme.rs`: a 2.0 file and a 3.0 file without `reference` and with `per_slide` load, `reference` defaults, `min_size` is 28. `Frame::cut`.
- `sda-core/tests/resolve.rs`: two boxes with reference on, frames do not overlap for Top and Bottom, one box with reference off, reference inherits color and font from `text`.
- `sda-render/tests/golden_test.rs`: `verses_reference_band` with a red reference at the top in Lora.
- `sda-app/tests/bible.rs`: click selects, second click deselects, shift click selects a range, typing `mat 10 1` jumps, toggling text mode and typing fills results.
- `sda-app/tests/themes.rs`: editing a verse field flips the sample to Bible.
