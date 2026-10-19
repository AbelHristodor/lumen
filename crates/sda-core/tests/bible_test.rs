use sda_core::bible::*;
use sda_core::i18n::Language;
use sda_core::theme::Theme;
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
    assert_eq!(
        packed.len(),
        5,
        "a verse that never fits still gets a slide"
    );
    let all_fit = |_: &[(u32, String)]| true;
    assert_eq!(pack_verses(&v, all_fit).len(), 1);
    assert!(pack_verses(&[], all_fit).is_empty());
}

#[test]
fn fits_heuristic_tiny_box_one_per_slide_huge_box_all_together() {
    let t = Theme::default();
    let v = vs(&[
        (1, "In the beginning God created the heaven and the earth."),
        (
            2,
            "And the earth was without form, and void; and darkness was upon the face of the deep.",
        ),
        (3, "And God said, Let there be light: and there was light."),
    ]);
    let tiny = pack_verses(&v, fits_heuristic(&t, (100.0, 30.0)));
    assert_eq!(tiny.len(), 3);
    let huge = pack_verses(&v, fits_heuristic(&t, (5000.0, 5000.0)));
    assert_eq!(huge.len(), 1);
    let real = pack_verses(&v, fits_heuristic(&t, sda_core::resolve::verse_box_pt(&t)));
    assert!((1..=3).contains(&real.len()));
}

#[test]
fn verse_slides_carry_their_own_reference() {
    let r = Reference {
        book: 42,
        chapter: 3,
        verses: Some((16, 21)),
    };
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

#[test]
fn parses_space_separated_chapter_and_verses() {
    let r = |s: &str| parse_reference(s, Language::En);
    let mat = |chapter, verses| {
        Some(Reference {
            book: 39,
            chapter,
            verses,
        })
    };
    assert_eq!(r("mat 10"), mat(10, None));
    assert_eq!(r("mat 10 1"), mat(10, Some((1, 1))));
    assert_eq!(r("matthew 10 1-5"), mat(10, Some((1, 5))));
    assert_eq!(r("mat 10:1"), mat(10, Some((1, 1))));
    assert_eq!(r("mat 10 1 5"), None);
    assert_eq!(
        r("1 sam 2 1"),
        Some(Reference {
            book: 8,
            chapter: 2,
            verses: Some((1, 1))
        })
    );
}
