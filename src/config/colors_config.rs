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
