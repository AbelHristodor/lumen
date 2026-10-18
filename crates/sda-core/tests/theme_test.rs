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

#[test]
fn background_animation_and_labels() {
    assert!(!Background::Solid(Rgba::new(0, 0, 0, 255)).is_animated());
    assert!(!Background::Gradient { stops: vec![], angle: 0.0, drift: 0.0 }.is_animated());
    assert!(Background::Gradient { stops: vec![], angle: 0.0, drift: 0.1 }.is_animated());
    assert!(Background::Particles { base: Rgba::new(0, 0, 0, 255), color: Rgba::new(0, 0, 0, 255), count: 1, speed: 1.0 }.is_animated());
    assert!(!Background::Image("/x.png".into()).is_animated());
    assert!(Background::Video { path: "/x.mp4".into() }.is_animated());
    assert_eq!(Background::Image("/x.png".into()).kind_label(), "Image");
}

#[test]
fn font_families_and_labels() {
    assert_eq!(BundledFont::all().len(), 4);
    assert_eq!(BundledFont::NotoSerif.label(), "Noto Serif");
    assert_eq!(Font::Bundled(BundledFont::RobotoSlab).family(), "Roboto Slab");
    assert_eq!(Font::System("Georgia".into()).family(), "Georgia");
}

#[test]
fn rgba_hex_round_trips() {
    let c = Rgba::new(11, 31, 58, 255);
    assert_eq!(c.to_hex(), "#0B1F3A");
    assert_eq!(Rgba::from_hex("#0B1F3A"), Some(Rgba::new(11, 31, 58, 255)));
    assert_eq!(Rgba::from_hex("ffffff"), Some(Rgba::new(255, 255, 255, 255)));
    assert_eq!(Rgba::from_hex("#FF"), None);
    assert_eq!(Rgba::from_hex("#GGGGGG"), None);
}
