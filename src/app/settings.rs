use iced::Task;
use iced::widget::scrollable::AbsoluteOffset;

use super::{HonkHonk, Message, SettingsSection, ViewMode};
use crate::settings::SettingId;
use crate::settings::search::{RestoreTarget, RowRestoreRequest, ScrollOffset};

#[cfg(test)]
mod gui_tests;
#[cfg(test)]
mod test_support;

#[derive(Debug, Clone, PartialEq)]
pub enum SettingsMessage {
    Show,
    ShowSection(SettingsSection),
    SearchChanged(String),
    Scrolled(AbsoluteOffset),
    Interacted {
        id: SettingId,
        action: Box<Message>,
    },
    RowLocated {
        request: RowRestoreRequest,
        offset: f32,
    },
}

impl From<SettingsMessage> for Message {
    fn from(message: SettingsMessage) -> Self {
        Self::Settings(message)
    }
}

impl HonkHonk {
    pub(super) fn update_settings(&mut self, message: SettingsMessage) -> Task<Message> {
        match message {
            SettingsMessage::Show => self.show_settings(),
            SettingsMessage::ShowSection(section) => self.show_settings_section(section),
            SettingsMessage::SearchChanged(query) => self.change_settings_search(query),
            SettingsMessage::Scrolled(offset) => self.record_settings_scroll(offset),
            SettingsMessage::Interacted { id, action } => {
                self.handle_setting_interaction(id, *action)
            }
            SettingsMessage::RowLocated { request, offset } => {
                self.restore_settings_row(request, offset)
            }
        }
    }

    pub(super) fn show_settings(&mut self) -> Task<Message> {
        self.dismiss_sound_sort_menu();
        self.view_mode = ViewMode::Settings;
        self.settings_ui.open();
        Task::none()
    }

    pub(super) fn show_settings_section(&mut self, section: SettingsSection) -> Task<Message> {
        // Any sort menu open on the previous section (tiles' or Hotkeys')
        // must not survive a section switch: the anchor is a shared field,
        // so leaving it set would let a menu opened on one section keep
        // rendering (or silently reopen) on another.
        self.sort_menu_anchor = None;
        self.settings_ui.select_section(section);
        Task::none()
    }

    pub(super) fn change_settings_search(&mut self, query: String) -> Task<Message> {
        // The staged settings search swaps the section body for `search_results`
        // while leaving `settings_ui.section()` untouched, so a Shortcuts sort
        // menu left open would keep stacking its overlay on top of the results.
        // Same shared-anchor reasoning as `show_settings_section` above.
        self.sort_menu_anchor = None;
        match self.settings_ui.replace_query(query) {
            Some(RestoreTarget::Setting(request)) => {
                crate::ui::settings::locate_setting_row(request)
            }
            Some(RestoreTarget::Offset { offset, .. }) => scroll_to(offset),
            None => Task::none(),
        }
    }

    pub(super) fn record_settings_scroll(&mut self, offset: AbsoluteOffset) -> Task<Message> {
        self.settings_ui.record_scroll(offset.x, offset.y);
        Task::none()
    }

    pub(super) fn handle_setting_interaction(
        &mut self,
        id: SettingId,
        action: Message,
    ) -> Task<Message> {
        self.settings_ui.record_interaction(id);
        self.update(action)
    }

    pub(super) fn restore_settings_row(
        &mut self,
        request: RowRestoreRequest,
        y: f32,
    ) -> Task<Message> {
        if self.settings_ui.accept_row_restore(request) {
            scroll_to(ScrollOffset { x: 0.0, y })
        } else {
            Task::none()
        }
    }
}

fn scroll_to(offset: ScrollOffset) -> Task<Message> {
    iced::widget::operation::scroll_to(
        crate::ui::settings::content_scroll_id(),
        AbsoluteOffset {
            x: offset.x,
            y: offset.y,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SettingCategory;

    #[test]
    fn opening_settings_resets_transient_search_state() {
        let mut app = HonkHonk::new_for_test();
        app.settings_ui.replace_query("temporary search".to_owned());

        let _ = app.show_settings();

        assert_eq!(app.settings_ui.query(), "");
        assert_eq!(app.settings_ui.section(), SettingCategory::Audio);
        assert_eq!(app.view_mode, ViewMode::Settings);
    }

    #[test]
    fn opening_settings_dismisses_sound_sort_menu() {
        let mut app = HonkHonk::new_for_test();
        app.toggle_sound_sort_menu();

        let _ = app.show_settings();

        assert!(app.sort_menu_anchor.is_none());
    }

    #[test]
    fn switching_section_dismisses_an_open_sort_menu() {
        let mut app = HonkHonk::new_for_test();
        app.settings_ui.select_section(SettingCategory::Hotkeys);
        app.toggle_hotkey_sort_menu();
        assert!(
            app.sort_menu_anchor.is_some(),
            "setup: sort menu did not open"
        );

        let _ = app.show_settings_section(SettingCategory::Appearance);

        assert!(app.sort_menu_anchor.is_none());
    }

    /// A staged settings search keeps `section()` on Hotkeys while replacing
    /// the body with search results, so an open sort menu would otherwise
    /// stay stacked over them — `view_hotkey_sort_overlay`'s section guard
    /// cannot catch this on its own.
    #[test]
    fn starting_a_settings_search_dismisses_an_open_sort_menu() {
        let mut app = HonkHonk::new_for_test();
        app.settings_ui.select_section(SettingCategory::Hotkeys);
        app.toggle_hotkey_sort_menu();
        assert!(
            app.sort_menu_anchor.is_some(),
            "setup: sort menu did not open"
        );

        let _ = app.change_settings_search("theme".to_owned());

        assert!(app.sort_menu_anchor.is_none());
        assert!(
            app.view_hotkey_sort_overlay(crate::ui::theme::Theme::Light)
                .is_none(),
            "no sort overlay may render over the search results"
        );
    }

    #[test]
    fn selecting_a_section_does_not_change_the_query() {
        let mut app = HonkHonk::new_for_test();
        app.settings_ui.replace_query("theme".to_owned());

        let _ = app.show_settings_section(SettingCategory::Appearance);

        assert_eq!(app.settings_ui.query(), "theme");
        assert_eq!(app.settings_ui.section(), SettingCategory::Appearance);
    }

    #[test]
    fn tracked_action_updates_state_and_runs_original_message() {
        let mut app = HonkHonk::new_for_test();
        app.settings_ui.replace_query("theme".to_owned());

        let _ = app.handle_setting_interaction(
            SettingId::Theme,
            Message::ThemeChanged(crate::ui::theme::Theme::Light),
        );

        assert_eq!(app.config.theme, crate::ui::theme::Theme::Light);
        assert!(matches!(
            app.settings_ui.replace_query(String::new()),
            Some(RestoreTarget::Setting(request))
                if request.setting() == SettingId::Theme
                    && request.category() == SettingCategory::Appearance
        ));
    }

    #[test]
    fn stale_row_location_is_ignored_after_section_change() {
        let mut app = HonkHonk::new_for_test();
        app.settings_ui.replace_query("mode".to_owned());
        app.settings_ui.record_interaction(SettingId::OverlapMode);
        let Some(RestoreTarget::Setting(request)) = app.settings_ui.replace_query(String::new())
        else {
            panic!("clearing an interacted search should request row restoration");
        };

        let _ = app.show_settings_section(SettingCategory::Appearance);
        let task = app.restore_settings_row(request, 240.0);

        assert_eq!(task.units(), 0);
    }
}
