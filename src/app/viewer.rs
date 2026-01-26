//! Main viewer application implementing eframe::App

use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui::{self, Key};

use super::state::{AppState, FileEvent};
use crate::config::Config;
use crate::markdown::renderer::MarkdownRenderer;
use crate::toc::panel::TocPanel;
use crate::watcher::file_watcher::FileWatcher;

/// Main mdview application
pub struct MdViewApp {
    state: AppState,
    _watcher: Option<FileWatcher>,
    renderer: MarkdownRenderer,
    toc_panel: TocPanel,
}

impl MdViewApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        file: Option<PathBuf>,
        config: Config,
    ) -> Self {
        // Set up custom fonts if needed
        setup_fonts(&cc.egui_ctx);

        let mut state = AppState::new(config);

        // Load file if provided
        if let Some(path) = file {
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

        Self {
            state,
            _watcher: watcher,
            renderer: MarkdownRenderer::new(),
            toc_panel: TocPanel::new(),
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let toggle_toc = ctx.input(|i| i.key_pressed(Key::T) && i.modifiers.command);
        let export_pdf = ctx.input(|i| i.key_pressed(Key::P) && i.modifiers.command);
        let reload = ctx.input(|i| {
            i.key_pressed(Key::F5) || (i.key_pressed(Key::R) && i.modifiers.command)
        });
        let open_file = ctx.input(|i| i.key_pressed(Key::O) && i.modifiers.command);
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

        if escape {
            self.state.creating_annotation = false;
            self.state.text_selection = None;
        }
    }

    fn handle_file_events(&mut self) {
        // Collect events first to avoid borrow issues
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
            let events: Vec<_> = crate::markdown::parser::parse(&self.state.content).collect();

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
            if let Err(e) = self.state.load_file(path) {
                self.state.set_status(format!("Failed to open: {}", e));
            }
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open (Ctrl+O)").clicked() {
                        self.open_file_dialog();
                        ui.close_menu();
                    }
                    if ui.button("Reload (F5)").clicked() {
                        let _ = self.state.reload_file();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export PDF (Ctrl+P)").clicked() {
                        self.export_pdf();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    if ui
                        .checkbox(&mut self.state.toc_visible, "Table of Contents (Ctrl+T)")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        // Show about dialog
                        ui.close_menu();
                    }
                });
            });
        });
    }

    fn render_status_bar(&mut self, ctx: &egui::Context) {
        self.state.clear_expired_status();

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some((msg, _)) = &self.state.status_message {
                    ui.label(msg);
                } else if let Some(file) = &self.state.current_file {
                    ui.label(file.display().to_string());
                } else {
                    ui.label("No file open");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.state.config.general.hot_reload {
                        ui.label("Watching");
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
            .width_range(150.0..=400.0)
            .show(ctx, |ui| {
                if let Some(scroll_to) =
                    self.toc_panel
                        .render(ui, &self.state.toc, self.state.current_heading_idx)
                {
                    // Scroll to the selected heading
                    if scroll_to < self.state.heading_positions.len() {
                        self.state.scroll_offset = self.state.heading_positions[scroll_to];
                    }
                }
            });
    }

    fn render_main_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.state.content.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Open a markdown file with Ctrl+O or drag and drop");
                });
                return;
            }

            // Clone what we need to avoid borrow conflicts
            let content = self.state.content.clone();
            let annotations = self.state.annotations.clone();
            let config = self.state.config.clone();

            let events: Vec<_> = crate::markdown::parser::parse(&content).collect();

            let mut heading_positions = Vec::new();
            let scroll_offset;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.renderer.render(
                        ui,
                        &events,
                        &annotations,
                        &mut heading_positions,
                        &config,
                    );
                });

            scroll_offset = ui.clip_rect().top();
            self.state.heading_positions = heading_positions;
            self.state.scroll_offset = scroll_offset;

            // Update current heading based on scroll position
            let mut current_idx = None;
            for (idx, &pos) in self.state.heading_positions.iter().enumerate() {
                if pos <= scroll_offset + 50.0 {
                    current_idx = Some(idx);
                } else {
                    break;
                }
            }
            self.state.current_heading_idx = current_idx;
        });
    }
}

impl eframe::App for MdViewApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle file change events from watcher
        self.handle_file_events();

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(ctx);

        // Handle drag and drop
        let dropped_file = ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                i.raw.dropped_files[0].path.clone()
            } else {
                None
            }
        });

        if let Some(path) = dropped_file {
            if let Err(e) = self.state.load_file(path) {
                self.state.set_status(format!("Failed to open: {}", e));
            }
        }

        // Render UI
        self.render_menu_bar(ctx);
        self.render_status_bar(ctx);
        self.render_toc_sidebar(ctx);
        self.render_main_content(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save annotations before exit
        if let Err(e) = self.state.save_annotations() {
            log::error!("Failed to save annotations: {}", e);
        }
    }
}

fn setup_fonts(ctx: &egui::Context) {
    let fonts = egui::FontDefinitions::default();
    ctx.set_fonts(fonts);
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
    // Simple file dialog using native dialog
    // In a real implementation, you'd use rfd crate
    // For now, return None (user can use CLI or drag-drop)
    None
}
