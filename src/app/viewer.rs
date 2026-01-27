//! Main viewer application implementing eframe::App
//!
//! Features a refined, modern UI inspired by Linear and modern IDEs.

use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui::{self, Key, Modifiers, Rounding, Stroke, Vec2};

use super::state::{AppState, FileEvent};

/// Parsed keybinding
#[derive(Debug, Clone)]
struct ParsedKeybinding {
    key: Key,
    modifiers: Modifiers,
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
        _ => return None,
    };

    Some(ParsedKeybinding { key, modifiers })
}

/// Check if a keybinding is pressed
fn is_keybinding_pressed(ctx: &egui::Context, binding: &str) -> bool {
    if let Some(parsed) = parse_keybinding(binding) {
        ctx.input(|i| {
            i.key_pressed(parsed.key) &&
            i.modifiers.command == parsed.modifiers.command &&
            i.modifiers.alt == parsed.modifiers.alt &&
            i.modifiers.shift == parsed.modifiers.shift
        })
    } else {
        false
    }
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
    _watcher: Option<FileWatcher>,
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
    /// Whether to show the About dialog
    show_about_dialog: bool,
    /// Native menu bar (macOS/Windows/Linux)
    native_menu: Option<crate::native_menu::NativeMenuBar>,
    /// Cached result of is_default_handler check (to avoid spawning processes every frame)
    cached_is_default_handler: Option<bool>,
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

        let mut state = AppState::new(config);
        let mut renderer = MarkdownRenderer::new();

        // Load file if provided
        if let Some(path) = file {
            // Set base path for image resolution
            let base_path = path.parent().map(|p| p.to_path_buf());
            renderer.set_base_path(base_path);

            if let Err(e) = state.load_file(path.clone()) {
                log::error!("Failed to load file: {}", e);
                state.set_status(format!("Error loading file: {}", e));
            }
        }

        // Set up file watcher
        let watcher = if state.config.general.hot_reload {
            setup_file_watcher(&mut state, cc.egui_ctx.clone())
        } else {
            None
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
            _watcher: watcher,
            renderer,
            toc_panel: TocPanel::new(),
            annotation_popup,
            file_browser: FileBrowserPanel::new(),
            file_browser_visible: false,
            shown_plugin_notification: false,
            show_file_association_dialog,
            update_checker,
            show_update_dialog: false,
            show_about_dialog: false,
            native_menu: None,
            cached_is_default_handler: None,
        }
    }

    /// Set the native menu bar (called from main after app creation)
    pub fn set_native_menu(&mut self, menu: Option<crate::native_menu::NativeMenuBar>) {
        self.native_menu = menu;
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
                self.state.set_status(format!("Failed to open folder: {}", e));
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
                self.state.set_status(format!("Failed to open browser: {}", e));
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
                            let _ = open::that("https://github.com/mdview/mdview");
                        }
                        ui.label(" | ");
                        if ui.link("Report Issue").clicked() {
                            let _ = open::that("https://github.com/mdview/mdview/issues");
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

        if should_close {
            self.show_about_dialog = false;
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
                        self.state.set_status(format!("Failed to reload: {}", e));
                    }
                }
                MenuAction::Close => {
                    // Clear current file
                    self.state.current_file = None;
                    self.state.content.clear();
                    self.state.toc = Default::default();
                }
                MenuAction::ExportPdf => {
                    if self.state.current_file.is_some() {
                        self.state.exporting_pdf = true;
                    }
                }
                MenuAction::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                MenuAction::ToggleToc => {
                    self.state.toc_visible = !self.state.toc_visible;
                }
                MenuAction::ToggleFileBrowser => {
                    self.file_browser_visible = !self.file_browser_visible;
                }
                MenuAction::ZoomIn => {
                    self.state.config.theme.fonts.size =
                        (self.state.config.theme.fonts.size + 1.0).min(32.0);
                }
                MenuAction::ZoomOut => {
                    self.state.config.theme.fonts.size =
                        (self.state.config.theme.fonts.size - 1.0).max(8.0);
                }
                MenuAction::ZoomReset => {
                    self.state.config.theme.fonts.size = 14.0;
                }
                MenuAction::About => {
                    self.show_about_dialog = true;
                }
                MenuAction::CheckUpdates => {
                    self.update_checker.check_async();
                    self.show_update_dialog = true;
                }
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let keybindings = &self.state.config.keybindings;

        let toggle_toc = is_keybinding_pressed(ctx, &keybindings.toggle_toc);
        let export_pdf = is_keybinding_pressed(ctx, &keybindings.export_pdf);
        let reload = is_keybinding_pressed(ctx, &keybindings.reload);
        let open_file = is_keybinding_pressed(ctx, &keybindings.open_file);
        let open_folder = is_keybinding_pressed(ctx, &keybindings.open_folder);
        let toggle_file_browser = is_keybinding_pressed(ctx, &keybindings.toggle_file_browser);
        let quit = is_keybinding_pressed(ctx, &keybindings.quit);
        let add_annotation = is_keybinding_pressed(ctx, &keybindings.add_annotation);
        let add_bookmark = is_keybinding_pressed(ctx, &keybindings.add_bookmark);
        let escape = ctx.input(|i| i.key_pressed(Key::Escape));

        if toggle_toc {
            self.state.toc_visible = !self.state.toc_visible;
        }

        if export_pdf {
            self.export_pdf();
        }

        if reload {
            if let Err(e) = self.state.reload_file() {
                self.state.set_status(format!("Reload failed: {}", e));
            } else {
                self.state.set_status("Reloaded");
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
            } else {
                self.open_folder_dialog();
            }
        }

        if quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if add_annotation && self.state.current_file.is_some() {
            // Show annotation popup at center of screen
            let screen_rect = ctx.screen_rect();
            let center = screen_rect.center();
            // Estimate character offset from scroll position
            let char_offset = self.estimate_char_offset_from_scroll();
            self.annotation_popup.show(center, (char_offset, char_offset.saturating_add(100)));
        }

        if add_bookmark && self.state.current_file.is_some() {
            // Add bookmark at current position
            let char_offset = self.estimate_char_offset_from_scroll();
            self.handle_annotation_action(AnnotationAction::CreateBookmark(char_offset));
        }

        if escape {
            self.state.creating_annotation = false;
            self.state.text_selection = None;
            self.annotation_popup.hide();
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
                FileEvent::Modified => {
                    if let Err(e) = self.state.reload_file() {
                        self.state.set_status(format!("Hot reload failed: {}", e));
                    } else {
                        self.state.set_status("File reloaded");
                    }
                }
                FileEvent::Removed => {
                    self.state.set_status("File was deleted");
                }
                FileEvent::Error(e) => {
                    self.state.set_status(format!("Watcher error: {}", e));
                }
            }
        }
    }

    fn export_pdf(&mut self) {
        if let Some(file_path) = &self.state.current_file {
            let pdf_path = file_path.with_extension("pdf");
            let events: Vec<_> = crate::markdown::parser::parse_with_config(&self.state.content, &self.state.config).collect();

            match crate::export::pdf::export_to_pdf(&events, &pdf_path, &self.state.config) {
                Ok(()) => {
                    self.state.set_status(format!("Exported to {}", pdf_path.display()));
                }
                Err(e) => {
                    self.state.set_status(format!("PDF export failed: {}", e));
                }
            }
        } else {
            self.state.set_status("No file open to export");
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
                    state.set_status(format!("Warning: Failed to save annotations: {}", e));
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

    /// Load a markdown file and update renderer state
    fn load_markdown_file(&mut self, path: PathBuf) {
        // Set the base path for image resolution
        let base_path = path.parent().map(|p| p.to_path_buf());
        self.renderer.set_base_path(base_path);
        self.renderer.clear_image_cache();

        if let Err(e) = self.state.load_file(path) {
            self.state.set_status(format!("Failed to open: {}", e));
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        let recent_files: Vec<_> = self
            .state
            .recent_files
            .get_existing()
            .iter()
            .map(|f| (f.path.clone(), f.display_name().to_string()))
            .collect();

        let mut file_to_open: Option<PathBuf> = None;
        let mut should_open_dialog = false;
        let mut should_open_folder_dialog = false;
        let mut should_reload = false;
        let mut should_export_pdf = false;
        let mut should_quit = false;
        let mut should_toggle_toc = false;
        let mut should_toggle_file_browser = false;
        let mut should_clear_recent = false;
        let mut should_show_about = false;
        let mut new_theme: Option<String> = None;

        let current_theme = self.state.current_theme.clone();
        let folder_is_open = self.state.folder_state.is_open();

        egui::TopBottomPanel::top("menu_bar")
            .frame(egui::Frame::none()
                .fill(palette::BG_DARK)
                .inner_margin(egui::Margin::symmetric(12.0, 6.0)))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        let open_file_label = format!("Open File...      {}", shortcuts::format("O"));
                        if ui.button(&open_file_label).clicked() {
                            should_open_dialog = true;
                            ui.close_menu();
                        }

                        let open_folder_label = format!("Open Folder...    {}", shortcuts::format_shift("O"));
                        if ui.button(&open_folder_label).clicked() {
                            should_open_folder_dialog = true;
                            ui.close_menu();
                        }

                        ui.menu_button("Recent Files", |ui| {
                            if recent_files.is_empty() {
                                ui.label(egui::RichText::new("No recent files").italics().weak());
                            } else {
                                for (path, name) in &recent_files {
                                    if ui.button(name).on_hover_text(path.display().to_string()).clicked() {
                                        file_to_open = Some(path.clone());
                                        ui.close_menu();
                                    }
                                }
                                ui.separator();
                                if ui.button("Clear Recent").clicked() {
                                    should_clear_recent = true;
                                    ui.close_menu();
                                }
                            }
                        });

                        ui.separator();

                        let reload_label = format!("Reload            {}", shortcuts::key_only("F5"));
                        if ui.button(&reload_label).clicked() {
                            should_reload = true;
                            ui.close_menu();
                        }

                        ui.separator();

                        let export_label = format!("Export PDF        {}", shortcuts::format("P"));
                        if ui.button(&export_label).clicked() {
                            should_export_pdf = true;
                            ui.close_menu();
                        }

                        ui.separator();

                        let quit_label = format!("Quit              {}", shortcuts::format("Q"));
                        if ui.button(&quit_label).clicked() {
                            should_quit = true;
                        }
                    });

                    ui.menu_button("View", |ui| {
                        let toc_shortcut = shortcuts::format("T");
                        let toc_label = if self.state.toc_visible {
                            format!("Hide Contents     {}", toc_shortcut)
                        } else {
                            format!("Show Contents     {}", toc_shortcut)
                        };
                        if ui.button(&toc_label).clicked() {
                            should_toggle_toc = true;
                            ui.close_menu();
                        }

                        if folder_is_open {
                            let fb_shortcut = shortcuts::format("E");
                            let fb_label = if self.file_browser_visible {
                                format!("Hide File Browser {}", fb_shortcut)
                            } else {
                                format!("Show File Browser {}", fb_shortcut)
                            };
                            if ui.button(&fb_label).clicked() {
                                should_toggle_file_browser = true;
                                ui.close_menu();
                            }
                        }

                        ui.separator();

                        ui.menu_button("Theme", |ui| {
                            let dark_selected = current_theme.to_lowercase() == "dark";
                            let light_selected = current_theme.to_lowercase() == "light";

                            if ui.selectable_label(dark_selected, "Dark").clicked() {
                                new_theme = Some("dark".to_string());
                                ui.close_menu();
                            }
                            if ui.selectable_label(light_selected, "Light").clicked() {
                                new_theme = Some("light".to_string());
                                ui.close_menu();
                            }
                        });
                    });

                    ui.menu_button("Help", |ui| {
                        if ui.button("About mdview").clicked() {
                            should_show_about = true;
                            ui.close_menu();
                        }
                    });

                    // Right-aligned file name
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(file) = &self.state.current_file {
                            if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                                ui.label(
                                    egui::RichText::new(name)
                                        .color(palette::TEXT_MUTED)
                                        .small()
                                );
                            }
                        }
                    });
                });
            });

        // Handle actions after UI rendering to avoid borrow issues
        if should_open_dialog {
            self.open_file_dialog();
        }
        if should_open_folder_dialog {
            self.open_folder_dialog();
        }
        if should_reload {
            let _ = self.state.reload_file();
        }
        if should_export_pdf {
            self.export_pdf();
        }
        if should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if should_show_about {
            self.show_about_dialog = true;
        }
        if should_toggle_toc {
            self.state.toc_visible = !self.state.toc_visible;
        }
        if should_toggle_file_browser {
            self.file_browser_visible = !self.file_browser_visible;
        }
        if should_clear_recent {
            self.state.recent_files.clear();
            let _ = crate::recent::save_recent_files(&self.state.recent_files);
        }
        if let Some(path) = file_to_open {
            self.load_markdown_file(path);
        }
        if let Some(theme) = new_theme {
            self.state.switch_theme(&theme);
            let style = create_style(&theme, &self.state.config);
            ctx.set_style(style);
            self.state.set_status(format!("Switched to {} theme", theme));
        }
    }

    fn render_status_bar(&mut self, ctx: &egui::Context) {
        self.state.clear_expired_status();

        egui::TopBottomPanel::bottom("status_bar")
            .frame(egui::Frame::none()
                .fill(palette::BG_DARK)
                .inner_margin(egui::Margin::symmetric(16.0, 6.0))
                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some((msg, _)) = &self.state.status_message {
                        ui.label(
                            egui::RichText::new(msg)
                                .color(palette::ACCENT)
                                .small()
                        );
                    } else if let Some(file) = &self.state.current_file {
                        ui.label(
                            egui::RichText::new(file.display().to_string())
                                .color(palette::TEXT_MUTED)
                                .small()
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("No file open")
                                .color(palette::TEXT_DISABLED)
                                .small()
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state.config.general.hot_reload {
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
        if !self.state.toc_visible {
            return;
        }

        egui::SidePanel::left("toc_panel")
            .resizable(true)
            .default_width(self.state.toc_width)
            .width_range(180.0..=400.0)
            .frame(egui::Frame::none()
                .fill(palette::BG_DARK)
                .inner_margin(egui::Margin::same(0.0))
                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE)))
            .show(ctx, |ui| {
                // Header
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("CONTENTS")
                            .color(palette::TEXT_MUTED)
                            .small()
                            .strong()
                    );
                });
                ui.add_space(8.0);

                // TOC entries
                if let Some(scroll_to) =
                    self.toc_panel
                        .render(ui, &self.state.toc, self.state.current_heading_idx)
                {
                    // Set the target heading to scroll to
                    self.state.scroll_to_heading = Some(scroll_to);
                }
            });
    }

    fn render_file_browser_sidebar(&mut self, ctx: &egui::Context) {
        if !self.file_browser_visible || !self.state.folder_state.is_open() {
            return;
        }

        let mut file_to_open: Option<PathBuf> = None;

        egui::SidePanel::right("file_browser_panel")
            .resizable(true)
            .default_width(250.0)
            .width_range(180.0..=400.0)
            .frame(egui::Frame::none()
                .fill(palette::BG_DARK)
                .inner_margin(egui::Margin::same(8.0))
                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE)))
            .show(ctx, |ui| {
                // Header
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("FILES")
                            .color(palette::TEXT_MUTED)
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
        let recent_files: Vec<_> = self
            .state
            .recent_files
            .get_existing()
            .iter()
            .map(|f| (f.path.clone(), f.display_name().to_string(), f.path.parent().and_then(|p| p.to_str()).unwrap_or("").to_string()))
            .collect();

        let mut file_to_open: Option<PathBuf> = None;

        egui::CentralPanel::default()
            .frame(egui::Frame::none()
                .fill(palette::BG_BASE)
                .inner_margin(egui::Margin::same(0.0)))
            .show(ctx, |ui| {
                if self.state.content.is_empty() {
                    // Refined welcome screen
                    render_welcome_screen(ui, &recent_files, &mut file_to_open);
                    return;
                }

                // Main content area with comfortable margins
                let content = self.state.content.clone();
                let annotations = self.state.annotations.clone();
                let config = self.state.config.clone();

                // Call pre-render hook
                #[cfg(feature = "plugins")]
                self.state.call_plugin_hook(crate::plugin::api::PluginHook::OnPreRender);

                let events: Vec<_> = crate::markdown::parser::parse_with_config(&content, &config).collect();

                let mut heading_positions = Vec::new();
                let scroll_target = self.state.scroll_to_heading.take();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(32.0);
                        ui.horizontal(|ui| {
                            ui.add_space(48.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(720.0); // Comfortable reading width
                                self.renderer.render_with_scroll_target(
                                    ui,
                                    &events,
                                    &annotations,
                                    &mut heading_positions,
                                    &config,
                                    scroll_target,
                                );
                            });
                        });
                        ui.add_space(64.0);
                    });

                let scroll_offset = ui.clip_rect().top();
                self.state.heading_positions = heading_positions;
                self.state.scroll_offset = scroll_offset;

                // Track visible character range for annotation positioning
                // The renderer's char_offset represents the end of rendered content
                let total_chars = self.renderer.current_char_offset();
                if total_chars > 0 {
                    // Estimate visible range based on scroll position ratio
                    // This gives us a much better estimate than the previous heuristic
                    let content_len = self.state.content.len();
                    if content_len > 0 {
                        let ratio = (scroll_offset / scroll_offset.max(1.0)).clamp(0.0, 1.0);
                        let start = (ratio * content_len as f32) as usize;
                        // Visible window is approximately one screen worth
                        let visible_chars = content_len / 10; // ~10% of document per screen
                        let end = (start + visible_chars).min(content_len);
                        self.state.visible_char_range = Some((start, end));
                    }
                }

                let mut current_idx = None;
                for (idx, &pos) in self.state.heading_positions.iter().enumerate() {
                    if pos <= scroll_offset + 50.0 {
                        current_idx = Some(idx);
                    } else {
                        break;
                    }
                }
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

/// Render the refined welcome screen
fn render_welcome_screen(
    ui: &mut egui::Ui,
    recent_files: &[(PathBuf, String, String)],
    file_to_open: &mut Option<PathBuf>,
) {
    let available_size = ui.available_size();

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
                        .color(palette::TEXT_PRIMARY)
                        .strong()
                );
            });
        });

        ui.add_space(8.0);

        ui.label(
            egui::RichText::new("A modern markdown viewer")
                .size(16.0)
                .color(palette::TEXT_MUTED)
        );

        ui.add_space(40.0);

        // Action hints
        ui.horizontal(|ui| {
            ui.add_space((available_size.x - 300.0) / 2.0);
            ui.vertical(|ui| {
                render_action_hint(ui, "⌘O", "Open file");
                ui.add_space(8.0);
                render_action_hint(ui, "drag", "Drop a file here");
            });
        });

        ui.add_space(48.0);

        // Recent files section
        if !recent_files.is_empty() {
            let card_width = 400.0;

            egui::Frame::none()
                .fill(palette::BG_ELEVATED)
                .rounding(Rounding::same(12.0))
                .stroke(Stroke::new(1.0, palette::BORDER_SUBTLE))
                .inner_margin(egui::Margin::same(20.0))
                .show(ui, |ui| {
                    ui.set_min_width(card_width);
                    ui.set_max_width(card_width);

                    ui.label(
                        egui::RichText::new("Recent Files")
                            .size(13.0)
                            .color(palette::TEXT_MUTED)
                            .strong()
                    );

                    ui.add_space(12.0);

                    for (path, name, dir) in recent_files.iter().take(5) {
                        let response = render_recent_file_item(ui, name, dir);
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

    // Blue gradient background (using solid color as egui doesn't support gradients easily)
    let bg_color = egui::Color32::from_rgb(59, 130, 246); // #3B82F6
    painter.rect_filled(rect, Rounding::same(12.0), bg_color);

    // Add subtle highlight at top
    let highlight_rect = egui::Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width(), rect.height() * 0.4),
    );
    painter.rect_filled(
        highlight_rect,
        Rounding::same(12.0),
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20),
    );

    // Draw the M letter
    let center = rect.center();
    let m_width = size * 0.6;
    let m_height = size * 0.5;
    let m_left = center.x - m_width / 2.0;
    let m_top = center.y - m_height / 2.0;

    // M shape using lines
    let stroke = egui::Stroke::new(4.0, egui::Color32::WHITE);

    // Left vertical
    painter.line_segment(
        [
            egui::Pos2::new(m_left, m_top + m_height),
            egui::Pos2::new(m_left, m_top),
        ],
        stroke,
    );

    // Left diagonal
    painter.line_segment(
        [
            egui::Pos2::new(m_left, m_top),
            egui::Pos2::new(center.x, m_top + m_height * 0.5),
        ],
        stroke,
    );

    // Right diagonal
    painter.line_segment(
        [
            egui::Pos2::new(center.x, m_top + m_height * 0.5),
            egui::Pos2::new(m_left + m_width, m_top),
        ],
        stroke,
    );

    // Right vertical
    painter.line_segment(
        [
            egui::Pos2::new(m_left + m_width, m_top),
            egui::Pos2::new(m_left + m_width, m_top + m_height),
        ],
        stroke,
    );

    // Small yellow accent dot in corner
    let dot_pos = egui::Pos2::new(rect.max.x - 8.0, rect.max.y - 8.0);
    painter.circle_filled(dot_pos, 4.0, egui::Color32::from_rgb(252, 211, 77));
}

/// Render an action hint (shortcut + description)
fn render_action_hint(ui: &mut egui::Ui, shortcut: &str, description: &str) {
    ui.horizontal(|ui| {
        egui::Frame::none()
            .fill(palette::BG_ELEVATED)
            .rounding(Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(shortcut)
                        .color(palette::TEXT_SECONDARY)
                        .small()
                        .strong()
                );
            });
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(description)
                .color(palette::TEXT_MUTED)
        );
    });
}

/// Render a recent file item as a clickable row
fn render_recent_file_item(ui: &mut egui::Ui, name: &str, dir: &str) -> egui::Response {
    // Truncate directory path
    let truncated_dir = if dir.len() > 50 {
        format!("...{}", &dir[dir.len()-47..])
    } else {
        dir.to_string()
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
            palette::BG_HOVER,
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
        if response.hovered() { palette::TEXT_SECONDARY } else { palette::TEXT_MUTED },
    );

    // File name
    ui.painter().text(
        egui::Pos2::new(text_x, rect.min.y + 14.0),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(14.0),
        if response.hovered() { palette::TEXT_PRIMARY } else { palette::TEXT_SECONDARY },
    );

    // Directory path
    ui.painter().text(
        egui::Pos2::new(text_x, rect.min.y + 32.0),
        egui::Align2::LEFT_CENTER,
        &truncated_dir,
        egui::FontId::proportional(11.0),
        palette::TEXT_DISABLED,
    );

    response
}

impl eframe::App for MdViewApp {
    #[allow(unused_variables)]
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Initialize native menu for Windows (must be done after window creation)
        #[cfg(windows)]
        self.init_native_menu_windows(frame);

        self.handle_file_events();
        self.handle_keyboard_shortcuts(ctx);

        // Poll for completed async mermaid renders
        self.renderer.poll_mermaid_renders(ctx);

        // Handle native menu events
        self.handle_native_menu_events(ctx);

        // Show plugin failure notification (once)
        #[cfg(feature = "plugins")]
        if !self.shown_plugin_notification && self.state.has_failed_plugins() {
            let count = self.state.failed_plugin_count();
            self.state.set_status(format!("Warning: {} plugin(s) failed to load", count));
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
                let style = create_style(&self.state.current_theme, &self.state.config);
                ctx.set_style(style);
            }
        }

        // Handle drag and drop
        let dropped_file = ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                i.raw.dropped_files[0].path.clone()
            } else {
                None
            }
        });

        if let Some(path) = dropped_file {
            // Check if it's a directory or file
            if path.is_dir() {
                if let Err(e) = self.state.open_folder(path) {
                    self.state.set_status(format!("Failed to open folder: {}", e));
                } else {
                    self.file_browser_visible = true;
                    self.state.set_status("Folder opened");
                }
            } else {
                self.load_markdown_file(path);
            }
        }

        // Render UI
        self.render_menu_bar(ctx);
        self.render_status_bar(ctx);
        self.render_toc_sidebar(ctx);
        self.render_file_browser_sidebar(ctx);
        self.render_main_content(ctx);

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
            if let Some(ref info) = self.update_checker.update_info() {
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

        // Render about dialog if needed
        if self.show_about_dialog {
            self.render_about_dialog(ctx);
        }
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
    }
}

fn setup_fonts(ctx: &egui::Context, config: &Config) {
    let mut fonts = egui::FontDefinitions::default();

    // Try to load custom fonts from config directory
    let font_dir = directories::ProjectDirs::from("", "", "mdview")
        .map(|dirs| dirs.config_dir().join("fonts"));

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
                            egui::FontData::from_owned(font_data).into(),
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
                            egui::FontData::from_owned(font_data).into(),
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
                    egui::FontData::from_owned(font_data).into(),
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
                    egui::FontData::from_owned(font_data).into(),
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

fn setup_file_watcher(state: &mut AppState, ctx: egui::Context) -> Option<FileWatcher> {
    let (tx, rx) = mpsc::channel();
    state.file_event_tx = Some(tx.clone());
    state.file_event_rx = Some(rx);

    if let Some(file_path) = &state.current_file {
        match FileWatcher::new(file_path.clone(), tx, ctx) {
            Ok(watcher) => Some(watcher),
            Err(e) => {
                log::error!("Failed to create file watcher: {}", e);
                None
            }
        }
    } else {
        None
    }
}

fn rfd_open_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Markdown", &["md", "markdown", "mdx", "txt"])
        .add_filter("All files", &["*"])
        .pick_file()
}
