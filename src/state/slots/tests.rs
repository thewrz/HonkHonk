use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::tempdir;

use super::{MacroIdError, SlotContent, SlotMap};
use crate::state::error::ConfigError;

const SLOT_COUNT: usize = 20;

fn empty_slots() -> Vec<Value> {
    vec![Value::Null; SLOT_COUNT]
}

fn legacy_json(assignments: &[(usize, &str)]) -> String {
    let mut slots = empty_slots();
    for (index, path) in assignments {
        slots[*index] = json!(path);
    }
    serde_json::to_string(&slots).unwrap()
}

fn versioned_json(slots: Vec<Value>) -> String {
    serde_json::to_string(&json!({ "version": 1, "slots": slots })).unwrap()
}

#[test]
fn default_and_out_of_range_access_are_empty() {
    let mut slots = SlotMap::default();

    for index in 0..SLOT_COUNT as u8 {
        assert!(slots.content(index).is_none());
    }
    slots.set(20, PathBuf::from("/sounds/ignored.wav"));
    slots.clear(255);

    assert!(slots.content(20).is_none());
    assert!(slots.get(255).is_none());
    assert!(slots.macro_id(20).is_none());
}

#[test]
fn sound_and_macro_accessors_are_isolated_and_setters_replace_content() {
    let mut slots = SlotMap::default();
    let sound = PathBuf::from("/sounds/honk.wav");

    slots.set(4, sound.clone());
    assert_eq!(slots.get(4), Some(&sound));
    assert!(slots.macro_id(4).is_none());
    assert_eq!(slots.content(4), Some(&SlotContent::Sound(sound)));

    slots.set_macro(4, "macro-4").unwrap();
    assert!(slots.get(4).is_none());
    assert_eq!(slots.macro_id(4), Some("macro-4"));
    assert_eq!(
        slots.content(4),
        Some(&SlotContent::Macro("macro-4".to_owned()))
    );
}

#[test]
fn sound_and_macro_lookups_return_first_matching_slot_only() {
    let mut slots = SlotMap::default();
    let sound = PathBuf::from("/sounds/honk.wav");

    slots.set(2, sound.clone());
    slots.set(7, sound.clone());
    slots.set_macro(3, "/sounds/honk.wav").unwrap();
    slots.set_macro(8, "macro-a").unwrap();
    slots.set_macro(9, "macro-a").unwrap();

    assert_eq!(slots.slot_for(&sound), Some(2));
    assert_eq!(slots.slot_for_macro("macro-a"), Some(8));
    assert!(slots.slot_for(Path::new("macro-a")).is_none());
    assert!(slots.slot_for_macro("/not-assigned.wav").is_none());
}

#[test]
fn clear_removes_either_content_kind_and_out_of_range_set_macro_is_noop() {
    let mut slots = SlotMap::default();
    slots.set(1, PathBuf::from("/sounds/a.wav"));
    slots.set_macro(2, "macro-b").unwrap();

    slots.clear(1);
    slots.clear(2);
    slots.set_macro(200, "valid-but-ignored").unwrap();

    assert!(slots.content(1).is_none());
    assert!(slots.content(2).is_none());
    assert!(slots.slot_for_macro("valid-but-ignored").is_none());
}

#[test]
fn invalid_macro_ids_are_typed_errors_and_do_not_replace_content() {
    let mut slots = SlotMap::default();
    let original = PathBuf::from("/sounds/original.wav");
    slots.set(0, original.clone());

    assert_eq!(slots.set_macro(0, ""), Err(MacroIdError::Empty));
    assert_eq!(
        slots.set_macro(0, "x".repeat(256)),
        Err(MacroIdError::TooLong {
            length: 256,
            max: 255
        })
    );
    assert_eq!(
        slots.set_macro(0, "macro\nid"),
        Err(MacroIdError::ControlCharacter)
    );
    assert_eq!(slots.get(0), Some(&original));
}

#[test]
fn macro_id_limit_counts_utf8_bytes_and_accepts_non_hash_ids() {
    let mut slots = SlotMap::default();
    let id = "é".repeat(127);

    slots.set_macro(0, id.clone()).unwrap();

    assert_eq!(id.len(), 254);
    assert_eq!(slots.macro_id(0), Some(id.as_str()));
}

#[test]
fn legacy_array_loads_and_next_save_migrates_to_version_one() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("slots.json");
    std::fs::write(
        &path,
        legacy_json(&[(0, "/sounds/a.wav"), (19, "/sounds/z.flac")]),
    )
    .unwrap();

    let slots = SlotMap::load_from(&path);
    assert_eq!(slots.get(0), Some(&PathBuf::from("/sounds/a.wav")));
    assert_eq!(slots.get(19), Some(&PathBuf::from("/sounds/z.flac")));
    slots.save_to(&path).unwrap();

    let persisted: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(persisted["version"], 1);
    assert_eq!(persisted["slots"].as_array().unwrap().len(), SLOT_COUNT);
    assert_eq!(persisted["slots"][0], "/sounds/a.wav");
}

#[test]
fn versioned_mixed_content_round_trips_with_disjoint_json_shapes() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("slots.json");
    let mut slots = SlotMap::default();
    slots.set(0, PathBuf::from("/sounds/honk.wav"));
    slots.set_macro(1, "macro-custom-id").unwrap();

    slots.save_to(&path).unwrap();
    let persisted: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let values = persisted["slots"].as_array().unwrap();
    assert_eq!(values.len(), SLOT_COUNT);
    assert_eq!(values[0], "/sounds/honk.wav");
    assert_eq!(
        values[1],
        json!({ "type": "macro", "id": "macro-custom-id" })
    );

    let loaded = SlotMap::load_from(&path);
    assert_eq!(loaded, slots);
}

#[test]
fn malformed_or_unsupported_data_is_empty_and_read_protected() {
    let mut short = empty_slots();
    short.pop();
    let mut invalid_id = empty_slots();
    invalid_id[0] = json!({ "type": "macro", "id": "" });
    let cases = [
        "not json".to_owned(),
        serde_json::to_string(&short).unwrap(),
        versioned_json(short),
        json!({ "version": 999, "slots": empty_slots() }).to_string(),
        json!({ "version": 1, "slots": empty_slots(), "extra": true }).to_string(),
        versioned_json({
            let mut values = empty_slots();
            values[0] = json!({ "type": "playlist", "id": "x" });
            values
        }),
        versioned_json({
            let mut values = empty_slots();
            values[0] = json!({ "type": "macro", "id": "x", "extra": true });
            values
        }),
        versioned_json(invalid_id),
    ];

    for (index, original) in cases.into_iter().enumerate() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(format!("invalid-{index}.json"));
        std::fs::write(&path, &original).unwrap();

        let loaded = SlotMap::load_from(&path);
        assert!(loaded.content(0).is_none(), "case {index}");
        assert!(matches!(
            loaded.save_to(&path),
            Err(ConfigError::UnsafeSlotsOverwrite { .. })
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
}

#[test]
fn missing_file_is_a_writable_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("missing").join("slots.json");
    let mut slots = SlotMap::load_from(&path);
    slots.set_macro(19, "created-later").unwrap();

    slots.save_to(&path).unwrap();

    assert_eq!(
        SlotMap::load_from(&path).macro_id(19),
        Some("created-later")
    );
}

#[test]
fn unreadable_source_is_read_protected() {
    let dir = tempdir().unwrap();
    let loaded = SlotMap::load_from(dir.path());

    assert!(matches!(
        loaded.save_to(dir.path()),
        Err(ConfigError::UnsafeSlotsOverwrite { .. })
    ));
}
