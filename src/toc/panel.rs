//! TOC sidebar panel UI
//!
//! Refined, minimal table of contents with smooth interactions.

#![allow(dead_code)]

use egui::{Color32, Rounding, Ui, Vec2};

use super::{TocEntry, TocTree};
use crate::theme::style::palette;

/// Pre-computed theme colors (computed once per render, not per entry)
struct TocColors {
    accent: Color32,
    bg_hover: Color32,
    bg_elevated: Color32,
    text_primary: Color32,
    text_secondary: Color32,
    text_muted: Color32,
    text_disabled: Color32,
}

impl TocColors {
    fn new(is_dark: bool) -> Self {
        if is_dark {
            Self {
                accent: palette::ACCENT,
                bg_hover: palette::BG_HOVER,
                bg_elevated: palette::BG_ELEVATED,
                text_primary: palette::TEXT_PRIMARY,
                text_secondary: palette::TEXT_SECONDARY,
                text_muted: palette::TEXT_MUTED,
                text_disabled: palette::TEXT_DISABLED,
            }
        } else {
            Self {
                accent: palette::light::ACCENT,
                bg_hover: palette::light::BG_HOVER,
                bg_elevated: palette::light::BG_ELEVATED,
                text_primary: palette::light::TEXT_PRIMARY,
                text_secondary: palette::light::TEXT_SECONDARY,
                text_muted: palette::light::TEXT_MUTED,
                text_disabled: palette::light::TEXT_DISABLED,
            }
        }
    }
}

/// TOC panel widget
pub struct TocPanel {
    /// Collapsed state for each entry (by index)
    collapsed: Vec<bool>,

    /// Currently focused entry index for keyboard navigation
    focused_index: Option<usize>,

    /// Whether the TOC panel has keyboard focus
    has_focus: bool,
}

impl TocPanel {
    pub fn new() -> Self {
        Self {
            collapsed: Vec::new(),
            focused_index: None,
            has_focus: false,
        }
    }

    /// Render the TOC panel and return the index of clicked heading (if any)
    pub fn render(
        &mut self,
        ui: &mut Ui,
        toc: &TocTree,
        current_heading: Option<usize>,
        is_dark: bool,
    ) -> Option<usize> {
        // Ensure collapsed vector is sized correctly
        if self.collapsed.len() != toc.len() {
            self.collapsed.resize(toc.len(), false);
        }

        // Pre-compute colors once per render (not per entry)
        let colors = TocColors::new(is_dark);

        if toc.is_empty() {
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new("No headings")
                        .color(colors.text_disabled)
                        .italics()
                );
            });
            return None;
        }

        let mut clicked = None;

        // Get flat list of visible entry indices for keyboard navigation
        let visible_indices = self.get_visible_indices(toc);

        // Handle keyboard navigation
        if self.has_focus {
            let nav_result = self.handle_keyboard_nav(ui, &visible_indices, toc.len());
            if nav_result.is_some() {
                clicked = nav_result;
            }
        }

        // Check for focus on the panel area
        let response = ui.interact(
            ui.available_rect_before_wrap(),
            ui.id().with("toc_focus"),
            egui::Sense::click(),
        );
        if response.clicked() {
            self.has_focus = true;
            if self.focused_index.is_none() && !visible_indices.is_empty() {
                self.focused_index = Some(visible_indices[0]);
            }
        }

        // Lose focus when clicking elsewhere
        if ui.input(|i| i.pointer.any_click()) && !response.hovered() {
            self.has_focus = false;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(4.0);
                for entry in &toc.entries {
                    if let Some(idx) = self.render_entry(ui, entry, current_heading, self.focused_index, 0, &colors) {
                        clicked = Some(idx);
                        self.focused_index = Some(idx);
                    }
                }
                ui.add_space(16.0);
            });

        clicked
    }

    /// Get a flat list of visible (not collapsed) entry indices
    fn get_visible_indices(&self, toc: &TocTree) -> Vec<usize> {
        let mut indices = Vec::new();
        self.collect_visible_indices(&toc.entries, &mut indices);
        indices
    }

    fn collect_visible_indices(&self, entries: &[TocEntry], indices: &mut Vec<usize>) {
        for entry in entries {
            indices.push(entry.index);
            let collapsed = self.collapsed.get(entry.index).copied().unwrap_or(false);
            if !collapsed && !entry.children.is_empty() {
                self.collect_visible_indices(&entry.children, indices);
            }
        }
    }

    /// Handle keyboard navigation, returns Some(index) if Enter was pressed on an entry
    fn handle_keyboard_nav(&mut self, ui: &mut Ui, visible_indices: &[usize], _toc_len: usize) -> Option<usize> {
        let mut result = None;

        ui.input(|i| {
            if i.key_pressed(egui::Key::ArrowDown) {
                if let Some(current) = self.focused_index {
                    if let Some(pos) = visible_indices.iter().position(|&x| x == current) {
                        if pos + 1 < visible_indices.len() {
                            self.focused_index = Some(visible_indices[pos + 1]);
                        }
                    }
                } else if !visible_indices.is_empty() {
                    self.focused_index = Some(visible_indices[0]);
                }
            }

            if i.key_pressed(egui::Key::ArrowUp) {
                if let Some(current) = self.focused_index {
                    if let Some(pos) = visible_indices.iter().position(|&x| x == current) {
                        if pos > 0 {
                            self.focused_index = Some(visible_indices[pos - 1]);
                        }
                    }
                } else if !visible_indices.is_empty() {
                    self.focused_index = Some(visible_indices[visible_indices.len() - 1]);
                }
            }

            if i.key_pressed(egui::Key::ArrowRight) {
                // Expand current entry
                if let Some(current) = self.focused_index {
                    if let Some(c) = self.collapsed.get_mut(current) {
                        *c = false;
                    }
                }
            }

            if i.key_pressed(egui::Key::ArrowLeft) {
                // Collapse current entry
                if let Some(current) = self.focused_index {
                    if let Some(c) = self.collapsed.get_mut(current) {
                        *c = true;
                    }
                }
            }

            if i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Space) {
                result = self.focused_index;
            }
        });

        result
    }

    /// Render a single TOC entry and its children recursively
    /// Uses pre-computed colors to avoid per-entry color lookups
    fn render_entry(
        &mut self,
        ui: &mut Ui,
        entry: &TocEntry,
        current_heading: Option<usize>,
        focused_index: Option<usize>,
        depth: usize,
        colors: &TocColors,
    ) -> Option<usize> {
        let mut clicked = None;
        let base_indent = 16.0;
        let indent = base_indent + (depth as f32 * 12.0);
        let is_current = current_heading == Some(entry.index);
        let is_focused = focused_index == Some(entry.index);
        let has_children = !entry.children.is_empty();

        // Calculate item height
        let item_height = 28.0;

        // Allocate space for the item
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), item_height),
            egui::Sense::click(),
        );

        let is_hovered = response.hovered();

        // Draw background for current/hovered/focused state
        if is_current {
            // Active indicator line on the left
            let indicator_rect = egui::Rect::from_min_size(
                rect.min,
                Vec2::new(3.0, item_height),
            );
            ui.painter().rect_filled(
                indicator_rect,
                Rounding::ZERO,
                colors.accent,
            );

            // Subtle background
            ui.painter().rect_filled(
                rect,
                Rounding::ZERO,
                colors.bg_hover,
            );
        } else if is_focused {
            // Focus indicator - dotted border effect
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                Rounding::same(2.0),
                egui::Stroke::new(1.0, colors.accent),
            );
            ui.painter().rect_filled(
                rect,
                Rounding::ZERO,
                colors.bg_elevated,
            );
        } else if is_hovered {
            ui.painter().rect_filled(
                rect,
                Rounding::ZERO,
                colors.bg_elevated,
            );
        }

        // Collapse toggle for entries with children
        if has_children {
            let collapsed = self.collapsed.get(entry.index).copied().unwrap_or(false);
            let toggle_text = if collapsed { "\u{25B6}" } else { "\u{25BC}" };

            let toggle_pos = rect.min + Vec2::new(indent - 14.0, item_height / 2.0);
            let toggle_rect = egui::Rect::from_center_size(toggle_pos, Vec2::splat(16.0));

            let toggle_response = ui.interact(toggle_rect, ui.id().with(("toggle", entry.index)), egui::Sense::click());

            ui.painter().text(
                toggle_pos,
                egui::Align2::CENTER_CENTER,
                toggle_text,
                egui::FontId::proportional(11.0),
                if toggle_response.hovered() { colors.text_primary } else { colors.text_muted },
            );

            if toggle_response.clicked() {
                if let Some(c) = self.collapsed.get_mut(entry.index) {
                    *c = !*c;
                }
            }
        }

        // Entry text
        let text_color = if is_current {
            colors.accent
        } else if is_hovered {
            colors.text_primary
        } else {
            colors.text_secondary
        };

        let font_size = match entry.level {
            1 => 13.0,
            2 => 12.5,
            _ => 12.0,
        };

        // Truncate text if needed
        let max_text_width = rect.width() - indent - 16.0;
        let text = truncate_text(&entry.text, max_text_width, font_size);

        ui.painter().text(
            rect.min + Vec2::new(indent, item_height / 2.0),
            egui::Align2::LEFT_CENTER,
            &text,
            egui::FontId::proportional(font_size),
            text_color,
        );

        // Handle click
        if response.clicked() {
            clicked = Some(entry.index);
        }

        // Show full text on hover if truncated
        if text != entry.text {
            response.on_hover_text(&entry.text);
        }

        // Render children if not collapsed
        let collapsed = self.collapsed.get(entry.index).copied().unwrap_or(false);
        if has_children && !collapsed {
            for child in &entry.children {
                if let Some(idx) = self.render_entry(ui, child, current_heading, focused_index, depth + 1, colors) {
                    clicked = Some(idx);
                }
            }
        }

        clicked
    }

    /// Expand all entries
    pub fn expand_all(&mut self) {
        self.collapsed.fill(false);
    }

    /// Collapse all entries
    pub fn collapse_all(&mut self) {
        self.collapsed.fill(true);
    }

    /// Expand to show a specific heading by expanding all its ancestors
    pub fn expand_to(&mut self, toc: &TocTree, index: usize) {
        if index >= self.collapsed.len() {
            return;
        }

        // Find the path to this entry (all ancestor indices)
        let ancestors = self.find_ancestors(toc, index);

        // Expand all ancestors and the target itself
        for ancestor_idx in ancestors {
            if ancestor_idx < self.collapsed.len() {
                self.collapsed[ancestor_idx] = false;
            }
        }

        // Also expand the target entry
        self.collapsed[index] = false;
    }

    /// Find all ancestor indices for a given entry index
    fn find_ancestors(&self, toc: &TocTree, target_index: usize) -> Vec<usize> {
        let mut ancestors = Vec::new();
        self.find_ancestors_recursive(&toc.entries, target_index, &mut ancestors);
        ancestors
    }

    /// Recursively search for ancestors
    fn find_ancestors_recursive(
        &self,
        entries: &[TocEntry],
        target_index: usize,
        path: &mut Vec<usize>,
    ) -> bool {
        for entry in entries {
            if entry.index == target_index {
                // Found the target
                return true;
            }

            if !entry.children.is_empty() {
                // Add this entry to the path and search children
                path.push(entry.index);
                if self.find_ancestors_recursive(&entry.children, target_index, path) {
                    return true;
                }
                // Not found in this subtree, remove from path
                path.pop();
            }
        }

        false
    }
}

/// Truncate text to fit within a given width
fn truncate_text(text: &str, max_width: f32, font_size: f32) -> String {
    // Rough estimate: average character width is about 0.5 * font_size
    let char_width = font_size * 0.5;
    let max_chars = (max_width / char_width) as usize;

    if text.len() <= max_chars {
        text.to_string()
    } else if max_chars > 3 {
        format!("{}...", &text[..max_chars - 3])
    } else {
        text.to_string()
    }
}

impl Default for TocPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl TocPanel {
    /// Set focus to the TOC panel
    pub fn focus(&mut self) {
        self.has_focus = true;
    }

    /// Remove focus from the TOC panel
    pub fn unfocus(&mut self) {
        self.has_focus = false;
    }

    /// Check if the TOC panel has focus
    pub fn is_focused(&self) -> bool {
        self.has_focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toc_panel_new() {
        let panel = TocPanel::new();
        assert!(panel.collapsed.is_empty());
    }

    #[test]
    fn test_truncate_text() {
        assert_eq!(truncate_text("short", 100.0, 14.0), "short");
        // With font_size 14.0, char_width ~7.0, max_chars for 50.0 width is ~7 chars
        let result = truncate_text("this is a very long text", 50.0, 14.0);
        assert!(result.ends_with("..."));
        assert!(result.len() < "this is a very long text".len());
    }
}
