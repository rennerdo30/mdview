//! Table of Contents module
//!
//! Extracts headings from markdown and provides a navigable TOC sidebar.

pub mod builder;
pub mod panel;

// Public API re-exports
#[allow(unused_imports)]
pub use builder::{build_toc, TocEntry, TocTree};
#[allow(unused_imports)]
pub use panel::TocPanel;
