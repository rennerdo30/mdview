//! Annotation persistence (JSON sidecar files)

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use super::model::AnnotationStore;

/// Get the annotation file path for a markdown file
pub fn get_annotation_path(markdown_path: &Path) -> PathBuf {
    let parent = markdown_path.parent().unwrap_or(Path::new("."));
    let filename = markdown_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    parent.join(format!(".{}.mdview-annotations.json", filename))
}

/// Maximum annotation file size (10 MB) to prevent excessive memory use
const MAX_ANNOTATION_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Load annotations from the sidecar file
pub fn load_annotations(markdown_path: &Path) -> Result<AnnotationStore, StorageError> {
    let annotation_path = get_annotation_path(markdown_path);

    if !annotation_path.exists() {
        return Ok(AnnotationStore::new());
    }

    // Check file size before loading to prevent OOM on malicious/corrupted files
    let metadata = std::fs::metadata(&annotation_path).map_err(StorageError::Io)?;
    if metadata.len() > MAX_ANNOTATION_FILE_SIZE {
        return Err(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Annotation file too large ({} bytes, max {})",
                metadata.len(),
                MAX_ANNOTATION_FILE_SIZE
            ),
        )));
    }

    let content = std::fs::read_to_string(&annotation_path).map_err(StorageError::Io)?;
    let store: AnnotationStore = serde_json::from_str(&content).map_err(StorageError::Parse)?;

    Ok(store)
}

/// Save annotations to the sidecar file
pub fn save_annotations(markdown_path: &Path, store: &AnnotationStore) -> Result<(), StorageError> {
    // Don't create file if there are no annotations
    if store.is_empty() {
        // Remove existing file if present
        let annotation_path = get_annotation_path(markdown_path);
        if annotation_path.exists() {
            std::fs::remove_file(&annotation_path).ok();
        }
        return Ok(());
    }

    let annotation_path = get_annotation_path(markdown_path);
    let content = serde_json::to_string_pretty(store).map_err(StorageError::Serialize)?;

    // Check serialized size before writing to ensure consistency with load limit
    if content.len() as u64 > MAX_ANNOTATION_FILE_SIZE {
        return Err(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Annotation data too large to save ({} bytes, max {}). Consider removing some annotations.",
                content.len(), MAX_ANNOTATION_FILE_SIZE
            ),
        )));
    }

    std::fs::write(&annotation_path, content).map_err(StorageError::Io)?;

    Ok(())
}

/// Delete the annotation file for a markdown file
pub fn delete_annotations(markdown_path: &Path) -> Result<(), StorageError> {
    let annotation_path = get_annotation_path(markdown_path);

    if annotation_path.exists() {
        std::fs::remove_file(&annotation_path).map_err(StorageError::Io)?;
    }

    Ok(())
}

/// Check if annotations exist for a markdown file
pub fn annotations_exist(markdown_path: &Path) -> bool {
    get_annotation_path(markdown_path).exists()
}

/// Storage errors
#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    Serialize(serde_json::Error),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "IO error: {}", e),
            StorageError::Parse(e) => write!(f, "Parse error: {}", e),
            StorageError::Serialize(e) => write!(f, "Serialize error: {}", e),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotations::model::Annotation;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_annotation_path() {
        let path = Path::new("/home/user/docs/readme.md");
        let ann_path = get_annotation_path(path);

        assert_eq!(
            ann_path,
            PathBuf::from("/home/user/docs/.readme.md.mdview-annotations.json")
        );
    }

    #[test]
    fn test_save_load_annotations() {
        let dir = tempdir().unwrap();
        let md_path = dir.path().join("test.md");
        fs::write(&md_path, "# Test").unwrap();

        let mut store = AnnotationStore::new();
        store.add(Annotation::highlight(0, 10, "#ffeb3b"));

        save_annotations(&md_path, &store).unwrap();

        let loaded = load_annotations(&md_path).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_empty_store_no_file() {
        let dir = tempdir().unwrap();
        let md_path = dir.path().join("test.md");
        fs::write(&md_path, "# Test").unwrap();

        let store = AnnotationStore::new();
        save_annotations(&md_path, &store).unwrap();

        // Should not create file for empty store
        assert!(!annotations_exist(&md_path));
    }
}
