use std::path::{Path, PathBuf};

use crate::state::error::ConfigError;

use super::AppConfig;

const CONFIG_DIR_NAME: &str = "honkhonk";
const CONFIG_FILE_NAME: &str = "config.json";

impl AppConfig {
    /// Returns the config file path under XDG_CONFIG_HOME.
    fn config_path() -> Result<PathBuf, ConfigError> {
        let proj_dirs = directories::ProjectDirs::from("", "", CONFIG_DIR_NAME)
            .ok_or(ConfigError::NoConfigDir)?;
        Ok(proj_dirs.config_dir().join(CONFIG_FILE_NAME))
    }

    /// Loads config from disk, creating defaults if the file is missing.
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;

        if !path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let contents = read_config(&path)?;
        serde_json::from_str(&contents).map_err(|source| ConfigError::Deserialize {
            path: path.display().to_string(),
            source,
        })
    }

    /// Persists config to disk, creating parent directories as needed.
    pub fn save(&self) -> Result<(), ConfigError> {
        self.save_to(&Self::config_path()?)
    }

    /// Loads config from a specific path (for testing).
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            let config = Self::default();
            config.save_to(path)?;
            return Ok(config);
        }

        let contents = read_config(path)?;
        serde_json::from_str(&contents).map_err(|source| ConfigError::Deserialize {
            path: path.display().to_string(),
            source,
        })
    }

    /// Saves config to a specific path (for testing).
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        create_parent(path)?;
        let json = serde_json::to_string_pretty(self).map_err(|source| ConfigError::Serialize {
            path: path.display().to_string(),
            source,
        })?;
        std::fs::write(path, json).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })
    }
}

fn read_config(path: &Path) -> Result<String, ConfigError> {
    std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn create_parent(path: &Path) -> Result<(), ConfigError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|source| ConfigError::DirectoryCreation {
        path: parent.display().to_string(),
        source,
    })
}
