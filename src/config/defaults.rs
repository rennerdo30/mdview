//! Default configuration values
//!
//! This module provides default values and constants used throughout the application.

/// Default window width
pub const DEFAULT_WINDOW_WIDTH: u32 = 1000;

/// Default window height
pub const DEFAULT_WINDOW_HEIGHT: u32 = 700;

/// Default TOC sidebar width
pub const DEFAULT_TOC_WIDTH: u32 = 250;

/// Minimum TOC sidebar width
pub const MIN_TOC_WIDTH: f32 = 150.0;

/// Maximum TOC sidebar width
pub const MAX_TOC_WIDTH: f32 = 400.0;

/// Default font size
pub const DEFAULT_FONT_SIZE: f32 = 14.0;

/// Default line height multiplier
pub const DEFAULT_LINE_HEIGHT: f32 = 1.5;

/// Default paragraph spacing
pub const DEFAULT_PARAGRAPH_SPACING: f32 = 12.0;

/// Default code block padding
pub const DEFAULT_CODE_PADDING: f32 = 8.0;

/// File watcher debounce duration in milliseconds
pub const WATCHER_DEBOUNCE_MS: u64 = 100;

/// Maximum render cache entries
pub const MAX_CACHE_ENTRIES: usize = 500;

/// Status message display duration in seconds
pub const STATUS_MESSAGE_DURATION_SECS: u64 = 3;

/// Default dark theme colors
pub mod dark_theme {
    pub const BACKGROUND: &str = "#1e1e1e";
    pub const TEXT: &str = "#d4d4d4";
    pub const HEADING: &str = "#569cd6";
    pub const LINK: &str = "#4ec9b0";
    pub const CODE_BACKGROUND: &str = "#2d2d2d";
    pub const CODE_TEXT: &str = "#ce9178";
    pub const SIDEBAR_BACKGROUND: &str = "#252526";
    pub const SELECTION: &str = "#264f78";
}

/// Default light theme colors
pub mod light_theme {
    pub const BACKGROUND: &str = "#ffffff";
    pub const TEXT: &str = "#333333";
    pub const HEADING: &str = "#0066cc";
    pub const LINK: &str = "#0077cc";
    pub const CODE_BACKGROUND: &str = "#f5f5f5";
    pub const CODE_TEXT: &str = "#d73a49";
    pub const SIDEBAR_BACKGROUND: &str = "#f3f3f3";
    pub const SELECTION: &str = "#add6ff";
}

/// Heading size multipliers relative to base font size
pub mod heading_sizes {
    pub const H1: f32 = 2.0;
    pub const H2: f32 = 1.5;
    pub const H3: f32 = 1.25;
    pub const H4: f32 = 1.1;
    pub const H5: f32 = 1.0;
    pub const H6: f32 = 0.9;
}

/// Get heading size multiplier for a heading level
pub fn heading_size_multiplier(level: usize) -> f32 {
    match level {
        1 => heading_sizes::H1,
        2 => heading_sizes::H2,
        3 => heading_sizes::H3,
        4 => heading_sizes::H4,
        5 => heading_sizes::H5,
        _ => heading_sizes::H6,
    }
}
