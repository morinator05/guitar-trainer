//! Start screen: what to practise today, in one click.

use super::common::Ctx;
use super::progress::Session;
use crate::theme;
use crate::widgets::{heading, hint};
use egui::{RichText, Ui};

pub enum Action {
    StartSession,
    Fretboard,
    Chords,
    Scales,
}

pub fn ui(ui: &mut Ui, ctx: &mut Ctx) -> Option<Action> {
    let mut out = None;
    egui::ScrollArea::vertical().id_salt("today").show(ui, |ui| {
        ui.add_space(6.0);
        ui.label(RichText::new("Today's practice").size(30.0).strong().color(theme::TEXT_STRONG));
        let d = ctx.progress.today();
        let streak = ctx.progress.streak();
        let status = if d.attempts == 0 {
            "Nothing practised yet today. About 10 minutes is enough.".to_string()
        } else {
            format!(
                "{} answers today, {} correct · streak: {} day{}",
                d.attempts,
                d.correct,
                streak,
                if streak == 1 { "" } else { "s" }
            )
        };
        ui.label(RichText::new(status).size(18.0));
        ui.add_space(12.0);

        let preview = Session::new(ctx.progress);
        ui.label(RichText::new("The session picks your weakest areas (6 questions each):").size(17.0));
        ui.add_space(4.0);
        for (i, d) in preview.plan().iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}.", i + 1)).size(17.0).strong());
                ui.label(RichText::new(d.label()).size(17.0).strong().color(theme::TEXT_STRONG));
            });
            ui.horizontal(|ui| {
                ui.add_space(22.0);
                hint(ui, d.how());
            });
        }
        ui.add_space(12.0);
        let big = egui::Button::new(RichText::new("▶  Start today's practice").size(24.0).strong().color(egui::Color32::WHITE))
            .fill(theme::PRIMARY)
            .stroke(egui::Stroke::new(1.5, theme::ACCENT_HOVER))
            .min_size(egui::vec2(360.0, 60.0));
        if ui.add(big).clicked() || super::common::key_once(ui, egui::Key::Enter) {
            out = Some(Action::StartSession);
        }
        hint(ui, "Enter starts it. You can stop any time; answers are saved right away.");

        ui.add_space(24.0);
        heading(ui, "Or explore on your own");
        for (label, text, a) in [
            ("Fretboard", "See where every note is; drills to name and find notes.", Action::Fretboard),
            ("Chords", "Chord library with shapes and formulas; build, read and name chords.", Action::Chords),
            ("Scales", "Pentatonic boxes and more, played up and down; fill-the-box drills.", Action::Scales),
        ] {
            ui.horizontal(|ui| {
                let b = egui::Button::new(RichText::new(label).size(17.0)).min_size(egui::vec2(120.0, 36.0));
                if ui.add(b).clicked() {
                    out = Some(a);
                }
                ui.label(RichText::new(text).size(16.0));
            });
        }

        ui.add_space(24.0);
        heading(ui, "How the drills work");
        for l in [
            "Answer on screen (click, or type the note letter) – or play it on the guitar, reveal, and rate yourself.",
            "Right answers turn blue with ✔, wrong ones are outlined orange with ✖. After two misses the app shows the answer.",
            "Questions you get wrong come back sooner; ones you know well come back rarely.",
        ] {
            ui.label(RichText::new(format!("• {l}")).size(16.0));
        }
    });
    out
}
