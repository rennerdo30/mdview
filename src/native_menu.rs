//! Native menu bar support using muda
//!
//! Provides cross-platform native menu bar integration for macOS, Windows, and Linux.

use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    Menu, MenuEvent, MenuEventReceiver, MenuItem, PredefinedMenuItem, Submenu,
};

/// Menu action events sent to the application
#[derive(Debug, Clone, PartialEq)]
pub enum MenuAction {
    Open,
    OpenFolder,
    Reload,
    Close,
    ExportPdf,
    EditConfig,
    Quit,
    ToggleToc,
    ToggleFileBrowser,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    About,
    QuickHelp,
    CheckUpdates,
    /// Set reading width (None = full width)
    SetReadingWidth(Option<f32>),
}

/// Native menu bar manager
#[allow(dead_code)]
pub struct NativeMenuBar {
    menu: Menu,
    receiver: MenuEventReceiver,
    #[cfg(windows)]
    initialized: bool,
    // Store menu items for state updates and event matching
    open_item: MenuItem,
    open_folder_item: MenuItem,
    reload_item: MenuItem,
    close_item: MenuItem,
    export_pdf_item: MenuItem,
    edit_config_item: MenuItem,
    toggle_toc_item: MenuItem,
    toggle_browser_item: MenuItem,
    zoom_in_item: MenuItem,
    zoom_out_item: MenuItem,
    zoom_reset_item: MenuItem,
    about_item: MenuItem,
    quick_help_item: MenuItem,
    check_updates_item: MenuItem,
    // Reading width menu items
    width_full_item: MenuItem,
    width_comfortable_item: MenuItem,
    width_narrow_item: MenuItem,
}

/// Helper to create command/ctrl accelerator
fn cmd_accel(code: Code) -> Accelerator {
    #[cfg(target_os = "macos")]
    let mods = Modifiers::META;
    #[cfg(not(target_os = "macos"))]
    let mods = Modifiers::CONTROL;
    Accelerator::new(Some(mods), code)
}

/// Helper to create command/ctrl + shift accelerator
fn cmd_shift_accel(code: Code) -> Accelerator {
    #[cfg(target_os = "macos")]
    let mods = Modifiers::META | Modifiers::SHIFT;
    #[cfg(not(target_os = "macos"))]
    let mods = Modifiers::CONTROL | Modifiers::SHIFT;
    Accelerator::new(Some(mods), code)
}

impl NativeMenuBar {
    /// Create the native menu bar
    pub fn new() -> Option<Self> {
        let menu = Menu::new();

        // === File Menu ===
        let file_menu = Submenu::new("File", true);

        let open_item = MenuItem::new("Open...", true, Some(cmd_accel(Code::KeyO)));
        let open_folder_item = MenuItem::new("Open Folder...", true, Some(cmd_shift_accel(Code::KeyO)));
        let reload_item = MenuItem::new("Reload", true, Some(cmd_accel(Code::KeyR)));
        let close_item = MenuItem::new("Close", true, Some(cmd_accel(Code::KeyW)));
        let export_pdf_item = MenuItem::new("Export as PDF...", true, Some(cmd_shift_accel(Code::KeyE)));
        let edit_config_item = MenuItem::new("Edit Config...", true, Some(cmd_accel(Code::Comma)));

        let _ = file_menu.append(&open_item);
        let _ = file_menu.append(&open_folder_item);
        let _ = file_menu.append(&PredefinedMenuItem::separator());
        let _ = file_menu.append(&reload_item);
        let _ = file_menu.append(&PredefinedMenuItem::separator());
        let _ = file_menu.append(&export_pdf_item);
        let _ = file_menu.append(&PredefinedMenuItem::separator());
        let _ = file_menu.append(&edit_config_item);
        let _ = file_menu.append(&PredefinedMenuItem::separator());
        let _ = file_menu.append(&close_item);

        // On macOS, Quit is in the app menu, but we add it to File for other platforms
        #[cfg(not(target_os = "macos"))]
        {
            let _ = file_menu.append(&PredefinedMenuItem::separator());
            let _ = file_menu.append(&PredefinedMenuItem::quit(Some("Quit")));
        }

        // === View Menu ===
        let view_menu = Submenu::new("View", true);

        let toggle_toc_item = MenuItem::new("Toggle Table of Contents", true, Some(cmd_accel(Code::KeyT)));
        let toggle_browser_item = MenuItem::new("Toggle File Browser", true, Some(cmd_accel(Code::KeyB)));
        let zoom_in_item = MenuItem::new("Zoom In", true, Some(cmd_accel(Code::Equal)));
        let zoom_out_item = MenuItem::new("Zoom Out", true, Some(cmd_accel(Code::Minus)));
        let zoom_reset_item = MenuItem::new("Reset Zoom", true, Some(cmd_accel(Code::Digit0)));

        // Reading Width submenu
        let reading_width_menu = Submenu::new("Reading Width", true);
        let width_full_item = MenuItem::new("Full Width", true, None::<Accelerator>);
        let width_comfortable_item = MenuItem::new("Comfortable (720px)", true, None::<Accelerator>);
        let width_narrow_item = MenuItem::new("Narrow (560px)", true, None::<Accelerator>);
        let _ = reading_width_menu.append(&width_full_item);
        let _ = reading_width_menu.append(&width_comfortable_item);
        let _ = reading_width_menu.append(&width_narrow_item);

        let _ = view_menu.append(&toggle_toc_item);
        let _ = view_menu.append(&toggle_browser_item);
        let _ = view_menu.append(&PredefinedMenuItem::separator());
        let _ = view_menu.append(&reading_width_menu);
        let _ = view_menu.append(&PredefinedMenuItem::separator());
        let _ = view_menu.append(&zoom_in_item);
        let _ = view_menu.append(&zoom_out_item);
        let _ = view_menu.append(&zoom_reset_item);

        // === Help Menu ===
        let help_menu = Submenu::new("Help", true);

        let quick_help_item = MenuItem::new("Quick Help", true, None::<Accelerator>);
        let about_item = MenuItem::new("About mdview", true, None::<Accelerator>);
        let check_updates_item = MenuItem::new("Check for Updates...", true, None::<Accelerator>);

        let _ = help_menu.append(&quick_help_item);
        let _ = help_menu.append(&PredefinedMenuItem::separator());
        let _ = help_menu.append(&about_item);
        let _ = help_menu.append(&check_updates_item);

        // === Build Menu Bar ===
        // On macOS, add the app menu first
        #[cfg(target_os = "macos")]
        {
            let app_menu = Submenu::new("mdview", true);
            let _ = app_menu.append(&PredefinedMenuItem::about(Some("About mdview"), None));
            let _ = app_menu.append(&PredefinedMenuItem::separator());
            let _ = app_menu.append(&PredefinedMenuItem::services(None));
            let _ = app_menu.append(&PredefinedMenuItem::separator());
            let _ = app_menu.append(&PredefinedMenuItem::hide(None));
            let _ = app_menu.append(&PredefinedMenuItem::hide_others(None));
            let _ = app_menu.append(&PredefinedMenuItem::show_all(None));
            let _ = app_menu.append(&PredefinedMenuItem::separator());
            let _ = app_menu.append(&PredefinedMenuItem::quit(None));
            let _ = menu.append(&app_menu);
        }

        let _ = menu.append(&file_menu);
        let _ = menu.append(&view_menu);
        let _ = menu.append(&help_menu);

        // Get the event receiver before initializing
        let receiver = MenuEvent::receiver().clone();

        // Initialize the menu bar as the app-wide menu on macOS
        #[cfg(target_os = "macos")]
        {
            menu.init_for_nsapp();
        }

        Some(Self {
            menu,
            receiver,
            #[cfg(windows)]
            initialized: false,
            open_item,
            open_folder_item,
            reload_item,
            close_item,
            export_pdf_item,
            edit_config_item,
            toggle_toc_item,
            toggle_browser_item,
            zoom_in_item,
            zoom_out_item,
            zoom_reset_item,
            about_item,
            quick_help_item,
            check_updates_item,
            width_full_item,
            width_comfortable_item,
            width_narrow_item,
        })
    }

    /// Initialize menu for Windows HWND (must be called after window creation)
    #[cfg(windows)]
    pub fn init_for_hwnd(&mut self, hwnd: isize) {
        if !self.initialized {
            // Safety: hwnd comes from a valid window
            unsafe {
                let _ = self.menu.init_for_hwnd(hwnd);
            }
            self.initialized = true;
            log::info!("Native menu initialized for Windows HWND");
        }
    }

    /// Check if menu needs initialization (Windows only)
    #[cfg(windows)]
    pub fn needs_init(&self) -> bool {
        !self.initialized
    }

    /// No-op on non-Windows platforms
    #[cfg(not(windows))]
    pub fn needs_init(&self) -> bool {
        false
    }

    /// Poll for menu events (non-blocking)
    /// Returns all pending actions
    pub fn poll_all(&self) -> Vec<MenuAction> {
        let mut actions = Vec::new();

        while let Ok(event) = self.receiver.try_recv() {
            let id = event.id();

            let action = if id == self.open_item.id() {
                Some(MenuAction::Open)
            } else if id == self.open_folder_item.id() {
                Some(MenuAction::OpenFolder)
            } else if id == self.reload_item.id() {
                Some(MenuAction::Reload)
            } else if id == self.close_item.id() {
                Some(MenuAction::Close)
            } else if id == self.export_pdf_item.id() {
                Some(MenuAction::ExportPdf)
            } else if id == self.edit_config_item.id() {
                Some(MenuAction::EditConfig)
            } else if id == self.toggle_toc_item.id() {
                Some(MenuAction::ToggleToc)
            } else if id == self.toggle_browser_item.id() {
                Some(MenuAction::ToggleFileBrowser)
            } else if id == self.zoom_in_item.id() {
                Some(MenuAction::ZoomIn)
            } else if id == self.zoom_out_item.id() {
                Some(MenuAction::ZoomOut)
            } else if id == self.zoom_reset_item.id() {
                Some(MenuAction::ZoomReset)
            } else if id == self.about_item.id() {
                Some(MenuAction::About)
            } else if id == self.quick_help_item.id() {
                Some(MenuAction::QuickHelp)
            } else if id == self.check_updates_item.id() {
                Some(MenuAction::CheckUpdates)
            } else if id == self.width_full_item.id() {
                Some(MenuAction::SetReadingWidth(None))
            } else if id == self.width_comfortable_item.id() {
                Some(MenuAction::SetReadingWidth(Some(720.0)))
            } else if id == self.width_narrow_item.id() {
                Some(MenuAction::SetReadingWidth(Some(560.0)))
            } else {
                None
            };

            if let Some(a) = action {
                actions.push(a);
            }
        }

        actions
    }

    /// Update menu item states based on application state
    /// Call this when the state changes (file opened/closed, etc.)
    pub fn update_state(&self, has_file: bool, _file_name: Option<&str>) {
        // Enable/disable items that require a file to be open
        self.reload_item.set_enabled(has_file);
        self.close_item.set_enabled(has_file);
        self.export_pdf_item.set_enabled(has_file);

        // TOC toggle only makes sense with a file open
        self.toggle_toc_item.set_enabled(has_file);
    }
}
