//! UI regression tests for the theme tokens.
//!
//! These render real widgets through a headless `egui::Context` and inspect the resulting
//! shape list, which is enough to catch two classes of bug that are easy to reintroduce:
//!
//! * text painted in a colour that has too little contrast against the surface behind it
//!   (typically caused by using a dark-theme constant in the light theme), and
//! * markers painted *before* the background that is supposed to sit behind them, which
//!   makes them invisible because egui paints shapes back to front.

use egui::{Color32, Shape};
use mdview::annotations::model::AnnotationStore;
use mdview::app::file_browser::{FileBrowserPanel, FolderState};
use mdview::markdown::parser::parse_with_config;
use mdview::markdown::renderer::{MarkdownRenderer, RenderTargets};
use mdview::theme::style::{create_style, icon, palette, ThemeColors};
use mdview::toc::{builder::build_toc, TocPanel};
use mdview::Config;

/// WCAG AA minimum contrast for body text.
const MIN_TEXT_CONTRAST: f32 = 4.5;
/// WCAG AA minimum contrast for large text and non-text indicators.
const MIN_LARGE_TEXT_CONTRAST: f32 = 3.0;

fn channel_luminance(channel: u8) -> f32 {
    let c = channel as f32 / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(color: Color32) -> f32 {
    0.2126 * channel_luminance(color.r())
        + 0.7152 * channel_luminance(color.g())
        + 0.0722 * channel_luminance(color.b())
}

/// WCAG 2.1 contrast ratio between two opaque colours.
fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (l1, l2) = (relative_luminance(a), relative_luminance(b));
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

fn assert_contrast(fg: Color32, bg: Color32, minimum: f32, what: &str) {
    let ratio = contrast_ratio(fg, bg);
    assert!(
        ratio >= minimum,
        "{what}: contrast {ratio:.2}:1 between {fg:?} and {bg:?} is below {minimum}:1"
    );
}

/// Run a UI closure headlessly and return the painted shapes in paint order.
fn painted_shapes(is_dark: bool, mut add_contents: impl FnMut(&mut egui::Ui)) -> Vec<Shape> {
    let mut config = Config::default();
    config.general.theme = if is_dark { "dark" } else { "light" }.to_string();

    let ctx = egui::Context::default();
    ctx.set_style(create_style(&config.general.theme, &config));

    let mut shapes = Vec::new();
    // Two frames: the first one sizes and lays out, the second one paints a stable frame.
    for _ in 0..2 {
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| add_contents(ui));
        });
        shapes = output
            .shapes
            .into_iter()
            .map(|clipped| clipped.shape)
            .collect();
    }
    shapes
}

/// Every colour used by the text in `shapes`.
fn text_colors(shapes: &[Shape]) -> Vec<(String, Color32)> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Text(text) => {
                let color = text
                    .galley
                    .job
                    .sections
                    .first()
                    .map(|section| section.format.color)
                    .unwrap_or(text.fallback_color);
                Some((text.galley.text().to_owned(), color))
            }
            _ => None,
        })
        .filter(|(text, _)| !text.trim().is_empty())
        .collect()
}

#[test]
fn text_tokens_meet_wcag_aa_against_their_surfaces() {
    for colors in [ThemeColors::DARK, ThemeColors::LIGHT] {
        let label = if colors.is_dark { "dark" } else { "light" };
        for surface in [colors.content_bg, colors.sidebar_bg, colors.elevated_bg] {
            assert_contrast(
                colors.text_primary,
                surface,
                MIN_TEXT_CONTRAST,
                &format!("{label} text_primary"),
            );
            assert_contrast(
                colors.text_secondary,
                surface,
                MIN_TEXT_CONTRAST,
                &format!("{label} text_secondary"),
            );
            assert_contrast(
                colors.text_muted,
                surface,
                MIN_TEXT_CONTRAST,
                &format!("{label} text_muted"),
            );
            assert_contrast(
                colors.accent,
                surface,
                MIN_TEXT_CONTRAST,
                &format!("{label} accent"),
            );
            assert_contrast(
                colors.quote_text,
                surface,
                MIN_TEXT_CONTRAST,
                &format!("{label} quote_text"),
            );
            // Disabled text and the "watching" dot are decorative or intentionally
            // de-emphasised, so they only need the large-text/non-text threshold.
            assert_contrast(
                colors.text_disabled,
                surface,
                MIN_LARGE_TEXT_CONTRAST,
                &format!("{label} text_disabled"),
            );
            assert_contrast(
                colors.success,
                surface,
                MIN_LARGE_TEXT_CONTRAST,
                &format!("{label} success"),
            );
        }

        assert_contrast(
            colors.code_text,
            colors.code_bg,
            MIN_TEXT_CONTRAST,
            &format!("{label} code_text on code_bg"),
        );
        assert_contrast(
            colors.warning_text,
            colors.warning_bg,
            MIN_TEXT_CONTRAST,
            &format!("{label} warning_text on warning_bg"),
        );
    }
}

#[test]
fn ui_icons_have_glyphs() {
    // egui only ships a handful of fonts; a glyph that none of them covers is drawn as an
    // empty box, so every icon in the chrome has to be part of that coverage.
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |_| {});
    let font = egui::FontId::proportional(14.0);

    for glyph in icon::ALL {
        assert!(
            ctx.fonts(|fonts| fonts.has_glyphs(&font, glyph)),
            "icon {glyph:?} has no glyph in egui's bundled fonts"
        );
    }
}

#[test]
fn toc_marks_the_active_heading_visibly() {
    let toc = build_toc("# One\n\ntext\n\n## Two\n\nmore text\n");
    let mut panel = TocPanel::new();

    let shapes = painted_shapes(true, |ui| {
        panel.render(ui, &toc, Some(0), true);
    });

    let accent = palette::ACCENT;
    let marker_index = shapes.iter().position(|shape| match shape {
        Shape::Rect(rect) => rect.fill == accent && rect.rect.width() < 8.0,
        _ => false,
    });
    let marker_index = marker_index.expect("active heading marker should be painted");

    // Nothing painted after the marker may cover it, or it is invisible on screen.
    let marker_rect = match &shapes[marker_index] {
        Shape::Rect(rect) => rect.rect,
        _ => unreachable!(),
    };
    for later in &shapes[marker_index + 1..] {
        if let Shape::Rect(rect) = later {
            assert!(
                !rect.rect.contains_rect(marker_rect) || rect.fill.a() == 0,
                "shape {rect:?} is painted over the active heading marker"
            );
        }
    }
}

#[test]
fn file_browser_text_is_readable_in_the_light_theme() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::write(dir.path().join("readme.md"), "# hi\n").expect("write fixture");
    std::fs::create_dir(dir.path().join("guides")).expect("create dir");
    std::fs::write(dir.path().join("guides").join("setup.md"), "# hi\n").expect("write fixture");

    let mut folder_state = FolderState::new();
    folder_state
        .open_folder(dir.path().to_path_buf())
        .expect("open folder");
    let mut panel = FileBrowserPanel::new();

    let shapes = painted_shapes(false, |ui| {
        panel.render(ui, &mut folder_state, None);
    });

    let colors = ThemeColors::LIGHT;
    let texts = text_colors(&shapes);
    assert!(
        texts.iter().any(|(text, _)| text.contains("readme.md")),
        "file browser should list the fixture file, got {texts:?}"
    );
    for (text, color) in texts {
        // Emoji glyphs are drawn from a colour font, so their nominal colour does not
        // decide what ends up on screen.
        if text.chars().all(|c| !c.is_alphanumeric()) {
            continue;
        }
        assert_contrast(
            color,
            colors.sidebar_bg,
            MIN_LARGE_TEXT_CONTRAST,
            &format!("file browser text {text:?}"),
        );
    }
}

#[test]
fn markdown_tables_render_their_header_row() {
    let markdown = "| Column | Value |\n|--------|-------|\n| one    | two   |\n";
    let config = Config::default();
    let events: Vec<_> = parse_with_config(markdown, &config).collect();
    let annotations = AnnotationStore::new();
    let mut renderer = MarkdownRenderer::new();
    let mut heading_positions = Vec::new();

    let shapes = painted_shapes(true, |ui| {
        renderer.render_with_scroll_target(
            ui,
            &events,
            &annotations,
            &mut heading_positions,
            &config,
            RenderTargets {
                heading: None,
                search_offset: None,
                active_search_range: None,
            },
        );
    });

    let texts = text_colors(&shapes);
    for expected in ["Column", "Value", "one", "two"] {
        assert!(
            texts.iter().any(|(text, _)| text.contains(expected)),
            "table cell {expected:?} should be rendered, got {texts:?}"
        );
    }
}

#[test]
fn markdown_content_is_readable_in_the_light_theme() {
    let markdown = "\
# Heading

Body text with `inline code` in it.

> A quoted line.

| Column | Value |
|--------|-------|
| one    | two   |
";
    let config = Config::default();
    let events: Vec<_> = parse_with_config(markdown, &config).collect();
    let annotations = AnnotationStore::new();
    let mut renderer = MarkdownRenderer::new();
    let mut heading_positions = Vec::new();

    let shapes = painted_shapes(false, |ui| {
        renderer.render_with_scroll_target(
            ui,
            &events,
            &annotations,
            &mut heading_positions,
            &config,
            RenderTargets {
                heading: None,
                search_offset: None,
                active_search_range: None,
            },
        );
    });

    let colors = ThemeColors::LIGHT;
    let texts = text_colors(&shapes);
    for expected in ["Heading", "A quoted line.", "Column", "one"] {
        assert!(
            texts.iter().any(|(text, _)| text.contains(expected)),
            "expected {expected:?} to be rendered, got {texts:?}"
        );
    }
    for (text, color) in texts {
        // Table cells and blockquotes sit on the code/table surface, so compare against the
        // darkest light surface any content is painted on.
        assert_contrast(
            color,
            colors.code_bg,
            MIN_LARGE_TEXT_CONTRAST,
            &format!("markdown text {text:?}"),
        );
    }
}
