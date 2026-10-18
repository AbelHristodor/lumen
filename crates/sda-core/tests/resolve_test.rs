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

#[test]
fn verses_reference_at_top_with_small_fixed_size_and_no_numbers() {
    let mut t = Theme::default();
    t.text.size = Prop::Set(TextSize::Pt(50.0));
    t.verses.reference_at = RefPos::Top;
    t.verses.show_numbers = false;
    let c = SlideContent::Verses { reference: "John 3:16".into(), verses: vec![(16, "For God".into())] };
    let r = resolve(&t, &c, None).unwrap();
    let first = &r.boxes[0].lines[0];
    assert_eq!(first.runs[0].text, "John 3:16");
    assert!(matches!(first.runs[0].size, TextSize::Pt(p) if (p - 30.0).abs() < 1e-4));
    assert_eq!(r.boxes[0].lines[1].runs[0].text, "For God");
    assert_eq!(r.boxes[0].lines[1].runs[0].size, TextSize::Pt(50.0));
}

#[test]
fn verses_without_reference_have_only_verse_lines() {
    let mut t = Theme::default();
    t.verses.show_reference = false;
    let c = SlideContent::Verses { reference: "John 3:16".into(), verses: vec![(16, "a".into()), (17, "b".into())] };
    let r = resolve(&t, &c, None).unwrap();
    assert_eq!(r.boxes[0].lines.len(), 2);
    assert_eq!(r.boxes[0].lines[1].runs[0].text, "17 b");
}

#[test]
fn background_prefers_solid_theme_then_slide_then_black() {
    let styled = slide(None, None, None);
    assert_eq!(resolve(&Theme::default(), &styled, None).unwrap().background, Rgba::new(0, 0, 0, 255));
    let t = Theme { background: Background::Image("/x.png".into()), ..Theme::default() };
    assert_eq!(resolve(&t, &styled, None).unwrap().background, Rgba::new(10, 10, 10, 255));
    let bare = SlideContent::Styled(Slide { background: None, boxes: vec![], notes: None });
    assert_eq!(resolve(&t, &bare, None).unwrap().background, Rgba::new(0, 0, 0, 255));
}

#[test]
fn box_without_frame_gets_margin_frame() {
    let c = SlideContent::Styled(Slide { background: None, notes: None, boxes: vec![TextBox { frame: None, paragraphs: vec![] }] });
    let r = resolve(&Theme::default(), &c, None).unwrap();
    let f = r.boxes[0].frame;
    assert!((f.x - FALLBACK_MARGIN).abs() < 1e-6);
    assert!((f.w - (1.0 - 2.0 * FALLBACK_MARGIN)).abs() < 1e-6);
}
