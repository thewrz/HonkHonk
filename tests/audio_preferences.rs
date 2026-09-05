use honkhonk::audio::processing::{OutputMode, SoundProcessing};
use honkhonk::state::{SoundMeta, SoundMetaStore};

#[test]
fn fingerprint_preferences_survive_rename_and_restart_without_copying_tags() {
    let mut store = SoundMetaStore::default();
    store.bind_fingerprint("old", "abc");
    store.set(
        "old".into(),
        SoundMeta {
            volume: 1.5,
            tags: vec!["Original".into()],
            display_name: Some("Old name".into()),
            processing: SoundProcessing {
                pan: 0.6,
                output: OutputMode::Stereo,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("meta.json");
    store.save_to(&path).unwrap();
    let mut loaded = SoundMetaStore::load_from(&path);
    loaded.set_tags("new", vec!["New".into()]);
    loaded.bind_fingerprint("new", "abc");
    let moved = loaded.get("new");
    assert_eq!(moved.volume, 1.5);
    assert_eq!(moved.processing.pan, 0.6);
    assert_eq!(moved.tags, ["New"]);
    assert_eq!(moved.display_name, None);
    loaded.set_volume("new", 0.5);
    assert_eq!(loaded.volume_for("old"), 0.5);
    assert_eq!(loaded.get("old").tags, ["Original"]);
    loaded.bind_fingerprint("different", "def");
    assert_eq!(loaded.volume_for("different"), 1.0);
}

#[test]
fn force_mono_stereo_and_pan_have_defined_channel_behavior() {
    use honkhonk::audio::processing::{convert_channels, pan};
    let source = std::sync::Arc::new(vec![0.6, 0.2, -0.2, 0.6]);
    let (mono, channels) = convert_channels(&source, 2, OutputMode::Mono);
    assert_eq!(channels, 1);
    assert!((mono[0] - 0.4).abs() < 0.0001);
    let (stereo, channels) = convert_channels(&mono, 1, OutputMode::Stereo);
    assert_eq!(channels, 2);
    let mut panned = stereo.as_ref().clone();
    pan(&mut panned, 2, 1.0);
    assert_eq!(panned[0], 0.0);
    assert_eq!(panned[1], mono[0]);
    assert_eq!(source.as_slice(), &[0.6, 0.2, -0.2, 0.6]);
}
