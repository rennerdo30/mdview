//! File watcher module
//!
//! Provides hot reload functionality by watching file changes.

pub mod file_watcher;

// Public API re-exports
#[allow(unused_imports)]
pub use file_watcher::FileWatcher;
