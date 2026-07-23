use std::path::PathBuf;

use super::*;
use crate::app::Message;
use crate::state::{AudioFormat, SortPref};

fn sound(id: &str, name: &str, path: &str) -> SoundEntry {
    SoundEntry {
        id: id.into(),
        name: name.into(),
        path: PathBuf::from(path),
        format: AudioFormat::Wav,
        duration_ms: None,
        modified_ms: None,
        category: "Other".into(),
    }
}

fn with_timing(
    mut sound: SoundEntry,
    duration_ms: Option<u64>,
    modified_ms: Option<u64>,
) -> SoundEntry {
    sound.duration_ms = duration_ms;
    sound.modified_ms = modified_ms;
    sound
}

fn with_category(mut sound: SoundEntry, category: &str) -> SoundEntry {
    sound.category = category.into();
    sound
}

fn sorted_ids<'a>(
    sounds: &'a [SoundEntry],
    state: SoundSortState,
    metadata: &SoundMetaStore,
) -> Vec<&'a str> {
    sorted_sound_indices(sounds, (0..sounds.len()).collect(), state, metadata)
        .into_iter()
        .map(|index| sounds[index].id.as_str())
        .collect()
}

#[test]
fn name_sort_is_case_insensitive_and_uses_path_then_id_ties() {
    let entries = [
        sound("z", "Zulu", "/sounds/z.wav"),
        sound("b", "ALPHA", "/sounds/b.wav"),
        sound("a2", "alpha", "/sounds/a.wav"),
        sound("a1", "Alpha", "/sounds/a.wav"),
    ];
    let ids = sorted_ids(
        &entries,
        SoundSortState::new(SoundSortKey::Name, Direction::Ascending),
        &SoundMetaStore::default(),
    );

    assert_eq!(ids, vec!["a1", "a2", "b", "z"]);
}

#[test]
fn customized_display_name_is_the_name_sort_value() {
    let entries = [
        sound("first", "Alpha", "/first.wav"),
        sound("second", "Zulu", "/second.wav"),
    ];
    let mut metadata = SoundMetaStore::default();
    metadata.set_display_name("second", Some("Aardvark".into()));
    let ids = sorted_ids(
        &entries,
        SoundSortState::new(SoundSortKey::Name, Direction::Ascending),
        &metadata,
    );

    assert_eq!(ids, vec!["second", "first"]);
}

#[test]
fn length_folder_modified_and_added_keys_order_sounds() {
    let entries = [
        with_category(
            with_timing(sound("late", "A", "/z/late.wav"), Some(300), Some(300)),
            "Zulu",
        ),
        with_category(
            with_timing(sound("early", "B", "/a/early.wav"), Some(100), Some(100)),
            "Alpha",
        ),
    ];
    let mut metadata = SoundMetaStore::default();
    metadata.reconcile_added(["late"], 300, false);
    metadata.reconcile_added(["early"], 100, false);

    for key in [
        SoundSortKey::Length,
        SoundSortKey::Folder,
        SoundSortKey::Modified,
        SoundSortKey::Added,
    ] {
        let ids = sorted_ids(
            &entries,
            SoundSortState::new(key, Direction::Ascending),
            &metadata,
        );
        assert_eq!(ids, vec!["early", "late"], "key: {key:?}");
    }
}

#[test]
fn unknown_dates_stay_last_in_both_directions() {
    let entries = [
        sound("unknown", "Unknown", "/u.wav"),
        with_timing(sound("older", "Older", "/o.wav"), None, Some(100)),
        with_timing(sound("newer", "Newer", "/n.wav"), None, Some(200)),
    ];
    let mut metadata = SoundMetaStore::default();
    metadata.reconcile_added(["older"], 100, false);
    metadata.reconcile_added(["newer"], 200, false);

    for key in [SoundSortKey::Modified, SoundSortKey::Added] {
        for (direction, expected) in [
            (Direction::Ascending, vec!["older", "newer", "unknown"]),
            (Direction::Descending, vec!["newer", "older", "unknown"]),
        ] {
            let ids = sorted_ids(&entries, SoundSortState::new(key, direction), &metadata);
            assert_eq!(ids, expected, "key: {key:?}");
        }
    }
}

#[test]
fn valid_preference_loads_and_unknown_data_uses_complete_default() {
    let mut valid = AppConfig::default();
    valid
        .sort_prefs
        .insert("tiles".into(), SortPref::new("modified", "descending"));
    assert_eq!(
        sound_sort_from_config(&valid),
        SoundSortState::new(SoundSortKey::Modified, Direction::Descending)
    );

    for pref in [
        SortPref::new("future", "descending"),
        SortPref::new("modified", "sideways"),
    ] {
        let mut config = AppConfig::default();
        config.sort_prefs.insert("tiles".into(), pref);
        assert_eq!(
            sound_sort_from_config(&config),
            SoundSortState::new(SoundSortKey::Name, Direction::Ascending)
        );
    }
}

#[test]
fn app_selection_and_direction_changes_update_persisted_preference() {
    let mut app = HonkHonk::new_for_test();

    let _ = app.update(Message::SelectSoundSort("added"));
    let _ = app.update(Message::ToggleSoundSortDirection);

    assert_eq!(
        app.sound_sort,
        SoundSortState::new(SoundSortKey::Added, Direction::Descending)
    );
    assert_eq!(
        app.config.sort_prefs.get("tiles"),
        Some(&SortPref::new("added", "descending"))
    );
}

#[test]
fn menu_open_captures_anchor_and_dismiss_does_not_change_sort() {
    let mut app = HonkHonk::new_for_test();
    app.cursor_pos = iced::Point::new(420.0, 64.0);
    let original = app.sound_sort;

    let _ = app.update(Message::ToggleSoundSortMenu);
    assert_eq!(app.sort_menu_anchor, Some(iced::Point::new(420.0, 64.0)));

    let _ = app.update(Message::DismissSoundSortMenu);
    assert!(app.sort_menu_anchor.is_none());
    assert_eq!(app.sound_sort, original);
    assert!(app.config.sort_prefs.is_empty());
}

#[test]
fn unknown_selection_closes_menu_without_changing_preference() {
    let mut app = HonkHonk::new_for_test();
    let _ = app.update(Message::ToggleSoundSortMenu);

    let _ = app.update(Message::SelectSoundSort("future"));

    assert!(app.sort_menu_anchor.is_none());
    assert_eq!(app.sound_sort, default_sound_sort());
    assert!(app.config.sort_prefs.is_empty());
}

#[test]
fn escape_closes_sort_menu_before_changing_filter_state() {
    let mut app = HonkHonk::new_for_test();
    let _ = app.update(Message::SearchChanged("honk".into()));
    let _ = app.update(Message::ToggleSoundSortMenu);

    let _ = app.update(Message::EscapePressed);

    assert!(app.sort_menu_anchor.is_none());
    assert_eq!(app.search_query(), "honk");
    assert!(app.filter.had_focus());
}
