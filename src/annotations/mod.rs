//! Annotations module
//!
//! Provides highlighting, notes, and bookmarks for markdown documents.

pub mod model;
pub mod storage;
pub mod ui;

pub use model::{Annotation, AnnotationKind, AnnotationStore};
pub use storage::{load_annotations, save_annotations};
