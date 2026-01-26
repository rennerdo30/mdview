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
    };

    exporter.render_events(&doc, events, &font, &font_bold, &font_mono)?;

    let file = std::fs::File::create(output_path).map_err(|e| PdfError::Io(e.to_string()))?;
    doc.save(&mut BufWriter::new(file))?;

    Ok(())
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
}

impl PdfExporter {
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
                    let f = if self.in_strong { font_bold } else { font };
                    self.draw_paragraph(doc, &text, f)?;
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
        Ok(())
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
            if current_line.len() + word.len() + 1 > chars_per_line {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
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

        layer.use_text(
            &format!("{}{}", bullet, text),
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
