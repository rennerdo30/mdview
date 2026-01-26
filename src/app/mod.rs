//! Application module
//!
//! Contains the main application state and eframe::App implementation.

pub mod file_browser;
pub mod state;
mod viewer;

// Public API re-exports (some may be unused internally but are part of the public API)
#[allow(unused_imports)]
pub use state::{AppState, FileEvent, FolderState};
#[allow(unused_imports)]
pub use file_browser::{FileBrowserPanel, FileEntry};
pub use viewer::MdViewApp;
