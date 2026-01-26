//! File association handling for registering mdview as default markdown handler
//!
//! This module provides platform-specific implementations for:
//! - macOS: Using Launch Services
//! - Linux: Using xdg-mime
//! - Windows: Using registry

use std::process::Command;
#[allow(unused_imports)]
use log::{debug, warn};

/// Result of attempting to register file association
#[derive(Debug)]
pub enum AssociationResult {
    /// Successfully registered
    Success,
    /// Failed with error message
    Failed(String),
    /// Not supported on this platform (used for non-macOS/Linux/Windows)
    #[allow(dead_code)]
    NotSupported,
}

/// Check if mdview is currently the default handler for .md files
pub fn is_default_handler() -> bool {
    #[cfg(target_os = "macos")]
    {
        is_default_handler_macos()
    }

    #[cfg(target_os = "linux")]
    {
        is_default_handler_linux()
    }

    #[cfg(target_os = "windows")]
    {
        is_default_handler_windows()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

/// Register mdview as the default handler for .md files
pub fn register_as_default() -> AssociationResult {
    #[cfg(target_os = "macos")]
    {
        register_macos()
    }

    #[cfg(target_os = "linux")]
    {
        register_linux()
    }

    #[cfg(target_os = "windows")]
    {
        register_windows()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        AssociationResult::NotSupported
    }
}

/// Get the path to the current executable
fn get_executable_path() -> Option<String> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
}

// ============================================================================
// macOS Implementation
// ============================================================================

#[cfg(target_os = "macos")]
fn is_default_handler_macos() -> bool {
    // Use duti to check default handler
    // duti -x md returns the current default app for .md files
    let output = Command::new("duti")
        .args(["-x", "md"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.to_lowercase().contains("mdview")
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "macos")]
fn register_macos() -> AssociationResult {
    // On macOS, we need to use duti or Launch Services
    // duti is a command-line utility for managing default applications
    // If duti is not installed, we fall back to showing instructions

    let exe_path = match get_executable_path() {
        Some(p) => p,
        None => return AssociationResult::Failed("Could not determine executable path".to_string()),
    };

    // Try using duti first
    let result = Command::new("duti")
        .args(["-s", "com.mdview.mdview", ".md", "all"])
        .output();

    match result {
        Ok(output) if output.status.success() => AssociationResult::Success,
        Ok(_) | Err(_) => {
            // duti not available, try using open command to set default
            // This requires the app to be properly bundled
            let result = Command::new("defaults")
                .args([
                    "write",
                    "com.apple.LaunchServices/com.apple.launchservices.secure",
                    "LSHandlers",
                    "-array-add",
                    &format!(
                        "{{LSHandlerContentType = \"net.daringfireball.markdown\"; LSHandlerRoleAll = \"com.mdview.mdview\";}}"
                    ),
                ])
                .output();

            match result {
                Ok(output) if output.status.success() => {
                    // Rebuild Launch Services database
                    let _ = Command::new("/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister")
                        .args(["-kill", "-r", "-domain", "local", "-domain", "system", "-domain", "user"])
                        .output();
                    AssociationResult::Success
                }
                _ => AssociationResult::Failed(
                    format!("To set mdview as default, right-click a .md file in Finder, \
                            select 'Get Info', change 'Open with' to mdview, \
                            then click 'Change All'. Executable: {}", exe_path)
                ),
            }
        }
    }
}

// ============================================================================
// Linux Implementation
// ============================================================================

#[cfg(target_os = "linux")]
fn is_default_handler_linux() -> bool {
    // Use xdg-mime to check default handler
    let output = Command::new("xdg-mime")
        .args(["query", "default", "text/markdown"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.to_lowercase().contains("mdview")
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "linux")]
fn register_linux() -> AssociationResult {
    let exe_path = match get_executable_path() {
        Some(p) => p,
        None => return AssociationResult::Failed("Could not determine executable path".to_string()),
    };

    // Create .desktop file
    let desktop_entry = format!(
        r#"[Desktop Entry]
Type=Application
Name=mdview
Comment=Fast Markdown Viewer
Exec="{}" %f
Icon=text-x-markdown
Terminal=false
Categories=Utility;TextEditor;
MimeType=text/markdown;text/x-markdown;text/mdx;
"#,
        exe_path
    );

    // Write desktop file to user's applications directory
    let home = std::env::var("HOME").unwrap_or_default();
    let desktop_path = format!("{}/.local/share/applications/mdview.desktop", home);

    if let Err(e) = std::fs::create_dir_all(format!("{}/.local/share/applications", home)) {
        return AssociationResult::Failed(format!("Failed to create applications directory: {}", e));
    }

    if let Err(e) = std::fs::write(&desktop_path, desktop_entry) {
        return AssociationResult::Failed(format!("Failed to write desktop file: {}", e));
    }

    // Register MIME type associations
    let mime_types = ["text/markdown", "text/x-markdown"];
    let mut all_succeeded = true;
    let mut errors = Vec::new();

    for mime in &mime_types {
        let result = Command::new("xdg-mime")
            .args(["default", "mdview.desktop", mime])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                log::debug!("Successfully set default for {}", mime);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::warn!("xdg-mime failed for {}: {}", mime, stderr);
                errors.push(format!("{}: {}", mime, stderr));
                all_succeeded = false;
            }
            Err(e) => {
                log::warn!("Failed to run xdg-mime for {}: {}", mime, e);
                errors.push(format!("{}: {}", mime, e));
                all_succeeded = false;
            }
        }
    }

    // Update desktop database
    let _ = Command::new("update-desktop-database")
        .arg(format!("{}/.local/share/applications", home))
        .output();

    if all_succeeded {
        AssociationResult::Success
    } else {
        AssociationResult::Failed(format!(
            "Some MIME associations failed: {}. Desktop file was created at {}",
            errors.join("; "),
            desktop_path
        ))
    }
}

// ============================================================================
// Windows Implementation
// ============================================================================

#[cfg(target_os = "windows")]
fn is_default_handler_windows() -> bool {
    use std::process::Command;

    // Query registry for .md file association
    let output = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.md\UserChoice",
            "/v",
            "ProgId",
        ])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.to_lowercase().contains("mdview")
        }
        Err(_) => false,
    }
}

#[cfg(target_os = "windows")]
fn register_windows() -> AssociationResult {
    let exe_path = match get_executable_path() {
        Some(p) => p,
        None => return AssociationResult::Failed("Could not determine executable path".to_string()),
    };

    // Create registry entries using reg.exe
    // Note: This may require elevated permissions

    // Register application
    let commands = [
        // Register the application
        format!(
            r#"reg add "HKCU\Software\Classes\mdview.md" /ve /d "Markdown Document" /f"#
        ),
        format!(
            r#"reg add "HKCU\Software\Classes\mdview.md\DefaultIcon" /ve /d "\"{}\"" /f"#,
            exe_path
        ),
        format!(
            r#"reg add "HKCU\Software\Classes\mdview.md\shell\open\command" /ve /d "\"{}\" \"%1\"" /f"#,
            exe_path
        ),
        // Associate .md extension
        format!(
            r#"reg add "HKCU\Software\Classes\.md" /ve /d "mdview.md" /f"#
        ),
        // Associate .markdown extension
        format!(
            r#"reg add "HKCU\Software\Classes\.markdown" /ve /d "mdview.md" /f"#
        ),
    ];

    for cmd in &commands {
        let result = Command::new("cmd")
            .args(["/C", cmd])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                log::debug!("Registry command succeeded: {}", cmd);
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Check for access denied errors which indicate need for admin
                if stderr.to_lowercase().contains("access")
                    || stderr.to_lowercase().contains("denied")
                    || output.status.code() == Some(5) // ERROR_ACCESS_DENIED
                {
                    return AssociationResult::Failed(
                        "Access denied. Try running mdview as Administrator, or \
                         manually set the default app in Windows Settings > Apps > Default apps."
                            .to_string()
                    );
                }
                return AssociationResult::Failed(format!(
                    "Registry command failed (exit {}): {}",
                    output.status.code().unwrap_or(-1),
                    stderr
                ));
            }
            Err(e) => {
                return AssociationResult::Failed(format!("Failed to run registry command: {}", e));
            }
        }
    }

    // Notify shell of the change
    let _ = Command::new("cmd")
        .args(["/C", "ie4uinit.exe -show"])
        .output();

    AssociationResult::Success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_executable_path() {
        let path = get_executable_path();
        assert!(path.is_some());
    }
}
