use super::common::Ctx;
use crate::settings::LabelMode;
use crate::theory::chord_db;
use crate::theory::note::Naming;
use crate::theory::tuning::ALL_TUNINGS;
use crate::widgets::{heading, hint};
use egui::{RichText, Ui};

pub fn ui(ui: &mut Ui, ctx: &mut Ctx) {
    egui::ScrollArea::vertical().id_salt("settings").show(ui, |ui| {
        let s = &mut *ctx.settings;
        heading(ui, "Instrument");
        ui.horizontal(|ui| {
            ui.label("Tuning:");
            for t in ALL_TUNINGS {
                ui.selectable_value(&mut s.tuning, t.id, t.name);
            }
        });
        ui.add(egui::Slider::new(&mut s.frets, 12..=24).text("frets shown"));
        ui.checkbox(&mut s.left_handed, "left-handed (mirror the neck)");

        ui.add_space(8.0);
        heading(ui, "Names and labels");
        ui.horizontal(|ui| {
            ui.label("Note names:");
            for n in Naming::ALL {
                ui.selectable_value(&mut s.naming, n, n.label());
            }
        });
        hint(ui, "German: H = B natural, B = Bb. Chord sheets and tabs on the internet use the English names.");
        ui.horizontal(|ui| {
            ui.label("Dots show:");
            for l in LabelMode::ALL {
                ui.selectable_value(&mut s.labels, l, l.label());
            }
        });

        ui.add_space(8.0);
        heading(ui, "Sound");
        ui.checkbox(&mut s.sound, "sound on");
        if ui.add(egui::Slider::new(&mut s.volume, 0.0..=1.0).text("volume")).changed() {
            ctx.audio.set_volume(s.volume);
        }
        ui.add(egui::Slider::new(&mut s.bpm, 40.0..=220.0).text("default BPM"));
        if let Some(e) = &ctx.audio.error {
            ui.label(RichText::new(format!("Audio not available: {e}")).color(egui::Color32::from_rgb(230, 80, 80)));
        }

        ui.add_space(8.0);
        heading(ui, "Display");
        ui.add(egui::Slider::new(&mut s.ui_scale, 0.8..=2.0).text("UI scale"));
        ui.checkbox(&mut s.dark, "dark mode");

        ui.add_space(16.0);
        heading(ui, "About");
        ui.label("Guitar Trainer – runs fully offline. Your settings and progress are stored locally in:");
        ui.monospace(crate::storage::data_dir().display().to_string());
        let (total, bad) = chord_db::stats();
        ui.label(format!(
            "Chord shapes: {total} hand-made shapes from the chords-db project ({bad} filtered out because they failed verification), plus generated voicings for every tuning."
        ));
        ui.label("Credits / licences:");
        ui.label("• chords-db by David Rubert (tombatossals) – MIT licence");
        ui.label("• Bravura music font by Steinberg Media Technologies – SIL Open Font License 1.1");
        ui.label("• egui/eframe, cpal, serde – MIT / Apache-2.0");
        hint(ui, "Full licence texts: data/LICENSES.md in the source folder.");
    });
}
