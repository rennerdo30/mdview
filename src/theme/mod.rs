//! Theme module
//!
//! Provides theme management and egui style generation.

pub mod builtin;
pub mod style;

pub use builtin::{get_builtin_theme, BUILTIN_THEMES};
pub use style::create_style;
