//! mdview library
//!
//! Core library for the mdview markdown viewer.

pub mod annotations;
pub mod app;
pub mod config;
pub mod export;
pub mod file_association;
pub mod markdown;
pub mod native_menu;
#[cfg(feature = "plugins")]
pub mod plugin;
pub mod recent;
pub mod theme;
pub mod toc;
pub mod update;
pub mod watcher;

pub use app::MdViewApp;
pub use config::Config;
