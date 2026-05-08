//! File watcher using notify crate

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use egui::Context;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};

use crate::app::state::FileEvent;
use crate::config::defaults::WATCHER_DEBOUNCE_MS;

/// File watcher that monitors a file for changes
pub struct FileWatcher {
    /// The debouncer that handles file events
    _debouncer: Debouncer<RecommendedWatcher>,
}

impl FileWatcher {
    /// Create a new file watcher for the given path
    pub fn new(
        path: PathBuf,
        sender: Sender<FileEvent>,
        ctx: Context,
    ) -> Result<Self, WatcherError> {
        let debounce_duration = Duration::from_millis(WATCHER_DEBOUNCE_MS);

        // Canonicalize the watched path for reliable comparison with event paths
        let watched_path = path.canonicalize().unwrap_or_else(|_| path.clone());

        // Create the debouncer
        let mut debouncer = new_debouncer(
            debounce_duration,
            move |result: Result<Vec<DebouncedEvent>, notify::Error>| {
                match result {
                    Ok(events) => {
                        // Process events - debouncer coalesces rapid changes
                        // so we typically get one event per change batch
                        let mut has_event = false;
                        for event in events {
                            log::debug!("File event for: {:?}", event.path);

                            // Compare canonicalized paths to handle symlinks and relative paths
                            let event_canonical = event
                                .path
                                .canonicalize()
                                .unwrap_or_else(|_| event.path.clone());
                            if event_canonical == watched_path {
                                has_event = true;
                            }
                        }

                        if has_event {
                            // Check if file was removed vs modified
                            if watched_path.exists() {
                                let _ = sender.send(FileEvent::Modified(watched_path.clone()));
                            } else {
                                let _ = sender.send(FileEvent::Removed(watched_path.clone()));
                            }
                            // Request repaint
                            ctx.request_repaint();
                        }
                    }
                    Err(e) => {
                        log::error!("File watcher error: {}", e);
                        let _ = sender.send(FileEvent::Error(e.to_string()));
                    }
                }
            },
        )
        .map_err(|e| WatcherError::Init(e.to_string()))?;

        // Watch the file
        debouncer
            .watcher()
            .watch(&path, RecursiveMode::NonRecursive)
            .map_err(|e| WatcherError::Watch(e.to_string()))?;

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}

/// File watcher errors
#[derive(Debug)]
pub enum WatcherError {
    Init(String),
    Watch(String),
}

impl std::fmt::Display for WatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatcherError::Init(e) => write!(f, "Failed to initialize watcher: {}", e),
            WatcherError::Watch(e) => write!(f, "Failed to watch file: {}", e),
        }
    }
}

impl std::error::Error for WatcherError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_error_display() {
        let err = WatcherError::Init("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }
}
