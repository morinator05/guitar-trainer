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

/// Answer button. Correct = filled + ✔, wrong = outlined + ✖, so the state never
/// depends on colour alone.
fn answer_button(text: &str, state: Option<bool>, min: egui::Vec2) -> egui::Button<'static> {
    let size = if text.contains('\n') { 17.0 } else { 20.0 };
    match state {
        Some(true) => egui::Button::new(RichText::new(format!("✔ {text}")).size(size).strong().color(Color32::BLACK))
            .fill(theme::SUCCESS)
            .stroke(Stroke::new(2.0, theme::TEXT_STRONG)),
        Some(false) => egui::Button::new(RichText::new(format!("✖ {text}")).size(size).strong().color(theme::ERROR))
            .fill(theme::EXTREME)
            .stroke(Stroke::new(3.0, theme::ERROR)),
        None => egui::Button::new(RichText::new(text.to_string()).size(size)),
    }
    .min_size(min)
}

/// 12 buttons, one per pitch class. Returns the clicked pitch class.
pub fn note_picker(ui: &mut Ui, naming: Naming, marks: Marks, enabled: bool) -> Option<Pc> {
    let mut out = None;
    // one row of 12 equally wide buttons when it fits (fixed positions = muscle memory)
    let gap = 6.0;
    let w = (((ui.available_width() - 12.0 * gap) / 12.0).floor() - 1.0).clamp(38.0, 110.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.spacing_mut().button_padding.x = 4.0;
        for pc in 0..12u8 {
            let state = if marks.correct == Some(pc) {
                Some(true)
            } else if marks.wrong == Some(pc) {
                Some(false)
            } else {
                None
            };
            // sharp and flat name stacked, so 12 buttons fit in one row
            let label = pc_button_label(pc, naming).replace('/', "\n");
            let b = answer_button(&label, state, egui::vec2(w, 56.0));
            if ui.add_enabled(enabled, b).clicked() {
                out = Some(pc);
            }
        }
    });
    if enabled && out.is_none() {
        out = letter_key(ui, naming);
    }
    out
}

/// Natural notes from the keyboard: C D E F G A B (German: H = B natural, B = B flat).
fn letter_key(ui: &Ui, naming: Naming) -> Option<Pc> {
    use egui::Key::*;
    let german = naming != Naming::English;
    let mut keys = vec![(C, 0), (D, 2), (E, 4), (F, 5), (G, 7), (A, 9)];
    if german {
        keys.extend([(H, 11), (B, 10)]);
    } else {
        keys.push((B, 11));
    }
    ui.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } if modifiers.is_none() => {
                keys.iter().find(|(k, _)| k == key).map(|(_, pc)| *pc)
            }
            _ => None,
        })
    })
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
            if ui.add_enabled(enabled, answer_button(it, state, egui::vec2(72.0, 48.0))).clicked() {
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
                    Stroke::new(2.0, theme::OUTLINE),
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
    ui.add(
        egui::Button::new(RichText::new(text).size(18.0).strong().color(Color32::WHITE))
            .fill(theme::PRIMARY)
            .stroke(Stroke::new(1.0, theme::ACCENT_HOVER))
            .min_size(egui::vec2(0.0, 40.0)),
    )
}

pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(21.0).strong().color(theme::TEXT_STRONG));
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(15.0).color(theme::TEXT_WEAK));
}
