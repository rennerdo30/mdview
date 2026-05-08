//! PDF export using printpdf

use std::io::BufWriter;
use std::path::{Path, PathBuf};

use printpdf::{BuiltinFont, Image, ImageTransform, Mm, PdfDocument};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, HeadingLevel, Tag, TagEnd};

use crate::config::defaults::heading_size_multiplier;
use crate::config::Config;

/// A4 page dimensions in mm
const A4_WIDTH_MM: f32 = 210.0;
const A4_HEIGHT_MM: f32 = 297.0;

/// Letter page dimensions in mm
const LETTER_WIDTH_MM: f32 = 215.9;
const LETTER_HEIGHT_MM: f32 = 279.4;

/// Export markdown events to PDF
#[allow(dead_code)]
pub fn export_to_pdf(
    events: &[Event<'_>],
    output_path: &Path,
    config: &Config,
) -> Result<(), PdfError> {
    export_to_pdf_with_base(events, output_path, config, None)
}

/// Export markdown events to PDF with a base path for resolving relative image paths
pub fn export_to_pdf_with_base(
    events: &[Event<'_>],
    output_path: &Path,
    config: &Config,
    base_path: Option<&Path>,
) -> Result<(), PdfError> {
    let (width_mm, height_mm) = match config.export.page_size.to_lowercase().as_str() {
        "letter" => (LETTER_WIDTH_MM, LETTER_HEIGHT_MM),
        _ => (A4_WIDTH_MM, A4_HEIGHT_MM),
    };

    let margin_mm = config.export.margin as f32;

    // Determine colors based on pdf_theme
    let is_dark_theme = config.export.pdf_theme.to_lowercase() == "dark";

    let (doc, page1, layer1) =
        PdfDocument::new("Markdown Export", Mm(width_mm), Mm(height_mm), "Layer 1");

    // Add built-in fonts
    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
    let font_mono = doc.add_builtin_font(BuiltinFont::Courier)?;

    let mut exporter = PdfExporter {
        current_page: page1,
        current_layer: layer1,
        width_mm,
        height_mm,
        margin_mm,
        cursor_y: height_mm - margin_mm,
        line_height: 5.0,
        font_size: 12.0,
        base_font_size: 12.0,
        in_code_block: false,
        code_content: String::new(),
        code_language: None,
        text_buffer: String::new(),
        heading_level: 0,
        list_depth: 0,
        list_stack: Vec::new(),
        in_emphasis: false,
        in_strong: false,
        in_strikethrough: false,
        in_blockquote: false,
        in_table: false,
        table_alignments: Vec::new(),
        table_row: Vec::new(),
        table_rows: Vec::new(),
        table_header: Vec::new(),
        in_table_head: false,
        task_list_marker: None,
        is_dark_theme,
        custom_heading_color: config.theme.colors.heading.clone(),
        custom_code_color: config.theme.colors.code_text.clone(),
        syntax_theme: config.markdown.syntax_theme.clone(),
        syntax_highlighting_enabled: config.markdown.syntax_highlighting,
        base_path: base_path.map(|p| p.to_path_buf()),
        toc_entries: Vec::new(),
    };

    // Fill first page background for dark theme
    if is_dark_theme {
        exporter.fill_page_background(&doc);
    }

    // First pass: collect TOC entries if enabled
    if config.export.include_toc {
        exporter.collect_toc_entries(events);
    }

    // Render TOC at the beginning if enabled and we have entries
    if config.export.include_toc && !exporter.toc_entries.is_empty() {
        exporter.render_toc(&doc, &font, &font_bold)?;
    }

    // Render the actual content
    exporter.render_events(&doc, events, &font, &font_bold, &font_mono)?;

    let file = std::fs::File::create(output_path).map_err(|e| PdfError::Io(e.to_string()))?;
    doc.save(&mut BufWriter::new(file))?;

    Ok(())
}

/// TOC entry for PDF export
#[derive(Clone)]
struct TocPdfEntry {
    text: String,
    level: usize,
}

#[derive(Clone, Debug)]
struct ListState {
    next_number: Option<u64>,
}

#[derive(Clone, Copy)]
struct TableRowLayout {
    num_cols: usize,
    col_width: f32,
    chars_per_line: usize,
    is_header: bool,
}

struct PdfExporter {
    current_page: printpdf::PdfPageIndex,
    current_layer: printpdf::PdfLayerIndex,
    width_mm: f32,
    height_mm: f32,
    margin_mm: f32,
    cursor_y: f32,
    line_height: f32,
    font_size: f32,
    base_font_size: f32,
    in_code_block: bool,
    code_content: String,
    code_language: Option<String>,
    text_buffer: String,
    heading_level: usize,
    list_depth: usize,
    list_stack: Vec<ListState>,
    in_emphasis: bool,
    in_strong: bool,
    in_strikethrough: bool,
    in_blockquote: bool,
    in_table: bool,
    table_alignments: Vec<Alignment>,
    table_row: Vec<String>,
    table_rows: Vec<Vec<String>>,
    table_header: Vec<String>,
    in_table_head: bool,
    task_list_marker: Option<bool>,
    /// Theme flag for PDF color theming
    is_dark_theme: bool,
    /// Custom heading color from config (hex)
    custom_heading_color: Option<String>,
    /// Custom code color from config (hex)
    custom_code_color: Option<String>,
    /// Syntax theme for code highlighting
    syntax_theme: String,
    /// Whether syntax highlighting is enabled
    syntax_highlighting_enabled: bool,
    /// Base path for resolving relative image paths
    base_path: Option<PathBuf>,
    toc_entries: Vec<TocPdfEntry>,
}

impl PdfExporter {
    /// Parse hex color to RGB values (0.0-1.0)
    fn parse_hex_color(hex: &str) -> Option<(f32, f32, f32)> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
        Some((r, g, b))
    }

    /// Get text color based on theme
    /// Note: PDF backgrounds are always white (printpdf limitation). Dark theme uses
    /// slightly different color palette but still works on white paper.
    fn text_color(&self) -> printpdf::Color {
        if self.is_dark_theme {
            // Dark theme: light gray text on dark background
            printpdf::Color::Rgb(printpdf::Rgb::new(0.85, 0.87, 0.90, None))
        } else {
            // Light theme: dark text on white background
            printpdf::Color::Rgb(printpdf::Rgb::new(0.1, 0.1, 0.15, None))
        }
    }

    /// Get blockquote text color (slightly muted)
    fn blockquote_color(&self) -> printpdf::Color {
        if self.is_dark_theme {
            // Muted light text for dark theme
            printpdf::Color::Rgb(printpdf::Rgb::new(0.65, 0.68, 0.72, None))
        } else {
            // Muted dark text for light theme
            printpdf::Color::Rgb(printpdf::Rgb::new(0.4, 0.4, 0.45, None))
        }
    }

    /// Get heading color based on theme or config
    fn heading_color(&self) -> printpdf::Color {
        // Use custom color from config if specified
        if let Some(ref hex) = self.custom_heading_color {
            if let Some((r, g, b)) = Self::parse_hex_color(hex) {
                return printpdf::Color::Rgb(printpdf::Rgb::new(r, g, b, None));
            }
        }

        if self.is_dark_theme {
            // Bright white for headings in dark theme
            printpdf::Color::Rgb(printpdf::Rgb::new(0.95, 0.96, 0.98, None))
        } else {
            // Very dark for headings in light theme
            printpdf::Color::Rgb(printpdf::Rgb::new(0.05, 0.05, 0.1, None))
        }
    }

    /// Get code color based on theme or config
    fn code_color(&self) -> printpdf::Color {
        // Use custom color from config if specified
        if let Some(ref hex) = self.custom_code_color {
            if let Some((r, g, b)) = Self::parse_hex_color(hex) {
                return printpdf::Color::Rgb(printpdf::Rgb::new(r, g, b, None));
            }
        }

        if self.is_dark_theme {
            // Cyan/teal for code in dark theme (good contrast on dark bg)
            printpdf::Color::Rgb(printpdf::Rgb::new(0.4, 0.8, 0.75, None))
        } else {
            // Brown-orange for code in light theme
            printpdf::Color::Rgb(printpdf::Rgb::new(0.5, 0.3, 0.2, None))
        }
    }

    /// Collect TOC entries from events
    fn collect_toc_entries(&mut self, events: &[Event<'_>]) {
        let mut in_heading = false;
        let mut heading_level = 0;
        let mut heading_text = String::new();

        for event in events {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    in_heading = true;
                    heading_level = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    heading_text.clear();
                }
                Event::End(TagEnd::Heading(_)) => {
                    if in_heading && !heading_text.is_empty() {
                        self.toc_entries.push(TocPdfEntry {
                            text: heading_text.trim().to_string(),
                            level: heading_level,
                        });
                    }
                    in_heading = false;
                }
                Event::Text(text) if in_heading => {
                    heading_text.push_str(text);
                }
                Event::Code(code) if in_heading => {
                    heading_text.push_str(code);
                }
                _ => {}
            }
        }
    }

    /// Render the table of contents
    fn render_toc(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        font: &printpdf::IndirectFontRef,
        font_bold: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        // TOC title
        self.ensure_space(doc, 10.0)?;
        let layer = doc
            .get_page(self.current_page)
            .get_layer(self.current_layer);
        layer.use_text(
            "Table of Contents",
            16.0,
            Mm(self.margin_mm),
            Mm(self.cursor_y),
            font_bold,
        );
        self.cursor_y -= 8.0;

        // Render each TOC entry
        for entry in self.toc_entries.clone() {
            self.ensure_space(doc, self.line_height)?;

            let indent = (entry.level - 1) as f32 * 5.0;
            let font_size = match entry.level {
                1 => self.base_font_size,
                2 => self.base_font_size * 0.95,
                _ => self.base_font_size * 0.9,
            };

            let layer = doc
                .get_page(self.current_page)
                .get_layer(self.current_layer);

            // Use bold for H1, regular for others
            let entry_font = if entry.level == 1 { font_bold } else { font };

            layer.use_text(
                &entry.text,
                font_size,
                Mm(self.margin_mm + indent),
                Mm(self.cursor_y),
                entry_font,
            );

            self.cursor_y -= self.line_height;
        }

        // Add separator after TOC
        self.cursor_y -= 5.0;
        self.draw_horizontal_rule(doc)?;
        self.cursor_y -= 5.0;

        Ok(())
    }

    fn render_events(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        events: &[Event<'_>],
        font: &printpdf::IndirectFontRef,
        font_bold: &printpdf::IndirectFontRef,
        font_mono: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        for event in events {
            match event {
                Event::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    // Handle image inline instead of in handle_start_tag
                    self.draw_image(doc, dest_url, title)?;
                }
                Event::Start(tag) => self.handle_start_tag(tag),
                Event::End(tag) => self.handle_end_tag(doc, tag, font, font_bold, font_mono)?,
                Event::Text(text) => {
                    if self.in_code_block {
                        self.code_content.push_str(text);
                    } else {
                        self.text_buffer.push_str(text);
                    }
                }
                Event::Code(code) => {
                    self.text_buffer.push('`');
                    self.text_buffer.push_str(code);
                    self.text_buffer.push('`');
                }
                Event::TaskListMarker(checked) => {
                    self.task_list_marker = Some(*checked);
                }
                Event::SoftBreak | Event::HardBreak => {
                    self.text_buffer.push(' ');
                }
                Event::Rule => {
                    self.draw_horizontal_rule(doc)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_start_tag(&mut self, tag: &Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
            }
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_content.clear();
                self.code_language = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
                    _ => None,
                };
            }
            Tag::List(start) => {
                self.list_depth += 1;
                self.list_stack.push(ListState {
                    next_number: *start,
                });
            }
            Tag::Emphasis => {
                self.in_emphasis = true;
            }
            Tag::Strong => {
                self.in_strong = true;
            }
            Tag::Strikethrough => {
                self.in_strikethrough = true;
            }
            Tag::BlockQuote(_) => {
                self.in_blockquote = true;
            }
            Tag::Table(alignments) => {
                self.in_table = true;
                self.table_alignments = alignments.clone();
            }
            Tag::TableHead => {
                self.in_table_head = true;
            }
            Tag::TableRow => {
                self.table_row.clear();
            }
            _ => {}
        }
    }

    fn handle_end_tag(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        tag: &TagEnd,
        font: &printpdf::IndirectFontRef,
        font_bold: &printpdf::IndirectFontRef,
        font_mono: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        match tag {
            TagEnd::Heading(_) => {
                let text = std::mem::take(&mut self.text_buffer);
                if !text.is_empty() {
                    self.draw_heading(doc, &text, font_bold)?;
                }
                self.heading_level = 0;
            }
            TagEnd::Paragraph => {
                let text = std::mem::take(&mut self.text_buffer);
                if !text.is_empty() {
                    if self.in_blockquote {
                        self.draw_blockquote(doc, &text, font)?;
                    } else {
                        let f = if self.in_strong { font_bold } else { font };
                        self.draw_paragraph(doc, &text, f)?;
                    }
                }
            }
            TagEnd::CodeBlock => {
                let code = std::mem::take(&mut self.code_content);
                let language = self.code_language.take();
                if !code.is_empty() {
                    self.draw_code_block(doc, &code, language.as_deref(), font_mono)?;
                }
                self.in_code_block = false;
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                self.list_stack.pop();
            }
            TagEnd::Item => {
                let text = std::mem::take(&mut self.text_buffer);
                if !text.is_empty() {
                    let marker =
                        next_list_marker(&mut self.list_stack, self.task_list_marker.take());
                    self.draw_list_item(doc, &text, font, &marker)?;
                }
            }
            TagEnd::Emphasis => {
                self.in_emphasis = false;
            }
            TagEnd::Strong => {
                self.in_strong = false;
            }
            TagEnd::Strikethrough => {
                self.in_strikethrough = false;
            }
            TagEnd::BlockQuote(_) => {
                self.in_blockquote = false;
            }
            TagEnd::Table => {
                self.draw_table(doc, font, font_bold)?;
                self.in_table = false;
                self.table_alignments.clear();
                self.table_row.clear();
                self.table_rows.clear();
                self.table_header.clear();
            }
            TagEnd::TableHead => {
                self.in_table_head = false;
            }
            TagEnd::TableRow => {
                let row = std::mem::take(&mut self.table_row);
                if self.in_table_head {
                    self.table_header = row;
                } else {
                    self.table_rows.push(row);
                }
            }
            TagEnd::TableCell => {
                self.table_row.push(std::mem::take(&mut self.text_buffer));
            }
            _ => {}
        }

        Ok(())
    }

    fn ensure_space(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        needed: f32,
    ) -> Result<(), PdfError> {
        if self.cursor_y - needed < self.margin_mm {
            self.new_page(doc)?;
        }
        Ok(())
    }

    fn new_page(&mut self, doc: &printpdf::PdfDocumentReference) -> Result<(), PdfError> {
        let (page, layer) = doc.add_page(Mm(self.width_mm), Mm(self.height_mm), "Layer 1");
        self.current_page = page;
        self.current_layer = layer;
        self.cursor_y = self.height_mm - self.margin_mm;

        // Fill background for dark theme
        if self.is_dark_theme {
            self.fill_page_background(doc);
        }

        Ok(())
    }

    /// Fill the current page with background color (for dark theme)
    fn fill_page_background(&self, doc: &printpdf::PdfDocumentReference) {
        let layer = doc
            .get_page(self.current_page)
            .get_layer(self.current_layer);

        // Dark theme background color (dark blue-gray)
        let bg_color = printpdf::Color::Rgb(printpdf::Rgb::new(0.12, 0.14, 0.18, None));
        layer.set_fill_color(bg_color.clone());
        layer.set_outline_color(bg_color);

        // Draw full-page rectangle
        let points = vec![
            (printpdf::Point::new(Mm(0.0), Mm(0.0)), false),
            (printpdf::Point::new(Mm(self.width_mm), Mm(0.0)), false),
            (
                printpdf::Point::new(Mm(self.width_mm), Mm(self.height_mm)),
                false,
            ),
            (printpdf::Point::new(Mm(0.0), Mm(self.height_mm)), false),
        ];
        let rect = printpdf::Line {
            points,
            is_closed: true,
        };
        layer.add_line(rect);
    }

    fn draw_heading(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        text: &str,
        font: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        let size_multiplier = heading_size_multiplier(self.heading_level);
        let font_size = self.base_font_size * size_multiplier;
        let line_height = font_size * 0.5;

        // Word wrap heading text
        let available_width = self.width_mm - (self.margin_mm * 2.0);
        let chars_per_line = (available_width / (font_size * 0.4)).max(1.0) as usize;

        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            if current_line.len() + word.len() + 1 > chars_per_line && !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        let total_height = line_height * lines.len() as f32 + 3.0;
        self.ensure_space(doc, total_height)?;

        // Add spacing before heading
        self.cursor_y -= 3.0;

        for line in &lines {
            let layer = doc
                .get_page(self.current_page)
                .get_layer(self.current_layer);

            // Apply heading color based on theme
            layer.set_fill_color(self.heading_color());

            layer.use_text(line, font_size, Mm(self.margin_mm), Mm(self.cursor_y), font);

            self.cursor_y -= line_height;
        }

        self.cursor_y -= 2.0;

        Ok(())
    }

    fn draw_paragraph(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        text: &str,
        font: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        let available_width = self.width_mm - (self.margin_mm * 2.0);
        let chars_per_line = (available_width / (self.font_size * 0.4)) as usize;

        // Simple word wrapping
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            if current_line.len() + word.len() + 1 > chars_per_line && !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        for line in lines {
            self.ensure_space(doc, self.line_height)?;

            let layer = doc
                .get_page(self.current_page)
                .get_layer(self.current_layer);

            // Apply text color based on theme
            layer.set_fill_color(self.text_color());

            layer.use_text(
                &line,
                self.font_size,
                Mm(self.margin_mm),
                Mm(self.cursor_y),
                font,
            );

            self.cursor_y -= self.line_height;
        }

        // Paragraph spacing
        self.cursor_y -= 2.0;

        Ok(())
    }

    fn draw_blockquote(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        text: &str,
        font: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        let indent = 10.0; // Left indent for blockquote
        let bar_width = 2.0; // Width of vertical bar
        let padding = 3.0; // Padding around text
        let available_width = self.width_mm - (self.margin_mm * 2.0) - indent - padding;
        let chars_per_line = (available_width / (self.font_size * 0.4)) as usize;

        // Simple word wrapping
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            if current_line.len() + word.len() + 1 > chars_per_line && !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Calculate total height needed
        let total_height = (lines.len() as f32 * self.line_height) + (padding * 2.0);
        self.ensure_space(doc, total_height)?;

        // Record starting position for background
        let bg_top_y = self.cursor_y + padding;
        let bg_left_x = self.margin_mm + bar_width + 1.0;
        let bg_width = self.width_mm - (self.margin_mm * 2.0) - bar_width - 1.0;

        // Draw background rectangle first (light gray fill)
        {
            let layer = doc
                .get_page(self.current_page)
                .get_layer(self.current_layer);
            let bg_color = if self.is_dark_theme {
                // Slightly lighter than page background for contrast
                printpdf::Color::Rgb(printpdf::Rgb::new(0.18, 0.20, 0.25, None))
            } else {
                printpdf::Color::Rgb(printpdf::Rgb::new(0.95, 0.95, 0.97, None))
            };
            layer.set_fill_color(bg_color.clone());
            layer.set_outline_color(bg_color);

            let bg_bottom_y = bg_top_y - total_height;
            let points = vec![
                (printpdf::Point::new(Mm(bg_left_x), Mm(bg_top_y)), false),
                (
                    printpdf::Point::new(Mm(bg_left_x + bg_width), Mm(bg_top_y)),
                    false,
                ),
                (
                    printpdf::Point::new(Mm(bg_left_x + bg_width), Mm(bg_bottom_y)),
                    false,
                ),
                (printpdf::Point::new(Mm(bg_left_x), Mm(bg_bottom_y)), false),
            ];
            let rect = printpdf::Line {
                points,
                is_closed: true,
            };
            layer.add_line(rect);
        }

        // Draw the text
        self.cursor_y -= padding;
        for line in &lines {
            let layer = doc
                .get_page(self.current_page)
                .get_layer(self.current_layer);

            // Apply blockquote text color (muted/gray)
            layer.set_fill_color(self.blockquote_color());

            layer.use_text(
                line,
                self.font_size,
                Mm(self.margin_mm + indent),
                Mm(self.cursor_y),
                font,
            );

            self.cursor_y -= self.line_height;
        }

        // Draw vertical bar on the left side
        let bar_top_y = bg_top_y;
        let bar_bottom_y = bg_top_y - total_height;
        let layer = doc
            .get_page(self.current_page)
            .get_layer(self.current_layer);

        let points = vec![
            (
                printpdf::Point::new(Mm(self.margin_mm + bar_width / 2.0), Mm(bar_top_y)),
                false,
            ),
            (
                printpdf::Point::new(Mm(self.margin_mm + bar_width / 2.0), Mm(bar_bottom_y)),
                false,
            ),
        ];

        let line = printpdf::Line {
            points,
            is_closed: false,
        };

        // Use accent color for the bar
        let bar_color = if self.is_dark_theme {
            // Brighter accent bar for dark theme
            printpdf::Color::Rgb(printpdf::Rgb::new(0.5, 0.6, 0.75, None))
        } else {
            printpdf::Color::Rgb(printpdf::Rgb::new(0.5, 0.55, 0.65, None))
        };
        layer.set_outline_color(bar_color);
        layer.set_outline_thickness(bar_width);
        layer.add_line(line);

        // Paragraph spacing
        self.cursor_y -= padding + 2.0;

        Ok(())
    }

    fn draw_code_block(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        code: &str,
        language: Option<&str>,
        font: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        let code_font_size = self.font_size * 0.9;
        let code_line_height = self.line_height * 0.9;

        // Try syntax highlighting if enabled
        #[cfg(feature = "syntax-highlighting")]
        if self.syntax_highlighting_enabled {
            if let Some(highlighted) = highlight_code_for_pdf(code, language, &self.syntax_theme) {
                return self.draw_highlighted_code_block(
                    doc,
                    &highlighted,
                    code_font_size,
                    code_line_height,
                    font,
                );
            }
        }

        // Fallback: render without highlighting
        let code_indent = 5.0;
        let continuation_indent = code_indent + 4.0;
        let available_width = self.width_mm - (self.margin_mm * 2.0) - code_indent;
        let chars_per_line = (available_width / (code_font_size * 0.4)).max(1.0) as usize;

        for line in code.lines() {
            // Character-based wrapping for code (not word-based)
            if line.len() <= chars_per_line {
                self.ensure_space(doc, code_line_height)?;

                let layer = doc
                    .get_page(self.current_page)
                    .get_layer(self.current_layer);
                layer.set_fill_color(self.code_color());

                layer.use_text(
                    line,
                    code_font_size,
                    Mm(self.margin_mm + code_indent),
                    Mm(self.cursor_y),
                    font,
                );

                self.cursor_y -= code_line_height;
            } else {
                let mut remaining = line;
                let mut first = true;
                while !remaining.is_empty() {
                    let split_at = remaining.len().min(if first {
                        chars_per_line
                    } else {
                        chars_per_line.saturating_sub(2)
                    });
                    // Ensure split is at a char boundary
                    let split_at = floor_to_char_boundary_pdf(remaining, split_at);
                    let (chunk, rest) = remaining.split_at(split_at);
                    remaining = rest;

                    self.ensure_space(doc, code_line_height)?;

                    let layer = doc
                        .get_page(self.current_page)
                        .get_layer(self.current_layer);
                    layer.set_fill_color(self.code_color());

                    let indent = if first {
                        code_indent
                    } else {
                        continuation_indent
                    };
                    layer.use_text(
                        chunk,
                        code_font_size,
                        Mm(self.margin_mm + indent),
                        Mm(self.cursor_y),
                        font,
                    );

                    self.cursor_y -= code_line_height;
                    first = false;
                }
            }
        }

        self.cursor_y -= 2.0;

        Ok(())
    }

    fn draw_list_item(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        text: &str,
        font: &printpdf::IndirectFontRef,
        marker: &str,
    ) -> Result<(), PdfError> {
        let indent = self.list_depth.saturating_sub(1) as f32 * 5.0;
        let marker_width = (marker.chars().count() as f32 + 1.0) * self.font_size * 0.4;
        let text_indent = indent + marker_width;

        // Word wrap list item text
        let available_width = self.width_mm - (self.margin_mm * 2.0) - text_indent;
        let chars_per_line = (available_width / (self.font_size * 0.4)).max(1.0) as usize;

        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            if current_line.len() + word.len() + 1 > chars_per_line && !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        for (i, line) in lines.iter().enumerate() {
            self.ensure_space(doc, self.line_height)?;

            let layer = doc
                .get_page(self.current_page)
                .get_layer(self.current_layer);

            // Apply text color based on theme
            layer.set_fill_color(self.text_color());

            if i == 0 {
                // First line includes marker
                layer.use_text(
                    format!("{} {}", marker, line),
                    self.font_size,
                    Mm(self.margin_mm + indent),
                    Mm(self.cursor_y),
                    font,
                );
            } else {
                // Continuation lines indented to align with text start
                layer.use_text(
                    line,
                    self.font_size,
                    Mm(self.margin_mm + text_indent),
                    Mm(self.cursor_y),
                    font,
                );
            }

            self.cursor_y -= self.line_height;
        }

        Ok(())
    }

    fn draw_table(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        font: &printpdf::IndirectFontRef,
        font_bold: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        let num_cols = self
            .table_header
            .len()
            .max(self.table_rows.iter().map(Vec::len).max().unwrap_or(0));
        if num_cols == 0 {
            return Ok(());
        }

        let table_width = self.width_mm - (self.margin_mm * 2.0);
        let col_width = table_width / num_cols as f32;
        let chars_per_line = ((col_width - 4.0) / (self.font_size * 0.4)).max(1.0) as usize;

        if !self.table_header.is_empty() {
            let header = self.table_header.clone();
            self.draw_table_row(
                doc,
                &header,
                TableRowLayout {
                    num_cols,
                    col_width,
                    chars_per_line,
                    is_header: true,
                },
                font_bold,
            )?;
        }

        for row in self.table_rows.clone() {
            self.draw_table_row(
                doc,
                &row,
                TableRowLayout {
                    num_cols,
                    col_width,
                    chars_per_line,
                    is_header: false,
                },
                font,
            )?;
        }

        self.cursor_y -= 4.0;
        Ok(())
    }

    fn draw_table_row(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        row: &[String],
        layout: TableRowLayout,
        font: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        let cell_padding = 2.0;
        let wrapped_cells: Vec<Vec<String>> = (0..layout.num_cols)
            .map(|col| {
                wrap_text_for_pdf(
                    row.get(col).map(|cell| cell.as_str()).unwrap_or(""),
                    layout.chars_per_line,
                )
            })
            .collect();

        let max_lines = wrapped_cells
            .iter()
            .map(|lines| lines.len().max(1))
            .max()
            .unwrap_or(1);
        let row_height = max_lines as f32 * self.line_height + cell_padding * 2.0;
        self.ensure_space(doc, row_height + 1.0)?;

        let row_top = self.cursor_y;
        let row_bottom = self.cursor_y - row_height;

        for (col, lines) in wrapped_cells.iter().enumerate() {
            let x = self.margin_mm + col as f32 * layout.col_width;
            let layer = doc
                .get_page(self.current_page)
                .get_layer(self.current_layer);
            let border_color = if self.is_dark_theme {
                printpdf::Color::Rgb(printpdf::Rgb::new(0.42, 0.47, 0.56, None))
            } else {
                printpdf::Color::Rgb(printpdf::Rgb::new(0.75, 0.78, 0.82, None))
            };
            layer.set_outline_color(border_color);
            layer.set_outline_thickness(0.3);
            let border = printpdf::Line {
                points: vec![
                    (printpdf::Point::new(Mm(x), Mm(row_top)), false),
                    (
                        printpdf::Point::new(Mm(x + layout.col_width), Mm(row_top)),
                        false,
                    ),
                    (
                        printpdf::Point::new(Mm(x + layout.col_width), Mm(row_bottom)),
                        false,
                    ),
                    (printpdf::Point::new(Mm(x), Mm(row_bottom)), false),
                ],
                is_closed: true,
            };
            layer.add_line(border);
            layer.set_fill_color(if layout.is_header {
                self.heading_color()
            } else {
                self.text_color()
            });

            for (line_idx, line) in lines.iter().enumerate() {
                let text_y = row_top - cell_padding - line_idx as f32 * self.line_height;
                layer.use_text(line, self.font_size, Mm(x + cell_padding), Mm(text_y), font);
            }
        }

        self.cursor_y -= row_height;
        Ok(())
    }

    fn draw_horizontal_rule(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
    ) -> Result<(), PdfError> {
        self.ensure_space(doc, 5.0)?;

        self.cursor_y -= 2.0;

        let layer = doc
            .get_page(self.current_page)
            .get_layer(self.current_layer);

        let points = vec![
            (
                printpdf::Point::new(Mm(self.margin_mm), Mm(self.cursor_y)),
                false,
            ),
            (
                printpdf::Point::new(Mm(self.width_mm - self.margin_mm), Mm(self.cursor_y)),
                false,
            ),
        ];

        let line = printpdf::Line {
            points,
            is_closed: false,
        };

        layer.add_line(line);

        self.cursor_y -= 3.0;

        Ok(())
    }

    /// Draw an image at the current cursor position
    fn draw_image(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        url: &str,
        title: &str,
    ) -> Result<(), PdfError> {
        // Skip remote URLs - only support local images
        if url.starts_with("http://") || url.starts_with("https://") {
            log::debug!("Skipping remote image in PDF (not supported): {}", url);
            return Ok(());
        }

        // Resolve the image path
        let image_path = self.resolve_image_path(url);

        let Some(path) = image_path else {
            log::warn!("Could not resolve image path: {}", url);
            return Ok(());
        };

        if !path.exists() {
            log::warn!("Image file not found: {}", path.display());
            return Ok(());
        }

        // Try to load and embed the image
        match self.load_and_embed_image(doc, &path) {
            Ok(()) => {
                // Add caption if there's a title
                if !title.is_empty() {
                    self.cursor_y -= 2.0;
                }
            }
            Err(e) => {
                log::warn!("Failed to embed image {}: {}", path.display(), e);
            }
        }

        Ok(())
    }

    /// Resolve a relative image path to an absolute path
    fn resolve_image_path(&self, url: &str) -> Option<PathBuf> {
        resolve_pdf_image_path(self.base_path.as_deref(), url)
    }

    /// Load and embed an image into the PDF
    fn load_and_embed_image(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        path: &Path,
    ) -> Result<(), PdfError> {
        use std::io::Cursor;

        // Read the image file
        let image_data = std::fs::read(path)
            .map_err(|e| PdfError::Io(format!("Failed to read image: {}", e)))?;

        // Try to create an image decoder based on file extension
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        let cursor = Cursor::new(&image_data);

        // Create decoder based on extension and try to create Image
        // Use printpdf::image_crate which is the internal re-export of the image crate
        // that printpdf uses, avoiding version mismatch issues
        use printpdf::image_crate::{codecs, ImageDecoder};

        let (image, width, height) = match extension.as_str() {
            "png" => {
                let decoder = codecs::png::PngDecoder::new(cursor)
                    .map_err(|e| PdfError::Io(format!("Failed to decode PNG: {}", e)))?;
                let (w, h) = decoder.dimensions();
                let img = Image::try_from(decoder)
                    .map_err(|e| PdfError::Io(format!("Failed to create image: {:?}", e)))?;
                (img, w, h)
            }
            "jpg" | "jpeg" => {
                let decoder = codecs::jpeg::JpegDecoder::new(cursor)
                    .map_err(|e| PdfError::Io(format!("Failed to decode JPEG: {}", e)))?;
                let (w, h) = decoder.dimensions();
                let img = Image::try_from(decoder)
                    .map_err(|e| PdfError::Io(format!("Failed to create image: {:?}", e)))?;
                (img, w, h)
            }
            "bmp" => {
                let decoder = codecs::bmp::BmpDecoder::new(cursor)
                    .map_err(|e| PdfError::Io(format!("Failed to decode BMP: {}", e)))?;
                let (w, h) = decoder.dimensions();
                let img = Image::try_from(decoder)
                    .map_err(|e| PdfError::Io(format!("Failed to create image: {:?}", e)))?;
                (img, w, h)
            }
            "gif" => {
                let decoder = codecs::gif::GifDecoder::new(cursor)
                    .map_err(|e| PdfError::Io(format!("Failed to decode GIF: {}", e)))?;
                let (w, h) = decoder.dimensions();
                let img = Image::try_from(decoder)
                    .map_err(|e| PdfError::Io(format!("Failed to create image: {:?}", e)))?;
                (img, w, h)
            }
            _ => {
                return Err(PdfError::Io(format!(
                    "Unsupported image format: {}",
                    extension
                )));
            }
        };

        // Calculate image dimensions in mm (assuming 150 DPI for reasonable size)
        let dpi = 150.0;
        let width_mm = (width as f32 / dpi) * 25.4;
        let height_mm = (height as f32 / dpi) * 25.4;

        // Scale to fit page width if needed
        let max_width = self.width_mm - (2.0 * self.margin_mm);
        let scale = if width_mm > max_width {
            max_width / width_mm
        } else {
            1.0
        };
        let final_height_mm = height_mm * scale;

        // Ensure we have enough space for the image
        self.ensure_space(doc, final_height_mm + 5.0)?;

        // Get the current layer
        let layer = doc
            .get_page(self.current_page)
            .get_layer(self.current_layer);

        // Calculate position (images are positioned from bottom-left)
        let x = self.margin_mm;
        let y = self.cursor_y - final_height_mm;

        // Add image to layer with transform
        image.add_to_layer(
            layer,
            ImageTransform {
                translate_x: Some(Mm(x)),
                translate_y: Some(Mm(y)),
                dpi: Some(dpi / scale), // Adjust DPI for scaling
                ..Default::default()
            },
        );

        // Move cursor down past the image
        self.cursor_y -= final_height_mm + 5.0;

        Ok(())
    }
}

/// A highlighted token for PDF rendering
#[cfg(feature = "syntax-highlighting")]
struct HighlightedToken {
    text: String,
    color: (f32, f32, f32), // RGB 0.0-1.0
}

/// A line of highlighted tokens
#[cfg(feature = "syntax-highlighting")]
struct HighlightedLine {
    tokens: Vec<HighlightedToken>,
}

/// Highlighted code ready for PDF rendering
#[cfg(feature = "syntax-highlighting")]
/// Find the largest byte index <= `pos` that is a valid char boundary in `s`.
fn floor_to_char_boundary_pdf(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        return s.len();
    }
    let mut idx = pos;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

struct HighlightedCode {
    lines: Vec<HighlightedLine>,
}

/// Highlight code for PDF export using syntect
#[cfg(feature = "syntax-highlighting")]
fn highlight_code_for_pdf(
    code: &str,
    language: Option<&str>,
    theme_name: &str,
) -> Option<HighlightedCode> {
    use std::sync::OnceLock;
    use syntect::easy::HighlightLines;
    use syntect::highlighting::ThemeSet;
    use syntect::parsing::SyntaxSet;

    // Lazy-load syntax and theme sets
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

    let ss = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = THEME_SET.get_or_init(ThemeSet::load_defaults);

    // Find syntax for the language
    let syntax = language
        .and_then(|lang| ss.find_syntax_by_token(lang))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    // Use theme from config, falling back to base16-ocean.dark
    let theme = ts
        .themes
        .get(theme_name)
        .or_else(|| ts.themes.get("base16-ocean.dark"))
        .or_else(|| ts.themes.values().next())?;

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut lines = Vec::new();

    for line in code.lines() {
        let Ok(ranges) = highlighter.highlight_line(line, ss) else {
            return None;
        };

        let tokens: Vec<HighlightedToken> = ranges
            .into_iter()
            .map(|(style, text)| {
                let color = (
                    style.foreground.r as f32 / 255.0,
                    style.foreground.g as f32 / 255.0,
                    style.foreground.b as f32 / 255.0,
                );
                HighlightedToken {
                    text: text.to_string(),
                    color,
                }
            })
            .collect();

        lines.push(HighlightedLine { tokens });
    }

    Some(HighlightedCode { lines })
}

impl PdfExporter {
    /// Draw a code block with syntax highlighting
    #[cfg(feature = "syntax-highlighting")]
    fn draw_highlighted_code_block(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        highlighted: &HighlightedCode,
        code_font_size: f32,
        code_line_height: f32,
        font: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        // Approximate character width for monospace font at this size
        // Courier at 12pt is roughly 7.2pt per character, scale proportionally
        let char_width_mm = code_font_size * 0.35; // Approximate mm per character

        for line in &highlighted.lines {
            self.ensure_space(doc, code_line_height)?;

            let layer = doc
                .get_page(self.current_page)
                .get_layer(self.current_layer);
            let mut x_offset = self.margin_mm + 5.0;

            for token in &line.tokens {
                // Set the token's color
                let color = printpdf::Color::Rgb(printpdf::Rgb::new(
                    token.color.0,
                    token.color.1,
                    token.color.2,
                    None,
                ));
                layer.set_fill_color(color);

                // Render the token
                layer.use_text(
                    &token.text,
                    code_font_size,
                    Mm(x_offset),
                    Mm(self.cursor_y),
                    font,
                );

                // Advance x position based on character count
                x_offset += token.text.len() as f32 * char_width_mm;
            }

            self.cursor_y -= code_line_height;
        }

        self.cursor_y -= 2.0;

        Ok(())
    }
}

fn next_list_marker(list_stack: &mut [ListState], task_marker: Option<bool>) -> String {
    if let Some(checked) = task_marker {
        return if checked {
            "[x]".to_string()
        } else {
            "[ ]".to_string()
        };
    }

    match list_stack
        .last_mut()
        .and_then(|state| state.next_number.as_mut())
    {
        Some(next_number) => {
            let marker = format!("{}.", *next_number);
            *next_number += 1;
            marker
        }
        None => "-".to_string(),
    }
}

fn wrap_text_for_pdf(text: &str, chars_per_line: usize) -> Vec<String> {
    if text.trim().is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if word.chars().count() > chars_per_line {
            if !current_line.is_empty() {
                lines.push(std::mem::take(&mut current_line));
            }

            let mut remaining = word;
            while !remaining.is_empty() {
                let mut split_at = remaining.len();
                for (char_count, (idx, ch)) in remaining.char_indices().enumerate() {
                    if char_count == chars_per_line {
                        split_at = idx;
                        break;
                    }
                    split_at = idx + ch.len_utf8();
                }

                let split_at = floor_to_char_boundary_pdf(remaining, split_at);
                let (chunk, rest) = remaining.split_at(split_at);
                lines.push(chunk.to_string());
                remaining = rest;
            }
            continue;
        }

        let pending_len = if current_line.is_empty() {
            word.chars().count()
        } else {
            current_line.chars().count() + 1 + word.chars().count()
        };

        if pending_len > chars_per_line && !current_line.is_empty() {
            lines.push(std::mem::take(&mut current_line));
        }

        if !current_line.is_empty() {
            current_line.push(' ');
        }
        current_line.push_str(word);
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

fn resolve_pdf_image_path(base_path: Option<&Path>, url: &str) -> Option<PathBuf> {
    let path = PathBuf::from(url);

    if path.is_absolute() {
        return path.exists().then(|| path.canonicalize().ok()).flatten();
    }

    if let Some(base) = base_path {
        let full_path = base.join(&path);
        if full_path.exists() {
            if let Ok(canonical) = full_path.canonicalize() {
                if let Ok(canonical_base) = base.canonicalize() {
                    if canonical.starts_with(&canonical_base) {
                        return Some(canonical);
                    }
                    log::warn!(
                        "PDF image path traversal blocked: {:?} is outside {:?}",
                        url,
                        base
                    );
                    return None;
                }
                return Some(canonical);
            }
        }
    }

    if path.exists() {
        return path.canonicalize().ok();
    }

    None
}

/// PDF export errors
#[derive(Debug)]
pub enum PdfError {
    Io(String),
    Pdf(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::Io(e) => write!(f, "IO error: {}", e),
            PdfError::Pdf(e) => write!(f, "PDF error: {}", e),
        }
    }
}

impl std::error::Error for PdfError {}

impl From<printpdf::Error> for PdfError {
    fn from(e: printpdf::Error) -> Self {
        PdfError::Pdf(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_page_dimensions() {
        // A4 should be 210x297mm
        assert!((A4_WIDTH_MM - 210.0).abs() < 0.1);
        assert!((A4_HEIGHT_MM - 297.0).abs() < 0.1);
    }

    #[test]
    fn test_next_list_marker_handles_ordered_and_tasks() {
        let mut ordered = vec![ListState {
            next_number: Some(2),
        }];
        assert_eq!(next_list_marker(&mut ordered, None), "2.");
        assert_eq!(next_list_marker(&mut ordered, None), "3.");

        let mut unordered = vec![ListState { next_number: None }];
        assert_eq!(next_list_marker(&mut unordered, Some(true)), "[x]");
        assert_eq!(next_list_marker(&mut unordered, Some(false)), "[ ]");
        assert_eq!(next_list_marker(&mut unordered, None), "-");
    }

    #[test]
    fn test_wrap_text_for_pdf_splits_long_words() {
        let lines = wrap_text_for_pdf("supercalifragilisticexpialidocious", 8);
        assert!(lines.iter().all(|line| line.chars().count() <= 8));
        assert_eq!(lines.concat(), "supercalifragilisticexpialidocious");
    }

    #[test]
    fn test_resolve_pdf_image_path_blocks_traversal() {
        let dir = tempdir().unwrap();
        let docs = dir.path().join("docs");
        let assets = dir.path().join("assets");
        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("image.png"), b"png").unwrap();

        let resolved = resolve_pdf_image_path(Some(&docs), "../assets/image.png");
        assert!(resolved.is_none());
    }
}
