//! Default configuration values
//!
//! This module provides default values and constants used throughout the application.

/// File watcher debounce duration in milliseconds
pub const WATCHER_DEBOUNCE_MS: u64 = 100;

/// Heading size multipliers relative to base font size
/// Designed for clear visual hierarchy with strong contrast between levels
pub mod heading_sizes {
    pub const H1: f32 = 2.2;
    pub const H2: f32 = 1.7;
    pub const H3: f32 = 1.35;
    pub const H4: f32 = 1.15;
    pub const H5: f32 = 1.0;
    pub const H6: f32 = 0.85;
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

/// Default dark theme colors (kept for config compatibility)
#[allow(dead_code)]
pub mod dark_theme {
    pub const BACKGROUND: &str = "#1e1e1e";
    pub const TEXT: &str = "#d4d4d4";
    pub const SELECTION: &str = "#264f78";
}

/// Default light theme colors (kept for config compatibility)
#[allow(dead_code)]
pub mod light_theme {
    pub const BACKGROUND: &str = "#ffffff";
    pub const TEXT: &str = "#333333";
    pub const SELECTION: &str = "#add6ff";
}
