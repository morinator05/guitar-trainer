pub mod chord_diagram;
pub mod fretboard;
pub mod notation;
pub mod wood;

use crate::theme;
use crate::theory::note::{Naming, Pc, pc_button_label};
use egui::{Color32, Response, RichText, Stroke, Ui};

/// Feedback colouring for answer buttons.
#[derive(Clone, Copy, Default)]
pub struct Marks {
    pub correct: Option<Pc>,
    pub wrong: Option<Pc>,
}

fn answer_button(text: &str, state: Option<bool>, min: egui::Vec2) -> egui::Button<'static> {
    let t = RichText::new(text.to_string()).size(18.0);
    match state {
        Some(true) => egui::Button::new(t.color(Color32::BLACK)).fill(theme::SUCCESS),
        Some(false) => egui::Button::new(t.color(Color32::BLACK)).fill(theme::ERROR),
        None => egui::Button::new(t),
    }
    .min_size(min)
}

/// 12 buttons, one per pitch class. Returns the clicked pitch class.
pub fn note_picker(ui: &mut Ui, naming: Naming, marks: Marks, enabled: bool) -> Option<Pc> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        for pc in 0..12u8 {
            let state = if marks.correct == Some(pc) {
                Some(true)
            } else if marks.wrong == Some(pc) {
                Some(false)
            } else {
                None
            };
            let b = answer_button(&pc_button_label(pc, naming), state, egui::vec2(66.0, 42.0));
            if ui.add_enabled(enabled, b).clicked() {
                out = Some(pc);
            }
        }
    });
    out
}

/// Row of text buttons; returns the index clicked.
pub fn choice_buttons(ui: &mut Ui, items: &[String], correct: Option<usize>, wrong: Option<usize>, enabled: bool) -> Option<usize> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        for (i, it) in items.iter().enumerate() {
            let state = if correct == Some(i) {
                Some(true)
            } else if wrong == Some(i) {
                Some(false)
            } else {
                None
            };
            if ui.add_enabled(enabled, answer_button(it, state, egui::vec2(58.0, 38.0))).clicked() {
                out = Some(i);
            }
        }
    });
    out
}

/// Top-level mode tabs of a screen. Looks different from filter chips:
/// bigger text, no background, accent underline on the active tab.
pub fn tab_bar<T: PartialEq + Copy>(ui: &mut Ui, current: &mut T, tabs: &[(T, &str)]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 18.0;
        for (t, label) in tabs {
            let sel = *current == *t;
            let color = if sel { theme::TEXT_STRONG } else { theme::TEXT_WEAK };
            let r = ui.add(egui::Button::new(RichText::new(*label).size(18.0).color(color)).frame(false));
            let line_y = r.rect.bottom() + 3.0;
            if sel {
                ui.painter().line_segment(
                    [egui::pos2(r.rect.left(), line_y), egui::pos2(r.rect.right(), line_y)],
                    Stroke::new(3.0, theme::ACCENT),
                );
            } else if r.hovered() {
                ui.painter().line_segment(
                    [egui::pos2(r.rect.left(), line_y), egui::pos2(r.rect.right(), line_y)],
                    Stroke::new(2.0, theme::STROKE),
                );
            }
            if r.clicked() && !sel {
                *current = *t;
                changed = true;
            }
        }
    });
    ui.add_space(4.0);
    ui.separator();
    changed
}

/// The one accent-filled button for the main action of a screen.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let fill = theme::ACCENT;
    ui.add(
        egui::Button::new(RichText::new(text).size(17.0).color(theme::TEXT_STRONG))
            .fill(fill)
            .min_size(egui::vec2(0.0, 36.0)),
    )
}

pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(21.0).strong().color(theme::TEXT_STRONG));
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(14.0).color(theme::TEXT_WEAK));
}
