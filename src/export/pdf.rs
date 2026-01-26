//! PDF export using printpdf

use std::path::Path;
use std::io::BufWriter;

use printpdf::{BuiltinFont, Mm, PdfDocument};
use pulldown_cmark::{Event, Tag, TagEnd, HeadingLevel};

use crate::config::Config;
use crate::config::defaults::heading_size_multiplier;

/// A4 page dimensions in mm
const A4_WIDTH_MM: f32 = 210.0;
const A4_HEIGHT_MM: f32 = 297.0;

/// Letter page dimensions in mm
const LETTER_WIDTH_MM: f32 = 215.9;
const LETTER_HEIGHT_MM: f32 = 279.4;

/// Export markdown events to PDF
pub fn export_to_pdf(
    events: &[Event<'_>],
    output_path: &Path,
    config: &Config,
) -> Result<(), PdfError> {
    let (width_mm, height_mm) = match config.export.page_size.to_lowercase().as_str() {
        "letter" => (LETTER_WIDTH_MM, LETTER_HEIGHT_MM),
        _ => (A4_WIDTH_MM, A4_HEIGHT_MM),
    };

    let margin_mm = config.export.margin as f32;

    // Determine colors based on pdf_theme
    let is_dark_theme = config.export.pdf_theme.to_lowercase() == "dark";

    let (doc, page1, layer1) = PdfDocument::new(
        "Markdown Export",
        Mm(width_mm),
        Mm(height_mm),
        "Layer 1",
    );

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
        text_buffer: String::new(),
        heading_level: 0,
        list_depth: 0,
        in_emphasis: false,
        in_strong: false,
        in_strikethrough: false,
        in_blockquote: false,
        is_dark_theme,
        custom_heading_color: config.theme.colors.heading.clone(),
        custom_code_color: config.theme.colors.code_text.clone(),
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
    text_buffer: String,
    heading_level: usize,
    list_depth: usize,
    in_emphasis: bool,
    in_strong: bool,
    in_strikethrough: bool,
    in_blockquote: bool,
    /// Theme flag for PDF color theming
    is_dark_theme: bool,
    /// Custom heading color from config (hex)
    custom_heading_color: Option<String>,
    /// Custom code color from config (hex)
    custom_code_color: Option<String>,
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
        let layer = doc.get_page(self.current_page).get_layer(self.current_layer);
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

            let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

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
            Tag::CodeBlock(_) => {
                self.in_code_block = true;
                self.code_content.clear();
            }
            Tag::List(_) => {
                self.list_depth += 1;
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
                if !code.is_empty() {
                    self.draw_code_block(doc, &code, font_mono)?;
                }
                self.in_code_block = false;
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
            }
            TagEnd::Item => {
                let text = std::mem::take(&mut self.text_buffer);
                if !text.is_empty() {
                    self.draw_list_item(doc, &text, font)?;
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
            _ => {}
        }

        Ok(())
    }

    fn ensure_space(&mut self, doc: &printpdf::PdfDocumentReference, needed: f32) -> Result<(), PdfError> {
        if self.cursor_y - needed < self.margin_mm {
            self.new_page(doc)?;
        }
        Ok(())
    }

    fn new_page(&mut self, doc: &printpdf::PdfDocumentReference) -> Result<(), PdfError> {
        let (page, layer) = doc.add_page(
            Mm(self.width_mm),
            Mm(self.height_mm),
            "Layer 1",
        );
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
        let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

        // Dark theme background color (dark blue-gray)
        let bg_color = printpdf::Color::Rgb(printpdf::Rgb::new(0.12, 0.14, 0.18, None));
        layer.set_fill_color(bg_color.clone());
        layer.set_outline_color(bg_color);

        // Draw full-page rectangle
        let points = vec![
            (printpdf::Point::new(Mm(0.0), Mm(0.0)), false),
            (printpdf::Point::new(Mm(self.width_mm), Mm(0.0)), false),
            (printpdf::Point::new(Mm(self.width_mm), Mm(self.height_mm)), false),
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

        self.ensure_space(doc, line_height + 3.0)?;

        // Add spacing before heading
        self.cursor_y -= 3.0;

        let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

        // Apply heading color based on theme
        layer.set_fill_color(self.heading_color());

        layer.use_text(
            text,
            font_size,
            Mm(self.margin_mm),
            Mm(self.cursor_y),
            font,
        );

        self.cursor_y -= line_height + 2.0;

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
            if current_line.len() + word.len() + 1 > chars_per_line
                && !current_line.is_empty() {
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

            let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

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
            let layer = doc.get_page(self.current_page).get_layer(self.current_layer);
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
                (printpdf::Point::new(Mm(bg_left_x + bg_width), Mm(bg_top_y)), false),
                (printpdf::Point::new(Mm(bg_left_x + bg_width), Mm(bg_bottom_y)), false),
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
            let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

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
        let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

        let points = vec![
            (printpdf::Point::new(Mm(self.margin_mm + bar_width / 2.0), Mm(bar_top_y)), false),
            (printpdf::Point::new(Mm(self.margin_mm + bar_width / 2.0), Mm(bar_bottom_y)), false),
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
        font: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        let code_font_size = self.font_size * 0.9;
        let code_line_height = self.line_height * 0.9;

        for line in code.lines() {
            self.ensure_space(doc, code_line_height)?;

            let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

            // Apply code color based on theme
            layer.set_fill_color(self.code_color());

            layer.use_text(
                line,
                code_font_size,
                Mm(self.margin_mm + 5.0), // Indent code
                Mm(self.cursor_y),
                font,
            );

            self.cursor_y -= code_line_height;
        }

        self.cursor_y -= 2.0;

        Ok(())
    }

    fn draw_list_item(
        &mut self,
        doc: &printpdf::PdfDocumentReference,
        text: &str,
        font: &printpdf::IndirectFontRef,
    ) -> Result<(), PdfError> {
        self.ensure_space(doc, self.line_height)?;

        let indent = self.list_depth as f32 * 5.0;
        let bullet = "- ";

        let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

        // Apply text color based on theme
        layer.set_fill_color(self.text_color());

        layer.use_text(
            format!("{}{}", bullet, text),
            self.font_size,
            Mm(self.margin_mm + indent),
            Mm(self.cursor_y),
            font,
        );

        self.cursor_y -= self.line_height;

        Ok(())
    }

    fn draw_horizontal_rule(&mut self, doc: &printpdf::PdfDocumentReference) -> Result<(), PdfError> {
        self.ensure_space(doc, 5.0)?;

        self.cursor_y -= 2.0;

        let layer = doc.get_page(self.current_page).get_layer(self.current_layer);

        let points = vec![
            (printpdf::Point::new(Mm(self.margin_mm), Mm(self.cursor_y)), false),
            (printpdf::Point::new(Mm(self.width_mm - self.margin_mm), Mm(self.cursor_y)), false),
        ];

        let line = printpdf::Line {
            points,
            is_closed: false,
        };

        layer.add_line(line);

        self.cursor_y -= 3.0;

        Ok(())
    }
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

    #[test]
    fn test_page_dimensions() {
        // A4 should be 210x297mm
        assert!((A4_WIDTH_MM - 210.0).abs() < 0.1);
        assert!((A4_HEIGHT_MM - 297.0).abs() < 0.1);
    }
}
