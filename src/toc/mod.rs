//! Table of Contents module
//!
//! Extracts headings from markdown and provides a navigable TOC sidebar.

pub mod builder;
pub mod panel;

pub use builder::{build_toc, TocEntry, TocTree};
pub use panel::TocPanel;
