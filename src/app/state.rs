//! Application state management

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use crate::annotations::AnnotationStore;
use crate::config::Config;
use crate::markdown::cache::RenderCache;
use crate::toc::TocTree;

/// File change event from the watcher
#[derive(Debug, Clone)]
pub enum FileEvent {
    Modified,
    Removed,
    Error(String),
}

/// Application state
pub struct AppState {
    /// Currently opened file path
    pub current_file: Option<PathBuf>,

    /// Raw markdown content
    pub content: String,

    /// Content hash for annotation tracking
    pub content_hash: String,

    /// Parsed table of contents
    pub toc: TocTree,

    /// Whether the TOC sidebar is visible
    pub toc_visible: bool,

    /// TOC sidebar width
    pub toc_width: f32,

    /// Current scroll offset in the main view
    pub scroll_offset: f32,

    /// Current visible heading index (for TOC highlighting)
    pub current_heading_idx: Option<usize>,

    /// Annotation store for current file
    pub annotations: AnnotationStore,

    /// Render cache for performance
    pub render_cache: RenderCache,

    /// File watcher event receiver
    pub file_event_rx: Option<Receiver<FileEvent>>,

    /// File watcher event sender (kept for watcher setup)
    pub file_event_tx: Option<Sender<FileEvent>>,

    /// Application configuration
    pub config: Config,

    /// Selected text range (start, end) for annotation creation
    pub text_selection: Option<(usize, usize)>,

    /// Whether we're in annotation creation mode
    pub creating_annotation: bool,

    /// Pending note text for annotation
    pub pending_note_text: String,

    /// Status message to display
    pub status_message: Option<(String, std::time::Instant)>,

    /// Whether PDF export is in progress
    pub exporting_pdf: bool,

    /// Heading positions for scroll-to navigation (heading_idx -> y_offset)
    pub heading_positions: Vec<f32>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let toc_visible = config.general.show_toc;
        let toc_width = config.general.toc_width as f32;

        Self {
            current_file: None,
            content: String::new(),
            content_hash: String::new(),
            toc: TocTree::new(),
            toc_visible,
            toc_width,
            scroll_offset: 0.0,
            current_heading_idx: None,
            annotations: AnnotationStore::new(),
            render_cache: RenderCache::new(500),
            file_event_rx: None,
            file_event_tx: None,
            config,
            text_selection: None,
            creating_annotation: false,
            pending_note_text: String::new(),
            status_message: None,
            exporting_pdf: false,
            heading_positions: Vec::new(),
        }
    }

    /// Set a status message that will be displayed temporarily
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some((message.into(), std::time::Instant::now()));
    }

    /// Clear expired status message (after 3 seconds)
    pub fn clear_expired_status(&mut self) {
        if let Some((_, instant)) = &self.status_message {
            if instant.elapsed().as_secs() >= 3 {
                self.status_message = None;
            }
        }
    }

    /// Compute content hash for annotation tracking
    pub fn compute_content_hash(content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Load a file and update state
    pub fn load_file(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(&path)?;
        let content_hash = Self::compute_content_hash(&content);

        // Parse TOC
        let toc = crate::toc::builder::build_toc(&content);

        // Load annotations
        let annotations = crate::annotations::storage::load_annotations(&path)
            .unwrap_or_else(|_| AnnotationStore::new());

        self.current_file = Some(path);
        self.content = content;
        self.content_hash = content_hash;
        self.toc = toc;
        self.annotations = annotations;
        self.heading_positions.clear();
        self.render_cache.clear();

        Ok(())
    }

    /// Reload the current file (preserving scroll position)
    pub fn reload_file(&mut self) -> Result<(), std::io::Error> {
        if let Some(path) = self.current_file.clone() {
            let scroll = self.scroll_offset;
            self.load_file(path)?;
            self.scroll_offset = scroll;
        }
        Ok(())
    }

    /// Save annotations for the current file
    pub fn save_annotations(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = &self.current_file {
            crate::annotations::storage::save_annotations(path, &self.annotations)?;
        }
        Ok(())
    }
}
