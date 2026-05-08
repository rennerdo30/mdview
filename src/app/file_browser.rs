//! File browser component for folder navigation
//!
//! Provides a tree view of markdown files in a folder.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use egui::{RichText, Rounding, Vec2};

use crate::theme::style::palette;

/// State for folder browsing
#[derive(Debug, Clone, Default)]
pub struct FolderState {
    /// Root folder path
    pub root_path: Option<PathBuf>,

    /// All markdown files in the folder (recursively)
    pub files: Vec<FileEntry>,

    /// Set of expanded directory paths
    pub expanded_dirs: HashSet<PathBuf>,
}

/// A file or directory entry
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Full path to the file/directory
    pub path: PathBuf,

    /// Display name
    pub name: String,

    /// Whether this is a directory
    pub is_dir: bool,

    /// Depth level in the tree (0 = root)
    pub depth: usize,
}

impl FolderState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a folder and scan for markdown files
    pub fn open_folder(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        if !path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Path is not a directory",
            ));
        }

        self.root_path = Some(path.clone());
        self.files = scan_directory(&path, 0)?;
        self.expanded_dirs.clear();

        // Auto-expand the root
        self.expanded_dirs.insert(path);

        Ok(())
    }

    /// Close the current folder
    pub fn close_folder(&mut self) {
        self.root_path = None;
        self.files.clear();
        self.expanded_dirs.clear();
    }

    /// Check if a folder is currently open
    pub fn is_open(&self) -> bool {
        self.root_path.is_some()
    }

    /// Toggle expansion of a directory
    pub fn toggle_dir(&mut self, path: &Path) {
        if self.expanded_dirs.contains(path) {
            self.expanded_dirs.remove(path);
        } else {
            self.expanded_dirs.insert(path.to_path_buf());
        }
    }

    /// Check if a directory is expanded
    pub fn is_expanded(&self, path: &Path) -> bool {
        self.expanded_dirs.contains(path)
    }

    /// Refresh the file list
    pub fn refresh(&mut self) -> Result<(), std::io::Error> {
        if let Some(root) = self.root_path.clone() {
            self.files = scan_directory(&root, 0)?;
        }
        Ok(())
    }

    /// Get visible entries (respecting expanded state)
    pub fn visible_entries(&self) -> Vec<&FileEntry> {
        let mut visible = Vec::new();
        let mut skip_until_depth: Option<usize> = None;

        for entry in &self.files {
            // Check if we should skip this entry
            if let Some(skip_depth) = skip_until_depth {
                if entry.depth > skip_depth {
                    continue;
                } else {
                    skip_until_depth = None;
                }
            }

            visible.push(entry);

            // If this is a collapsed directory, skip its children
            if entry.is_dir && !self.is_expanded(&entry.path) {
                skip_until_depth = Some(entry.depth);
            }
        }

        visible
    }
}

/// Maximum recursion depth for directory scanning (prevents stack overflow on deep trees)
const MAX_SCAN_DEPTH: usize = 32;
/// Hard cap for scanned entries to avoid freezing on very large trees
const MAX_SCANNED_ENTRIES: usize = 20_000;
/// Approximate row height used by the virtualized file list
const FILE_BROWSER_ROW_HEIGHT: f32 = 24.0;
const IGNORED_DIRECTORIES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
];

/// Scan a directory for markdown files and subdirectories
fn scan_directory(path: &Path, depth: usize) -> Result<Vec<FileEntry>, std::io::Error> {
    let mut entries = Vec::new();
    let mut scanned_entries = 0usize;
    scan_directory_inner(path, depth, &mut scanned_entries, &mut entries)?;
    Ok(entries)
}

fn scan_directory_inner(
    path: &Path,
    depth: usize,
    scanned_entries: &mut usize,
    entries: &mut Vec<FileEntry>,
) -> Result<(), std::io::Error> {
    if *scanned_entries >= MAX_SCANNED_ENTRIES {
        return Ok(());
    }

    // Prevent excessive recursion depth
    if depth >= MAX_SCAN_DEPTH {
        log::warn!(
            "Directory scan depth limit ({}) reached at {:?}",
            MAX_SCAN_DEPTH,
            path
        );
        return Ok(());
    }

    let mut dir_entries: Vec<_> = std::fs::read_dir(path)?.filter_map(|e| e.ok()).collect();

    // Sort: directories first, then by name
    dir_entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for dir_entry in dir_entries {
        let entry_path = dir_entry.path();
        let name = dir_entry.file_name().to_string_lossy().to_string();

        // Skip hidden files/directories
        if name.starts_with('.') {
            continue;
        }

        if entry_path.is_dir() {
            if IGNORED_DIRECTORIES.contains(&name.as_str()) {
                continue;
            }

            // Skip symlinks to directories to prevent cycles
            if entry_path
                .symlink_metadata()
                .map(|m| m.is_symlink())
                .unwrap_or(false)
            {
                log::debug!("Skipping symlinked directory: {:?}", entry_path);
                continue;
            }

            if *scanned_entries >= MAX_SCANNED_ENTRIES {
                log::warn!(
                    "File browser scan capped at {} entries under {:?}",
                    MAX_SCANNED_ENTRIES,
                    path
                );
                break;
            }
            entries.push(FileEntry {
                path: entry_path.clone(),
                name,
                is_dir: true,
                depth,
            });
            *scanned_entries += 1;

            // Recursively scan subdirectory
            scan_directory_inner(&entry_path, depth + 1, scanned_entries, entries)?;
        } else if is_markdown_file(&entry_path) {
            if *scanned_entries >= MAX_SCANNED_ENTRIES {
                log::warn!(
                    "File browser scan capped at {} entries under {:?}",
                    MAX_SCANNED_ENTRIES,
                    path
                );
                break;
            }
            entries.push(FileEntry {
                path: entry_path,
                name,
                is_dir: false,
                depth,
            });
            *scanned_entries += 1;
        }
    }

    Ok(())
}

/// Check if a file is a markdown file (including MDX)
fn is_markdown_file(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => matches!(ext.to_lowercase().as_str(), "md" | "markdown" | "mdx"),
        None => false,
    }
}

/// File browser panel for rendering in the UI
pub struct FileBrowserPanel {
    /// Currently hovered entry index
    hovered_idx: Option<usize>,
}

impl FileBrowserPanel {
    pub fn new() -> Self {
        Self { hovered_idx: None }
    }

    /// Render the file browser panel
    /// Returns the path of a file to open if clicked
    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        folder_state: &mut FolderState,
        current_file: Option<&Path>,
    ) -> Option<PathBuf> {
        let mut file_to_open: Option<PathBuf> = None;
        let mut should_close = false;
        let mut should_refresh = false;
        let mut dir_to_toggle: Option<PathBuf> = None;

        // Get folder name before rendering (to avoid borrow issues)
        let folder_name = folder_state
            .root_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        // Header with folder name
        if let Some(name) = folder_name {
            ui.horizontal(|ui| {
                ui.add_space(8.0);

                ui.label(
                    RichText::new(format!("\u{1F4C1} {}", name))
                        .color(palette::TEXT_PRIMARY)
                        .strong(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Close button
                    if ui
                        .small_button("\u{2715}")
                        .on_hover_text("Close folder")
                        .clicked()
                    {
                        should_close = true;
                    }

                    // Refresh button
                    if ui
                        .small_button("\u{21BB}")
                        .on_hover_text("Refresh")
                        .clicked()
                    {
                        should_refresh = true;
                    }
                });
            });

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
        }

        // File list
        {
            let visible_entries = folder_state.visible_entries();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show_rows(
                    ui,
                    FILE_BROWSER_ROW_HEIGHT,
                    visible_entries.len(),
                    |ui, row_range| {
                        for idx in row_range {
                            let entry = visible_entries[idx];
                            let is_expanded = folder_state.is_expanded(&entry.path);
                            let is_current = current_file
                                .map(|f| f == entry.path.as_path())
                                .unwrap_or(false);

                            let response =
                                self.render_entry_data(ui, entry, idx, is_expanded, is_current);

                            if response.clicked() {
                                if entry.is_dir {
                                    dir_to_toggle = Some(entry.path.clone());
                                } else {
                                    file_to_open = Some(entry.path.clone());
                                }
                            }
                        }
                    },
                );
        }

        // Apply mutations after UI rendering
        if should_close {
            folder_state.close_folder();
        }
        if should_refresh {
            if let Err(e) = folder_state.refresh() {
                log::warn!("Failed to refresh folder: {}", e);
            }
        }
        if let Some(path) = dir_to_toggle {
            folder_state.toggle_dir(&path);
        }

        file_to_open
    }

    fn render_entry_data(
        &mut self,
        ui: &mut egui::Ui,
        entry: &FileEntry,
        idx: usize,
        is_expanded: bool,
        is_current: bool,
    ) -> egui::Response {
        let indent = entry.depth as f32 * 16.0;
        let height = FILE_BROWSER_ROW_HEIGHT;

        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height),
            egui::Sense::click(),
        );

        let is_hovered = response.hovered();
        if is_hovered {
            self.hovered_idx = Some(idx);
        }

        // Background
        let bg_color = if is_current {
            palette::BG_ACTIVE
        } else if is_hovered {
            palette::BG_HOVER
        } else {
            egui::Color32::TRANSPARENT
        };

        if bg_color != egui::Color32::TRANSPARENT {
            ui.painter()
                .rect_filled(rect, Rounding::same(4.0), bg_color);
        }

        // Selection indicator
        if is_current {
            let indicator_rect = egui::Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height()));
            ui.painter()
                .rect_filled(indicator_rect, Rounding::same(1.0), palette::ACCENT);
        }

        // Icon and text
        let text_start = rect.min + Vec2::new(8.0 + indent, (rect.height() - 14.0) / 2.0);

        let (icon, icon_color) = if entry.is_dir {
            if is_expanded {
                ("\u{1F4C2}", palette::ACCENT) // Open folder
            } else {
                ("\u{1F4C1}", palette::TEXT_MUTED) // Closed folder
            }
        } else {
            ("\u{1F4C4}", palette::TEXT_SECONDARY) // Document
        };

        ui.painter().text(
            text_start,
            egui::Align2::LEFT_TOP,
            icon,
            egui::FontId::proportional(12.0),
            icon_color,
        );

        let text_color = if is_current || is_hovered {
            palette::TEXT_PRIMARY
        } else {
            palette::TEXT_SECONDARY
        };

        ui.painter().text(
            text_start + Vec2::new(20.0, 0.0),
            egui::Align2::LEFT_TOP,
            &entry.name,
            egui::FontId::proportional(13.0),
            text_color,
        );

        response
    }
}

impl Default for FileBrowserPanel {
    fn default() -> Self {
        Self::new()
    }
}

/// Open a folder dialog and return the selected path
pub fn rfd_open_folder() -> Option<PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_is_markdown_file() {
        assert!(is_markdown_file(Path::new("test.md")));
        assert!(is_markdown_file(Path::new("test.markdown")));
        assert!(is_markdown_file(Path::new("test.mdx")));
        assert!(!is_markdown_file(Path::new("test.rs")));
        assert!(!is_markdown_file(Path::new("test.txt")));
        assert!(!is_markdown_file(Path::new("test")));
    }

    #[test]
    fn test_folder_state_open_close() {
        let dir = tempdir().unwrap();
        let md_file = dir.path().join("test.md");
        fs::write(&md_file, "# Test").unwrap();

        let mut state = FolderState::new();
        assert!(!state.is_open());

        state.open_folder(dir.path().to_path_buf()).unwrap();
        assert!(state.is_open());
        assert!(!state.files.is_empty());

        state.close_folder();
        assert!(!state.is_open());
    }

    #[test]
    fn test_scan_directory() {
        let dir = tempdir().unwrap();

        // Create test structure
        fs::write(dir.path().join("readme.md"), "# Readme").unwrap();
        fs::write(dir.path().join("notes.txt"), "Notes").unwrap();
        fs::write(dir.path().join("code.rs"), "fn main() {}").unwrap();

        let subdir = dir.path().join("docs");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("guide.md"), "# Guide").unwrap();

        let entries = scan_directory(dir.path(), 0).unwrap();

        // Should have: docs/ (dir), guide.md, readme.md
        // notes.txt and code.rs should be excluded
        let md_files: Vec<_> = entries.iter().filter(|e| !e.is_dir).collect();
        assert_eq!(md_files.len(), 2); // readme.md, docs/guide.md
    }

    #[test]
    fn test_scan_directory_skips_ignored_directories() {
        let dir = tempdir().unwrap();
        let target_dir = dir.path().join("target");
        fs::create_dir(&target_dir).unwrap();
        fs::write(target_dir.join("generated.md"), "# Generated").unwrap();
        fs::write(dir.path().join("readme.md"), "# Readme").unwrap();

        let entries = scan_directory(dir.path(), 0).unwrap();
        assert!(entries
            .iter()
            .all(|entry| !entry.path.starts_with(&target_dir)));
    }
}
