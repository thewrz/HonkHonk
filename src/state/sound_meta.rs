use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

mod persistence;

const META_FILE_NAME: &str = "sound_meta.json";
const CONFIG_DIR_NAME: &str = "honkhonk";
const META_FORMAT_VERSION: u32 = 1;

/// Per-sound user customisations persisted independently of library scan.
/// Keyed by sound ID (deterministic hex hash of file path).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundMeta {
    /// Star / unstar: included in "Favorites" filtered view.
    #[serde(default)]
    pub favorite: bool,
    /// Per-sound volume multiplier applied on top of master volume.
    /// 1.0 = no change. Range: [0.0, 2.0].
    #[serde(default = "default_volume")]
    pub volume: f32,
    /// Optional display-name override. `None` means use the filename stem.
    #[serde(default)]
    pub display_name: Option<String>,
}

fn default_volume() -> f32 {
    1.0
}

impl Default for SoundMeta {
    fn default() -> Self {
        Self {
            favorite: false,
            volume: 1.0,
            display_name: None,
        }
    }
}

impl SoundMeta {
    pub fn is_default(&self) -> bool {
        !self.favorite && (self.volume - 1.0).abs() < f32::EPSILON && self.display_name.is_none()
    }
}

/// In-memory store for all sound metadata, backed by a JSON file.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundMetaStore {
    custom: HashMap<String, SoundMeta>,
    added: BTreeMap<String, u64>,
    writable: bool,
}

impl Default for SoundMetaStore {
    fn default() -> Self {
        Self {
            custom: HashMap::new(),
            added: BTreeMap::new(),
            writable: true,
        }
    }
}

impl SoundMetaStore {
    fn read_protected() -> Self {
        Self {
            writable: false,
            ..Self::default()
        }
    }

    /// Returns metadata for a sound, falling back to default if not set.
    pub fn get(&self, id: &str) -> SoundMeta {
        self.custom.get(id).cloned().unwrap_or_default()
    }

    /// Returns a reference to the metadata if it exists.
    pub fn get_ref(&self, id: &str) -> Option<&SoundMeta> {
        self.custom.get(id)
    }

    /// Upserts metadata for a sound. Removes the entry if it becomes default.
    pub fn set(&mut self, id: String, meta: SoundMeta) {
        if meta.is_default() {
            self.custom.remove(&id);
        } else {
            self.custom.insert(id, meta);
        }
    }

    /// Toggles the favorite flag for a sound, returning the new value.
    pub fn toggle_favorite(&mut self, id: &str) -> bool {
        let mut meta = self.get(id);
        meta.favorite = !meta.favorite;
        let new_val = meta.favorite;
        self.set(id.to_owned(), meta);
        new_val
    }

    /// Sets per-sound volume for a sound.
    pub fn set_volume(&mut self, id: &str, volume: f32) {
        let mut meta = self.get(id);
        meta.volume = volume.clamp(0.0, 2.0);
        self.set(id.to_owned(), meta);
    }

    /// Sets the display name override for a sound. Pass `None` to clear.
    pub fn set_display_name(&mut self, id: &str, name: Option<String>) {
        let mut meta = self.get(id);
        meta.display_name = name;
        self.set(id.to_owned(), meta);
    }

    /// Returns `true` if the sound is a favorite.
    pub fn is_favorite(&self, id: &str) -> bool {
        self.custom.get(id).map(|m| m.favorite).unwrap_or(false)
    }

    /// Returns the per-sound volume multiplier (defaults to 1.0).
    pub fn volume_for(&self, id: &str) -> f32 {
        self.custom.get(id).map(|m| m.volume).unwrap_or(1.0)
    }

    pub fn added_ms(&self, id: &str) -> Option<u64> {
        self.added.get(id).copied()
    }

    pub fn reconcile_added<I, S>(&mut self, ids: I, observed_at_ms: u64, complete: bool) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let observed: BTreeSet<String> = ids.into_iter().map(|id| id.as_ref().to_owned()).collect();
        let mut changed = false;

        for id in &observed {
            if !self.added.contains_key(id) {
                self.added.insert(id.clone(), observed_at_ms);
                changed = true;
            }
        }

        if complete {
            let previous_len = self.added.len();
            self.added.retain(|id, _| observed.contains(id));
            changed |= self.added.len() != previous_len;
        }

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn default_meta_is_not_favorite() {
        let store = SoundMetaStore::default();
        assert!(!store.is_favorite("any-id"));
    }

    #[test]
    fn default_volume_is_one() {
        let store = SoundMetaStore::default();
        let eps = f32::EPSILON;
        assert!((store.volume_for("any-id") - 1.0).abs() < eps);
    }

    #[test]
    fn toggle_favorite_sets_true_then_false() {
        let mut store = SoundMetaStore::default();
        assert!(store.toggle_favorite("id1"));
        assert!(!store.toggle_favorite("id1"));
    }

    #[test]
    fn set_volume_clamps_to_range() {
        let mut store = SoundMetaStore::default();
        store.set_volume("id1", 3.0);
        let eps = f32::EPSILON;
        assert!((store.volume_for("id1") - 2.0).abs() < eps);
        store.set_volume("id1", -0.5);
        assert!((store.volume_for("id1") - 0.0).abs() < eps);
    }

    #[test]
    fn set_volume_in_range_is_preserved() {
        let mut store = SoundMetaStore::default();
        store.set_volume("id1", 1.5);
        let eps = 1e-5_f32;
        assert!((store.volume_for("id1") - 1.5).abs() < eps);
    }

    #[test]
    fn set_cleans_up_default_entries() {
        let mut store = SoundMetaStore::default();
        store.set_volume("id1", 1.5);
        // Reset to default
        store.set("id1".to_owned(), SoundMeta::default());
        assert!(
            store.custom.is_empty(),
            "default meta should be pruned from map"
        );
    }

    #[test]
    fn set_display_name_stores_override() {
        let mut store = SoundMetaStore::default();
        store.set_display_name("id1", Some("My Honk".to_owned()));
        assert_eq!(store.get("id1").display_name.as_deref(), Some("My Honk"));
    }

    #[test]
    fn set_display_name_none_clears_override() {
        let mut store = SoundMetaStore::default();
        store.set_display_name("id1", Some("Override".to_owned()));
        store.set_display_name("id1", None);
        assert!(store.get("id1").display_name.is_none());
    }

    #[test]
    fn save_and_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("meta.json");

        let mut store = SoundMetaStore::default();
        store.toggle_favorite("abc");
        store.set_volume("abc", 1.25);
        store.save_to(&path).unwrap();

        let loaded = SoundMetaStore::load_from(&path);
        assert!(loaded.is_favorite("abc"));
        let eps = 1e-5_f32;
        assert!((loaded.volume_for("abc") - 1.25).abs() < eps);
    }

    #[test]
    fn load_from_missing_file_returns_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let store = SoundMetaStore::load_from(&path);
        assert!(!store.is_favorite("any"));
    }

    #[test]
    fn load_from_corrupt_file_returns_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, b"not json!!!").unwrap();
        let store = SoundMetaStore::load_from(&path);
        assert!(!store.is_favorite("any"));
    }

    #[test]
    fn corrupt_file_cannot_be_overwritten() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, b"not json!!!").unwrap();
        let store = SoundMetaStore::load_from(&path);

        assert!(store.save_to(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"not json!!!");
    }

    #[test]
    fn future_version_file_cannot_be_downgraded() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("future.json");
        let future = r#"{"version":999,"custom":{},"added":{"abc":42}}"#;
        std::fs::write(&path, future).unwrap();
        let store = SoundMetaStore::load_from(&path);

        assert!(store.save_to(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), future);
    }

    #[test]
    fn malformed_envelope_without_version_cannot_be_overwritten() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("malformed-envelope.json");
        let malformed = r#"{"custom":{"abc":{"favorite":true}},"added":{"abc":42}}"#;
        std::fs::write(&path, malformed).unwrap();
        let store = SoundMetaStore::load_from(&path);

        assert!(store.save_to(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
    }

    #[test]
    fn is_default_detects_all_fields_at_default() {
        assert!(SoundMeta::default().is_default());
        assert!(
            !SoundMeta {
                favorite: true,
                ..Default::default()
            }
            .is_default()
        );
    }

    #[test]
    fn reconcile_added_stamps_new_ids_once() {
        let mut store = SoundMetaStore::default();

        assert!(store.reconcile_added(["first", "second"], 1_000, true));
        assert_eq!(store.added_ms("first"), Some(1_000));
        assert_eq!(store.added_ms("second"), Some(1_000));
        assert!(!store.reconcile_added(["first", "second"], 2_000, true));
        assert_eq!(store.added_ms("first"), Some(1_000));
    }

    #[test]
    fn complete_reconcile_prunes_unseen_ids() {
        let mut store = SoundMetaStore::default();
        store.reconcile_added(["kept", "removed"], 1_000, true);

        assert!(store.reconcile_added(["kept"], 2_000, true));
        assert_eq!(store.added_ms("kept"), Some(1_000));
        assert_eq!(store.added_ms("removed"), None);
    }

    #[test]
    fn partial_reconcile_preserves_unseen_ids() {
        let mut store = SoundMetaStore::default();
        store.reconcile_added(["observed", "temporarily-missing"], 1_000, true);

        assert!(!store.reconcile_added(["observed"], 2_000, false));
        assert_eq!(store.added_ms("temporarily-missing"), Some(1_000));
    }

    #[test]
    fn load_from_accepts_legacy_top_level_metadata_map() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        std::fs::write(
            &path,
            r#"{"abc":{"favorite":true,"volume":1.25,"display_name":"Honk"}}"#,
        )
        .unwrap();

        let store = SoundMetaStore::load_from(&path);

        assert!(store.is_favorite("abc"));
        assert_eq!(store.get("abc").display_name.as_deref(), Some("Honk"));
        assert_eq!(store.added_ms("abc"), None);
        store.save_to(&path).unwrap();
        assert!(SoundMetaStore::load_from(&path).is_favorite("abc"));
    }

    #[test]
    fn versioned_store_round_trips_added_timestamps() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("meta.json");
        let mut store = SoundMetaStore::default();
        store.toggle_favorite("abc");
        store.reconcile_added(["abc"], 4_242, true);

        store.save_to(&path).unwrap();
        let loaded = SoundMetaStore::load_from(&path);

        assert!(loaded.is_favorite("abc"));
        assert_eq!(loaded.added_ms("abc"), Some(4_242));
        let json = std::fs::read_to_string(path).unwrap();
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"custom\""));
        assert!(json.contains("\"added\""));
    }

    #[test]
    fn first_seen_timestamps_do_not_create_custom_metadata() {
        let mut store = SoundMetaStore::default();

        store.reconcile_added(["abc"], 1_000, true);

        assert!(store.get_ref("abc").is_none());
    }
}
