//! Annotations module
//!
//! Provides highlighting, notes, and bookmarks for markdown documents.

#[allow(dead_code)]
pub mod model;
pub mod storage;
#[allow(dead_code)]
pub mod ui;

// Public API re-exports
#[allow(unused_imports)]
pub use model::{Annotation, AnnotationKind, AnnotationStore};
#[allow(unused_imports)]
pub use storage::{load_annotations, save_annotations};
