use sda_core::theme::{Font, FontSize, HAlign, Rgba, Theme, VAlign};

#[test]
fn theme_json_round_trips() {
    let t = Theme::default();
    assert_eq!(Theme::from_json(&t.to_json().unwrap()).unwrap(), t);
}

#[test]
fn default_theme_matches_spec() {
    let t = Theme::default();
    assert_eq!(t.name, "Default");
    assert_eq!(t.font, Font::Inter);
    assert_eq!(t.size, FontSize::Auto);
    assert!(t.bold);
    assert_eq!(t.text_color, Rgba::new(255, 255, 255, 255));
    assert_eq!(t.bg_color, Rgba::new(11, 31, 58, 255));
    assert_eq!(t.h_align, HAlign::Center);
    assert_eq!(t.v_align, VAlign::Middle);
    assert_eq!(t.line_spacing, 1.3);
    assert_eq!(t.margin, 0.06);
    assert!(!t.footer);
}

#[test]
fn custom_theme_round_trips_all_fields() {
    let t = Theme {
        name: "Christmas".into(),
        font: Font::Lora,
        size: FontSize::Pt(48.0),
        bold: false,
        text_color: Rgba::new(1, 2, 3, 255),
        bg_color: Rgba::new(4, 5, 6, 255),
        h_align: HAlign::Left,
        v_align: VAlign::Bottom,
        line_spacing: 1.1,
        margin: 0.1,
        footer: true,
    };
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
    assert_eq!(t.name, "Old");
    assert_eq!(t.font, Font::Inter);
    assert_eq!(t.size, FontSize::Auto);
    assert!(t.footer, "old footer.show must map to the new bool");
}

#[test]
fn missing_fields_take_defaults() {
    let t = Theme::from_json(r#"{"name":"Bare"}"#).unwrap();
    assert_eq!(t.name, "Bare");
    assert_eq!(t.bg_color, Theme::default().bg_color);
}

#[test]
fn cache_key_changes_with_any_field() {
    let a = Theme::default();
    let mut b = a.clone();
    b.margin = 0.07;
    assert_ne!(a.cache_key(), b.cache_key());
    assert_eq!(a.cache_key(), Theme::default().cache_key());
}

#[test]
fn font_labels_and_all() {
    assert_eq!(Font::all().len(), 4);
    assert_eq!(Font::NotoSerif.label(), "Noto Serif");
}

#[test]
fn rgba_hex_round_trips() {
    let c = Rgba::new(11, 31, 58, 255);
    assert_eq!(c.to_hex(), "#0B1F3A");
    assert_eq!(Rgba::from_hex("#0B1F3A"), Some(Rgba::new(11, 31, 58, 255)));
}

#[test]
fn rgba_from_hex_accepts_lowercase_and_no_hash() {
    assert_eq!(Rgba::from_hex("ffffff"), Some(Rgba::new(255, 255, 255, 255)));
    assert_eq!(Rgba::from_hex("#abcdef"), Some(Rgba::new(0xab, 0xcd, 0xef, 255)));
}

#[test]
fn rgba_from_hex_rejects_invalid() {
    assert_eq!(Rgba::from_hex("#FF"), None);
    assert_eq!(Rgba::from_hex("#GGGGGG"), None);
    assert_eq!(Rgba::from_hex(""), None);
    assert_eq!(Rgba::from_hex("#1234567"), None);
}
