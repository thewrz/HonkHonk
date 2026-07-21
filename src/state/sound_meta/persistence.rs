use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{CONFIG_DIR_NAME, META_FILE_NAME, META_FORMAT_VERSION, SoundMeta, SoundMetaStore};
use crate::state::error::ConfigError;

#[derive(Debug, Serialize, Deserialize)]
struct PersistedSoundMeta {
    version: u32,
    #[serde(default)]
    custom: HashMap<String, SoundMeta>,
    #[serde(default)]
    added: BTreeMap<String, u64>,
}

impl SoundMetaStore {
    fn meta_path() -> Result<PathBuf, ConfigError> {
        let project_dirs = directories::ProjectDirs::from("", "", CONFIG_DIR_NAME)
            .ok_or(ConfigError::NoConfigDir)?;
        Ok(project_dirs.config_dir().join(META_FILE_NAME))
    }

    /// Loads from the default XDG path. Unreadable or unsupported data yields
    /// an empty, write-protected store so startup cannot destroy the source.
    pub fn load() -> Self {
        let Ok(path) = Self::meta_path() else {
            return Self::read_protected();
        };
        Self::load_from(&path)
    }

    /// Persists the store to the default XDG path.
    pub fn save(&self) -> Result<(), ConfigError> {
        self.save_to(&Self::meta_path()?)
    }

    /// Persists the store to an arbitrary path (used in tests).
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        if !self.writable {
            return Err(ConfigError::UnsafeMetadataOverwrite {
                path: path.display().to_string(),
            });
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ConfigError::DirectoryCreation {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let persisted = PersistedSoundMeta {
            version: META_FORMAT_VERSION,
            custom: self.custom.clone(),
            added: self.added.clone(),
        };
        let json =
            serde_json::to_string_pretty(&persisted).map_err(|source| ConfigError::Serialize {
                path: path.display().to_string(),
                source,
            })?;
        std::fs::write(path, json).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Ok(())
    }

    /// Loads from an arbitrary path (used in tests).
    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(json) => Self::deserialize(&json).unwrap_or_else(Self::read_protected),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(_) => Self::read_protected(),
        }
    }

    fn deserialize(json: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(json).ok()?;
        if value.get("version").is_some() {
            return Self::from_versioned(value);
        }
        if value.get("custom").is_some() || value.get("added").is_some() {
            return None;
        }

        let custom = serde_json::from_value(value).ok()?;
        Some(Self {
            custom,
            added: BTreeMap::new(),
            writable: true,
        })
    }

    fn from_versioned(value: serde_json::Value) -> Option<Self> {
        let persisted: PersistedSoundMeta = serde_json::from_value(value).ok()?;
        (persisted.version == META_FORMAT_VERSION).then_some(Self {
            custom: persisted.custom,
            added: persisted.added,
            writable: true,
        })
    }
}
