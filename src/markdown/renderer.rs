//! Markdown renderer - converts pulldown-cmark events to egui widgets

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use indexmap::IndexMap;

use egui::{epaint, Color32, Label, Pos2, RichText, Sense, Stroke, TextureHandle, Ui, Vec2};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, MetadataBlockKind, Tag, TagEnd};

use crate::annotations::model::{Annotation, AnnotationKind};
use crate::annotations::AnnotationStore;
use crate::config::defaults::heading_size_multiplier;
use crate::config::Config;

/// Maximum number of images to cache (LRU eviction when exceeded)
const IMAGE_CACHE_MAX_SIZE: usize = 50;

/// Maximum number of syntax-highlighted code blocks to cache
const SYNTAX_CACHE_MAX_SIZE: usize = 100;

/// Maximum number of mermaid metadata entries to cache
const MERMAID_METADATA_CACHE_MAX_SIZE: usize = 200;

/// Default viewport buffer when visible rect is unknown (in pixels)
const DEFAULT_VIEWPORT_BUFFER: f32 = 2000.0;

/// Pre-built annotation index for efficient O(log n) lookups
/// Instead of O(n) scan per character, we build a sorted list of annotation boundaries
struct AnnotationIndex<'a> {
    /// Annotations sorted by start position for binary search
    sorted_annotations: Vec<&'a Annotation>,
}

impl<'a> AnnotationIndex<'a> {
    /// Build an index from all annotations (sorted by start position).
    /// If `ensure_sorted_cache()` was called on the store beforehand,
    /// this returns pre-sorted refs in O(n) instead of O(n log n).
    fn new(annotations: &'a AnnotationStore) -> Self {
        let sorted = annotations.sorted_by_position();
        Self {
            sorted_annotations: sorted,
        }
    }

    /// Get annotations overlapping a range using binary search
    /// Returns annotations already sorted by start position
    fn in_range(&self, start: usize, end: usize) -> Vec<&'a Annotation> {
        if self.sorted_annotations.is_empty() {
            return Vec::new();
        }

        // Binary search to find first annotation that might overlap
        // An annotation overlaps [start, end) if ann.start < end AND ann.end > start
        let first_idx = self.sorted_annotations.partition_point(|a| a.end <= start);

        // Collect overlapping annotations — partition_point guarantees a.end > start
        // for all remaining entries, so we only need to check a.start < end.
        // Using take_while for early exit since annotations are sorted by start.
        self.sorted_annotations[first_idx..]
            .iter()
            .take_while(|a| a.start < end)
            .copied()
            .collect()
    }
}

/// Result of an async mermaid render
type MermaidRenderResult = (String, Result<Vec<u8>, String>);

/// Result of an async image load
type ImageLoadResult = (String, Result<Vec<u8>, String>);

/// HTTP timeout for remote image fetching (10 seconds)
const IMAGE_FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// Cached mermaid diagram metadata (avoid re-parsing every frame)
#[derive(Clone)]
struct MermaidMetadata {
    /// Type of diagram (flowchart, sequence, etc.)
    #[allow(dead_code)]
    diagram_type: String,
    /// Various counts extracted from the diagram
    count1: usize,
    count2: usize,
    /// Preview nodes for flowchart
    nodes: Vec<String>,
}

/// Parse a hex color string from config to Color32 (delegates to shared implementation)
fn parse_config_hex_color(hex: &str) -> Color32 {
    crate::theme::style::parse_hex_color(hex)
}

/// Theme-aware colors for markdown rendering
mod theme_colors {
    use egui::Color32;

    pub fn code_block_bg(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(30, 35, 45)
        } else {
            Color32::from_rgb(245, 247, 250)
        }
    }

    pub fn inline_code_text(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(206, 145, 120)
        } else {
            Color32::from_rgb(180, 80, 80)
        }
    }

    pub fn code_line_number(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(100, 110, 130)
        } else {
            Color32::from_rgb(150, 155, 165)
        }
    }

    pub fn link_color(is_dark: bool) -> Color32 {
        if is_dark {
            Color32::from_rgb(78, 201, 176)
        } else {
            Color32::from_rgb(0, 120, 150)
        }
    }
}

/// Estimated height of a single line of text in egui (used for viewport culling estimates).
/// Based on default font size 14.0 * line_height 1.6 = 22.4px.
const ESTIMATED_LINE_HEIGHT: f32 = 22.4;

/// Average character width estimate for proportional font (used for height estimation).
/// Slightly narrow to produce conservative (taller) height estimates — overestimates cause
/// less visible layout shift than underestimates when blocks transition from culled to rendered.
const ESTIMATED_CHAR_WIDTH: f32 = 7.0;
const INLINE_CODE_MARKER: char = '\x00';
const FOOTNOTE_MARKER: char = '\x01';
const LINK_MARKER: char = '\x02';
const LINK_END_MARKER: char = '\x03';

/// Clamp an index down to the nearest valid UTF-8 boundary in `text`.
/// Annotation offsets are byte-based, so this keeps slicing safe for non-ASCII content.
fn floor_to_char_boundary(text: &str, index: usize) -> usize {
    let mut clamped = index.min(text.len());
    while clamped > 0 && !text.is_char_boundary(clamped) {
        clamped -= 1;
    }
    clamped
}

fn byte_offset_for_char_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

/// Convert renderer control markers to plain text for contexts that do not render inline widgets
/// (e.g. headings, blockquotes, table cells).
fn strip_inline_markers(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;

    while !rest.is_empty() {
        if let Some(code_part) = rest.strip_prefix("\x00CODE:") {
            if let Some(end_idx) = code_part.find(INLINE_CODE_MARKER) {
                out.push_str(&code_part[..end_idx]);
                rest = &code_part[end_idx + INLINE_CODE_MARKER.len_utf8()..];
                continue;
            }
        }

        if let Some(fn_part) = rest.strip_prefix("\x01FN:") {
            if let Some(end_idx) = fn_part.find(FOOTNOTE_MARKER) {
                let payload = &fn_part[..end_idx];
                if let Some((_name, num)) = payload.split_once(':') {
                    out.push('[');
                    out.push_str(num);
                    out.push(']');
                }
                rest = &fn_part[end_idx + FOOTNOTE_MARKER.len_utf8()..];
                continue;
            }
        }

        if let Some(link_part) = rest.strip_prefix(LINK_MARKER) {
            if let Some(url_end_idx) = link_part.find(LINK_MARKER) {
                let after_url = &link_part[url_end_idx + LINK_MARKER.len_utf8()..];
                if let Some(link_end_idx) = after_url.find(LINK_END_MARKER) {
                    out.push_str(&after_url[..link_end_idx]);
                    rest = &after_url[link_end_idx + LINK_END_MARKER.len_utf8()..];
                    continue;
                }
            }
        }

        if let Some(ch) = rest.chars().next() {
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        } else {
            break;
        }
    }

    out
}

/// Find highlight color for a byte offset using sorted annotations.
fn annotation_highlight_color(annotations: &[&Annotation], byte_pos: usize) -> Option<Color32> {
    for ann in annotations {
        if ann.start > byte_pos {
            break;
        }
        if ann.kind == AnnotationKind::Highlight && ann.contains(byte_pos) {
            let (r, g, b, _) = ann.get_color_rgba();
            return Some(Color32::from_rgba_unmultiplied(r, g, b, 80));
        }
    }
    None
}

/// Check whether a note annotation exists at a byte offset.
fn annotation_has_note(annotations: &[&Annotation], byte_pos: usize) -> bool {
    for ann in annotations {
        if ann.start > byte_pos {
            break;
        }
        if ann.kind == AnnotationKind::Note && ann.contains(byte_pos) {
            return true;
        }
    }
    false
}

/// A pre-computed block boundary in the event stream.
/// Blocks represent top-level renderable elements (paragraphs, headings, code blocks, etc.)
/// that can be skipped entirely during viewport culling without processing their events.
#[derive(Debug, Clone)]
struct ContentBlock {
    /// Start index in the events slice (inclusive)
    event_start: usize,
    /// End index in the events slice (exclusive)
    event_end: usize,
    /// Estimated height of this block in pixels (used when actual_height is unknown)
    estimated_height: f32,
    /// Actual rendered height measured after a real render pass.
    /// Once set, this is used instead of estimated_height for more accurate culling.
    actual_height: Option<f32>,
    /// Whether this block contains a heading (needed for TOC position tracking)
    is_heading: bool,
    /// Number of text bytes in this block (for char_offset tracking)
    text_byte_len: usize,
    /// The heading index within this block (for scroll-to-heading targeting)
    heading_index: Option<usize>,
}

impl ContentBlock {
    /// Returns the best known height for this block (actual if measured, estimated otherwise)
    fn height(&self) -> f32 {
        self.actual_height.unwrap_or(self.estimated_height)
    }
}

/// Screen-space hit target for rendered text with optional galley geometry for wrapped labels.
struct TextHitTarget {
    rect: egui::Rect,
    start: usize,
    end: usize,
    galley_pos: Option<Pos2>,
    galley: Option<Arc<egui::Galley>>,
}

#[derive(Clone, Copy)]
struct MixedContentStyle {
    base_font_size: f32,
    start_offset: usize,
    code_text_color: Option<Color32>,
    in_strikethrough: bool,
}

/// One-frame navigation targets for markdown rendering.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderTargets {
    /// Heading index to scroll to from TOC navigation.
    pub heading: Option<usize>,
    /// Document byte offset to scroll to from full-document search.
    pub search_offset: Option<usize>,
    /// Active search range to highlight.
    pub active_search_range: Option<(usize, usize)>,
}

/// Pre-compute block boundaries from a pulldown-cmark event stream.
/// This allows the renderer to skip entire blocks that are off-screen without
/// processing any of their events through the state machine.
///
/// Each heading block is assigned a `heading_index` so the renderer can force-render
/// blocks containing the scroll target heading (for TOC jump navigation).
fn compute_block_map(events: &[Event<'_>], available_width: f32) -> Vec<ContentBlock> {
    let mut blocks = Vec::new();
    let mut i = 0;
    let mut heading_counter = 0usize;
    let chars_per_line = (available_width / ESTIMATED_CHAR_WIDTH).max(1.0);

    while i < events.len() {
        match &events[i] {
            // Top-level block elements that we can skip as units
            Event::Start(Tag::Paragraph) => {
                let start = i;
                let mut text_len = 0usize;
                i += 1;
                while i < events.len() {
                    match &events[i] {
                        Event::End(TagEnd::Paragraph) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t)
                        | Event::Code(t)
                        | Event::InlineMath(t)
                        | Event::InlineHtml(t)
                        | Event::Html(t) => {
                            text_len += t.len();
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                let num_lines = (text_len as f32 / chars_per_line).ceil().max(1.0);
                let height = num_lines * ESTIMATED_LINE_HEIGHT + 16.0; // paragraph spacing (matches SpacingConfig::default().paragraph)
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: height,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::Start(Tag::Heading { .. }) => {
                let start = i;
                let mut text_len = 0usize;
                let h_idx = heading_counter;
                heading_counter += 1;
                i += 1;
                while i < events.len() {
                    match &events[i] {
                        Event::End(TagEnd::Heading(_)) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t)
                        | Event::Code(t)
                        | Event::InlineMath(t)
                        | Event::InlineHtml(t) => {
                            text_len += t.len();
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                let height = ESTIMATED_LINE_HEIGHT * 2.0 + 24.0; // heading + spacing
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: height,
                    actual_height: None,
                    is_heading: true,
                    text_byte_len: text_len,
                    heading_index: Some(h_idx),
                });
            }
            Event::Start(Tag::CodeBlock(_)) => {
                let start = i;
                let mut text_len = 0usize;
                i += 1;
                while i < events.len() {
                    match &events[i] {
                        Event::End(TagEnd::CodeBlock) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t) => {
                            text_len += t.len();
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                let num_lines = events[start + 1..i]
                    .iter()
                    .filter_map(|e| match e {
                        Event::Text(t) => Some(t.lines().count()),
                        _ => None,
                    })
                    .sum::<usize>()
                    .max(1);
                let height = num_lines as f32 * ESTIMATED_LINE_HEIGHT + 32.0; // padding + margins
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: height,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::Start(Tag::HtmlBlock) => {
                let start = i;
                let mut text_len = 0usize;
                i += 1;
                while i < events.len() {
                    match &events[i] {
                        Event::End(TagEnd::HtmlBlock) => {
                            i += 1;
                            break;
                        }
                        Event::Html(t) | Event::InlineHtml(t) | Event::Text(t) => {
                            text_len += t.len();
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                let num_lines = (text_len as f32 / chars_per_line).ceil().max(1.0);
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: num_lines * ESTIMATED_LINE_HEIGHT + 24.0,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let start = i;
                let mut text_len = 0usize;
                let mut depth = 1;
                i += 1;
                while i < events.len() && depth > 0 {
                    match &events[i] {
                        Event::Start(Tag::BlockQuote(_)) => {
                            depth += 1;
                        }
                        Event::End(TagEnd::BlockQuote(_)) => {
                            depth -= 1;
                        }
                        Event::Text(t) | Event::Code(t) | Event::InlineMath(t) => {
                            text_len += t.len();
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let num_lines = (text_len as f32 / chars_per_line).ceil().max(1.0);
                let height = num_lines * ESTIMATED_LINE_HEIGHT + 16.0;
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: height,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::Start(Tag::List(_)) => {
                let start = i;
                let mut text_len = 0usize;
                let mut item_count = 0usize;
                let mut depth = 1;
                i += 1;
                while i < events.len() && depth > 0 {
                    match &events[i] {
                        Event::Start(Tag::List(_)) => {
                            depth += 1;
                        }
                        Event::End(TagEnd::List(_)) => {
                            depth -= 1;
                        }
                        Event::Start(Tag::Item) => {
                            item_count += 1;
                        }
                        Event::Text(t)
                        | Event::Code(t)
                        | Event::InlineMath(t)
                        | Event::InlineHtml(t) => {
                            text_len += t.len();
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let height = item_count as f32 * ESTIMATED_LINE_HEIGHT + 8.0;
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: height,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::Start(Tag::Table(_)) => {
                let start = i;
                let mut text_len = 0usize;
                let mut row_count = 0usize;
                let mut depth = 1;
                i += 1;
                while i < events.len() && depth > 0 {
                    match &events[i] {
                        Event::Start(Tag::Table(_)) => {
                            depth += 1;
                        }
                        Event::End(TagEnd::Table) => {
                            depth -= 1;
                        }
                        Event::Start(Tag::TableRow) | Event::Start(Tag::TableHead) => {
                            row_count += 1;
                        }
                        Event::Text(t)
                        | Event::Code(t)
                        | Event::InlineMath(t)
                        | Event::InlineHtml(t) => {
                            text_len += t.len();
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let height = row_count as f32 * 24.0 + 40.0;
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: height,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::Start(Tag::FootnoteDefinition(_)) => {
                // Footnote definitions are always processed (not culled)
                let start = i;
                let mut text_len = 0usize;
                i += 1;
                while i < events.len() {
                    match &events[i] {
                        Event::End(TagEnd::FootnoteDefinition) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t) | Event::Code(t) | Event::InlineMath(t) => {
                            text_len += t.len();
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: 0.0, // footnotes rendered at end, height doesn't matter for culling
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::Start(Tag::MetadataBlock(_)) => {
                let start = i;
                let mut text_len = 0usize;
                i += 1;
                while i < events.len() {
                    match &events[i] {
                        Event::End(TagEnd::MetadataBlock(_)) => {
                            i += 1;
                            break;
                        }
                        Event::Text(t) => {
                            text_len += t.len();
                            i += 1;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                let num_lines = (text_len as f32 / chars_per_line).ceil().max(1.0);
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: num_lines * ESTIMATED_LINE_HEIGHT + 32.0,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::Start(Tag::DefinitionList) => {
                let start = i;
                let mut text_len = 0usize;
                let mut depth = 1;
                i += 1;
                while i < events.len() && depth > 0 {
                    match &events[i] {
                        Event::Start(Tag::DefinitionList) => depth += 1,
                        Event::End(TagEnd::DefinitionList) => depth -= 1,
                        Event::Text(t)
                        | Event::Code(t)
                        | Event::InlineMath(t)
                        | Event::InlineHtml(t) => text_len += t.len(),
                        _ => {}
                    }
                    i += 1;
                }
                let num_lines = (text_len as f32 / chars_per_line).ceil().max(1.0);
                blocks.push(ContentBlock {
                    event_start: start,
                    event_end: i,
                    estimated_height: num_lines * ESTIMATED_LINE_HEIGHT + 24.0,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: text_len,
                    heading_index: None,
                });
            }
            Event::DisplayMath(math) => {
                let num_lines = math.lines().count().max(1);
                blocks.push(ContentBlock {
                    event_start: i,
                    event_end: i + 1,
                    estimated_height: num_lines as f32 * ESTIMATED_LINE_HEIGHT + 32.0,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: math.len(),
                    heading_index: None,
                });
                i += 1;
            }
            Event::Rule => {
                blocks.push(ContentBlock {
                    event_start: i,
                    event_end: i + 1,
                    estimated_height: 20.0,
                    actual_height: None,
                    is_heading: false,
                    text_byte_len: 0,
                    heading_index: None,
                });
                i += 1;
            }
            _ => {
                // Single events that don't form blocks (soft break, etc.)
                // These are rare at the top level; just advance
                i += 1;
            }
        }
    }

    blocks
}

/// Hash the style/layout settings that can affect measured block heights.
/// Keeping this separate from the parsed-event cache lets viewport culling reuse work
/// aggressively while still invalidating stale measurements after visual settings change.
fn render_layout_cache_key(config: &Config) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let prime: u64 = 0x100000001b3;

    fn feed_u32(hash: &mut u64, prime: u64, val: u32) {
        *hash ^= val as u64;
        *hash = hash.wrapping_mul(prime);
    }

    fn feed_bool(hash: &mut u64, prime: u64, val: bool) {
        feed_u32(hash, prime, val as u32);
    }

    fn feed_f32(hash: &mut u64, prime: u64, val: f32) {
        feed_u32(hash, prime, val.to_bits());
    }

    fn feed_opt_f32(hash: &mut u64, prime: u64, val: Option<f32>) {
        match val {
            Some(value) => {
                feed_bool(hash, prime, true);
                feed_f32(hash, prime, value);
            }
            None => feed_bool(hash, prime, false),
        }
    }

    fn feed_str(hash: &mut u64, prime: u64, val: &str) {
        for byte in val.bytes() {
            *hash ^= byte as u64;
            *hash = hash.wrapping_mul(prime);
        }
        *hash ^= 0xff;
        *hash = hash.wrapping_mul(prime);
    }

    feed_str(&mut hash, prime, &config.theme.fonts.body);
    feed_str(&mut hash, prime, &config.theme.fonts.heading);
    feed_str(&mut hash, prime, &config.theme.fonts.code);
    feed_f32(&mut hash, prime, config.theme.fonts.size);
    feed_f32(&mut hash, prime, config.theme.fonts.line_height);
    feed_f32(&mut hash, prime, config.theme.spacing.paragraph);
    feed_f32(&mut hash, prime, config.theme.spacing.heading_top);
    feed_f32(&mut hash, prime, config.theme.spacing.heading_bottom);
    feed_f32(&mut hash, prime, config.theme.spacing.list_indent);
    feed_f32(&mut hash, prime, config.theme.spacing.code_padding);
    feed_opt_f32(&mut hash, prime, config.layout.image_width);
    feed_bool(&mut hash, prime, config.markdown.show_line_numbers);
    hash
}

/// Markdown renderer that converts events to egui widgets
pub struct MarkdownRenderer {
    /// Current text buffer for accumulating inline content
    text_buffer: String,

    /// Current heading level (0 = not in heading)
    heading_level: usize,

    /// Whether we're in a code block
    in_code_block: bool,

    /// Whether we're collecting a raw HTML block fallback
    in_html_block: bool,

    /// Raw HTML block content
    html_content: String,

    /// Whether we're collecting a front matter / metadata block
    in_metadata_block: bool,

    /// Metadata block kind
    metadata_kind: Option<MetadataBlockKind>,

    /// Metadata block content
    metadata_content: String,

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

    /// Image texture cache with LRU ordering (max IMAGE_CACHE_MAX_SIZE entries)
    /// Using IndexMap for O(1) access and LRU eviction (oldest at front)
    image_cache: IndexMap<String, TextureHandle>,

    /// Set of images currently being loaded asynchronously
    image_pending: std::collections::HashSet<String>,

    /// Channel sender for spawning async image loads
    image_sender: mpsc::Sender<ImageLoadResult>,

    /// Channel receiver for completed async image loads
    image_receiver: mpsc::Receiver<ImageLoadResult>,

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

    /// Byte offset at the start of the current paragraph (for annotation ranges)
    paragraph_start_offset: Option<usize>,

    /// Target heading index to scroll to (for TOC navigation)
    scroll_target: Option<usize>,

    /// Target document byte offset to scroll to (for document search navigation)
    search_target: Option<usize>,

    /// Active document search match to emphasize while rendering
    active_search_range: Option<(usize, usize)>,

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

    /// Cache for syntax-highlighted code blocks with LRU ordering
    /// Key: (code_hash, language, theme_name)
    /// Using IndexMap for O(1) access and LRU eviction.
    /// Arc wrapper avoids expensive LayoutJob clones on cache hits.
    #[cfg(feature = "syntax-highlighting")]
    syntax_cache: IndexMap<String, Arc<egui::text::LayoutJob>>,

    /// Cache for mermaid diagram metadata (avoids re-parsing every frame)
    mermaid_metadata_cache: HashMap<String, MermaidMetadata>,

    /// Visible rect from last render (for viewport culling)
    visible_rect: Option<egui::Rect>,

    /// Maximum image width from config (defaults to 600.0)
    image_max_width: Option<f32>,

    /// Number of elements skipped by viewport culling (for diagnostics)
    culled_count: usize,

    /// Cached block map for event-level viewport culling
    /// Invalidated when events, available width, or height-affecting style settings change
    /// Tuple: (events_ptr, available_width_rounded, render_layout_key, blocks)
    cached_block_map: Option<(usize, u32, u64, Vec<ContentBlock>)>,

    /// Running Y offset accumulator for block-level culling
    /// Tracks estimated cumulative height as blocks are skipped/rendered
    block_y_offset: f32,

    /// Screen-space hit targets for rendered text runs (start/end byte offsets).
    /// Rebuilt every frame and used for pointer-to-text selection mapping.
    text_hit_boxes: Vec<TextHitTarget>,
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        let (mermaid_sender, mermaid_receiver) = mpsc::channel();
        let (image_sender, image_receiver) = mpsc::channel();
        Self {
            text_buffer: String::new(),
            heading_level: 0,
            in_code_block: false,
            in_html_block: false,
            html_content: String::new(),
            in_metadata_block: false,
            metadata_kind: None,
            metadata_content: String::new(),
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
            image_cache: IndexMap::new(),
            image_pending: std::collections::HashSet::new(),
            image_sender,
            image_receiver,
            footnote_definitions: Vec::new(),
            current_footnote: None,
            in_footnote_definition: false,
            footnote_counter: HashMap::new(),
            char_offset: 0,
            paragraph_start_offset: None,
            scroll_target: None,
            search_target: None,
            active_search_range: None,
            heading_index: 0,
            table_count: 0,
            code_block_count: 0,
            mermaid_failed: std::collections::HashSet::new(),
            mermaid_pending: std::collections::HashSet::new(),
            mermaid_sender,
            mermaid_receiver,
            #[cfg(feature = "syntax-highlighting")]
            syntax_cache: IndexMap::new(),
            mermaid_metadata_cache: HashMap::new(),
            visible_rect: None,
            image_max_width: Some(600.0),
            culled_count: 0,
            cached_block_map: None,
            block_y_offset: 0.0,
            text_hit_boxes: Vec::new(),
        }
    }

    /// Set the base path for resolving relative image URLs
    pub fn set_base_path(&mut self, path: Option<PathBuf>) {
        self.base_path = path;
    }

    /// Check if a vertical position is potentially visible (with buffer for smooth scrolling)
    /// Returns true if the position might be in or near the visible viewport
    fn is_position_visible(&self, y_pos: f32) -> bool {
        match self.visible_rect {
            Some(rect) => {
                // Add a buffer (2x viewport height) to avoid popping during scroll
                let buffer = rect.height() * 2.0;
                y_pos >= rect.top() - buffer && y_pos <= rect.bottom() + buffer
            }
            None => true, // If no rect cached, assume visible
        }
    }

    /// Clear the image cache (call when document changes)
    pub fn clear_image_cache(&mut self) {
        self.image_cache.clear();
        self.image_pending.clear();
        self.mermaid_failed.clear();
        self.mermaid_pending.clear();
        self.cached_block_map = None;
    }

    /// Insert an image into the cache with LRU eviction (O(1) operations using IndexMap)
    fn cache_image(&mut self, key: String, texture: TextureHandle) {
        // If key already exists, move to end for LRU (O(1) with IndexMap)
        if self.image_cache.contains_key(&key) {
            self.image_cache.shift_remove(&key);
        }

        // Evict oldest entries if at capacity (oldest is at front of IndexMap)
        while self.image_cache.len() >= IMAGE_CACHE_MAX_SIZE {
            self.image_cache.shift_remove_index(0);
        }

        // Insert new entry at end (most recently used)
        self.image_cache.insert(key, texture);
    }

    /// Poll for completed async image loads
    /// Call this at the start of each frame to process completed loads
    /// Returns true if any loads completed (UI should repaint)
    pub fn poll_image_loads(&mut self, ctx: &egui::Context) -> bool {
        let mut any_completed = false;

        // Process all available completed loads
        while let Ok((cache_key, result)) = self.image_receiver.try_recv() {
            self.image_pending.remove(&cache_key);
            any_completed = true;

            match result {
                Ok(image_data) => {
                    // Load texture from image bytes
                    if let Ok(img) = image::load_from_memory(&image_data) {
                        let rgba = img.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        let pixels = rgba.into_raw();
                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                        let texture =
                            ctx.load_texture(&cache_key, color_image, egui::TextureOptions::LINEAR);
                        self.cache_image(cache_key.clone(), texture);
                        log::debug!("Async image load completed: {}", cache_key);
                    } else {
                        log::warn!("Failed to decode async image: {}", cache_key);
                    }
                }
                Err(e) => {
                    log::debug!("Async image load failed: {} - {}", cache_key, e);
                }
            }
        }

        if any_completed {
            ctx.request_repaint();
        }

        any_completed
    }

    /// Get syntax-highlighted code block with caching (O(1) LRU using IndexMap)
    /// Returns cached LayoutJob if available, otherwise computes and caches it.
    /// Uses Arc to avoid expensive LayoutJob clones on cache hits.
    #[cfg(feature = "syntax-highlighting")]
    fn get_highlighted_code(
        &mut self,
        code: &str,
        language: Option<&str>,
        theme_name: &str,
        is_dark: bool,
        show_line_numbers: bool,
    ) -> Option<Arc<egui::text::LayoutJob>> {
        let normalized_language = normalize_language_token(language);
        // Generate cache key from code hash + language + theme + dark mode + line numbers
        let cache_key = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(code.as_bytes());
            hasher.update(normalized_language.as_deref().unwrap_or("").as_bytes());
            hasher.update(theme_name.as_bytes());
            hasher.update(if is_dark { "dark" } else { "light" });
            hasher.update(if show_line_numbers { "ln" } else { "" });
            hex::encode(&hasher.finalize()[..12])
        };

        // Check cache first - if found, move to end for LRU (O(1) with IndexMap)
        // Arc clones are cheap (atomic refcount) vs full LayoutJob clones
        if let Some(job) = self.syntax_cache.get(&cache_key).cloned() {
            // Move to end for LRU tracking (most recently used)
            self.syntax_cache.shift_remove(&cache_key);
            self.syntax_cache.insert(cache_key, job.clone());
            return Some(job);
        }

        // Cache miss - compute syntax highlighting
        let job = Arc::new(highlight_code(
            code,
            normalized_language.as_deref(),
            theme_name,
            is_dark,
            show_line_numbers,
        )?);

        // Cache with LRU eviction (O(1) operations)
        while self.syntax_cache.len() >= SYNTAX_CACHE_MAX_SIZE {
            // Remove oldest (first) entry
            self.syntax_cache.shift_remove_index(0);
        }

        self.syntax_cache.insert(cache_key, job.clone());

        Some(job)
    }

    /// Get the current character offset (position in document after last rendered element)
    pub fn current_char_offset(&self) -> usize {
        self.char_offset
    }

    /// Map a pointer position to an approximate document byte offset.
    pub fn hit_test_char_offset(&self, pos: egui::Pos2) -> Option<usize> {
        for target in self.text_hit_boxes.iter().rev() {
            if target.rect.contains(pos) {
                if let (Some(galley_pos), Some(galley)) =
                    (target.galley_pos, target.galley.as_ref())
                {
                    let local = pos - galley_pos;
                    let cursor = galley.cursor_from_pos(local);
                    let byte_offset =
                        byte_offset_for_char_index(galley.text(), cursor.ccursor.index);
                    return Some((target.start + byte_offset).min(target.end));
                }

                let span = target.end.saturating_sub(target.start);
                if span == 0 {
                    return Some(target.start);
                }
                let width = target.rect.width().max(1.0);
                let x = (pos.x - target.rect.left()).clamp(0.0, width);
                let ratio = x / width;
                let offset = target.start + (ratio * span as f32).floor() as usize;
                return Some(offset.min(target.end));
            }
        }
        None
    }

    /// Clear only the mermaid caches (call to retry failed renders)
    /// Useful when user installs mermaid-cli and wants to retry without reloading document
    #[allow(dead_code)]
    pub fn clear_mermaid_cache(&mut self) {
        // Remove only mermaid entries from image cache (O(n) but rarely called)
        self.image_cache.retain(|k, _| !k.starts_with("mermaid_"));
        self.mermaid_failed.clear();
        self.mermaid_pending.clear();
        self.mermaid_metadata_cache.clear();
    }

    /// Get cached mermaid metadata or compute and cache it
    /// Returns a reference to avoid cloning on cache hit
    fn get_mermaid_metadata(&mut self, code: &str, diagram_type: &str) -> &MermaidMetadata {
        // Generate cache key from code hash
        let cache_key = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(code.as_bytes());
            hex::encode(&hasher.finalize()[..8])
        };

        // Evict oldest entries when cache is full
        if self.mermaid_metadata_cache.len() >= MERMAID_METADATA_CACHE_MAX_SIZE
            && !self.mermaid_metadata_cache.contains_key(&cache_key)
        {
            // Remove ~25% of entries to amortize eviction cost
            let to_remove: Vec<String> = self
                .mermaid_metadata_cache
                .keys()
                .take(MERMAID_METADATA_CACHE_MAX_SIZE / 4)
                .cloned()
                .collect();
            for key in to_remove {
                self.mermaid_metadata_cache.remove(&key);
            }
        }

        // Use entry API to avoid clone - returns reference to cached or newly inserted value
        // Only allocate owned strings inside the cache-miss closure
        self.mermaid_metadata_cache
            .entry(cache_key)
            .or_insert_with(|| Self::compute_mermaid_metadata(code, diagram_type))
    }

    /// Compute mermaid metadata for a diagram (helper for caching)
    fn compute_mermaid_metadata(code: &str, diagram_type: &str) -> MermaidMetadata {
        match diagram_type {
            "flowchart" | "graph" => {
                let nodes: Vec<String> = code
                    .lines()
                    .filter(|l| {
                        !l.trim().starts_with("graph") && !l.trim().starts_with("flowchart")
                    })
                    .filter(|l| !l.trim().is_empty())
                    .take(6)
                    .map(extract_node_text)
                    .collect();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: nodes.len(),
                    count2: 0,
                    nodes,
                }
            }
            "sequence" => {
                // Single-pass counting of participants and arrows
                let (participant_count, arrow_count) = code.lines().fold((0, 0), |(p, a), l| {
                    let trimmed = l.trim();
                    let is_participant =
                        trimmed.starts_with("participant") || trimmed.starts_with("actor");
                    let is_arrow = l.contains("->>") || l.contains("-->>") || l.contains("->");
                    (p + is_participant as usize, a + is_arrow as usize)
                });
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: participant_count,
                    count2: arrow_count,
                    nodes: Vec::new(),
                }
            }
            "class" => {
                let class_count = code
                    .lines()
                    .filter(|l| l.trim().starts_with("class "))
                    .count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: class_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            "state" => {
                let state_count = code
                    .lines()
                    .filter(|l| {
                        let t = l.trim();
                        t.starts_with("state ") || t.contains(" --> ")
                    })
                    .count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: state_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            "gantt" => {
                let task_count = code.lines().filter(|l| l.contains(':')).count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: task_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            "pie" => {
                let slice_count = code.lines().filter(|l| l.trim().starts_with('"')).count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: slice_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            "er" | "erDiagram" => {
                let entity_count = code
                    .lines()
                    .filter(|l| l.contains('{') || l.contains("||"))
                    .count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: entity_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            "journey" => {
                let section_count = code
                    .lines()
                    .filter(|l| l.trim().starts_with("section"))
                    .count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: section_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            "gitGraph" => {
                // Single-pass counting of commits and branches
                let (commit_count, branch_count) = code.lines().fold((0, 0), |(c, b), l| {
                    let trimmed = l.trim();
                    let is_commit = trimmed.starts_with("commit");
                    let is_branch = trimmed.starts_with("branch");
                    (c + is_commit as usize, b + is_branch as usize)
                });
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: commit_count,
                    count2: branch_count,
                    nodes: Vec::new(),
                }
            }
            "mindmap" => {
                let node_count = code
                    .lines()
                    .filter(|l| !l.trim().is_empty() && !l.trim().starts_with("mindmap"))
                    .count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: node_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            "timeline" => {
                let event_count = code.lines().filter(|l| l.contains(':')).count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: event_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            "requirementDiagram" => {
                let req_count = code
                    .lines()
                    .filter(|l| {
                        l.trim().starts_with("requirement") || l.trim().starts_with("element")
                    })
                    .count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: req_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
            _ => {
                let line_count = code.lines().count();
                MermaidMetadata {
                    diagram_type: diagram_type.to_string(),
                    count1: line_count,
                    count2: 0,
                    nodes: Vec::new(),
                }
            }
        }
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
                        let texture =
                            ctx.load_texture(&cache_key, color_image, egui::TextureOptions::LINEAR);
                        self.cache_image(cache_key.clone(), texture);
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
        targets: RenderTargets,
    ) {
        self.scroll_target = targets.heading;
        self.search_target = targets.search_offset;
        self.active_search_range = targets.active_search_range;
        self.render(ui, events, annotations, heading_positions, config);
        self.scroll_target = None;
        self.search_target = None;
        self.active_search_range = None;
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

        // Capture visible rect for viewport culling
        // Use clip_rect which represents the visible scrolled area
        self.visible_rect = Some(ui.clip_rect());

        // Set image max width from config
        self.image_max_width = config.layout.image_width;

        // Build annotation index only when there are annotations (skip for empty stores)
        let annotation_index = if annotations.is_empty() {
            None
        } else {
            Some(AnnotationIndex::new(annotations))
        };
        // Empty index for passing to methods that require a reference
        let empty_index = AnnotationIndex {
            sorted_annotations: Vec::new(),
        };
        let ann_index_ref = annotation_index.as_ref().unwrap_or(&empty_index);

        let base_font_size = config.theme.fonts.size;
        let spacing = &config.theme.spacing;

        // Compute or reuse block map for event-level viewport culling.
        // Invalidate when events change OR available width changes significantly
        // (width changes affect height estimates for text wrapping).
        let events_id = events.as_ptr() as usize;
        let available_width = ui.available_width();
        // Round width to nearest 10px to avoid thrashing on subpixel changes
        let width_key = (available_width / 10.0) as u32;
        let layout_key = render_layout_cache_key(config);
        let mut block_map = match self.cached_block_map.take() {
            Some((id, w, key, blocks))
                if id == events_id && w == width_key && key == layout_key =>
            {
                blocks
            }
            Some((id, _w, key, old_blocks)) if id == events_id && key == layout_key => {
                // Width changed but same document/style — recompute estimates but preserve
                // actual_height measurements from previous frames to avoid layout jumps.
                let mut new_blocks = compute_block_map(events, available_width);
                for (new_block, old_block) in new_blocks.iter_mut().zip(old_blocks.iter()) {
                    if new_block.event_start == old_block.event_start
                        && new_block.event_end == old_block.event_end
                    {
                        new_block.actual_height = old_block.actual_height;
                    }
                }
                new_blocks
            }
            _ => compute_block_map(events, available_width),
        };

        // Use block-level culling: skip entire blocks that are off-screen
        let visible_rect = self.visible_rect;
        let buffer = visible_rect
            .map(|r| r.height() * 2.0)
            .unwrap_or(DEFAULT_VIEWPORT_BUFFER);
        let viewport_top = visible_rect.map(|r| r.top()).unwrap_or(0.0);
        let viewport_bottom = visible_rect.map(|r| r.bottom()).unwrap_or(f32::MAX);

        // When a scroll target is active, find the block containing that heading
        // so we can force-render it even if it appears off-screen
        let scroll_target_block_idx = self.scroll_target.and_then(|target_heading| {
            block_map
                .iter()
                .position(|b| b.heading_index == Some(target_heading))
        });

        for (block_idx, block) in block_map.iter_mut().enumerate() {
            let cursor_y = ui.cursor().top();
            let block_start_offset = self.char_offset;
            let block_end_offset = block_start_offset.saturating_add(block.text_byte_len);

            // Use actual measured height when available, otherwise estimated
            let block_height = block.height();

            // Use a larger buffer for blocks that have never been measured so they get
            // rendered and their actual height recorded before they reach the visible area.
            // This prevents layout jumps when estimated heights differ from actual heights.
            let effective_buffer = if block.actual_height.is_none() {
                buffer * 1.5 // 3x viewport for unmeasured blocks (buffer is already 2x viewport)
            } else {
                buffer // 2x viewport for blocks with known heights
            };

            // Check if this block is far outside the viewport
            let is_above_viewport = cursor_y + block_height < viewport_top - effective_buffer;
            let is_below_viewport = cursor_y > viewport_bottom + effective_buffer;

            // Special cases: always process headings (for TOC positions), footnote definitions,
            // and blocks containing scroll/search targets.
            let is_scroll_target_block = scroll_target_block_idx == Some(block_idx);
            let is_search_target_block = self
                .search_target
                .is_some_and(|target| target >= block_start_offset && target <= block_end_offset);
            let must_process = block.is_heading
                || is_scroll_target_block
                || is_search_target_block
                || matches!(
                    events.get(block.event_start),
                    Some(Event::Start(Tag::FootnoteDefinition(_)))
                );

            if !must_process && (is_above_viewport || is_below_viewport) {
                // Skip this entire block - allocate space and advance char_offset
                let w = ui.available_width();
                ui.allocate_space(Vec2::new(w, block_height));
                self.char_offset += block.text_byte_len;
                self.culled_count += 1;
                continue;
            }

            // Record cursor position before rendering this block for height measurement
            let pre_render_y = ui.cursor().top();
            if is_search_target_block {
                let animation = egui::style::ScrollAnimation {
                    points_per_second: 800.0,
                    duration: egui::emath::Rangef::new(0.2, 0.4),
                };
                ui.scroll_to_cursor_animation(Some(egui::Align::Center), animation);
            }

            // Process all events in this block normally
            for event in &events[block.event_start..block.event_end] {
                match event {
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
                    Event::End(tag) => {
                        self.handle_end_tag(tag, ui, base_font_size, spacing, config, ann_index_ref)
                    }
                    Event::Text(text) => self.handle_text(text),
                    Event::Code(code) => self.handle_inline_code(code),
                    Event::InlineMath(math) => self.handle_inline_math(math),
                    Event::DisplayMath(math) => self.render_display_math(ui, math, base_font_size),
                    Event::Html(html) => self.handle_html(html, config.markdown.html),
                    Event::InlineHtml(html) => self.handle_inline_html(html, config.markdown.html),
                    Event::SoftBreak => self.text_buffer.push(' '),
                    Event::HardBreak => self.text_buffer.push('\n'),
                    Event::Rule => self.render_horizontal_rule(ui),
                    Event::TaskListMarker(checked) => self.task_list_marker = Some(*checked),
                    Event::FootnoteReference(name) => {
                        let num = {
                            let next_num = self.footnote_counter.len() + 1;
                            *self
                                .footnote_counter
                                .entry(name.to_string())
                                .or_insert(next_num)
                        };
                        if self.current_link.is_some() {
                            self.text_buffer.push_str(&format!("[{}]", num));
                        } else {
                            self.text_buffer
                                .push_str(&format!("\x01FN:{}:{}\x01", name, num));
                        }
                    }
                }
            }

            // Measure actual rendered height and store for future frames.
            // This improves scroll accuracy on subsequent renders since we replace
            // rough estimates with precise measurements.
            let post_render_y = ui.cursor().top();
            let measured = post_render_y - pre_render_y;
            if measured > 0.0 {
                block.actual_height = Some(measured);
            }
        }

        // Store the block map back with updated actual heights
        self.cached_block_map = Some((events_id, width_key, layout_key, block_map));

        // Render footnote definitions at the end if any exist
        if !self.footnote_definitions.is_empty() {
            self.render_footnote_definitions(ui, base_font_size);
        }
    }

    fn reset(&mut self) {
        self.text_buffer.clear();
        self.heading_level = 0;
        self.in_code_block = false;
        self.in_html_block = false;
        self.html_content.clear();
        self.in_metadata_block = false;
        self.metadata_kind = None;
        self.metadata_content.clear();
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
        // Reuse Vec capacity - clear without deallocating
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
        self.paragraph_start_offset = None;
        self.heading_index = 0;
        self.table_count = 0;
        self.code_block_count = 0;
        self.culled_count = 0;
        self.block_y_offset = 0.0;
        self.text_hit_boxes.clear();
    }

    fn handle_start_tag(&mut self, tag: &Tag<'_>, ui: &mut Ui, heading_positions: &mut Vec<f32>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.heading_level = super::parser::heading_level_to_usize(*level);
                // Record heading position for TOC navigation
                heading_positions.push(ui.cursor().top());
                // Check if this is the scroll target - if so, scroll now (before rendering)
                // Use a custom animation for smooth scrolling (200-400ms ease)
                if self.scroll_target == Some(self.heading_index) {
                    let animation = egui::style::ScrollAnimation {
                        points_per_second: 800.0,
                        duration: egui::emath::Rangef::new(0.2, 0.4),
                    };
                    ui.scroll_to_cursor_animation(Some(egui::Align::TOP), animation);
                }
                self.heading_index += 1;
            }
            Tag::Paragraph => {
                self.paragraph_start_offset = Some(self.char_offset);
            }
            Tag::BlockQuote(_) => {
                self.in_blockquote = true;
            }
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_language = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
                    _ => None,
                };
            }
            Tag::HtmlBlock => {
                self.in_html_block = true;
                self.html_content.clear();
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
                self.text_buffer.push(LINK_MARKER);
                self.text_buffer.push_str(dest_url);
                self.text_buffer.push(LINK_MARKER);
            }
            Tag::Image {
                dest_url, title, ..
            } => {
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
            Tag::MetadataBlock(kind) => {
                self.in_metadata_block = true;
                self.metadata_kind = Some(*kind);
                self.metadata_content.clear();
            }
            Tag::DefinitionList => {
                ui.add_space(4.0);
            }
            Tag::DefinitionListTitle => {
                self.text_buffer.clear();
            }
            Tag::DefinitionListDefinition => {}
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
        annotation_index: &AnnotationIndex<'_>,
    ) {
        match tag {
            TagEnd::Heading(_level) => {
                // Apply heading color from config if specified
                let heading_color = config
                    .theme
                    .colors
                    .heading
                    .as_ref()
                    .map(|c| parse_config_hex_color(c));
                self.render_heading_with_config(ui, base_font_size, spacing, heading_color);
                self.heading_level = 0;
            }
            TagEnd::Paragraph => {
                if self.in_blockquote {
                    self.render_blockquote(ui, base_font_size);
                } else {
                    // Apply code_text color from config if specified
                    let code_text_color = config
                        .theme
                        .colors
                        .code_text
                        .as_ref()
                        .map(|c| parse_config_hex_color(c));
                    self.render_paragraph(
                        ui,
                        base_font_size,
                        spacing,
                        annotation_index,
                        code_text_color,
                    );
                }
                self.paragraph_start_offset = None;
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
            TagEnd::HtmlBlock => {
                if config.markdown.html {
                    let html = std::mem::take(&mut self.html_content);
                    self.render_raw_block(ui, "HTML", &html, base_font_size);
                } else {
                    self.html_content.clear();
                }
                self.in_html_block = false;
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
                self.text_buffer.push(LINK_END_MARKER);
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
                let cell = std::mem::take(&mut self.text_buffer);
                self.table_row.push(strip_inline_markers(&cell));
            }
            TagEnd::MetadataBlock(kind) => {
                let title = match kind {
                    MetadataBlockKind::YamlStyle => "Metadata",
                    MetadataBlockKind::PlusesStyle => "Metadata",
                };
                let metadata = std::mem::take(&mut self.metadata_content);
                self.render_raw_block(ui, title, &metadata, base_font_size);
                self.metadata_kind = None;
                self.in_metadata_block = false;
            }
            TagEnd::DefinitionListTitle => {
                let text = strip_inline_markers(&std::mem::take(&mut self.text_buffer));
                if !text.trim().is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(text).size(base_font_size).strong());
                }
            }
            TagEnd::DefinitionListDefinition => {
                ui.add_space(2.0);
            }
            TagEnd::DefinitionList => {
                ui.add_space(6.0);
            }
            _ => {}
        }
    }

    fn handle_text(&mut self, text: &pulldown_cmark::CowStr<'_>) {
        if self.in_code_block {
            self.code_content.push_str(text);
        } else if self.in_html_block {
            self.html_content.push_str(text);
        } else if self.in_metadata_block {
            self.metadata_content.push_str(text);
        } else {
            self.text_buffer.push_str(text);
        }

        // Use byte length for offset tracking — annotations and pulldown-cmark
        // both use byte offsets into the source document.
        self.char_offset += text.len();
    }

    fn handle_inline_code(&mut self, code: &pulldown_cmark::CowStr<'_>) {
        // If we're inside a link, keep inline code as plain link text so link tokens stay valid.
        if self.current_link.is_some() {
            self.text_buffer.push_str(code);
        } else {
            // Store inline code with markers for later rendering
            self.text_buffer.push_str("\x00CODE:");
            self.text_buffer.push_str(code);
            self.text_buffer.push(INLINE_CODE_MARKER);
        }

        self.char_offset += code.len();
    }

    fn handle_inline_math(&mut self, math: &pulldown_cmark::CowStr<'_>) {
        self.text_buffer.push('$');
        self.text_buffer.push_str(math);
        self.text_buffer.push('$');
        self.char_offset += math.len();
    }

    fn handle_html(&mut self, html: &pulldown_cmark::CowStr<'_>, enabled: bool) {
        if !enabled {
            self.char_offset += html.len();
            return;
        }

        if self.in_html_block {
            self.html_content.push_str(html);
        } else {
            self.text_buffer.push_str(html);
        }
        self.char_offset += html.len();
    }

    fn handle_inline_html(&mut self, html: &pulldown_cmark::CowStr<'_>, enabled: bool) {
        if enabled {
            self.text_buffer.push_str(html);
        }
        self.char_offset += html.len();
    }

    fn render_display_math(
        &mut self,
        ui: &mut Ui,
        math: &pulldown_cmark::CowStr<'_>,
        base_font_size: f32,
    ) {
        self.render_raw_block(ui, "Math", math, base_font_size);
        self.char_offset += math.len();
    }

    fn render_raw_block(&self, ui: &mut Ui, label: &str, content: &str, base_font_size: f32) {
        let content = content.trim();
        if content.is_empty() {
            return;
        }

        let is_dark = ui.visuals().dark_mode;
        let bg = theme_colors::code_block_bg(is_dark);
        let text_color = ui.style().visuals.text_color();
        ui.add_space(6.0);
        egui::Frame::none()
            .fill(bg)
            .stroke(Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(label)
                        .size(base_font_size * 0.75)
                        .strong()
                        .color(theme_colors::code_line_number(is_dark)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(content)
                        .size(base_font_size * 0.9)
                        .monospace()
                        .color(text_color),
                );
            });
        ui.add_space(6.0);
    }

    fn render_heading_with_config(
        &mut self,
        ui: &mut Ui,
        base_font_size: f32,
        spacing: &crate::config::schema::SpacingConfig,
        heading_color: Option<Color32>,
    ) {
        let text = strip_inline_markers(&std::mem::take(&mut self.text_buffer));
        if text.is_empty() {
            return;
        }

        // Scale heading spacing by level: H1 gets full spacing, H6 gets less
        let level = self.heading_level;
        let spacing_scale = match level {
            1 => 1.4,
            2 => 1.2,
            3 => 1.0,
            4 => 0.85,
            5 => 0.7,
            _ => 0.6,
        };
        let top_space = spacing.heading_top * spacing_scale;
        let bottom_space = spacing.heading_bottom * spacing_scale;

        ui.add_space(top_space);

        let size_multiplier = heading_size_multiplier(level);
        let font_size = base_font_size * size_multiplier;

        let mut rich_text = RichText::new(&text).size(font_size).strong();

        // H5 and H6 use lighter weight to further differentiate from larger headings
        if level >= 5 {
            rich_text = RichText::new(&text).size(font_size);
            // H6 uses muted/secondary text appearance
            if level >= 6 {
                let is_dark = ui.visuals().dark_mode;
                let muted_color = if is_dark {
                    Color32::from_rgb(160, 160, 176)
                } else {
                    Color32::from_rgb(90, 90, 105)
                };
                rich_text = rich_text.color(muted_color);
            }
        }

        // Apply custom heading color if provided (overrides level-based color)
        if let Some(color) = heading_color {
            rich_text = rich_text.color(color);
        }

        let heading_start = self.char_offset.saturating_sub(text.len());
        let heading_end = self.char_offset;
        self.add_label_with_hit(
            ui,
            egui::Label::new(rich_text).wrap_mode(egui::TextWrapMode::Wrap),
            heading_start,
            heading_end,
            ui.style().visuals.text_color(),
            Stroke::NONE,
        );

        ui.add_space(bottom_space);
    }

    fn render_paragraph(
        &mut self,
        ui: &mut Ui,
        base_font_size: f32,
        spacing: &crate::config::schema::SpacingConfig,
        annotation_index: &AnnotationIndex<'_>,
        code_text_color: Option<Color32>,
    ) {
        let text = std::mem::take(&mut self.text_buffer);
        if text.is_empty() {
            return;
        }

        // Offsets are tracked while processing events (handle_text/handle_inline_code), so
        // paragraph boundaries use the captured start and current offset here.
        let start_offset = self.paragraph_start_offset.unwrap_or(self.char_offset);
        let end_offset = self.char_offset;

        // Check if any annotations overlap with this text (O(log n) binary search)
        let overlapping = annotation_index.in_range(start_offset, end_offset);
        let bookmark_count = overlapping
            .iter()
            .filter(|ann| ann.kind == AnnotationKind::Bookmark)
            .count();

        if bookmark_count > 0 {
            let marker = if bookmark_count == 1 {
                "🔖 Bookmark".to_string()
            } else {
                format!("🔖 {} bookmarks", bookmark_count)
            };
            let marker_color = if ui.visuals().dark_mode {
                Color32::from_rgb(140, 180, 220)
            } else {
                Color32::from_rgb(70, 110, 150)
            };
            ui.label(RichText::new(marker).small().italics().color(marker_color));
            ui.add_space(2.0);
        }

        // Parse and render mixed content (text + inline code) with annotations
        // Annotations are already sorted from the index, no need to re-sort
        self.render_mixed_content_with_annotations(
            ui,
            &text,
            MixedContentStyle {
                base_font_size,
                start_offset,
                code_text_color,
                in_strikethrough: self.in_strikethrough,
            },
            &overlapping,
        );

        ui.add_space(spacing.paragraph);
    }

    fn record_text_hit_with_galley(
        &mut self,
        rect: egui::Rect,
        start: usize,
        end: usize,
        galley_pos: Option<Pos2>,
        galley: Option<Arc<egui::Galley>>,
    ) {
        if end > start {
            self.text_hit_boxes.push(TextHitTarget {
                rect,
                start,
                end,
                galley_pos,
                galley,
            });
        }
    }

    fn add_label_with_hit(
        &mut self,
        ui: &mut Ui,
        label: Label,
        start: usize,
        end: usize,
        fallback_color: Color32,
        underline: Stroke,
    ) -> egui::Response {
        let (galley_pos, galley, response) = label.layout_in_ui(ui);
        if ui.is_rect_visible(response.rect) {
            ui.painter().add(
                epaint::TextShape::new(galley_pos, galley.clone(), fallback_color)
                    .with_underline(underline),
            );
        }
        self.record_text_hit_with_galley(response.rect, start, end, Some(galley_pos), Some(galley));
        response
    }

    fn render_annotated_text_run(
        &mut self,
        ui: &mut Ui,
        text: &str,
        base_font_size: f32,
        start_offset: usize,
        annotations: &[&Annotation],
        in_strikethrough: bool,
    ) {
        if text.is_empty() {
            return;
        }

        let end_offset = start_offset + text.len();
        let mut boundaries = Vec::with_capacity(annotations.len() * 2 + 2);
        boundaries.push(start_offset);
        boundaries.push(end_offset);

        for ann in annotations {
            if ann.start > start_offset && ann.start < end_offset {
                boundaries.push(ann.start);
            }
            if ann.end > start_offset && ann.end < end_offset {
                boundaries.push(ann.end);
            }
        }

        if let Some((search_start, search_end)) = self.active_search_range {
            if search_start < end_offset && search_end > start_offset {
                boundaries.push(search_start.clamp(start_offset, end_offset));
                boundaries.push(search_end.clamp(start_offset, end_offset));
            }
        }

        boundaries.sort_unstable();
        boundaries.dedup();

        for boundary_window in boundaries.windows(2) {
            let run_start = boundary_window[0];
            let run_end = boundary_window[1];
            if run_end <= run_start {
                continue;
            }

            let local_start = floor_to_char_boundary(text, run_start.saturating_sub(start_offset));
            let local_end = floor_to_char_boundary(text, run_end.saturating_sub(start_offset));
            if local_end <= local_start {
                continue;
            }

            let run_text = &text[local_start..local_end];
            if run_text.is_empty() {
                continue;
            }

            let highlight = annotation_highlight_color(annotations, run_start);
            let note_here = annotation_has_note(annotations, run_start);
            let search_hit = self
                .active_search_range
                .is_some_and(|(start, end)| run_start < end && run_end > start);

            let mut rich = RichText::new(run_text).size(base_font_size);
            if let Some(bg_color) = highlight {
                rich = rich.background_color(bg_color);
            } else if search_hit {
                rich = rich.background_color(if ui.visuals().dark_mode {
                    Color32::from_rgb(106, 85, 28)
                } else {
                    Color32::from_rgb(255, 228, 130)
                });
            }
            if note_here {
                rich = rich.underline();
                if highlight.is_none() {
                    rich = rich.color(Color32::from_rgb(100, 149, 237));
                }
            }
            if in_strikethrough {
                rich = rich.strikethrough();
            }
            self.add_label_with_hit(
                ui,
                egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Wrap),
                run_start,
                run_end,
                ui.style().visuals.text_color(),
                Stroke::NONE,
            );
        }
    }

    fn render_mixed_content_with_annotations(
        &mut self,
        ui: &mut Ui,
        text: &str,
        style: MixedContentStyle,
        annotations: &[&Annotation],
    ) {
        // Fast path: no annotations and no inline code or footnotes - use single label
        let has_special_markers = text.contains(INLINE_CODE_MARKER)
            || text.contains(FOOTNOTE_MARKER)
            || text.contains(LINK_MARKER)
            || text.contains(LINK_END_MARKER);
        let has_active_search = self.active_search_range.is_some_and(|(start, end)| {
            start < style.start_offset + text.len() && end > style.start_offset
        });
        if annotations.is_empty()
            && !has_active_search
            && !has_special_markers
            && !style.in_strikethrough
        {
            ui.horizontal_wrapped(|ui| {
                self.add_label_with_hit(
                    ui,
                    egui::Label::new(RichText::new(text).size(style.base_font_size))
                        .wrap_mode(egui::TextWrapMode::Wrap),
                    style.start_offset,
                    style.start_offset + text.len(),
                    ui.style().visuals.text_color(),
                    Stroke::NONE,
                );
            });
            return;
        }

        // Fast path: no annotations, no footnotes, but has inline code - minimal processing
        if annotations.is_empty()
            && !has_active_search
            && !text.contains(FOOTNOTE_MARKER)
            && !text.contains(LINK_MARKER)
            && !text.contains(LINK_END_MARKER)
        {
            ui.horizontal_wrapped(|ui| {
                let mut current_offset = style.start_offset;
                for part in text.split(INLINE_CODE_MARKER) {
                    if let Some(code) = part.strip_prefix("CODE:") {
                        let text_color = style
                            .code_text_color
                            .unwrap_or(Color32::from_rgb(206, 145, 120));
                        let mut code_text = RichText::new(code)
                            .size(style.base_font_size * 0.9)
                            .monospace()
                            .color(text_color)
                            .background_color(Color32::from_gray(50));
                        if style.in_strikethrough {
                            code_text = code_text.strikethrough();
                        }
                        self.add_label_with_hit(
                            ui,
                            egui::Label::new(code_text).wrap_mode(egui::TextWrapMode::Wrap),
                            current_offset,
                            current_offset + code.len(),
                            text_color,
                            Stroke::NONE,
                        );
                        current_offset += code.len();
                    } else if !part.is_empty() {
                        let mut rich = RichText::new(part).size(style.base_font_size);
                        if style.in_strikethrough {
                            rich = rich.strikethrough();
                        }
                        self.add_label_with_hit(
                            ui,
                            egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Wrap),
                            current_offset,
                            current_offset + part.len(),
                            ui.style().visuals.text_color(),
                            Stroke::NONE,
                        );
                        current_offset += part.len();
                    }
                }
            });
            return;
        }

        ui.horizontal_wrapped(|ui| {
            let mut current_offset = style.start_offset;
            let is_dark = ui.visuals().dark_mode;

            // Split on inline code markers - use iterator directly (no allocation)
            for part in text.split(INLINE_CODE_MARKER) {
                if let Some(code) = part.strip_prefix("CODE:") {
                    // Apply code_text color from config if specified, otherwise use default
                    let text_color = style
                        .code_text_color
                        .unwrap_or(Color32::from_rgb(206, 145, 120));
                    let mut code_text = RichText::new(code)
                        .size(style.base_font_size * 0.9)
                        .monospace()
                        .color(text_color)
                        .background_color(Color32::from_gray(50));
                    if style.in_strikethrough {
                        code_text = code_text.strikethrough();
                    }
                    self.add_label_with_hit(
                        ui,
                        egui::Label::new(code_text).wrap_mode(egui::TextWrapMode::Wrap),
                        current_offset,
                        current_offset + code.len(),
                        text_color,
                        Stroke::NONE,
                    );
                    current_offset += code.len();
                } else if !part.is_empty() {
                    // Handle footnote references within the text - use iterator directly
                    for fn_part in part.split(FOOTNOTE_MARKER) {
                        if let Some(fn_ref) = fn_part.strip_prefix("FN:") {
                            // Format is "name:num"
                            if let Some((_name, num_str)) = fn_ref.split_once(':') {
                                // Render as superscript number
                                let superscript = RichText::new(format!("[{}]", num_str))
                                    .size(style.base_font_size * 0.75)
                                    .color(Color32::from_rgb(78, 201, 176))
                                    .raised();
                                ui.label(superscript);
                            }
                        } else if !fn_part.is_empty() {
                            let mut remaining = fn_part;
                            while !remaining.is_empty() {
                                if let Some(link_start_idx) = remaining.find(LINK_MARKER) {
                                    let before_link = &remaining[..link_start_idx];
                                    if !before_link.is_empty() {
                                        self.render_annotated_text_run(
                                            ui,
                                            before_link,
                                            style.base_font_size,
                                            current_offset,
                                            annotations,
                                            style.in_strikethrough,
                                        );
                                        current_offset += before_link.len();
                                    }

                                    let after_link_start =
                                        &remaining[link_start_idx + LINK_MARKER.len_utf8()..];
                                    if let Some(url_end_idx) = after_link_start.find(LINK_MARKER) {
                                        let url = &after_link_start[..url_end_idx];
                                        let after_url = &after_link_start
                                            [url_end_idx + LINK_MARKER.len_utf8()..];
                                        if let Some(link_end_idx) = after_url.find(LINK_END_MARKER)
                                        {
                                            let link_text = &after_url[..link_end_idx];
                                            if !link_text.is_empty() {
                                                let mut link_rich = RichText::new(link_text)
                                                    .size(style.base_font_size)
                                                    .color(theme_colors::link_color(is_dark))
                                                    .underline();
                                                if let Some(bg) = annotation_highlight_color(
                                                    annotations,
                                                    current_offset,
                                                ) {
                                                    link_rich = link_rich.background_color(bg);
                                                }
                                                if style.in_strikethrough {
                                                    link_rich = link_rich.strikethrough();
                                                }
                                                let response = self.add_label_with_hit(
                                                    ui,
                                                    egui::Label::new(link_rich)
                                                        .wrap_mode(egui::TextWrapMode::Wrap)
                                                        .sense(Sense::click()),
                                                    current_offset,
                                                    current_offset + link_text.len(),
                                                    theme_colors::link_color(is_dark),
                                                    Stroke::NONE,
                                                );
                                                if response.hovered() {
                                                    ui.ctx().set_cursor_icon(
                                                        egui::CursorIcon::PointingHand,
                                                    );
                                                }
                                                if response.clicked() {
                                                    if let Err(e) = open::that(url) {
                                                        log::error!("Failed to open link: {}", e);
                                                    }
                                                }
                                                current_offset += link_text.len();
                                            }

                                            remaining = &after_url
                                                [link_end_idx + LINK_END_MARKER.len_utf8()..];
                                            continue;
                                        }
                                    }

                                    // Malformed token fallback: render the rest as plain text.
                                    let fallback = strip_inline_markers(remaining);
                                    self.render_annotated_text_run(
                                        ui,
                                        &fallback,
                                        style.base_font_size,
                                        current_offset,
                                        annotations,
                                        style.in_strikethrough,
                                    );
                                    current_offset += fallback.len();
                                    break;
                                }

                                self.render_annotated_text_run(
                                    ui,
                                    remaining,
                                    style.base_font_size,
                                    current_offset,
                                    annotations,
                                    style.in_strikethrough,
                                );
                                current_offset += remaining.len();
                                break;
                            }
                        }
                    }
                }
            }
        });
    }

    fn render_blockquote(&mut self, ui: &mut Ui, base_font_size: f32) {
        let text = strip_inline_markers(&std::mem::take(&mut self.text_buffer));
        if text.is_empty() {
            return;
        }

        let bar_width = 3.0;
        let bar_spacing = 12.0;
        let indent = bar_width + bar_spacing;

        // Render text in an indented sub-region so it wraps within the available width
        let outer_rect = ui.available_rect_before_wrap();
        let text_width = (outer_rect.width() - indent).max(1.0);

        let response = ui.allocate_ui_with_layout(
            Vec2::new(outer_rect.width(), 0.0),
            egui::Layout::left_to_right(egui::Align::Min),
            |ui| {
                ui.add_space(indent);
                ui.allocate_ui_with_layout(
                    Vec2::new(text_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let quote_text = RichText::new(&text)
                            .size(base_font_size)
                            .italics()
                            .color(Color32::from_gray(180));

                        let quote_start = self.char_offset.saturating_sub(text.len());
                        let quote_end = self.char_offset;
                        self.add_label_with_hit(
                            ui,
                            egui::Label::new(quote_text).wrap_mode(egui::TextWrapMode::Wrap),
                            quote_start,
                            quote_end,
                            Color32::from_gray(180),
                            Stroke::NONE,
                        );
                    },
                );
            },
        );

        // Paint the vertical bar to match the actual rendered height
        let bar_rect = egui::Rect::from_min_size(
            outer_rect.min,
            Vec2::new(bar_width, response.response.rect.height()),
        );
        ui.painter()
            .rect_filled(bar_rect, 0.0, Color32::from_gray(100));

        ui.add_space(8.0);
    }

    fn render_code_block(&mut self, ui: &mut Ui, config: &Config) {
        let code = std::mem::take(&mut self.code_content);
        if code.is_empty() {
            return;
        }

        // Check if this is a Mermaid diagram
        let is_mermaid = self
            .code_language
            .as_deref()
            .map(|l| l.to_lowercase() == "mermaid")
            .unwrap_or(false);

        if is_mermaid {
            self.render_mermaid_diagram(ui, &code);
            return;
        }

        let padding = config.theme.spacing.code_padding;
        let is_dark = ui.visuals().dark_mode;

        // Copy language label for use (avoids borrow issues)
        let lang_label = self.code_language.clone();

        // Viewport culling: skip expensive syntax highlighting if not visible
        // We still render a placeholder frame to maintain layout
        let cursor_y = ui.cursor().top();
        let is_visible = self.is_position_visible(cursor_y);

        let show_line_numbers = config.markdown.show_line_numbers;

        // Try syntax highlighting with caching - only if visible or already cached
        #[cfg(feature = "syntax-highlighting")]
        let highlighted_job = if config.markdown.syntax_highlighting && is_visible {
            self.get_highlighted_code(
                &code,
                lang_label.as_deref(),
                &config.markdown.syntax_theme,
                is_dark,
                show_line_numbers,
            )
        } else if config.markdown.syntax_highlighting {
            // Check cache without computing - if already cached, use it
            let cache_key = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(code.as_bytes());
                hasher.update(lang_label.as_deref().unwrap_or("").as_bytes());
                hasher.update(config.markdown.syntax_theme.as_bytes());
                hasher.update(if is_dark { "dark" } else { "light" });
                hasher.update(if show_line_numbers { "ln" } else { "" });
                hex::encode(&hasher.finalize()[..12])
            };
            self.syntax_cache.get(&cache_key).cloned()
        } else {
            None
        };

        #[cfg(not(feature = "syntax-highlighting"))]
        let _ = is_visible; // Suppress unused warning

        egui::Frame::none()
            .fill(theme_colors::code_block_bg(is_dark))
            .inner_margin(padding)
            .outer_margin(egui::Margin::symmetric(0.0, 4.0))
            .rounding(4.0)
            .show(ui, |ui| {
                // Show language label if present
                if let Some(lang) = &lang_label {
                    ui.label(
                        RichText::new(lang)
                            .small()
                            .color(theme_colors::code_line_number(is_dark)),
                    );
                    ui.add_space(4.0);
                }

                // Try syntax highlighting if enabled (use cached result)
                let mut rendered = false;

                #[cfg(feature = "syntax-highlighting")]
                if let Some(job) = highlighted_job {
                    ui.label((*job).clone());
                    rendered = true;
                }

                // Fallback: plain monospace text (with optional line numbers)
                if !rendered {
                    if show_line_numbers {
                        let line_count = code.lines().count();
                        let digits = if line_count == 0 {
                            1
                        } else {
                            (line_count as f32).log10().floor() as usize + 1
                        };
                        let mut numbered =
                            String::with_capacity(code.len() + line_count * (digits + 3));
                        for (i, line) in code.lines().enumerate() {
                            use std::fmt::Write;
                            let _ =
                                writeln!(numbered, "{:>width$}  {}", i + 1, line, width = digits);
                        }
                        ui.label(
                            RichText::new(&numbered)
                                .monospace()
                                .color(theme_colors::inline_code_text(is_dark)),
                        );
                    } else {
                        ui.label(
                            RichText::new(&code)
                                .monospace()
                                .color(theme_colors::inline_code_text(is_dark)),
                        );
                    }
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
            use sha2::{Digest, Sha256};
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
            self.render_mermaid_fallback(ui, code, diagram_type);
            self.code_block_count += 1;
            ui.add_space(8.0);
            return;
        }

        // Check if already rendering asynchronously
        if self.mermaid_pending.contains(&cache_key) {
            self.render_mermaid_loading(ui, code, diagram_type);
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
            self.render_mermaid_loading(ui, code, diagram_type);
            self.code_block_count += 1;
            ui.add_space(8.0);
            return;
        }

        // Fallback to text preview
        self.render_mermaid_fallback(ui, code, diagram_type);
        self.code_block_count += 1;
        ui.add_space(8.0);
    }

    fn render_mermaid_image(
        &self,
        ui: &mut Ui,
        texture: &TextureHandle,
        code: &str,
        diagram_type: &str,
    ) {
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
                            .color(Color32::from_rgb(255, 100, 120)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Mermaid: {}", diagram_type))
                            .strong()
                            .color(Color32::from_rgb(60, 70, 90)),
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
                        .color(Color32::from_gray(100)),
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
                                    .color(Color32::from_gray(80)),
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
                            .color(Color32::from_rgb(100, 140, 200)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Rendering {}...", diagram_type))
                            .color(Color32::from_rgb(80, 90, 110)),
                    );
                });

                ui.add_space(8.0);

                // Show a placeholder area
                let placeholder_height = 150.0;
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), placeholder_height),
                    egui::Sense::hover(),
                );
                ui.painter()
                    .rect_filled(rect, 4.0, Color32::from_rgb(230, 235, 240));

                // Request repaint to animate spinner
                ui.ctx().request_repaint();
            });
    }

    fn render_mermaid_fallback(&mut self, ui: &mut Ui, code: &str, diagram_type: &str) {
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
                            .color(Color32::from_rgb(255, 154, 162)),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Mermaid Diagram: {}", diagram_type))
                            .strong()
                            .color(Color32::from_rgb(180, 190, 210)),
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
                        .color(Color32::from_gray(140)),
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
                                    .color(Color32::from_rgb(150, 160, 180)),
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
                    ui.label(RichText::new("💡").size(16.0));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Install mermaid-cli for full diagram rendering")
                            .color(Color32::from_rgb(140, 180, 220)),
                    );
                });

                ui.add_space(8.0);

                // Step 1: Install Node.js (platform-specific)
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Step 1:")
                            .strong()
                            .color(Color32::from_rgb(180, 180, 200)),
                    );
                    ui.label(
                        RichText::new("Install Node.js (if not already installed)")
                            .color(Color32::from_rgb(160, 160, 180)),
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
                            .color(Color32::from_rgb(180, 180, 200)),
                    );
                    ui.label(
                        RichText::new("Install mermaid-cli")
                            .color(Color32::from_rgb(160, 160, 180)),
                    );
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    // Copy command button
                    if ui
                        .button(RichText::new("📋 Copy command").small())
                        .clicked()
                    {
                        ui.ctx()
                            .copy_text("npm install -g @mermaid-js/mermaid-cli".to_string());
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
                                .color(Color32::from_rgb(120, 200, 120)),
                        );
                    });

                ui.add_space(6.0);

                ui.label(
                    RichText::new(
                        "After installation, restart mdview to enable diagram rendering.",
                    )
                    .small()
                    .color(Color32::from_gray(130)),
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
                            .color(Color32::from_rgb(200, 180, 120)),
                    );
                });
        }

        #[cfg(target_os = "windows")]
        {
            ui.horizontal(|ui| {
                if ui.button(RichText::new("📋 winget").small()).clicked() {
                    ui.ctx()
                        .copy_text("winget install OpenJS.NodeJS".to_string());
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
                                .color(Color32::from_rgb(200, 180, 120)),
                        );
                        ui.label(
                            RichText::new("# or download installer from nodejs.org")
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(100)),
                        );
                    });
                });
        }

        #[cfg(target_os = "linux")]
        {
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("📋 apt (Debian/Ubuntu)").small())
                    .clicked()
                {
                    ui.ctx()
                        .copy_text("sudo apt install nodejs npm".to_string());
                }
                if ui
                    .button(RichText::new("📋 dnf (Fedora)").small())
                    .clicked()
                {
                    ui.ctx()
                        .copy_text("sudo dnf install nodejs npm".to_string());
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
                                .color(Color32::from_gray(100)),
                        );
                        ui.label(
                            RichText::new("sudo apt install nodejs npm")
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(200, 180, 120)),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("# Fedora:")
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.label(
                            RichText::new("sudo dnf install nodejs npm")
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(200, 180, 120)),
                        );
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("# Arch:")
                                .monospace()
                                .size(10.0)
                                .color(Color32::from_gray(100)),
                        );
                        ui.label(
                            RichText::new("sudo pacman -S nodejs npm")
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(200, 180, 120)),
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
                    .color(Color32::from_gray(140)),
            );
        }
    }

    fn render_mermaid_preview(&mut self, ui: &mut Ui, code: &str, diagram_type: &str) {
        // Get cached metadata to avoid re-parsing every frame
        let internal_type = match diagram_type {
            "Flowchart" | "Graph" => "flowchart",
            "Sequence Diagram" => "sequence",
            "Class Diagram" => "class",
            "State Diagram" => "state",
            "Gantt Chart" => "gantt",
            "Pie Chart" => "pie",
            "ER Diagram" => "er",
            "User Journey" => "journey",
            "Git Graph" => "gitGraph",
            "Mind Map" => "mindmap",
            "Timeline" => "timeline",
            "Quadrant Chart" => "quadrant",
            "Requirement Diagram" => "requirementDiagram",
            "C4 Diagram" => "c4",
            _ => "generic",
        };

        let metadata = self.get_mermaid_metadata(code, internal_type);

        // Render preview using cached metadata (metadata is already a reference)
        match diagram_type {
            "Flowchart" | "Graph" => Self::render_flowchart_preview_cached(ui, metadata),
            "Sequence Diagram" => Self::render_sequence_preview_cached(ui, metadata),
            "Class Diagram" => Self::render_class_preview_cached(ui, metadata),
            "State Diagram" => Self::render_state_preview_cached(ui, metadata),
            "Gantt Chart" => Self::render_gantt_preview_cached(ui, metadata),
            "Pie Chart" => Self::render_pie_preview_cached(ui, metadata),
            "ER Diagram" => Self::render_er_preview_cached(ui, metadata),
            "User Journey" => Self::render_journey_preview_cached(ui, metadata),
            "Git Graph" => Self::render_gitgraph_preview_cached(ui, metadata),
            "Mind Map" => Self::render_mindmap_preview_cached(ui, metadata),
            "Timeline" => Self::render_timeline_preview_cached(ui, metadata),
            "Quadrant Chart" => Self::render_quadrant_preview_cached(ui),
            "Requirement Diagram" => Self::render_requirement_preview_cached(ui, metadata),
            "C4 Diagram" => Self::render_c4_preview_cached(ui),
            _ => Self::render_generic_preview_cached(ui, metadata),
        }
    }

    fn render_flowchart_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        if metadata.nodes.is_empty() {
            Self::render_generic_preview_cached(ui, metadata);
            return;
        }

        ui.horizontal_wrapped(|ui| {
            for (i, text) in metadata.nodes.iter().enumerate() {
                egui::Frame::none()
                    .fill(Color32::from_rgb(70, 130, 180))
                    .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(text).color(Color32::WHITE).size(12.0));
                    });

                if i < metadata.nodes.len() - 1 {
                    ui.label(RichText::new(" → ").color(Color32::from_gray(120)));
                }
            }
        });

        if metadata.nodes.len() >= 6 {
            ui.label(
                RichText::new("...")
                    .color(Color32::from_gray(100))
                    .italics(),
            );
        }
    }

    fn render_sequence_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        let participant_count = metadata.count1;
        let arrow_count = metadata.count2;

        if participant_count == 0 && arrow_count == 0 {
            Self::render_generic_preview_cached(ui, metadata);
            return;
        }

        ui.horizontal_wrapped(|ui| {
            // Show participant boxes
            for i in 0..participant_count.min(4) {
                egui::Frame::none()
                    .fill(Color32::from_rgb(100, 140, 180))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .rounding(2.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("P{}", i + 1))
                                .color(Color32::WHITE)
                                .size(11.0),
                        );
                    });

                if i < participant_count.min(4) - 1 {
                    ui.label(RichText::new(" ").color(Color32::TRANSPARENT));
                }
            }
        });

        ui.label(
            RichText::new(format!(
                "📨 {} messages between {} participants",
                arrow_count.max(1),
                participant_count.max(1)
            ))
            .color(Color32::from_rgb(140, 180, 220)),
        );
    }

    fn render_class_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!("📦 {} classes defined", metadata.count1.max(1)))
                .color(Color32::from_rgb(180, 140, 200)),
        );
    }

    fn render_state_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!(
                "🔄 State machine with ~{} transitions",
                metadata.count1.max(1)
            ))
            .color(Color32::from_rgb(140, 200, 180)),
        );
    }

    fn render_gantt_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!(
                "📅 Gantt chart with {} tasks",
                metadata.count1.max(1)
            ))
            .color(Color32::from_rgb(200, 180, 140)),
        );
    }

    fn render_pie_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!(
                "🥧 Pie chart with {} slices",
                metadata.count1.max(1)
            ))
            .color(Color32::from_rgb(200, 160, 180)),
        );
    }

    fn render_er_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!(
                "🗃️ ER diagram with {} entities/relations",
                metadata.count1.max(1)
            ))
            .color(Color32::from_rgb(160, 190, 200)),
        );
    }

    fn render_journey_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!(
                "🚶 User journey with {} sections",
                metadata.count1.max(1)
            ))
            .color(Color32::from_rgb(140, 180, 220)),
        );
    }

    fn render_gitgraph_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!(
                "🌿 Git graph: {} commits, {} branches",
                metadata.count1.max(1),
                metadata.count2
            ))
            .color(Color32::from_rgb(180, 200, 140)),
        );
    }

    fn render_mindmap_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!("🧠 Mind map with {} nodes", metadata.count1.max(1)))
                .color(Color32::from_rgb(200, 160, 200)),
        );
    }

    fn render_timeline_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!(
                "📅 Timeline with {} events",
                metadata.count1.max(1)
            ))
            .color(Color32::from_rgb(160, 200, 180)),
        );
    }

    fn render_quadrant_preview_cached(ui: &mut Ui) {
        ui.label(RichText::new("📊 Quadrant chart").color(Color32::from_rgb(180, 180, 200)));
    }

    fn render_requirement_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!(
                "📋 Requirements diagram with {} items",
                metadata.count1.max(1)
            ))
            .color(Color32::from_rgb(200, 180, 160)),
        );
    }

    fn render_c4_preview_cached(ui: &mut Ui) {
        ui.label(
            RichText::new("🏗️ C4 Architecture diagram").color(Color32::from_rgb(160, 180, 200)),
        );
    }

    fn render_generic_preview_cached(ui: &mut Ui, metadata: &MermaidMetadata) {
        ui.label(
            RichText::new(format!("📊 Diagram ({} lines)", metadata.count1))
                .color(Color32::from_gray(160))
                .italics(),
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
                let item_start = self.char_offset.saturating_sub(text.len());
                // Give list item text an explicit width so wrapped inline content
                // stays beside the marker instead of collapsing underneath it.
                let text_width = ui.available_width().max(1.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(text_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        self.render_mixed_content_with_annotations(
                            ui,
                            &text,
                            MixedContentStyle {
                                base_font_size,
                                start_offset: item_start,
                                code_text_color: None,
                                in_strikethrough: self.in_strikethrough,
                            },
                            &[],
                        );
                    },
                );
            }
        });
    }

    fn render_table(&mut self, ui: &mut Ui, base_font_size: f32) {
        if self.table_header.is_empty() && self.table_rows.is_empty() {
            return;
        }

        ui.add_space(8.0);

        let num_cols = self
            .table_header
            .len()
            .max(self.table_rows.first().map(|r| r.len()).unwrap_or(0));

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
                                let alignment = self
                                    .table_alignments
                                    .get(col_idx)
                                    .copied()
                                    .unwrap_or(Alignment::None);

                                let text = RichText::new(cell)
                                    .size(base_font_size)
                                    .strong()
                                    .color(Color32::from_gray(220));

                                match alignment {
                                    Alignment::Left | Alignment::None => {
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.label(text);
                                            },
                                        );
                                    }
                                    Alignment::Center => {
                                        ui.with_layout(
                                            egui::Layout::centered_and_justified(
                                                egui::Direction::LeftToRight,
                                            ),
                                            |ui| {
                                                ui.label(text);
                                            },
                                        );
                                    }
                                    Alignment::Right => {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(text);
                                            },
                                        );
                                    }
                                }
                            }
                            ui.end_row();
                        }

                        // Render data rows
                        for row in &self.table_rows {
                            for (col_idx, cell) in row.iter().enumerate() {
                                let alignment = self
                                    .table_alignments
                                    .get(col_idx)
                                    .copied()
                                    .unwrap_or(Alignment::None);

                                let text = RichText::new(cell)
                                    .size(base_font_size)
                                    .color(Color32::from_gray(180));

                                match alignment {
                                    Alignment::Left | Alignment::None => {
                                        ui.with_layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.label(text);
                                            },
                                        );
                                    }
                                    Alignment::Center => {
                                        ui.with_layout(
                                            egui::Layout::centered_and_justified(
                                                egui::Direction::LeftToRight,
                                            ),
                                            |ui| {
                                                ui.label(text);
                                            },
                                        );
                                    }
                                    Alignment::Right => {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(text);
                                            },
                                        );
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
        // Viewport culling: skip loading new images if not visible
        // Already-cached images are cheap to render, so we still show those
        let cursor_y = ui.cursor().top();
        let is_visible = self.is_position_visible(cursor_y);

        // Check if we have this image cached (O(1) lookup with IndexMap)
        if let Some(texture) = self.image_cache.get(url) {
            let size = texture.size_vec2();
            // Scale to fit width if needed
            let max_width = ui
                .available_width()
                .min(self.image_max_width.unwrap_or(600.0));
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
        let cache_key = url.to_string();

        // Check if already loading asynchronously
        if self.image_pending.contains(&cache_key) {
            // Show loading placeholder
            self.render_image_loading_placeholder(ui, title, is_remote);
            return;
        }

        // Viewport culling: only start loading images if visible
        if is_visible {
            if is_remote {
                // Spawn async load for remote image
                self.image_pending.insert(cache_key.clone());
                let url_owned = url.to_string();
                let key_owned = cache_key.clone();
                let sender = self.image_sender.clone();
                let ctx = ui.ctx().clone();

                std::thread::spawn(move || {
                    let result = fetch_remote_image_async(&url_owned);
                    let _ = sender.send((key_owned, result));
                    ctx.request_repaint();
                });

                // Show loading state for this frame
                self.render_image_loading_placeholder(ui, title, is_remote);
                return;
            } else {
                // Try to resolve local path
                let image_path = self.resolve_image_path(url);

                if let Some(path) = image_path {
                    // Check file size - for small files, load synchronously
                    // For larger files (>1MB), load asynchronously to avoid UI stutter
                    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

                    if file_size < 1_000_000 {
                        // Small file - load synchronously (fast enough)
                        if let Ok(texture) = load_image_texture(ui.ctx(), &path, url) {
                            let size = texture.size_vec2();
                            let max_width = ui
                                .available_width()
                                .min(self.image_max_width.unwrap_or(600.0));
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
                            self.cache_image(cache_key, texture);
                            return;
                        }
                    } else {
                        // Large file - load asynchronously
                        self.image_pending.insert(cache_key.clone());
                        let key_owned = cache_key.clone();
                        let sender = self.image_sender.clone();
                        let ctx = ui.ctx().clone();

                        std::thread::spawn(move || {
                            let result = match std::fs::read(&path) {
                                Ok(data) => Ok(data),
                                Err(e) => Err(format!("Failed to read file: {}", e)),
                            };
                            let _ = sender.send((key_owned, result));
                            ctx.request_repaint();
                        });

                        // Show loading state for this frame
                        self.render_image_loading_placeholder(ui, title, is_remote);
                        return;
                    }
                }
            }
        }

        // Fallback: show placeholder (for failed loads or not-yet-visible images)
        ui.horizontal(|ui| {
            ui.label(RichText::new("🖼").size(20.0));
            let display_text = if !title.is_empty() {
                title.to_string()
            } else if is_remote {
                format!("[Loading: {}]", truncate_url(url, 50))
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

    /// Render a loading placeholder for images being fetched asynchronously
    fn render_image_loading_placeholder(&self, ui: &mut Ui, title: &str, is_remote: bool) {
        ui.horizontal(|ui| {
            // Animated spinner
            let time = ui.ctx().input(|i| i.time);
            let spinner_char = match ((time * 4.0) as usize) % 4 {
                0 => "◐",
                1 => "◓",
                2 => "◑",
                _ => "◒",
            };
            ui.label(
                RichText::new(spinner_char)
                    .size(20.0)
                    .color(Color32::from_rgb(100, 140, 200)),
            );
            let display_text = if !title.is_empty() {
                format!("Loading: {}", title)
            } else if is_remote {
                "Loading remote image...".to_string()
            } else {
                "Loading image...".to_string()
            };
            ui.label(
                RichText::new(display_text)
                    .italics()
                    .color(Color32::from_gray(150)),
            );
        });
        // Request repaint to animate spinner
        ui.ctx().request_repaint();
    }

    /// Resolve an image URL to a local path.
    /// Canonicalizes the result and ensures it doesn't escape the base directory
    /// (prevents path traversal attacks via `../../` in markdown image references).
    fn resolve_image_path(&self, url: &str) -> Option<PathBuf> {
        // Skip remote URLs - they're handled separately
        if url.starts_with("http://") || url.starts_with("https://") {
            return None;
        }

        let path = PathBuf::from(url);

        // If absolute path, use directly
        if path.is_absolute() {
            if path.exists() {
                return path.canonicalize().ok();
            }
            return None;
        }

        // Try relative to base_path
        if let Some(base) = &self.base_path {
            let full_path = base.join(&path);
            if full_path.exists() {
                if let Ok(canonical) = full_path.canonicalize() {
                    // Verify the resolved path is under the base directory
                    if let Ok(canonical_base) = base.canonicalize() {
                        if canonical.starts_with(&canonical_base) {
                            return Some(canonical);
                        }
                        log::warn!(
                            "Image path traversal blocked: {:?} is outside {:?}",
                            url,
                            base
                        );
                        return None;
                    }
                    return Some(canonical);
                }
            }
        }

        // Try relative to current directory
        if path.exists() {
            return path.canonicalize().ok();
        }

        None
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
fn resolve_syntect_theme_name(theme_name: &str, is_dark: bool) -> &str {
    if theme_name == "auto" {
        if is_dark {
            "base16-ocean.dark"
        } else {
            "base16-ocean.light"
        }
    } else {
        theme_name
    }
}

fn normalize_language_token(language: Option<&str>) -> Option<Cow<'_, str>> {
    let raw = language?;
    let token = raw
        .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';')
        .next()
        .unwrap_or("")
        .trim()
        .trim_start_matches("language-");

    if token.is_empty() {
        return None;
    }

    let lower = token.to_ascii_lowercase();
    let alias = match lower.as_str() {
        "sh" | "shell" | "zsh" => "bash",
        "js" | "mjs" | "cjs" => "javascript",
        "typescript" | "ts" | "mts" | "cts" | "jsx" | "tsx" => "javascript",
        "yml" => "yaml",
        "c++" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "golang" => "go",
        "rs" => "rust",
        "py" => "python",
        "rb" => "ruby",
        "ps1" | "powershell" => "powershell",
        "md" | "mdown" | "mkd" => "markdown",
        "html" | "htm" => "html",
        "css" => "css",
        "jsonc" => "json",
        _ => return Some(Cow::Owned(lower)),
    };

    Some(Cow::Borrowed(alias))
}

#[cfg(feature = "syntax-highlighting")]
fn highlight_code(
    code: &str,
    language: Option<&str>,
    theme_name: &str,
    is_dark: bool,
    show_line_numbers: bool,
) -> Option<egui::text::LayoutJob> {
    use std::sync::OnceLock;
    use syntect::easy::HighlightLines;
    use syntect::highlighting::ThemeSet;
    use syntect::parsing::SyntaxSet;

    // Lazy-load syntax and theme sets (they're large)
    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

    let ss = SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines);
    let ts = THEME_SET.get_or_init(ThemeSet::load_defaults);

    // Find syntax for the language
    let syntax = language
        .and_then(|lang| ss.find_syntax_by_token(lang))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    // Resolve theme name (handle "auto" for theme-aware selection)
    let resolved_theme = resolve_syntect_theme_name(theme_name, is_dark);

    // Use theme from config, falling back to base16-ocean.dark or any available theme
    let theme = ts
        .themes
        .get(resolved_theme)
        .or_else(|| ts.themes.get("base16-ocean.dark"))
        .or_else(|| ts.themes.values().next())?;
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut job = egui::text::LayoutJob::default();
    let mono_font = egui::FontId::monospace(13.0);

    let line_number_color = theme_colors::code_line_number(is_dark);
    let lines: Vec<&str> = code.lines().collect();
    let line_count = lines.len();
    // Calculate width for line number gutter (digits needed + padding)
    let line_num_width = if show_line_numbers {
        let digits = if line_count == 0 {
            1
        } else {
            (line_count as f32).log10().floor() as usize + 1
        };
        digits + 2 // extra space for padding
    } else {
        0
    };

    for (i, line) in lines.iter().enumerate() {
        // Add line number if enabled
        if show_line_numbers {
            let line_num = format!("{:>width$}  ", i + 1, width = line_num_width - 2);
            job.append(
                &line_num,
                0.0,
                egui::TextFormat {
                    font_id: mono_font.clone(),
                    color: line_number_color,
                    ..Default::default()
                },
            );
        }

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
fn highlight_code(
    _code: &str,
    _language: Option<&str>,
    _theme_name: &str,
    _is_dark: bool,
    _show_line_numbers: bool,
) -> Option<egui::text::LayoutJob> {
    None
}

/// Load an image from a local path into an egui texture
fn load_image_texture(
    ctx: &egui::Context,
    path: &std::path::Path,
    name: &str,
) -> Result<TextureHandle, String> {
    // Read the file
    let image_data = std::fs::read(path).map_err(|e| format!("Failed to read image: {}", e))?;

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

    Ok(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR))
}

/// Truncate a URL for display
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_string()
    } else {
        format!("{}...", &url[..max_len - 3])
    }
}

/// Fetch a remote image asynchronously with timeout (called from background thread)
/// Returns Ok(bytes) on success, Err(message) on failure
fn fetch_remote_image_async(url: &str) -> Result<Vec<u8>, String> {
    // Use ureq with timeout to prevent indefinite blocking
    let agent = ureq::AgentBuilder::new()
        .timeout_read(IMAGE_FETCH_TIMEOUT)
        .timeout_write(IMAGE_FETCH_TIMEOUT)
        .build();

    let response = agent
        .get(url)
        .call()
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    // Check content type
    let content_type = response.content_type();
    if !content_type.starts_with("image/") {
        return Err(format!("Not an image (content-type: {})", content_type));
    }

    // Limit size to 10MB
    let max_size = 10 * 1024 * 1024;
    let mut bytes = Vec::new();

    // Read with size limit
    response
        .into_reader()
        .take(max_size as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Failed to read: {}", e))?;

    Ok(bytes)
}

/// Case-insensitive prefix check (avoids allocation from to_lowercase)
#[inline]
fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix)
}

/// Detect the type of Mermaid diagram from its source code
fn detect_mermaid_type(code: &str) -> &'static str {
    let first_line = code
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim())
        .unwrap_or("");

    // Use case-insensitive prefix matching (no allocation)
    if starts_with_ignore_case(first_line, "graph")
        || starts_with_ignore_case(first_line, "flowchart")
    {
        "Flowchart"
    } else if starts_with_ignore_case(first_line, "sequencediagram")
        || starts_with_ignore_case(first_line, "sequence")
    {
        "Sequence Diagram"
    } else if starts_with_ignore_case(first_line, "classdiagram")
        || starts_with_ignore_case(first_line, "class")
    {
        "Class Diagram"
    } else if starts_with_ignore_case(first_line, "statediagram")
        || starts_with_ignore_case(first_line, "state")
    {
        "State Diagram"
    } else if starts_with_ignore_case(first_line, "gantt") {
        "Gantt Chart"
    } else if starts_with_ignore_case(first_line, "pie") {
        "Pie Chart"
    } else if starts_with_ignore_case(first_line, "erdiagram")
        || starts_with_ignore_case(first_line, "er")
    {
        "ER Diagram"
    } else if starts_with_ignore_case(first_line, "journey") {
        "User Journey"
    } else if starts_with_ignore_case(first_line, "gitgraph") {
        "Git Graph"
    } else if starts_with_ignore_case(first_line, "mindmap") {
        "Mind Map"
    } else if starts_with_ignore_case(first_line, "timeline") {
        "Timeline"
    } else if starts_with_ignore_case(first_line, "quadrantchart") {
        "Quadrant Chart"
    } else if starts_with_ignore_case(first_line, "requirementdiagram") {
        "Requirement Diagram"
    } else if starts_with_ignore_case(first_line, "c4context")
        || starts_with_ignore_case(first_line, "c4container")
    {
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
    let node_id: String = line
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    if !node_id.is_empty() {
        node_id
    } else {
        line.chars().take(20).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    /// Helper: parse markdown to owned events
    fn parse_events(markdown: &str) -> Vec<Event<'static>> {
        let parser = Parser::new_ext(markdown, Options::all());
        parser.map(|e| e.into_static()).collect()
    }

    #[test]
    fn test_content_block_height_uses_actual_when_available() {
        let block = ContentBlock {
            event_start: 0,
            event_end: 1,
            estimated_height: 40.0,
            actual_height: None,
            is_heading: false,
            text_byte_len: 10,
            heading_index: None,
        };
        assert_eq!(
            block.height(),
            40.0,
            "should use estimated when actual is None"
        );

        let block_with_actual = ContentBlock {
            actual_height: Some(55.0),
            ..block
        };
        assert_eq!(
            block_with_actual.height(),
            55.0,
            "should use actual when available"
        );
    }

    #[test]
    fn test_byte_offset_for_char_index_handles_unicode() {
        let text = "aé日";
        assert_eq!(byte_offset_for_char_index(text, 0), 0);
        assert_eq!(byte_offset_for_char_index(text, 1), 1);
        assert_eq!(byte_offset_for_char_index(text, 2), 3);
        assert_eq!(byte_offset_for_char_index(text, 3), text.len());
    }

    #[test]
    fn test_compute_block_map_paragraph() {
        let md = "Hello world, this is a paragraph.\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        assert_eq!(blocks.len(), 1);
        assert!(!blocks[0].is_heading);
        assert!(blocks[0].heading_index.is_none());
        assert!(blocks[0].estimated_height > 0.0);
        assert!(blocks[0].text_byte_len > 0);
        assert!(blocks[0].actual_height.is_none());
    }

    #[test]
    fn test_compute_block_map_heading_tracking() {
        let md = "# First\n\nSome text\n\n## Second\n\nMore text\n\n### Third\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        // Should have: heading, paragraph, heading, paragraph, heading
        let headings: Vec<_> = blocks.iter().filter(|b| b.is_heading).collect();
        assert_eq!(headings.len(), 3);

        // Heading indices should be sequential
        assert_eq!(headings[0].heading_index, Some(0));
        assert_eq!(headings[1].heading_index, Some(1));
        assert_eq!(headings[2].heading_index, Some(2));
    }

    #[test]
    fn test_compute_block_map_code_block() {
        let md = "```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        assert_eq!(blocks.len(), 1);
        assert!(!blocks[0].is_heading);
        // 3 lines of code + fixed code-block padding/margins
        let expected_height = 3.0 * ESTIMATED_LINE_HEIGHT + 32.0;
        assert!(
            (blocks[0].estimated_height - expected_height).abs() < 0.001,
            "expected {}, got {}",
            expected_height,
            blocks[0].estimated_height
        );
    }

    #[test]
    fn test_compute_block_map_list() {
        let md = "- item one\n- item two\n- item three\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        assert_eq!(blocks.len(), 1);
        // List height should account for items
        assert!(blocks[0].estimated_height >= ESTIMATED_LINE_HEIGHT * 3.0);
    }

    #[test]
    fn test_compute_block_map_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        assert_eq!(blocks.len(), 1);
        // Table with header + 2 rows
        assert!(blocks[0].estimated_height > 0.0);
    }

    #[test]
    fn test_compute_block_map_blockquote() {
        let md = "> This is a quote\n> with multiple lines\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].estimated_height > 0.0);
    }

    #[test]
    fn test_compute_block_map_horizontal_rule() {
        let md = "---\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].estimated_height, 20.0);
        assert_eq!(blocks[0].text_byte_len, 0);
    }

    #[test]
    fn test_compute_block_map_extended_markdown_blocks() {
        let md = r#"---
title: Demo
---

Term
: Definition

$$
x = y
$$

<section>
Raw HTML
</section>
"#;
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        assert!(
            blocks.iter().any(|block| matches!(
                events.get(block.event_start),
                Some(Event::Start(Tag::MetadataBlock(_)))
            )),
            "metadata block should be represented for viewport culling"
        );
        assert!(
            blocks.iter().any(|block| matches!(
                events.get(block.event_start),
                Some(Event::Start(Tag::DefinitionList))
            )),
            "definition list should be represented for viewport culling"
        );
        assert!(
            blocks
                .iter()
                .any(|block| events[block.event_start..block.event_end]
                    .iter()
                    .any(|event| matches!(event, Event::DisplayMath(_)))),
            "display math should be represented for viewport culling"
        );
        assert!(
            blocks.iter().any(|block| matches!(
                events.get(block.event_start),
                Some(Event::Start(Tag::HtmlBlock))
            )),
            "HTML block should be represented for viewport culling"
        );
    }

    #[test]
    fn test_compute_block_map_mixed_content() {
        let md = "# Title\n\nParagraph one.\n\n- item\n\n## Subtitle\n\n> quote\n\n---\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        // heading, paragraph, list, heading, blockquote, rule
        assert_eq!(blocks.len(), 6);

        // First and fourth blocks should be headings
        assert!(blocks[0].is_heading);
        assert_eq!(blocks[0].heading_index, Some(0));
        assert!(blocks[3].is_heading);
        assert_eq!(blocks[3].heading_index, Some(1));
    }

    #[test]
    fn test_compute_block_map_width_affects_height_estimation() {
        let md = "This is a long paragraph that should wrap differently at different widths. It contains enough text to demonstrate that width changes affect the estimated height calculation.\n";
        let events = parse_events(md);

        let blocks_wide = compute_block_map(&events, 800.0);
        let blocks_narrow = compute_block_map(&events, 200.0);

        // Narrower width should produce taller estimated height
        assert!(blocks_narrow[0].estimated_height > blocks_wide[0].estimated_height);
    }

    #[test]
    fn test_render_layout_cache_key_tracks_height_affecting_settings() {
        let config = Config::default();
        let baseline = render_layout_cache_key(&config);

        let mut font_changed = config.clone();
        font_changed.theme.fonts.size += 2.0;
        assert_ne!(baseline, render_layout_cache_key(&font_changed));

        let mut spacing_changed = config.clone();
        spacing_changed.theme.spacing.paragraph += 4.0;
        assert_ne!(baseline, render_layout_cache_key(&spacing_changed));

        let mut image_width_changed = config.clone();
        image_width_changed.layout.image_width = Some(320.0);
        assert_ne!(baseline, render_layout_cache_key(&image_width_changed));

        let mut line_numbers_changed = config;
        line_numbers_changed.markdown.show_line_numbers =
            !line_numbers_changed.markdown.show_line_numbers;
        assert_ne!(baseline, render_layout_cache_key(&line_numbers_changed));
    }

    #[test]
    fn test_compute_block_map_empty_document() {
        let events: Vec<Event<'static>> = Vec::new();
        let blocks = compute_block_map(&events, 800.0);
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_compute_block_map_preserves_event_ranges() {
        let md = "# Hello\n\nWorld\n";
        let events = parse_events(md);
        let blocks = compute_block_map(&events, 800.0);

        // All events should be covered by blocks (no gaps)
        let total_events_in_blocks: usize =
            blocks.iter().map(|b| b.event_end - b.event_start).sum();

        // Each block's range should be valid
        for block in &blocks {
            assert!(block.event_start < block.event_end);
            assert!(block.event_end <= events.len());
        }

        // Blocks should not overlap
        for pair in blocks.windows(2) {
            assert!(pair[0].event_end <= pair[1].event_start);
        }

        assert!(total_events_in_blocks > 0);
    }

    #[test]
    fn test_strip_inline_markers_code_and_footnote() {
        let input = "Hello \x00CODE:world\x00 and \x01FN:id:3\x01!";
        assert_eq!(strip_inline_markers(input), "Hello world and [3]!");
    }

    #[test]
    fn test_strip_inline_markers_link() {
        let input = "Go to \x02https://example.com\x02Example\x03 now";
        assert_eq!(strip_inline_markers(input), "Go to Example now");
    }

    #[test]
    fn test_normalize_language_token_aliases() {
        assert_eq!(
            normalize_language_token(Some("rs")).as_deref(),
            Some("rust")
        );
        assert_eq!(
            normalize_language_token(Some("language-tsx")).as_deref(),
            Some("javascript")
        );
        assert_eq!(
            normalize_language_token(Some("shell session")).as_deref(),
            Some("bash")
        );
        assert_eq!(
            normalize_language_token(Some("yml")).as_deref(),
            Some("yaml")
        );
        assert_eq!(
            normalize_language_token(Some("C++")).as_deref(),
            Some("cpp")
        );
        assert_eq!(
            normalize_language_token(Some("unknown-lang")).as_deref(),
            Some("unknown-lang")
        );
        assert!(normalize_language_token(Some("")).is_none());
        assert!(normalize_language_token(None).is_none());
    }

    #[test]
    fn test_normalize_language_token_common_language_set() {
        let cases = [
            ("rust", "rust"),
            ("python", "python"),
            ("py", "python"),
            ("javascript", "javascript"),
            ("js", "javascript"),
            ("typescript", "javascript"),
            ("ts", "javascript"),
            ("tsx", "javascript"),
            ("jsx", "javascript"),
            ("go", "go"),
            ("golang", "go"),
            ("c", "c"),
            ("c++", "cpp"),
            ("java", "java"),
            ("toml", "toml"),
            ("yaml", "yaml"),
            ("yml", "yaml"),
            ("json", "json"),
            ("bash", "bash"),
            ("sh", "bash"),
            ("sql", "sql"),
            ("html", "html"),
            ("css", "css"),
            ("markdown", "markdown"),
            ("md", "markdown"),
            ("diff", "diff"),
        ];

        for (input, expected) in cases {
            assert_eq!(
                normalize_language_token(Some(input)).as_deref(),
                Some(expected),
                "{input} should normalize to {expected}"
            );
        }
    }

    #[cfg(feature = "syntax-highlighting")]
    #[test]
    fn test_highlight_common_language_set_produces_layout_jobs() {
        let languages = [
            "rust",
            "python",
            "javascript",
            "typescript",
            "tsx",
            "jsx",
            "go",
            "c",
            "cpp",
            "java",
            "toml",
            "yaml",
            "json",
            "bash",
            "sql",
            "html",
            "css",
            "markdown",
            "diff",
        ];

        for language in languages {
            let normalized = normalize_language_token(Some(language));
            let job = highlight_code(
                "fn main() { println!(\"hello\"); }\n",
                normalized.as_deref(),
                "auto",
                true,
                false,
            );
            assert!(
                job.is_some(),
                "{language} should produce highlighted or plain fallback output"
            );
        }
    }

    #[cfg(feature = "syntax-highlighting")]
    #[test]
    fn test_highlight_unknown_language_falls_back_to_plain_text() {
        let job = highlight_code(
            "plain text\nsecond line",
            normalize_language_token(Some("unknown-lang")).as_deref(),
            "auto",
            true,
            true,
        )
        .expect("fallback highlighting should produce a layout job");

        assert!(job.text.contains("plain text"));
        assert!(job.text.contains("second line"));
    }

    #[test]
    fn test_mermaid_fixture_type_detection() {
        let cases = [
            (
                include_str!("../../fixtures/mermaid/flowchart.mmd"),
                "Flowchart",
            ),
            (
                include_str!("../../fixtures/mermaid/sequence.mmd"),
                "Sequence Diagram",
            ),
            (
                include_str!("../../fixtures/mermaid/class.mmd"),
                "Class Diagram",
            ),
            (
                include_str!("../../fixtures/mermaid/state.mmd"),
                "State Diagram",
            ),
            (
                include_str!("../../fixtures/mermaid/gantt.mmd"),
                "Gantt Chart",
            ),
        ];

        for (fixture, expected) in cases {
            assert_eq!(detect_mermaid_type(fixture), expected);
        }
    }
}
