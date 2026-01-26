//! Mermaid diagram rendering support
//!
//! This module provides rendering of Mermaid diagrams to PNG images.
//! Supports native rendering (mermaid feature) and CLI fallback (mmdc).

use std::sync::OnceLock;
use std::process::Command;

/// Cached check for mmdc availability (computed once at startup)
static MMDC_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Check if mermaid-cli (mmdc) is available in PATH
pub fn is_mmdc_available() -> bool {
    Command::new("mmdc")
        .arg("--version")
        .output()
        .is_ok()
}

/// Get cached mmdc availability status
fn check_mmdc_available() -> bool {
    *MMDC_AVAILABLE.get_or_init(|| {
        let available = is_mmdc_available();
        if available {
            log::info!("mermaid-cli (mmdc) detected - will use for unsupported diagrams");
        } else {
            log::debug!("mermaid-cli (mmdc) not found - install with: npm install -g @mermaid-js/mermaid-cli");
        }
        available
    })
}

/// Render mermaid via official CLI (mmdc)
/// Requires: npm install -g @mermaid-js/mermaid-cli
pub fn render_mermaid_via_cli(code: &str, scale: f32) -> Result<Vec<u8>, String> {
    use std::fs;
    use std::env;
    use std::time::SystemTime;

    // Create unique temp files using timestamp and pid
    let temp_dir = env::temp_dir();
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let input_path = temp_dir.join(format!("mdview_mermaid_{}_{}.mmd", std::process::id(), timestamp));
    let output_path = temp_dir.join(format!("mdview_mermaid_{}_{}.png", std::process::id(), timestamp));

    // Write mermaid code to temp file
    fs::write(&input_path, code)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

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
        .map_err(|e| format!("Failed to run mmdc: {}", e))?;

    // Clean up input file
    let _ = fs::remove_file(&input_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_file(&output_path);
        return Err(format!("mmdc failed: {}", stderr));
    }

    // Read output PNG
    let png_data = fs::read(&output_path)
        .map_err(|e| format!("Failed to read output: {}", e))?;

    // Clean up output file
    let _ = fs::remove_file(&output_path);

    log::debug!("mermaid-cli rendered {} bytes", png_data.len());
    Ok(png_data)
}

/// Render mermaid code to PNG bytes using native renderer
#[cfg(feature = "mermaid")]
fn render_mermaid_native(code: &str, scale: f32) -> Result<Vec<u8>, String> {
    use mermaid_rs_renderer::render as render_mermaid_svg;

    // 1. Render to SVG using mermaid-rs-renderer
    let svg_string = render_mermaid_svg(code)
        .map_err(|e| format!("Mermaid parse error: {}", e))?;

    // 2. Parse SVG with usvg
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg_string, &opt)
        .map_err(|e| format!("SVG parse error: {}", e))?;

    // 3. Render to pixmap with resvg
    let size = tree.size();
    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;

    // Ensure minimum size
    let width = width.max(1);
    let height = height.max(1);

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or("Failed to create pixmap")?;

    // Fill with white background
    pixmap.fill(tiny_skia::Color::WHITE);

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // 4. Encode to PNG
    pixmap.encode_png()
        .map_err(|e| format!("PNG encode error: {}", e))
}

/// Render mermaid code to PNG bytes
///
/// Tries native rendering first (fast), falls back to CLI if available.
///
/// # Arguments
/// * `code` - The mermaid diagram source code
/// * `scale` - Scale factor for the output (2.0 recommended for HiDPI)
///
/// # Returns
/// PNG image bytes on success, or an error message on failure
#[cfg(feature = "mermaid")]
pub fn render_mermaid_to_png(code: &str, scale: f32) -> Result<Vec<u8>, String> {
    use std::panic;

    // Try native rendering with panic protection
    let code_owned = code.to_string();
    let native_result = panic::catch_unwind(move || {
        render_mermaid_native(&code_owned, scale)
    });

    match native_result {
        Ok(Ok(png)) => {
            log::debug!("Native mermaid renderer succeeded");
            return Ok(png);
        }
        Ok(Err(e)) => {
            log::debug!("Native mermaid failed: {}", e);
        }
        Err(_) => {
            log::debug!("Native mermaid panicked (dagre_rust bug)");
        }
    }

    // Try CLI fallback if available
    if check_mmdc_available() {
        log::debug!("Attempting mermaid-cli fallback...");
        return render_mermaid_via_cli(code, scale);
    }

    Err("Mermaid rendering failed. Install mermaid-cli for better support: npm install -g @mermaid-js/mermaid-cli".to_string())
}

/// Fallback when mermaid feature is not enabled - try CLI only
#[cfg(not(feature = "mermaid"))]
pub fn render_mermaid_to_png(code: &str, scale: f32) -> Result<Vec<u8>, String> {
    // Without native feature, only CLI is available
    if check_mmdc_available() {
        return render_mermaid_via_cli(code, scale);
    }
    Err("Mermaid rendering not available. Either compile with --features mermaid or install mermaid-cli: npm install -g @mermaid-js/mermaid-cli".to_string())
}

/// Check if mermaid rendering is available (native or CLI)
pub fn is_mermaid_available() -> bool {
    cfg!(feature = "mermaid") || check_mmdc_available()
}
