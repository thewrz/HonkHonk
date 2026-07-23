use super::*;
use tempfile::tempdir;

#[test]
fn default_meta_is_not_favorite() {
    let store = SoundMetaStore::default();
    assert!(!store.is_favorite("any-id"));
}

#[test]
fn default_volume_is_one() {
    let store = SoundMetaStore::default();
    let eps = f32::EPSILON;
    assert!((store.volume_for("any-id") - 1.0).abs() < eps);
}

#[test]
fn toggle_favorite_sets_true_then_false() {
    let mut store = SoundMetaStore::default();
    assert!(store.toggle_favorite("id1"));
    assert!(!store.toggle_favorite("id1"));
}

#[test]
fn set_volume_clamps_to_range() {
    let mut store = SoundMetaStore::default();
    store.set_volume("id1", 3.0);
    let eps = f32::EPSILON;
    assert!((store.volume_for("id1") - 2.0).abs() < eps);
    store.set_volume("id1", -0.5);
    assert!((store.volume_for("id1") - 0.0).abs() < eps);
}

#[test]
fn set_volume_in_range_is_preserved() {
    let mut store = SoundMetaStore::default();
    store.set_volume("id1", 1.5);
    let eps = 1e-5_f32;
    assert!((store.volume_for("id1") - 1.5).abs() < eps);
}

#[test]
fn set_cleans_up_default_entries() {
    let mut store = SoundMetaStore::default();
    store.set_volume("id1", 1.5);
    store.set("id1".to_owned(), SoundMeta::default());
    assert!(
        store.custom.is_empty(),
        "default meta should be pruned from map"
    );
}

#[test]
fn set_display_name_stores_override() {
    let mut store = SoundMetaStore::default();
    store.set_display_name("id1", Some("My Honk".to_owned()));
    assert_eq!(store.get("id1").display_name.as_deref(), Some("My Honk"));
}

#[test]
fn set_display_name_none_clears_override() {
    let mut store = SoundMetaStore::default();
    store.set_display_name("id1", Some("Override".to_owned()));
    store.set_display_name("id1", None);
    assert!(store.get("id1").display_name.is_none());
}

#[test]
fn save_and_load_round_trips() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("meta.json");

    let mut store = SoundMetaStore::default();
    store.toggle_favorite("abc");
    store.set_volume("abc", 1.25);
    store.save_to(&path).unwrap();

    let loaded = SoundMetaStore::load_from(&path);
    assert!(loaded.is_favorite("abc"));
    let eps = 1e-5_f32;
    assert!((loaded.volume_for("abc") - 1.25).abs() < eps);
}

#[test]
fn load_from_missing_file_returns_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.json");
    let store = SoundMetaStore::load_from(&path);
    assert!(!store.is_favorite("any"));
}

#[test]
fn load_from_corrupt_file_returns_empty() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, b"not json!!!").unwrap();
    let store = SoundMetaStore::load_from(&path);
    assert!(!store.is_favorite("any"));
}

#[test]
fn corrupt_file_cannot_be_overwritten() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, b"not json!!!").unwrap();
    let store = SoundMetaStore::load_from(&path);

    assert!(store.save_to(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"not json!!!");
}

#[test]
fn future_version_file_cannot_be_downgraded() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("future.json");
    let future = r#"{"version":999,"custom":{},"added":{"abc":42}}"#;
    std::fs::write(&path, future).unwrap();
    let store = SoundMetaStore::load_from(&path);

    assert!(store.save_to(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), future);
}

#[test]
fn malformed_envelope_without_version_cannot_be_overwritten() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("malformed-envelope.json");
    let malformed = r#"{"custom":{"abc":{"favorite":true}},"added":{"abc":42}}"#;
    std::fs::write(&path, malformed).unwrap();
    let store = SoundMetaStore::load_from(&path);

    assert!(store.save_to(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
}

#[test]
fn is_default_detects_all_fields_at_default() {
    assert!(SoundMeta::default().is_default());
    assert!(
        !SoundMeta {
            favorite: true,
            ..Default::default()
        }
        .is_default()
    );
}

#[test]
fn reconcile_added_stamps_new_ids_once() {
    let mut store = SoundMetaStore::default();

    assert!(store.reconcile_added(["first", "second"], 1_000, true));
    assert_eq!(store.added_ms("first"), Some(1_000));
    assert_eq!(store.added_ms("second"), Some(1_000));
    assert!(!store.reconcile_added(["first", "second"], 2_000, true));
    assert_eq!(store.added_ms("first"), Some(1_000));
}

#[test]
fn complete_reconcile_prunes_unseen_ids() {
    let mut store = SoundMetaStore::default();
    store.reconcile_added(["kept", "removed"], 1_000, true);

    assert!(store.reconcile_added(["kept"], 2_000, true));
    assert_eq!(store.added_ms("kept"), Some(1_000));
    assert_eq!(store.added_ms("removed"), None);
}

#[test]
fn partial_reconcile_preserves_unseen_ids() {
    let mut store = SoundMetaStore::default();
    store.reconcile_added(["observed", "temporarily-missing"], 1_000, true);

    assert!(!store.reconcile_added(["observed"], 2_000, false));
    assert_eq!(store.added_ms("temporarily-missing"), Some(1_000));
}

#[test]
fn load_from_accepts_legacy_top_level_metadata_map() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("legacy.json");
    std::fs::write(
        &path,
        r#"{"abc":{"favorite":true,"volume":1.25,"display_name":"Honk","assigned_graphic":"ignored.png"}}"#,
    )
    .unwrap();

    let store = SoundMetaStore::load_from(&path);

    assert!(store.is_favorite("abc"));
    assert_eq!(store.get("abc").display_name.as_deref(), Some("Honk"));
    assert!(store.assigned_graphic("abc").is_none());
    assert_eq!(store.added_ms("abc"), None);
    store.save_to(&path).unwrap();
    let migrated = SoundMetaStore::load_from(&path);
    assert!(migrated.is_favorite("abc"));
    assert!(migrated.assigned_graphic("abc").is_none());
}

#[test]
fn versioned_store_round_trips_added_timestamps() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("meta.json");
    let mut store = SoundMetaStore::default();
    store.toggle_favorite("abc");
    store.reconcile_added(["abc"], 4_242, true);

    store.save_to(&path).unwrap();
    let loaded = SoundMetaStore::load_from(&path);

    assert!(loaded.is_favorite("abc"));
    assert_eq!(loaded.added_ms("abc"), Some(4_242));
    let json = std::fs::read_to_string(path).unwrap();
    assert!(json.contains("\"version\""));
    assert!(json.contains("\"custom\""));
    assert!(json.contains("\"added\""));
}

#[test]
fn first_seen_timestamps_do_not_create_custom_metadata() {
    let mut store = SoundMetaStore::default();

    store.reconcile_added(["abc"], 1_000, true);

    assert!(store.get_ref("abc").is_none());
}

#[test]
fn assigned_graphic_can_be_set_and_cleared_with_default_pruning() {
    let mut store = SoundMetaStore::default();
    let graphic = GraphicAssetRef::new("airhorn.webp").unwrap();

    store.set_assigned_graphic("abc", graphic);
    assert_eq!(
        store.assigned_graphic("abc").map(GraphicAssetRef::as_str),
        Some("airhorn.webp")
    );

    store.clear_assigned_graphic("abc");
    assert!(store.assigned_graphic("abc").is_none());
    assert!(store.get_ref("abc").is_none());
}

#[test]
fn graphic_reference_accepts_one_unicode_filename_component() {
    let graphic = GraphicAssetRef::new("hönk tile.png").unwrap();
    let max_length = GraphicAssetRef::new("a".repeat(255)).unwrap();

    assert_eq!(graphic.as_str(), "hönk tile.png");
    assert_eq!(max_length.as_str().len(), 255);
}

#[test]
fn graphic_reference_rejects_unsafe_filenames() {
    let too_long = "a".repeat(256);
    assert_eq!(
        GraphicAssetRef::new("").unwrap_err(),
        GraphicRefError::Empty
    );
    assert_eq!(
        GraphicAssetRef::new(&too_long).unwrap_err(),
        GraphicRefError::TooLong
    );
    let invalid = [
        ".",
        "..",
        "/absolute.png",
        "../escape.png",
        "folder/file.png",
        r"folder\file.png",
        "line\nbreak.png",
    ];

    for filename in invalid {
        assert!(
            GraphicAssetRef::new(filename).is_err(),
            "{filename:?} should be rejected"
        );
    }
}

#[test]
fn clearing_graphic_preserves_other_custom_metadata() {
    let mut store = SoundMetaStore::default();
    store.toggle_favorite("abc");
    store.set_assigned_graphic("abc", GraphicAssetRef::new("airhorn.webp").unwrap());

    store.clear_assigned_graphic("abc");

    assert!(store.is_favorite("abc"));
    assert!(store.assigned_graphic("abc").is_none());
    assert!(store.get_ref("abc").is_some());
}

#[test]
fn assigned_graphic_round_trips_in_v2_envelope() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("meta.json");
    let mut store = SoundMetaStore::default();
    store.set_assigned_graphic("abc", GraphicAssetRef::new("airhorn.webp").unwrap());

    store.save_to(&path).unwrap();
    let loaded = SoundMetaStore::load_from(&path);

    assert_eq!(
        loaded.assigned_graphic("abc").map(GraphicAssetRef::as_str),
        Some("airhorn.webp")
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(json["version"], 2);
}

#[test]
fn v1_envelope_migrates_without_assigned_graphic() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("v1.json");
    let v1 = r#"{"version":1,"custom":{"abc":{"favorite":true,"volume":1.25,"display_name":"Honk","assigned_graphic":"ignored.png"}},"added":{"abc":42}}"#;
    std::fs::write(&path, v1).unwrap();

    let store = SoundMetaStore::load_from(&path);

    assert!(store.is_favorite("abc"));
    assert!(store.assigned_graphic("abc").is_none());
    assert_eq!(store.added_ms("abc"), Some(42));
    store.save_to(&path).unwrap();
    let migrated: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(migrated["version"], 2);
}

#[test]
fn explicit_older_version_is_read_protected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("v0.json");
    let v0 = r#"{"version":0,"custom":{},"added":{}}"#;
    std::fs::write(&path, v0).unwrap();
    let store = SoundMetaStore::load_from(&path);

    assert!(store.save_to(&path).is_err());
    assert_eq!(std::fs::read_to_string(path).unwrap(), v0);
}

#[test]
fn invalid_persisted_graphic_is_read_protected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("invalid-graphic.json");

    for reference in ["/tmp/image.png", "../image.png", "folder/image.png"] {
        let json = format!(
            r#"{{"version":2,"custom":{{"abc":{{"assigned_graphic":{reference:?}}}}},"added":{{}}}}"#
        );
        std::fs::write(&path, &json).unwrap();
        let store = SoundMetaStore::load_from(&path);

        assert!(store.assigned_graphic("abc").is_none());
        assert!(store.save_to(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), json);
    }
}
