//! Theme module
//!
//! Provides theme management and egui style generation.

#[allow(dead_code)]
pub mod builtin;
pub mod style;

// Public API re-exports
#[allow(unused_imports)]
pub use builtin::{get_builtin_theme, BUILTIN_THEMES};
#[allow(unused_imports)]
pub use style::create_style;
