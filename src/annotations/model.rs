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

    /// Start position in the document (byte offset)
    pub start: usize,

    /// End position in the document (byte offset)
    pub end: usize,

    /// Color for highlights (hex string)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    /// Pre-parsed color as RGBA tuple (computed lazily, cached to avoid parsing every frame)
    #[serde(skip)]
    pub color_rgba: Option<(u8, u8, u8, u8)>,

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

        let color_rgba = Self::parse_hex_color(color);

        Self {
            id: generate_id(),
            kind: AnnotationKind::Highlight,
            start,
            end,
            color: Some(color.to_string()),
            color_rgba: Some(color_rgba),
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
            color_rgba: None,
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
            color_rgba: None,
            note_text: None,
            created_at: now,
            updated_at: now,
            content_hash: None,
        }
    }

    /// Parse a hex color string to RGBA tuple
    fn parse_hex_color(hex: &str) -> (u8, u8, u8, u8) {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return (255, 235, 59, 255); // Default yellow
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(235);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(59);
        (r, g, b, 255)
    }

    /// Get the parsed RGBA color (computes lazily if not cached)
    pub fn get_color_rgba(&self) -> (u8, u8, u8, u8) {
        if let Some(rgba) = self.color_rgba {
            rgba
        } else if let Some(ref hex) = self.color {
            Self::parse_hex_color(hex)
        } else {
            (255, 235, 59, 255) // Default yellow
        }
    }

    /// Check if annotation overlaps with a range
    pub fn overlaps(&self, start: usize, end: usize) -> bool {
        if self.start == self.end {
            // Zero-length annotations (e.g. bookmarks) overlap when their point is in range.
            self.start >= start && self.start < end
        } else {
            self.start < end && self.end > start
        }
    }

    /// Check if annotation contains a position
    pub fn contains(&self, pos: usize) -> bool {
        if self.start == self.end {
            // Zero-length annotations represent a point location.
            pos == self.start
        } else {
            pos >= self.start && pos < self.end
        }
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
        self.color_rgba = Some(Self::parse_hex_color(color));
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

    /// Cached sorted annotations for efficient range queries (invalidated on mutation)
    #[serde(skip)]
    sorted_cache: Option<Vec<Annotation>>,

    /// Cached sorted IDs for immutable access (invalidated on mutation)
    #[serde(skip)]
    sorted_ids_cache: Option<Vec<String>>,

    /// Generation counter incremented on each mutation (for external cache invalidation)
    #[serde(skip)]
    generation: u64,
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
            sorted_cache: None,
            sorted_ids_cache: None,
            generation: 0,
        }
    }

    /// Invalidate the sorted cache (call after any mutation)
    fn invalidate_cache(&mut self) {
        self.sorted_cache = None;
        self.sorted_ids_cache = None;
        self.generation = self.generation.wrapping_add(1);
    }

    /// Get the generation counter (incremented on each mutation)
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Add an annotation
    pub fn add(&mut self, annotation: Annotation) -> String {
        let id = annotation.id.clone();
        self.annotations.insert(id.clone(), annotation);
        self.invalidate_cache();
        id
    }

    /// Remove an annotation by ID
    pub fn remove(&mut self, id: &str) -> Option<Annotation> {
        let result = self.annotations.remove(id);
        if result.is_some() {
            self.invalidate_cache();
        }
        result
    }

    /// Get an annotation by ID
    pub fn get(&self, id: &str) -> Option<&Annotation> {
        self.annotations.get(id)
    }

    /// Get a mutable annotation by ID (invalidates sorted cache)
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Annotation> {
        // Invalidate cache since the returned reference could modify the annotation
        self.invalidate_cache();
        self.annotations.get_mut(id)
    }

    /// Get all annotations
    pub fn all(&self) -> impl Iterator<Item = &Annotation> {
        self.annotations.values()
    }

    /// Get all annotations sorted by position (uses cache for efficiency)
    pub fn sorted_by_position(&self) -> Vec<&Annotation> {
        // Use cached sorted order if available (avoids re-sorting every call)
        if let Some(ref ids) = self.sorted_ids_cache {
            return ids.iter()
                .filter_map(|id| self.annotations.get(id))
                .collect();
        }
        let mut annotations: Vec<_> = self.annotations.values().collect();
        annotations.sort_by_key(|a| a.start);
        annotations
    }

    /// Ensure the sorted IDs cache is populated (call before immutable sorted access)
    pub fn ensure_sorted_cache(&mut self) {
        if self.sorted_ids_cache.is_none() {
            let mut pairs: Vec<_> = self.annotations.iter()
                .map(|(id, a)| (a.start, id.clone()))
                .collect();
            pairs.sort_by_key(|(start, _)| *start);
            self.sorted_ids_cache = Some(pairs.into_iter().map(|(_, id)| id).collect());
        }
    }

    /// Get cached sorted annotations for efficient repeated access
    /// The cache is automatically invalidated when annotations are modified
    pub fn get_sorted_cache(&mut self) -> &[Annotation] {
        if self.sorted_cache.is_none() {
            let mut sorted: Vec<_> = self.annotations.values().cloned().collect();
            sorted.sort_by_key(|a| a.start);
            self.sorted_cache = Some(sorted);
        }
        self.sorted_cache.as_ref().unwrap()
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
        self.invalidate_cache();
    }

    /// Set the document hash
    pub fn set_document_hash(&mut self, hash: String) {
        self.document_hash = Some(hash);
    }
}

/// Generate a unique annotation ID using timestamp + atomic counter
/// to avoid collisions when multiple annotations are created in the same nanosecond.
fn generate_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("ann_{:x}_{:x}", timestamp, seq)
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

    #[test]
    fn test_bookmark_point_overlap() {
        let bookmark = Annotation::bookmark(42);

        assert!(bookmark.overlaps(40, 50));
        assert!(!bookmark.overlaps(0, 42));
        assert!(!bookmark.overlaps(43, 100));
    }

    #[test]
    fn test_bookmark_contains_exact_position() {
        let bookmark = Annotation::bookmark(42);

        assert!(bookmark.contains(42));
        assert!(!bookmark.contains(41));
        assert!(!bookmark.contains(43));
    }
}
