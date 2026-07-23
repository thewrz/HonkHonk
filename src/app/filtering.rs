use iced::event::Status;
use iced::keyboard;
use std::borrow::Cow;

use super::{FAVORITES_TAB, HonkHonk, Message, ViewMode, sorting};
use crate::state::SoundEntry;
use crate::ui::list_controls::filter::{Activation, ActivationContext, filter_items};
use crate::ui::search_bar;

pub(super) fn type_to_filter_text(event: &iced::Event, status: Status) -> Option<String> {
    if status != Status::Ignored {
        return None;
    }

    let iced::Event::Keyboard(keyboard::Event::KeyPressed {
        modifiers,
        text: Some(text),
        ..
    }) = event
    else {
        return None;
    };

    if modifiers.control() || modifiers.alt() || modifiers.logo() {
        return None;
    }

    (!text.is_empty() && text.chars().all(|character| !character.is_control()))
        .then(|| text.to_string())
}

impl HonkHonk {
    fn filter_context(&self) -> ActivationContext {
        let activation = match self.view_mode {
            ViewMode::Main => Activation::TypeToFilter,
            ViewMode::SlotManager | ViewMode::Settings => Activation::ClickOnly,
        };
        ActivationContext::new(activation, self.filter_is_blocked())
    }

    fn filter_is_blocked(&self) -> bool {
        self.context_menu.is_some()
            || self.editor_sound_id.is_some()
            || self.macro_editor_draft.is_some()
            || self.effects_panel.is_visible()
            || self.sort_menu_anchor.is_some()
    }

    pub(super) fn handle_type_to_filter(&mut self, text: &str) -> iced::Task<Message> {
        if !self.filter_context().allows_typing() {
            return iced::Task::none();
        }

        self.filter.insert(text);
        iced::widget::operation::focus(search_bar::input_id())
    }

    pub(super) fn handle_escape(&mut self, event_was_captured: bool) -> iced::Task<Message> {
        if self.dismiss_sound_sort_menu() {
            return iced::Task::none();
        }
        if self.context_menu.is_some() {
            self.context_menu = None;
            self.context_menu_pos = None;
        } else if self.editor_sound_id.is_some() {
            self.editor_sound_id = None;
            self.editor_draft_name.clear();
            self.editor_draft_volume = 1.0;
        } else if self.macro_editor_draft.is_some() {
            // The draft belongs to the macro editor; its own close/discard flow
            // decides its fate, so global Escape must not alter filter state.
            return iced::Task::none();
        } else if self.effects_panel.is_visible() {
            self.close_effects_panel_from_escape(std::time::Instant::now());
        } else if event_was_captured {
            self.filter.consume_focus();
        } else {
            self.filter.escape();
        }
        iced::Task::none()
    }

    /// Returns sounds matching the shared query and active category filters.
    pub fn filtered_sounds(&self) -> Vec<&SoundEntry> {
        let sounds = filter_items(&self.sounds, self.filter.query(), |sound| {
            let display_name = self
                .sound_meta
                .get_ref(&sound.id)
                .and_then(|meta| meta.display_name.as_deref())
                .unwrap_or("");
            let filename = sound
                .path
                .file_name()
                .map(std::ffi::OsStr::to_string_lossy)
                .unwrap_or_default();
            [
                Cow::Borrowed(display_name),
                filename,
                Cow::Borrowed(sound.name.as_str()),
                Cow::Borrowed(sound.category.as_str()),
            ]
        })
        .into_iter()
        .filter(|sound| match self.active_category.as_deref() {
            Some(FAVORITES_TAB) => self.sound_meta.is_favorite(&sound.id),
            Some(category) => sound.category == category,
            None => true,
        })
        .collect();
        sorting::sorted_sounds(sounds, self.sound_sort, &self.sound_meta)
    }
}

#[cfg(test)]
mod tests {
    use iced::event::Status;
    use iced::keyboard::key::{NativeCode, Physical};
    use iced::keyboard::{self, Key, Location, Modifiers};

    use super::*;
    use crate::state::{AudioFormat, Macro};

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
        app.sounds = ["Zulu", "alpha"]
            .into_iter()
            .map(|name| SoundEntry {
                id: name.into(),
                name: name.into(),
                path: format!("/sounds/{name}.wav").into(),
                format: AudioFormat::Wav,
                duration_ms: None,
                category: "Other".into(),
                modified_ms: None,
            })
            .collect();

        assert_eq!(app.filtered_sounds()[0].name, "alpha");

        let _ = app.update(Message::ToggleSoundSortDirection);

        assert_eq!(app.filtered_sounds()[0].name, "Zulu");
    }
}
