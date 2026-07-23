use serde::{Deserialize, Serialize};

#[cfg(test)]
use super::AppConfig;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortPref {
    #[serde(default)]
    key: String,
    #[serde(default)]
    direction: String,
}

impl SortPref {
    pub fn new(key: impl Into<String>, direction: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            direction: direction.into(),
        }
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn direction(&self) -> &str {
        &self.direction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_preferences_round_trip_through_app_config() {
        let mut config = AppConfig::default();
        config
            .sort_prefs
            .insert("tiles".into(), SortPref::new("modified", "descending"));

        let json = serde_json::to_string(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(loaded.sort_prefs, config.sort_prefs);
    }

    #[test]
    fn unknown_preference_data_does_not_reject_config() {
        let mut config = AppConfig::default();
        config
            .sort_prefs
            .insert("tiles".into(), SortPref::new("future-key", "sideways"));

        let json = serde_json::to_string(&config).unwrap();
        let loaded: AppConfig = serde_json::from_str(&json).unwrap();
        let pref = loaded.sort_prefs.get("tiles").unwrap();

        assert_eq!(pref.key(), "future-key");
        assert_eq!(pref.direction(), "sideways");
    }

    #[test]
    fn missing_sort_preferences_use_empty_default() {
        let json = serde_json::to_value(AppConfig::default()).unwrap();
        let mut object = json.as_object().unwrap().clone();
        object.remove("sort_prefs");

        let loaded: AppConfig = serde_json::from_value(serde_json::Value::Object(object)).unwrap();

        assert!(loaded.sort_prefs.is_empty());
    }

    #[test]
    fn incomplete_preference_data_does_not_reject_config() {
        let mut value = serde_json::to_value(AppConfig::default()).unwrap();
        value["sort_prefs"] = serde_json::json!({"tiles": {}});

        let loaded: AppConfig = serde_json::from_value(value).unwrap();
        let pref = loaded.sort_prefs.get("tiles").unwrap();

        assert_eq!(pref, &SortPref::default());
    }
}
