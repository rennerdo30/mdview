//! File watcher using notify crate

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

use egui::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};

use crate::app::state::FileEvent;
use crate::config::defaults::WATCHER_DEBOUNCE_MS;

/// File watcher that monitors a file for changes
pub struct FileWatcher {
    /// The debouncer that handles file events
    _debouncer: Debouncer<RecommendedWatcher>,

    /// Path being watched
    path: PathBuf,
}

impl FileWatcher {
    /// Create a new file watcher for the given path
    pub fn new(
        path: PathBuf,
        sender: Sender<FileEvent>,
        ctx: Context,
    ) -> Result<Self, WatcherError> {
        let debounce_duration = Duration::from_millis(WATCHER_DEBOUNCE_MS);

        // Create the debouncer
        let mut debouncer = new_debouncer(
            debounce_duration,
            move |result: Result<Vec<DebouncedEvent>, notify::Error>| {
                match result {
                    Ok(events) => {
                        for event in events {
                            // Send file modified event
                            let _ = sender.send(FileEvent::Modified);
                            // Request repaint
                            ctx.request_repaint();
                        }
                    }
                    Err(e) => {
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
            path,
        })
    }

    /// Get the path being watched
    pub fn path(&self) -> &PathBuf {
        &self.path
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
    use std::sync::mpsc;
    use tempfile::tempdir;

    #[test]
    fn test_watcher_error_display() {
        let err = WatcherError::Init("test error".to_string());
        assert!(err.to_string().contains("test error"));
    }
}
