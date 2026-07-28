//! Settings → Shortcuts bindings list: sort state foundation (#199).
//!
//! This module currently holds only the sort-state type and its persistence
//! key. Row-building, filtering, and the `HonkHonk` update-loop wiring land
//! in follow-up tasks of #199.

use crate::app::slot_sort::SlotSortKey;
use crate::ui::list_controls::sort::SortState;

/// Sort state for the Settings → Shortcuts bindings list.
///
/// Declared `pub(crate)`, not `pub(super)`: Rust's re-export rule only lets
/// `pub use` narrow an item's own visibility, never widen it (E0365). A
/// `pub(crate)` accessor that names this type in a return position (e.g.
/// `hotkey_sort_state()`, added once `HonkHonk` grows the field) would be a
/// private-interfaces violation under `pub(super)`.
#[allow(
    dead_code,
    reason = "foundation type for #199; wired into HonkHonk state by follow-up tasks"
)]
pub(crate) type HotkeySortState = SortState<SlotSortKey>;

/// Sort-preference persistence key for this view, stored in
/// `AppConfig::sort_prefs` alongside the tiles view's `"tiles"` key.
#[allow(
    dead_code,
    reason = "foundation constant for #199; read by hotkey_sort_from_config in a follow-up task"
)]
const HOTKEYS_VIEW_KEY: &str = "shortcuts";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::list_controls::sort::Direction;

    #[test]
    fn hotkeys_view_key_matches_the_issue_acceptance_text() {
        assert_eq!(HOTKEYS_VIEW_KEY, "shortcuts");
    }

    #[test]
    fn hotkey_sort_state_composes_slot_sort_key_and_direction() {
        let state = HotkeySortState::new(SlotSortKey::default(), Direction::Ascending);

        assert_eq!(state.key(), SlotSortKey::SlotNumber);
        assert_eq!(state.direction(), Direction::Ascending);
    }
}
