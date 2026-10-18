use sda_core::model::Slide;
use sda_core::present::{Presentation, Source};
use sda_core::resolve::SlideContent;

fn slides() -> Vec<SlideContent> {
    ["title", "verse 1", "verse 2"]
        .iter()
        .map(|_| {
            SlideContent::Styled(Slide {
                background: None,
                boxes: vec![],
                notes: None,
            })
        })
        .collect()
}

fn loaded() -> Presentation {
    let mut p = Presentation {
        blank: true,
        ..Presentation::default()
    };
    p.load(
        "T".into(),
        Some("150".into()),
        slides(),
        Source::Hymn("/a.pptx".into()),
    );
    p
}

#[test]
fn load_resets_to_first_slide_unblanked() {
    let p = loaded();
    assert_eq!(p.index, 0);
    assert!(!p.blank);
    assert_eq!(p.title, "T");
    assert_eq!(p.subtitle.as_deref(), Some("150"));
    assert_eq!(p.source, Source::Hymn("/a.pptx".into()));
    assert!(p.current().is_some());
    assert_eq!(p.slide_count(), 3);
}

#[test]
fn next_and_prev_clamp_at_bounds() {
    let mut p = loaded();
    p.prev();
    assert_eq!(p.index, 0);
    p.next();
    p.next();
    assert_eq!(p.index, 2);
    p.next();
    assert_eq!(p.index, 2);
    p.prev();
    assert_eq!(p.index, 1);
}

#[test]
fn jump_is_one_based_and_ignores_out_of_range() {
    let mut p = loaded();
    p.jump(3);
    assert_eq!(p.index, 2);
    p.jump(0);
    assert_eq!(p.index, 2);
    p.jump(4);
    assert_eq!(p.index, 2);
    p.jump(1);
    assert_eq!(p.index, 0);
}

#[test]
fn blank_toggles() {
    let mut p = loaded();
    p.toggle_blank();
    assert!(p.blank);
    p.toggle_blank();
    assert!(!p.blank);
}

#[test]
fn empty_presentation_is_not_loaded() {
    let p = Presentation::default();
    assert!(!p.is_loaded());
    assert!(p.current().is_none());
    assert_eq!(p.slide_count(), 0);
    assert_eq!(p.source, Source::None);
    assert!(loaded().is_loaded());
}
