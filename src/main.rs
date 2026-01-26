//! mdview - High-Performance Extensible Markdown Viewer
//!
//! A fast, cross-platform GUI markdown viewer built with egui.

use clap::Parser;
use eframe::egui;
use std::path::PathBuf;

mod app;
mod config;
mod markdown;
mod toc;
mod annotations;
mod export;
mod theme;
mod watcher;
#[cfg(feature = "plugins")]
mod plugin;

pub use app::MdViewApp;
pub use config::Config;

/// Fast, cross-platform markdown viewer
#[derive(Parser, Debug)]
#[command(name = "mdview")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Markdown file to open
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Theme to use (dark, light, or custom theme name)
    #[arg(short, long, default_value = "dark")]
    theme: String,

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
}

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn")
    ).init();

    let args = Args::parse();

    // Load configuration
    let mut config = if let Some(config_path) = &args.config {
        config::loader::load_from_path(config_path).unwrap_or_default()
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
    config.general.theme = args.theme.clone();

    // Handle PDF export mode
    if let (Some(file), Some(pdf_path)) = (&args.file, &args.export_pdf) {
        return export_pdf_and_exit(file, pdf_path, &config);
    }

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

            Ok(Box::new(MdViewApp::new(cc, args.file.clone(), config)))
        }),
    )
}

fn export_pdf_and_exit(
    markdown_file: &PathBuf,
    pdf_path: &PathBuf,
    config: &Config,
) -> eframe::Result<()> {
    let content = std::fs::read_to_string(markdown_file)
        .expect("Failed to read markdown file");

    let events: Vec<_> = markdown::parser::parse(&content).collect();

    export::pdf::export_to_pdf(&events, pdf_path, config)
        .expect("Failed to export PDF");

    println!("Exported to: {}", pdf_path.display());
    Ok(())
}
