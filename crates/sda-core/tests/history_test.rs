use sda_core::history::*;

fn e(p: &str, t: i64) -> HistoryEntry {
    HistoryEntry {
        path: p.into(),
        number: None,
        title: p.into(),
        opened_at: t,
    }
}

#[test]
fn record_dedupes_by_path_newest_first_and_caps() {
    let mut l = Vec::new();
    for i in 0..60 {
        record(&mut l, e(&format!("/{i}"), i));
    }
    assert_eq!(l.len(), 50);
    assert_eq!(l[0].path.to_str(), Some("/59"));
    record(&mut l, e("/30", 100));
    assert_eq!(l[0].path.to_str(), Some("/30"));
    assert_eq!(
        l.iter().filter(|x| x.path.to_str() == Some("/30")).count(),
        1
    );
    assert_eq!(recent(&l, 3).len(), 3);
}

#[test]
fn recent_clamps_to_list_length_and_entry_round_trips_json() {
    let mut l = Vec::new();
    record(&mut l, e("/a", 1));
    assert_eq!(recent(&l, 10).len(), 1);
    assert!(recent(&[], 3).is_empty());
    let entry = HistoryEntry {
        path: "/x".into(),
        number: Some("664b".into()),
        title: "T".into(),
        opened_at: 5,
    };
    let text = serde_json::to_string(&entry).unwrap();
    let back: HistoryEntry = serde_json::from_str(&text).unwrap();
    assert_eq!(back, entry);
}
