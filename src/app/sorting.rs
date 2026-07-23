use std::cmp::Ordering;

use super::HonkHonk;
use crate::state::{AppConfig, SortPref, SoundEntry, SoundMetaStore};
use crate::ui::list_controls::sort::{Direction, SortKey, SortLabel, SortState};

mod view;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SoundSortKey {
    #[default]
    Name,
    Length,
    Folder,
    Modified,
    Added,
}

impl SoundSortKey {
    pub(super) const ALL: [Self; 5] = [
        Self::Name,
        Self::Length,
        Self::Folder,
        Self::Modified,
        Self::Added,
    ];

    pub(super) const fn id(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Length => "length",
            Self::Folder => "folder",
            Self::Modified => "modified",
            Self::Added => "added",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "name" => Some(Self::Name),
            "length" => Some(Self::Length),
            "folder" => Some(Self::Folder),
            "modified" => Some(Self::Modified),
            "added" => Some(Self::Added),
            _ => None,
        }
    }
}

pub(super) type SoundSortState = SortState<SoundSortKey>;

fn default_sound_sort() -> SoundSortState {
    SoundSortState::new(SoundSortKey::Name, Direction::Ascending)
}

pub(super) fn sound_sort_from_config(config: &AppConfig) -> SoundSortState {
    let Some(pref) = config.sort_prefs.get("tiles") else {
        return default_sound_sort();
    };
    let Some(key) = SoundSortKey::from_id(pref.key()) else {
        return default_sound_sort();
    };
    let direction = match pref.direction() {
        "ascending" => Direction::Ascending,
        "descending" => Direction::Descending,
        _ => return default_sound_sort(),
    };
    SoundSortState::new(key, direction)
}

pub(super) fn sorted_sounds<'a>(
    sounds: Vec<&'a SoundEntry>,
    state: SoundSortState,
    metadata: &SoundMetaStore,
) -> Vec<&'a SoundEntry> {
    let sortable = sounds
        .into_iter()
        .map(|sound| SoundSortItem::new(sound, metadata))
        .collect::<Vec<_>>();
    state
        .sorted(sortable)
        .into_iter()
        .map(|item| item.sound)
        .collect()
}

impl HonkHonk {
    pub(super) fn toggle_sound_sort_menu(&mut self) {
        self.sort_menu_anchor = if self.sort_menu_anchor.is_some() {
            None
        } else {
            Some(self.cursor_pos)
        };
    }

    pub(super) fn toggle_sound_sort_direction(&mut self) {
        self.sound_sort.toggle_direction();
        self.persist_sound_sort();
    }

    pub(super) fn select_sound_sort(&mut self, key_id: &str) {
        let Some(key) = SoundSortKey::from_id(key_id) else {
            self.sort_menu_anchor = None;
            return;
        };
        self.sound_sort.select(key);
        self.sort_menu_anchor = None;
        self.persist_sound_sort();
    }

    pub(super) fn dismiss_sound_sort_menu(&mut self) -> bool {
        self.sort_menu_anchor.take().is_some()
    }

    fn persist_sound_sort(&mut self) {
        self.config.sort_prefs.insert(
            "tiles".into(),
            SortPref::new(self.sound_sort.key().id(), self.sound_sort.direction().id()),
        );
        self.persist_config();
    }
}

struct SoundSortItem<'a> {
    sound: &'a SoundEntry,
    name: String,
    folder: String,
    added_ms: Option<u64>,
}

impl<'a> SoundSortItem<'a> {
    fn new(sound: &'a SoundEntry, metadata: &SoundMetaStore) -> Self {
        let name = metadata
            .get_ref(&sound.id)
            .and_then(|meta| meta.display_name.as_deref())
            .unwrap_or(&sound.name)
            .to_lowercase();
        Self {
            sound,
            name,
            folder: sound.category.to_lowercase(),
            added_ms: metadata.added_ms(&sound.id),
        }
    }

    fn tie_break(&self, other: &Self) -> Ordering {
        self.sound
            .path
            .cmp(&other.sound.path)
            .then_with(|| self.sound.id.cmp(&other.sound.id))
    }
}

impl SortLabel for SoundSortKey {
    fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Length => "Length",
            Self::Folder => "Folder",
            Self::Modified => "Modified",
            Self::Added => "Added",
        }
    }
}

impl SortKey<SoundSortItem<'_>> for SoundSortKey {
    fn compare(self, left: &SoundSortItem<'_>, right: &SoundSortItem<'_>) -> Ordering {
        let primary = match self {
            Self::Name => left.name.cmp(&right.name),
            Self::Length => left.sound.duration_ms.cmp(&right.sound.duration_ms),
            Self::Folder => left.folder.cmp(&right.folder),
            Self::Modified => left.sound.modified_ms.cmp(&right.sound.modified_ms),
            Self::Added => left.added_ms.cmp(&right.added_ms),
        };
        primary.then_with(|| left.tie_break(right))
    }

    fn value_unknown(self, item: &SoundSortItem<'_>) -> bool {
        match self {
            Self::Length => item.sound.duration_ms.is_none(),
            Self::Modified => item.sound.modified_ms.is_none(),
            Self::Added => item.added_ms.is_none(),
            Self::Name | Self::Folder => false,
        }
    }
}

#[cfg(test)]
mod tests {
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

    fn ids<'a>(sounds: &'a [&'a SoundEntry]) -> Vec<&'a str> {
        sounds.iter().map(|sound| sound.id.as_str()).collect()
    }

    #[test]
    fn name_sort_is_case_insensitive_and_uses_path_then_id_ties() {
        let entries = [
            sound("z", "Zulu", "/sounds/z.wav"),
            sound("b", "ALPHA", "/sounds/b.wav"),
            sound("a2", "alpha", "/sounds/a.wav"),
            sound("a1", "Alpha", "/sounds/a.wav"),
        ];
        let sounds = sorted_sounds(
            entries.iter().collect(),
            SoundSortState::new(SoundSortKey::Name, Direction::Ascending),
            &SoundMetaStore::default(),
        );

        assert_eq!(ids(&sounds), vec!["a1", "a2", "b", "z"]);
    }

    #[test]
    fn customized_display_name_is_the_name_sort_value() {
        let entries = [
            sound("first", "Alpha", "/first.wav"),
            sound("second", "Zulu", "/second.wav"),
        ];
        let mut metadata = SoundMetaStore::default();
        metadata.set_display_name("second", Some("Aardvark".into()));
        let sounds = sorted_sounds(
            entries.iter().collect(),
            SoundSortState::new(SoundSortKey::Name, Direction::Ascending),
            &metadata,
        );

        assert_eq!(ids(&sounds), vec!["second", "first"]);
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
            let sounds = sorted_sounds(
                entries.iter().collect(),
                SoundSortState::new(key, Direction::Ascending),
                &metadata,
            );
            assert_eq!(ids(&sounds), vec!["early", "late"], "key: {key:?}");
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
                let sounds = sorted_sounds(
                    entries.iter().collect(),
                    SoundSortState::new(key, direction),
                    &metadata,
                );
                assert_eq!(ids(&sounds), expected, "key: {key:?}");
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
}
