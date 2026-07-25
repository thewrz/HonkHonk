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

#[derive(Debug, Clone, Default)]
struct CachedMatches {
    categories: Vec<SettingCategory>,
    rows: Vec<(SettingCategory, Vec<&'static SettingDef>)>,
}

impl CachedMatches {
    fn new(needle: &str) -> Self {
        if needle.is_empty() {
            return Self::default();
        }

        let rows = SEARCHABLE_CATEGORIES
            .iter()
            .copied()
            .filter_map(|category| {
                let matches = SETTINGS_REGISTRY
                    .iter()
                    .filter(|setting| setting.category == category)
                    .filter(|setting| setting_matches(setting, needle))
                    .collect::<Vec<_>>();
                (!matches.is_empty()).then_some((category, matches))
            })
            .collect::<Vec<_>>();
        let categories = rows.iter().map(|(category, _)| *category).collect();
        Self { categories, rows }
    }

    fn settings(&self, category: SettingCategory) -> &[&'static SettingDef] {
        self.rows
            .iter()
            .find(|(candidate, _)| *candidate == category)
            .map_or(&[], |(_, settings)| settings.as_slice())
    }
}

#[derive(Debug, Clone, Default)]
pub struct SettingsSearchState {
    query: String,
    is_searching: bool,
    matches: CachedMatches,
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
        self.search.is_searching
    }

    pub fn matching_categories(&self) -> &[SettingCategory] {
        &self.search.matches.categories
    }

    pub fn matching_settings(&self) -> &[&'static SettingDef] {
        self.search.matches.settings(self.section)
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
        let needle = normalized_query(&query);
        let will_search = !needle.is_empty();
        self.invalidate_row_restore();

        if !was_searching && will_search {
            self.search.restore_section = self.section;
            self.search.restore_offset = self.search.current_offset;
            self.search.last_interacted = None;
        }

        self.search.query = query;
        self.search.is_searching = will_search;
        self.search.matches = CachedMatches::new(&needle);
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
mod tests;
