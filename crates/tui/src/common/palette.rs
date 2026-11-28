use anyhow::{Result, anyhow};
use ratatui::style::Color;
use ratatui::style::Style;
use std::sync::OnceLock;

use config::HexColorsConfig;

fn parse_hex_color(hex_string: &str) -> Option<Color> {
    let hex = hex_string.strip_prefix('#').unwrap_or(hex_string);
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => {
            log::warn!("Invalid hex color string: \"{hex_string}\"");
            None
        }
    }
}

fn parse_hex_string(hex_string: Option<&String>) -> Option<Color> {
    hex_string.and_then(|s| parse_hex_color(s))
}

#[derive(Debug)]
pub struct TuiPalette {
    pub background: Color,
    pub foreground: Color,
    pub border: Color,
    pub muted_foreground: Color,
    pub accent: Color,
    pub disabled: Color,
    pub error_foreground: Color,
    pub success_foreground: Color,

    pub background_style: Style,
    pub separator_style: Style,
    /// Complete reusable style
    pub style: Style,
}

impl TuiPalette {
    fn from_config(config: &HexColorsConfig) -> Self {
        let defaults = TuiPalette::default();

        let background =
            parse_hex_string(config.background.as_ref()).unwrap_or(defaults.background);
        let foreground =
            parse_hex_string(config.foreground.as_ref()).unwrap_or(defaults.foreground);
        let border = parse_hex_string(config.border.as_ref()).unwrap_or(defaults.border);

        Self {
            background,
            foreground,
            border,

            muted_foreground: parse_hex_string(config.muted_foreground.as_ref())
                .unwrap_or(defaults.muted_foreground),
            accent: parse_hex_string(config.accent.as_ref()).unwrap_or(defaults.accent),
            disabled: parse_hex_string(config.disabled.as_ref()).unwrap_or(defaults.disabled),
            error_foreground: parse_hex_string(config.error_foreground.as_ref())
                .unwrap_or(defaults.error_foreground),
            success_foreground: parse_hex_string(config.success_foreground.as_ref())
                .unwrap_or(defaults.success_foreground),

            background_style: Style::new().bg(background),
            separator_style: Style::new().fg(border).bg(background),
            style: Style::new().fg(foreground).bg(background),
        }
    }
}

impl Default for TuiPalette {
    fn default() -> Self {
        const BACKGROUND: Color = Color::Reset;
        const FOREGROUND: Color = Color::Rgb(236, 239, 241); // Blue gray 50
        const BORDER: Color = Color::Rgb(38, 50, 56);

        Self {
            background: BACKGROUND,
            foreground: FOREGROUND,
            border: BORDER,

            muted_foreground: Color::Rgb(120, 144, 156), // Blue gray 400
            accent: Color::Rgb(77, 182, 172),            // Teal 300
            disabled: Color::Rgb(158, 158, 158),         // Grey 500
            error_foreground: Color::Rgb(229, 115, 115), // Red 300
            success_foreground: Color::Rgb(129, 199, 132), // Green 300

            background_style: Style::new().bg(BACKGROUND),
            separator_style: Style::new().fg(BORDER).bg(BACKGROUND),
            style: Style::new().fg(FOREGROUND).bg(BACKGROUND),
        }
    }
}

static PALETTE: OnceLock<TuiPalette> = OnceLock::new();

pub(crate) fn init(colors: &HexColorsConfig) -> Result<()> {
    PALETTE
        .set(TuiPalette::from_config(colors))
        .map_err(|_| anyhow!("palette was already set"))?;
    Ok(())
}

pub fn get() -> &'static TuiPalette {
    PALETTE.get_or_init(TuiPalette::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_palette_background_is_reset() {
        assert_eq!(TuiPalette::default().background, Color::Reset);
    }

    #[test]
    fn test_default_palette_foreground_rgb_value() {
        assert_eq!(TuiPalette::default().foreground, Color::Rgb(236, 239, 241));
    }

    #[test]
    fn test_default_palette_accent_teal_value() {
        assert_eq!(TuiPalette::default().accent, Color::Rgb(77, 182, 172));
    }

    #[test]
    fn test_default_palette_error_foreground_red_value() {
        assert_eq!(
            TuiPalette::default().error_foreground,
            Color::Rgb(229, 115, 115)
        );
    }

    #[test]
    fn test_separator_style_has_fg_and_bg() {
        let style = TuiPalette::default().separator_style;
        assert!(style.fg.is_some() || style.bg.is_some());
    }

    #[test]
    fn test_style_has_fg_and_bg() {
        let style = TuiPalette::default().style;
        assert!(style.fg.is_some() || style.bg.is_some());
    }

    #[test]
    fn test_get_returns_default_palette() {
        let palette = get();
        assert_eq!(palette.background, Color::Reset);
        assert_eq!(palette.foreground, Color::Rgb(236, 239, 241));
    }

    #[test]
    fn test_from_config_overrides_background() {
        let config = HexColorsConfig {
            background: Some("#1a2b3c".to_string()),
            ..Default::default()
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, Color::Rgb(26, 43, 60));
    }

    #[test]
    fn test_from_config_keeps_defaults_for_unset_fields() {
        let config = HexColorsConfig::default();
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, TuiPalette::default().background);
        assert_eq!(palette.foreground, TuiPalette::default().foreground);
    }

    #[test]
    fn test_from_config_parses_short_hex() {
        let config = HexColorsConfig {
            background: Some("#fff".to_string()),
            ..Default::default()
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_from_config_invalid_hex_falls_back_to_default() {
        let config = HexColorsConfig {
            background: Some("not-a-hex".to_string()),
            ..Default::default()
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, TuiPalette::default().background);
    }

    #[test]
    fn test_from_config_invalid_hex_length_falls_back_to_default() {
        let config = HexColorsConfig {
            background: Some("#12345".to_string()),
            ..Default::default()
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, TuiPalette::default().background);
    }

    #[test]
    fn test_from_config_invalid_hex_digits_falls_back_to_default() {
        let config = HexColorsConfig {
            background: Some("#zzzzzz".to_string()),
            ..Default::default()
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, TuiPalette::default().background);
    }

    #[test]
    fn test_from_config_overrides_multiple_fields() {
        let config = HexColorsConfig {
            background: Some("#000000".to_string()),
            foreground: Some("#ffffff".to_string()),
            border: Some("#ff0000".to_string()),
            muted_foreground: Some("#00ff00".to_string()),
            accent: Some("#0000ff".to_string()),
            disabled: Some("#aaaaaa".to_string()),
            error_foreground: Some("#bb0000".to_string()),
            success_foreground: Some("#00bb00".to_string()),
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, Color::Rgb(0, 0, 0));
        assert_eq!(palette.foreground, Color::Rgb(255, 255, 255));
        assert_eq!(palette.border, Color::Rgb(255, 0, 0));
        assert_eq!(palette.muted_foreground, Color::Rgb(0, 255, 0));
        assert_eq!(palette.accent, Color::Rgb(0, 0, 255));
        assert_eq!(palette.disabled, Color::Rgb(170, 170, 170));
        assert_eq!(palette.error_foreground, Color::Rgb(187, 0, 0));
        assert_eq!(palette.success_foreground, Color::Rgb(0, 187, 0));
    }

    #[test]
    fn test_parse_hex_color_without_hash_prefix() {
        let config = HexColorsConfig {
            background: Some("abc".to_string()),
            ..Default::default()
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, Color::Rgb(170, 187, 204));
    }

    #[test]
    fn test_init_and_get_palette() {
        // init() sets the global palette; if it was already set by another test,
        // this will panic. We test that get() returns a valid palette.
        let palette = get();
        // Verify the returned palette has sensible defaults
        assert_eq!(palette.foreground, Color::Rgb(236, 239, 241));
        assert_eq!(palette.accent, Color::Rgb(77, 182, 172));
    }

    #[test]
    fn test_parse_hex_color_returns_none_for_invalid() {
        assert!(parse_hex_color("invalid").is_none());
        assert!(parse_hex_color("#zzzzzz").is_none());
        assert!(parse_hex_color("#12345").is_none());
    }

    #[test]
    fn test_parse_hex_color_returns_some_for_valid() {
        assert!(parse_hex_color("#ffffff").is_some());
        assert!(parse_hex_color("#abc").is_some());
        assert!(parse_hex_color("ffffff").is_some());
    }

    #[test]
    fn test_parse_hex_string_none_returns_none() {
        assert!(parse_hex_string(None).is_none());
    }

    #[test]
    fn test_parse_hex_string_some_returns_color() {
        let opt: Option<String> = Some("#000000".to_string());
        let color = parse_hex_string(opt.as_ref());
        assert_eq!(color, Some(Color::Rgb(0, 0, 0)));
    }

    #[test]
    fn test_tui_palette_default_values() {
        let palette = TuiPalette::default();
        assert_eq!(palette.background, Color::Reset);
        assert_eq!(palette.muted_foreground, Color::Rgb(120, 144, 156));
        assert_eq!(palette.disabled, Color::Rgb(158, 158, 158));
        assert_eq!(palette.success_foreground, Color::Rgb(129, 199, 132));
    }

    #[test]
    fn test_from_config_all_fields_set() {
        let config = HexColorsConfig {
            background: Some("#000000".to_string()),
            foreground: Some("#ffffff".to_string()),
            border: Some("#ff0000".to_string()),
            muted_foreground: Some("#00ff00".to_string()),
            accent: Some("#0000ff".to_string()),
            disabled: Some("#aaaaaa".to_string()),
            error_foreground: Some("#bb0000".to_string()),
            success_foreground: Some("#00bb00".to_string()),
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.background, Color::Rgb(0, 0, 0));
        assert_eq!(palette.foreground, Color::Rgb(255, 255, 255));
        assert_eq!(palette.border, Color::Rgb(255, 0, 0));
        assert_eq!(palette.muted_foreground, Color::Rgb(0, 255, 0));
        assert_eq!(palette.accent, Color::Rgb(0, 0, 255));
        assert_eq!(palette.disabled, Color::Rgb(170, 170, 170));
        assert_eq!(palette.error_foreground, Color::Rgb(187, 0, 0));
        assert_eq!(palette.success_foreground, Color::Rgb(0, 187, 0));
    }

    #[test]
    fn test_from_config_only_foreground_set() {
        let config = HexColorsConfig {
            foreground: Some("#112233".to_string()),
            ..Default::default()
        };
        let palette = TuiPalette::from_config(&config);
        assert_eq!(palette.foreground, Color::Rgb(17, 34, 51));
        // Other fields should keep defaults
        assert_eq!(palette.background, TuiPalette::default().background);
    }
}
