//! TOC sidebar panel UI

use egui::{Color32, RichText, Ui};

use super::{TocEntry, TocTree};

/// TOC panel widget
pub struct TocPanel {
    /// Collapsed state for each entry (by index)
    collapsed: Vec<bool>,
}

impl TocPanel {
    pub fn new() -> Self {
        Self {
            collapsed: Vec::new(),
        }
    }

    /// Render the TOC panel and return the index of clicked heading (if any)
    pub fn render(
        &mut self,
        ui: &mut Ui,
        toc: &TocTree,
        current_heading: Option<usize>,
    ) -> Option<usize> {
        // Ensure collapsed vector is sized correctly
        if self.collapsed.len() != toc.len() {
            self.collapsed.resize(toc.len(), false);
        }

        ui.heading("Contents");
        ui.separator();

        if toc.is_empty() {
            ui.label(RichText::new("No headings").italics().color(Color32::GRAY));
            return None;
        }

        let mut clicked = None;

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in &toc.entries {
                    if let Some(idx) = self.render_entry(ui, entry, current_heading, 0) {
                        clicked = Some(idx);
                    }
                }
            });

        clicked
    }

    /// Render a single TOC entry and its children recursively
    fn render_entry(
        &mut self,
        ui: &mut Ui,
        entry: &TocEntry,
        current_heading: Option<usize>,
        depth: usize,
    ) -> Option<usize> {
        let mut clicked = None;
        let indent = depth as f32 * 16.0;
        let is_current = current_heading == Some(entry.index);
        let has_children = !entry.children.is_empty();

        ui.horizontal(|ui| {
            ui.add_space(indent);

            // Collapse toggle for entries with children
            if has_children {
                let collapsed = self.collapsed.get(entry.index).copied().unwrap_or(false);
                let toggle_text = if collapsed { "▶" } else { "▼" };

                if ui
                    .small_button(RichText::new(toggle_text).size(10.0))
                    .clicked()
                {
                    if let Some(c) = self.collapsed.get_mut(entry.index) {
                        *c = !*c;
                    }
                }
            } else {
                // Spacer for alignment
                ui.add_space(18.0);
            }

            // Entry text
            let text = &entry.text;
            let mut rich_text = RichText::new(text);

            // Style based on level and current state
            rich_text = match entry.level {
                1 => rich_text.strong().size(14.0),
                2 => rich_text.strong().size(13.0),
                3 => rich_text.size(12.0),
                _ => rich_text.size(11.0),
            };

            if is_current {
                rich_text = rich_text.color(Color32::from_rgb(78, 201, 176));
            }

            let response = ui.selectable_label(is_current, rich_text);

            if response.clicked() {
                clicked = Some(entry.index);
            }

            // Tooltip with full text for truncated entries
            response.on_hover_text(text);
        });

        // Render children if not collapsed
        let collapsed = self.collapsed.get(entry.index).copied().unwrap_or(false);
        if has_children && !collapsed {
            for child in &entry.children {
                if let Some(idx) = self.render_entry(ui, child, current_heading, depth + 1) {
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

    /// Expand to show a specific heading
    pub fn expand_to(&mut self, _toc: &TocTree, index: usize) {
        // For now, just expand all
        // A proper implementation would trace the path to the entry
        if index < self.collapsed.len() {
            self.collapsed[index] = false;
        }
    }
}

impl Default for TocPanel {
    fn default() -> Self {
        Self::new()
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
}
