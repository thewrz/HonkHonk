use iced::Element;

use crate::app::slot_sort::SlotSortKey;
use crate::app::{HonkHonk, Message, SettingsSection};
use crate::ui::list_controls::sort;
use crate::ui::theme::Theme;

#[allow(
    dead_code,
    reason = "wired into ui/settings/mod.rs's Stack-compose overlay layer by a follow-up task in this issue's task chain (#199)"
)]
impl HonkHonk {
    /// Sort-menu overlay for the Settings → Shortcuts sort chip. Reuses the
    /// same `sort_menu_anchor` field as the tiles view's sort menu, so it is
    /// only rendered while both the anchor is set *and* the Hotkeys section
    /// is the one currently on screen — otherwise a menu opened on the
    /// Shortcuts view would keep rendering after switching to another
    /// settings section (or the anchor is stale from the tiles sort menu).
    ///
    /// `pub(crate)`, not `pub(in crate::app)` like `view_sound_sort_overlay`:
    /// `src/ui/settings/hotkeys.rs` is a sibling module tree to `crate::app`,
    /// not a descendant of it, so it needs full crate-wide reach the way
    /// `hotkey_filter_query`/`hotkey_sort_state` do.
    pub(crate) fn view_hotkey_sort_overlay(&self, theme: Theme) -> Option<Element<'_, Message>> {
        if self.settings_ui.section() != SettingsSection::Hotkeys {
            return None;
        }
        let anchor = self.sort_menu_anchor?;
        Some(sort::view_sort_menu_overlay(
            sort::SortMenu {
                state: self.hotkey_sort_state(),
                options: &SlotSortKey::ALL,
                theme,
                anchor,
                window_size: self.window_size,
            },
            |key| Message::SelectHotkeySort(key.id()),
            Message::DismissHotkeySortMenu,
        ))
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;

    use super::*;

    fn open_hotkey_sort_menu(app: &mut HonkHonk) {
        app.settings_ui.select_section(SettingsSection::Hotkeys);
        app.toggle_hotkey_sort_menu();
        assert!(app.sort_menu_anchor.is_some(), "setup: menu did not open");
    }

    #[test]
    fn overlay_renders_when_anchor_set_and_hotkeys_section_active() {
        let mut app = HonkHonk::new_for_test();
        open_hotkey_sort_menu(&mut app);

        assert!(app.view_hotkey_sort_overlay(Theme::Light).is_some());
    }

    #[test]
    fn overlay_hidden_without_an_anchor() {
        let app = HonkHonk::new_for_test();
        assert!(
            app.settings_ui.section() != SettingsSection::Hotkeys || app.sort_menu_anchor.is_none()
        );

        assert!(app.view_hotkey_sort_overlay(Theme::Light).is_none());
    }

    #[test]
    fn overlay_hidden_when_anchor_set_but_a_different_section_is_active() {
        let mut app = HonkHonk::new_for_test();
        open_hotkey_sort_menu(&mut app);
        app.settings_ui.select_section(SettingsSection::Appearance);

        assert!(app.view_hotkey_sort_overlay(Theme::Light).is_none());
    }

    /// A stale anchor left over from the tiles sort menu must not leak an
    /// overlay onto an unrelated settings section.
    #[test]
    fn overlay_hidden_for_stale_anchor_on_a_non_hotkeys_section() {
        let mut app = HonkHonk::new_for_test();
        app.settings_ui.select_section(SettingsSection::Appearance);
        app.sort_menu_anchor = Some(Point::ORIGIN);

        assert!(app.view_hotkey_sort_overlay(Theme::Light).is_none());
    }
}
