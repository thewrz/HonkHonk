use super::*;

#[test]
fn search_matches_all_approved_registry_fields_case_insensitively() {
    let mut state = SettingsUiState::default();
    state.select_section(SettingCategory::Audio);

    state.replace_query("PaSsThRoUgH".into());
    assert_eq!(
        state.matching_settings().first().map(|setting| setting.id),
        Some(SettingId::MicPassthrough)
    );

    state.replace_query("virtual mic".into());
    assert_eq!(
        state.matching_settings().first().map(|setting| setting.id),
        Some(SettingId::MicPassthrough)
    );

    state.replace_query("microphone".into());
    assert_eq!(
        state.matching_settings().first().map(|setting| setting.id),
        Some(SettingId::MicPassthrough)
    );

    state.replace_query("audio".into());
    assert!(
        state
            .matching_settings()
            .iter()
            .any(|setting| setting.id == SettingId::OverlapMode)
    );
}

#[test]
fn category_search_excludes_non_registry_sections() {
    let mut state = SettingsUiState::default();
    state.replace_query("theme".into());
    assert_eq!(state.matching_categories(), &[SettingCategory::Appearance]);

    state.replace_query("portal".into());
    assert!(
        !state
            .matching_categories()
            .contains(&SettingCategory::Hotkeys)
    );

    state.replace_query("license".into());
    assert!(
        !state
            .matching_categories()
            .contains(&SettingCategory::About)
    );
}

#[test]
fn whitespace_is_inactive_and_nonmatches_are_empty() {
    let mut state = SettingsUiState::default();
    state.replace_query(" \t ".into());
    assert!(state.matching_categories().is_empty());
    state.replace_query("definitely absent".into());
    assert!(state.matching_categories().is_empty());
}

#[test]
fn cached_matches_refresh_for_query_and_section_changes() {
    let mut state = SettingsUiState::default();

    state.replace_query("theme".into());
    assert_eq!(state.matching_categories(), &[SettingCategory::Appearance]);
    assert!(state.matching_settings().is_empty());

    state.select_section(SettingCategory::Appearance);
    assert_eq!(
        state
            .matching_settings()
            .iter()
            .map(|setting| setting.id)
            .collect::<Vec<_>>(),
        vec![SettingId::Theme]
    );

    state.replace_query("mode".into());
    assert!(state.matching_settings().is_empty());
    state.select_section(SettingCategory::Audio);
    assert_eq!(
        state
            .matching_settings()
            .iter()
            .map(|setting| setting.id)
            .collect::<Vec<_>>(),
        vec![SettingId::OverlapMode]
    );
}

#[test]
fn inactive_or_unmatched_queries_clear_cached_matches() {
    let mut state = SettingsUiState::default();
    state.replace_query("theme".into());
    assert!(state.is_searching());
    assert!(!state.matching_categories().is_empty());

    state.replace_query("definitely absent".into());
    assert!(state.is_searching());
    assert!(state.matching_categories().is_empty());
    assert!(state.matching_settings().is_empty());

    state.replace_query(" \t ".into());
    assert!(!state.is_searching());
    assert!(state.matching_categories().is_empty());
    assert!(state.matching_settings().is_empty());
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
