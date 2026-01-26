//! egui Style generation from theme configuration

use egui::{Color32, Style, Visuals};

use crate::config::defaults::{dark_theme, light_theme};
use crate::config::Config;

/// Create an egui Style from theme name and config
pub fn create_style(theme_name: &str, config: &Config) -> Style {
    let mut style = Style::default();

    // Get base colors from theme
    let (bg, text, heading, link, code_bg, code_text, sidebar_bg, selection) =
        match theme_name.to_lowercase().as_str() {
            "light" => (
                light_theme::BACKGROUND,
                light_theme::TEXT,
                light_theme::HEADING,
                light_theme::LINK,
                light_theme::CODE_BACKGROUND,
                light_theme::CODE_TEXT,
                light_theme::SIDEBAR_BACKGROUND,
                light_theme::SELECTION,
            ),
            _ => (
                dark_theme::BACKGROUND,
                dark_theme::TEXT,
                dark_theme::HEADING,
                dark_theme::LINK,
                dark_theme::CODE_BACKGROUND,
                dark_theme::CODE_TEXT,
                dark_theme::SIDEBAR_BACKGROUND,
                dark_theme::SELECTION,
            ),
        };

    // Apply custom color overrides from config
    let bg = config
        .theme
        .colors
        .background
        .as_deref()
        .unwrap_or(bg);
    let text = config
        .theme
        .colors
        .text
        .as_deref()
        .unwrap_or(text);
    let selection = config
        .theme
        .colors
        .selection
        .as_deref()
        .unwrap_or(selection);

    // Parse colors
    let bg_color = parse_hex_color(bg);
    let text_color = parse_hex_color(text);
    let selection_color = parse_hex_color(selection);

    // Create visuals based on theme
    let is_dark = theme_name.to_lowercase() != "light";
    let mut visuals = if is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    // Apply custom colors
    visuals.panel_fill = bg_color;
    visuals.window_fill = bg_color;
    visuals.extreme_bg_color = bg_color;
    visuals.faint_bg_color = adjust_brightness(bg_color, if is_dark { 1.1 } else { 0.95 });

    visuals.override_text_color = Some(text_color);

    visuals.selection.bg_fill = selection_color;
    visuals.selection.stroke.color = text_color;

    // Widget colors
    visuals.widgets.noninteractive.bg_fill = adjust_brightness(bg_color, if is_dark { 1.2 } else { 0.9 });
    visuals.widgets.inactive.bg_fill = adjust_brightness(bg_color, if is_dark { 1.3 } else { 0.85 });
    visuals.widgets.hovered.bg_fill = adjust_brightness(bg_color, if is_dark { 1.4 } else { 0.8 });
    visuals.widgets.active.bg_fill = adjust_brightness(bg_color, if is_dark { 1.5 } else { 0.75 });

    style.visuals = visuals;

    // Apply font size from config
    let font_size = config.theme.fonts.size;
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(font_size),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(font_size * 1.5),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::monospace(font_size * 0.9),
    );
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(font_size * 0.85),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(font_size),
    );

    // Spacing
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.window_margin = egui::Margin::same(12.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    style
}

/// Parse a hex color string to Color32
pub fn parse_hex_color(hex: &str) -> Color32 {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Color32::from_gray(128);
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);

    Color32::from_rgb(r, g, b)
}

/// Adjust brightness of a color
fn adjust_brightness(color: Color32, factor: f32) -> Color32 {
    let r = ((color.r() as f32 * factor).min(255.0)) as u8;
    let g = ((color.g() as f32 * factor).min(255.0)) as u8;
    let b = ((color.b() as f32 * factor).min(255.0)) as u8;
    Color32::from_rgb(r, g, b)
}

/// Convert Color32 to hex string
pub fn color_to_hex(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        assert_eq!(parse_hex_color("#ff0000"), Color32::from_rgb(255, 0, 0));
        assert_eq!(parse_hex_color("00ff00"), Color32::from_rgb(0, 255, 0));
        assert_eq!(parse_hex_color("#0000ff"), Color32::from_rgb(0, 0, 255));
    }

    #[test]
    fn test_color_to_hex() {
        assert_eq!(color_to_hex(Color32::from_rgb(255, 0, 0)), "#ff0000");
        assert_eq!(color_to_hex(Color32::from_rgb(0, 255, 0)), "#00ff00");
    }

    #[test]
    fn test_create_style() {
        let config = Config::default();
        let style = create_style("dark", &config);
        assert!(style.visuals.dark_mode);

        let style = create_style("light", &config);
        assert!(!style.visuals.dark_mode);
    }
}
