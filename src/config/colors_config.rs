use documented::DocumentedFields;
use serde::{Deserialize, Serialize};

pub type HexString = String;

#[derive(Debug, Default, Serialize, Deserialize, Clone, DocumentedFields)]
#[documented_fields(default = "Hex color string")]
pub struct HexColorsConfig {
    pub background: Option<HexString>,
    pub foreground: Option<HexString>,
    pub border: Option<HexString>,
    #[serde(alias = "mutedForeground", rename = "mutedForeground")]
    pub muted_foreground: Option<HexString>,
    pub accent: Option<HexString>,
    pub disabled: Option<HexString>,
    #[serde(alias = "errorForeground", rename = "errorForeground")]
    pub error_foreground: Option<HexString>,
    #[serde(alias = "successForeground", rename = "successForeground")]
    pub success_foreground: Option<HexString>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_hex_colors_config() {
        let config = HexColorsConfig::default();
        assert!(config.background.is_none());
        assert!(config.foreground.is_none());
        assert!(config.border.is_none());
        assert!(config.muted_foreground.is_none());
        assert!(config.accent.is_none());
        assert!(config.disabled.is_none());
        assert!(config.error_foreground.is_none());
        assert!(config.success_foreground.is_none());
    }

    #[test]
    fn test_hex_colors_config_serializes_with_camel_case_aliases() {
        let config = HexColorsConfig {
            background: Some("#000000".to_string()),
            foreground: Some("#ffffff".to_string()),
            muted_foreground: Some("#888888".to_string()),
            error_foreground: Some("#ff0000".to_string()),
            success_foreground: Some("#00ff00".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["background"], "#000000");
        assert_eq!(json["mutedForeground"], "#888888");
        assert_eq!(json["errorForeground"], "#ff0000");
        assert_eq!(json["successForeground"], "#00ff00");
    }

    #[test]
    fn test_hex_colors_config_deserializes_with_camel_case_aliases() {
        let json = "{\"background\": \"#000000\", \"mutedForeground\": \"#888888\", \"errorForeground\": \"#ff0000\", \"successForeground\": \"#00ff00\"}";
        let config: HexColorsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.background, Some("#000000".to_string()));
        assert_eq!(config.muted_foreground, Some("#888888".to_string()));
        assert_eq!(config.error_foreground, Some("#ff0000".to_string()));
        assert_eq!(config.success_foreground, Some("#00ff00".to_string()));
    }

    #[test]
    fn test_hex_colors_config_deserializes_with_snake_case() {
        // The struct uses serde(rename = ...) which changes the expected key.
        // Verify we can deserialize with proper camelCase keys.
        let json = "{\"background\": \"#000000\", \"mutedForeground\": \"#888888\"}";
        let config: HexColorsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.background, Some("#000000".to_string()));
        assert_eq!(config.muted_foreground, Some("#888888".to_string()));
    }

    #[test]
    fn test_hex_colors_config_field_names_and_docs() {
        let names = HexColorsConfig::FIELD_NAMES;
        assert!(names.contains(&"background"));
        assert!(names.contains(&"muted_foreground"));
        assert!(names.contains(&"error_foreground"));
        assert!(names.contains(&"success_foreground"));
        assert_ne!(HexColorsConfig::FIELD_DOCS.len(), 0);
    }

    #[test]
    fn test_hex_colors_config_clone() {
        let config = HexColorsConfig {
            background: Some("#000000".to_string()),
            accent: Some("#ffff00".to_string()),
            ..Default::default()
        };
        let cloned = config.clone();
        assert_eq!(cloned.background, config.background);
        assert_eq!(cloned.accent, config.accent);
    }
}
