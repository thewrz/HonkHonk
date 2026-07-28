//! Resolves which view currently owns type-to-filter keyboard input and
//! Escape-clearing, so `filtering.rs` can route to the right `FilterState`
//! without re-deriving view-mode/section checks at every call site (#199).

use super::HonkHonk;
use crate::app::{SettingsSection, ViewMode};

/// The view that owns type-to-filter input for the current app state.
///
/// Routing is total and mutually exclusive: [`active_filter_target`] maps
/// every `(view_mode, settings section, staged-search state)` combination to
/// exactly one of `Some(Tiles)`, `Some(Hotkeys)`, or `None` — never more than
/// one target is active at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FilterTarget {
    /// The main sound grid's search bar.
    Tiles,
    /// The Settings → Shortcuts bindings list's own, independent search bar.
    Hotkeys,
}

/// Resolves the active filter target, if any, for the current app state.
///
/// `Settings` only routes to `Hotkeys` while the Shortcuts section is
/// selected *and* the staged settings search (#213, click-only, searches
/// settings themselves — a distinct list-controls surface) is not itself
/// active. The two search surfaces are independent and must never claim the
/// same keystroke.
pub(super) fn active_filter_target(state: &HonkHonk) -> Option<FilterTarget> {
    match state.view_mode {
        ViewMode::Main => Some(FilterTarget::Tiles),
        ViewMode::Settings
            if state.settings_ui.section() == SettingsSection::Hotkeys
                && !state.settings_ui.is_searching() =>
        {
            Some(FilterTarget::Hotkeys)
        }
        ViewMode::Settings | ViewMode::SlotManager => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECTIONS: [SettingsSection; 5] = [
        SettingsSection::Audio,
        SettingsSection::Library,
        SettingsSection::Hotkeys,
        SettingsSection::Appearance,
        SettingsSection::About,
    ];

    fn expected_target(
        view_mode: ViewMode,
        section: SettingsSection,
        searching: bool,
    ) -> Option<FilterTarget> {
        match view_mode {
            ViewMode::Main => Some(FilterTarget::Tiles),
            ViewMode::Settings if section == SettingsSection::Hotkeys && !searching => {
                Some(FilterTarget::Hotkeys)
            }
            ViewMode::Settings | ViewMode::SlotManager => None,
        }
    }

    #[test]
    fn routing_is_total_and_mutually_exclusive_across_every_state() {
        for view_mode in [ViewMode::Main, ViewMode::SlotManager, ViewMode::Settings] {
            for section in SECTIONS {
                for searching in [false, true] {
                    let mut app = HonkHonk::new_for_test();
                    app.view_mode = view_mode;
                    app.settings_ui.select_section(section);
                    if searching {
                        let _ = app.settings_ui.replace_query("query".into());
                    }

                    assert_eq!(
                        active_filter_target(&app),
                        expected_target(view_mode, section, searching),
                        "view_mode={view_mode:?} section={section:?} searching={searching}"
                    );
                }
            }
        }
    }

    #[test]
    fn main_view_always_targets_tiles_regardless_of_settings_state() {
        let mut app = HonkHonk::new_for_test();
        app.view_mode = ViewMode::Main;
        app.settings_ui.select_section(SettingsSection::Hotkeys);

        assert_eq!(active_filter_target(&app), Some(FilterTarget::Tiles));
    }

    #[test]
    fn staged_settings_search_takes_priority_over_hotkeys_filter() {
        let mut app = HonkHonk::new_for_test();
        app.view_mode = ViewMode::Settings;
        app.settings_ui.select_section(SettingsSection::Hotkeys);
        let _ = app.settings_ui.replace_query("theme".into());

        assert_eq!(active_filter_target(&app), None);
    }

    #[test]
    fn slot_manager_never_targets_a_filter() {
        let mut app = HonkHonk::new_for_test();
        app.view_mode = ViewMode::SlotManager;

        assert_eq!(active_filter_target(&app), None);
    }
}
