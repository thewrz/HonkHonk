use iced::event::Status;
use iced::keyboard::key::{NativeCode, Physical};
use iced::keyboard::{self, Key, Location, Modifiers};

use super::*;
use crate::app::FAVORITES_TAB;
use crate::state::{AudioFormat, Macro, SoundEntry};

fn sound(id: &str, name: &str, duration_ms: Option<u64>, category: &str) -> SoundEntry {
    SoundEntry {
        id: id.into(),
        name: name.into(),
        path: format!("/sounds/{category}/{id}.wav").into(),
        format: AudioFormat::Wav,
        duration_ms,
        category: category.into(),
        modified_ms: None,
    }
}

fn filtered_ids(app: &HonkHonk) -> Vec<&str> {
    app.filtered_sounds()
        .into_iter()
        .map(|sound| sound.id.as_str())
        .collect()
}

fn key_event(text: Option<&str>, modifiers: Modifiers) -> iced::Event {
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: Key::Unidentified,
        modified_key: Key::Unidentified,
        physical_key: Physical::Unidentified(NativeCode::Unidentified),
        location: Location::Standard,
        modifiers,
        text: text.map(Into::into),
        repeat: false,
    })
}

#[test]
fn ignored_printable_text_becomes_filter_input() {
    let event = key_event(Some("h"), Modifiers::NONE);
    assert_eq!(
        type_to_filter_text(&event, Status::Ignored),
        Some("h".into())
    );
}

#[test]
fn captured_text_is_never_claimed() {
    let event = key_event(Some("h"), Modifiers::NONE);
    assert_eq!(type_to_filter_text(&event, Status::Captured), None);
}

#[test]
fn shifted_unicode_text_is_preserved() {
    let event = key_event(Some("Ö"), Modifiers::SHIFT);
    assert_eq!(
        type_to_filter_text(&event, Status::Ignored),
        Some("Ö".into())
    );
}

#[test]
fn shortcut_modifiers_are_rejected() {
    for modifier in [Modifiers::CTRL, Modifiers::ALT, Modifiers::LOGO] {
        let event = key_event(Some("h"), modifier);
        assert_eq!(type_to_filter_text(&event, Status::Ignored), None);
    }
}

#[test]
fn empty_or_control_text_is_rejected() {
    for text in [None, Some(""), Some("\n"), Some("a\tb")] {
        let event = key_event(text, Modifiers::NONE);
        assert_eq!(type_to_filter_text(&event, Status::Ignored), None);
    }
}

#[test]
fn main_view_accepts_first_text_exactly_once() {
    let mut app = HonkHonk::new_for_test();
    let _ = app.update(Message::TypeToFilter("Ö".into()));
    assert_eq!(app.search_query(), "Ö");

    let _ = app.update(Message::SearchChanged("Öh".into()));
    assert_eq!(app.search_query(), "Öh");
}

#[test]
fn captured_escape_after_refocus_does_not_clear_existing_query() {
    let mut app = HonkHonk::new_for_test();
    let _ = app.update(Message::SearchChanged("honk".into()));
    let _ = app.update(Message::EscapePressed);
    assert_eq!(app.search_query(), "honk");

    let _ = app.update(Message::CapturedEscapePressed);
    assert_eq!(app.search_query(), "honk");
}

#[test]
fn macro_editor_draft_absorbs_escape_without_changing_filter_state() {
    let mut app = HonkHonk::new_for_test();
    let _ = app.update(Message::SearchChanged("honk".into()));
    app.macro_editor_draft = Some(Macro {
        id: "draft".into(),
        name: "Draft".into(),
        steps: Vec::new(),
    });

    let _ = app.update(Message::EscapePressed);

    assert_eq!(app.search_query(), "honk");
    assert!(app.filter.had_focus());
    assert!(app.macro_editor_draft().is_some());
}

#[test]
fn click_only_views_reject_typed_filter_text() {
    for view_mode in [ViewMode::SlotManager, ViewMode::Settings] {
        let mut app = HonkHonk::new_for_test();
        app.view_mode = view_mode;
        let _ = app.update(Message::TypeToFilter("h".into()));
        assert_eq!(app.search_query(), "");
    }
}

#[test]
fn typed_filter_text_routes_to_hotkeys_when_shortcuts_section_is_active() {
    let mut app = HonkHonk::new_for_test();
    app.view_mode = ViewMode::Settings;
    app.settings_ui.select_section(SettingsSection::Hotkeys);

    let _ = app.update(Message::TypeToFilter("h".into()));

    assert_eq!(app.hotkey_filter_query(), "h");
    assert_eq!(
        app.search_query(),
        "",
        "hotkeys-targeted typing must never reach the tiles filter"
    );
}

/// Type-to-filter must *focus* the input it seeds, not merely update the
/// query: `FilterState`'s activation contract is "focus and seed the filter
/// input". Without the focus task the list filters, but Backspace, Delete,
/// and arrow keys keep going to whatever widget was focused before typing
/// began — the query becomes uneditable.
#[test]
fn typed_filter_text_focuses_the_targeted_search_input() {
    let mut tiles = HonkHonk::new_for_test();
    tiles.view_mode = ViewMode::Main;

    assert!(
        tiles.update(Message::TypeToFilter("h".into())).units() > 0,
        "tiles type-to-filter must schedule a focus task"
    );

    let mut hotkeys = HonkHonk::new_for_test();
    hotkeys.view_mode = ViewMode::Settings;
    hotkeys.settings_ui.select_section(SettingsSection::Hotkeys);

    assert!(
        hotkeys.update(Message::TypeToFilter("h".into())).units() > 0,
        "hotkeys type-to-filter must schedule a focus task, like the tiles path"
    );
}

/// `units() > 0` above proves a focus task is scheduled but not *where* it
/// points — Iced's `Task` cannot be inspected. `filter_input_id` is the seam
/// both branches focus through, so pinning it here covers the other half:
/// the two targets must never focus each other's input.
#[test]
fn each_filter_target_focuses_its_own_search_input() {
    assert_eq!(
        super::filter_input_id(FilterTarget::Tiles),
        search_bar::input_id()
    );
    assert_eq!(
        super::filter_input_id(FilterTarget::Hotkeys),
        search_bar::hotkeys_input_id()
    );
    assert_ne!(
        super::filter_input_id(FilterTarget::Tiles),
        super::filter_input_id(FilterTarget::Hotkeys)
    );
}

#[test]
fn staged_settings_search_blocks_hotkeys_type_to_filter() {
    let mut app = HonkHonk::new_for_test();
    app.view_mode = ViewMode::Settings;
    app.settings_ui.select_section(SettingsSection::Hotkeys);
    let _ = app.settings_ui.replace_query("theme".into());

    let _ = app.update(Message::TypeToFilter("h".into()));

    assert_eq!(
        app.hotkey_filter_query(),
        "",
        "the staged settings search (#213) must retain sole ownership of typing while active"
    );
}

#[test]
fn escape_in_settings_hotkeys_section_never_touches_the_tiles_filter() {
    let mut app = HonkHonk::new_for_test();
    app.view_mode = ViewMode::Settings;
    app.settings_ui.select_section(SettingsSection::Hotkeys);
    let _ = app.update(Message::SearchChanged("goose".into()));
    let _ = app.update(Message::HotkeySearchChanged("honk".into()));

    // First Escape only consumes the staged-focus flag.
    let _ = app.update(Message::EscapePressed);
    assert_eq!(app.hotkey_filter_query(), "honk");
    assert_eq!(app.search_query(), "goose");

    // Second Escape clears the active (hotkeys) target only.
    let _ = app.update(Message::EscapePressed);
    assert_eq!(app.hotkey_filter_query(), "");
    assert_eq!(
        app.search_query(),
        "goose",
        "escape scoped to the hotkeys target must never clear the inactive tiles query"
    );
}

#[test]
fn escape_in_main_view_never_touches_the_hotkeys_filter() {
    let mut app = HonkHonk::new_for_test();
    let _ = app.update(Message::HotkeySearchChanged("honk".into()));
    let _ = app.update(Message::SearchChanged("goose".into()));

    let _ = app.update(Message::EscapePressed);
    let _ = app.update(Message::EscapePressed);

    assert_eq!(app.search_query(), "");
    assert_eq!(
        app.hotkey_filter_query(),
        "honk",
        "escape scoped to the tiles target must never clear the inactive hotkeys query"
    );
}

#[test]
fn every_blocking_layer_rejects_typed_filter_text() {
    let mut context_menu = HonkHonk::new_for_test();
    context_menu.context_menu = Some("sound".into());

    let mut sound_editor = HonkHonk::new_for_test();
    sound_editor.editor_sound_id = Some("sound".into());

    let mut macro_editor = HonkHonk::new_for_test();
    macro_editor.macro_editor_draft = Some(Macro {
        id: "draft".into(),
        name: "Draft".into(),
        steps: Vec::new(),
    });

    let mut effects_drawer = HonkHonk::new_for_test();
    let _ = effects_drawer.update(Message::ToggleEffectsPanel);

    for mut app in [context_menu, sound_editor, macro_editor, effects_drawer] {
        let _ = app.update(Message::TypeToFilter("h".into()));
        assert_eq!(app.search_query(), "");
    }
}

#[test]
fn shared_filter_matches_display_name_filename_and_category() {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![SoundEntry {
        id: "goose".into(),
        name: "goose_honk".into(),
        path: "/sounds/Animals/goose_honk.WAV".into(),
        format: AudioFormat::Wav,
        duration_ms: None,
        category: "Animals".into(),
        modified_ms: None,
    }];
    app.sound_meta
        .set_display_name("goose", Some("Angry Bird".into()));

    for query in ["angry", ".wav", "animals", "goose_honk"] {
        let _ = app.update(Message::SearchChanged(query.into()));
        assert_eq!(app.filtered_sounds().len(), 1, "query: {query}");
    }
}

#[test]
fn main_grid_filter_results_follow_the_active_sort_state() {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![
        sound("zulu", "Zulu", None, "Other"),
        sound("alpha", "alpha", None, "Other"),
    ];
    app.refresh_filtered_sounds();

    assert_eq!(app.filtered_sounds()[0].name, "alpha");

    let _ = app.update(Message::ToggleSoundSortDirection);

    assert_eq!(app.filtered_sounds()[0].name, "Zulu");
}

#[test]
fn filtered_sounds_reads_cached_order_without_resorting() {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![
        sound("zulu", "Zulu", None, "Other"),
        sound("alpha", "alpha", None, "Other"),
    ];
    app.refresh_filtered_sounds();

    assert_eq!(filtered_ids(&app), vec!["alpha", "zulu"]);
    app.sound_sort.toggle_direction();

    assert_eq!(
        filtered_ids(&app),
        vec!["alpha", "zulu"],
        "reading filtered sounds must not recompute their order"
    );
}

#[test]
fn query_category_and_favorite_updates_refresh_cached_membership() {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![
        sound("alpha", "Alpha", None, "Animals"),
        sound("beta", "Beta", None, "Memes"),
    ];
    app.refresh_filtered_sounds();

    let _ = app.update(Message::SearchChanged("beta".into()));
    assert_eq!(filtered_ids(&app), vec!["beta"]);

    let _ = app.update(Message::SearchChanged(String::new()));
    let _ = app.update(Message::SelectCategory(Some("Animals".into())));
    assert_eq!(filtered_ids(&app), vec!["alpha"]);

    let _ = app.update(Message::SelectCategory(None));
    let _ = app.update(Message::TypeToFilter("beta".into()));
    assert_eq!(filtered_ids(&app), vec!["beta"]);
    let _ = app.update(Message::EscapePressed);
    let _ = app.update(Message::EscapePressed);
    assert_eq!(filtered_ids(&app), vec!["alpha", "beta"]);

    let _ = app.update(Message::ToggleFavorite("beta".into()));
    let _ = app.update(Message::SelectCategory(Some(FAVORITES_TAB.into())));
    assert_eq!(filtered_ids(&app), vec!["beta"]);

    let _ = app.update(Message::ToggleFavorite("beta".into()));
    assert_eq!(filtered_ids(&app), vec!["alpha", "beta"]);
}

#[test]
fn duration_and_display_name_updates_refresh_cached_order() {
    let mut app = HonkHonk::new_for_test();
    app.sounds = vec![
        sound("alpha", "Alpha", Some(200), "Other"),
        sound("zulu", "Zulu", None, "Other"),
    ];
    app.refresh_filtered_sounds();

    let _ = app.update(Message::SelectSoundSort("length"));
    assert_eq!(filtered_ids(&app), vec!["alpha", "zulu"]);

    let durations = std::collections::HashMap::from([("zulu".to_owned(), 100)]);
    let _ = app.update(Message::DurationsLoaded(durations));
    assert_eq!(filtered_ids(&app), vec!["zulu", "alpha"]);

    let _ = app.update(Message::SelectSoundSort("name"));
    let _ = app.update(Message::OpenSoundEditor("zulu".into()));
    let _ = app.update(Message::SoundEditorNameChanged("Aardvark".into()));
    let _ = app.update(Message::SaveSoundMeta("zulu".into()));
    assert_eq!(filtered_ids(&app), vec!["zulu", "alpha"]);

    let _ = app.update(Message::SearchChanged("aardvark".into()));
    assert_eq!(filtered_ids(&app), vec!["zulu"]);
}
