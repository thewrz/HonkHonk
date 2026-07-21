use std::time::SystemTime;

use crate::state::{LibraryScan, SoundEntry, SoundMetaStore};

use super::HonkHonk;

pub(super) fn load_sound_meta(scan: &LibraryScan) -> SoundMetaStore {
    let mut store = SoundMetaStore::load();
    reconcile_sound_meta(&mut store, &scan.entries, scan.complete, true);
    store
}

fn reconcile_sound_meta(
    store: &mut SoundMetaStore,
    sounds: &[SoundEntry],
    complete: bool,
    persist: bool,
) {
    let Some(observed_at_ms) = crate::state::library::system_time_to_epoch_ms(SystemTime::now())
    else {
        tracing::warn!("system clock is before Unix epoch; first-seen timestamps not reconciled");
        return;
    };
    let changed = store.reconcile_added(
        sounds.iter().map(|sound| &sound.id),
        observed_at_ms,
        complete,
    );
    save_reconciled_meta(store, changed && persist);
}

fn save_reconciled_meta(store: &SoundMetaStore, should_save: bool) {
    if !should_save {
        return;
    }
    if let Err(error) = store.save() {
        tracing::warn!(error = %error, "failed to save reconciled sound metadata");
    }
}

impl HonkHonk {
    pub(super) fn apply_library_scan(&mut self, scan: LibraryScan) {
        reconcile_sound_meta(
            &mut self.sound_meta,
            &scan.entries,
            scan.complete,
            self.persist,
        );
        self.duration_scan_pairs = std::sync::Arc::new(
            scan.entries
                .iter()
                .map(|sound| (sound.id.clone(), sound.path.clone()))
                .collect(),
        );
        self.sounds = scan.entries;
        self.reconcile_playback_with_library();
        self.durations_loaded = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Message;
    use crate::state::AudioFormat;

    fn sound(id: &str) -> SoundEntry {
        SoundEntry {
            id: id.to_owned(),
            name: id.to_owned(),
            path: format!("/sounds/{id}.wav").into(),
            format: AudioFormat::Wav,
            duration_ms: None,
            modified_ms: None,
            category: "Test".to_owned(),
        }
    }

    #[test]
    fn applying_complete_scan_reconciles_first_seen_ids_and_prunes_stale_ids() {
        let mut app = HonkHonk::new_for_test();
        app.sound_meta.reconcile_added(["stale"], 1, true);

        app.apply_library_scan(LibraryScan {
            entries: vec![sound("current")],
            complete: true,
        });

        assert!(app.sound_meta.added_ms("current").is_some());
        assert_eq!(app.sound_meta.added_ms("stale"), None);
    }

    #[test]
    fn applying_partial_scan_preserves_unseen_first_seen_ids() {
        let mut app = HonkHonk::new_for_test();
        app.sound_meta.reconcile_added(["unseen"], 1, true);

        app.apply_library_scan(LibraryScan {
            entries: vec![sound("current")],
            complete: false,
        });

        assert_eq!(app.sound_meta.added_ms("unseen"), Some(1));
        assert!(app.sound_meta.added_ms("current").is_some());
    }

    #[test]
    fn rescan_library_resets_durations_loaded() {
        let mut app = HonkHonk::new_for_test();
        app.durations_loaded = true;
        let _ = app.update(Message::RescanLibrary);
        assert!(!app.durations_loaded);
    }

    #[test]
    fn remove_sound_directory_removes_path() {
        let mut app = HonkHonk::new_for_test();
        let path = std::path::PathBuf::from("/tmp/hh_test_sounds");
        app.config.sound_directories.push(path.clone());

        let _ = app.update(Message::RemoveSoundDirectory(path.clone()));

        assert!(!app.config.sound_directories.contains(&path));
    }

    #[test]
    fn sound_directory_pick_some_appends_to_config() {
        let mut app = HonkHonk::new_for_test();
        let path = std::path::PathBuf::from("/tmp/hh_new_sounds");
        let before = app.config.sound_directories.len();

        let _ = app.update(Message::SoundDirectoryPickResult(Some(path.clone())));

        assert_eq!(app.config.sound_directories.len(), before + 1);
        assert!(app.config.sound_directories.contains(&path));
    }

    #[test]
    fn sound_directory_pick_none_is_noop() {
        let mut app = HonkHonk::new_for_test();
        let before = app.config.sound_directories.clone();

        let _ = app.update(Message::SoundDirectoryPickResult(None));

        assert_eq!(app.config.sound_directories, before);
    }
}
