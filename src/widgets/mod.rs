pub mod chord_diagram;
pub mod fretboard;
pub mod notation;

use crate::theory::note::{Naming, Pc, pc_button_label};
use egui::{Color32, RichText, Ui};

/// Feedback colouring for answer buttons.
#[derive(Clone, Copy, Default)]
pub struct Marks {
    pub correct: Option<Pc>,
    pub wrong: Option<Pc>,
}

/// 12 buttons, one per pitch class. Returns the clicked pitch class.
pub fn note_picker(ui: &mut Ui, naming: Naming, marks: Marks, enabled: bool) -> Option<Pc> {
    let mut out = None;
    ui.horizontal_wrapped(|ui| {
        for pc in 0..12u8 {
            let label = pc_button_label(pc, naming);
            let mut text = RichText::new(label).size(18.0);
            let mut b = egui::Button::new(text.clone()).min_size(egui::vec2(64.0, 40.0));
            if marks.correct == Some(pc) {
                text = text.color(Color32::BLACK);
                b = egui::Button::new(text).fill(Color32::from_rgb(90, 200, 110)).min_size(egui::vec2(64.0, 40.0));
            } else if marks.wrong == Some(pc) {
                text = text.color(Color32::WHITE);
                b = egui::Button::new(text).fill(Color32::from_rgb(210, 60, 60)).min_size(egui::vec2(64.0, 40.0));
            }
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
            let mut b = egui::Button::new(RichText::new(it).size(17.0)).min_size(egui::vec2(56.0, 36.0));
            if correct == Some(i) {
                b = egui::Button::new(RichText::new(it).size(17.0).color(Color32::BLACK))
                    .fill(Color32::from_rgb(90, 200, 110))
                    .min_size(egui::vec2(56.0, 36.0));
            } else if wrong == Some(i) {
                b = egui::Button::new(RichText::new(it).size(17.0).color(Color32::WHITE))
                    .fill(Color32::from_rgb(210, 60, 60))
                    .min_size(egui::vec2(56.0, 36.0));
            }
            if ui.add_enabled(enabled, b).clicked() {
                out = Some(i);
            }
        }
    });
    out
}

pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(22.0).strong());
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).italics().color(ui.visuals().weak_text_color()));
}
