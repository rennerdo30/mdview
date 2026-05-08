//! Annotation UI components

use egui::{Color32, Pos2, Rect, RichText, Ui, Vec2};

use super::model::{AnnotationKind, AnnotationStore};

/// Predefined highlight colors
pub const HIGHLIGHT_COLORS: &[(&str, &str)] = &[
    ("#ffeb3b", "Yellow"),
    ("#4caf50", "Green"),
    ("#2196f3", "Blue"),
    ("#f44336", "Red"),
    ("#9c27b0", "Purple"),
    ("#ff9800", "Orange"),
];

/// Parse a hex color string to Color32 for annotation use.
/// Falls back to yellow (default highlight color) on invalid input.
pub fn parse_hex_color(hex: &str) -> Color32 {
    let trimmed = hex.trim_start_matches('#');
    if trimmed.len() == 6 || trimmed.len() == 3 {
        crate::theme::style::parse_hex_color(hex)
    } else {
        Color32::YELLOW
    }
}

/// Annotation creation popup
pub struct AnnotationPopup {
    /// Position of the popup
    pub position: Pos2,

    /// Text selection range
    pub selection: (usize, usize),

    /// Whether popup is visible
    pub visible: bool,

    /// Note text being edited
    pub note_text: String,

    /// Selected color for highlight
    pub selected_color: usize,
}

impl AnnotationPopup {
    pub fn new() -> Self {
        Self {
            position: Pos2::ZERO,
            selection: (0, 0),
            visible: false,
            note_text: String::new(),
            selected_color: 0,
        }
    }

    /// Show the popup at a position
    pub fn show(&mut self, position: Pos2, selection: (usize, usize)) {
        self.position = position;
        self.selection = selection;
        self.visible = true;
        self.note_text.clear();
    }

    /// Hide the popup
    pub fn hide(&mut self) {
        self.visible = false;
        self.note_text.clear();
    }

    /// Render the popup and return action to take
    pub fn render(&mut self, ui: &mut Ui) -> Option<AnnotationAction> {
        if !self.visible {
            return None;
        }

        let mut action = None;

        egui::Area::new(egui::Id::new("annotation_popup"))
            .fixed_pos(self.position)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(200.0);

                    ui.label(RichText::new("Add Annotation").strong());
                    ui.separator();

                    // Highlight button with color picker
                    ui.horizontal(|ui| {
                        if ui.button("Highlight").clicked() {
                            let color = HIGHLIGHT_COLORS[self.selected_color].0;
                            action = Some(AnnotationAction::CreateHighlight(
                                self.selection.0,
                                self.selection.1,
                                color.to_string(),
                            ));
                            self.hide();
                        }

                        // Color buttons
                        for (idx, (color, _name)) in HIGHLIGHT_COLORS.iter().enumerate() {
                            let color32 = parse_hex_color(color);
                            let is_selected = idx == self.selected_color;

                            let size = if is_selected { 20.0 } else { 16.0 };
                            let (rect, response) =
                                ui.allocate_exact_size(Vec2::splat(size), egui::Sense::click());

                            if response.clicked() {
                                self.selected_color = idx;
                            }

                            ui.painter().rect_filled(rect, 2.0, color32);
                            if is_selected {
                                ui.painter().rect_stroke(
                                    rect,
                                    2.0,
                                    egui::Stroke::new(2.0, Color32::WHITE),
                                );
                            }
                        }
                    });

                    ui.separator();

                    // Note input
                    ui.label("Add note:");
                    ui.text_edit_multiline(&mut self.note_text);

                    ui.horizontal(|ui| {
                        if ui.button("Add Note").clicked() && !self.note_text.is_empty() {
                            action = Some(AnnotationAction::CreateNote(
                                self.selection.0,
                                self.selection.1,
                                self.note_text.clone(),
                            ));
                            self.hide();
                        }

                        if ui.button("Cancel").clicked() {
                            self.hide();
                        }
                    });
                });
            });

        action
    }
}

impl Default for AnnotationPopup {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions from annotation UI
#[derive(Debug, Clone)]
pub enum AnnotationAction {
    CreateHighlight(usize, usize, String),
    CreateNote(usize, usize, String),
    CreateBookmark(usize),
    Delete(String),
    UpdateNote(String, String),
    UpdateColor(String, String),
}

/// Render annotation margin icons
pub fn render_margin_icons(
    ui: &mut Ui,
    annotations: &AnnotationStore,
    line_positions: &[(usize, f32)], // (char_offset, y_position)
) -> Option<String> {
    let mut clicked_id = None;

    for annotation in annotations.all() {
        // Find the y position for this annotation
        let y_pos = line_positions
            .iter()
            .find(|(offset, _)| *offset <= annotation.start)
            .map(|(_, y)| *y);

        if let Some(y) = y_pos {
            let icon = match annotation.kind {
                AnnotationKind::Highlight => "🖍",
                AnnotationKind::Note => "📝",
                AnnotationKind::Bookmark => "🔖",
            };

            let rect = Rect::from_min_size(Pos2::new(5.0, y), Vec2::new(20.0, 20.0));

            let response = ui.put(rect, egui::Label::new(icon).sense(egui::Sense::click()));

            if response.clicked() {
                clicked_id = Some(annotation.id.clone());
            }

            // Tooltip
            response.on_hover_ui(|ui| match annotation.kind {
                AnnotationKind::Highlight => {
                    ui.label("Highlight");
                }
                AnnotationKind::Note => {
                    if let Some(text) = &annotation.note_text {
                        ui.label(text);
                    }
                }
                AnnotationKind::Bookmark => {
                    ui.label("Bookmark");
                }
            });
        }
    }

    clicked_id
}

/// Render highlights over text
pub fn render_highlights(
    painter: &egui::Painter,
    annotations: &AnnotationStore,
    text_positions: &[(usize, Rect)], // (char_offset, rect)
) {
    for annotation in annotations.highlights() {
        let color = annotation
            .color
            .as_deref()
            .map(parse_hex_color)
            .unwrap_or(Color32::YELLOW)
            .gamma_multiply(0.4); // Semi-transparent

        for (offset, rect) in text_positions {
            if *offset >= annotation.start && *offset < annotation.end {
                painter.rect_filled(*rect, 0.0, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        let color = parse_hex_color("#ff0000");
        assert_eq!(color, Color32::from_rgb(255, 0, 0));

        let color = parse_hex_color("00ff00");
        assert_eq!(color, Color32::from_rgb(0, 255, 0));
    }

    #[test]
    fn test_annotation_popup() {
        let mut popup = AnnotationPopup::new();
        assert!(!popup.visible);

        popup.show(Pos2::new(100.0, 100.0), (10, 20));
        assert!(popup.visible);
        assert_eq!(popup.selection, (10, 20));

        popup.hide();
        assert!(!popup.visible);
    }
}
