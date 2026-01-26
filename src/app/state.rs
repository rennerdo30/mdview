//! Application state management

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use crate::annotations::AnnotationStore;
use crate::config::Config;
use crate::recent::{load_recent_files, save_recent_files, RecentFiles};
use crate::toc::TocTree;

#[cfg(feature = "plugins")]
use crate::plugin::LuaRuntime;

pub use super::file_browser::FolderState;

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

    /// Visible character offset range (start, end) based on current scroll position
    /// Updated during rendering to track which part of the document is visible
    pub visible_char_range: Option<(usize, usize)>,

    /// Target heading index to scroll to (set by TOC clicks)
    pub scroll_to_heading: Option<usize>,

    /// Recently opened files
    pub recent_files: RecentFiles,

    /// Whether to show the recent files panel
    pub show_recent_files: bool,

    /// Plugin runtime (when plugins feature is enabled)
    #[cfg(feature = "plugins")]
    pub plugin_runtime: Option<LuaRuntime>,

    /// Failed plugins (path, error message) - for showing notifications
    #[cfg(feature = "plugins")]
    pub failed_plugins: Vec<(PathBuf, String)>,

    /// Folder browsing state
    pub folder_state: FolderState,

    /// Current theme name (for theme switching)
    pub current_theme: String,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let toc_visible = config.general.show_toc;
        let toc_width = config.general.toc_width as f32;
        let recent_files = load_recent_files();
        let current_theme = config.general.theme.clone();

        // Initialize plugin runtime if feature is enabled
        #[cfg(feature = "plugins")]
        let (plugin_runtime, failed_plugins) = {
            let mut failed = Vec::new();
            match LuaRuntime::new() {
                Ok(runtime) => {
                    // Load plugins from config directory
                    if let Some(dirs) = directories::ProjectDirs::from("", "", "mdview") {
                        let plugins_dir = dirs.config_dir().join("plugins");
                        if plugins_dir.exists() {
                            if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
                                for entry in entries.flatten() {
                                    let path = entry.path();
                                    if path.extension().map_or(false, |e| e == "lua") {
                                        if let Err(e) = runtime.load_plugin(&path) {
                                            let err_msg = e.to_string();
                                            log::error!("Failed to load plugin {:?}: {}", path, err_msg);
                                            failed.push((path, err_msg));
                                        } else {
                                            log::info!("Loaded plugin: {:?}", path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Initialize config snapshot
                    runtime.update_config_snapshot(&config);
                    (Some(runtime), failed)
                }
                Err(e) => {
                    log::error!("Failed to create plugin runtime: {}", e);
                    (None, failed)
                }
            }
        };

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
            file_event_rx: None,
            file_event_tx: None,
            config,
            text_selection: None,
            creating_annotation: false,
            pending_note_text: String::new(),
            status_message: None,
            exporting_pdf: false,
            heading_positions: Vec::new(),
            visible_char_range: None,
            scroll_to_heading: None,
            recent_files,
            show_recent_files: false,
            #[cfg(feature = "plugins")]
            plugin_runtime,
            #[cfg(feature = "plugins")]
            failed_plugins,
            folder_state: FolderState::new(),
            current_theme,
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
        // Call file close hook for previous file if any
        #[cfg(feature = "plugins")]
        if let Some(ref prev_path) = self.current_file {
            self.call_plugin_hook_with_path(crate::plugin::api::PluginHook::OnFileClose, prev_path);
        }

        let content = std::fs::read_to_string(&path)?;
        let content_hash = Self::compute_content_hash(&content);

        // Parse TOC
        let toc = crate::toc::builder::build_toc(&content);

        // Load annotations with improved error handling
        let annotations = match crate::annotations::storage::load_annotations(&path) {
            Ok(store) => store,
            Err(e) => {
                log::warn!("Failed to load annotations for {:?}: {}", path, e);
                // Will show status message after setting current_file
                AnnotationStore::new()
            }
        };

        // Add to recent files
        self.recent_files.add(&path);
        if let Err(e) = save_recent_files(&self.recent_files) {
            log::warn!("Failed to save recent files: {}", e);
        }

        let had_annotation_error = annotations.is_empty() &&
            crate::annotations::storage::annotations_exist(&path);

        self.current_file = Some(path.clone());
        self.content = content;
        self.content_hash = content_hash;
        self.toc = toc;
        self.annotations = annotations;
        self.heading_positions.clear();
        self.show_recent_files = false;

        // Show warning if annotations couldn't be loaded
        if had_annotation_error {
            self.set_status("Warning: Could not load annotations for this file");
        }

        // Update plugin state with new content
        #[cfg(feature = "plugins")]
        if let Some(ref runtime) = self.plugin_runtime {
            runtime.set_content(&self.content);
        }

        // Call plugin hook for file open
        #[cfg(feature = "plugins")]
        self.call_plugin_hook_with_path(crate::plugin::api::PluginHook::OnFileOpen, &path);

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

    /// Call a plugin hook with the given name
    #[cfg(feature = "plugins")]
    pub fn call_plugin_hook(&self, hook: crate::plugin::api::PluginHook) {
        if let Some(ref runtime) = self.plugin_runtime {
            if let Err(e) = runtime.call_hook(hook.lua_name(), ()) {
                log::warn!("Plugin hook {} failed: {}", hook.lua_name(), e);
            }
        }
    }

    /// Call a plugin hook with a file path argument
    #[cfg(feature = "plugins")]
    pub fn call_plugin_hook_with_path(&self, hook: crate::plugin::api::PluginHook, path: &std::path::Path) {
        if let Some(ref runtime) = self.plugin_runtime {
            let path_str = path.to_string_lossy().to_string();
            if let Err(e) = runtime.call_hook(hook.lua_name(), path_str) {
                log::warn!("Plugin hook {} failed: {}", hook.lua_name(), e);
            }
        }
    }

    /// No-op for when plugins feature is disabled
    #[cfg(not(feature = "plugins"))]
    pub fn call_plugin_hook(&self, _hook_name: &str) {}

    /// No-op for when plugins feature is disabled
    #[cfg(not(feature = "plugins"))]
    pub fn call_plugin_hook_with_path(&self, _hook_name: &str, _path: &std::path::Path) {}

    /// Switch the current theme
    pub fn switch_theme(&mut self, theme_name: &str) {
        let old_theme = self.current_theme.clone();
        self.current_theme = theme_name.to_string();
        self.config.general.theme = theme_name.to_string();

        // Call theme change hook if theme actually changed
        #[cfg(feature = "plugins")]
        if old_theme != self.current_theme {
            self.call_plugin_hook(crate::plugin::api::PluginHook::OnThemeChange);
        }

        #[cfg(not(feature = "plugins"))]
        let _ = old_theme; // Silence unused warning
    }

    /// Check if there are failed plugins to report
    #[cfg(feature = "plugins")]
    pub fn has_failed_plugins(&self) -> bool {
        !self.failed_plugins.is_empty()
    }

    /// Get failed plugin count
    #[cfg(feature = "plugins")]
    pub fn failed_plugin_count(&self) -> usize {
        self.failed_plugins.len()
    }

    /// Clear failed plugins list (after showing notification)
    #[cfg(feature = "plugins")]
    pub fn clear_failed_plugins(&mut self) {
        self.failed_plugins.clear();
    }

    /// Open a folder for browsing
    pub fn open_folder(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        self.folder_state.open_folder(path)
    }

    /// Close the current folder
    pub fn close_folder(&mut self) {
        self.folder_state.close_folder();
    }

    /// Get pending plugin notifications and set them as status messages
    #[cfg(feature = "plugins")]
    pub fn process_plugin_notifications(&mut self) {
        if let Some(ref runtime) = self.plugin_runtime {
            let notifications = runtime.take_notifications();
            for (msg, level) in notifications {
                let prefix = match level.as_str() {
                    "warn" => "[Plugin Warning] ",
                    "error" => "[Plugin Error] ",
                    _ => "[Plugin] ",
                };
                self.set_status(format!("{}{}", prefix, msg));
            }
        }
    }

    /// Process pending annotation actions from plugins
    #[cfg(feature = "plugins")]
    pub fn process_plugin_annotations(&mut self) {
        use crate::annotations::model::Annotation;
        use crate::plugin::api::PendingAnnotationAction;

        if let Some(ref runtime) = self.plugin_runtime {
            let actions = runtime.take_pending_annotations();
            let mut added = false;

            for action in actions {
                match action {
                    PendingAnnotationAction::AddHighlight { start, end, color } => {
                        let ann = Annotation::highlight(start, end, &color);
                        self.annotations.add(ann);
                        added = true;
                        log::debug!("[plugin] Created highlight at {}..{}", start, end);
                    }
                    PendingAnnotationAction::AddNote { start, end, text } => {
                        let ann = Annotation::note(start, end, &text);
                        self.annotations.add(ann);
                        added = true;
                        log::debug!("[plugin] Created note at {}..{}", start, end);
                    }
                }
            }

            // Auto-save if enabled and we added annotations
            if added && self.config.annotations.auto_save {
                if let Err(e) = self.save_annotations() {
                    log::error!("Failed to auto-save plugin annotations: {}", e);
                }
            }
        }
    }

    /// Process pending config changes from plugins
    /// Returns true if any config was changed
    #[cfg(feature = "plugins")]
    pub fn process_plugin_config_changes(&mut self) -> bool {
        let mut changed = false;

        if let Some(ref runtime) = self.plugin_runtime {
            let changes = runtime.take_pending_config_changes();

            for (key, value) in changes {
                match key.as_str() {
                    "theme" => {
                        self.config.general.theme = value.clone();
                        self.current_theme = value;
                        changed = true;
                        log::debug!("[plugin] Changed theme");
                    }
                    "hot_reload" => {
                        if let Ok(b) = value.parse::<bool>() {
                            self.config.general.hot_reload = b;
                            changed = true;
                            log::debug!("[plugin] Changed hot_reload to {}", b);
                        }
                    }
                    "show_toc" => {
                        if let Ok(b) = value.parse::<bool>() {
                            self.config.general.show_toc = b;
                            self.toc_visible = b;
                            changed = true;
                            log::debug!("[plugin] Changed show_toc to {}", b);
                        }
                    }
                    "syntax_highlighting" => {
                        if let Ok(b) = value.parse::<bool>() {
                            self.config.markdown.syntax_highlighting = b;
                            changed = true;
                            log::debug!("[plugin] Changed syntax_highlighting to {}", b);
                        }
                    }
                    _ => {
                        log::warn!("[plugin] Unknown config key: {}", key);
                    }
                }
            }

            // Update the config snapshot in plugin state
            if changed {
                runtime.update_config_snapshot(&self.config);
            }
        }

        changed
    }

    /// Sync config to plugin state (call after config changes)
    #[cfg(feature = "plugins")]
    pub fn sync_config_to_plugins(&self) {
        if let Some(ref runtime) = self.plugin_runtime {
            runtime.update_config_snapshot(&self.config);
        }
    }
}
