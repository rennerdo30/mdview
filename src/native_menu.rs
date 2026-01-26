//! Native menu bar support using muda
//!
//! Provides cross-platform native menu bar integration for macOS, Windows, and Linux.

use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    Menu, MenuEvent, MenuEventReceiver, MenuItem, PredefinedMenuItem, Submenu,
};

/// Menu action events sent to the application
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    Open,
    OpenFolder,
    Reload,
    Close,
    ExportPdf,
    Quit,
    ToggleToc,
    ToggleFileBrowser,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    About,
    CheckUpdates,
}

/// Native menu bar manager
pub struct NativeMenuBar {
    _menu: Menu,
    receiver: MenuEventReceiver,
    // Store menu IDs for matching events
    open_id: muda::MenuId,
    open_folder_id: muda::MenuId,
    reload_id: muda::MenuId,
    close_id: muda::MenuId,
    export_pdf_id: muda::MenuId,
    toggle_toc_id: muda::MenuId,
    toggle_browser_id: muda::MenuId,
    zoom_in_id: muda::MenuId,
    zoom_out_id: muda::MenuId,
    zoom_reset_id: muda::MenuId,
    about_id: muda::MenuId,
    check_updates_id: muda::MenuId,
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

        let open_id = open_item.id().clone();
        let open_folder_id = open_folder_item.id().clone();
        let reload_id = reload_item.id().clone();
        let close_id = close_item.id().clone();
        let export_pdf_id = export_pdf_item.id().clone();

        let _ = file_menu.append(&open_item);
        let _ = file_menu.append(&open_folder_item);
        let _ = file_menu.append(&PredefinedMenuItem::separator());
        let _ = file_menu.append(&reload_item);
        let _ = file_menu.append(&PredefinedMenuItem::separator());
        let _ = file_menu.append(&export_pdf_item);
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

        let toggle_toc_id = toggle_toc_item.id().clone();
        let toggle_browser_id = toggle_browser_item.id().clone();
        let zoom_in_id = zoom_in_item.id().clone();
        let zoom_out_id = zoom_out_item.id().clone();
        let zoom_reset_id = zoom_reset_item.id().clone();

        let _ = view_menu.append(&toggle_toc_item);
        let _ = view_menu.append(&toggle_browser_item);
        let _ = view_menu.append(&PredefinedMenuItem::separator());
        let _ = view_menu.append(&zoom_in_item);
        let _ = view_menu.append(&zoom_out_item);
        let _ = view_menu.append(&zoom_reset_item);

        // === Help Menu ===
        let help_menu = Submenu::new("Help", true);

        let about_item = MenuItem::new("About mdview", true, None::<Accelerator>);
        let check_updates_item = MenuItem::new("Check for Updates...", true, None::<Accelerator>);

        let about_id = about_item.id().clone();
        let check_updates_id = check_updates_item.id().clone();

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
            _menu: menu,
            receiver,
            open_id,
            open_folder_id,
            reload_id,
            close_id,
            export_pdf_id,
            toggle_toc_id,
            toggle_browser_id,
            zoom_in_id,
            zoom_out_id,
            zoom_reset_id,
            about_id,
            check_updates_id,
        })
    }

    /// Poll for menu events (non-blocking)
    /// Returns all pending actions
    pub fn poll_all(&self) -> Vec<MenuAction> {
        let mut actions = Vec::new();

        while let Ok(event) = self.receiver.try_recv() {
            let id = event.id();

            let action = if id == &self.open_id {
                Some(MenuAction::Open)
            } else if id == &self.open_folder_id {
                Some(MenuAction::OpenFolder)
            } else if id == &self.reload_id {
                Some(MenuAction::Reload)
            } else if id == &self.close_id {
                Some(MenuAction::Close)
            } else if id == &self.export_pdf_id {
                Some(MenuAction::ExportPdf)
            } else if id == &self.toggle_toc_id {
                Some(MenuAction::ToggleToc)
            } else if id == &self.toggle_browser_id {
                Some(MenuAction::ToggleFileBrowser)
            } else if id == &self.zoom_in_id {
                Some(MenuAction::ZoomIn)
            } else if id == &self.zoom_out_id {
                Some(MenuAction::ZoomOut)
            } else if id == &self.zoom_reset_id {
                Some(MenuAction::ZoomReset)
            } else if id == &self.about_id {
                Some(MenuAction::About)
            } else if id == &self.check_updates_id {
                Some(MenuAction::CheckUpdates)
            } else {
                None
            };

            if let Some(a) = action {
                actions.push(a);
            }
        }

        actions
    }
}
