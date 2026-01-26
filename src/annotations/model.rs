//! Annotation data structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of annotation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationKind {
    /// Text highlight with color
    Highlight,
    /// Note with text content
    Note,
    /// Bookmark marker
    Bookmark,
}

/// A single annotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    /// Unique identifier
    pub id: String,

    /// Type of annotation
    pub kind: AnnotationKind,

    /// Start position in the document (character offset)
    pub start: usize,

    /// End position in the document (character offset)
    pub end: usize,

    /// Color for highlights (hex string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Text content for notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note_text: Option<String>,

    /// Creation timestamp (Unix epoch seconds)
    pub created_at: u64,

    /// Last modification timestamp
    pub updated_at: u64,

    /// Content hash when annotation was created (for robustness)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

impl Annotation {
    /// Create a new highlight annotation
    pub fn highlight(start: usize, end: usize, color: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: generate_id(),
            kind: AnnotationKind::Highlight,
            start,
            end,
            color: Some(color.to_string()),
            note_text: None,
            created_at: now,
            updated_at: now,
            content_hash: None,
        }
    }

    /// Create a new note annotation
    pub fn note(start: usize, end: usize, text: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: generate_id(),
            kind: AnnotationKind::Note,
            start,
            end,
            color: None,
            note_text: Some(text.to_string()),
            created_at: now,
            updated_at: now,
            content_hash: None,
        }
    }

    /// Create a new bookmark annotation
    pub fn bookmark(position: usize) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: generate_id(),
            kind: AnnotationKind::Bookmark,
            start: position,
            end: position,
            color: None,
            note_text: None,
            created_at: now,
            updated_at: now,
            content_hash: None,
        }
    }

    /// Check if annotation overlaps with a range
    pub fn overlaps(&self, start: usize, end: usize) -> bool {
        self.start < end && self.end > start
    }

    /// Check if annotation contains a position
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }

    /// Update the note text
    pub fn set_note(&mut self, text: &str) {
        self.note_text = Some(text.to_string());
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Update the color
    pub fn set_color(&mut self, color: &str) {
        self.color = Some(color.to_string());
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Store for managing annotations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnnotationStore {
    /// All annotations indexed by ID
    #[serde(default)]
    pub annotations: HashMap<String, Annotation>,

    /// Content hash of the document (for tracking changes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_hash: Option<String>,

    /// Schema version for future compatibility
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

impl AnnotationStore {
    pub fn new() -> Self {
        Self {
            annotations: HashMap::new(),
            document_hash: None,
            version: 1,
        }
    }

    /// Add an annotation
    pub fn add(&mut self, annotation: Annotation) -> String {
        let id = annotation.id.clone();
        self.annotations.insert(id.clone(), annotation);
        id
    }

    /// Remove an annotation by ID
    pub fn remove(&mut self, id: &str) -> Option<Annotation> {
        self.annotations.remove(id)
    }

    /// Get an annotation by ID
    pub fn get(&self, id: &str) -> Option<&Annotation> {
        self.annotations.get(id)
    }

    /// Get a mutable annotation by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Annotation> {
        self.annotations.get_mut(id)
    }

    /// Get all annotations
    pub fn all(&self) -> impl Iterator<Item = &Annotation> {
        self.annotations.values()
    }

    /// Get all annotations sorted by position
    pub fn sorted_by_position(&self) -> Vec<&Annotation> {
        let mut annotations: Vec<_> = self.annotations.values().collect();
        annotations.sort_by_key(|a| a.start);
        annotations
    }

    /// Get annotations that overlap with a range
    pub fn in_range(&self, start: usize, end: usize) -> Vec<&Annotation> {
        self.annotations
            .values()
            .filter(|a| a.overlaps(start, end))
            .collect()
    }

    /// Get annotations at a specific position
    pub fn at_position(&self, pos: usize) -> Vec<&Annotation> {
        self.annotations
            .values()
            .filter(|a| a.contains(pos))
            .collect()
    }

    /// Get all highlights
    pub fn highlights(&self) -> impl Iterator<Item = &Annotation> {
        self.annotations
            .values()
            .filter(|a| a.kind == AnnotationKind::Highlight)
    }

    /// Get all notes
    pub fn notes(&self) -> impl Iterator<Item = &Annotation> {
        self.annotations
            .values()
            .filter(|a| a.kind == AnnotationKind::Note)
    }

    /// Get all bookmarks
    pub fn bookmarks(&self) -> impl Iterator<Item = &Annotation> {
        self.annotations
            .values()
            .filter(|a| a.kind == AnnotationKind::Bookmark)
    }

    /// Check if store has any annotations
    pub fn is_empty(&self) -> bool {
        self.annotations.is_empty()
    }

    /// Get number of annotations
    pub fn len(&self) -> usize {
        self.annotations.len()
    }

    /// Clear all annotations
    pub fn clear(&mut self) {
        self.annotations.clear();
    }

    /// Set the document hash
    pub fn set_document_hash(&mut self, hash: String) {
        self.document_hash = Some(hash);
    }
}

/// Generate a unique annotation ID
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    format!("ann_{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_highlight() {
        let ann = Annotation::highlight(10, 20, "#ffeb3b");
        assert_eq!(ann.kind, AnnotationKind::Highlight);
        assert_eq!(ann.start, 10);
        assert_eq!(ann.end, 20);
        assert_eq!(ann.color.as_deref(), Some("#ffeb3b"));
    }

    #[test]
    fn test_create_note() {
        let ann = Annotation::note(5, 15, "This is important");
        assert_eq!(ann.kind, AnnotationKind::Note);
        assert_eq!(ann.note_text.as_deref(), Some("This is important"));
    }

    #[test]
    fn test_annotation_store() {
        let mut store = AnnotationStore::new();

        let id = store.add(Annotation::highlight(0, 10, "#fff"));
        assert_eq!(store.len(), 1);

        assert!(store.get(&id).is_some());
        store.remove(&id);
        assert!(store.is_empty());
    }

    #[test]
    fn test_overlaps() {
        let ann = Annotation::highlight(10, 20, "#fff");

        assert!(ann.overlaps(5, 15));
        assert!(ann.overlaps(15, 25));
        assert!(ann.overlaps(12, 18));
        assert!(!ann.overlaps(0, 10));
        assert!(!ann.overlaps(20, 30));
    }
}
