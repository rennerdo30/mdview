//! Markdown renderer - converts pulldown-cmark events to egui widgets

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc;

use egui::{Color32, RichText, TextureHandle, Ui, Vec2};
use pulldown_cmark::{Alignment, Event, Tag, TagEnd, CodeBlockKind};

use crate::annotations::AnnotationStore;
use crate::annotations::model::{Annotation, AnnotationKind};
use crate::config::defaults::heading_size_multiplier;
use crate::config::Config;

/// Pre-built annotation index for efficient O(log n) lookups
/// Instead of O(n) scan per character, we build a sorted list of annotation boundaries
/// Note: Currently the optimization is inlined in render_mixed_content_with_annotations,
/// but this struct is kept for potential future use with larger annotation sets.
#[allow(dead_code)]
struct AnnotationIndex<'a> {
    /// Annotations sorted by start position for binary search
    sorted_annotations: Vec<&'a Annotation>,
}

#[allow(dead_code)]
impl<'a> AnnotationIndex<'a> {
    /// Build an index from annotations in a given range
    fn new(annotations: &'a AnnotationStore, start: usize, end: usize) -> Self {
        let mut sorted: Vec<&'a Annotation> = annotations
            .all()
            .filter(|a| a.overlaps(start, end))
            .collect();
        sorted.sort_by_key(|a| a.start);
        Self { sorted_annotations: sorted }
    }

    /// Get highlight color at position using binary search (O(log n))
    fn get_highlight_color(&self, pos: usize) -> Option<Color32> {
        // Find annotations that contain this position
        // Since annotations are sorted by start, we can use binary search to find candidates
        for ann in &self.sorted_annotations {
            if ann.start > pos {
                break; // No more candidates (sorted by start)
            }
            if ann.kind == AnnotationKind::Highlight && ann.contains(pos) {
                let color = ann.color.as_deref().unwrap_or("#ffeb3b");
                let c = crate::annotations::ui::parse_hex_color(color);
                return Some(Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 80));
            }
        }
        None
    }

    /// Check if there's a note at position (O(log n))
    fn has_note_at(&self, pos: usize) -> bool {
        for ann in &self.sorted_annotations {
            if ann.start > pos {
                break;
            }
            if ann.kind == AnnotationKind::Note && ann.contains(pos) {
                return true;
            }
        }
        false
    }
}

/// Result of an async mermaid render
type MermaidRenderResult = (String, Result<Vec<u8>, String>);

/// Parse a hex color string from config to Color32
fn parse_config_hex_color(hex: &str) -> Color32 {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Color32::from_gray(128);
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
    Color32::from_rgb(r, g, b)
}

/// Check if the current theme is a dark theme
fn is_dark_theme(config: &Config) -> bool {
    !config.general.theme.to_lowercase().eq("light")
}

/// Theme-aware colors for markdown rendering
mod theme_colors {
    use egui::Color32;

    // Code block backgrounds
    pub fn code_block_bg(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(30, 35, 45)
        } else {
            Color32::from_rgb(245, 247, 250)
        }
    }

    pub fn code_block_border(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(60, 70, 90)
        } else {
            Color32::from_rgb(210, 215, 225)
        }
    }

    // Inline code
    pub fn inline_code_text(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(206, 145, 120)
        } else {
            Color32::from_rgb(180, 80, 80)
        }
    }

    // Code block text colors
    pub fn code_text(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(180, 190, 210)
        } else {
            Color32::from_rgb(50, 55, 70)
        }
    }

    pub fn code_line_number(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(100, 110, 130)
        } else {
            Color32::from_rgb(150, 155, 165)
        }
    }

    pub fn code_lang_label(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(255, 154, 162)
        } else {
            Color32::from_rgb(180, 60, 80)
        }
    }

    // Links
    pub fn link_color(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(78, 201, 176)
        } else {
            Color32::from_rgb(0, 120, 150)
        }
    }

    // Blockquote
    pub fn blockquote_bg(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(45, 50, 65)
        } else {
            Color32::from_rgb(240, 245, 250)
        }
    }

    pub fn blockquote_border(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(80, 130, 180)
        } else {
            Color32::from_rgb(100, 150, 200)
        }
    }

    pub fn blockquote_text(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(180, 180, 200)
        } else {
            Color32::from_rgb(70, 75, 90)
        }
    }

    // Table
    pub fn table_header_bg(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(40, 45, 55)
        } else {
            Color32::from_rgb(235, 240, 245)
        }
    }

    pub fn table_border(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(60, 65, 80)
        } else {
            Color32::from_rgb(200, 205, 215)
        }
    }

    pub fn table_row_alt(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(35, 40, 50)
        } else {
            Color32::from_rgb(248, 250, 252)
        }
    }
}

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
    table_rows: Vec<Vec<String>>,
    table_header: Vec<String>,
    in_table_head: bool,

    /// Task list item state
    task_list_marker: Option<bool>,

    /// Base path for resolving relative image paths
    base_path: Option<PathBuf>,

    /// Image texture cache
    image_cache: HashMap<String, TextureHandle>,

    /// Footnote definitions collected during rendering
    footnote_definitions: Vec<(String, String)>,

    /// Current footnote definition name (when inside a footnote)
    current_footnote: Option<String>,

    /// Whether we're inside a footnote definition
    in_footnote_definition: bool,

    /// Counter for footnote reference numbering
    footnote_counter: HashMap<String, usize>,

    /// Current document character offset for annotation tracking
    char_offset: usize,

    /// Target heading index to scroll to (for TOC navigation)
    scroll_target: Option<usize>,

    /// Current heading index counter (for matching scroll target)
    heading_index: usize,

    /// Counter for unique table IDs (prevents egui ID collisions)
    table_count: usize,

    /// Counter for unique code block IDs (prevents egui ID collisions)
    code_block_count: usize,

    /// Cache of mermaid diagrams that failed to render (by hash)
    /// Prevents repeated render attempts that would fail/panic every frame
    mermaid_failed: std::collections::HashSet<String>,

    /// Set of mermaid diagrams currently being rendered asynchronously
    mermaid_pending: std::collections::HashSet<String>,

    /// Channel sender for spawning async mermaid renders
    mermaid_sender: mpsc::Sender<MermaidRenderResult>,

    /// Channel receiver for completed async mermaid renders
    mermaid_receiver: mpsc::Receiver<MermaidRenderResult>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
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
            table_rows: Vec::new(),
            table_header: Vec::new(),
            in_table_head: false,
            task_list_marker: None,
            base_path: None,
            image_cache: HashMap::new(),
            footnote_definitions: Vec::new(),
            current_footnote: None,
            in_footnote_definition: false,
            footnote_counter: HashMap::new(),
            char_offset: 0,
            scroll_target: None,
            heading_index: 0,
            table_count: 0,
            code_block_count: 0,
            mermaid_failed: std::collections::HashSet::new(),
            mermaid_pending: std::collections::HashSet::new(),
            mermaid_sender: sender,
            mermaid_receiver: receiver,
        }
    }

    /// Set the base path for resolving relative image URLs
    pub fn set_base_path(&mut self, path: Option<PathBuf>) {
        self.base_path = path;
    }

    /// Clear the image cache (call when document changes)
    pub fn clear_image_cache(&mut self) {
        self.image_cache.clear();
        self.mermaid_failed.clear();
        self.mermaid_pending.clear();
    }

    /// Get the current character offset (position in document after last rendered element)
    pub fn current_char_offset(&self) -> usize {
        self.char_offset
    }

    /// Clear only the mermaid caches (call to retry failed renders)
    /// Useful when user installs mermaid-cli and wants to retry without reloading document
    #[allow(dead_code)]
    pub fn clear_mermaid_cache(&mut self) {
        // Remove only mermaid entries from image cache
        self.image_cache.retain(|k, _| !k.starts_with("mermaid_"));
        self.mermaid_failed.clear();
        self.mermaid_pending.clear();
    }

    /// Poll for completed async mermaid renders
    /// Call this at the start of each frame to process completed renders
    /// Returns true if any renders completed (UI should repaint)
    pub fn poll_mermaid_renders(&mut self, ctx: &egui::Context) -> bool {
        let mut any_completed = false;

        // Process all available completed renders
        while let Ok((cache_key, result)) = self.mermaid_receiver.try_recv() {
            self.mermaid_pending.remove(&cache_key);
            any_completed = true;

            match result {
                Ok(png_bytes) => {
                    // Load texture from PNG bytes
                    if let Ok(img) = image::load_from_memory(&png_bytes) {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                        let texture = ctx.load_texture(
                            &cache_key,
                            color_image,
                            egui::TextureOptions::LINEAR,
                        );
                        self.image_cache.insert(cache_key.clone(), texture);
                        log::debug!("Async mermaid render completed: {}", cache_key);
                    } else {
                        log::warn!("Failed to decode async mermaid PNG: {}", cache_key);
                        self.mermaid_failed.insert(cache_key);
                    }
                }
                Err(e) => {
                    log::debug!("Async mermaid render failed: {} - {}", cache_key, e);
                    self.mermaid_failed.insert(cache_key);
                }
            }
        }

        if any_completed {
            ctx.request_repaint();
        }

        any_completed
    }

    /// Render markdown events to the UI
    /// Render with an optional scroll target (heading index to scroll to)
    pub fn render_with_scroll_target(
        &mut self,
        ui: &mut Ui,
        events: &[Event<'_>],
        annotations: &AnnotationStore,
        heading_positions: &mut Vec<f32>,
        config: &Config,
        scroll_target: Option<usize>,
    ) {
        self.scroll_target = scroll_target;
        self.render(ui, events, annotations, heading_positions, config);
        self.scroll_target = None;
    }

    pub fn render(
        &mut self,
        ui: &mut Ui,
        events: &[Event<'_>],
        annotations: &AnnotationStore,
        heading_positions: &mut Vec<f32>,
        config: &Config,
    ) {
        // Reset state
        self.reset();

        let base_font_size = config.theme.fonts.size;
        let spacing = &config.theme.spacing;

        for event in events {
            match event {
                // Handle footnote definitions before generic Start/End tags
                Event::Start(Tag::FootnoteDefinition(name)) => {
                    self.in_footnote_definition = true;
                    self.current_footnote = Some(name.to_string());
                }
                Event::End(TagEnd::FootnoteDefinition) => {
                    if let Some(name) = self.current_footnote.take() {
                        let text = std::mem::take(&mut self.text_buffer);
                        self.footnote_definitions.push((name, text));
                    }
                    self.in_footnote_definition = false;
                }
                Event::Start(tag) => self.handle_start_tag(tag, ui, heading_positions),
                Event::End(tag) => self.handle_end_tag(tag, ui, base_font_size, spacing, config, annotations),
                Event::Text(text) => self.handle_text(text),
                Event::Code(code) => self.handle_inline_code(code),
                Event::SoftBreak => self.text_buffer.push(' '),
                Event::HardBreak => self.text_buffer.push('\n'),
                Event::Rule => self.render_horizontal_rule(ui),
                Event::TaskListMarker(checked) => self.task_list_marker = Some(*checked),
                Event::FootnoteReference(name) => {
                    // Assign a number to this footnote if not already assigned
                    let num = {
                        let next_num = self.footnote_counter.len() + 1;
                        *self.footnote_counter.entry(name.to_string()).or_insert(next_num)
                    };
                    // Insert a marker that we'll render as superscript
                    self.text_buffer.push_str(&format!("\x01FN:{}:{}\x01", name, num));
                }
                _ => {}
            }
        }

        // Render footnote definitions at the end if any exist
        if !self.footnote_definitions.is_empty() {
            self.render_footnote_definitions(ui, base_font_size);
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
        self.table_rows.clear();
        self.table_header.clear();
        self.in_table_head = false;
        self.task_list_marker = None;
        self.footnote_definitions.clear();
        self.current_footnote = None;
        self.in_footnote_definition = false;
        self.footnote_counter.clear();
        self.char_offset = 0;
        self.heading_index = 0;
        self.table_count = 0;
        self.code_block_count = 0;
    }

    fn handle_start_tag(&mut self, tag: &Tag<'_>, ui: &mut Ui, heading_positions: &mut Vec<f32>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.heading_level = super::parser::heading_level_to_usize(*level);
                // Record heading position for TOC navigation
                heading_positions.push(ui.cursor().top());
                // Check if this is the scroll target - if so, scroll now (before rendering)
                if self.scroll_target == Some(self.heading_index) {
                    ui.scroll_to_cursor(Some(egui::Align::TOP));
                }
                self.heading_index += 1;
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
        annotations: &AnnotationStore,
    ) {
        match tag {
            TagEnd::Heading(_level) => {
                // Apply heading color from config if specified
                let heading_color = config.theme.colors.heading.as_ref().map(|c| parse_config_hex_color(c));
                self.render_heading_with_config(ui, base_font_size, spacing, heading_color);
                self.heading_level = 0;
            }
            TagEnd::Paragraph => {
                if self.in_blockquote {
                    self.render_blockquote(ui, base_font_size);
                } else {
                    // Apply code_text color from config if specified
                    let code_text_color = config.theme.colors.code_text.as_ref().map(|c| parse_config_hex_color(c));
                    self.render_paragraph(ui, base_font_size, spacing, annotations, code_text_color);
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
                self.render_table(ui, base_font_size);
                self.in_table = false;
                self.table_alignments.clear();
                self.table_header.clear();
                self.table_rows.clear();
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
        self.text_buffer.push('\x00');
    }

    fn render_heading_with_config(
        &mut self,
        ui: &mut Ui,
        base_font_size: f32,
        spacing: &crate::config::schema::SpacingConfig,
        heading_color: Option<Color32>,
    ) {
        let text = std::mem::take(&mut self.text_buffer);
        if text.is_empty() {
            return;
        }

        ui.add_space(spacing.heading_top);

        let size_multiplier = heading_size_multiplier(self.heading_level);
        let font_size = base_font_size * size_multiplier;

        let mut rich_text = RichText::new(&text)
            .size(font_size)
            .strong();

        // Apply custom heading color if provided
        if let Some(color) = heading_color {
            rich_text = rich_text.color(color);
        }

        ui.label(rich_text);

        ui.add_space(spacing.heading_bottom);
    }

    fn render_paragraph(
        &mut self,
        ui: &mut Ui,
        base_font_size: f32,
        spacing: &crate::config::schema::SpacingConfig,
        annotations: &AnnotationStore,
        code_text_color: Option<Color32>,
    ) {
        let text = std::mem::take(&mut self.text_buffer);
        if text.is_empty() {
            return;
        }

        let text_len = text.chars().count();
        let start_offset = self.char_offset;
        let end_offset = start_offset + text_len;

        // Check if any annotations overlap with this text
        let overlapping = annotations.in_range(start_offset, end_offset);

        // Parse and render mixed content (text + inline code) with annotations
        self.render_mixed_content_with_annotations(ui, &text, base_font_size, start_offset, &overlapping, code_text_color, self.in_strikethrough);

        // Update character offset
        self.char_offset = end_offset;

        ui.add_space(spacing.paragraph);
    }

    fn render_mixed_content(&self, ui: &mut Ui, text: &str, base_font_size: f32) {
        self.render_mixed_content_with_annotations(ui, text, base_font_size, 0, &[], None, self.in_strikethrough);
    }

    fn render_mixed_content_with_annotations(
        &self,
        ui: &mut Ui,
        text: &str,
        base_font_size: f32,
        start_offset: usize,
        annotations: &[&Annotation],
        code_text_color: Option<Color32>,
        in_strikethrough: bool,
    ) {
        // Build a simple index for the annotations we have
        // This avoids O(n) lookup per character by using sorted annotations
        let end_offset = start_offset + text.chars().count();

        // Create a local sorted copy for efficient lookup
        let mut sorted_anns: Vec<&Annotation> = annotations.iter()
            .filter(|a| a.overlaps(start_offset, end_offset))
            .copied()
            .collect();
        sorted_anns.sort_by_key(|a| a.start);

        // Split on inline code markers and footnote markers, render appropriately
        // First split on \x00 (inline code), then handle \x01 (footnotes) within each part
        let parts: Vec<&str> = text.split('\x00').collect();

        // Find highlight color for a given position using sorted annotations
        let get_highlight_color = |char_pos: usize| -> Option<Color32> {
            for ann in &sorted_anns {
                if ann.start > char_pos {
                    break; // No more candidates
                }
                if ann.kind == AnnotationKind::Highlight && ann.contains(char_pos) {
                    let color = ann.color.as_deref().unwrap_or("#ffeb3b");
                    let c = crate::annotations::ui::parse_hex_color(color);
                    return Some(Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 80));
                }
            }
            None
        };

        // Check if there's a note at this position
        let has_note_at = |char_pos: usize| -> bool {
            for ann in &sorted_anns {
                if ann.start > char_pos {
                    break;
                }
                if ann.kind == AnnotationKind::Note && ann.contains(char_pos) {
                    return true;
                }
            }
            false
        };

        ui.horizontal_wrapped(|ui| {
            let mut current_offset = start_offset;

            for part in parts {
                if let Some(code) = part.strip_prefix("CODE:") {
                    // Apply code_text color from config if specified, otherwise use default
                    let text_color = code_text_color.unwrap_or(Color32::from_rgb(206, 145, 120));
                    let mut code_text = RichText::new(code)
                        .size(base_font_size * 0.9)
                        .monospace()
                        .color(text_color)
                        .background_color(Color32::from_gray(50));
                    if in_strikethrough {
                        code_text = code_text.strikethrough();
                    }
                    ui.label(code_text);
                    current_offset += code.chars().count();
                } else if !part.is_empty() {
                    // Handle footnote references within the text
                    let fn_parts: Vec<&str> = part.split('\x01').collect();
                    for fn_part in fn_parts {
                        if let Some(fn_ref) = fn_part.strip_prefix("FN:") {
                            // Format is "name:num"
                            if let Some((_name, num_str)) = fn_ref.split_once(':') {
                                // Render as superscript number
                                let superscript = RichText::new(format!("[{}]", num_str))
                                    .size(base_font_size * 0.75)
                                    .color(Color32::from_rgb(78, 201, 176))
                                    .raised();
                                ui.label(superscript);
                            }
                        } else if !fn_part.is_empty() {
                            // Check if this text has a highlight annotation
                            if let Some(bg_color) = get_highlight_color(current_offset) {
                                let mut rich = RichText::new(fn_part)
                                    .size(base_font_size)
                                    .background_color(bg_color);
                                // Add note indicator if there's a note
                                if has_note_at(current_offset) {
                                    rich = rich.underline();
                                }
                                if in_strikethrough {
                                    rich = rich.strikethrough();
                                }
                                ui.label(rich);
                            } else if has_note_at(current_offset) {
                                // Just a note, no highlight - show with underline
                                let mut rich = RichText::new(fn_part)
                                    .size(base_font_size)
                                    .underline()
                                    .color(Color32::from_rgb(100, 149, 237));
                                if in_strikethrough {
                                    rich = rich.strikethrough();
                                }
                                ui.label(rich);
                            } else {
                                let mut rich = RichText::new(fn_part).size(base_font_size);
                                if in_strikethrough {
                                    rich = rich.strikethrough();
                                }
                                ui.label(rich);
                            }
                            current_offset += fn_part.chars().count();
                        }
                    }
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

        // Check if this is a Mermaid diagram
        let is_mermaid = self.code_language.as_deref()
            .map(|l| l.to_lowercase() == "mermaid")
            .unwrap_or(false);

        if is_mermaid {
            self.render_mermaid_diagram(ui, &code);
            return;
        }

        let padding = config.theme.spacing.code_padding;
        let is_dark = ui.visuals().dark_mode;

        egui::Frame::none()
            .fill(theme_colors::code_block_bg(is_dark))
            .inner_margin(padding)
            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
            .rounding(4.0)
            .show(ui, |ui| {
                // Show language label if present
                if let Some(lang) = &self.code_language {
                    ui.label(
                        RichText::new(lang)
                            .small()
                            .color(theme_colors::code_line_number(is_dark)),
                    );
                    ui.add_space(4.0);
                }

                // Try syntax highlighting if enabled
                let mut rendered = false;

                #[cfg(feature = "syntax-highlighting")]
                if config.markdown.syntax_highlighting {
                    if let Some(job) = highlight_code(&code, self.code_language.as_deref(), &config.markdown.syntax_theme) {
                        ui.label(job);
                        rendered = true;
                    }
                }

                // Fallback: plain monospace text
                if !rendered {
                    ui.label(
                        RichText::new(&code)
                            .monospace()
                            .color(theme_colors::inline_code_text(is_dark)),
                    );
                }
            });

        self.code_block_count += 1;
        ui.add_space(8.0);
    }

    fn render_mermaid_diagram(&mut self, ui: &mut Ui, code: &str) {
        ui.add_space(8.0);

        // Detect diagram type from the first line
        let diagram_type = detect_mermaid_type(code);

        // Generate cache key based on code content
        let cache_key = format!("mermaid_{}", {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(code.as_bytes());
            hex::encode(&hasher.finalize()[..8])
        });

        // Check success cache first - if we have a rendered image, show it
        if let Some(texture) = self.image_cache.get(&cache_key) {
            self.render_mermaid_image(ui, texture, code, diagram_type);
            self.code_block_count += 1;
            ui.add_space(8.0);
            return;
        }

        // Check failure cache - don't retry renders that already failed
        if self.mermaid_failed.contains(&cache_key) {
            self.render_mermaid_fallback(ui, code, &diagram_type);
            self.code_block_count += 1;
            ui.add_space(8.0);
            return;
        }

        // Check if already rendering asynchronously
        if self.mermaid_pending.contains(&cache_key) {
            self.render_mermaid_loading(ui, code, &diagram_type);
            self.code_block_count += 1;
            ui.add_space(8.0);
            return;
        }

        // Spawn async render if mermaid is available
        if super::mermaid::is_mermaid_available() {
            // Mark as pending and spawn background thread
            self.mermaid_pending.insert(cache_key.clone());

            let code_owned = code.to_string();
            let key_owned = cache_key.clone();
            let sender = self.mermaid_sender.clone();
            let ctx = ui.ctx().clone();

            std::thread::spawn(move || {
                let result = super::mermaid::render_mermaid_to_png(&code_owned, 2.0);
                let _ = sender.send((key_owned, result));
                // Request repaint when done
                ctx.request_repaint();
            });

            // Show loading state for this frame
            self.render_mermaid_loading(ui, code, &diagram_type);
            self.code_block_count += 1;
            ui.add_space(8.0);
            return;
        }

        // Fallback to text preview
        self.render_mermaid_fallback(ui, code, &diagram_type);
        self.code_block_count += 1;
        ui.add_space(8.0);
    }

    fn render_mermaid_image(&self, ui: &mut Ui, texture: &TextureHandle, code: &str, diagram_type: &str) {
        egui::Frame::none()
            .fill(Color32::from_rgb(250, 250, 252))
            .inner_margin(egui::Margin::same(16.0))
            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(200, 210, 220)))
            .show(ui, |ui| {
                // Header
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("◇")
                            .size(18.0)
                            .color(Color32::from_rgb(255, 100, 120))
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Mermaid: {}", diagram_type))
                            .strong()
                            .color(Color32::from_rgb(60, 70, 90))
                    );
                });

                ui.add_space(12.0);

                // Render the actual diagram image
                let tex_size = texture.size_vec2();
                let max_width = ui.available_width() - 32.0;
                let scale = (max_width / tex_size.x).min(1.0);
                let display_size = tex_size * scale;
                ui.image((texture.id(), display_size));

                ui.add_space(8.0);

                // Show source code in collapsible section
                egui::CollapsingHeader::new(
                    RichText::new("View Source")
                        .small()
                        .color(Color32::from_gray(100))
                )
                .default_open(false)
                .id_salt(format!("mermaid_source_{}", self.code_block_count))
                .show(ui, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_gray(240))
                        .inner_margin(8.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(code)
                                    .monospace()
                                    .size(11.0)
                                    .color(Color32::from_gray(80))
                            );
                        });
                });
            });
    }

    fn render_mermaid_loading(&self, ui: &mut Ui, _code: &str, diagram_type: &str) {
        egui::Frame::none()
            .fill(Color32::from_rgb(240, 245, 250))
            .inner_margin(egui::Margin::same(16.0))
            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(200, 210, 220)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Animated spinner effect using time
                    let time = ui.ctx().input(|i| i.time);
                    let spinner_char = match ((time * 4.0) as usize) % 4 {
                        0 => "◐",
                        1 => "◓",
                        2 => "◑",
                        _ => "◒",
                    };
                    ui.label(
                        RichText::new(spinner_char)
                            .size(18.0)
                            .color(Color32::from_rgb(100, 140, 200))
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Rendering {}...", diagram_type))
                            .color(Color32::from_rgb(80, 90, 110))
                    );
                });

                ui.add_space(8.0);

                // Show a placeholder area
                let placeholder_height = 150.0;
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), placeholder_height),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(
                    rect,
                    4.0,
                    Color32::from_rgb(230, 235, 240),
                );

                // Request repaint to animate spinner
                ui.ctx().request_repaint();
            });
    }

    fn render_mermaid_fallback(&self, ui: &mut Ui, code: &str, diagram_type: &str) {
        egui::Frame::none()
            .fill(Color32::from_rgb(30, 35, 45))
            .inner_margin(egui::Margin::same(16.0))
            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
            .rounding(8.0)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(60, 70, 90)))
            .show(ui, |ui| {
                // Header with diagram icon and type
                ui.horizontal(|ui| {
                    // Mermaid icon (flowchart symbol)
                    ui.label(
                        RichText::new("◇")
                            .size(18.0)
                            .color(Color32::from_rgb(255, 154, 162))
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Mermaid Diagram: {}", diagram_type))
                            .strong()
                            .color(Color32::from_rgb(180, 190, 210))
                    );
                });

                ui.add_space(12.0);

                // Render diagram preview based on type
                self.render_mermaid_preview(ui, code, diagram_type);

                ui.add_space(8.0);

                // Show install helper if mermaid-cli is not available
                if !super::mermaid::is_mmdc_available() {
                    self.render_mermaid_install_helper(ui);
                    ui.add_space(8.0);
                }

                // Show source code in collapsible section
                egui::CollapsingHeader::new(
                    RichText::new("View Source")
                        .small()
                        .color(Color32::from_gray(140))
                )
                .default_open(false)
                .id_salt(format!("mermaid_source_{}", self.code_block_count))
                .show(ui, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_gray(25))
                        .inner_margin(8.0)
                        .rounding(4.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(code)
                                    .monospace()
                                    .size(11.0)
                                    .color(Color32::from_rgb(150, 160, 180))
                            );
                        });
                });
            });
    }

    fn render_mermaid_install_helper(&self, ui: &mut Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgb(45, 50, 65))
            .inner_margin(egui::Margin::same(12.0))
            .rounding(6.0)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(80, 130, 180)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("💡")
                            .size(16.0)
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Install mermaid-cli for full diagram rendering")
                            .color(Color32::from_rgb(140, 180, 220))
                    );
                });

                ui.add_space(8.0);

                // Step 1: Install Node.js (platform-specific)
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Step 1:")
                            .strong()
                            .color(Color32::from_rgb(180, 180, 200))
                    );
                    ui.label(
                        RichText::new("Install Node.js (if not already installed)")
                            .color(Color32::from_rgb(160, 160, 180))
                    );
                });

                ui.add_space(4.0);

                // Platform-specific Node.js install instructions
                self.render_nodejs_install_instructions(ui);

                ui.add_space(10.0);

                // Step 2: Install mermaid-cli
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Step 2:")
                            .strong()
                            .color(Color32::from_rgb(180, 180, 200))
                    );
                    ui.label(
                        RichText::new("Install mermaid-cli")
                            .color(Color32::from_rgb(160, 160, 180))
                    );
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    // Copy command button
                    if ui.button(RichText::new("📋 Copy command").small()).clicked() {
                        ui.ctx().copy_text("npm install -g @mermaid-js/mermaid-cli".to_string());
                    }

                    ui.add_space(8.0);

                    // Open npm page button
                    if ui.button(RichText::new("🌐 npm page").small()).clicked() {
                        let _ = open::that("https://www.npmjs.com/package/@mermaid-js/mermaid-cli");
                    }
                });

                ui.add_space(4.0);

                // Show the command
                egui::Frame::none()
                    .fill(Color32::from_gray(25))
                    .inner_margin(6.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("npm install -g @mermaid-js/mermaid-cli")
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(120, 200, 120))
                        );
                    });

                ui.add_space(6.0);

                ui.label(
                    RichText::new("After installation, restart mdview to enable diagram rendering.")
                        .small()
                        .color(Color32::from_gray(130))
                );
            });
    }

    fn render_nodejs_install_instructions(&self, ui: &mut Ui) {
        #[cfg(target_os = "macos")]
        {
            ui.horizontal(|ui| {
                if ui.button(RichText::new("📋 Homebrew").small()).clicked() {
                    ui.ctx().copy_text("brew install node".to_string());
                }
                if ui.button(RichText::new("🌐 nodejs.org").small()).clicked() {
                    let _ = open::that("https://nodejs.org/");
                }
            });
            egui::Frame::none()
                .fill(Color32::from_gray(25))
                .inner_margin(6.0)
                .rounding(4.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("brew install node")
                            .monospace()
                            .size(11.0)
                            .color(Color32::from_rgb(200, 180, 120))
                    );
                });
        }

        #[cfg(target_os = "windows")]
        {
            ui.horizontal(|ui| {
                if ui.button(RichText::new("📋 winget").small()).clicked() {
                    ui.ctx().copy_text("winget install OpenJS.NodeJS".to_string());
                }
                if ui.button(RichText::new("🌐 nodejs.org").small()).clicked() {
                    let _ = open::that("https://nodejs.org/");
                }
            });
            egui::Frame::none()
                .fill(Color32::from_gray(25))
                .inner_margin(6.0)
                .rounding(4.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("winget install OpenJS.NodeJS")
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(200, 180, 120))
                        );
                        ui.label(
                            RichText::new("# or download installer from nodejs.org")
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(100))
                        );
                    });
                });
        }

        #[cfg(target_os = "linux")]
        {
            ui.horizontal(|ui| {
                if ui.button(RichText::new("📋 apt (Debian/Ubuntu)").small()).clicked() {
                    ui.ctx().copy_text("sudo apt install nodejs npm".to_string());
                }
                if ui.button(RichText::new("📋 dnf (Fedora)").small()).clicked() {
                    ui.ctx().copy_text("sudo dnf install nodejs npm".to_string());
                }
                if ui.button(RichText::new("🌐 nodejs.org").small()).clicked() {
                    let _ = open::that("https://nodejs.org/");
                }
            });
            egui::Frame::none()
                .fill(Color32::from_gray(25))
                .inner_margin(6.0)
                .rounding(4.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("# Debian/Ubuntu:")
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(100))
                        );
                        ui.label(
                            RichText::new("sudo apt install nodejs npm")
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(200, 180, 120))
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("# Fedora:")
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(100))
                        );
                        ui.label(
                            RichText::new("sudo dnf install nodejs npm")
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(200, 180, 120))
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("# Arch:")
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(100))
                        );
                        ui.label(
                            RichText::new("sudo pacman -S nodejs npm")
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(200, 180, 120))
                        );
                    });
                });
        }

        // Fallback for other platforms
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            ui.horizontal(|ui| {
                if ui.button(RichText::new("🌐 nodejs.org").small()).clicked() {
                    let _ = open::that("https://nodejs.org/");
                }
            });
            ui.label(
                RichText::new("Download and install Node.js from nodejs.org")
                    .small()
                    .color(Color32::from_gray(140))
            );
        }
    }

    fn render_mermaid_preview(&self, ui: &mut Ui, code: &str, diagram_type: &str) {
        // Parse and render a simplified preview based on diagram type
        match diagram_type {
            "Flowchart" | "Graph" => self.render_flowchart_preview(ui, code),
            "Sequence Diagram" => self.render_sequence_preview(ui, code),
            "Class Diagram" => self.render_class_preview(ui, code),
            "State Diagram" => self.render_state_preview(ui, code),
            "Gantt Chart" => self.render_gantt_preview(ui, code),
            "Pie Chart" => self.render_pie_preview(ui, code),
            "ER Diagram" => self.render_er_preview(ui, code),
            "User Journey" => self.render_journey_preview(ui, code),
            "Git Graph" => self.render_gitgraph_preview(ui, code),
            "Mind Map" => self.render_mindmap_preview(ui, code),
            "Timeline" => self.render_timeline_preview(ui, code),
            "Quadrant Chart" => self.render_quadrant_preview(ui, code),
            "Requirement Diagram" => self.render_requirement_preview(ui, code),
            "C4 Diagram" => self.render_c4_preview(ui, code),
            _ => self.render_generic_preview(ui, code),
        }
    }

    fn render_flowchart_preview(&self, ui: &mut Ui, code: &str) {
        // Extract nodes and display them
        let nodes: Vec<&str> = code.lines()
            .filter(|l| !l.trim().starts_with("graph") && !l.trim().starts_with("flowchart"))
            .filter(|l| l.contains('[') || l.contains('(') || l.contains('{'))
            .take(6)
            .collect();

        if nodes.is_empty() {
            self.render_generic_preview(ui, code);
            return;
        }

        ui.horizontal_wrapped(|ui| {
            for (i, node) in nodes.iter().enumerate() {
                // Extract node text
                let text = extract_node_text(node);

                egui::Frame::none()
                    .fill(Color32::from_rgb(70, 130, 180))
                    .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(&text)
                                .color(Color32::WHITE)
                                .size(12.0)
                        );
                    });

                if i < nodes.len() - 1 {
                    ui.label(
                        RichText::new(" → ")
                            .color(Color32::from_gray(120))
                    );
                }
            }
        });

        if nodes.len() >= 6 {
            ui.label(
                RichText::new("...")
                    .color(Color32::from_gray(100))
                    .italics()
            );
        }
    }

    fn render_sequence_preview(&self, ui: &mut Ui, code: &str) {
        // Extract participants and messages
        let mut participants: Vec<&str> = Vec::new();
        let mut messages: Vec<&str> = Vec::new();

        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("participant") {
                if let Some(name) = trimmed.strip_prefix("participant").map(|s| s.trim()) {
                    participants.push(name);
                }
            } else if trimmed.contains("->>") || trimmed.contains("-->>") {
                messages.push(trimmed);
            }
        }

        // Show participants
        if !participants.is_empty() {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Participants: ").color(Color32::from_gray(140)).small());
                for (i, p) in participants.iter().take(4).enumerate() {
                    if i > 0 {
                        ui.label(RichText::new(", ").color(Color32::from_gray(100)));
                    }
                    ui.label(RichText::new(*p).color(Color32::from_rgb(130, 180, 230)));
                }
            });
        }

        // Show message count
        if !messages.is_empty() {
            ui.label(
                RichText::new(format!("📨 {} messages", messages.len()))
                    .color(Color32::from_gray(140))
                    .small()
            );
        }

        if participants.is_empty() && messages.is_empty() {
            self.render_generic_preview(ui, code);
        }
    }

    fn render_class_preview(&self, ui: &mut Ui, code: &str) {
        let class_count = code.lines()
            .filter(|l| l.trim().starts_with("class "))
            .count();

        ui.label(
            RichText::new(format!("📦 {} classes defined", class_count.max(1)))
                .color(Color32::from_rgb(180, 140, 200))
        );
    }

    fn render_state_preview(&self, ui: &mut Ui, code: &str) {
        let state_count = code.lines()
            .filter(|l| {
                let t = l.trim();
                t.starts_with("state ") || t.contains(" --> ")
            })
            .count();

        ui.label(
            RichText::new(format!("🔄 State machine with ~{} transitions", state_count.max(1)))
                .color(Color32::from_rgb(140, 200, 180))
        );
    }

    fn render_gantt_preview(&self, ui: &mut Ui, code: &str) {
        let task_count = code.lines()
            .filter(|l| l.contains(':'))
            .count();

        ui.label(
            RichText::new(format!("📅 Gantt chart with {} tasks", task_count.max(1)))
                .color(Color32::from_rgb(200, 180, 140))
        );
    }

    fn render_pie_preview(&self, ui: &mut Ui, code: &str) {
        let slice_count = code.lines()
            .filter(|l| l.trim().starts_with('"'))
            .count();

        ui.label(
            RichText::new(format!("🥧 Pie chart with {} slices", slice_count.max(1)))
                .color(Color32::from_rgb(200, 160, 180))
        );
    }

    fn render_er_preview(&self, ui: &mut Ui, code: &str) {
        let entity_count = code.lines()
            .filter(|l| l.contains('{') || l.contains("||"))
            .count();

        ui.label(
            RichText::new(format!("🗃️ ER diagram with {} entities/relations", entity_count.max(1)))
                .color(Color32::from_rgb(160, 190, 200))
        );
    }

    fn render_journey_preview(&self, ui: &mut Ui, code: &str) {
        let section_count = code.lines()
            .filter(|l| l.trim().starts_with("section"))
            .count();

        ui.label(
            RichText::new(format!("🚶 User journey with {} sections", section_count.max(1)))
                .color(Color32::from_rgb(140, 180, 220))
        );
    }

    fn render_gitgraph_preview(&self, ui: &mut Ui, code: &str) {
        let commit_count = code.lines()
            .filter(|l| l.trim().starts_with("commit"))
            .count();
        let branch_count = code.lines()
            .filter(|l| l.trim().starts_with("branch"))
            .count();

        ui.label(
            RichText::new(format!("🌿 Git graph: {} commits, {} branches", commit_count.max(1), branch_count))
                .color(Color32::from_rgb(180, 200, 140))
        );
    }

    fn render_mindmap_preview(&self, ui: &mut Ui, code: &str) {
        let node_count = code.lines()
            .filter(|l| !l.trim().is_empty() && !l.trim().starts_with("mindmap"))
            .count();

        ui.label(
            RichText::new(format!("🧠 Mind map with {} nodes", node_count.max(1)))
                .color(Color32::from_rgb(200, 160, 200))
        );
    }

    fn render_timeline_preview(&self, ui: &mut Ui, code: &str) {
        let event_count = code.lines()
            .filter(|l| l.contains(':'))
            .count();

        ui.label(
            RichText::new(format!("📅 Timeline with {} events", event_count.max(1)))
                .color(Color32::from_rgb(160, 200, 180))
        );
    }

    fn render_quadrant_preview(&self, ui: &mut Ui, _code: &str) {
        ui.label(
            RichText::new("📊 Quadrant chart")
                .color(Color32::from_rgb(180, 180, 200))
        );
    }

    fn render_requirement_preview(&self, ui: &mut Ui, code: &str) {
        let req_count = code.lines()
            .filter(|l| l.trim().starts_with("requirement"))
            .count();

        ui.label(
            RichText::new(format!("📋 Requirements diagram with {} items", req_count.max(1)))
                .color(Color32::from_rgb(200, 180, 160))
        );
    }

    fn render_c4_preview(&self, ui: &mut Ui, _code: &str) {
        ui.label(
            RichText::new("🏗️ C4 Architecture diagram")
                .color(Color32::from_rgb(160, 180, 200))
        );
    }

    fn render_generic_preview(&self, ui: &mut Ui, code: &str) {
        let line_count = code.lines().count();
        ui.label(
            RichText::new(format!("📊 Diagram ({} lines)", line_count))
                .color(Color32::from_gray(160))
                .italics()
        );
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
            let is_dark = ui.visuals().dark_mode;
            let link_text = RichText::new(&text)
                .size(base_font_size)
                .color(theme_colors::link_color(is_dark))
                .underline();

            if ui.link(link_text).clicked() {
                if let Err(e) = open::that(&url) {
                    log::error!("Failed to open link: {}", e);
                }
            }
        }
    }

    fn render_table(&mut self, ui: &mut Ui, base_font_size: f32) {
        if self.table_header.is_empty() && self.table_rows.is_empty() {
            return;
        }

        ui.add_space(8.0);

        let num_cols = self.table_header.len().max(
            self.table_rows.first().map(|r| r.len()).unwrap_or(0)
        );

        if num_cols == 0 {
            return;
        }

        egui::Frame::none()
            .fill(Color32::from_gray(35))
            .rounding(4.0)
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                egui::Grid::new(format!("markdown_table_{}", self.table_count))
                    .num_columns(num_cols)
                    .spacing([12.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Render header row
                        if !self.table_header.is_empty() {
                            for (col_idx, cell) in self.table_header.iter().enumerate() {
                                let alignment = self.table_alignments
                                    .get(col_idx)
                                    .copied()
                                    .unwrap_or(Alignment::None);

                                let text = RichText::new(cell)
                                    .size(base_font_size)
                                    .strong()
                                    .color(Color32::from_gray(220));

                                match alignment {
                                    Alignment::Left | Alignment::None => {
                                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                            ui.label(text);
                                        });
                                    }
                                    Alignment::Center => {
                                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                            ui.label(text);
                                        });
                                    }
                                    Alignment::Right => {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(text);
                                        });
                                    }
                                }
                            }
                            ui.end_row();
                        }

                        // Render data rows
                        for row in &self.table_rows {
                            for (col_idx, cell) in row.iter().enumerate() {
                                let alignment = self.table_alignments
                                    .get(col_idx)
                                    .copied()
                                    .unwrap_or(Alignment::None);

                                let text = RichText::new(cell)
                                    .size(base_font_size)
                                    .color(Color32::from_gray(180));

                                match alignment {
                                    Alignment::Left | Alignment::None => {
                                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                            ui.label(text);
                                        });
                                    }
                                    Alignment::Center => {
                                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                            ui.label(text);
                                        });
                                    }
                                    Alignment::Right => {
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(text);
                                        });
                                    }
                                }
                            }
                            // Fill in empty cells if row is shorter than header
                            for _ in row.len()..num_cols {
                                ui.label("");
                            }
                            ui.end_row();
                        }
                    });
            });

        self.table_count += 1;
        ui.add_space(8.0);
    }

    fn render_image(&mut self, ui: &mut Ui, url: &str, title: &str) {
        // Check if we have this image cached
        if let Some(texture) = self.image_cache.get(url) {
            let size = texture.size_vec2();
            // Scale to fit width if needed
            let max_width = ui.available_width().min(600.0);
            let scale = if size.x > max_width {
                max_width / size.x
            } else {
                1.0
            };
            let display_size = Vec2::new(size.x * scale, size.y * scale);
            ui.image((texture.id(), display_size));
            if !title.is_empty() {
                ui.label(
                    RichText::new(title)
                        .italics()
                        .small()
                        .color(Color32::from_gray(150)),
                );
            }
            return;
        }

        // Check if it's a remote URL
        let is_remote = url.starts_with("http://") || url.starts_with("https://");

        if is_remote {
            // Try to fetch and load remote image
            if let Some(image_data) = self.fetch_remote_image(url) {
                if let Ok(texture) = load_image_from_memory(ui.ctx(), &image_data, url) {
                    let size = texture.size_vec2();
                    let max_width = ui.available_width().min(600.0);
                    let scale = if size.x > max_width {
                        max_width / size.x
                    } else {
                        1.0
                    };
                    let display_size = Vec2::new(size.x * scale, size.y * scale);
                    ui.image((texture.id(), display_size));
                    if !title.is_empty() {
                        ui.label(
                            RichText::new(title)
                                .italics()
                                .small()
                                .color(Color32::from_gray(150)),
                        );
                    }
                    // Cache for future frames
                    self.image_cache.insert(url.to_string(), texture);
                    return;
                }
            }
        } else {
            // Try to load from local path
            let image_path = self.resolve_image_path(url);

            if let Some(path) = image_path {
                if let Ok(texture) = load_image_texture(ui.ctx(), &path, url) {
                    let size = texture.size_vec2();
                    let max_width = ui.available_width().min(600.0);
                    let scale = if size.x > max_width {
                        max_width / size.x
                    } else {
                        1.0
                    };
                    let display_size = Vec2::new(size.x * scale, size.y * scale);
                    ui.image((texture.id(), display_size));
                    if !title.is_empty() {
                        ui.label(
                            RichText::new(title)
                                .italics()
                                .small()
                                .color(Color32::from_gray(150)),
                        );
                    }
                    // Cache for future frames
                    self.image_cache.insert(url.to_string(), texture);
                    return;
                }
            }
        }

        // Fallback: show placeholder for failed loads
        ui.horizontal(|ui| {
            ui.label(RichText::new("🖼").size(20.0));
            let display_text = if !title.is_empty() {
                title.to_string()
            } else if is_remote {
                format!("[Loading failed: {}]", truncate_url(url, 50))
            } else {
                format!("[Image not found: {}]", url)
            };
            ui.label(
                RichText::new(display_text)
                    .italics()
                    .color(Color32::from_gray(150)),
            );
        });
    }

    /// Resolve an image URL to a local path
    fn resolve_image_path(&self, url: &str) -> Option<PathBuf> {
        // Skip remote URLs - they're handled separately
        if url.starts_with("http://") || url.starts_with("https://") {
            return None;
        }

        let path = PathBuf::from(url);

        // If absolute path, use directly
        if path.is_absolute() {
            if path.exists() {
                return Some(path);
            }
            return None;
        }

        // Try relative to base_path
        if let Some(base) = &self.base_path {
            let full_path = base.join(&path);
            if full_path.exists() {
                return Some(full_path);
            }
        }

        // Try relative to current directory
        if path.exists() {
            return Some(path);
        }

        None
    }

    /// Fetch a remote image and cache it locally
    fn fetch_remote_image(&self, url: &str) -> Option<Vec<u8>> {
        // Use ureq to fetch the image
        let response = match ureq::get(url).call() {
            Ok(resp) => resp,
            Err(e) => {
                log::warn!("Failed to fetch remote image {}: {}", url, e);
                return None;
            }
        };

        // Check content type
        let content_type = response.content_type();
        if !content_type.starts_with("image/") {
            log::warn!("Remote URL {} is not an image (content-type: {})", url, content_type);
            return None;
        }

        // Limit size to 10MB
        let max_size = 10 * 1024 * 1024;
        let mut bytes = Vec::new();

        // Read with size limit
        match response.into_reader().take(max_size as u64).read_to_end(&mut bytes) {
            Ok(_) => Some(bytes),
            Err(e) => {
                log::warn!("Failed to read remote image {}: {}", url, e);
                None
            }
        }
    }

    fn render_horizontal_rule(&self, ui: &mut Ui) {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);
    }

    fn render_footnote_definitions(&self, ui: &mut Ui, base_font_size: f32) {
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(8.0);

        ui.label(
            RichText::new("Footnotes")
                .size(base_font_size * 0.9)
                .strong()
                .color(Color32::from_gray(150)),
        );

        ui.add_space(8.0);

        for (name, text) in &self.footnote_definitions {
            let num = self.footnote_counter.get(name).copied().unwrap_or(0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("{}.", num))
                        .size(base_font_size * 0.85)
                        .strong()
                        .color(Color32::from_rgb(78, 201, 176)),
                );
                ui.add_space(4.0);
                ui.label(RichText::new(text).size(base_font_size * 0.9));
            });
            ui.add_space(4.0);
        }
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "syntax-highlighting")]
fn highlight_code(code: &str, language: Option<&str>, theme_name: &str) -> Option<egui::text::LayoutJob> {
    use syntect::highlighting::ThemeSet;
    use syntect::parsing::SyntaxSet;
    use syntect::easy::HighlightLines;
    use std::sync::OnceLock;

    // Lazy-load syntax and theme sets (they're large)
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

    let ss = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = THEME_SET.get_or_init(ThemeSet::load_defaults);

    // Find syntax for the language
    let syntax = language
        .and_then(|lang| ss.find_syntax_by_token(lang))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    // Use theme from config, falling back to base16-ocean.dark if not found
    let theme = ts.themes.get(theme_name)
        .or_else(|| ts.themes.get("base16-ocean.dark"))
        .unwrap_or_else(|| ts.themes.values().next().unwrap());
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut job = egui::text::LayoutJob::default();
    let mono_font = egui::FontId::monospace(13.0);

    for line in code.lines() {
        let Ok(ranges) = highlighter.highlight_line(line, ss) else {
            return None;
        };

        for (style, text) in ranges {
            let color = syntect_color_to_egui(style);
            job.append(
                text,
                0.0,
                egui::TextFormat {
                    font_id: mono_font.clone(),
                    color,
                    ..Default::default()
                },
            );
        }
        // Add newline
        job.append(
            "\n",
            0.0,
            egui::TextFormat {
                font_id: mono_font.clone(),
                color: Color32::WHITE,
                ..Default::default()
            },
        );
    }

    Some(job)
}

#[cfg(feature = "syntax-highlighting")]
fn syntect_color_to_egui(style: syntect::highlighting::Style) -> Color32 {
    Color32::from_rgba_unmultiplied(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
        style.foreground.a,
    )
}

#[cfg(not(feature = "syntax-highlighting"))]
fn highlight_code(_code: &str, _language: Option<&str>, _theme_name: &str) -> Option<egui::text::LayoutJob> {
    None
}

/// Load an image from a local path into an egui texture
fn load_image_texture(
    ctx: &egui::Context,
    path: &std::path::Path,
    name: &str,
) -> Result<TextureHandle, String> {
    // Read the file
    let image_data = std::fs::read(path)
        .map_err(|e| format!("Failed to read image: {}", e))?;

    load_image_from_memory(ctx, &image_data, name)
}

/// Load an image from memory into an egui texture
fn load_image_from_memory(
    ctx: &egui::Context,
    image_data: &[u8],
    name: &str,
) -> Result<TextureHandle, String> {
    // Decode the image
    let image = image::load_from_memory(image_data)
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();

    // Create the texture
    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);

    Ok(ctx.load_texture(
        name,
        color_image,
        egui::TextureOptions::LINEAR,
    ))
}

/// Truncate a URL for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len - 3])
    }
}

/// Detect the type of Mermaid diagram from its source code
fn detect_mermaid_type(code: &str) -> &'static str {
    let first_line = code.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if first_line.starts_with("graph") || first_line.starts_with("flowchart") {
        "Flowchart"
    } else if first_line.starts_with("sequencediagram") || first_line.starts_with("sequence") {
        "Sequence Diagram"
    } else if first_line.starts_with("classdiagram") || first_line.starts_with("class") {
        "Class Diagram"
    } else if first_line.starts_with("statediagram") || first_line.starts_with("state") {
        "State Diagram"
    } else if first_line.starts_with("gantt") {
        "Gantt Chart"
    } else if first_line.starts_with("pie") {
        "Pie Chart"
    } else if first_line.starts_with("erdiagram") || first_line.starts_with("er") {
        "ER Diagram"
    } else if first_line.starts_with("journey") {
        "User Journey"
    } else if first_line.starts_with("gitgraph") {
        "Git Graph"
    } else if first_line.starts_with("mindmap") {
        "Mind Map"
    } else if first_line.starts_with("timeline") {
        "Timeline"
    } else if first_line.starts_with("quadrantchart") {
        "Quadrant Chart"
    } else if first_line.starts_with("requirementdiagram") {
        "Requirement Diagram"
    } else if first_line.starts_with("c4context") || first_line.starts_with("c4container") {
        "C4 Diagram"
    } else {
        "Diagram"
    }
}

/// Extract node text from a flowchart line like "A[Node Text]" or "B(Round Node)"
fn extract_node_text(line: &str) -> String {
    let line = line.trim();

    // Try to find text in brackets: [], (), {}, (())
    for (open, close) in &[('[', ']'), ('(', ')'), ('{', '}'), ('<', '>')] {
        if let Some(start) = line.find(*open) {
            if let Some(end) = line.rfind(*close) {
                if end > start {
                    let text = &line[start + 1..end];
                    // Handle double brackets like (())
                    let text = text.trim_start_matches(*open).trim_end_matches(*close);
                    if !text.is_empty() {
                        return text.to_string();
                    }
                }
            }
        }
    }

    // Try to extract node ID before any special characters
    let node_id: String = line.chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    if !node_id.is_empty() {
        node_id
    } else {
        line.chars().take(20).collect()
    }
}
