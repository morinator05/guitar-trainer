//! "What's this chord?" – click notes (max one per string) and get the chord name,
//! alternative names, the notes written out and the scales that fit.

use super::common::*;
use crate::theme;
use crate::theory::fretboard::{Pos, midi_at, pc_at};
use crate::theory::identify::{Analysis, analyse, bass_intervals, fitting_scales};
use crate::theory::interval::interval_color;
use crate::theory::note::{Note, Pc, Pitch};
use crate::widgets::fretboard::{Dot, contrast_text, rgb};
use crate::widgets::notation::{Column, NotationView, STAFF_LIB};
use crate::widgets::{heading, hint};
use egui::{RichText, Ui};

#[derive(Default)]
pub struct Identify {
    /// fret per string (0 = lowest string), None = not played
    frets: [Option<u8>; 6],
    /// which interpretation is shown (index into the candidates)
    chosen: usize,
}

impl Identify {
    fn tab_string(&self) -> String {
        let big = self.frets.iter().any(|f| matches!(f, Some(f) if *f >= 10));
        let parts: Vec<String> = self
            .frets
            .iter()
            .map(|f| f.map(|f| f.to_string()).unwrap_or_else(|| "x".into()))
            .collect();
        if big { parts.join("-") } else { parts.concat() }
    }

    fn positions(&self) -> Vec<Pos> {
        (0..6u8).filter_map(|s| self.frets[s as usize].map(|f| Pos::new(s, f))).collect()
    }

    pub fn ui(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let t = ctx.tuning();
        // notes from a previous, longer neck setting
        for f in self.frets.iter_mut() {
            if matches!(f, Some(x) if *x > ctx.settings.frets) {
                *f = None;
            }
        }
        let positions = self.positions();
        let midis: Vec<i32> = positions.iter().map(|p| midi_at(t, *p)).collect();
        let a: Analysis = analyse(&midis);
        if self.chosen >= a.candidates.len() {
            self.chosen = 0;
        }
        let cand = a.candidates.get(self.chosen);
        let naming = ctx.settings.naming;

        prompt(ui, "What's this chord?");
        hint(ui, "Click notes on the fretboard – at most one per string, click again to remove. Strings without a note are muted.");

        // fretboard
        let dots: Vec<Dot> = positions
            .iter()
            .map(|&p| {
                let pc = pc_at(t, p);
                match cand {
                    Some(c) => ctx.interval_dot(p, c.root, c.interval_of(pc), c.spell(pc)),
                    None => Dot::new(p, ctx.pc_name(pc), MARKER),
                }
            })
            .collect();
        let mut board = ctx.board(&dots);
        board.clickable = true;
        if let Some(p) = board.show(ui) {
            let s = p.string as usize;
            self.frets[s] = if self.frets[s] == Some(p.fret) { None } else { Some(p.fret) };
            self.chosen = 0;
            if self.frets[s].is_some() {
                ctx.play_note(midi_at(t, p));
            }
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new(self.tab_string()).monospace().size(18.0));
            if ui.button("Clear").clicked() {
                self.frets = [None; 6];
                self.chosen = 0;
            }
            if ui.add_enabled(!midis.is_empty(), egui::Button::new("▶ strum")).clicked() {
                let mut m = midis.clone();
                m.sort();
                ctx.play_chord(&m);
            }
        });
        ui.add_space(6.0);

        match a.pcs.len() {
            0 => {
                hint(ui, "No notes yet. Tip: play a shape you like on your guitar, then click the same frets here. (Listening through the microphone comes later.)");
                return;
            }
            1 => {
                ui.label(RichText::new(format!("Single note: {}", ctx.pc_name(a.pcs[0]))).size(20.0));
                if midis.len() > 1 {
                    hint(ui, "All clicked notes are the same note (in different octaves). Add another note to get a chord.");
                }
                return;
            }
            _ => {}
        }

        egui::ScrollArea::vertical().id_salt("identify_results").show(ui, |ui| {
        // best name + details
        if let Some(c) = cand {
            ui.horizontal(|ui| {
                ui.label(RichText::new(c.symbol(naming)).size(40.0).strong().color(theme::TEXT_STRONG));
                ui.vertical(|ui| {
                    ui.label(RichText::new(c.description(naming)).size(17.0));
                    ui.label(RichText::new(c.inversion()).color(theme::TEXT_WEAK));
                });
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Formula:");
                for i in c.ty.intervals().into_iter().chain(c.tensions.iter().copied()) {
                    let n = i.apply(c.root);
                    if c.missing.contains(&i) {
                        ui.label(RichText::new(format!(" ({} = {}, not played) ", i.label(), ctx.name(n))).color(theme::TEXT_WEAK));
                    } else {
                        let col = rgb(interval_color(i.pc_offset()));
                        ui.label(
                            RichText::new(format!(" {} = {} ", i.label(), ctx.name(n)))
                                .size(16.0)
                                .background_color(col)
                                .color(contrast_text(col)),
                        );
                    }
                }
            });
            if let Some(d) = &a.dyad {
                ui.label(RichText::new(format!("Two different notes: {d}")).size(16.0));
            }
            if let Some(id) = c.catalog_id
                && ui.button(format!("📚 Open {} in the chord library", c.symbol(naming).split('/').next().unwrap_or(""))).clicked()
            {
                *ctx.nav = Some(Nav::Chord { root: c.root, ty: id });
            }

            if a.candidates.len() > 1 {
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label("Also known as:");
                    for (i, other) in a.candidates.iter().enumerate() {
                        if ui.selectable_label(i == self.chosen, other.symbol(naming)).on_hover_text(other.description(naming)).clicked() {
                            self.chosen = i;
                        }
                    }
                });
                hint(ui, "Same notes, different names: the first one is the most common. Click a name to see the notes as intervals of that chord.");
            }
        } else if let Some(d) = &a.dyad {
            ui.label(RichText::new(format!("Interval: {d}")).size(22.0));
        } else {
            ui.label(RichText::new("No common chord name for this combination.").size(18.0));
        }

        ui.add_space(8.0);
        ui.horizontal_top(|ui| {
            // notes written out
            ui.vertical(|ui| {
                heading(ui, "Notes, low to high");
                let spell = |pc: Pc| cand.map(|c| c.spell(pc)).unwrap_or_else(|| Note::from_pc(pc));
                let mut sorted: Vec<(i32, Pos)> = positions.iter().map(|p| (midi_at(t, *p), *p)).collect();
                sorted.sort();
                let cols = [Column {
                    pitches: sorted
                        .iter()
                        .map(|(m, _)| Pitch::from_midi_spelled(*m, spell(m.rem_euclid(12) as Pc)))
                        .collect(),
                    tab: positions.clone(),
                    label: String::new(),
                }];
                let mut nv = NotationView::new(&cols);
                nv.staff_space = STAFF_LIB;
                ui.allocate_ui(egui::vec2(200.0, 260.0), |ui| nv.show(ui));
                let names: Vec<String> = sorted.iter().map(|(m, _)| Pitch::from_midi_spelled(*m, spell(m.rem_euclid(12) as Pc)).name(naming)).collect();
                ui.label(names.join("  "));
            });
            ui.separator();
            ui.vertical(|ui| {
                heading(ui, "From the bass note");
                let spell = |pc: Pc| cand.map(|c| c.spell(pc)).unwrap_or_else(|| Note::from_pc(pc));
                for (n, int) in bass_intervals(&midis, spell, naming) {
                    ui.label(format!("{n}: {int} above the bass"));
                }
                ui.add_space(10.0);
                heading(ui, "Scales that fit");
                hint(ui, "Every note of the chord is in these scales – good choices for improvising over it.");
                let scales = fitting_scales(&a.pcs, cand.map(|c| c.root.pc()), 10);
                if scales.is_empty() {
                    ui.label("None of the scales in the library contains all these notes.");
                }
                ui.horizontal_wrapped(|ui| {
                    for s in scales {
                        if ui.button(format!("{} {}", ctx.name(s.root), s.ty.name)).clicked() {
                            *ctx.nav = Some(Nav::Scale { root: s.root, id: s.ty.id });
                        }
                    }
                });
            });
        });
        });
    }
}
