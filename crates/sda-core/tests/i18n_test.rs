use sda_core::i18n::{Language, Strings};

#[test]
fn every_language_has_no_empty_strings() {
    for lang in Language::all() {
        let s = Strings::for_language(lang);
        for (i, field) in s.as_fields().iter().enumerate() {
            assert!(
                !field.trim().is_empty(),
                "language {:?} has an empty string at field index {}",
                lang,
                i
            );
        }
    }
}

#[test]
fn format_strings_keep_their_placeholders() {
    // Guard: placeholders must survive translation in all languages.
    for lang in Language::all() {
        let s = Strings::for_language(lang);
        assert!(s.status_synced_fmt.contains("{}"), "{:?} synced fmt", lang);
        assert!(s.update_staged_fmt.contains("{}"), "{:?} staged fmt", lang);
        assert!(s.update_ready_fmt.contains("{}"), "{:?} ready fmt", lang);
        assert_eq!(
            s.slide_counter_fmt.matches("{}").count(),
            3,
            "{:?} slide counter needs 3 placeholders",
            lang
        );
    }
}

#[test]
fn italian_and_romanian_are_translated_not_copied() {
    let en = Strings::for_language(Language::En);
    for lang in [Language::It, Language::Ro] {
        let s = Strings::for_language(lang);
        assert_ne!(s.go_live, en.go_live, "{lang:?}");
        assert_ne!(s.projector_on, en.projector_on, "{lang:?}");
        assert_ne!(s.nothing_live, en.nothing_live, "{lang:?}");
        assert_ne!(s.theme_footer_label, en.theme_footer_label, "{lang:?}");
        assert_ne!(s.tab_bible, en.tab_bible, "{lang:?}");
        assert_ne!(
            s.bible_present_selected, en.bible_present_selected,
            "{lang:?}"
        );
        assert_ne!(s.projector_failed, en.projector_failed, "{lang:?}");
    }
}

#[test]
fn field_count_matches_struct() {
    assert_eq!(Strings::for_language(Language::En).as_fields().len(), 178);
}

#[test]
fn review_fixes_landed() {
    let it = Strings::for_language(Language::It);
    let ro = Strings::for_language(Language::Ro);
    assert_eq!(it.theme_easing, "Curva");
    assert_eq!(ro.theme_easing, "Curbă");
    assert_eq!(it.theme_bg_solid, "Tinta unita");
    for lang in Language::all() {
        let s = Strings::for_language(lang);
        assert!(!s.library_unavailable_suffix.contains('('), "{lang:?}");
        assert!(!s.library_unavailable_suffix.contains(')'), "{lang:?}");
        assert!(!s.font_licenses.is_empty());
        assert!(!s.transition_blur.is_empty());
    }
}

#[test]
fn romanian_uses_comma_below_not_cedilla() {
    for lang in Language::all() {
        let s = Strings::for_language(lang);
        for (i, field) in s.as_fields().iter().enumerate() {
            assert!(
                !field.contains(['\u{015F}', '\u{0163}', '\u{015E}', '\u{0162}']),
                "{lang:?} field {i} uses a cedilla: {field}"
            );
        }
    }
}
