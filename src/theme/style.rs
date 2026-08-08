//! egui Style generation from theme configuration
//!
//! Creates a refined, modern aesthetic inspired by Linear and modern IDEs.

#![allow(dead_code)]

use egui::{Color32, FontId, Rounding, Shadow, Stroke, Style, Vec2, Visuals};

use crate::config::Config;

/// Color palette for the refined dark theme
pub mod palette {
    use egui::Color32;

    // Base colors - deep, rich backgrounds
    pub const BG_DARKEST: Color32 = Color32::from_rgb(13, 13, 15); // #0d0d0f
    pub const BG_DARK: Color32 = Color32::from_rgb(18, 18, 22); // #121216
    pub const BG_BASE: Color32 = Color32::from_rgb(24, 24, 30); // #18181e
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(32, 32, 40); // #202028
    pub const BG_HOVER: Color32 = Color32::from_rgb(42, 42, 52); // #2a2a34
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(52, 52, 64); // #343440

    // Text colors - carefully balanced for readability.
    // Contrast ratios are measured against BG_DARK (#121216), the darkest surface
    // that carries text (side panels, status bar, menu bar).
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(237, 237, 242); // #edeff2 - 15.9:1
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 176); // #a0a0b0 - 7.2:1
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(138, 138, 155); // #8a8a9b - 5.5:1
    /// Only for genuinely inert affordances (disabled buttons, placeholders): 3.6:1.
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(108, 108, 124); // #6c6c7c

    // Accent colors - teal/cyan for a distinctive look
    pub const ACCENT: Color32 = Color32::from_rgb(56, 189, 186); // #38bdba - teal, 8.2:1
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(72, 205, 202); // #48cdca
    /// Translucent accent for overlays and drop targets (#38bdba at ~16% alpha,
    /// stored premultiplied so it can be a `const`).
    pub const ACCENT_TINT: Color32 = Color32::from_rgba_premultiplied(9, 30, 29, 40);

    // Semantic colors
    pub const SUCCESS: Color32 = Color32::from_rgb(80, 200, 120); // #50c878
    pub const WARNING: Color32 = Color32::from_rgb(255, 183, 77); // #ffb74d
    pub const ERROR: Color32 = Color32::from_rgb(239, 83, 80); // #ef5350
    pub const WARNING_BG: Color32 = Color32::from_rgb(80, 40, 40); // dark warning bg
    pub const WARNING_TEXT: Color32 = Color32::from_rgb(255, 180, 180); // dark warning text

    // Border colors
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 38, 48); // #262630
    pub const BORDER_DEFAULT: Color32 = Color32::from_rgb(50, 50, 62); // #32323e
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(65, 65, 80); // #414150

    // Selection - using a semi-transparent teal
    pub const SELECTION: Color32 = Color32::from_rgb(40, 60, 70);

    // Markdown content colors
    pub const CODE_BG: Color32 = Color32::from_rgb(30, 35, 45); // #1e232d
    pub const CODE_TEXT: Color32 = Color32::from_rgb(230, 176, 150); // #e6b096 - 8.0:1 on CODE_BG
    pub const CODE_LINE_NUMBER: Color32 = Color32::from_rgb(128, 138, 158); // #808a9e
    pub const QUOTE_TEXT: Color32 = Color32::from_rgb(184, 184, 198); // #b8b8c6
    pub const QUOTE_BAR: Color32 = Color32::from_rgb(86, 86, 104); // #565668

    // Light theme palette. Contrast ratios are measured against BG_SIDEBAR (#f8f8fa),
    // the darkest light surface that carries text.
    pub mod light {
        use egui::Color32;

        pub const BG_BASE: Color32 = Color32::from_rgb(252, 252, 253); // #fcfcfd
        pub const BG_ELEVATED: Color32 = Color32::from_rgb(255, 255, 255); // #ffffff
        pub const BG_HOVER: Color32 = Color32::from_rgb(240, 240, 245); // #f0f0f5
        pub const BG_ACTIVE: Color32 = Color32::from_rgb(225, 225, 235); // #e1e1eb
        pub const BG_SIDEBAR: Color32 = Color32::from_rgb(248, 248, 250); // #f8f8fa

        pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(28, 28, 35); // #1c1c23 - 15.0:1
        pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(90, 90, 105); // #5a5a69 - 6.4:1
        pub const TEXT_MUTED: Color32 = Color32::from_rgb(106, 106, 122); // #6a6a7a - 5.0:1
        /// Only for genuinely inert affordances (disabled buttons, placeholders): 3.5:1.
        pub const TEXT_DISABLED: Color32 = Color32::from_rgb(132, 132, 145); // #84_8491

        pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(230, 230, 236);
        pub const BORDER_DEFAULT: Color32 = Color32::from_rgb(214, 214, 224);
        pub const BORDER_STRONG: Color32 = Color32::from_rgb(180, 180, 194);

        /// Teal dark enough to pass AA as body text on light surfaces (5.0:1).
        pub const ACCENT: Color32 = Color32::from_rgb(13, 124, 122); // #0d7c7a
        pub const ACCENT_HOVER: Color32 = Color32::from_rgb(9, 102, 100); // #096664
        /// Translucent accent for overlays and drop targets (#0d7c7a at ~13% alpha,
        /// stored premultiplied so it can be a `const`).
        pub const ACCENT_TINT: Color32 = Color32::from_rgba_premultiplied(2, 16, 15, 32);

        pub const SUCCESS: Color32 = Color32::from_rgb(22, 128, 88); // #168058 - 4.7:1
        pub const WARNING_BG: Color32 = Color32::from_rgb(255, 240, 240); // light warning bg
        pub const WARNING_TEXT: Color32 = Color32::from_rgb(166, 46, 46); // #a62e2e - 6.0:1

        pub const CODE_BG: Color32 = Color32::from_rgb(243, 244, 248); // #f3f4f8
        pub const CODE_TEXT: Color32 = Color32::from_rgb(168, 58, 58); // #a83a3a - 6.1:1 on CODE_BG
        pub const CODE_LINE_NUMBER: Color32 = Color32::from_rgb(130, 136, 150); // #828896
        pub const QUOTE_TEXT: Color32 = Color32::from_rgb(84, 84, 98); // #545462
        pub const QUOTE_BAR: Color32 = Color32::from_rgb(196, 196, 208); // #c4c4d0
    }
}

/// Single-glyph icons used in the chrome.
///
/// egui renders text with its own bundled fonts plus whatever system fonts could be loaded,
/// and a glyph that is missing everywhere shows up as an empty box. Only glyphs that are
/// covered by egui's bundled fonts belong here - `ui_theme::ui_icons_have_glyphs` guards that.
pub mod icon {
    pub const FOLDER: &str = "\u{1F4C1}";
    pub const FOLDER_OPEN: &str = "\u{1F4C2}";
    pub const DOCUMENT: &str = "\u{1F4C4}";
    pub const WARNING: &str = "\u{26A0}";
    /// Status dot, e.g. next to "Watching"
    pub const DOT: &str = "\u{2022}";
    pub const CLOSE: &str = "\u{00D7}";
    pub const REFRESH: &str = "\u{21BB}";
    pub const ELLIPSIS: &str = "\u{2026}";
    pub const MINUS: &str = "\u{2212}";
    pub const PLUS: &str = "+";

    /// All icons, for the glyph-coverage test.
    pub const ALL: &[&str] = &[
        FOLDER,
        FOLDER_OPEN,
        DOCUMENT,
        WARNING,
        DOT,
        CLOSE,
        REFRESH,
        ELLIPSIS,
        MINUS,
        PLUS,
    ];
}

/// Spacing scale in points. Every gap in the UI should come from this scale so the
/// vertical rhythm stays consistent (4pt base grid).
pub mod space {
    pub const XXS: f32 = 2.0;
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
    pub const HUGE: f32 = 48.0;
}

/// Corner radius scale in points.
pub mod radius {
    pub const XS: f32 = 2.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const XL: f32 = 12.0;
}

/// Type scale multipliers applied to the configured base font size.
pub mod type_scale {
    pub const CAPTION: f32 = 0.8;
    pub const SMALL: f32 = 0.9;
    pub const BODY: f32 = 1.0;
    pub const TITLE: f32 = 1.3;
    pub const DISPLAY: f32 = 3.0;
}

/// Semantic colors for one theme, resolved once per render instead of branching on
/// `is_dark` at every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeColors {
    pub is_dark: bool,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_tint: Color32,
    pub content_bg: Color32,
    pub sidebar_bg: Color32,
    pub elevated_bg: Color32,
    pub hover_bg: Color32,
    pub active_bg: Color32,
    pub border_subtle: Color32,
    pub border_default: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub text_disabled: Color32,
    pub success: Color32,
    pub warning_bg: Color32,
    pub warning_text: Color32,
    pub code_bg: Color32,
    pub code_text: Color32,
    pub code_line_number: Color32,
    pub quote_text: Color32,
    pub quote_bar: Color32,
}

impl ThemeColors {
    pub const DARK: Self = Self {
        is_dark: true,
        accent: palette::ACCENT,
        accent_hover: palette::ACCENT_HOVER,
        accent_tint: palette::ACCENT_TINT,
        content_bg: palette::BG_BASE,
        sidebar_bg: palette::BG_DARK,
        elevated_bg: palette::BG_ELEVATED,
        hover_bg: palette::BG_HOVER,
        active_bg: palette::BG_ACTIVE,
        border_subtle: palette::BORDER_SUBTLE,
        border_default: palette::BORDER_DEFAULT,
        border_strong: palette::BORDER_STRONG,
        text_primary: palette::TEXT_PRIMARY,
        text_secondary: palette::TEXT_SECONDARY,
        text_muted: palette::TEXT_MUTED,
        text_disabled: palette::TEXT_DISABLED,
        success: palette::SUCCESS,
        warning_bg: palette::WARNING_BG,
        warning_text: palette::WARNING_TEXT,
        code_bg: palette::CODE_BG,
        code_text: palette::CODE_TEXT,
        code_line_number: palette::CODE_LINE_NUMBER,
        quote_text: palette::QUOTE_TEXT,
        quote_bar: palette::QUOTE_BAR,
    };

    pub const LIGHT: Self = Self {
        is_dark: false,
        accent: palette::light::ACCENT,
        accent_hover: palette::light::ACCENT_HOVER,
        accent_tint: palette::light::ACCENT_TINT,
        content_bg: palette::light::BG_BASE,
        sidebar_bg: palette::light::BG_SIDEBAR,
        elevated_bg: palette::light::BG_ELEVATED,
        hover_bg: palette::light::BG_HOVER,
        active_bg: palette::light::BG_ACTIVE,
        border_subtle: palette::light::BORDER_SUBTLE,
        border_default: palette::light::BORDER_DEFAULT,
        border_strong: palette::light::BORDER_STRONG,
        text_primary: palette::light::TEXT_PRIMARY,
        text_secondary: palette::light::TEXT_SECONDARY,
        text_muted: palette::light::TEXT_MUTED,
        text_disabled: palette::light::TEXT_DISABLED,
        success: palette::light::SUCCESS,
        warning_bg: palette::light::WARNING_BG,
        warning_text: palette::light::WARNING_TEXT,
        code_bg: palette::light::CODE_BG,
        code_text: palette::light::CODE_TEXT,
        code_line_number: palette::light::CODE_LINE_NUMBER,
        quote_text: palette::light::QUOTE_TEXT,
        quote_bar: palette::light::QUOTE_BAR,
    };

    /// Pick the token set matching the active theme mode.
    pub const fn new(is_dark: bool) -> Self {
        if is_dark {
            Self::DARK
        } else {
            Self::LIGHT
        }
    }

    /// Token set for the theme currently installed on the egui context.
    pub fn from_ctx(ctx: &egui::Context) -> Self {
        Self::new(ctx.style().visuals.dark_mode)
    }

    /// Token set for the theme active in the given `Ui`.
    pub fn from_ui(ui: &egui::Ui) -> Self {
        Self::new(ui.visuals().dark_mode)
    }
}

/// Create a refined egui Style
pub fn create_style(theme_name: &str, config: &Config) -> Style {
    let mut style = Style::default();
    let normalized_theme = theme_name.trim().to_lowercase();
    let is_dark = normalized_theme != "light";

    // An unknown name silently renders as dark, which is confusing enough to be worth a log line.
    if !matches!(normalized_theme.as_str(), "dark" | "light") {
        log::warn!(
            "Unknown theme {:?}; falling back to the dark theme. Known themes: dark, light.",
            theme_name
        );
    }

    if is_dark {
        apply_dark_theme(&mut style, config);
    } else {
        apply_light_theme(&mut style, config);
    }

    // Common spacing and sizing
    style.spacing.item_spacing = Vec2::new(space::SM, 6.0);
    style.spacing.window_margin = egui::Margin::same(space::LG);
    style.spacing.button_padding = Vec2::new(space::MD, 6.0);
    style.spacing.menu_margin = egui::Margin::same(space::SM);
    style.spacing.indent = 20.0;
    style.spacing.slider_width = 180.0;
    style.spacing.combo_width = 160.0;
    // Roomier hit targets for buttons, checkboxes and combo boxes.
    style.spacing.interact_size.y = style.spacing.interact_size.y.max(MIN_HIT_TARGET);
    style.spacing.icon_width = 18.0;

    // Refined rounding
    style.visuals.window_rounding = Rounding::same(radius::XL);
    style.visuals.menu_rounding = Rounding::same(radius::LG);

    // Apply font sizes
    let base_size = config.theme.fonts.size;
    style.text_styles.insert(
        egui::TextStyle::Small,
        FontId::proportional(base_size * type_scale::CAPTION),
    );
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(base_size));
    style.text_styles.insert(
        egui::TextStyle::Button,
        FontId::proportional(base_size * type_scale::SMALL),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        FontId::proportional(base_size * type_scale::TITLE),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        FontId::monospace(base_size * type_scale::SMALL),
    );

    // Apply line height spacing - egui uses item_spacing.y as the baseline for line spacing
    let line_height = config.theme.fonts.line_height;
    style.spacing.item_spacing.y = (base_size * (line_height - 1.0)).max(space::XS);

    // Animation: short enough to feel instant, long enough to read as a transition.
    style.animation_time = ANIMATION_TIME;

    style
}

/// Minimum height for interactive widgets, so buttons and checkboxes stay easy to hit.
const MIN_HIT_TARGET: f32 = 22.0;
/// Duration of egui's built-in hover/expand animations, in seconds.
const ANIMATION_TIME: f32 = 0.12;
/// Stroke width used for focus and pressed outlines (2px reads clearly at any DPI).
const FOCUS_STROKE_WIDTH: f32 = 2.0;
/// Stroke width for resting widget borders.
const BORDER_STROKE_WIDTH: f32 = 1.0;
/// How far hovered/pressed widgets grow, for a subtle tactile response.
const HOVER_EXPANSION: f32 = 1.0;

fn apply_dark_theme(style: &mut Style, config: &Config) {
    apply_theme(style, config, Visuals::dark(), ThemeColors::DARK);
}

fn apply_light_theme(style: &mut Style, config: &Config) {
    apply_theme(style, config, Visuals::light(), ThemeColors::LIGHT);
}

/// Resolve a config colour override, falling back to a token.
fn override_or(configured: Option<&String>, fallback: Color32) -> Color32 {
    configured.map(|c| parse_hex_color(c)).unwrap_or(fallback)
}

/// Apply one coherent set of visuals for a theme. Both themes go through this function so
/// every widget state (resting, hovered, pressed/focused, open) stays in sync between them.
fn apply_theme(style: &mut Style, config: &Config, mut visuals: Visuals, colors: ThemeColors) {
    let overrides = &config.theme.colors;

    // Surfaces
    visuals.panel_fill = override_or(overrides.background.as_ref(), colors.content_bg);
    visuals.window_fill = colors.elevated_bg;
    visuals.extreme_bg_color = if colors.is_dark {
        palette::BG_DARKEST
    } else {
        palette::light::BG_BASE
    };
    visuals.faint_bg_color = override_or(overrides.sidebar_background.as_ref(), colors.sidebar_bg);
    visuals.code_bg_color = override_or(overrides.code_background.as_ref(), colors.code_bg);

    // Text and links
    visuals.override_text_color = Some(override_or(overrides.text.as_ref(), colors.text_primary));
    visuals.hyperlink_color = override_or(overrides.link.as_ref(), colors.accent);

    // Selection
    let default_selection = if colors.is_dark {
        palette::SELECTION
    } else {
        colors.accent_tint
    };
    visuals.selection.bg_fill = override_or(overrides.selection.as_ref(), default_selection);
    visuals.selection.stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.accent);

    // Elevation
    let (shadow_alpha, popup_alpha) = if colors.is_dark { (80, 60) } else { (28, 20) };
    visuals.window_shadow = Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 20.0,
        spread: 0.0,
        color: Color32::from_black_alpha(shadow_alpha),
    };
    visuals.popup_shadow = Shadow {
        offset: Vec2::new(0.0, 2.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(popup_alpha),
    };

    let rounding = Rounding::same(radius::MD);

    // Non-interactive (labels, separators, frames)
    visuals.widgets.noninteractive.bg_fill = Color32::TRANSPARENT;
    visuals.widgets.noninteractive.weak_bg_fill = colors.elevated_bg;
    visuals.widgets.noninteractive.bg_stroke =
        Stroke::new(BORDER_STROKE_WIDTH, colors.border_subtle);
    visuals.widgets.noninteractive.fg_stroke =
        Stroke::new(BORDER_STROKE_WIDTH, colors.text_secondary);
    visuals.widgets.noninteractive.rounding = rounding;

    // Resting interactive widgets
    visuals.widgets.inactive.bg_fill = colors.elevated_bg;
    visuals.widgets.inactive.weak_bg_fill = colors.elevated_bg;
    visuals.widgets.inactive.bg_stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.border_default);
    visuals.widgets.inactive.fg_stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.text_primary);
    visuals.widgets.inactive.rounding = rounding;

    // Hover
    visuals.widgets.hovered.bg_fill = colors.hover_bg;
    visuals.widgets.hovered.weak_bg_fill = colors.hover_bg;
    visuals.widgets.hovered.bg_stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.border_strong);
    visuals.widgets.hovered.fg_stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.text_primary);
    visuals.widgets.hovered.rounding = rounding;
    visuals.widgets.hovered.expansion = HOVER_EXPANSION;

    // Pressed and keyboard-focused widgets share this state in egui, so the accent outline
    // here doubles as the focus ring.
    visuals.widgets.active.bg_fill = colors.active_bg;
    visuals.widgets.active.weak_bg_fill = colors.active_bg;
    visuals.widgets.active.bg_stroke = Stroke::new(FOCUS_STROKE_WIDTH, colors.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.text_primary);
    visuals.widgets.active.rounding = rounding;
    visuals.widgets.active.expansion = HOVER_EXPANSION;

    // Open menus and combo boxes
    visuals.widgets.open.bg_fill = colors.active_bg;
    visuals.widgets.open.weak_bg_fill = colors.active_bg;
    visuals.widgets.open.bg_stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.border_strong);
    visuals.widgets.open.fg_stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.text_primary);
    visuals.widgets.open.rounding = rounding;

    visuals.window_stroke = Stroke::new(BORDER_STROKE_WIDTH, colors.border_default);
    visuals.warn_fg_color = colors.warning_text;

    style.visuals = visuals;
}

/// Parse a hex color string to Color32.
/// Supports 6-char (`#rrggbb`) and 3-char (`#rgb`) shorthand formats.
pub fn parse_hex_color(hex: &str) -> Color32 {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
            Color32::from_rgb(r, g, b)
        }
        3 => {
            // Expand shorthand: #rgb -> #rrggbb
            let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(8);
            let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(8);
            let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(8);
            Color32::from_rgb(r << 4 | r, g << 4 | g, b << 4 | b)
        }
        _ => Color32::from_gray(128),
    }
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
