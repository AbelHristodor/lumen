use sda_core::bible::*;
use sda_core::i18n::Language;
use std::path::Path;

fn mini() -> Bible {
    load(Path::new("tests/fixtures/bible_mini.json")).unwrap()
}

#[test]
fn loads_66_books() {
    assert_eq!(mini().books.len(), 66);
    assert_eq!(mini().code, "bible_mini");
}

#[test]
fn parses_references_in_three_languages() {
    let r = parse_reference("John 3:16-18", Language::En).unwrap();
    assert_eq!((r.book, r.chapter, r.verses), (42, 3, Some((16, 18))));
    assert_eq!(
        parse_reference("Ioan 3,16", Language::Ro).unwrap().verses,
        Some((16, 16))
    );
    assert_eq!(
        parse_reference("Giovanni 3", Language::It).unwrap().verses,
        None
    );
    assert_eq!(parse_reference("1 Sam 2:1", Language::En).unwrap().book, 8);
    assert_eq!(parse_reference("gen 1:1", Language::En).unwrap().book, 0);
    assert!(parse_reference("Nowhere 1:1", Language::En).is_none());
}

#[test]
fn formats_reference_localized() {
    let r = Reference {
        book: 42,
        chapter: 3,
        verses: Some((16, 18)),
    };
    assert_eq!(format_reference(&r, Language::En), "John 3:16-18");
    assert_eq!(format_reference(&r, Language::Ro), "Ioan 3:16-18");
    assert_eq!(
        format_reference(
            &Reference {
                book: 0,
                chapter: 1,
                verses: None
            },
            Language::It
        ),
        "Genesi 1"
    );
}

#[test]
fn verses_returns_range_clamped() {
    let v = verses(
        &mini(),
        &Reference {
            book: 42,
            chapter: 3,
            verses: Some((17, 99)),
        },
    );
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].0, 17);
}

#[test]
fn search_folds_accents_and_limits() {
    let hits = search(&mini(), "god so LOVED", 10);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0.verses, Some((16, 16)));
    assert_eq!(search(&mini(), "v", 3).len(), 3);
}

#[test]
fn verse_slides_groups_per_slide() {
    let v = vec![
        (16, "a".to_string()),
        (17, "b".to_string()),
        (18, "c".to_string()),
    ];
    let s = verse_slides(&v, "John 3:16-18", 2);
    assert_eq!(s.len(), 2);
    match &s[1] {
        sda_core::resolve::SlideContent::Verses { verses, reference } => {
            assert_eq!(verses.len(), 1);
            assert_eq!(reference, "John 3:16-18");
        }
        _ => panic!(),
    }
}

#[test]
fn index_parses_and_urls_are_pinned() {
    // Real repo shape: the abbreviation already carries the language prefix.
    let json = r#"[{"language":"English","versions":[{"name":"King James Version","abbreviation":"en_kjv"}]},{"language":"Romanian","versions":[{"name":"VDC","abbreviation":"ro_vdc"}]}]"#;
    let v = parse_index(json).unwrap();
    assert_eq!(v[0].code, "en_kjv");
    assert_eq!(v[0].abbreviation, "kjv");
    assert_eq!(v[0].public_domain, Some(true));
    assert_eq!(v[1].code, "ro_vdc");
    assert_eq!(v[1].abbreviation, "vdc");
    assert_eq!(v[1].public_domain, Some(false));
    assert!(version_url("en_kjv").contains(PIN));
}

#[test]
fn index_parses_bare_abbreviations() {
    let json = r#"[{"language":"English","versions":[{"name":"King James Version","abbreviation":"kjv"}]},{"language":"Romanian","versions":[{"name":"VDC","abbreviation":"vdc"}]}]"#;
    let v = parse_index(json).unwrap();
    assert_eq!(v[0].code, "en_kjv");
    assert_eq!(v[0].abbreviation, "kjv");
    assert_eq!(v[1].code, "ro_vdc");
    assert_eq!(v[1].abbreviation, "vdc");
}
