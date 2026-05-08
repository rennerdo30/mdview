//! Auto-update checker via GitHub releases
//!
//! Checks GitHub releases API for newer versions and notifies users.

use serde::Deserialize;
use std::sync::mpsc;
use std::thread;

/// Current version from Cargo.toml
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// GitHub repository for releases
const GITHUB_REPO: &str = "rennerdo30/mdview";

/// GitHub API endpoint for latest release
fn releases_url() -> String {
    format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    )
}

/// Response from GitHub releases API
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    name: Option<String>,
    body: Option<String>,
    prerelease: bool,
    draft: bool,
}

/// Result of update check
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    /// New version available (e.g., "0.2.0")
    pub version: String,
    /// URL to the release page
    pub url: String,
    /// Release name/title
    pub name: Option<String>,
    /// Release notes (truncated)
    pub notes: Option<String>,
}

/// Update checker state
#[derive(Debug, Default)]
pub struct UpdateChecker {
    /// Receiver for async update check results
    receiver: Option<mpsc::Receiver<Option<UpdateInfo>>>,
    /// Cached result
    pub result: Option<Option<UpdateInfo>>,
    /// Whether user dismissed the notification
    pub dismissed: bool,
}

impl UpdateChecker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start an async update check in a background thread
    pub fn check_async(&mut self) {
        if self.receiver.is_some() {
            // Already checking
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);

        thread::spawn(move || {
            let result = check_for_update();
            let _ = tx.send(result);
        });
    }

    /// Poll for async result (non-blocking)
    /// Returns true if a new result is available
    pub fn poll(&mut self) -> bool {
        if let Some(ref receiver) = self.receiver {
            match receiver.try_recv() {
                Ok(result) => {
                    self.result = Some(result);
                    self.receiver = None;
                    return true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.receiver = None;
                }
            }
        }
        false
    }

    /// Check if an update is available
    pub fn has_update(&self) -> bool {
        matches!(self.result, Some(Some(_)))
    }

    /// Get update info if available
    pub fn update_info(&self) -> Option<&UpdateInfo> {
        self.result.as_ref().and_then(|r| r.as_ref())
    }

    /// Dismiss the update notification
    pub fn dismiss(&mut self) {
        self.dismissed = true;
    }

    /// Check if notification should be shown
    pub fn should_show(&self) -> bool {
        self.has_update() && !self.dismissed
    }
}

/// Check for updates synchronously
pub fn check_for_update() -> Option<UpdateInfo> {
    let response = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .get(&releases_url())
        .set("User-Agent", &format!("mdview/{}", CURRENT_VERSION))
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .ok()?;

    let body = response.into_string().ok()?;
    let release: GitHubRelease = serde_json::from_str(&body).ok()?;

    // Skip prereleases and drafts
    if release.prerelease || release.draft {
        return None;
    }

    // Parse version from tag (remove 'v' prefix if present)
    let new_version = release.tag_name.trim_start_matches('v').to_string();

    // Compare versions
    if is_newer_version(&new_version, CURRENT_VERSION) {
        // Truncate release notes to first 500 chars
        let notes = release.body.map(|b: String| {
            const MAX_CHARS: usize = 500;
            if b.chars().count() > MAX_CHARS {
                let truncated: String = b.chars().take(MAX_CHARS).collect();
                format!("{}...", truncated)
            } else {
                b
            }
        });

        Some(UpdateInfo {
            version: new_version,
            url: release.html_url,
            name: release.name,
            notes,
        })
    } else {
        None
    }
}

/// Compare semantic versions, returns true if new > current
fn is_newer_version(new: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|part| {
                // Handle versions like "1.0.0-beta" by taking only the numeric part
                part.split('-').next().and_then(|p| p.parse().ok())
            })
            .collect()
    };

    let new_parts = parse_version(new);
    let current_parts = parse_version(current);

    for (n, c) in new_parts.iter().zip(current_parts.iter()) {
        if n > c {
            return true;
        } else if n < c {
            return false;
        }
    }

    // If all compared parts are equal, longer version is newer
    new_parts.len() > current_parts.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_newer_version("0.2.0", "0.1.0"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("0.1.1", "0.1.0"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.0.9", "0.1.0"));
        assert!(is_newer_version("1.0.0", "0.1.0-beta"));
        assert!(is_newer_version("0.1.0.1", "0.1.0"));
    }

    #[test]
    fn test_current_version_exists() {
        assert!(!CURRENT_VERSION.is_empty());
    }
}
