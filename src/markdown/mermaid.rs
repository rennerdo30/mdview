//! Mermaid diagram rendering support
//!
//! This module provides rendering of Mermaid diagrams to PNG images via CLI.
//! Native rendering is disabled due to upstream repo issues on Windows.
//! Install mermaid-cli for support: npm install -g @mermaid-js/mermaid-cli

use std::process::Command;
use std::sync::OnceLock;

/// Cached check for mmdc availability (computed once at startup)
static MMDC_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if mermaid-cli (mmdc) is available in PATH
pub fn is_mmdc_available() -> bool {
    Command::new("mmdc").arg("--version").output().is_ok()
}

/// Get cached mmdc availability status
fn check_mmdc_available() -> bool {
    *MMDC_AVAILABLE.get_or_init(|| {
        let available = is_mmdc_available();
        if available {
            log::info!("mermaid-cli (mmdc) detected");
        } else {
            log::debug!("mermaid-cli (mmdc) not found - install with: npm install -g @mermaid-js/mermaid-cli");
        }
        available
    })
}

/// Render mermaid via official CLI (mmdc)
/// Requires: npm install -g @mermaid-js/mermaid-cli
pub fn render_mermaid_via_cli(code: &str, scale: f32) -> Result<Vec<u8>, String> {
    use std::env;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    // Create unique temp files using PID + atomic counter (collision-free)
    let temp_dir = env::temp_dir();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let input_path = temp_dir.join(format!("mdview_mermaid_{}_{}.mmd", std::process::id(), seq));
    let output_path = temp_dir.join(format!("mdview_mermaid_{}_{}.png", std::process::id(), seq));

    // Write mermaid code to temp file
    fs::write(&input_path, code).map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Calculate dimensions based on scale
    let width = (800.0 * scale) as u32;
    let height = (600.0 * scale) as u32;

    // Call mmdc
    let output = Command::new("mmdc")
        .arg("-i")
        .arg(&input_path)
        .arg("-o")
        .arg(&output_path)
        .arg("-w")
        .arg(width.to_string())
        .arg("-H")
        .arg(height.to_string())
        .arg("-b")
        .arg("white")
        .output()
        .map_err(|e| {
            // Clean up input on command failure
            if let Err(ce) = fs::remove_file(&input_path) {
                log::warn!(
                    "Failed to clean up mermaid temp input {:?}: {}",
                    input_path,
                    ce
                );
            }
            format!("Failed to run mmdc: {}", e)
        })?;

    // Clean up input file
    if let Err(e) = fs::remove_file(&input_path) {
        log::warn!(
            "Failed to clean up mermaid temp input {:?}: {}",
            input_path,
            e
        );
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if let Err(e) = fs::remove_file(&output_path) {
            log::debug!(
                "Failed to clean up mermaid temp output {:?}: {}",
                output_path,
                e
            );
        }
        return Err(format!("mmdc failed: {}", stderr));
    }

    // Read output PNG
    let png_data = fs::read(&output_path).map_err(|e| format!("Failed to read output: {}", e))?;

    // Clean up output file
    if let Err(e) = fs::remove_file(&output_path) {
        log::warn!(
            "Failed to clean up mermaid temp output {:?}: {}",
            output_path,
            e
        );
    }

    log::debug!("mermaid-cli rendered {} bytes", png_data.len());
    Ok(png_data)
}

/// Render mermaid code to PNG bytes
///
/// Uses mermaid-cli (mmdc) for rendering.
///
/// # Arguments
/// * `code` - The mermaid diagram source code
/// * `scale` - Scale factor for the output (2.0 recommended for HiDPI)
///
/// # Returns
/// PNG image bytes on success, or an error message on failure
pub fn render_mermaid_to_png(code: &str, scale: f32) -> Result<Vec<u8>, String> {
    if check_mmdc_available() {
        return render_mermaid_via_cli(code, scale);
    }
    Err("Mermaid rendering not available. Install mermaid-cli: npm install -g @mermaid-js/mermaid-cli".to_string())
}

/// Check if mermaid rendering is available
pub fn is_mermaid_available() -> bool {
    check_mmdc_available()
}
