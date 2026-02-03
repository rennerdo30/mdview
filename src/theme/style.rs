//! egui Style generation from theme configuration
//!
//! Creates a refined, modern aesthetic inspired by Linear and modern IDEs.

#![allow(dead_code)]

use egui::{Color32, FontId, Rounding, Shadow, Stroke, Style, Visuals, Vec2};

use crate::config::Config;

/// Color palette for the refined dark theme
pub mod palette {
    use egui::Color32;

    // Base colors - deep, rich backgrounds
    pub const BG_DARKEST: Color32 = Color32::from_rgb(13, 13, 15);      // #0d0d0f
    pub const BG_DARK: Color32 = Color32::from_rgb(18, 18, 22);         // #121216
    pub const BG_BASE: Color32 = Color32::from_rgb(24, 24, 30);         // #18181e
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(32, 32, 40);     // #202028
    pub const BG_HOVER: Color32 = Color32::from_rgb(42, 42, 52);        // #2a2a34
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(52, 52, 64);       // #343440

    // Text colors - carefully balanced for readability
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(237, 237, 242);   // #edeff2
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 176); // #a0a0b0
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(100, 100, 116);     // #646474
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(70, 70, 82);     // #464652

    // Accent colors - teal/cyan for a distinctive look
    pub const ACCENT: Color32 = Color32::from_rgb(56, 189, 186);        // #38bdba - teal
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(72, 205, 202);  // #48cdca
    pub const ACCENT_MUTED: Color32 = Color32::from_rgb(56, 189, 186);  // with alpha

    // Semantic colors
    pub const SUCCESS: Color32 = Color32::from_rgb(80, 200, 120);       // #50c878
    pub const WARNING: Color32 = Color32::from_rgb(255, 183, 77);       // #ffb74d
    pub const ERROR: Color32 = Color32::from_rgb(239, 83, 80);          // #ef5350

    // Border colors
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 38, 48);   // #262630
    pub const BORDER_DEFAULT: Color32 = Color32::from_rgb(50, 50, 62);  // #32323e
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(65, 65, 80);   // #414150

    // Selection - using a semi-transparent teal
    pub const SELECTION: Color32 = Color32::from_rgb(40, 60, 70);

    // Light theme palette
    pub mod light {
        use egui::Color32;

        pub const BG_BASE: Color32 = Color32::from_rgb(252, 252, 253);     // #fcfcfd
        pub const BG_ELEVATED: Color32 = Color32::from_rgb(255, 255, 255); // #ffffff
        pub const BG_HOVER: Color32 = Color32::from_rgb(245, 245, 248);    // #f5f5f8
        pub const BG_ACTIVE: Color32 = Color32::from_rgb(225, 225, 235);   // #e1e1eb
        pub const BG_SIDEBAR: Color32 = Color32::from_rgb(248, 248, 250);  // #f8f8fa

        pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(28, 28, 35);   // #1c1c23
        pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(90, 90, 105);// #5a5a69
        pub const TEXT_MUTED: Color32 = Color32::from_rgb(140, 140, 155);  // #8c8c9b
        pub const TEXT_DISABLED: Color32 = Color32::from_rgb(180, 180, 190); // #b4b4be

        pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(235, 235, 240);
        pub const BORDER_DEFAULT: Color32 = Color32::from_rgb(220, 220, 228);

        pub const ACCENT: Color32 = Color32::from_rgb(45, 160, 158);      // darker teal for contrast
    }
}

/// Create a refined egui Style
pub fn create_style(theme_name: &str, config: &Config) -> Style {
    let mut style = Style::default();
    let is_dark = theme_name.to_lowercase() != "light";

    if is_dark {
        apply_dark_theme(&mut style, config);
    } else {
        apply_light_theme(&mut style, config);
    }

    // Common spacing and sizing
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    style.spacing.indent = 20.0;
    style.spacing.slider_width = 180.0;
    style.spacing.combo_width = 160.0;

    // Refined rounding
    style.visuals.window_rounding = Rounding::same(8.0);
    style.visuals.menu_rounding = Rounding::same(8.0);

    // Apply font sizes
    let base_size = config.theme.fonts.size;
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::proportional(base_size * 0.85),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        FontId::proportional(base_size),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::proportional(base_size * 0.95),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::proportional(base_size * 1.3),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::monospace(base_size * 0.9),
    );

    // Apply line height spacing - egui uses item_spacing.y as the baseline for line spacing
    let line_height = config.theme.fonts.line_height;
    style.spacing.item_spacing.y = (base_size * (line_height - 1.0)).max(4.0);

    // Animation
    style.animation_time = 0.12;

    style
}

fn apply_dark_theme(style: &mut Style, config: &Config) {
    let mut visuals = Visuals::dark();

    // Get custom colors or fall back to defaults
    let bg_color = config.theme.colors.background
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::BG_BASE);
    let code_bg_color = config.theme.colors.code_background
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::BG_DARK);
    let sidebar_bg = config.theme.colors.sidebar_background
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::BG_DARK);

    // Background colors
    visuals.panel_fill = bg_color;
    visuals.window_fill = palette::BG_ELEVATED;
    visuals.extreme_bg_color = palette::BG_DARKEST;
    visuals.faint_bg_color = sidebar_bg;
    visuals.code_bg_color = code_bg_color;

    // Text - use custom color if provided
    let text_color = config.theme.colors.text
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::TEXT_PRIMARY);
    visuals.override_text_color = Some(text_color);

    // Selection - use custom color if provided
    let selection_color = config.theme.colors.selection
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::SELECTION);
    visuals.selection.bg_fill = selection_color;
    visuals.selection.stroke = Stroke::new(1.0, palette::ACCENT);

    // Hyperlinks - use custom link color if provided
    let link_color = config.theme.colors.link
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::ACCENT);
    visuals.hyperlink_color = link_color;

    // Window shadow
    visuals.window_shadow = Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 20.0,
        spread: 0.0,
        color: Color32::from_black_alpha(80),
    };

    visuals.popup_shadow = Shadow {
        offset: Vec2::new(0.0, 2.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(60),
    };

    // Widget styling
    // Non-interactive (labels, etc.)
    visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.noninteractive.weak_bg_fill = palette::BG_ELEVATED;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette::BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette::TEXT_SECONDARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

    // Inactive widgets (buttons, etc.)
    visuals.widgets.inactive.bg_fill = palette::BG_ELEVATED;
    visuals.widgets.inactive.weak_bg_fill = palette::BG_ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette::BORDER_DEFAULT);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette::TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    // Hovered
    visuals.widgets.hovered.bg_fill = palette::BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = palette::BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette::BORDER_STRONG);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette::TEXT_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    // Active (pressed)
    visuals.widgets.active.bg_fill = palette::BG_ACTIVE;
    visuals.widgets.active.weak_bg_fill = palette::BG_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette::ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette::TEXT_PRIMARY);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    // Open (expanded menus, etc.)
    visuals.widgets.open.bg_fill = palette::BG_ACTIVE;
    visuals.widgets.open.weak_bg_fill = palette::BG_ACTIVE;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, palette::BORDER_STRONG);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, palette::TEXT_PRIMARY);
    visuals.widgets.open.rounding = Rounding::same(6.0);

    // Separators and resize handles
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette::BORDER_SUBTLE);

    style.visuals = visuals;
}

fn apply_light_theme(style: &mut Style, config: &Config) {
    let mut visuals = Visuals::light();

    // Get custom colors or fall back to defaults
    let bg_color = config.theme.colors.background
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::light::BG_BASE);
    let code_bg_color = config.theme.colors.code_background
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::light::BG_HOVER);

    visuals.panel_fill = bg_color;
    visuals.window_fill = palette::light::BG_ELEVATED;
    visuals.extreme_bg_color = palette::light::BG_BASE;
    visuals.faint_bg_color = palette::light::BG_HOVER;
    visuals.code_bg_color = code_bg_color;

    // Text - use custom color if provided
    let text_color = config.theme.colors.text
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::light::TEXT_PRIMARY);
    visuals.override_text_color = Some(text_color);

    // Hyperlinks - use custom link color if provided
    let link_color = config.theme.colors.link
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(palette::light::ACCENT);
    visuals.hyperlink_color = link_color;

    // Selection - use custom color if provided
    let selection_color = config.theme.colors.selection
        .as_ref()
        .map(|c| parse_hex_color(c))
        .unwrap_or(Color32::from_rgba_unmultiplied(45, 160, 158, 40));
    visuals.selection.bg_fill = selection_color;
    visuals.selection.stroke = Stroke::new(1.0, palette::light::ACCENT);

    visuals.window_shadow = Shadow {
        offset: Vec2::new(0.0, 2.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(20),
    };

    // Widget styling for light theme
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette::light::BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette::light::TEXT_SECONDARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(6.0);

    visuals.widgets.inactive.bg_fill = palette::light::BG_ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette::light::BORDER_DEFAULT);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette::light::TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    visuals.widgets.hovered.bg_fill = palette::light::BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette::light::BORDER_DEFAULT);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette::light::ACCENT);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    style.visuals = visuals;
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
pub fn adjust_brightness(color: Color32, factor: f32) -> Color32 {
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
