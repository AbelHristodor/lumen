use sda_core::index::refresh_index;
use sda_core::model::{Hymn, Paragraph, Run, Slide, TextBox};
use sda_core::search::Searcher;
use std::path::{Path, PathBuf};

/// Build a hymn whose single slide holds one run per line.
fn hymn(number: &str, title: &str, lines: &[&str]) -> Hymn {
    let paragraphs = lines
        .iter()
        .map(|l| Paragraph {
            align: None,
            line_spacing: None,
            runs: vec![Run { text: (*l).into(), ..Default::default() }],
        })
        .collect();
    Hymn {
        number: Some(number.into()),
        title: title.into(),
        slides: vec![Slide {
            background: None,
            boxes: vec![TextBox { frame: None, paragraphs }],
            notes: None,
        }],
        path: PathBuf::from(format!("/x/{number}.pptx")),
        library: "L".into(),
        mtime: 0,
    }
}


fn searcher() -> Searcher {
    let entries = refresh_index(Path::new("tests/fixtures"), "test-lib", &[]);
    Searcher::new(entries)
}

#[test]
fn matches_without_diacritics() {
    let mut s = searcher();
    let hits = s.search("plecati");
    assert!(!hits.is_empty());
    assert_eq!(hits[0].entry.number.as_deref(), Some("1"));
}

#[test]
fn matches_by_number() {
    let mut s = searcher();
    let hits = s.search("150");
    assert_eq!(hits[0].entry.number.as_deref(), Some("150"));
}

#[test]
fn matches_body_text() {
    let mut s = searcher();
    let hits = s.search("Popoare");
    assert!(hits.iter().any(|h| h.entry.number.as_deref() == Some("1")));
}

#[test]
fn empty_query_returns_all_sorted_by_number() {
    let mut s = searcher();
    let hits = s.search("");
    assert!(hits.len() >= 2);
    assert_eq!(hits[0].entry.number.as_deref(), Some("1"));
}

#[test]
fn lettered_number_sorts_numerically_then_by_suffix() {
    use sda_core::model::MatchField;
    let mk = |n: &str| hymn(n, &format!("hymn {n}"), &[]);
    // Deliberately unsorted, with a letter-suffixed number among integers.
    let mut s = Searcher::new(vec![mk("665"), mk("664b"), mk("70"), mk("664"), mk("664a")]);
    let order: Vec<String> = s
        .search("")
        .into_iter()
        .map(|h| h.entry.number.clone().unwrap())
        .collect();
    assert_eq!(order, vec!["70", "664", "664a", "664b", "665"]);
    // Sanity: an empty-query hit reports the Number field.
    assert_eq!(s.search("").first().unwrap().field, MatchField::Number);
}

#[test]
fn label_is_number_title_library() {
    let s = searcher();
    let idx = (0..s.len()).find(|&i| s.entry(i).unwrap().number.as_deref() == Some("150")).unwrap();
    let label = s.label(idx).unwrap();
    assert!(label.starts_with("150  Cerul, pământul"), "{label}");
    assert!(label.ends_with("  · test-lib"), "{label}");
}

#[test]
fn hit_index_resolves_to_same_entry() {
    let mut s = searcher();
    let hits = s.search("cerb");
    let h = &hits[0];
    assert_eq!(h.entry.number.as_deref(), Some("356"));
    let idx = h.index;
    drop(hits);
    assert_eq!(s.entry(idx).unwrap().number.as_deref(), Some("356"));
}

#[test]
fn quoted_query_requires_exact_phrase() {
    let mut s = searcher();
    let hits = s.search("\"se vor topi\"");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entry.number.as_deref(), Some("150"));
    assert!(s.search("\"topi se vor\"").is_empty());
}

#[test]
fn exact_number_ranks_first() {
    let mut s = searcher();
    let hits = s.search("1");
    assert!(hits.len() >= 3, "150 and 664b also contain the digit");
    assert_eq!(hits[0].entry.number.as_deref(), Some("1"));
}

#[test]
fn body_search_uses_plain_text_of_styled_runs() {
    let mut s = Searcher::new(vec![hymn("9", "Nine", &["Lumina lumii", "a doua"]), hymn("10", "Ten", &["altceva"])]);
    let hits = s.search("lumina");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].entry.number.as_deref(), Some("9"));
}
