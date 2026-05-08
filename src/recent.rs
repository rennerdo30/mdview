//! Recently opened files tracking
//!
//! Persists a list of recently opened markdown files for quick access.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Maximum number of recent files to track
const MAX_RECENT_FILES: usize = 10;

/// A recently opened file entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFile {
    /// Full path to the file
    pub path: PathBuf,
    /// Last opened timestamp (Unix epoch seconds)
    pub last_opened: u64,
}

impl RecentFile {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            last_opened: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Get the file name for display
    pub fn display_name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
    }

    /// Check if the file still exists
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
}

/// Store for managing recently opened files
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentFiles {
    /// List of recent files, most recent first
    #[serde(default)]
    pub files: Vec<RecentFile>,

    /// Cached list of existing files (computed lazily, invalidated on mutation or after time)
    #[serde(skip)]
    existing_cache: Option<(Vec<RecentFile>, std::time::Instant)>,
}

/// How long to cache the existence check result (5 seconds)
const EXISTENCE_CACHE_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

impl RecentFiles {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            existing_cache: None,
        }
    }

    /// Invalidate the existence cache (call after mutations or on window focus)
    fn invalidate_cache(&mut self) {
        self.existing_cache = None;
    }

    /// Add a file to the recent list
    pub fn add(&mut self, path: &Path) {
        self.invalidate_cache();
        // Try to canonicalize to resolve symlinks and get absolute path
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                log::debug!(
                    "Could not canonicalize path {:?}: {} (using original)",
                    path,
                    e
                );
                path.to_path_buf()
            }
        };

        // Remove existing entry for this file if present
        // Check both canonical path and original path to avoid duplicates
        self.files.retain(|f| f.path != canonical && f.path != path);

        // Add new entry at the front
        self.files.insert(0, RecentFile::new(canonical));

        // Trim to max size
        self.files.truncate(MAX_RECENT_FILES);
    }

    /// Remove a file from the recent list
    pub fn remove(&mut self, path: &Path) {
        self.invalidate_cache();
        // Try to canonicalize to match how files were added
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                log::debug!(
                    "Could not canonicalize path {:?}: {} (using original)",
                    path,
                    e
                );
                path.to_path_buf()
            }
        };
        // Remove by both paths to ensure cleanup
        self.files.retain(|f| f.path != canonical && f.path != path);
    }

    /// Remove files that no longer exist
    pub fn cleanup(&mut self) {
        self.invalidate_cache();
        self.files.retain(|f| f.exists());
    }

    /// Get all recent files (existing ones only)
    /// Results are cached for performance (avoids file existence checks every frame)
    pub fn get_existing(&mut self) -> Vec<&RecentFile> {
        // Check if cache is still valid
        let cache_valid = self
            .existing_cache
            .as_ref()
            .is_some_and(|(_, cached_at)| cached_at.elapsed() < EXISTENCE_CACHE_DURATION);

        if !cache_valid {
            // Recompute and store cache
            let existing: Vec<RecentFile> =
                self.files.iter().filter(|f| f.exists()).cloned().collect();
            self.existing_cache = Some((existing, std::time::Instant::now()));
        }

        // Return references to cached results
        self.existing_cache.as_ref().unwrap().0.iter().collect()
    }

    /// Check if there are any recent files
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Clear all recent files
    pub fn clear(&mut self) {
        self.invalidate_cache();
        self.files.clear();
    }

    /// Explicitly invalidate the existence cache (call on window focus)
    pub fn refresh_cache(&mut self) {
        self.invalidate_cache();
    }
}

/// Get the path to the recent files storage
fn get_recent_files_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "mdview", "mdview")
        .map(|dirs| dirs.config_dir().join("recent.json"))
}

/// Load recent files from disk
pub fn load_recent_files() -> RecentFiles {
    let Some(path) = get_recent_files_path() else {
        return RecentFiles::new();
    };

    if !path.exists() {
        return RecentFiles::new();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => RecentFiles::new(),
    }
}

/// Save recent files to disk
pub fn save_recent_files(recent: &RecentFiles) -> Result<(), std::io::Error> {
    let Some(path) = get_recent_files_path() else {
        return Ok(());
    };

    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(recent).map_err(std::io::Error::other)?;

    std::fs::write(&path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_add_recent_file() {
        let mut recent = RecentFiles::new();
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("test1.md");
        let file2 = dir.path().join("test2.md");

        std::fs::write(&file1, "# Test 1").unwrap();
        std::fs::write(&file2, "# Test 2").unwrap();

        recent.add(&file1);
        recent.add(&file2);

        assert_eq!(recent.files.len(), 2);
        // Most recent should be first
        assert_eq!(recent.files[0].path, file2.canonicalize().unwrap());
    }

    #[test]
    fn test_duplicate_moves_to_front() {
        let mut recent = RecentFiles::new();
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("test1.md");
        let file2 = dir.path().join("test2.md");

        std::fs::write(&file1, "# Test 1").unwrap();
        std::fs::write(&file2, "# Test 2").unwrap();

        recent.add(&file1);
        recent.add(&file2);
        recent.add(&file1); // Add file1 again

        assert_eq!(recent.files.len(), 2);
        // file1 should now be first
        assert_eq!(recent.files[0].path, file1.canonicalize().unwrap());
    }

    #[test]
    fn test_max_files_limit() {
        let mut recent = RecentFiles::new();
        let dir = tempdir().unwrap();

        // Add more than MAX_RECENT_FILES
        for i in 0..15 {
            let file = dir.path().join(format!("test{}.md", i));
            std::fs::write(&file, format!("# Test {}", i)).unwrap();
            recent.add(&file);
        }

        assert_eq!(recent.files.len(), MAX_RECENT_FILES);
    }

    #[test]
    fn test_cleanup_removes_missing() {
        let mut recent = RecentFiles::new();
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.md");

        std::fs::write(&file, "# Test").unwrap();
        recent.add(&file);

        assert_eq!(recent.files.len(), 1);

        // Delete the file
        std::fs::remove_file(&file).unwrap();
        recent.cleanup();

        assert!(recent.files.is_empty());
    }
}
