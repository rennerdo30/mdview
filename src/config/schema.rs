//! Configuration schema definitions

use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub window: WindowConfig,
    pub markdown: MarkdownConfig,
    pub annotations: AnnotationsConfig,
    pub export: ExportConfig,
    pub keybindings: KeybindingsConfig,
    pub theme: ThemeConfig,
    pub layout: LayoutConfig,
}

/// Layout configuration for content width and margins
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutConfig {
    /// Maximum content width in pixels (None = full width)
    pub content_width: Option<f32>,
    /// Maximum image width in pixels (None = content width)
    pub image_width: Option<f32>,
    /// Content margins (left/right)
    pub content_margin: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            content_width: Some(720.0),
            image_width: Some(600.0),
            content_margin: 48.0,
        }
    }
}


/// General application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Theme name (dark, light, or custom)
    pub theme: String,

    /// Enable hot reload / file watching
    pub hot_reload: bool,

    /// Show TOC sidebar by default
    pub show_toc: bool,

    /// Default TOC sidebar width
    pub toc_width: u32,

    /// Whether we've asked about file association (None = never asked)
    pub file_association_asked: bool,

    /// Whether mdview is registered as default .md handler
    pub file_association_enabled: bool,

    /// Check for updates on startup
    pub check_for_updates: bool,

    /// Version that was dismissed for update notification
    pub dismissed_update_version: Option<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            hot_reload: true,
            show_toc: true,
            toc_width: 250,
            file_association_asked: false,
            file_association_enabled: false,
            check_for_updates: true,
            dismissed_update_version: None,
        }
    }
}

/// Window settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1000,
            height: 700,
            maximized: false,
        }
    }
}

/// Markdown parsing options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MarkdownConfig {
    /// Enable table support
    pub tables: bool,

    /// Enable strikethrough
    pub strikethrough: bool,

    /// Enable task lists
    pub task_lists: bool,

    /// Enable footnotes
    pub footnotes: bool,

    /// Enable smart punctuation
    pub smart_punctuation: bool,

    /// Enable syntax highlighting for code blocks
    pub syntax_highlighting: bool,

    /// Syntax highlighting theme
    pub syntax_theme: String,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            tables: true,
            strikethrough: true,
            task_lists: true,
            footnotes: true,
            smart_punctuation: false,
            syntax_highlighting: true,
            syntax_theme: "base16-ocean.dark".to_string(),
        }
    }
}

/// Annotations settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnnotationsConfig {
    /// Enable annotations feature
    pub enabled: bool,

    /// Auto-save annotations on change
    pub auto_save: bool,

    /// Default highlight color (hex)
    pub default_highlight_color: String,
}

impl Default for AnnotationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            auto_save: true,
            default_highlight_color: "#ffeb3b".to_string(),
        }
    }
}

/// Export settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportConfig {
    /// Theme to use for PDF export
    pub pdf_theme: String,

    /// Include TOC in PDF
    pub include_toc: bool,

    /// PDF page size (A4, Letter, etc.)
    pub page_size: String,

    /// PDF margins in mm
    pub margin: u32,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            pdf_theme: "light".to_string(),
            include_toc: true,
            page_size: "A4".to_string(),
            margin: 20,
        }
    }
}

/// Keybindings configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingsConfig {
    pub toggle_toc: String,
    pub export_pdf: String,
    pub reload: String,
    pub open_file: String,
    pub open_folder: String,
    pub quit: String,
    pub add_annotation: String,
    pub add_bookmark: String,
    pub toggle_file_browser: String,
    pub focus_toc_search: String,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            toggle_toc: "Ctrl+T".to_string(),
            export_pdf: "Ctrl+P".to_string(),
            reload: "F5".to_string(),
            open_file: "Ctrl+O".to_string(),
            open_folder: "Ctrl+Shift+O".to_string(),
            quit: "Ctrl+Q".to_string(),
            add_annotation: "Ctrl+H".to_string(),
            add_bookmark: "Ctrl+B".to_string(),
            toggle_file_browser: "Ctrl+E".to_string(),
            focus_toc_search: "Ctrl+F".to_string(),
        }
    }
}

/// Theme customization
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ThemeConfig {
    /// Custom colors (overrides built-in theme)
    pub colors: ThemeColors,

    /// Font settings
    pub fonts: FontConfig,

    /// Spacing settings
    pub spacing: SpacingConfig,
}

/// Theme color definitions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ThemeColors {
    /// Background color (hex)
    pub background: Option<String>,

    /// Text color (hex)
    pub text: Option<String>,

    /// Heading color (hex)
    pub heading: Option<String>,

    /// Link color (hex)
    pub link: Option<String>,

    /// Code background color (hex)
    pub code_background: Option<String>,

    /// Code text color (hex)
    pub code_text: Option<String>,

    /// Sidebar background color (hex)
    pub sidebar_background: Option<String>,

    /// Selection color (hex)
    pub selection: Option<String>,
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    /// Body font family
    pub body: String,

    /// Heading font family
    pub heading: String,

    /// Code font family
    pub code: String,

    /// Base font size
    pub size: f32,

    /// Line height multiplier
    pub line_height: f32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            body: "sans-serif".to_string(),
            heading: "sans-serif".to_string(),
            code: "monospace".to_string(),
            size: 14.0,
            line_height: 1.5,
        }
    }
}

/// Spacing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SpacingConfig {
    /// Paragraph spacing
    pub paragraph: f32,

    /// Heading spacing (top)
    pub heading_top: f32,

    /// Heading spacing (bottom)
    pub heading_bottom: f32,

    /// List item indent
    pub list_indent: f32,

    /// Code block padding
    pub code_padding: f32,
}

impl Default for SpacingConfig {
    fn default() -> Self {
        Self {
            paragraph: 12.0,
            heading_top: 24.0,
            heading_bottom: 8.0,
            list_indent: 20.0,
            code_padding: 8.0,
        }
    }
}
