//! mdview library
//!
//! Core library for the mdview markdown viewer.

pub mod app;
pub mod config;
pub mod markdown;
pub mod toc;
pub mod annotations;
pub mod export;
pub mod theme;
pub mod watcher;
#[cfg(feature = "plugins")]
pub mod plugin;

pub use app::MdViewApp;
pub use config::Config;
