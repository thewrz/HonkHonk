//! Settings → Shortcuts bindings list: sort state, row model, and the pure
//! filtered/sorted query surface (#199).

use super::HonkHonk;
use crate::app::slot_sort::SlotSortKey;
use crate::state::AppConfig;
use crate::ui::list_controls::filter::filter_items;
use crate::ui::list_controls::sort::{Direction, SortState};

mod rows;
#[cfg(test)]
mod tests;

pub(crate) use rows::HotkeyRow;

/// Sort state for the Settings → Shortcuts bindings list.
///
/// Declared `pub(crate)`, not `pub(super)`: Rust's re-export rule only lets
/// `pub use` narrow an item's own visibility, never widen it (E0365). A
/// `pub(crate)` accessor that names this type in a return position (e.g.
/// `hotkey_sort_state()`, below) would be a private-interfaces violation
/// under `pub(super)`.
pub(crate) type HotkeySortState = SortState<SlotSortKey>;

/// Sort-preference persistence key for this view, stored in
/// `AppConfig::sort_prefs` alongside the tiles view's `"tiles"` key.
const HOTKEYS_VIEW_KEY: &str = "shortcuts";

fn default_hotkey_sort() -> HotkeySortState {
    HotkeySortState::new(SlotSortKey::default(), Direction::Ascending)
}

/// Resolves a persisted sort id back into a `SlotSortKey`. `SlotSortKey`
/// keeps its own `from_id` private to `slot_sort.rs` (only its internal
/// round-trip test needs it); this module already has `pub(super)` access to
/// `ALL` + `id()`, which is enough to do the same lookup here without
/// widening `slot_sort.rs`'s surface further.
fn slot_sort_key_from_id(id: &str) -> Option<SlotSortKey> {
    SlotSortKey::ALL.into_iter().find(|key| key.id() == id)
}

/// Reads the persisted sort preference, falling back to the default
/// (SlotNumber ascending) for a missing, unknown, or corrupt entry. Never
/// panics on untrusted config content.
pub(super) fn hotkey_sort_from_config(config: &AppConfig) -> HotkeySortState {
    let Some(pref) = config.sort_prefs.get(HOTKEYS_VIEW_KEY) else {
        return default_hotkey_sort();
    };
    let Some(key) = slot_sort_key_from_id(pref.key()) else {
        return default_hotkey_sort();
    };
    let direction = match pref.direction() {
        "ascending" => Direction::Ascending,
        "descending" => Direction::Descending,
        _ => return default_hotkey_sort(),
    };
    HotkeySortState::new(key, direction)
}

#[allow(
    dead_code,
    reason = "query surface for #199; wired into the Settings → Shortcuts view by follow-up tasks in this issue's task chain"
)]
impl HonkHonk {
    /// Bound shortcut rows, filtered by the shared query and sorted by the
    /// active `SlotSortKey` — the acceptance-criterion query surface for
    /// #199. Pure: rebuilt from current state on every call, no cache (the
    /// list is at most 20 rows, so there's no cost to justify one).
    pub(crate) fn hotkey_rows(&self) -> Vec<HotkeyRow> {
        let rows = rows::build_hotkey_rows(self);
        let matched: Vec<HotkeyRow> =
            filter_items(&rows, self.hotkey_filter.query(), rows::hotkey_haystacks)
                .into_iter()
                .cloned()
                .collect();
        self.hotkey_sort.sorted(matched)
    }

    /// Mirrors `search_query()`: `ui/settings/hotkeys.rs` lives in a sibling
    /// module tree to `crate::app` (unlike `src/app/header.rs`, a descendant
    /// that can read `self.filter` directly), so it needs an accessor rather
    /// than a private-field reach-across.
    pub(crate) fn hotkey_filter_query(&self) -> &str {
        self.hotkey_filter.query()
    }

    /// Mirrors the above for the active sort state.
    pub(crate) fn hotkey_sort_state(&self) -> HotkeySortState {
        self.hotkey_sort
    }
}
