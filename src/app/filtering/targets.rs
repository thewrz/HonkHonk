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

    const VIEW_MODES: [ViewMode; 3] = [ViewMode::Main, ViewMode::SlotManager, ViewMode::Settings];

    /// Every `(view_mode, section, staged-search)` state with its intended
    /// target written out literally, rather than re-deriving it from
    /// [`active_filter_target`]'s own match — a mirrored oracle can only fail
    /// when the two copies diverge, never when the rule itself is wrong.
    #[rustfmt::skip]
    const EXPECTED_ROUTING: [(ViewMode, SettingsSection, bool, Option<FilterTarget>); 30] = [
        // The main grid owns typing regardless of any settings state behind it.
        (ViewMode::Main, SettingsSection::Audio,      false, Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::Audio,      true,  Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::Library,    false, Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::Library,    true,  Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::Hotkeys,    false, Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::Hotkeys,    true,  Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::Appearance, false, Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::Appearance, true,  Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::About,      false, Some(FilterTarget::Tiles)),
        (ViewMode::Main, SettingsSection::About,      true,  Some(FilterTarget::Tiles)),
        // The slot manager has no filter surface at all.
        (ViewMode::SlotManager, SettingsSection::Audio,      false, None),
        (ViewMode::SlotManager, SettingsSection::Audio,      true,  None),
        (ViewMode::SlotManager, SettingsSection::Library,    false, None),
        (ViewMode::SlotManager, SettingsSection::Library,    true,  None),
        (ViewMode::SlotManager, SettingsSection::Hotkeys,    false, None),
        (ViewMode::SlotManager, SettingsSection::Hotkeys,    true,  None),
        (ViewMode::SlotManager, SettingsSection::Appearance, false, None),
        (ViewMode::SlotManager, SettingsSection::Appearance, true,  None),
        (ViewMode::SlotManager, SettingsSection::About,      false, None),
        (ViewMode::SlotManager, SettingsSection::About,      true,  None),
        // Settings routes to the bindings list only on Shortcuts, and only
        // while the staged settings search is not itself claiming keystrokes.
        (ViewMode::Settings, SettingsSection::Audio,      false, None),
        (ViewMode::Settings, SettingsSection::Audio,      true,  None),
        (ViewMode::Settings, SettingsSection::Library,    false, None),
        (ViewMode::Settings, SettingsSection::Library,    true,  None),
        (ViewMode::Settings, SettingsSection::Hotkeys,    false, Some(FilterTarget::Hotkeys)),
        (ViewMode::Settings, SettingsSection::Hotkeys,    true,  None),
        (ViewMode::Settings, SettingsSection::Appearance, false, None),
        (ViewMode::Settings, SettingsSection::Appearance, true,  None),
        (ViewMode::Settings, SettingsSection::About,      false, None),
        (ViewMode::Settings, SettingsSection::About,      true,  None),
    ];

    /// The table above is only a totality proof if it actually enumerates
    /// every state — a new `ViewMode` or `SettingsSection` must fail here
    /// rather than silently go unrouted.
    #[test]
    fn expectation_table_covers_every_state_exactly_once() {
        for view_mode in VIEW_MODES {
            for section in SECTIONS {
                for searching in [false, true] {
                    let matches = EXPECTED_ROUTING
                        .iter()
                        .filter(|(m, s, q, _)| *m == view_mode && *s == section && *q == searching)
                        .count();
                    assert_eq!(
                        matches, 1,
                        "view_mode={view_mode:?} section={section:?} searching={searching}"
                    );
                }
            }
        }
    }

    #[test]
    fn routing_is_total_and_mutually_exclusive_across_every_state() {
        for (view_mode, section, searching, expected) in EXPECTED_ROUTING {
            let mut app = HonkHonk::new_for_test();
            app.view_mode = view_mode;
            app.settings_ui.select_section(section);
            if searching {
                let _ = app.settings_ui.replace_query("query".into());
            }

            assert_eq!(
                active_filter_target(&app),
                expected,
                "view_mode={view_mode:?} section={section:?} searching={searching}"
            );
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
