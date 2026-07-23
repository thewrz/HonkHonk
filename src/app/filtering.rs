use iced::event::Status;
use iced::keyboard;

use super::{HonkHonk, Message, ViewMode};
use crate::ui::list_controls::filter::{Activation, ActivationContext};
use crate::ui::search_bar;

mod cache;

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
    pub(super) fn select_sound_category(&mut self, category: Option<String>) {
        if self.active_category == category {
            return;
        }
        self.active_category = category;
        self.refresh_filtered_sounds();
    }

    pub(super) fn replace_filter_query(&mut self, query: String) {
        let changed = self.filter.query() != query;
        self.filter.replace(query);
        if changed {
            self.refresh_filtered_sounds();
        }
    }

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
        if !text.is_empty() {
            self.refresh_filtered_sounds();
        }
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
            let query_was_present = !self.filter.query().is_empty();
            self.filter.escape();
            if query_was_present && self.filter.query().is_empty() {
                self.refresh_filtered_sounds();
            }
        }
        iced::Task::none()
    }
}

#[cfg(test)]
mod tests {
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
}
