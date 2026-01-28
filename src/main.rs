// Hide console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! mdview - High-Performance Extensible Markdown Viewer
//!
//! A fast, cross-platform GUI markdown viewer built with egui.

use clap::Parser;
use eframe::egui;
use std::path::{Path, PathBuf};

mod app;
mod config;
mod markdown;
mod toc;
mod annotations;
mod export;
mod theme;
mod watcher;
mod recent;
mod file_association;
mod update;
mod native_menu;
#[cfg(feature = "plugins")]
mod plugin;

pub use app::MdViewApp;
pub use config::Config;

/// Fast, cross-platform markdown viewer
#[derive(Parser, Debug)]
#[command(name = "mdview")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Markdown file or folder to open
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Theme to use (dark, light, or custom theme name). Overrides config file.
    #[arg(short, long)]
    theme: Option<String>,

    /// Disable hot reload / file watching
    #[arg(long)]
    no_watch: bool,

    /// Hide the TOC sidebar
    #[arg(long)]
    no_toc: bool,

    /// Window width
    #[arg(long, default_value = "1000")]
    width: u32,

    /// Window height
    #[arg(long, default_value = "700")]
    height: u32,

    /// Export to PDF immediately and exit
    #[arg(long)]
    export_pdf: Option<PathBuf>,

    /// Config file path (default: ~/.config/mdview/config.toml)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Reset file association prompt (ask again on next launch)
    #[arg(long)]
    reset_file_association: bool,
}

/// Input type determined from CLI path
enum InputType {
    File(PathBuf),
    Folder(PathBuf),
    None,
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn")
    ).init();

    let args = Args::parse();

    // Load configuration with improved error handling
    let mut config = if let Some(config_path) = &args.config {
        match config::loader::load_from_path(config_path) {
            Ok(cfg) => cfg,
            Err(e) => {
                log::warn!("Failed to load config from {:?}: {}. Using defaults.", config_path, e);
                eprintln!("Warning: Could not load config file {:?}: {}", config_path, e);
                Config::default()
            }
        }
    } else {
        config::loader::load_default().unwrap_or_default()
    };

    // Override config with CLI args
    if args.no_watch {
        config.general.hot_reload = false;
    }
    if args.no_toc {
        config.general.show_toc = false;
    }
    if args.reset_file_association {
        config.general.file_association_asked = false;
        log::info!("File association prompt reset - will ask on next dialog");
    }
    // Only override theme if explicitly specified on command line
    if let Some(theme) = &args.theme {
        config.general.theme = theme.clone();
    }

    // Determine input type (file vs folder)
    let input_type = match &args.path {
        Some(path) if path.is_dir() => InputType::Folder(path.clone()),
        Some(path) if path.is_file() => InputType::File(path.clone()),
        Some(path) => {
            eprintln!("Error: Path does not exist: {}", path.display());
            std::process::exit(1);
        }
        None => InputType::None,
    };

    // Handle PDF export mode (only for files)
    if let (InputType::File(ref file), Some(pdf_path)) = (&input_type, &args.export_pdf) {
        return export_pdf_and_exit(file, pdf_path, &config);
    }

    // Extract file path if it's a file input
    let file_path = match &input_type {
        InputType::File(path) => Some(path.clone()),
        _ => None,
    };

    // Extract folder path if it's a folder input
    let folder_path = match input_type {
        InputType::Folder(path) => Some(path),
        _ => None,
    };

    // Initialize native menu bar (macOS only - Windows/Linux use egui in-window menu)
    #[cfg(target_os = "macos")]
    let native_menu = {
        let menu = native_menu::NativeMenuBar::new();
        if menu.is_some() {
            log::info!("Native menu bar initialized (macOS)");
        }
        menu
    };
    #[cfg(not(target_os = "macos"))]
    let native_menu: Option<native_menu::NativeMenuBar> = None;

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([args.width as f32, args.height as f32])
            .with_min_inner_size([400.0, 300.0])
            .with_title("mdview"),
        ..Default::default()
    };

    eframe::run_native(
        "mdview",
        native_options,
        Box::new(move |cc| {
            // Apply theme
            let theme_style = theme::style::create_style(&config.general.theme, &config);
            cc.egui_ctx.set_style(theme_style);

            let mut app = MdViewApp::new(cc, file_path.clone(), config);

            // Store native menu in app for event handling
            app.set_native_menu(native_menu);

            // If a folder was specified, open it
            if let Some(folder) = folder_path.clone() {
                if let Err(e) = app.state.open_folder(folder) {
                    log::error!("Failed to open folder: {}", e);
                } else {
                    app.file_browser_visible = true;
                }
            }

            Ok(Box::new(app))
        }),
    )
}

fn export_pdf_and_exit(
    markdown_file: &Path,
    pdf_path: &Path,
    config: &Config,
) -> eframe::Result<()> {
    // Read markdown file with proper error handling
    let content = match std::fs::read_to_string(markdown_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: Failed to read markdown file {:?}: {}", markdown_file, e);
            std::process::exit(1);
        }
    };

    let events: Vec<_> = markdown::parser::parse(&content).collect();

    // Export to PDF with proper error handling
    if let Err(e) = export::pdf::export_to_pdf(&events, pdf_path, config) {
        eprintln!("Error: Failed to export PDF to {:?}: {}", pdf_path, e);
        std::process::exit(1);
    }

    println!("Exported to: {}", pdf_path.display());
    Ok(())
}
