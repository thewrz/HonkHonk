use iced::keyboard::Key;
use iced::keyboard::key::Named;

use super::test_support::GuiHarness;
use crate::app::Message;
use crate::settings::{SettingCategory, SettingId};
use crate::ui::search_bar;
use crate::ui::theme::Theme;

static GUI_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn serialize_gui_test() -> std::sync::MutexGuard<'static, ()> {
    GUI_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn sort_chip_selects_and_persists_name() {
    let _guard = serialize_gui_test();
    let mut harness = GuiHarness::new();
    let _ = harness.app.update(Message::SelectSoundSort("length"));

    harness.click("Sort: Length");
    assert!(harness.find("✓ Length"));
    assert!(harness.find("  Name"));
    assert!(harness.find("  Folder"));

    harness.click("  Name");

    assert!(harness.find("Sort: Name"));
    assert!(!harness.find("Length"));
    let pref = harness
        .app
        .config
        .sort_prefs
        .get("tiles")
        .expect("selecting a sort key should update the tiles preference");
    assert_eq!(pref.key(), "name");
    assert_eq!(pref.direction(), "ascending");
}

#[test]
fn staged_settings_search_filters_highlights_and_restores() {
    let _guard = serialize_gui_test();
    let mut harness = GuiHarness::new();
    search_for_theme(&mut harness);
    select_theme_result(&mut harness);
    clear_search_and_assert_restore(&mut harness);
}

fn search_for_theme(harness: &mut GuiHarness) {
    harness.click("Settings");
    let input = search_bar::settings_input_id();
    harness.tap_key(input.clone(), Key::Named(Named::End));
    harness.typewrite(input, "theme");

    assert_eq!(harness.app.settings_ui.query(), "theme");
    assert!(harness.find("Appearance"));
    assert!(!harness.find("Audio"));
    assert!(!harness.find("Library"));
}

fn select_theme_result(harness: &mut GuiHarness) {
    harness.click("Appearance");
    assert!(harness.find("Theme"));
    assert!(harness.find_id(crate::ui::settings::highlighted_row_id(SettingId::Theme)));

    harness.click("System");
    assert_eq!(harness.app.config.theme, Theme::System);
}

fn clear_search_and_assert_restore(harness: &mut GuiHarness) {
    let restore_tasks = harness.click("✕");

    assert!(
        restore_tasks > 0,
        "clearing should schedule row restoration"
    );
    assert_eq!(harness.app.settings_ui.query(), "");
    assert_eq!(
        harness.app.settings_ui.section(),
        SettingCategory::Appearance
    );
    assert!(harness.find("Theme"));
    assert!(!harness.find_id(crate::ui::settings::highlighted_row_id(SettingId::Theme)));
}
