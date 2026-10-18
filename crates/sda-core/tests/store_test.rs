#[test]
fn json_round_trip_and_missing_is_default() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.json");
    assert_eq!(
        sda_core::store::json_load::<Vec<u32>>(&p),
        Vec::<u32>::new()
    );
    sda_core::store::json_save(&p, &vec![1u32, 2]).unwrap();
    assert_eq!(sda_core::store::json_load::<Vec<u32>>(&p), vec![1, 2]);
    assert!(!dir.path().join("x.json.tmp").exists());
}

#[test]
fn json_save_creates_parent_dir_and_corrupt_file_is_default() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("deep").join("nested").join("x.json");
    sda_core::store::json_save(&p, &vec![7u32]).unwrap();
    assert_eq!(sda_core::store::json_load::<Vec<u32>>(&p), vec![7]);
    std::fs::write(&p, b"not json").unwrap();
    assert_eq!(
        sda_core::store::json_load::<Vec<u32>>(&p),
        Vec::<u32>::new()
    );
}
