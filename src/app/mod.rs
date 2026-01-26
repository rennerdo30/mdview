//! Application module
//!
//! Contains the main application state and eframe::App implementation.

pub mod state;
mod viewer;

pub use state::{AppState, FileEvent};
pub use viewer::MdViewApp;
