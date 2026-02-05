//! Main viewer application implementing eframe::App
//!
//! Features a refined, modern UI inspired by Linear and modern IDEs.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui::{self, Key, Modifiers, Rounding, Stroke, Vec2};

use super::state::{AppState, FileEvent};

/// Parsed keybinding
#[derive(Debug, Clone)]
struct ParsedKeybinding {
    key: Key,
    modifiers: Modifiers,
}

/// Cached keybindings parsed at startup
#[derive(Debug, Clone, Default)]
struct CachedKeybindings {
    toggle_toc: Option<ParsedKeybinding>,
    export_pdf: Option<ParsedKeybinding>,
    reload: Option<ParsedKeybinding>,
    open_file: Option<ParsedKeybinding>,
    open_folder: Option<ParsedKeybinding>,
    toggle_file_browser: Option<ParsedKeybinding>,
    quit: Option<ParsedKeybinding>,
    add_annotation: Option<ParsedKeybinding>,
    add_bookmark: Option<ParsedKeybinding>,
    focus_toc_search: Option<ParsedKeybinding>,
    zoom_in: Option<ParsedKeybinding>,
    zoom_out: Option<ParsedKeybinding>,
    zoom_reset: Option<ParsedKeybinding>,
}

/// Debounce duration for config writes
const CONFIG_SAVE_DEBOUNCE_MS: u64 = 400;
const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");

impl CachedKeybindings {
    fn from_config(config: &crate::config::schema::KeybindingsConfig) -> Self {
        Self {
            toggle_toc: parse_keybinding(&config.toggle_toc),
            export_pdf: parse_keybinding(&config.export_pdf),
            reload: parse_keybinding(&config.reload),
            open_file: parse_keybinding(&config.open_file),
            open_folder: parse_keybinding(&config.open_folder),
            toggle_file_browser: parse_keybinding(&config.toggle_file_browser),
            quit: parse_keybinding(&config.quit),
            add_annotation: parse_keybinding(&config.add_annotation),
            add_bookmark: parse_keybinding(&config.add_bookmark),
            focus_toc_search: parse_keybinding(&config.focus_toc_search),
            zoom_in: parse_keybinding(&config.zoom_in),
            zoom_out: parse_keybinding(&config.zoom_out),
            zoom_reset: parse_keybinding(&config.zoom_reset),
        }
    }
}

/// Actions triggered by menu interactions, collected during UI rendering
/// and processed afterwards to avoid borrow conflicts
#[derive(Default)]
struct MenuActions {
    open_dialog: bool,
    open_folder_dialog: bool,
    reload: bool,
    export_pdf: bool,
    quit: bool,
    toggle_toc: bool,
    toggle_file_browser: bool,
    clear_recent: bool,
    show_help: bool,
    show_about: bool,
    edit_config: bool,
    file_to_open: Option<PathBuf>,
    new_theme: Option<String>,
    /// New reading width setting (None = full width)
    new_reading_width: Option<Option<f32>>,
    /// Plugin menu item callback to invoke
    #[cfg(feature = "plugins")]
    plugin_menu_callback: Option<String>,
}

/// Parse a keybinding string like "Ctrl+T" or "F5" into Key and Modifiers
fn parse_keybinding(binding: &str) -> Option<ParsedKeybinding> {
    let parts: Vec<&str> = binding.split('+').map(|s| s.trim()).collect();

    let mut modifiers = Modifiers::NONE;
    let mut key_str = "";

    for part in &parts {
        let lower = part.to_lowercase();
        match lower.as_str() {
            "ctrl" | "control" | "cmd" | "command" => modifiers.command = true,
            "alt" | "option" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "super" | "meta" | "win" => modifiers.command = true,
            _ => key_str = part,
        }
    }

    let key = match key_str.to_uppercase().as_str() {
        "A" => Key::A, "B" => Key::B, "C" => Key::C, "D" => Key::D,
        "E" => Key::E, "F" => Key::F, "G" => Key::G, "H" => Key::H,
        "I" => Key::I, "J" => Key::J, "K" => Key::K, "L" => Key::L,
        "M" => Key::M, "N" => Key::N, "O" => Key::O, "P" => Key::P,
        "Q" => Key::Q, "R" => Key::R, "S" => Key::S, "T" => Key::T,
        "U" => Key::U, "V" => Key::V, "W" => Key::W, "X" => Key::X,
        "Y" => Key::Y, "Z" => Key::Z,
        "0" => Key::Num0, "1" => Key::Num1, "2" => Key::Num2, "3" => Key::Num3,
        "4" => Key::Num4, "5" => Key::Num5, "6" => Key::Num6, "7" => Key::Num7,
        "8" => Key::Num8, "9" => Key::Num9,
        "F1" => Key::F1, "F2" => Key::F2, "F3" => Key::F3, "F4" => Key::F4,
        "F5" => Key::F5, "F6" => Key::F6, "F7" => Key::F7, "F8" => Key::F8,
        "F9" => Key::F9, "F10" => Key::F10, "F11" => Key::F11, "F12" => Key::F12,
        "ESCAPE" | "ESC" => Key::Escape,
        "ENTER" | "RETURN" => Key::Enter,
        "SPACE" => Key::Space,
        "TAB" => Key::Tab,
        "BACKSPACE" => Key::Backspace,
        "DELETE" => Key::Delete,
        "HOME" => Key::Home,
        "END" => Key::End,
        "PAGEUP" => Key::PageUp,
        "PAGEDOWN" => Key::PageDown,
        "UP" | "ARROWUP" => Key::ArrowUp,
        "DOWN" | "ARROWDOWN" => Key::ArrowDown,
        "LEFT" | "ARROWLEFT" => Key::ArrowLeft,
        "RIGHT" | "ARROWRIGHT" => Key::ArrowRight,
        "=" | "EQUAL" | "EQUALS" => Key::Equals,
        "+" => Key::Plus,
        "-" | "MINUS" | "DASH" => Key::Minus,
        _ => return None,
    };

    Some(ParsedKeybinding { key, modifiers })
}

/// Check if a keybinding is pressed (used for dynamic keybindings)
#[allow(dead_code)]
fn is_keybinding_pressed(ctx: &egui::Context, binding: &str) -> bool {
    if let Some(parsed) = parse_keybinding(binding) {
        is_parsed_keybinding_pressed(ctx, &parsed)
    } else {
        false
    }
}

/// Check if a pre-parsed keybinding is pressed (more efficient for cached keybindings)
fn is_parsed_keybinding_pressed(ctx: &egui::Context, parsed: &ParsedKeybinding) -> bool {
    ctx.input(|i| {
        i.key_pressed(parsed.key) &&
        i.modifiers.command == parsed.modifiers.command &&
        i.modifiers.alt == parsed.modifiers.alt &&
        i.modifiers.shift == parsed.modifiers.shift
    })
}

/// Format a file load error with user-friendly messages
/// Format file operation errors with user-friendly messages
mod friendly_errors {
    use std::io::ErrorKind;

    pub fn format_load_error(e: &std::io::Error) -> String {
        match e.kind() {
            ErrorKind::NotFound => "File not found. It may have been moved or deleted.".to_string(),
            ErrorKind::PermissionDenied => "Permission denied. Check file permissions.".to_string(),
            ErrorKind::InvalidData => "File contains invalid data. It may be corrupted.".to_string(),
            _ => format!("Could not open file: {}", e),
        }
    }

    pub fn format_folder_error(e: &std::io::Error) -> String {
        match e.kind() {
            ErrorKind::NotFound => "Folder not found. It may have been moved or deleted.".to_string(),
            ErrorKind::PermissionDenied => "Cannot access folder. Check permissions.".to_string(),
            _ => format!("Could not open folder: {}", e),
        }
    }

    pub fn format_reload_error(e: &std::io::Error) -> String {
        match e.kind() {
            ErrorKind::NotFound => "File no longer exists. It may have been deleted.".to_string(),
            ErrorKind::PermissionDenied => "Cannot reload: permission denied.".to_string(),
            _ => format!("Reload failed: {}", e),
        }
    }

    pub fn format_export_error(e: &crate::export::pdf::PdfError) -> String {
        match e {
            crate::export::pdf::PdfError::Io(msg) => {
                if msg.contains("permission") || msg.contains("Permission") {
                    "Cannot save PDF: permission denied. Try a different location.".to_string()
                } else if msg.contains("space") {
                    "Cannot save PDF: not enough disk space.".to_string()
                } else {
                    format!("Could not save PDF: {}", msg)
                }
            }
            crate::export::pdf::PdfError::Pdf(msg) => {
                format!("PDF generation error: {}", msg)
            }
        }
    }

    pub fn format_config_error(action: &str, e: &impl std::fmt::Display) -> String {
        format!("Could not {} config file: {}", action, e)
    }

}

fn format_load_error(e: &std::io::Error) -> String {
    friendly_errors::format_load_error(e)
}

/// Platform-specific shortcut helper
mod shortcuts {
    /// Get the platform-specific modifier key symbol
    pub fn modifier() -> &'static str {
        #[cfg(target_os = "macos")]
        { "\u{2318}" } // ⌘
        #[cfg(not(target_os = "macos"))]
        { "Ctrl+" }
    }

    /// Get the platform-specific shift modifier
    pub fn shift_modifier() -> &'static str {
        #[cfg(target_os = "macos")]
        { "\u{21E7}" } // ⇧
        #[cfg(not(target_os = "macos"))]
        { "Shift+" }
    }

    /// Format a shortcut with the key (e.g., "O" -> "⌘O" or "Ctrl+O")
    pub fn format(key: &str) -> String {
        format!("{}{}", modifier(), key)
    }

    /// Format a shortcut with shift (e.g., "O" -> "⇧⌘O" or "Ctrl+Shift+O")
    pub fn format_shift(key: &str) -> String {
        #[cfg(target_os = "macos")]
        { format!("{}{}{}", shift_modifier(), modifier(), key) }
        #[cfg(not(target_os = "macos"))]
        { format!("{}{}{}",  modifier(), shift_modifier(), key) }
    }

    /// Format just a key without modifier (e.g., "F5")
    pub fn key_only(key: &str) -> String {
        key.to_string()
    }
}

use crate::annotations::ui::{AnnotationPopup, AnnotationAction};
use crate::app::file_browser::{FileBrowserPanel, rfd_open_folder};
use crate::config::Config;
use crate::markdown::renderer::MarkdownRenderer;
use crate::theme::style::{create_style, palette};
use crate::toc::panel::TocPanel;
use crate::update::UpdateChecker;
use crate::watcher::file_watcher::FileWatcher;

/// Main mdview application
pub struct MdViewApp {
    /// Application state
    pub state: AppState,
    watcher: Option<FileWatcher>,
    /// File currently being watched for hot reload
    watched_file: Option<PathBuf>,
    renderer: MarkdownRenderer,
    toc_panel: TocPanel,
    annotation_popup: AnnotationPopup,
    file_browser: FileBrowserPanel,
    /// Whether the file browser sidebar is visible
    pub file_browser_visible: bool,
    /// Whether we've shown the plugin failure notification
    #[cfg_attr(not(feature = "plugins"), allow(dead_code))]
    shown_plugin_notification: bool,
    /// Whether to show the file association dialog
    show_file_association_dialog: bool,
    /// Update checker for GitHub releases
    update_checker: UpdateChecker,
    /// Whether to show the update notification dialog
    show_update_dialog: bool,
    /// Whether to show the Quick Help dialog
    show_help_dialog: bool,
    /// Whether to show the About dialog
    show_about_dialog: bool,
    /// Native menu bar (macOS/Windows/Linux)
    native_menu: Option<crate::native_menu::NativeMenuBar>,
    /// Cached result of is_default_handler check (to avoid spawning processes every frame)
    cached_is_default_handler: Option<bool>,
    /// Cached keybindings parsed at startup (avoid parsing every frame)
    cached_keybindings: CachedKeybindings,
    /// Last time a config save was requested (for debouncing)
    config_save_requested_at: Option<Instant>,
    /// Whether a file is being dragged over the window (for visual feedback)
    is_dragging_file: bool,
    /// Pending file path to load after fade-out transition completes
    pending_file_load: Option<PathBuf>,
    /// Anchor offset used while dragging to create a text selection
    selection_drag_anchor: Option<usize>,
}

impl MdViewApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        file: Option<PathBuf>,
        config: Config,
    ) -> Self {
        // Apply our refined style
        let style = create_style(&config.general.theme, &config);
        cc.egui_ctx.set_style(style);

        // Set up custom fonts
        setup_fonts(&cc.egui_ctx, &config);

        // Get default highlight color before moving config
        let default_highlight_color = config.annotations.default_highlight_color.clone();

        // Parse keybindings once at startup
        let cached_keybindings = CachedKeybindings::from_config(&config.keybindings);

        let mut state = AppState::new(config);
        let mut renderer = MarkdownRenderer::new();

        // Load file if provided
        if let Some(path) = file {
            // Set base path for image resolution
            let base_path = path.parent().map(|p| p.to_path_buf());
            renderer.set_base_path(base_path);

            if let Err(e) = state.load_file(path.clone()) {
                log::error!("Failed to load file: {}", e);
                state.set_status(format_load_error(&e));
            }
        }

        // Set up file watcher (only when opening directly into a file)
        let (watcher, watched_file) = if state.config.general.hot_reload {
            if let Some(path) = state.current_file.clone() {
                let watcher = setup_file_watcher(&mut state, cc.egui_ctx.clone(), path.clone());
                (watcher, Some(path))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Initialize annotation popup with default color from config
        let mut annotation_popup = AnnotationPopup::new();
        // Find the index of the default color in HIGHLIGHT_COLORS, or default to 0
        for (idx, (color, _)) in crate::annotations::ui::HIGHLIGHT_COLORS.iter().enumerate() {
            if *color == default_highlight_color {
                annotation_popup.selected_color = idx;
                break;
            }
        }

        // Check if we should show file association dialog
        let show_file_association_dialog = !state.config.general.file_association_asked;

        // Initialize update checker and start async check if enabled
        let mut update_checker = UpdateChecker::new();
        if state.config.general.check_for_updates {
            // Check if this version was already dismissed
            let dismissed_version = state.config.general.dismissed_update_version.clone();
            update_checker.check_async();
            // We'll check the dismissed version when results come in
            if let Some(ref ver) = dismissed_version {
                // Mark as dismissed if the user previously dismissed this version
                // (will be checked when results arrive)
                log::debug!("Previously dismissed update version: {}", ver);
            }
        }

        Self {
            state,
            watcher,
            watched_file,
            renderer,
            toc_panel: TocPanel::new(),
            annotation_popup,
            file_browser: FileBrowserPanel::new(),
            file_browser_visible: false,
            shown_plugin_notification: false,
            show_file_association_dialog,
            update_checker,
            show_update_dialog: false,
            show_help_dialog: false,
            show_about_dialog: false,
            native_menu: None,
            cached_is_default_handler: None,
            cached_keybindings,
            config_save_requested_at: None,
            is_dragging_file: false,
            pending_file_load: None,
            selection_drag_anchor: None,
        }
    }

    /// Set the native menu bar (called from main after app creation)
    pub fn set_native_menu(&mut self, menu: Option<crate::native_menu::NativeMenuBar>) {
        self.native_menu = menu;
        // Sync state immediately (in case a file was loaded via CLI)
        self.sync_native_menu_state();
    }

    /// Initialize native menu for Windows (called on first frame when we have HWND)
    #[cfg(windows)]
    fn init_native_menu_windows(&mut self, frame: &eframe::Frame) {
        use raw_window_handle::HasWindowHandle;

        if let Some(ref mut menu) = self.native_menu {
            if menu.needs_init() {
                if let Ok(handle) = frame.window_handle() {
                    if let raw_window_handle::RawWindowHandle::Win32(win32) = handle.as_raw() {
                        menu.init_for_hwnd(win32.hwnd.get() as isize);
                    }
                }
            }
        }
    }

    /// Open a folder for browsing
    fn open_folder_dialog(&mut self) {
        if let Some(path) = rfd_open_folder() {
            if let Err(e) = self.state.open_folder(path) {
                self.state.set_status(friendly_errors::format_folder_error(&e));
            } else {
                self.file_browser_visible = true;
                self.state.set_status("Folder opened");
            }
        }
    }

    /// Render the file association dialog
    fn render_file_association_dialog(&mut self, ctx: &egui::Context) {
        // Cache the default handler check to avoid spawning processes every frame
        let is_already_default = self.cached_is_default_handler.get_or_insert_with(|| {
            crate::file_association::is_default_handler()
        });
        let is_already_default = *is_already_default;

        let mut should_close = false;
        let mut should_register = false;

        egui::Window::new("Set as Default Markdown Viewer?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(8.0);

                if is_already_default {
                    ui.label("mdview is already set as the default markdown viewer.");
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        should_close = true;
                    }
                } else {
                    ui.label("Would you like to set mdview as the default application for opening markdown files (.md)?");

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        if ui.button("Yes, set as default").clicked() {
                            should_register = true;
                            should_close = true;
                        }

                        ui.add_space(8.0);

                        if ui.button("No, thanks").clicked() {
                            should_close = true;
                        }

                        ui.add_space(8.0);

                        if ui.button("Ask me later").clicked() {
                            // Don't mark as asked, just close for this session
                            self.show_file_association_dialog = false;
                        }
                    });
                }

                ui.add_space(8.0);
            });

        if should_register {
            use crate::file_association::{register_as_default, AssociationResult};

            match register_as_default() {
                AssociationResult::Success => {
                    self.state.config.general.file_association_enabled = true;
                    self.state.set_status("Successfully set as default markdown viewer");
                }
                AssociationResult::Failed(msg) => {
                    self.state.set_status(format!("Could not set as default: {}", msg));
                }
                AssociationResult::NotSupported => {
                    self.state.set_status("File association not supported on this platform");
                }
            }
        }

        if should_close {
            self.show_file_association_dialog = false;
            self.state.config.general.file_association_asked = true;

            // Save the config
            if let Some(config_path) = crate::config::loader::get_default_config_path() {
                if let Err(e) = crate::config::loader::save_config(&self.state.config, &config_path) {
                    log::warn!("Failed to save config: {}", e);
                }
            }
        }
    }

    /// Render the update available dialog
    fn render_update_dialog(&mut self, ctx: &egui::Context) {
        let update_info = match self.update_checker.update_info() {
            Some(info) => info.clone(),
            None => return,
        };

        let mut should_close = false;
        let mut should_dismiss = false;
        let mut should_open_url = false;

        egui::Window::new("Update Available")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(350.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                ui.label(
                    egui::RichText::new(format!(
                        "A new version of mdview is available: v{}",
                        update_info.version
                    ))
                    .strong()
                );

                ui.add_space(4.0);

                ui.label(format!(
                    "You are currently running v{}",
                    crate::update::CURRENT_VERSION
                ));

                // Show release name if available
                if let Some(ref name) = update_info.name {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(name).italics());
                }

                // Show truncated release notes if available
                if let Some(ref notes) = update_info.notes {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .show(ui, |ui| {
                            ui.label(notes);
                        });
                }

                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    if ui.button("View Release").clicked() {
                        should_open_url = true;
                        should_close = true;
                    }

                    ui.add_space(8.0);

                    if ui.button("Remind Me Later").clicked() {
                        should_close = true;
                    }

                    ui.add_space(8.0);

                    if ui.button("Skip This Version").clicked() {
                        should_dismiss = true;
                        should_close = true;
                    }
                });

                ui.add_space(8.0);
            });

        if should_open_url {
            if let Err(e) = open::that(&update_info.url) {
                self.state.set_status(format!("Could not open browser: {}", e));
            }
        }

        if should_dismiss {
            // Remember this version as dismissed
            self.state.config.general.dismissed_update_version = Some(update_info.version.clone());
            self.update_checker.dismiss();

            // Save config
            if let Some(config_path) = crate::config::loader::get_default_config_path() {
                if let Err(e) = crate::config::loader::save_config(&self.state.config, &config_path) {
                    log::warn!("Failed to save config: {}", e);
                }
            }
        }

        if should_close {
            self.show_update_dialog = false;
        }
    }

    /// Render the About dialog
    fn render_about_dialog(&mut self, ctx: &egui::Context) {
        let mut should_close = false;
        let mut should_open_repo = false;
        let mut should_open_issues = false;
        let issue_url = format!("{}/issues", REPOSITORY_URL.trim_end_matches('/'));

        egui::Window::new("About mdview")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(300.0)
            .show(ctx, |ui| {
                ui.add_space(16.0);

                ui.vertical_centered(|ui| {
                    // App name
                    ui.label(
                        egui::RichText::new("mdview")
                            .size(24.0)
                            .strong()
                            .color(palette::TEXT_PRIMARY)
                    );

                    ui.add_space(4.0);

                    // Version
                    ui.label(
                        egui::RichText::new(format!("Version {}", crate::update::CURRENT_VERSION))
                            .size(14.0)
                            .color(palette::TEXT_SECONDARY)
                    );

                    ui.add_space(16.0);

                    // Description
                    ui.label(
                        egui::RichText::new("A fast, cross-platform markdown viewer")
                            .size(13.0)
                            .color(palette::TEXT_MUTED)
                    );

                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new("Built with Rust and egui")
                            .size(12.0)
                            .color(palette::TEXT_DISABLED)
                    );

                    ui.add_space(16.0);

                    // Links
                    ui.horizontal(|ui| {
                        if ui.link("GitHub").clicked() {
                            should_open_repo = true;
                        }
                        ui.label(" | ");
                        if ui.link("Report Issue").clicked() {
                            should_open_issues = true;
                        }
                    });

                    ui.add_space(16.0);

                    // Close button
                    if ui.button("Close").clicked() {
                        should_close = true;
                    }
                });

                ui.add_space(8.0);
            });

        if should_open_repo {
            if let Err(e) = open::that(REPOSITORY_URL) {
                log::error!("Failed to open repository URL: {}", e);
                self.state
                    .set_status(format!("Could not open link: {}", e));
            }
        }

        if should_open_issues {
            if let Err(e) = open::that(&issue_url) {
                log::error!("Failed to open issues URL: {}", e);
                self.state
                    .set_status(format!("Could not open link: {}", e));
            }
        }

        if should_close {
            self.show_about_dialog = false;
        }
    }

    /// Render the Quick Help dialog
    fn render_help_dialog(&mut self, ctx: &egui::Context) {
        let mut should_close = false;
        let mut should_open_repo = false;
        let mut should_open_issues = false;
        let issue_url = format!("{}/issues", REPOSITORY_URL.trim_end_matches('/'));
        let kb = &self.state.config.keybindings;

        egui::Window::new("Quick Help")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(430.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Getting Started").strong());
                ui.add_space(4.0);
                ui.label("Open a file from File -> Open File... or drag and drop a .md file.");
                ui.label("Use View -> Show/Hide Contents to toggle the table of contents.");
                ui.label("Open a folder to browse files with the sidebar.");

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(egui::RichText::new("Keyboard Shortcuts").strong());
                ui.add_space(6.0);

                egui::Grid::new("quick_help_shortcuts")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Open file");
                        ui.monospace(&kb.open_file);
                        ui.end_row();

                        ui.label("Open folder");
                        ui.monospace(&kb.open_folder);
                        ui.end_row();

                        ui.label("Toggle contents");
                        ui.monospace(&kb.toggle_toc);
                        ui.end_row();

                        ui.label("Toggle file browser");
                        ui.monospace(&kb.toggle_file_browser);
                        ui.end_row();

                        ui.label("Reload file");
                        ui.monospace(&kb.reload);
                        ui.end_row();

                        ui.label("Export PDF");
                        ui.monospace(&kb.export_pdf);
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.link("GitHub").clicked() {
                        should_open_repo = true;
                    }
                    ui.label(" | ");
                    if ui.link("Report Issue").clicked() {
                        should_open_issues = true;
                    }
                });

                ui.add_space(12.0);
                if ui.button("Close").clicked() {
                    should_close = true;
                }
            });

        if should_open_repo {
            if let Err(e) = open::that(REPOSITORY_URL) {
                log::error!("Failed to open repository URL: {}", e);
                self.state
                    .set_status(format!("Could not open link: {}", e));
            }
        }

        if should_open_issues {
            if let Err(e) = open::that(&issue_url) {
                log::error!("Failed to open issues URL: {}", e);
                self.state
                    .set_status(format!("Could not open link: {}", e));
            }
        }

        if should_close {
            self.show_help_dialog = false;
        }
    }

    fn handle_native_menu_events(&mut self, ctx: &egui::Context) {
        use crate::native_menu::MenuAction;

        // Collect all actions first to avoid borrow issues
        let actions = match self.native_menu {
            Some(ref menu) => menu.poll_all(),
            None => return,
        };

        for action in actions {
            match action {
                MenuAction::Open => {
                    if let Some(path) = rfd_open_file() {
                        self.load_markdown_file(path);
                    }
                }
                MenuAction::OpenFolder => {
                    self.open_folder_dialog();
                }
                MenuAction::Reload => {
                    if let Err(e) = self.state.reload_file() {
                        self.state.set_status(friendly_errors::format_reload_error(&e));
                    }
                }
                MenuAction::Close => {
                    // Clear current file
                    self.state.current_file = None;
                    self.state.clear_content();
                }
                MenuAction::ExportPdf => {
                    if self.state.current_file.is_some() {
                        self.state.exporting_pdf = true;
                    }
                }
                MenuAction::EditConfig => {
                    match crate::config::loader::create_default_config() {
                        Ok(config_path) => {
                            if let Err(e) = open::that(&config_path) {
                                log::error!("Failed to open config file: {}", e);
                                self.state.set_status(friendly_errors::format_config_error("open", &e));
                            } else {
                                self.state.set_status(format!("Config file opened: {}", config_path.display()));
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to create config file: {}", e);
                            self.state.set_status(friendly_errors::format_config_error("create", &e));
                        }
                    }
                }
                MenuAction::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                MenuAction::ToggleToc => {
                    self.state.toggle_toc();
                }
                MenuAction::ToggleFileBrowser => {
                    self.file_browser_visible = !self.file_browser_visible;
                }
                MenuAction::ZoomIn => {
                    self.apply_zoom_delta(ctx, 1.0);
                }
                MenuAction::ZoomOut => {
                    self.apply_zoom_delta(ctx, -1.0);
                }
                MenuAction::ZoomReset => {
                    self.reset_zoom(ctx);
                }
                MenuAction::About => {
                    self.show_about_dialog = true;
                }
                MenuAction::QuickHelp => {
                    self.show_help_dialog = true;
                }
                MenuAction::CheckUpdates => {
                    self.update_checker.check_async();
                    self.show_update_dialog = true;
                }
                MenuAction::SetReadingWidth(width) => {
                    self.state.config.layout.content_width = width;
                    let width_desc = match width {
                        None => "Full Width".to_string(),
                        Some(w) => format!("{}px", w as i32),
                    };
                    self.state.set_status(format!("Reading width: {}", width_desc));
                    self.save_config_debounced(ctx);
                }
            }
        }

        // Sync native menu state after handling actions
        self.sync_native_menu_state();
    }

    /// Synchronize native menu item states with application state
    fn sync_native_menu_state(&self) {
        if let Some(ref menu) = self.native_menu {
            let has_file = self.state.current_file.is_some();
            let file_name = self.state.current_file
                .as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str());
            menu.update_state(has_file, file_name);
        }
    }

    fn clear_file_watcher(&mut self) {
        self.watcher = None;
        self.watched_file = None;
        self.state.file_event_rx = None;
        self.state.file_event_tx = None;
    }

    fn floor_content_boundary(&self, offset: usize) -> Option<usize> {
        let content = self.state.content.as_str();
        if content.is_empty() {
            return None;
        }

        let mut idx = offset.min(content.len());
        while idx > 0 && !content.is_char_boundary(idx) {
            idx -= 1;
        }
        Some(idx)
    }

    fn ceil_content_boundary(&self, offset: usize) -> Option<usize> {
        let content = self.state.content.as_str();
        if content.is_empty() {
            return None;
        }

        let mut idx = offset.min(content.len());
        while idx < content.len() && !content.is_char_boundary(idx) {
            idx += 1;
        }
        Some(idx)
    }

    fn normalize_selection_range(&self, start: usize, end: usize) -> Option<(usize, usize)> {
        if self.state.content.is_empty() {
            return None;
        }

        let (raw_start, raw_end) = if start <= end { (start, end) } else { (end, start) };
        let normalized_start = self.floor_content_boundary(raw_start)?;
        let normalized_end = self.ceil_content_boundary(raw_end)?;
        if normalized_end > normalized_start {
            Some((normalized_start, normalized_end))
        } else {
            None
        }
    }

    fn normalize_bookmark_offset(&self, offset: usize) -> Option<usize> {
        let content = self.state.content.as_str();
        if content.is_empty() {
            return None;
        }

        let clamped = offset.min(content.len().saturating_sub(1));
        self.floor_content_boundary(clamped)
    }

    /// Keep the file watcher in sync with the current file and hot-reload setting.
    fn sync_file_watcher(&mut self, ctx: &egui::Context) {
        if !self.state.config.general.hot_reload {
            self.clear_file_watcher();
            return;
        }

        let Some(path) = self.state.current_file.clone() else {
            self.clear_file_watcher();
            return;
        };

        if self.watched_file.as_ref() == Some(&path) {
            // Already watching this file (or previous watch attempt for this file failed).
            return;
        }

        self.watcher = setup_file_watcher(&mut self.state, ctx.clone(), path.clone());
        self.watched_file = Some(path);
        if self.watcher.is_none() {
            self.state.set_status("Hot reload unavailable for this file");
        }
    }

    /// Track a rough text selection range by mapping pointer drag positions to document offsets.
    fn update_text_selection_from_pointer(&mut self, ctx: &egui::Context) {
        let (pressed, down, released, pos) = ctx.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_down(),
                i.pointer.primary_released(),
                i.pointer.interact_pos(),
            )
        });

        if pressed {
            self.selection_drag_anchor = pos
                .and_then(|p| self.renderer.hit_test_char_offset(p))
                .and_then(|offset| self.floor_content_boundary(offset));
            self.state.text_selection = None;
            return;
        }

        if down {
            if let (Some(anchor), Some(pointer_pos)) = (self.selection_drag_anchor, pos) {
                if let Some(current) = self.renderer.hit_test_char_offset(pointer_pos) {
                    self.state.text_selection = self.normalize_selection_range(anchor, current);
                }
            }
        }

        if released {
            self.selection_drag_anchor = None;
        }
    }

    fn apply_zoom_delta(&mut self, ctx: &egui::Context, delta: f32) -> bool {
        let old_size = self.state.config.theme.fonts.size;
        let new_size = (old_size + delta).clamp(8.0, 32.0);

        if (new_size - old_size).abs() < f32::EPSILON {
            return false;
        }

        self.state.config.theme.fonts.size = new_size;
        let style = create_style(self.state.current_theme(), &self.state.config);
        ctx.set_style(style);
        self.state
            .set_status(format!("Zoom: {}px", self.state.config.theme.fonts.size as i32));
        self.save_config_debounced(ctx);
        true
    }

    fn reset_zoom(&mut self, ctx: &egui::Context) {
        self.state.config.theme.fonts.size = 14.0;
        let style = create_style(self.state.current_theme(), &self.state.config);
        ctx.set_style(style);
        self.state.set_status("Zoom: Reset to default");
        self.save_config_debounced(ctx);
    }

    fn handle_ctrl_scroll_zoom(&mut self, ctx: &egui::Context) {
        let scroll_delta = ctx.input(|i| {
            if i.modifiers.command {
                i.raw_scroll_delta.y
            } else {
                0.0
            }
        });

        if scroll_delta.abs() < f32::EPSILON {
            return;
        }

        let direction = if scroll_delta > 0.0 { 1.0 } else { -1.0 };
        let steps = (scroll_delta.abs() / 40.0).ceil().max(1.0) as usize;
        for _ in 0..steps {
            if !self.apply_zoom_delta(ctx, direction) {
                break;
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Use cached keybindings for efficiency (avoids parsing strings 60 times/second)
        let kb = &self.cached_keybindings;

        // Batch all keyboard input checks into a single ctx.input() call for efficiency
        let (toggle_toc, export_pdf, reload, open_file, open_folder, toggle_file_browser, quit, add_annotation, add_bookmark, focus_toc_search, zoom_in, zoom_out, zoom_reset, escape) = ctx.input(|i| {
            let check = |parsed: &Option<ParsedKeybinding>| -> bool {
                parsed.as_ref().is_some_and(|p| {
                    i.key_pressed(p.key) &&
                    i.modifiers.command == p.modifiers.command &&
                    i.modifiers.alt == p.modifiers.alt &&
                    i.modifiers.shift == p.modifiers.shift
                })
            };
            (
                check(&kb.toggle_toc),
                check(&kb.export_pdf),
                check(&kb.reload),
                check(&kb.open_file),
                check(&kb.open_folder),
                check(&kb.toggle_file_browser),
                check(&kb.quit),
                check(&kb.add_annotation),
                check(&kb.add_bookmark),
                check(&kb.focus_toc_search),
                check(&kb.zoom_in),
                check(&kb.zoom_out),
                check(&kb.zoom_reset),
                i.key_pressed(Key::Escape),
            )
        });

        if toggle_toc {
            self.state.toggle_toc();
            let visible = self.state.toc_visible();
            self.state.set_status(if visible { "Contents shown" } else { "Contents hidden" });
        }

        if export_pdf {
            if self.state.file_deleted {
                self.state.set_status("Cannot export: file was deleted");
            } else {
                self.export_pdf();
            }
        }

        if reload {
            if self.state.file_deleted {
                self.state.set_status("Cannot reload: the file was deleted");
            } else if let Err(e) = self.state.reload_file() {
                self.state.set_status(friendly_errors::format_reload_error(&e));
            } else {
                self.state.set_status("File reloaded");
            }
        }

        if open_file {
            self.open_file_dialog();
        }

        if open_folder {
            self.open_folder_dialog();
        }

        if toggle_file_browser {
            if self.state.folder_state.is_open() {
                self.file_browser_visible = !self.file_browser_visible;
                self.state.set_status(if self.file_browser_visible { "File browser shown" } else { "File browser hidden" });
            } else {
                self.open_folder_dialog();
            }
        }

        if quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if add_annotation && self.state.current_file.is_some() && !self.state.file_deleted {
            let selection = self
                .state
                .text_selection
                .and_then(|(start, end)| self.normalize_selection_range(start, end))
                .or_else(|| {
                    ctx.input(|i| i.pointer.hover_pos()).and_then(|pos| {
                        self.renderer
                            .hit_test_char_offset(pos)
                            .and_then(|offset| {
                                self.normalize_selection_range(offset, offset.saturating_add(1))
                            })
                    })
                });

            if let Some((start, end)) = selection {
                let popup_pos = ctx
                    .input(|i| i.pointer.interact_pos())
                    .unwrap_or_else(|| ctx.screen_rect().center());
                self.annotation_popup.show(popup_pos, (start, end));
            } else {
                self.state
                    .set_status("Select text first, then add an annotation");
            }
        }

        if add_bookmark && self.state.current_file.is_some() && !self.state.file_deleted {
            // Add bookmark at current cursor position when available.
            let char_offset = ctx
                .input(|i| i.pointer.hover_pos())
                .and_then(|pos| self.renderer.hit_test_char_offset(pos))
                .or_else(|| Some(self.estimate_char_offset_from_scroll()))
                .and_then(|offset| self.normalize_bookmark_offset(offset))
                .unwrap_or(0);
            self.handle_annotation_action(AnnotationAction::CreateBookmark(char_offset));
        }

        if focus_toc_search {
            // Ensure TOC is visible and focus the search field
            if !self.state.toc_visible() {
                self.state.set_toc_visible(true);
            }
            self.toc_panel.focus_search();
        }

        if zoom_in {
            self.apply_zoom_delta(ctx, 1.0);
        }

        if zoom_out {
            self.apply_zoom_delta(ctx, -1.0);
        }

        if zoom_reset {
            self.reset_zoom(ctx);
        }

        if escape {
            self.state.creating_annotation = false;
            self.state.text_selection = None;
            self.selection_drag_anchor = None;
            self.annotation_popup.hide();
            self.toc_panel.clear_search();
        }
    }

    fn handle_file_events(&mut self) {
        let events: Vec<_> = self
            .state
            .file_event_rx
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();

        for event in events {
            match event {
                FileEvent::Modified(path) => {
                    if self.state.current_file.as_ref() != Some(&path) {
                        continue;
                    }
                    if self.state.file_deleted {
                        // File was recreated after deletion
                        self.state.file_deleted = false;
                    }
                    if let Err(e) = self.state.reload_file() {
                        self.state.set_status(friendly_errors::format_reload_error(&e));
                    } else {
                        self.state.set_status("File updated");
                    }
                }
                FileEvent::Removed(path) => {
                    if self.state.current_file.as_ref() != Some(&path) {
                        continue;
                    }
                    // Mark file as deleted and show persistent warning
                    self.state.file_deleted = true;
                    self.state.set_status("The file was deleted or moved");
                }
                FileEvent::Error(e) => {
                    self.state.set_status(format!("File watcher error: {}", e));
                }
            }
        }
    }

    fn export_pdf(&mut self) {
        if let Some(file_path) = &self.state.current_file {
            let pdf_path = file_path.with_extension("pdf");
            let events: Vec<_> = crate::markdown::parser::parse_with_config(&self.state.content, &self.state.config).collect();

            // Get the base path for resolving relative image paths
            let base_path = file_path.parent();

            match crate::export::pdf::export_to_pdf_with_base(&events, &pdf_path, &self.state.config, base_path) {
                Ok(()) => {
                    self.state.set_status(format!("PDF saved to {}", pdf_path.display()));
                }
                Err(e) => {
                    self.state.set_status(friendly_errors::format_export_error(&e));
                }
            }
        } else {
            self.state.set_status("Open a file first to export as PDF");
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd_open_file() {
            self.load_markdown_file(path);
        }
    }

    /// Estimate character offset in document based on current scroll position.
    /// Uses content length and scroll offset to calculate an approximate position.
    fn estimate_char_offset_from_scroll(&self) -> usize {
        let content_len = self.state.content.len();
        if content_len == 0 {
            return 0;
        }

        // Use tracked visible range if available (more accurate)
        if let Some((start, _end)) = self.state.visible_char_range {
            return start.min(content_len.saturating_sub(1));
        }

        // Fallback: estimate based on scroll position ratio
        let scroll = self.state.scroll_offset.max(0.0);

        // Estimated total scroll height based on content
        // Assume ~20 pixels per line, ~80 chars per line
        let estimated_total_height = (content_len as f32) * 0.25;

        if estimated_total_height <= 0.0 {
            return 0;
        }

        let ratio = (scroll / estimated_total_height).clamp(0.0, 1.0);
        let char_offset = (ratio * content_len as f32) as usize;

        char_offset.min(content_len.saturating_sub(1))
    }

    fn handle_annotation_action(&mut self, action: AnnotationAction) {
        use crate::annotations::model::Annotation;

        // Helper to save annotations with status feedback
        let save_with_feedback = |state: &mut AppState| {
            if state.config.annotations.auto_save {
                if let Err(e) = state.save_annotations() {
                    log::error!("Failed to auto-save annotations: {}", e);
                    state.set_status("Could not save annotations. Check file permissions.");
                }
            }
        };

        match action {
            AnnotationAction::CreateHighlight(start, end, color) => {
                let annotation = Annotation::highlight(start, end, &color);
                self.state.annotations.add(annotation);
                self.state.set_status("Highlight added");

                save_with_feedback(&mut self.state);

                // Call plugin hook
                #[cfg(feature = "plugins")]
                self.state.call_plugin_hook(crate::plugin::api::PluginHook::OnAnnotationAdd);
            }
            AnnotationAction::CreateNote(start, end, text) => {
                let annotation = Annotation::note(start, end, &text);
                self.state.annotations.add(annotation);
                self.state.set_status("Note added");

                save_with_feedback(&mut self.state);

                #[cfg(feature = "plugins")]
                self.state.call_plugin_hook(crate::plugin::api::PluginHook::OnAnnotationAdd);
            }
            AnnotationAction::CreateBookmark(position) => {
                let annotation = Annotation::bookmark(position);
                self.state.annotations.add(annotation);
                self.state.set_status("Bookmark added");

                save_with_feedback(&mut self.state);

                #[cfg(feature = "plugins")]
                self.state.call_plugin_hook(crate::plugin::api::PluginHook::OnAnnotationAdd);
            }
            AnnotationAction::Delete(id) => {
                self.state.annotations.remove(&id);
                self.state.set_status("Annotation deleted");

                save_with_feedback(&mut self.state);

                #[cfg(feature = "plugins")]
                self.state.call_plugin_hook(crate::plugin::api::PluginHook::OnAnnotationRemove);
            }
            AnnotationAction::UpdateNote(id, text) => {
                if let Some(ann) = self.state.annotations.get_mut(&id) {
                    ann.set_note(&text);
                }

                save_with_feedback(&mut self.state);
            }
            AnnotationAction::UpdateColor(id, color) => {
                if let Some(ann) = self.state.annotations.get_mut(&id) {
                    ann.set_color(&color);
                }

                save_with_feedback(&mut self.state);
            }
        }
    }

    /// Load a markdown file with fade transition
    fn load_markdown_file(&mut self, path: PathBuf) {
        // If content is already displayed, fade out first then load
        if !self.state.content.is_empty() {
            self.pending_file_load = Some(path);
            self.state.start_file_fade_out();
        } else {
            // No content to fade out — load directly with fade-in
            self.perform_file_load(path);
            self.state.file_transition_opacity = 0.0;
            self.state.start_file_fade_in();
        }
    }

    /// Actually load a file (called after fade-out completes or for first load)
    fn perform_file_load(&mut self, path: PathBuf) {
        let base_path = path.parent().map(|p| p.to_path_buf());
        self.renderer.set_base_path(base_path);
        self.renderer.clear_image_cache();

        if let Err(e) = self.state.load_file(path) {
            self.state.set_status(format_load_error(&e));
        }

        self.sync_native_menu_state();
    }

    /// Save config to disk (for persisting changes like zoom level)
    fn save_config_debounced(&mut self, ctx: &egui::Context) {
        self.config_save_requested_at = Some(Instant::now());
        ctx.request_repaint_after(Duration::from_millis(CONFIG_SAVE_DEBOUNCE_MS));
    }

    fn flush_config_save_if_due(&mut self) {
        let Some(requested_at) = self.config_save_requested_at else {
            return;
        };

        if requested_at.elapsed() >= Duration::from_millis(CONFIG_SAVE_DEBOUNCE_MS) {
            self.save_config_now();
            self.config_save_requested_at = None;
        }
    }

    fn save_config_now(&self) {
        if let Some(config_path) = crate::config::loader::get_default_config_path() {
            if let Err(e) = crate::config::loader::save_config(&self.state.config, &config_path) {
                log::warn!("Failed to save config: {}", e);
            }
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        let recent_files = self.state.get_cached_recent_files();

        let mut actions = MenuActions::default();
        let current_theme = self.state.current_theme().to_string();
        let folder_is_open = self.state.folder_state.is_open();
        let toc_visible = self.state.toc_visible();
        let file_browser_visible = self.file_browser_visible;
        let current_file_name = self.state.current_file
            .as_ref()
            .and_then(|f| f.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string());

        let is_dark = ctx.style().visuals.dark_mode;
        let menu_bg = if is_dark { palette::BG_DARK } else { palette::light::BG_SIDEBAR };

        egui::TopBottomPanel::top("menu_bar")
            .frame(egui::Frame::none()
                .fill(menu_bg)
                .inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    Self::render_file_menu(ui, recent_files.as_slice(), &mut actions);
                    Self::render_view_menu(ui, &current_theme, folder_is_open, toc_visible, file_browser_visible, self.state.config.layout.content_width, &mut actions);

                    // Plugins menu (only if plugins feature enabled and items registered)
                    #[cfg(feature = "plugins")]
                    {
                        let plugin_items = self.state.plugin_runtime.as_ref()
                            .and_then(|rt| rt.state.lock().ok())
                            .map(|state| state.menu_items.clone())
                            .unwrap_or_default();

                        if !plugin_items.is_empty() {
                            Self::render_plugins_menu(ui, &plugin_items, &mut actions);
                        }
                    }

                    Self::render_help_menu(ui, &mut actions);
                    Self::render_menu_file_name(ui, current_file_name.as_deref());
                });
            });

        self.handle_menu_actions(ctx, actions);
    }

    fn render_file_menu(
        ui: &mut egui::Ui,
        recent_files: &[(PathBuf, String, String)],
        actions: &mut MenuActions,
    ) {
        ui.menu_button("File", |ui| {
            let open_file_label = format!("Open File...      {}", shortcuts::format("O"));
            if ui.button(&open_file_label).clicked() {
                actions.open_dialog = true;
                ui.close_menu();
            }

            let open_folder_label = format!("Open Folder...    {}", shortcuts::format_shift("O"));
            if ui.button(&open_folder_label).clicked() {
                actions.open_folder_dialog = true;
                ui.close_menu();
            }

            ui.menu_button("Recent Files", |ui| {
                if recent_files.is_empty() {
                    ui.label(egui::RichText::new("No recent files").italics().weak());
                } else {
                    for (path, name, _dir) in recent_files {
                        if ui.button(name).on_hover_text(path.display().to_string()).clicked() {
                            actions.file_to_open = Some(path.clone());
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Clear Recent").clicked() {
                        actions.clear_recent = true;
                        ui.close_menu();
                    }
                }
            });

            ui.separator();

            let reload_label = format!("Reload            {}", shortcuts::key_only("F5"));
            if ui.button(&reload_label).clicked() {
                actions.reload = true;
                ui.close_menu();
            }

            ui.separator();

            let export_label = format!("Export PDF        {}", shortcuts::format("P"));
            if ui.button(&export_label).clicked() {
                actions.export_pdf = true;
                ui.close_menu();
            }

            ui.separator();

            if ui.button("Edit Config...").clicked() {
                actions.edit_config = true;
                ui.close_menu();
            }

            ui.separator();

            let quit_label = format!("Quit              {}", shortcuts::format("Q"));
            if ui.button(&quit_label).clicked() {
                actions.quit = true;
            }
        });
    }

    fn render_view_menu(
        ui: &mut egui::Ui,
        current_theme: &str,
        folder_is_open: bool,
        toc_visible: bool,
        file_browser_visible: bool,
        content_width: Option<f32>,
        actions: &mut MenuActions,
    ) {
        ui.menu_button("View", |ui| {
            let toc_shortcut = shortcuts::format("T");
            let toc_label = if toc_visible {
                format!("Hide Contents     {}", toc_shortcut)
            } else {
                format!("Show Contents     {}", toc_shortcut)
            };
            if ui.button(&toc_label).clicked() {
                actions.toggle_toc = true;
                ui.close_menu();
            }

            if folder_is_open {
                let fb_shortcut = shortcuts::format("E");
                let fb_label = if file_browser_visible {
                    format!("Hide File Browser {}", fb_shortcut)
                } else {
                    format!("Show File Browser {}", fb_shortcut)
                };
                if ui.button(&fb_label).clicked() {
                    actions.toggle_file_browser = true;
                    ui.close_menu();
                }
            }

            ui.separator();

            ui.menu_button("Reading Width", |ui| {
                let is_full = content_width.is_none();
                let is_comfortable = content_width == Some(720.0);
                let is_narrow = content_width == Some(560.0);

                if ui.selectable_label(is_full, "Full Width").clicked() {
                    actions.new_reading_width = Some(None);
                    ui.close_menu();
                }
                if ui.selectable_label(is_comfortable, "Comfortable (720px)").clicked() {
                    actions.new_reading_width = Some(Some(720.0));
                    ui.close_menu();
                }
                if ui.selectable_label(is_narrow, "Narrow (560px)").clicked() {
                    actions.new_reading_width = Some(Some(560.0));
                    ui.close_menu();
                }
            });

            ui.menu_button("Theme", |ui| {
                let normalized = current_theme.trim().to_lowercase();
                let dark_selected = normalized == "dark";
                let light_selected = normalized == "light";

                if ui.selectable_label(dark_selected, "Dark").clicked() {
                    actions.new_theme = Some("dark".to_string());
                    ui.close_menu();
                }
                if ui.selectable_label(light_selected, "Light").clicked() {
                    actions.new_theme = Some("light".to_string());
                    ui.close_menu();
                }
            });
        });
    }

    fn render_help_menu(ui: &mut egui::Ui, actions: &mut MenuActions) {
        ui.menu_button("Help", |ui| {
            if ui.button("Quick Help").clicked() {
                actions.show_help = true;
                ui.close_menu();
            }

            ui.separator();

            if ui.button("About mdview").clicked() {
                actions.show_about = true;
                ui.close_menu();
            }
        });
    }

    /// Render the Plugins menu with dynamically registered items
    #[cfg(feature = "plugins")]
    fn render_plugins_menu(
        ui: &mut egui::Ui,
        items: &[crate::plugin::api::PluginMenuItem],
        actions: &mut MenuActions,
    ) {
        ui.menu_button("Plugins", |ui| {
            for item in items {
                if ui.button(&item.label).clicked() {
                    actions.plugin_menu_callback = Some(item.callback.clone());
                    ui.close_menu();
                }
            }
        });
    }

    fn render_menu_file_name(ui: &mut egui::Ui, file_name: Option<&str>) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(name) = file_name {
                ui.label(
                    egui::RichText::new(name)
                        .color(palette::TEXT_MUTED)
                        .small()
                );
            }
        });
    }

    fn handle_menu_actions(&mut self, ctx: &egui::Context, actions: MenuActions) {
        if actions.open_dialog {
            self.open_file_dialog();
        }
        if actions.open_folder_dialog {
            self.open_folder_dialog();
        }
        if actions.reload {
            if self.state.file_deleted {
                self.state.set_status("Cannot reload: the file was deleted");
            } else if let Err(e) = self.state.reload_file() {
                self.state.set_status(friendly_errors::format_reload_error(&e));
            } else {
                self.state.set_status("File reloaded");
            }
        }
        if actions.export_pdf {
            if self.state.file_deleted {
                self.state.set_status("Cannot export: file was deleted");
            } else {
                self.export_pdf();
            }
        }
        if actions.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if actions.show_help {
            self.show_help_dialog = true;
        }
        if actions.show_about {
            self.show_about_dialog = true;
        }
        if actions.toggle_toc {
            self.state.toggle_toc();
        }
        if actions.toggle_file_browser {
            self.file_browser_visible = !self.file_browser_visible;
        }
        if actions.clear_recent {
            self.state.recent_files.clear();
            self.state.invalidate_recent_files_cache();
            let _ = crate::recent::save_recent_files(&self.state.recent_files);
        }
        if let Some(path) = actions.file_to_open {
            self.load_markdown_file(path);
        }
        if let Some(theme) = actions.new_theme {
            self.state.switch_theme(&theme);
            let style = create_style(self.state.current_theme(), &self.state.config);
            ctx.set_style(style);
            self.state
                .set_status(format!("Switched to {} theme", self.state.current_theme()));
            self.save_config_debounced(ctx);
        }
        if let Some(width) = actions.new_reading_width {
            self.state.config.layout.content_width = width;
            let width_desc = match width {
                None => "Full Width".to_string(),
                Some(w) => format!("{}px", w as i32),
            };
            self.state.set_status(format!("Reading width: {}", width_desc));
            self.save_config_debounced(ctx);
        }
        if actions.edit_config {
            self.handle_edit_config();
        }

        // Handle plugin menu callbacks
        #[cfg(feature = "plugins")]
        if let Some(callback_name) = actions.plugin_menu_callback {
            self.invoke_plugin_callback(&callback_name);
        }
    }

    /// Invoke a Lua callback function registered by a plugin
    #[cfg(feature = "plugins")]
    fn invoke_plugin_callback(&mut self, callback_name: &str) {
        if let Some(ref runtime) = self.state.plugin_runtime {
            // Call the callback function in Lua globals
            if let Err(e) = runtime.lua().load(format!("{}()", callback_name)).exec() {
                log::error!("[plugin] Failed to invoke callback '{}': {}", callback_name, e);
                self.state.set_status(format!("Plugin error: {}", e));
            }
        }
    }

    fn handle_edit_config(&mut self) {
        match crate::config::loader::create_default_config() {
            Ok(config_path) => {
                if let Err(e) = open::that(&config_path) {
                    log::error!("Failed to open config file: {}", e);
                    self.state.set_status(friendly_errors::format_config_error("open", &e));
                } else {
                    self.state.set_status(format!("Config file opened: {}", config_path.display()));
                }
            }
            Err(e) => {
                log::error!("Failed to create config file: {}", e);
                self.state.set_status(friendly_errors::format_config_error("create", &e));
            }
        }
    }

    fn render_status_bar(&mut self, ctx: &egui::Context) {
        self.state.clear_expired_status();

        let is_dark = ctx.style().visuals.dark_mode;
        let status_bg = if is_dark { palette::BG_DARK } else { palette::light::BG_SIDEBAR };
        let border_color = if is_dark { palette::BORDER_SUBTLE } else { palette::light::BORDER_SUBTLE };
        let text_muted = if is_dark { palette::TEXT_MUTED } else { palette::light::TEXT_MUTED };
        let accent = if is_dark { palette::ACCENT } else { palette::light::ACCENT };

        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none()
                .fill(status_bg)
                .inner_margin(egui::Margin::symmetric(16.0, 6.0))
                .stroke(Stroke::new(1.0, border_color)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some((msg, _)) = &self.state.status_message {
                        ui.label(
                            egui::RichText::new(msg)
                                .color(accent)
                                .small()
                        );
                    } else if let Some(file) = &self.state.current_file {
                        ui.label(
                            egui::RichText::new(file.display().to_string())
                                .color(text_muted)
                                .small()
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("No file open")
                                .color(if is_dark { palette::TEXT_DISABLED } else { palette::light::TEXT_MUTED })
                                .small()
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.config.general.hot_reload && self.watcher.is_some() {
                            // Watching indicator with subtle dot
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("●")
                                        .color(palette::SUCCESS)
                                        .small()
                                );
                                ui.label(
                                    egui::RichText::new("Watching")
                                        .color(palette::TEXT_MUTED)
                                        .small()
                                );
                            });
                        }
                    });
                });
            });
    }

    fn render_toc_sidebar(&mut self, ctx: &egui::Context) {
        // Update animation and request repaint if still animating
        let animating = self.state.update_toc_animation();
        if animating {
            ctx.request_repaint();
        }

        let progress = self.state.toc_animation_eased();

        // Don't render panel at all when fully closed
        if progress < 0.001 {
            return;
        }

        let is_dark = ctx.style().visuals.dark_mode;
        let panel_bg = if is_dark { palette::BG_DARK } else { palette::light::BG_SIDEBAR };
        let border_color = if is_dark { palette::BORDER_SUBTLE } else { palette::light::BORDER_SUBTLE };
        let text_muted = if is_dark { palette::TEXT_MUTED } else { palette::light::TEXT_MUTED };

        // Animate width: from 0 to full toc_width
        let target_width = self.state.toc_width;
        let animated_width = target_width * progress;

        // During animation, use exact width (not resizable); when fully open, allow resizing
        let fully_open = progress > 0.999;

        let mut panel = egui::SidePanel::left("toc_panel")
            .resizable(fully_open)
            .default_width(animated_width);

        if fully_open {
            panel = panel.width_range(180.0..=400.0);
        } else {
            // Lock to animated width during transition
            panel = panel.width_range(animated_width..=animated_width);
        }

        panel
            .frame(egui::Frame::none()
                .fill(panel_bg)
                .inner_margin(egui::Margin::same(0.0))
                .stroke(Stroke::new(1.0, border_color)))
            .show(ctx, |ui| {
                // Clip contents during animation to prevent overflow
                ui.set_clip_rect(ui.available_rect_before_wrap());

                // Header
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("CONTENTS")
                            .color(text_muted)
                            .small()
                            .strong()
                    );
                });
                ui.add_space(8.0);

                // TOC entries (only interactive when fully open)
                if fully_open {
                    if let Some(scroll_to) =
                        self.toc_panel
                            .render(ui, &self.state.toc, self.state.current_heading_idx, is_dark)
                    {
                        // Set the target heading to scroll to
                        self.state.scroll_to_heading = Some(scroll_to);
                    }
                } else {
                    // During animation, render entries non-interactively for visual continuity
                    ui.disable();
                    self.toc_panel
                        .render(ui, &self.state.toc, self.state.current_heading_idx, is_dark);
                }
            });
    }

    fn render_file_browser_sidebar(&mut self, ctx: &egui::Context) {
        if !self.file_browser_visible || !self.state.folder_state.is_open() {
            return;
        }

        let is_dark = ctx.style().visuals.dark_mode;
        let panel_bg = if is_dark { palette::BG_DARK } else { palette::light::BG_SIDEBAR };
        let border_color = if is_dark { palette::BORDER_SUBTLE } else { palette::light::BORDER_SUBTLE };
        let text_muted = if is_dark { palette::TEXT_MUTED } else { palette::light::TEXT_MUTED };

        let mut file_to_open: Option<PathBuf> = None;

        egui::SidePanel::right("file_browser_panel")
            .resizable(true)
            .default_width(250.0)
            .width_range(180.0..=400.0)
            .frame(egui::Frame::none()
                .fill(panel_bg)
                .inner_margin(egui::Margin::same(8.0))
                .stroke(Stroke::new(1.0, border_color)))
            .show(ctx, |ui| {
                // Header
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("FILES")
                            .color(text_muted)
                            .small()
                            .strong()
                    );
                });
                ui.add_space(8.0);

                // File browser
                let current_file = self.state.current_file.as_deref();
                if let Some(path) = self.file_browser.render(ui, &mut self.state.folder_state, current_file) {
                    file_to_open = Some(path);
                }
            });

        // Handle file selection outside the closure
        if let Some(path) = file_to_open {
            self.load_markdown_file(path);
        }
    }

    fn render_main_content(&mut self, ctx: &egui::Context) {
        let is_dark = ctx.style().visuals.dark_mode;
        let main_bg = if is_dark { palette::BG_BASE } else { palette::light::BG_BASE };

        let mut file_to_open: Option<PathBuf> = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(main_bg)
                .inner_margin(egui::Margin::same(0.0)))
            .show(ctx, |ui| {
                // Show drag-drop overlay if dragging
                if self.is_dragging_file {
                    render_drag_drop_overlay(ui, is_dark);
                }

                // Show loading indicator
                if self.state.is_loading {
                    render_loading_indicator(ui, is_dark);
                    return;
                }

                // Apply fade transition opacity
                let opacity = self.state.file_transition_opacity;
                if opacity < 1.0 {
                    ui.set_opacity(opacity);
                }

                // Show deleted file warning banner
                if self.state.file_deleted {
                    render_deleted_file_banner(ui, is_dark);
                }

                if self.state.content.is_empty() {
                    // Refined welcome screen
                    let recent_files = self.state.get_cached_recent_files();
                    render_welcome_screen(ui, recent_files.as_slice(), &mut file_to_open, is_dark);
                    return;
                }

                // Get cached events (avoids parsing every frame)
                let events = self.state.get_cached_events();

                // Get references instead of cloning
                let annotations = &self.state.annotations;
                let config = &self.state.config;

                // Call pre-render hook
                #[cfg(feature = "plugins")]
                self.state.call_plugin_hook(crate::plugin::api::PluginHook::OnPreRender);

                // Reuse heading_positions Vec instead of allocating new one per frame
                let mut heading_positions = std::mem::take(&mut self.state.heading_positions);
                heading_positions.clear();
                let scroll_target = self.state.scroll_to_heading.take();

                let scroll_output = egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(32.0);
                        ui.horizontal(|ui| {
                            ui.add_space(self.state.config.layout.content_margin);
                            ui.vertical(|ui| {
                                if let Some(width) = self.state.config.layout.content_width {
                                    ui.set_max_width(width);
                                }
                                self.renderer.render_with_scroll_target(
                                    ui,
                                    events.as_slice(),
                                    annotations,
                                    &mut heading_positions,
                                    config,
                                    scroll_target,
                                );
                            });
                        });
                        ui.add_space(64.0);
                    });

                let scroll_offset = scroll_output.state.offset.y;
                let viewport_top = scroll_output.inner_rect.top();
                self.state.heading_positions = heading_positions;
                self.state.scroll_offset = scroll_offset;

                // Track visible character range for annotation positioning
                // The renderer's char_offset represents the end of rendered content
                let total_chars = self.renderer.current_char_offset();
                if total_chars > 0 {
                    // Estimate visible range based on scroll position ratio
                    let content_len = self.state.content.len();
                    if content_len > 0 {
                        let max_scroll = (scroll_output.content_size.y - scroll_output.inner_rect.height())
                            .max(1.0);
                        let ratio = (scroll_offset / max_scroll).clamp(0.0, 1.0);
                        let start = (ratio * content_len as f32) as usize;
                        let visible_fraction = if scroll_output.content_size.y > 0.0 {
                            (scroll_output.inner_rect.height() / scroll_output.content_size.y)
                                .clamp(0.0, 1.0)
                        } else {
                            0.1
                        };
                        let visible_chars = ((content_len as f32) * visible_fraction).ceil() as usize;
                        let end = (start + visible_chars.max(1)).min(content_len);
                        self.state.visible_char_range = Some((start, end));
                    }
                } else {
                    self.state.visible_char_range = None;
                }

                // Use binary search to find current heading (O(log n) instead of O(n))
                // Find the last heading position that is <= viewport_top + 50.0
                let target = viewport_top + 50.0;
                let current_idx = if self.state.heading_positions.is_empty() {
                    None
                } else {
                    // partition_point returns the index where all elements before satisfy the predicate
                    let idx = self.state.heading_positions.partition_point(|&pos| pos <= target);
                    if idx > 0 { Some(idx - 1) } else { None }
                };
                self.state.current_heading_idx = current_idx;

                // Call post-render hook
                #[cfg(feature = "plugins")]
                self.state.call_plugin_hook(crate::plugin::api::PluginHook::OnPostRender);
            });

        if let Some(path) = file_to_open {
            self.load_markdown_file(path);
        }
    }
}

/// Render a loading indicator (spinner)
fn render_loading_indicator(ui: &mut egui::Ui, is_dark: bool) {
    let text_color = if is_dark { palette::TEXT_PRIMARY } else { palette::light::TEXT_PRIMARY };
    let accent_color = if is_dark {
        egui::Color32::from_rgb(78, 201, 176)
    } else {
        egui::Color32::from_rgb(0, 120, 150)
    };

    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 3.0);

            // Animated spinner
            ui.spinner();

            ui.add_space(16.0);

            ui.label(
                egui::RichText::new("Loading...")
                    .size(18.0)
                    .color(text_color),
            );

            ui.add_space(8.0);

            ui.label(
                egui::RichText::new("Opening file")
                    .size(13.0)
                    .color(accent_color),
            );
        });
    });

    // Request continuous repainting for spinner animation
    ui.ctx().request_repaint();
}

/// Render the drag-and-drop overlay
fn render_drag_drop_overlay(ui: &mut egui::Ui, is_dark: bool) {
    let overlay_color = if is_dark {
        egui::Color32::from_rgba_unmultiplied(78, 201, 176, 40)
    } else {
        egui::Color32::from_rgba_unmultiplied(0, 120, 150, 40)
    };
    let border_color = if is_dark {
        egui::Color32::from_rgb(78, 201, 176)
    } else {
        egui::Color32::from_rgb(0, 120, 150)
    };
    let text_color = if is_dark { palette::TEXT_PRIMARY } else { palette::light::TEXT_PRIMARY };

    let rect = ui.available_rect_before_wrap();

    // Semi-transparent overlay
    ui.painter().rect_filled(rect, 0.0, overlay_color);

    // Dashed border effect (using multiple lines)
    let stroke = Stroke::new(3.0, border_color);
    ui.painter().rect_stroke(rect.shrink(20.0), Rounding::same(12.0), stroke);

    // Center text
    let center = rect.center();
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "Drop file or folder here",
        egui::FontId::proportional(24.0),
        text_color,
    );
}

/// Render the deleted file warning banner
fn render_deleted_file_banner(ui: &mut egui::Ui, is_dark: bool) {
    let bg_color = if is_dark {
        egui::Color32::from_rgb(80, 40, 40)
    } else {
        egui::Color32::from_rgb(255, 240, 240)
    };
    let text_color = if is_dark {
        egui::Color32::from_rgb(255, 180, 180)
    } else {
        egui::Color32::from_rgb(180, 60, 60)
    };

    egui::Frame::none()
        .fill(bg_color)
        .inner_margin(egui::Margin::symmetric(16.0, 8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("⚠")
                        .size(16.0)
                        .color(text_color)
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("This file was deleted externally. Export and reload are disabled.")
                        .color(text_color)
                );
            });
        });
}

/// Render the refined welcome screen
fn render_welcome_screen(
    ui: &mut egui::Ui,
    recent_files: &[(PathBuf, String, String)],
    file_to_open: &mut Option<PathBuf>,
    is_dark: bool,
) {
    let available_size = ui.available_size();

    // Theme-aware colors
    let text_primary = if is_dark { palette::TEXT_PRIMARY } else { palette::light::TEXT_PRIMARY };
    let text_muted = if is_dark { palette::TEXT_MUTED } else { palette::light::TEXT_MUTED };
    let bg_elevated = if is_dark { palette::BG_ELEVATED } else { palette::light::BG_ELEVATED };
    let border_subtle = if is_dark { palette::BORDER_SUBTLE } else { palette::light::BORDER_SUBTLE };

    ui.vertical_centered(|ui| {
        ui.add_space(available_size.y * 0.15);

        // Logo/Title area with styled icon
        ui.horizontal(|ui| {
            // Center the logo
            let logo_width = 200.0;
            ui.add_space((ui.available_width() - logo_width) / 2.0);

            // Render the styled logo icon
            render_logo_icon(ui);

            ui.add_space(12.0);

            ui.vertical(|ui| {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("mdview")
                        .size(42.0)
                        .color(text_primary)
                        .strong()
                );
            });
        });

        ui.add_space(8.0);

        ui.label(
            egui::RichText::new("A modern markdown viewer")
                .size(16.0)
                .color(text_muted)
        );

        ui.add_space(40.0);

        // Action hints
        ui.horizontal(|ui| {
            ui.add_space((available_size.x - 300.0) / 2.0);
            ui.vertical(|ui| {
                let open_shortcut = shortcuts::format("O");
                render_action_hint(ui, &open_shortcut, "Open file", is_dark);
                ui.add_space(8.0);
                render_action_hint(ui, "drag", "Drop a file here", is_dark);
            });
        });

        ui.add_space(48.0);

        // Recent files section
        if !recent_files.is_empty() {
            let card_width = 400.0;

            egui::Frame::none()
                .fill(bg_elevated)
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0, border_subtle))
                .inner_margin(egui::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(card_width);
                    ui.set_max_width(card_width);

                    ui.label(
                        egui::RichText::new("Recent Files")
                            .size(13.0)
                            .color(text_muted)
                            .strong()
                    );

                    ui.add_space(12.0);

                    for (path, name, dir) in recent_files.iter().take(5) {
                        let response = render_recent_file_item(ui, name, dir, is_dark);
                        if response.clicked() {
                            *file_to_open = Some(path.clone());
                        }
                    }
                });
        }
    });
}

/// Render the mdview logo icon
fn render_logo_icon(ui: &mut egui::Ui) {
    let size = 56.0;
    let (rect, _response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());

    let painter = ui.painter();

    // Purple gradient background (using solid color as egui doesn't support gradients easily)
    let bg_color = egui::Color32::from_rgb(139, 92, 246); // #8B5CF6
    painter.rect_filled(rect, Rounding::same(14.0), bg_color);

    // Add subtle highlight at top
    let highlight_rect = egui::Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width(), rect.height() * 0.4),
    );
    painter.rect_filled(
        highlight_rect,
        Rounding::same(14.0),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15),
    );

    let center = rect.center();

    // Geometric M - two vertical bars
    let bar_width = 5.0;
    let bar_height = size * 0.44;
    let bar_spacing = size * 0.35;

    // Left bar
    let left_bar = egui::Rect::from_center_size(
        egui::Pos2::new(center.x - bar_spacing / 2.0, center.y),
        Vec2::new(bar_width, bar_height),
    );
    painter.rect_filled(left_bar, Rounding::same(2.0), egui::Color32::WHITE);

    // Right bar
    let right_bar = egui::Rect::from_center_size(
        egui::Pos2::new(center.x + bar_spacing / 2.0, center.y),
        Vec2::new(bar_width, bar_height),
    );
    painter.rect_filled(right_bar, Rounding::same(2.0), egui::Color32::WHITE);

    // Center diamond
    let diamond_size = size * 0.18;
    let diamond_points = [
        egui::Pos2::new(center.x, center.y - diamond_size),      // top
        egui::Pos2::new(center.x + diamond_size, center.y),      // right
        egui::Pos2::new(center.x, center.y + diamond_size),      // bottom
        egui::Pos2::new(center.x - diamond_size, center.y),      // left
    ];
    painter.add(egui::Shape::convex_polygon(
        diamond_points.to_vec(),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 230),
        egui::Stroke::NONE,
    ));
}

/// Render an action hint (shortcut + description)
fn render_action_hint(ui: &mut egui::Ui, shortcut: &str, description: &str, is_dark: bool) {
    let bg_elevated = if is_dark { palette::BG_ELEVATED } else { palette::light::BG_ELEVATED };
    let text_secondary = if is_dark { palette::TEXT_SECONDARY } else { palette::light::TEXT_SECONDARY };
    let text_muted = if is_dark { palette::TEXT_MUTED } else { palette::light::TEXT_MUTED };

    ui.horizontal(|ui| {
        egui::Frame::none()
            .fill(bg_elevated)
            .rounding(Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(shortcut)
                        .color(text_secondary)
                        .small()
                        .strong()
                );
            });
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(description)
                .color(text_muted)
        );
    });
}

/// Render a recent file item as a clickable row
fn render_recent_file_item(ui: &mut egui::Ui, name: &str, dir: &str, is_dark: bool) -> egui::Response {
    // Theme-aware colors
    let bg_hover = if is_dark { palette::BG_HOVER } else { palette::light::BG_HOVER };
    let text_primary = if is_dark { palette::TEXT_PRIMARY } else { palette::light::TEXT_PRIMARY };
    let text_secondary = if is_dark { palette::TEXT_SECONDARY } else { palette::light::TEXT_SECONDARY };
    let text_muted = if is_dark { palette::TEXT_MUTED } else { palette::light::TEXT_MUTED };
    let text_disabled = if is_dark { palette::TEXT_DISABLED } else { palette::light::TEXT_DISABLED };

    // Truncate directory path by character count (UTF-8 safe).
    let truncated_dir = {
        let char_count = dir.chars().count();
        if char_count > 50 {
            let tail_start = char_count.saturating_sub(47);
            let tail: String = dir.chars().skip(tail_start).collect();
            format!("...{}", tail)
        } else {
            dir.to_string()
        }
    };

    let row_height = 44.0;
    let available_width = ui.available_width();

    // Allocate space for the clickable row
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(available_width, row_height),
        egui::Sense::click(),
    );

    // Draw hover background
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            Rounding::same(6.0),
            bg_hover,
        );
    }

    // Draw content using the painter
    let icon_pos = rect.min + Vec2::new(8.0, (row_height - 16.0) / 2.0);
    let text_x = rect.min.x + 32.0;

    // File icon (📄 document emoji)
    ui.painter().text(
        icon_pos,
        egui::Align2::LEFT_CENTER,
        "\u{1F4C4}",
        egui::FontId::proportional(14.0),
        if response.hovered() { text_secondary } else { text_muted },
    );

    // File name
    ui.painter().text(
        egui::Pos2::new(text_x, rect.min.y + 14.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(14.0),
        if response.hovered() { text_primary } else { text_secondary },
    );

    // Directory path
    ui.painter().text(
        egui::Pos2::new(text_x, rect.min.y + 32.0),
        egui::Align2::LEFT_CENTER,
        &truncated_dir,
        egui::FontId::proportional(11.0),
        text_disabled,
    );

    response
}

impl eframe::App for MdViewApp {
    #[allow(unused_variables)]
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Initialize native menu for Windows (must be done after window creation)
        #[cfg(windows)]
        self.init_native_menu_windows(frame);

        self.sync_file_watcher(ctx);
        self.handle_file_events();
        self.handle_keyboard_shortcuts(ctx);
        self.handle_ctrl_scroll_zoom(ctx);

        // Poll for completed async mermaid renders and image loads
        self.renderer.poll_mermaid_renders(ctx);
        self.renderer.poll_image_loads(ctx);

        // Handle native menu events
        self.handle_native_menu_events(ctx);

        // Show plugin failure notification (once)
        #[cfg(feature = "plugins")]
        if !self.shown_plugin_notification && self.state.has_failed_plugins() {
            let count = self.state.failed_plugin_count();
            let msg = if count == 1 {
                "A plugin failed to load. Check logs for details.".to_string()
            } else {
                format!("{} plugins failed to load. Check logs for details.", count)
            };
            self.state.set_status(msg);
            self.state.clear_failed_plugins();
            self.shown_plugin_notification = true;
        }

        // Process plugin notifications
        #[cfg(feature = "plugins")]
        self.state.process_plugin_notifications();

        // Process plugin-created annotations
        #[cfg(feature = "plugins")]
        self.state.process_plugin_annotations();

        // Process plugin config changes and apply theme if needed
        #[cfg(feature = "plugins")]
        {
            if self.state.process_plugin_config_changes() {
                let style = create_style(self.state.current_theme(), &self.state.config);
                ctx.set_style(style);
            }
        }
        self.sync_file_watcher(ctx);

        // Handle drag and drop with visual feedback
        let (is_dragging, dropped_file) = ctx.input(|i| {
            let dragging = !i.raw.hovered_files.is_empty();
            let dropped = if !i.raw.dropped_files.is_empty() {
                i.raw.dropped_files[0].path.clone()
            } else {
                None
            };
            (dragging, dropped)
        });

        self.is_dragging_file = is_dragging;

        if let Some(path) = dropped_file {
            self.is_dragging_file = false;
            // Check if it's a directory or file
            if path.is_dir() {
                if let Err(e) = self.state.open_folder(path) {
                    self.state.set_status(friendly_errors::format_folder_error(&e));
                } else {
                    self.file_browser_visible = true;
                    self.state.set_status("Folder opened");
                }
            } else {
                self.load_markdown_file(path);
            }
        }

        // Drive file transition animation
        if self.state.update_file_transition() {
            ctx.request_repaint();
            // Check if fade-out just completed — time to load the pending file
            if self.state.is_fade_out_complete() {
                if let Some(path) = self.pending_file_load.take() {
                    self.perform_file_load(path);
                    self.state.start_file_fade_in();
                    self.sync_file_watcher(ctx);
                }
            }
        }

        // Render UI (skip egui menu bar if native menu is active)
        if self.native_menu.is_none() {
            self.render_menu_bar(ctx);
        }
        self.render_status_bar(ctx);
        self.render_toc_sidebar(ctx);
        self.render_file_browser_sidebar(ctx);
        self.render_main_content(ctx);
        self.update_text_selection_from_pointer(ctx);

        // Render annotation popup if visible
        if self.annotation_popup.visible {
            egui::CentralPanel::default()
                .frame(egui::Frame::none())
                .show(ctx, |ui| {
                    if let Some(action) = self.annotation_popup.render(ui) {
                        self.handle_annotation_action(action);
                    }
                });
        }

        // Render file association dialog if needed
        if self.show_file_association_dialog {
            self.render_file_association_dialog(ctx);
        }

        // Poll for update check results
        if self.update_checker.poll() {
            // Check if this version was dismissed
            if let Some(info) = self.update_checker.update_info() {
                let dismissed = self.state.config.general.dismissed_update_version
                    .as_ref()
                    .map(|v| v == &info.version)
                    .unwrap_or(false);

                if dismissed {
                    self.update_checker.dismiss();
                } else {
                    self.show_update_dialog = true;
                }
            }
        }

        // Render update dialog if needed
        if self.show_update_dialog && self.update_checker.should_show() {
            self.render_update_dialog(ctx);
        }

        // Render quick help dialog if needed
        if self.show_help_dialog {
            self.render_help_dialog(ctx);
        }

        // Render about dialog if needed
        if self.show_about_dialog {
            self.render_about_dialog(ctx);
        }

        // Reconcile watcher state after any file/config changes made this frame.
        self.sync_file_watcher(ctx);

        // Flush any pending config save after UI work is done
        self.flush_config_save_if_due();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Call file close hook
        #[cfg(feature = "plugins")]
        if let Some(ref path) = self.state.current_file {
            self.state.call_plugin_hook_with_path(crate::plugin::api::PluginHook::OnFileClose, path);
        }

        if let Err(e) = self.state.save_annotations() {
            log::error!("Failed to save annotations on exit: {}", e);
            // Note: Can't show UI dialog here as context is being destroyed
            // The error is logged for debugging purposes
        }

        if self.config_save_requested_at.is_some() {
            self.save_config_now();
        }
    }
}

fn setup_fonts(ctx: &egui::Context, config: &Config) {
    let mut fonts = egui::FontDefinitions::default();

    // Try to load custom fonts from config directory
    let font_dir = crate::config::loader::get_config_dir()
        .map(|config_dir| config_dir.join("fonts"));

    // Load body font if specified and not default
    if config.theme.fonts.body != "sans-serif" {
        if let Some(ref dir) = font_dir {
            // Try common font file extensions
            for ext in &["ttf", "otf"] {
                let font_path = dir.join(format!("{}.{}", config.theme.fonts.body, ext));
                if font_path.exists() {
                    if let Ok(font_data) = std::fs::read(&font_path) {
                        fonts.font_data.insert(
                            "custom_body".to_owned(),
                            egui::FontData::from_owned(font_data),
                        );
                        fonts.families.entry(egui::FontFamily::Proportional)
                            .or_default()
                            .insert(0, "custom_body".to_owned());
                        log::info!("Loaded custom body font: {:?}", font_path);
                        break;
                    }
                }
            }
        }
    }

    // Load code font if specified and not default
    if config.theme.fonts.code != "monospace" {
        if let Some(ref dir) = font_dir {
            for ext in &["ttf", "otf"] {
                let font_path = dir.join(format!("{}.{}", config.theme.fonts.code, ext));
                if font_path.exists() {
                    if let Ok(font_data) = std::fs::read(&font_path) {
                        fonts.font_data.insert(
                            "custom_code".to_owned(),
                            egui::FontData::from_owned(font_data),
                        );
                        fonts.families.entry(egui::FontFamily::Monospace)
                            .or_default()
                            .insert(0, "custom_code".to_owned());
                        log::info!("Loaded custom code font: {:?}", font_path);
                        break;
                    }
                }
            }
        }
    }

    // Try to load system symbol and emoji fonts as fallback for Unicode support
    let system_paths = get_system_font_paths();

    // Load symbol font first (for ⌘, ⇧, ▶, ▼, etc.)
    // These are critical for UI elements like keyboard shortcuts and TOC arrows
    let mut symbol_loaded = false;
    for path in system_paths.symbol_fonts {
        // Skip .ttc files - egui can't parse TrueType Collections
        if path.extension().map(|e| e == "ttc").unwrap_or(false) {
            log::debug!("Skipping TrueType Collection: {:?}", path);
            continue;
        }

        if path.exists() {
            if let Ok(font_data) = std::fs::read(&path) {
                fonts.font_data.insert(
                    "symbols".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                // Add symbol font with highest priority for proportional text
                fonts.families.entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "symbols".to_owned());
                log::info!("Loaded symbol font: {:?}", path);
                symbol_loaded = true;
                break;
            } else {
                log::debug!("Failed to read symbol font: {:?}", path);
            }
        }
    }

    if !symbol_loaded {
        log::warn!("No symbol font loaded - some UI symbols may not display correctly");
    }

    // Load emoji font (for 📁, 📄, 📂, etc.)
    // Note: emojis will render monochrome in egui as it doesn't support color fonts
    let mut emoji_loaded = false;
    for path in system_paths.emoji_fonts {
        // Skip .ttc files
        if path.extension().map(|e| e == "ttc").unwrap_or(false) {
            log::debug!("Skipping TrueType Collection: {:?}", path);
            continue;
        }

        if path.exists() {
            if let Ok(font_data) = std::fs::read(&path) {
                fonts.font_data.insert(
                    "emoji".to_owned(),
                    egui::FontData::from_owned(font_data),
                );
                // Add emoji font with high priority for proportional text
                fonts.families.entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "emoji".to_owned());
                log::info!("Loaded emoji font: {:?}", path);
                emoji_loaded = true;
                break;
            } else {
                log::debug!("Failed to read emoji font: {:?}", path);
            }
        }
    }

    if !emoji_loaded {
        log::debug!("No emoji font loaded - file browser icons may show as rectangles");
    }

    ctx.set_fonts(fonts);
}

/// System font paths for symbol and emoji support
struct SystemFontPaths {
    /// Fonts for keyboard symbols, geometric shapes (⌘, ⇧, ▶, ▼)
    symbol_fonts: Vec<std::path::PathBuf>,
    /// Fonts for emojis (📁, 📄, 📂)
    emoji_fonts: Vec<std::path::PathBuf>,
}

/// Get paths to system fonts based on platform
/// Returns separate paths for symbol fonts (⌘, ⇧, ▶, ▼) and emoji fonts (📁, 📄)
fn get_system_font_paths() -> SystemFontPaths {
    let mut paths = SystemFontPaths {
        symbol_fonts: Vec::new(),
        emoji_fonts: Vec::new(),
    };

    #[cfg(target_os = "macos")]
    {
        // Symbol fonts - keyboard symbols (⌘, ⇧, ⌥) and geometric shapes (▶, ▼)
        // NOTE: These are in /System/Library/Fonts/, NOT in Supplemental/
        paths.symbol_fonts.push("/System/Library/Fonts/Keyboard.ttf".into());
        paths.symbol_fonts.push("/System/Library/Fonts/Apple Symbols.ttf".into());
        paths.symbol_fonts.push("/System/Library/Fonts/Symbol.ttf".into());
        paths.symbol_fonts.push("/System/Library/Fonts/SFNS.ttf".into());
        // User-installed Noto fonts as fallback
        paths.symbol_fonts.push("/Library/Fonts/NotoSansSymbols2-Regular.ttf".into());
        paths.symbol_fonts.push("/Library/Fonts/NotoSansSymbols-Regular.ttf".into());

        // Emoji fonts - Note: egui renders these monochrome, not color
        paths.emoji_fonts.push("/Library/Fonts/NotoEmoji-Regular.ttf".into());
        paths.emoji_fonts.push("/Library/Fonts/NotoColorEmoji.ttf".into());
    }

    #[cfg(target_os = "linux")]
    {
        // Symbol fonts
        paths.symbol_fonts.push("/usr/share/fonts/truetype/noto/NotoSansSymbols2-Regular.ttf".into());
        paths.symbol_fonts.push("/usr/share/fonts/noto/NotoSansSymbols2-Regular.ttf".into());
        paths.symbol_fonts.push("/usr/share/fonts/TTF/NotoSansSymbols2-Regular.ttf".into());
        // DejaVu Sans has good Unicode coverage including many symbols
        paths.symbol_fonts.push("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".into());
        paths.symbol_fonts.push("/usr/share/fonts/TTF/DejaVuSans.ttf".into());
        // Symbola for basic Unicode symbols
        paths.symbol_fonts.push("/usr/share/fonts/truetype/ancient-scripts/Symbola_hint.ttf".into());
        paths.symbol_fonts.push("/usr/share/fonts/TTF/Symbola.ttf".into());

        // Emoji fonts
        paths.emoji_fonts.push("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf".into());
        paths.emoji_fonts.push("/usr/share/fonts/noto-emoji/NotoColorEmoji.ttf".into());
        paths.emoji_fonts.push("/usr/share/fonts/google-noto-emoji/NotoColorEmoji.ttf".into());
        paths.emoji_fonts.push("/usr/share/fonts/TTF/NotoColorEmoji.ttf".into());
        paths.emoji_fonts.push("/usr/share/fonts/truetype/noto/NotoEmoji-Regular.ttf".into());
    }

    #[cfg(target_os = "windows")]
    {
        // Symbol fonts - comprehensive fallback chain for Unicode symbols
        if let Ok(windir) = std::env::var("WINDIR") {
            // Segoe UI Symbol - best for keyboard symbols and geometric shapes
            paths.symbol_fonts.push(format!("{}\\Fonts\\seguisym.ttf", windir).into());
            // Segoe UI - general UI font with some symbol support
            paths.symbol_fonts.push(format!("{}\\Fonts\\segoeui.ttf", windir).into());
            // Cambria Math - mathematical and technical symbols
            paths.symbol_fonts.push(format!("{}\\Fonts\\cambria.ttc", windir).into());
            // Arial Unicode MS - comprehensive Unicode coverage (may not be installed)
            paths.symbol_fonts.push(format!("{}\\Fonts\\ARIALUNI.TTF", windir).into());
            // Lucida Sans Unicode - good Unicode support
            paths.symbol_fonts.push(format!("{}\\Fonts\\l_10646.ttf", windir).into());
            // Microsoft Sans Serif - fallback
            paths.symbol_fonts.push(format!("{}\\Fonts\\micross.ttf", windir).into());
        }
        // Hardcoded fallbacks if WINDIR not set
        paths.symbol_fonts.push("C:\\Windows\\Fonts\\seguisym.ttf".into());
        paths.symbol_fonts.push("C:\\Windows\\Fonts\\segoeui.ttf".into());
        paths.symbol_fonts.push("C:\\Windows\\Fonts\\cambria.ttc".into());

        // Emoji fonts - multiple fallbacks
        if let Ok(windir) = std::env::var("WINDIR") {
            // Segoe UI Emoji - Windows default emoji font
            paths.emoji_fonts.push(format!("{}\\Fonts\\seguiemj.ttf", windir).into());
            // Segoe UI Symbol also has some emoji-like symbols
            paths.emoji_fonts.push(format!("{}\\Fonts\\seguisym.ttf", windir).into());
            // Noto Emoji if user has installed it
            paths.emoji_fonts.push(format!("{}\\Fonts\\NotoEmoji-Regular.ttf", windir).into());
        }
        paths.emoji_fonts.push("C:\\Windows\\Fonts\\seguiemj.ttf".into());
        paths.emoji_fonts.push("C:\\Windows\\Fonts\\seguisym.ttf".into());
    }

    paths
}

fn setup_file_watcher(state: &mut AppState, ctx: egui::Context, path: PathBuf) -> Option<FileWatcher> {
    let (tx, rx) = mpsc::channel();
    state.file_event_tx = Some(tx.clone());
    state.file_event_rx = Some(rx);

    match FileWatcher::new(path, tx, ctx) {
        Ok(watcher) => Some(watcher),
        Err(e) => {
            log::error!("Failed to create file watcher: {}", e);
            None
        }
    }
}

fn rfd_open_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "mdx", "txt"])
        .add_filter("All files", &["*"])
        .pick_file()
}
