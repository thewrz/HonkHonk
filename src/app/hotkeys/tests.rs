use std::path::PathBuf;

use super::*;
use crate::app::Message;
use crate::state::{AudioFormat, Macro, SortPref, SoundEntry};

fn sound(id: &str, name: &str, path: &str, category: &str) -> SoundEntry {
    SoundEntry {
        id: id.into(),
        name: name.into(),
        path: PathBuf::from(path),
        format: AudioFormat::Wav,
        duration_ms: Some(1_000),
        modified_ms: Some(500),
        category: category.into(),
    }
}

fn macro_entry(id: &str, name: &str) -> Macro {
    Macro {
        id: id.into(),
        name: name.into(),
        steps: Vec::new(),
    }
}

/// A slot 0 bound to a trigger with a resolved sound assigned.
fn app_with_resolved_sound() -> HonkHonk {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![sound(
        "goose",
        "goose_honk",
        "/sounds/Animals/goose.wav",
        "Animals",
    )];
    app.slots.set(0, PathBuf::from("/sounds/Animals/goose.wav"));
    app.slot_triggers[0] = Some("Meta+1".into());
    app
}

#[test]
fn hotkeys_view_key_matches_the_issue_acceptance_text() {
    assert_eq!(HOTKEYS_VIEW_KEY, "shortcuts");
}

#[test]
fn hotkey_sort_state_composes_slot_sort_key_and_direction() {
    let state = HotkeySortState::new(SlotSortKey::default(), Direction::Ascending);

    assert_eq!(state.key(), SlotSortKey::SlotNumber);
    assert_eq!(state.direction(), Direction::Ascending);
}

#[test]
fn valid_preference_loads_and_unknown_data_uses_complete_default() {
    let mut valid = AppConfig::default();
    valid
        .sort_prefs
        .insert(HOTKEYS_VIEW_KEY.into(), SortPref::new("tag", "descending"));
    assert_eq!(
        hotkey_sort_from_config(&valid),
        HotkeySortState::new(SlotSortKey::Tag, Direction::Descending)
    );

    for pref in [
        SortPref::new("future", "descending"),
        SortPref::new("tag", "sideways"),
    ] {
        let mut config = AppConfig::default();
        config.sort_prefs.insert(HOTKEYS_VIEW_KEY.into(), pref);
        assert_eq!(hotkey_sort_from_config(&config), default_hotkey_sort());
    }

    assert_eq!(
        hotkey_sort_from_config(&AppConfig::default()),
        default_hotkey_sort()
    );
}

/// Row membership is keyed by `slot_triggers`, not by `SlotMap` content: a
/// slot with content but no bound trigger produces no row, and a slot with a
/// bound trigger always produces exactly one row regardless of what (if
/// anything) is assigned to it.
#[test]
fn row_membership_follows_bound_triggers_not_slot_content() {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![sound(
        "goose",
        "goose_honk",
        "/sounds/Animals/goose.wav",
        "Animals",
    )];
    // Slot 0: content assigned, but no trigger bound -> no row.
    app.slots.set(0, PathBuf::from("/sounds/Animals/goose.wav"));
    // Slot 1: trigger bound, resolved sound assigned -> one row.
    app.slots.set(1, PathBuf::from("/sounds/Animals/goose.wav"));
    app.slot_triggers[1] = Some("Meta+2".into());
    // Slot 2: trigger bound, no content assigned -> one row.
    app.slot_triggers[2] = Some("Meta+3".into());

    let rows = app.hotkey_rows();

    let slot_indices: Vec<u8> = rows.iter().map(|row| row.slot_index).collect();
    assert_eq!(slot_indices, vec![1, 2]);
}

#[test]
fn display_name_is_never_empty_across_every_resolution_branch() {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![sound(
        "goose",
        "goose_honk",
        "/sounds/Animals/goose.wav",
        "Animals",
    )];
    app.macros.0.push(macro_entry("macro-1", "Boop sequence"));

    // Slot 0: resolved sound.
    app.slots.set(0, PathBuf::from("/sounds/Animals/goose.wav"));
    app.slot_triggers[0] = Some("Meta+1".into());
    // Slot 1: dangling sound path (never scanned into the library).
    app.slots.set(1, PathBuf::from("/sounds/Animals/gone.wav"));
    app.slot_triggers[1] = Some("Meta+2".into());
    // Slot 2: resolved macro.
    app.slots.set_macro(2, "macro-1").unwrap();
    app.slot_triggers[2] = Some("Meta+3".into());
    // Slot 3: dangling macro id (deleted after the trigger was bound).
    app.slots.set_macro(3, "gone-macro").unwrap();
    app.slot_triggers[3] = Some("Meta+4".into());
    // Slot 4: trigger bound, nothing assigned.
    app.slot_triggers[4] = Some("Meta+5".into());

    let rows = app.hotkey_rows();

    assert_eq!(rows.len(), 5, "every bound trigger must produce a row");
    for row in &rows {
        assert!(
            !row.display_name.is_empty(),
            "slot {} has an empty display_name",
            row.slot_index
        );
    }
    assert_eq!(rows[0].display_name, "goose_honk");
    assert_eq!(rows[2].display_name, "Boop sequence");
}

#[test]
fn hotkey_rows_is_pure_and_rebuilds_the_same_result() {
    let app = app_with_resolved_sound();

    assert_eq!(app.hotkey_rows(), app.hotkey_rows());
}

#[test]
fn filtering_matches_display_name_filename_and_tag_but_not_trigger() {
    let mut app = app_with_resolved_sound();

    for query in ["goose_honk", "goose.wav", "animals"] {
        app.hotkey_filter.replace(query.into());
        assert_eq!(
            app.hotkey_rows().len(),
            1,
            "query {query:?} should match the bound row"
        );
    }

    app.hotkey_filter.replace("Meta+1".into());
    assert!(
        app.hotkey_rows().is_empty(),
        "the trigger text itself must not be searchable"
    );
}

#[test]
fn sorting_every_key_preserves_the_full_row_set() {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![
        sound("a", "Alpha", "/a.wav", "Zoo"),
        sound("b", "Bravo", "/b.wav", "Ark"),
    ];
    app.slots.set(0, PathBuf::from("/a.wav"));
    app.slot_triggers[0] = Some("Meta+1".into());
    app.slots.set(1, PathBuf::from("/b.wav"));
    app.slot_triggers[1] = Some("Meta+2".into());
    // Slot with unknown values for Length/Modified/Added, to exercise the
    // unknown-sorts-last path without dropping a row.
    app.slot_triggers[2] = Some("Meta+3".into());

    for key in SlotSortKey::ALL {
        for direction in [Direction::Ascending, Direction::Descending] {
            app.hotkey_sort = HotkeySortState::new(key, direction);
            let mut slot_indices: Vec<u8> =
                app.hotkey_rows().iter().map(|row| row.slot_index).collect();
            slot_indices.sort_unstable();
            assert_eq!(
                slot_indices,
                vec![0, 1, 2],
                "key {key:?} direction {direction:?} dropped or duplicated a row"
            );
        }
    }
}

#[test]
fn accessors_mirror_the_underlying_filter_and_sort_state() {
    let mut app = HonkHonk::new_for_test();
    app.hotkey_filter.replace("honk".into());
    app.hotkey_sort = HotkeySortState::new(SlotSortKey::Added, Direction::Descending);

    assert_eq!(app.hotkey_filter_query(), "honk");
    assert_eq!(
        app.hotkey_sort_state(),
        HotkeySortState::new(SlotSortKey::Added, Direction::Descending)
    );
}

/// Message-driven round trip pinned at the boundary: selecting a sort key
/// and flipping direction through `Message` must both land in
/// `hotkey_sort` and persist to `config.sort_prefs["shortcuts"]` —
/// mirrors `app_selection_and_direction_changes_update_persisted_preference`
/// for the tiles view.
#[test]
fn message_driven_selection_and_direction_changes_persist_under_shortcuts_key() {
    let mut app = HonkHonk::new_for_test();

    let _ = app.update(Message::SelectHotkeySort("tag"));
    let _ = app.update(Message::ToggleHotkeySortDirection);

    assert_eq!(
        app.hotkey_sort,
        HotkeySortState::new(SlotSortKey::Tag, Direction::Descending)
    );
    assert_eq!(
        app.config.sort_prefs.get(HOTKEYS_VIEW_KEY),
        Some(&SortPref::new("tag", "descending"))
    );
}

#[test]
fn message_driven_menu_open_captures_anchor_and_dismiss_does_not_change_sort() {
    let mut app = HonkHonk::new_for_test();
    app.cursor_pos = iced::Point::new(420.0, 64.0);
    let original = app.hotkey_sort;

    let _ = app.update(Message::ToggleHotkeySortMenu);
    assert_eq!(app.sort_menu_anchor, Some(iced::Point::new(420.0, 64.0)));

    let _ = app.update(Message::DismissHotkeySortMenu);
    assert!(app.sort_menu_anchor.is_none());
    assert_eq!(app.hotkey_sort, original);
    assert!(app.config.sort_prefs.is_empty());
}

#[test]
fn message_driven_unknown_selection_closes_menu_without_changing_preference() {
    let mut app = HonkHonk::new_for_test();
    let _ = app.update(Message::ToggleHotkeySortMenu);

    let _ = app.update(Message::SelectHotkeySort("future"));

    assert!(app.sort_menu_anchor.is_none());
    assert_eq!(app.hotkey_sort, default_hotkey_sort());
    assert!(app.config.sort_prefs.is_empty());
}

/// The filter query is transient like the tiles view's: it lands in
/// `hotkey_filter` but is never written to `config.sort_prefs`.
#[test]
fn message_driven_filter_query_replaces_state_without_persisting() {
    let mut app = HonkHonk::new_for_test();

    let _ = app.update(Message::HotkeySearchChanged("goose".into()));

    assert_eq!(app.hotkey_filter_query(), "goose");
    assert!(app.config.sort_prefs.is_empty());
}
