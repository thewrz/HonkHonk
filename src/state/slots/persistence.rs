use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{CONFIG_DIR_NAME, SLOT_COUNT, SLOTS_FILE_NAME, SlotContent, SlotMap};
use crate::state::error::ConfigError;

const SLOT_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedSlotMap {
    version: u32,
    slots: [Option<PersistedSlotContent>; SLOT_COUNT],
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum PersistedSlotContent {
    Sound(PathBuf),
    Object(PersistedSlotObject),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum PersistedSlotObject {
    Macro { id: String },
}

impl From<&SlotContent> for PersistedSlotContent {
    fn from(content: &SlotContent) -> Self {
        match content {
            SlotContent::Sound(path) => Self::Sound(path.clone()),
            SlotContent::Macro(id) => Self::Object(PersistedSlotObject::Macro { id: id.clone() }),
        }
    }
}

impl TryFrom<PersistedSlotContent> for SlotContent {
    type Error = super::MacroIdError;

    fn try_from(content: PersistedSlotContent) -> Result<Self, Self::Error> {
        match content {
            PersistedSlotContent::Sound(path) => Ok(Self::Sound(path)),
            PersistedSlotContent::Object(PersistedSlotObject::Macro { id }) => {
                Self::macro_with_id(id)
            }
        }
    }
}

impl SlotMap {
    fn slots_path() -> Result<PathBuf, ConfigError> {
        let project_dirs = directories::ProjectDirs::from("", "", CONFIG_DIR_NAME)
            .ok_or(ConfigError::NoConfigDir)?;
        Ok(project_dirs.config_dir().join(SLOTS_FILE_NAME))
    }

    /// Loads from the default XDG path. Unsupported or unreadable data becomes
    /// an empty read-protected map so startup cannot destroy the source.
    pub fn load() -> Self {
        let Ok(path) = Self::slots_path() else {
            return Self::read_protected();
        };
        Self::load_from(&path)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        self.save_to(&Self::slots_path()?)
    }

    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        if !self.writable {
            return Err(ConfigError::UnsafeSlotsOverwrite {
                path: path.display().to_string(),
            });
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ConfigError::DirectoryCreation {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let persisted = self.to_persisted();
        let json =
            serde_json::to_string_pretty(&persisted).map_err(|source| ConfigError::Serialize {
                path: path.display().to_string(),
                source,
            })?;
        std::fs::write(path, json).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })
    }

    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(json) => Self::deserialize(&json).unwrap_or_else(Self::read_protected),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(_) => Self::read_protected(),
        }
    }

    fn deserialize(json: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(json).ok()?;
        if value.is_array() {
            let legacy: [Option<PathBuf>; SLOT_COUNT] = serde_json::from_value(value).ok()?;
            return Some(Self::from_legacy(legacy));
        }

        let persisted: PersistedSlotMap = serde_json::from_value(value).ok()?;
        if persisted.version != SLOT_FORMAT_VERSION {
            return None;
        }
        Self::from_persisted(persisted.slots)
    }

    fn from_legacy(legacy: [Option<PathBuf>; SLOT_COUNT]) -> Self {
        Self {
            slots: legacy.map(|slot| slot.map(SlotContent::Sound)),
            writable: true,
        }
    }

    fn from_persisted(persisted: [Option<PersistedSlotContent>; SLOT_COUNT]) -> Option<Self> {
        let mut map = Self::default();
        for (index, content) in persisted.into_iter().enumerate() {
            map.slots[index] = content.map(SlotContent::try_from).transpose().ok()?;
        }
        Some(map)
    }

    fn to_persisted(&self) -> PersistedSlotMap {
        PersistedSlotMap {
            version: SLOT_FORMAT_VERSION,
            slots: std::array::from_fn(|index| {
                self.slots[index].as_ref().map(PersistedSlotContent::from)
            }),
        }
    }
}
