use super::{SETTINGS_REGISTRY, SettingCategory, SettingDef, SettingId};

const SEARCHABLE_CATEGORIES: &[SettingCategory] = &[
    SettingCategory::Audio,
    SettingCategory::Library,
    SettingCategory::Appearance,
];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ScrollOffset {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RestoreTarget {
    Setting {
        id: SettingId,
        category: SettingCategory,
    },
    Offset {
        category: SettingCategory,
        offset: ScrollOffset,
    },
}

#[derive(Debug, Clone, Default)]
pub struct SettingsSearchState {
    query: String,
    current_offset: ScrollOffset,
    restore_section: SettingCategory,
    restore_offset: ScrollOffset,
    last_interacted: Option<(SettingId, SettingCategory)>,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsUiState {
    section: SettingCategory,
    search: SettingsSearchState,
}

pub fn matching_settings(query: &str, category: SettingCategory) -> Vec<&'static SettingDef> {
    let needle = normalized_query(query);
    if needle.is_empty() {
        return Vec::new();
    }

    SETTINGS_REGISTRY
        .iter()
        .filter(|setting| setting.category == category)
        .filter(|setting| setting_matches(setting, &needle))
        .collect()
}

pub fn matching_categories(query: &str) -> Vec<SettingCategory> {
    SEARCHABLE_CATEGORIES
        .iter()
        .copied()
        .filter(|category| !matching_settings(query, *category).is_empty())
        .collect()
}

fn normalized_query(query: &str) -> String {
    query.trim().to_lowercase()
}

fn setting_matches(setting: &SettingDef, needle: &str) -> bool {
    [setting.label, setting.hint, setting.category.label()]
        .into_iter()
        .chain(setting.keywords.iter().copied())
        .any(|field| field.to_lowercase().contains(needle))
}

fn category_for_setting(id: SettingId) -> Option<SettingCategory> {
    SETTINGS_REGISTRY
        .iter()
        .find(|setting| setting.id == id)
        .map(|setting| setting.category)
}

impl SettingsUiState {
    pub fn open(&mut self) {
        *self = Self::default();
    }

    pub fn section(&self) -> SettingCategory {
        self.section
    }

    pub fn query(&self) -> &str {
        &self.search.query
    }

    pub fn is_searching(&self) -> bool {
        !normalized_query(self.query()).is_empty()
    }

    pub fn select_section(&mut self, section: SettingCategory) {
        self.section = section;
    }

    pub fn record_scroll(&mut self, x: f32, y: f32) {
        self.search.current_offset = ScrollOffset { x, y };
    }

    pub fn record_interaction(&mut self, id: SettingId) {
        if self.is_searching()
            && let Some(category) = category_for_setting(id)
        {
            self.search.last_interacted = Some((id, category));
        }
    }

    pub fn replace_query(&mut self, query: String) -> Option<RestoreTarget> {
        let was_searching = self.is_searching();
        let will_search = !normalized_query(&query).is_empty();

        if !was_searching && will_search {
            self.search.restore_section = self.section;
            self.search.restore_offset = self.search.current_offset;
            self.search.last_interacted = None;
        }

        self.search.query = query;
        if was_searching && !will_search {
            return Some(self.finish_search());
        }

        None
    }

    fn finish_search(&mut self) -> RestoreTarget {
        let target = match self.search.last_interacted.take() {
            Some((id, category)) => RestoreTarget::Setting { id, category },
            None => RestoreTarget::Offset {
                category: self.search.restore_section,
                offset: self.search.restore_offset,
            },
        };
        self.section = match target {
            RestoreTarget::Setting { category, .. } | RestoreTarget::Offset { category, .. } => {
                category
            }
        };
        target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_all_approved_registry_fields_case_insensitively() {
        assert_eq!(
            matching_settings("PaSsThRoUgH", SettingCategory::Audio)
                .first()
                .map(|setting| setting.id),
            Some(SettingId::MicPassthrough)
        );
        assert_eq!(
            matching_settings("virtual mic", SettingCategory::Audio)
                .first()
                .map(|setting| setting.id),
            Some(SettingId::MicPassthrough)
        );
        assert_eq!(
            matching_settings("microphone", SettingCategory::Audio)
                .first()
                .map(|setting| setting.id),
            Some(SettingId::MicPassthrough)
        );
        assert!(
            matching_settings("audio", SettingCategory::Audio)
                .iter()
                .any(|setting| setting.id == SettingId::OverlapMode)
        );
    }

    #[test]
    fn category_search_excludes_non_registry_sections() {
        assert_eq!(
            matching_categories("theme"),
            vec![SettingCategory::Appearance]
        );
        assert!(!matching_categories("portal").contains(&SettingCategory::Hotkeys));
        assert!(!matching_categories("license").contains(&SettingCategory::About));
    }

    #[test]
    fn whitespace_is_inactive_and_nonmatches_are_empty() {
        assert!(matching_categories(" \t ").is_empty());
        assert!(matching_categories("definitely absent").is_empty());
    }

    #[test]
    fn typing_does_not_switch_sections_and_snapshots_offset_once() {
        let mut state = SettingsUiState::default();
        state.select_section(SettingCategory::Library);
        state.record_scroll(2.0, 140.0);

        assert_eq!(state.replace_query("scan".into()), None);
        assert_eq!(state.section(), SettingCategory::Library);

        state.record_scroll(0.0, 12.0);
        assert_eq!(state.replace_query("scan now".into()), None);
        assert_eq!(state.section(), SettingCategory::Library);

        assert_eq!(
            state.replace_query(String::new()),
            Some(RestoreTarget::Offset {
                category: SettingCategory::Library,
                offset: ScrollOffset { x: 2.0, y: 140.0 },
            })
        );
    }

    #[test]
    fn clearing_restores_latest_interacted_setting_and_its_section() {
        let mut state = SettingsUiState::default();
        state.replace_query("mode".into());
        state.select_section(SettingCategory::Audio);
        state.record_interaction(SettingId::MicPassthrough);
        state.record_interaction(SettingId::OverlapMode);
        state.select_section(SettingCategory::Appearance);

        assert_eq!(
            state.replace_query(String::new()),
            Some(RestoreTarget::Setting {
                id: SettingId::OverlapMode,
                category: SettingCategory::Audio,
            })
        );
        assert_eq!(state.section(), SettingCategory::Audio);
    }

    #[test]
    fn interaction_outside_search_is_not_reused_by_later_session() {
        let mut state = SettingsUiState::default();
        state.record_interaction(SettingId::Theme);
        state.replace_query("theme".into());

        assert_eq!(
            state.replace_query(String::new()),
            Some(RestoreTarget::Offset {
                category: SettingCategory::Audio,
                offset: ScrollOffset::default(),
            })
        );
    }
}
