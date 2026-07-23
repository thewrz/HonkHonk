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
    Setting(RowRestoreRequest),
    Offset {
        category: SettingCategory,
        offset: ScrollOffset,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowRestoreRequest {
    setting: SettingId,
    category: SettingCategory,
    generation: u64,
}

impl RowRestoreRequest {
    const fn new(setting: SettingId, category: SettingCategory, generation: u64) -> Self {
        Self {
            setting,
            category,
            generation,
        }
    }

    pub const fn setting(self) -> SettingId {
        self.setting
    }

    pub const fn category(self) -> SettingCategory {
        self.category
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMatchScope {
    SelectedCategory,
    OtherCategory,
    None,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsSearchState {
    query: String,
    current_offset: ScrollOffset,
    restore_section: SettingCategory,
    restore_offset: ScrollOffset,
    last_interacted: Option<(SettingId, SettingCategory)>,
    restore_generation: u64,
    pending_row_restore: Option<RowRestoreRequest>,
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

pub fn search_match_scope(query: &str, selected: SettingCategory) -> SearchMatchScope {
    let categories = matching_categories(query);
    if categories.contains(&selected) {
        SearchMatchScope::SelectedCategory
    } else if categories.is_empty() {
        SearchMatchScope::None
    } else {
        SearchMatchScope::OtherCategory
    }
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
        let generation = self.search.restore_generation.wrapping_add(1);
        *self = Self::default();
        self.search.restore_generation = generation;
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
        self.invalidate_row_restore();
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
        self.invalidate_row_restore();

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
            Some((setting, category)) => RestoreTarget::Setting(RowRestoreRequest::new(
                setting,
                category,
                self.search.restore_generation,
            )),
            None => RestoreTarget::Offset {
                category: self.search.restore_section,
                offset: self.search.restore_offset,
            },
        };
        self.section = match target {
            RestoreTarget::Setting(request) => request.category(),
            RestoreTarget::Offset { category, .. } => category,
        };
        if let RestoreTarget::Setting(request) = target {
            self.search.pending_row_restore = Some(request);
        }
        target
    }

    pub fn accept_row_restore(&mut self, request: RowRestoreRequest) -> bool {
        let is_current = self.search.pending_row_restore == Some(request)
            && self.section == request.category()
            && !self.is_searching();
        if is_current {
            self.search.pending_row_restore = None;
        }
        is_current
    }

    fn invalidate_row_restore(&mut self) {
        self.search.restore_generation = self.search.restore_generation.wrapping_add(1);
        self.search.pending_row_restore = None;
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
    fn search_scope_distinguishes_selected_other_and_global_matches() {
        assert_eq!(
            search_match_scope("theme", SettingCategory::Appearance),
            SearchMatchScope::SelectedCategory
        );
        assert_eq!(
            search_match_scope("theme", SettingCategory::Audio),
            SearchMatchScope::OtherCategory
        );
        assert_eq!(
            search_match_scope("definitely absent", SettingCategory::Audio),
            SearchMatchScope::None
        );
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

        assert!(matches!(
            state.replace_query(String::new()),
            Some(RestoreTarget::Setting(request))
                if request.setting() == SettingId::OverlapMode
                    && request.category() == SettingCategory::Audio
        ));
        assert_eq!(state.section(), SettingCategory::Audio);
    }

    #[test]
    fn query_and_section_changes_reject_stale_row_restore_requests() {
        let mut state = SettingsUiState::default();
        state.replace_query("mode".into());
        state.record_interaction(SettingId::OverlapMode);
        let Some(RestoreTarget::Setting(query_stale)) = state.replace_query(String::new()) else {
            panic!("clearing an interacted search should request row restoration");
        };

        state.replace_query("theme".into());
        assert!(!state.accept_row_restore(query_stale));

        state.record_interaction(SettingId::Theme);
        let Some(RestoreTarget::Setting(section_stale)) = state.replace_query(String::new()) else {
            panic!("clearing the second search should request row restoration");
        };
        state.select_section(SettingCategory::Audio);
        assert!(!state.accept_row_restore(section_stale));
    }

    #[test]
    fn restore_generation_rejects_an_older_request_for_the_same_row() {
        let mut state = SettingsUiState::default();
        state.replace_query("mode".into());
        state.record_interaction(SettingId::OverlapMode);
        let Some(RestoreTarget::Setting(first)) = state.replace_query(String::new()) else {
            panic!("first search should request row restoration");
        };

        state.replace_query("mode".into());
        state.record_interaction(SettingId::OverlapMode);
        let Some(RestoreTarget::Setting(second)) = state.replace_query(String::new()) else {
            panic!("second search should request row restoration");
        };

        assert_ne!(first, second);
        assert!(!state.accept_row_restore(first));
        assert!(state.accept_row_restore(second));
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
