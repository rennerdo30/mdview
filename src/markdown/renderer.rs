//! Markdown renderer - converts pulldown-cmark events to egui widgets

use egui::{Color32, RichText, TextStyle, Ui, Vec2};
use pulldown_cmark::{Event, Tag, TagEnd, HeadingLevel, CodeBlockKind, Alignment};

use crate::annotations::AnnotationStore;
use crate::config::defaults::heading_size_multiplier;
use crate::config::Config;

/// Markdown renderer that converts events to egui widgets
pub struct MarkdownRenderer {
    /// Current text buffer for accumulating inline content
    text_buffer: String,

    /// Current heading level (0 = not in heading)
    heading_level: usize,

    /// Whether we're in a code block
    in_code_block: bool,

    /// Code block language
    code_language: Option<String>,

    /// Code block content
    code_content: String,

    /// List nesting level
    list_depth: usize,

    /// Current list item number (for ordered lists)
    list_number: Option<u64>,

    /// Whether we're in emphasis (italic)
    in_emphasis: bool,

    /// Whether we're in strong (bold)
    in_strong: bool,

    /// Whether we're in strikethrough
    in_strikethrough: bool,

    /// Current link URL
    current_link: Option<String>,

    /// Whether we're in a blockquote
    in_blockquote: bool,

    /// Table state
    in_table: bool,
    table_alignments: Vec<Alignment>,
    table_row: Vec<String>,
    in_table_head: bool,

    /// Task list item state
    task_list_marker: Option<bool>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            text_buffer: String::new(),
            heading_level: 0,
            in_code_block: false,
            code_language: None,
            code_content: String::new(),
            list_depth: 0,
            list_number: None,
            in_emphasis: false,
            in_strong: false,
            in_strikethrough: false,
            current_link: None,
            in_blockquote: false,
            in_table: false,
            table_alignments: Vec::new(),
            table_row: Vec::new(),
            in_table_head: false,
            task_list_marker: None,
        }
    }

    /// Render markdown events to the UI
    pub fn render(
        &mut self,
        ui: &mut Ui,
        events: &[Event<'_>],
        _annotations: &AnnotationStore,
        heading_positions: &mut Vec<f32>,
        config: &Config,
    ) {
        // Reset state
        self.reset();

        let base_font_size = config.theme.fonts.size;
        let spacing = &config.theme.spacing;

        for event in events {
            match event {
                Event::Start(tag) => self.handle_start_tag(tag, ui, heading_positions),
                Event::End(tag) => self.handle_end_tag(tag, ui, base_font_size, spacing, config),
                Event::Text(text) => self.handle_text(text),
                Event::Code(code) => self.handle_inline_code(code),
                Event::SoftBreak => self.text_buffer.push(' '),
                Event::HardBreak => self.text_buffer.push('\n'),
                Event::Rule => self.render_horizontal_rule(ui),
                Event::TaskListMarker(checked) => self.task_list_marker = Some(*checked),
                Event::FootnoteReference(name) => {
                    self.text_buffer.push_str(&format!("[^{}]", name));
                }
                _ => {}
            }
        }
    }

    fn reset(&mut self) {
        self.text_buffer.clear();
        self.heading_level = 0;
        self.in_code_block = false;
        self.code_language = None;
        self.code_content.clear();
        self.list_depth = 0;
        self.list_number = None;
        self.in_emphasis = false;
        self.in_strong = false;
        self.in_strikethrough = false;
        self.current_link = None;
        self.in_blockquote = false;
        self.in_table = false;
        self.table_alignments.clear();
        self.table_row.clear();
        self.in_table_head = false;
        self.task_list_marker = None;
    }

    fn handle_start_tag(&mut self, tag: &Tag<'_>, ui: &mut Ui, heading_positions: &mut Vec<f32>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.heading_level = super::parser::heading_level_to_usize(*level);
                // Record heading position for TOC navigation
                heading_positions.push(ui.cursor().top());
            }
            Tag::Paragraph => {}
            Tag::BlockQuote(_) => {
                self.in_blockquote = true;
            }
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_language = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => {
                        Some(lang.to_string())
                    }
                    _ => None,
                };
            }
            Tag::List(start) => {
                self.list_depth += 1;
                self.list_number = *start;
            }
            Tag::Item => {}
            Tag::Emphasis => {
                self.in_emphasis = true;
            }
            Tag::Strong => {
                self.in_strong = true;
            }
            Tag::Strikethrough => {
                self.in_strikethrough = true;
            }
            Tag::Link { dest_url, .. } => {
                self.current_link = Some(dest_url.to_string());
            }
            Tag::Image { dest_url, title, .. } => {
                // Handle image rendering
                self.render_image(ui, dest_url, title);
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
            Tag::TableCell => {}
            _ => {}
        }
    }

    fn handle_end_tag(
        &mut self,
        tag: &TagEnd,
        ui: &mut Ui,
        base_font_size: f32,
        spacing: &crate::config::schema::SpacingConfig,
        config: &Config,
    ) {
        match tag {
            TagEnd::Heading(_level) => {
                self.render_heading(ui, base_font_size, spacing);
                self.heading_level = 0;
            }
            TagEnd::Paragraph => {
                if self.in_blockquote {
                    self.render_blockquote(ui, base_font_size);
                } else {
                    self.render_paragraph(ui, base_font_size, spacing);
                }
            }
            TagEnd::BlockQuote(_) => {
                self.in_blockquote = false;
            }
            TagEnd::CodeBlock => {
                self.render_code_block(ui, config);
                self.in_code_block = false;
                self.code_language = None;
                self.code_content.clear();
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                if self.list_depth == 0 {
                    self.list_number = None;
                }
            }
            TagEnd::Item => {
                self.render_list_item(ui, base_font_size, spacing);
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
            TagEnd::Link => {
                self.render_link(ui, base_font_size);
                self.current_link = None;
            }
            TagEnd::Table => {
                self.in_table = false;
                self.table_alignments.clear();
            }
            TagEnd::TableHead => {
                self.in_table_head = false;
            }
            TagEnd::TableRow => {
                // Row rendering is handled by TableCell
            }
            TagEnd::TableCell => {
                self.table_row.push(std::mem::take(&mut self.text_buffer));
            }
            _ => {}
        }
    }

    fn handle_text(&mut self, text: &pulldown_cmark::CowStr<'_>) {
        if self.in_code_block {
            self.code_content.push_str(text);
        } else {
            self.text_buffer.push_str(text);
        }
    }

    fn handle_inline_code(&mut self, code: &pulldown_cmark::CowStr<'_>) {
        // Store inline code with markers for later rendering
        self.text_buffer.push_str("\x00CODE:");
        self.text_buffer.push_str(code);
        self.text_buffer.push_str("\x00");
    }

    fn render_heading(
        &mut self,
        ui: &mut Ui,
        base_font_size: f32,
        spacing: &crate::config::schema::SpacingConfig,
    ) {
        let text = std::mem::take(&mut self.text_buffer);
        if text.is_empty() {
            return;
        }

        ui.add_space(spacing.heading_top);

        let size_multiplier = heading_size_multiplier(self.heading_level);
        let font_size = base_font_size * size_multiplier;

        let rich_text = RichText::new(&text)
            .size(font_size)
            .strong();

        ui.label(rich_text);

        ui.add_space(spacing.heading_bottom);
    }

    fn render_paragraph(
        &mut self,
        ui: &mut Ui,
        base_font_size: f32,
        spacing: &crate::config::schema::SpacingConfig,
    ) {
        let text = std::mem::take(&mut self.text_buffer);
        if text.is_empty() {
            return;
        }

        // Parse and render mixed content (text + inline code)
        self.render_mixed_content(ui, &text, base_font_size);

        ui.add_space(spacing.paragraph);
    }

    fn render_mixed_content(&self, ui: &mut Ui, text: &str, base_font_size: f32) {
        // Split on inline code markers and render appropriately
        let parts: Vec<&str> = text.split('\x00').collect();

        ui.horizontal_wrapped(|ui| {
            for part in parts {
                if part.starts_with("CODE:") {
                    let code = &part[5..];
                    let code_text = RichText::new(code)
                        .size(base_font_size * 0.9)
                        .monospace()
                        .background_color(Color32::from_gray(50));
                    ui.label(code_text);
                } else if !part.is_empty() {
                    ui.label(RichText::new(part).size(base_font_size));
                }
            }
        });
    }

    fn render_blockquote(&mut self, ui: &mut Ui, base_font_size: f32) {
        let text = std::mem::take(&mut self.text_buffer);
        if text.is_empty() {
            return;
        }

        ui.horizontal(|ui| {
            // Vertical bar for blockquote
            let rect = ui.available_rect_before_wrap();
            let bar_rect = egui::Rect::from_min_size(
                rect.min,
                Vec2::new(3.0, ui.spacing().interact_size.y),
            );
            ui.painter().rect_filled(bar_rect, 0.0, Color32::from_gray(100));

            ui.add_space(12.0);

            let quote_text = RichText::new(&text)
                .size(base_font_size)
                .italics()
                .color(Color32::from_gray(180));

            ui.label(quote_text);
        });

        ui.add_space(8.0);
    }

    fn render_code_block(&mut self, ui: &mut Ui, config: &Config) {
        let code = std::mem::take(&mut self.code_content);
        if code.is_empty() {
            return;
        }

        let padding = config.theme.spacing.code_padding;

        egui::Frame::none()
            .fill(Color32::from_gray(30))
            .inner_margin(padding)
            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
            .rounding(4.0)
            .show(ui, |ui| {
                // Show language label if present
                if let Some(lang) = &self.code_language {
                    ui.label(
                        RichText::new(lang)
                            .small()
                            .color(Color32::from_gray(120)),
                    );
                    ui.add_space(4.0);
                }

                #[cfg(feature = "syntax-highlighting")]
                {
                    if config.markdown.syntax_highlighting {
                        if let Some(highlighted) =
                            highlight_code(&code, self.code_language.as_deref())
                        {
                            ui.label(highlighted);
                            return;
                        }
                    }
                }

                // Fallback: plain monospace text
                ui.label(
                    RichText::new(&code)
                        .monospace()
                        .color(Color32::from_rgb(206, 145, 120)),
                );
            });

        ui.add_space(8.0);
    }

    fn render_list_item(
        &mut self,
        ui: &mut Ui,
        base_font_size: f32,
        spacing: &crate::config::schema::SpacingConfig,
    ) {
        let text = std::mem::take(&mut self.text_buffer);
        let indent = self.list_depth as f32 * spacing.list_indent;

        ui.horizontal(|ui| {
            ui.add_space(indent);

            // Render bullet or number
            let marker = if let Some(ref mut num) = self.list_number {
                let marker = format!("{}.", num);
                *num += 1;
                marker
            } else if let Some(checked) = self.task_list_marker.take() {
                if checked { "☑" } else { "☐" }.to_string()
            } else {
                "•".to_string()
            };

            ui.label(RichText::new(&marker).size(base_font_size));
            ui.add_space(4.0);

            if !text.is_empty() {
                self.render_mixed_content(ui, &text, base_font_size);
            }
        });
    }

    fn render_link(&mut self, ui: &mut Ui, base_font_size: f32) {
        let text = std::mem::take(&mut self.text_buffer);
        let url = self.current_link.clone().unwrap_or_default();

        if !text.is_empty() {
            let link_text = RichText::new(&text)
                .size(base_font_size)
                .color(Color32::from_rgb(78, 201, 176))
                .underline();

            if ui.link(link_text).clicked() {
                if let Err(e) = open::that(&url) {
                    log::error!("Failed to open link: {}", e);
                }
            }
        }
    }

    fn render_image(&self, ui: &mut Ui, url: &str, title: &str) {
        // For now, just show a placeholder
        // Full image support would require async loading
        ui.horizontal(|ui| {
            ui.label(RichText::new("🖼").size(20.0));
            ui.label(
                RichText::new(if title.is_empty() { url } else { title })
                    .italics()
                    .color(Color32::from_gray(150)),
            );
        });
    }

    fn render_horizontal_rule(&self, ui: &mut Ui) {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "syntax-highlighting")]
fn highlight_code(code: &str, language: Option<&str>) -> Option<egui::RichText> {
    use syntect::easy::HighlightLines;
    use syntect::highlighting::{ThemeSet, Style};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = language
        .and_then(|lang| ps.find_syntax_by_token(lang))
        .unwrap_or_else(|| ps.find_syntax_plain_text());

    let theme = &ts.themes["base16-ocean.dark"];
    let mut h = HighlightLines::new(syntax, theme);

    // For simplicity, return None and let fallback handle it
    // Full implementation would build a LayoutJob with colored spans
    None
}

#[cfg(not(feature = "syntax-highlighting"))]
fn highlight_code(_code: &str, _language: Option<&str>) -> Option<egui::RichText> {
    None
}
