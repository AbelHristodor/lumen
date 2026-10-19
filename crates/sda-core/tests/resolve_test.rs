use sda_core::model::*;
use sda_core::resolve::*;
use sda_core::theme::*;

fn slide(color: Option<Rgba>, size: Option<f32>, align: Option<HAlign>) -> SlideContent {
    SlideContent::Styled(Slide {
        background: Some(Rgba::new(10, 10, 10, 255)),
        notes: None,
        boxes: vec![TextBox {
            frame: Some(Frame {
                x: 0.1,
                y: 0.1,
                w: 0.8,
                h: 0.3,
            }),
            paragraphs: vec![Paragraph {
                align,
                line_spacing: None,
                runs: vec![Run {
                    text: "Hi".into(),
                    color,
                    size_pt: size,
                    ..Default::default()
                }],
            }],
        }],
    })
}

#[test]
fn inherit_takes_pptx_values() {
    let r = resolve(
        &Theme::default(),
        &slide(
            Some(Rgba::new(1, 2, 3, 255)),
            Some(40.0),
            Some(HAlign::Left),
        ),
        None,
    )
    .unwrap();
    let run = &r.boxes[0].lines[0].runs[0];
    assert_eq!(run.color, Rgba::new(1, 2, 3, 255));
    assert_eq!(run.size, TextSize::Pt(40.0));
    assert_eq!(r.boxes[0].lines[0].align, HAlign::Left);
    assert_eq!(
        r.boxes[0].frame,
        Frame {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.3
        }
    );
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
    t.text.color = Prop::Set(Rgba::new(9, 9, 9, 255));
    t.text.h_align = Prop::Set(HAlign::Right);
    t.text.margin = Prop::Set(0.2);
    let r = resolve(
        &t,
        &slide(Some(Rgba::new(1, 2, 3, 255)), None, Some(HAlign::Left)),
        None,
    )
    .unwrap();
    assert_eq!(r.boxes[0].lines[0].runs[0].color, Rgba::new(9, 9, 9, 255));
    assert_eq!(r.boxes[0].lines[0].align, HAlign::Right);
    assert_eq!(
        r.boxes[0].frame,
        Frame {
            x: 0.2,
            y: 0.2,
            w: 0.6,
            h: 0.6
        }
    ); // Set margin replaces pptx frame
}

#[test]
fn bitmap_resolves_to_none_and_footer_passes_through() {
    assert!(resolve(
        &Theme::default(),
        &SlideContent::Bitmap("/a.png".into()),
        None
    )
    .is_none());
    let r = resolve(
        &Theme::default(),
        &slide(None, None, None),
        Some("150 · Title"),
    )
    .unwrap();
    assert_eq!(r.footer.as_deref(), Some("150 · Title"));
}

#[test]
fn background_prefers_solid_theme_then_slide_then_black() {
    let styled = slide(None, None, None);
    assert_eq!(
        resolve(&Theme::default(), &styled, None)
            .unwrap()
            .background,
        Rgba::new(0, 0, 0, 255)
    );
    let t = Theme {
        background: Background::Image("/x.png".into()),
        ..Theme::default()
    };
    assert_eq!(
        resolve(&t, &styled, None).unwrap().background,
        Rgba::new(10, 10, 10, 255)
    );
    let bare = SlideContent::Styled(Slide {
        background: None,
        boxes: vec![],
        notes: None,
    });
    assert_eq!(
        resolve(&t, &bare, None).unwrap().background,
        Rgba::new(0, 0, 0, 255)
    );
}

#[test]
fn box_without_frame_gets_margin_frame() {
    let c = SlideContent::Styled(Slide {
        background: None,
        notes: None,
        boxes: vec![TextBox {
            frame: None,
            paragraphs: vec![],
        }],
    });
    let r = resolve(&Theme::default(), &c, None).unwrap();
    let f = r.boxes[0].frame;
    assert!((f.x - FALLBACK_MARGIN).abs() < 1e-6);
    assert!((f.w - (1.0 - 2.0 * FALLBACK_MARGIN)).abs() < 1e-6);
}

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
    let full = Frame {
        x: 0.06,
        y: 0.06,
        w: 0.88,
        h: 0.88,
    };
    assert_eq!(r.boxes[0].frame, full);
}

#[test]
fn frame_cut_splits_a_band_off_one_side() {
    let f = Frame {
        x: 0.1,
        y: 0.1,
        w: 0.8,
        h: 0.8,
    };
    let (strip, rest) = f.cut(VAlign::Top, 0.2);
    assert_eq!(
        strip,
        Frame {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.2
        }
    );
    assert!((rest.y - 0.3).abs() < 1e-6 && (rest.h - 0.6).abs() < 1e-6);
    let (strip, rest) = f.cut(VAlign::Bottom, 0.2);
    assert!((strip.y - 0.7).abs() < 1e-6 && (strip.h - 0.2).abs() < 1e-6);
    assert_eq!(
        rest,
        Frame {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.6
        }
    );
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
