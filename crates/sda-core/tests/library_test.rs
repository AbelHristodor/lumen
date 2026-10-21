use sda_core::library::Config;

#[test]
fn config_video_fields_default_and_round_trip() {
    let c = Config::default();
    assert_eq!(c.video_dir, None);
    assert_eq!(c.video_volume, 1.0);
    let toml = toml::to_string(&Config {
        video_dir: Some("/clips".into()),
        video_volume: 0.4,
        ..Config::default()
    })
    .unwrap();
    let back: Config = toml::from_str(&toml).unwrap();
    assert_eq!(back.video_dir.as_deref(), Some("/clips"));
    assert_eq!(back.video_volume, 0.4);
    // An old config without the keys still loads.
    let old: Config = toml::from_str("hymns_url = \"x\"\nlibraries = []\n").unwrap();
    assert_eq!(old.video_volume, 1.0);
}
