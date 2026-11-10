//! Chords: library (voicings, notation, intervals), "build it" view, and drills
//! for sight-reading chords anywhere on the neck.

use super::common::*;
use super::fretboard_trainer::stats_line;
use crate::theory::chord::{CHORD_TYPES, Chord, ChordCategory, ChordType, common_roots};
use crate::theory::chord_db;
use crate::theory::fretboard::{Pos, midi_at, pc_at};
use crate::theory::interval::interval_color;
use crate::theory::note::{Note, Pc, Pitch};
use crate::theory::tuning::{Tuning, TuningId};
use crate::theory::voicing::{self, GenOptions, Source, Voicing};
use crate::widgets::chord_diagram::ChordDiagram;
use crate::widgets::fretboard::{Dot, rgb};
use crate::widgets::notation::{Column, NotationView, STAFF_DRILL, STAFF_LIB};
use crate::widgets::{Marks, choice_buttons, heading, hint, note_picker, primary_button, tab_bar};
use egui::{Color32, RichText, ScrollArea, Ui};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Library,
    BuildIt,
    SightRead,
    NameIt,
    Identify,
}

const REGIONS: [(&str, Option<(u8, u8)>); 6] = [
    ("anywhere", None),
    ("open (0–4)", Some((0, 4))),
    ("frets 3–7", Some((3, 7))),
    ("frets 5–9", Some((5, 9))),
    ("frets 7–11", Some((7, 11))),
    ("frets 9–15", Some((9, 15))),
];

const DRILL_ZONES: [(u8, u8); 6] = [(0, 4), (2, 6), (4, 8), (5, 9), (7, 11), (9, 13)];

/// The 12 roots with the spelling most common in chord charts.
fn drill_roots() -> Vec<Note> {
    ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]
        .iter()
        .map(|s| Note::parse(s).unwrap())
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    Symbol,
    Staff,
    Mixed,
}

struct ReadQ {
    chord: Chord,
    staff: bool,
    zone: (u8, u8),
}

struct NameQ {
    chord: Chord,
    voicing: Option<Voicing>,
    root_answer: Option<Pc>,
    type_answer: Option<usize>,
}

pub struct Chords {
    pub tab: Tab,
    root: Note,
    ty: &'static ChordType,
    region: usize,
    root_string: Option<u8>,
    inversions: bool,
    more: bool,
    selected: usize,
    label_fingers: bool,
    cache: HashMap<(TuningId, Pc, &'static str), Vec<Voicing>>,
    // build it
    build_string: u8,
    // drills
    pool: [bool; 3],
    natural_roots: bool,
    prompt_kind: PromptKind,
    answer: AnswerMode,
    read_q: Option<ReadQ>,
    clicked: [Option<u8>; 6],
    name_q: Option<NameQ>,
    started: f64,
    wrong_once: bool,
    solved: bool,
    revealed: bool,
    feedback: Option<Feedback>,
    advance_at: Option<f64>,
    marks: Marks,
    wrong_type: Option<usize>,
    stamp: Option<Stamp>,
    identify: super::identify::Identify,
}

impl Default for Chords {
    fn default() -> Self {
        Chords {
            tab: Tab::Library,
            root: Note::parse("A").unwrap(),
            ty: ChordType::by_id("min").unwrap(),
            region: 0,
            root_string: None,
            inversions: false,
            more: false,
            selected: 0,
            label_fingers: false,
            cache: HashMap::new(),
            build_string: 0,
            pool: [true, false, false],
            natural_roots: true,
            prompt_kind: PromptKind::Symbol,
            answer: AnswerMode::OnScreen,
            read_q: None,
            clicked: [None; 6],
            name_q: None,
            started: 0.0,
            wrong_once: false,
            solved: false,
            revealed: false,
            feedback: None,
            advance_at: None,
            marks: Marks::default(),
            wrong_type: None,
            stamp: None,
            identify: Default::default(),
        }
    }
}

fn fits_zone(v: &Voicing, zone: (u8, u8)) -> bool {
    v.frets.iter().flatten().all(|&f| (f == 0 && zone.0 <= 1) || (f >= zone.0.max(1) && f <= zone.1))
}

impl Chords {
    pub fn open(&mut self, root: Note, ty: &'static str) {
        if let Some(t) = ChordType::by_id(ty) {
            self.root = root;
            self.ty = t;
            self.tab = Tab::Library;
            self.selected = 0;
        }
    }

    pub fn set_tab(&mut self, t: Tab) {
        self.tab = t;
        self.read_q = None;
        self.name_q = None;
    }

    fn voicings(&mut self, t: &'static Tuning, chord: &Chord) -> &Vec<Voicing> {
        self.cache.entry((t.id, chord.root_pc(), chord.ty.id)).or_insert_with(|| {
            let mut v = chord_db::voicings(t, chord);
            for g in voicing::generate(t, chord, GenOptions { inversions: false, ..Default::default() }) {
                if !v.iter().any(|x| x.frets == g.frets) {
                    v.push(g);
                }
            }
            v
        })
    }

    pub fn ui(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let mut tab = self.tab;
        if tab_bar(
            ui,
            &mut tab,
            &[
                (Tab::Library, "📚 Library"),
                (Tab::BuildIt, "🔨 Build it"),
                (Tab::SightRead, "👁 Sight-read drill"),
                (Tab::NameIt, "❓ Name this chord"),
                (Tab::Identify, "🔎 Identify"),
            ],
        ) {
            self.set_tab(tab);
        }
        // tuning or fret count changed → the current question is stale
        if self.stamp.is_some() && self.stamp != Some(ctx.stamp()) {
            self.read_q = None;
            self.name_q = None;
            self.stamp = None;
        }
        match self.tab {
            Tab::Library => self.library(ui, ctx),
            Tab::BuildIt => self.build_it(ui, ctx),
            Tab::SightRead => self.sight_read(ui, ctx),
            Tab::NameIt => self.name_it(ui, ctx),
            Tab::Identify => self.identify.ui(ui, ctx),
        }
    }

    /// Between questions (nothing half-answered on screen)?
    pub fn is_idle(&self) -> bool {
        !self.solved && !self.revealed && self.advance_at.is_none()
    }

    fn chord_picker(&mut self, ui: &mut Ui, ctx: &Ctx) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Root:");
            for r in common_roots() {
                if ui
                    .selectable_label(self.root == r, RichText::new(ctx.name(r)).size(16.0))
                    .clicked()
                {
                    self.root = r;
                    self.selected = 0;
                }
            }
        });
        for cat in ChordCategory::ALL {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{}:", cat.label())).strong());
                for t in CHORD_TYPES.iter().filter(|t| t.category == cat) {
                    let sym = format!("{}{}", ctx.name(self.root), t.suffix);
                    if ui
                        .selectable_label(std::ptr::eq(self.ty, t), RichText::new(sym).size(16.0))
                        .on_hover_text(t.name)
                        .clicked()
                    {
                        self.ty = t;
                        self.selected = 0;
                    }
                }
            });
        }
    }

    fn chord_header(&self, ui: &mut Ui, ctx: &Ctx, chord: &Chord) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(chord.symbol(ctx.settings.naming)).size(34.0).strong());
            ui.label(RichText::new(chord.ty.name).size(20.0));
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Formula:").size(16.0));
            for (i, n) in chord.ty.intervals().iter().zip(chord.notes()) {
                let c = rgb(interval_color(i.pc_offset()));
                ui.label(
                    RichText::new(format!(" {} = {} ", i.label(), ctx.name(n)))
                        .size(17.0)
                        .background_color(c)
                        .color(crate::widgets::fretboard::contrast_text(c)),
                );
            }
        });
        let names: Vec<String> = chord
            .ty
            .intervals()
            .iter()
            .filter(|i| !i.long_name().is_empty())
            .map(|i| format!("{} ({})", i.label(), i.long_name()))
            .collect();
        hint(ui, &names.join(" · "));
        ui.label(RichText::new(chord.ty.info).size(15.0));
    }

    /// Close-position pitches for the staff (root between C3 and B3, written C4–B4).
    fn close_position(chord: &Chord) -> Vec<Pitch> {
        let root_midi = 48 + chord.root_pc() as i32;
        chord
            .ty
            .intervals()
            .iter()
            .map(|i| Pitch::from_midi_spelled(root_midi + i.semitones(), i.apply(chord.root)))
            .collect()
    }

    fn voicing_pitches(t: &Tuning, chord: &Chord, v: &Voicing) -> Vec<Pitch> {
        v.played()
            .map(|p| Pitch::from_midi_spelled(midi_at(t, p), chord.spell(pc_at(t, p))))
            .collect()
    }

    fn diagram_labels(&self, ctx: &Ctx, chord: &Chord, v: &Voicing, fingers: bool) -> ([String; 6], [Color32; 6]) {
        let t = ctx.tuning();
        let mut labels: [String; 6] = Default::default();
        let mut colors = [GREY; 6];
        for s in 0..6 {
            if let Some(f) = v.frets[s] {
                let pc = pc_at(t, Pos::new(s as u8, f));
                let semis = (pc as i32 - chord.root_pc() as i32).rem_euclid(12) as Pc;
                colors[s] = rgb(interval_color(semis));
                labels[s] = if fingers {
                    if v.fingers[s] > 0 { v.fingers[s].to_string() } else { String::new() }
                } else {
                    match ctx.settings.labels {
                        crate::settings::LabelMode::Notes => ctx.name(chord.spell(pc)),
                        _ => chord.interval_of(pc).map(|i| i.label()).unwrap_or_default(),
                    }
                };
            }
        }
        (labels, colors)
    }

    // ------------------------------------------------------------------ library
    fn library(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        self.chord_picker(ui, ctx);
        ui.separator();
        let t = ctx.tuning();
        let chord = Chord::new(self.root, self.ty);
        ScrollArea::vertical().id_salt("chord_lib").show(ui, |ui| {
            self.chord_header(ui, ctx, &chord);

            // filters
            ui.horizontal_wrapped(|ui| {
                ui.label("Region:");
                for (i, (name, _)) in REGIONS.iter().enumerate() {
                    if ui.selectable_label(self.region == i, *name).clicked() {
                        self.region = i;
                        self.selected = 0;
                    }
                }
                ui.separator();
                ui.label("Root on string:");
                for (label, val) in [("any", None), ("6", Some(0u8)), ("5", Some(1)), ("4", Some(2))] {
                    if ui.selectable_label(self.root_string == val, label).clicked() {
                        self.root_string = val;
                        self.selected = 0;
                    }
                }
                ui.separator();
                ui.checkbox(&mut self.inversions, "inversions (DB)");
                ui.checkbox(&mut self.label_fingers, "show fingers");
            });

            let region = REGIONS[self.region].1;
            let root_string = self.root_string;
            let inversions = self.inversions;
            let all = self.voicings(t, &chord).clone();
            let filtered: Vec<Voicing> = all
                .into_iter()
                .filter(|v| region.is_none_or(|z| fits_zone(v, z)))
                .filter(|v| {
                    let root_bass = v.bass_pos().map(|p| pc_at(t, p)) == Some(chord.root_pc());
                    if !root_bass && !inversions {
                        return false;
                    }
                    root_string.is_none_or(|s| v.bass_string() == Some(s) && root_bass)
                })
                .collect();
            let db: Vec<&Voicing> = filtered.iter().filter(|v| v.source == Source::Database).collect();
            let mut generated: Vec<&Voicing> = filtered.iter().filter(|v| v.source == Source::Generated).collect();
            generated.sort_by(|a, b| a.score.total_cmp(&b.score));
            let shown_gen = if self.more { generated.len() } else { generated.len().min(18) };
            let list: Vec<&Voicing> = db.iter().copied().chain(generated.iter().copied().take(shown_gen)).collect();
            if self.selected >= list.len() {
                self.selected = 0;
            }

            if list.is_empty() {
                ui.label(RichText::new("No voicing with these filters – try another region.").size(16.0));
                return;
            }
            let sel = list[self.selected].clone();

            // selected voicing big
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                heading(ui, &format!("Selected: {}", sel.tab_string()));
                let fam = sel.family(chord.ty.is_power_chord());
                ui.label(format!(
                    "{} · root on string {} · {}",
                    fam.label(),
                    sel.bass_string().map(|s| Tuning::string_number(s as usize)).unwrap_or(0),
                    if sel.source == Source::Database { "★ hand-made shape" } else { "generated" }
                ));
                if ui.button(RichText::new("▶ strum").size(16.0)).clicked() {
                    ctx.play_chord(&sel.midi_notes(t));
                }
                if ui.button(RichText::new("▶ arpeggio").size(16.0)).clicked() && ctx.settings.sound {
                    let bpm = ctx.settings.bpm * 2.0;
                    ctx.audio.sequence(&sel.midi_notes(t), bpm, 1.0);
                }
            });
            let dots: Vec<Dot> = sel
                .played()
                .map(|p| {
                    let pc = pc_at(t, p);
                    ctx.interval_dot(p, chord.root, chord.interval_of(pc), chord.spell(pc))
                })
                .collect();
            let mut board = ctx.board(&dots);
            board.zone = Some((if sel.has_open() { 0 } else { sel.min_fret() }, sel.max_fret().max(1)));
            board.height = Some(220.0);
            board.show(ui);

            // voicing strip (left) + notation of the selection (right column)
            ui.add_space(6.0);
            let label_fingers = self.label_fingers;
            let mut clicked = None;
            let strip_w = (ui.available_width() - 330.0).max(360.0);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(egui::vec2(strip_w, 4000.0), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    heading(ui, &format!("Voicings ({} hand-made ★, {} generated)", db.len(), generated.len()));
                    hint(ui, "Click a diagram to select and hear it. ★ = hand-made shape from the chord database.");
                    ui.horizontal_wrapped(|ui| {
                        for (i, v) in list.iter().enumerate() {
                            let (labels, colors) = self.diagram_labels(ctx, &chord, v, label_fingers);
                            let d = ChordDiagram {
                                voicing: v,
                                labels,
                                colors,
                                width: 112.0,
                                title: Some(if v.source == Source::Database { "★".into() } else { String::new() }),
                                selected: i == self.selected,
                                subtitle: Some(v.tab_string()),
                            };
                            if d.show(ui).clicked() {
                                clicked = Some(i);
                            }
                        }
                    });
                    if generated.len() > 18 {
                        let label = if self.more { "show fewer" } else { "show all generated voicings" };
                        if ui.button(label).clicked() {
                            self.more = !self.more;
                        }
                    }
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label(RichText::new("This voicing, written out:").strong());
                    let cols = [Column {
                        pitches: Self::voicing_pitches(t, &chord, &sel),
                        tab: sel.played().collect(),
                        label: String::new(),
                    }];
                    let mut nv = NotationView::new(&cols);
                    nv.staff_space = STAFF_LIB;
                    ui.allocate_ui(egui::vec2(200.0, 260.0), |ui| nv.show(ui));
                    ui.add_space(6.0);
                    ui.label(RichText::new("Chord tones, close position:").strong());
                    let cols = [Column {
                        pitches: Self::close_position(&chord),
                        tab: Vec::new(),
                        label: String::new(),
                    }];
                    let mut nv = NotationView::new(&cols);
                    nv.tab = false;
                    nv.staff_space = STAFF_LIB;
                    ui.allocate_ui(egui::vec2(200.0, 140.0), |ui| nv.show(ui));
                });
            });
            if let Some(i) = clicked {
                self.selected = i;
                ctx.play_chord(&list[i].midi_notes(t));
            }
        });
    }

    // ------------------------------------------------------------------ build it
    fn build_it(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        self.chord_picker(ui, ctx);
        ui.separator();
        let t = ctx.tuning();
        let chord = Chord::new(self.root, self.ty);
        self.chord_header(ui, ctx, &chord);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Root on string:").size(16.0));
            for s in [0u8, 1, 2, 3] {
                ui.selectable_value(&mut self.build_string, s, Tuning::string_number(s as usize).to_string());
            }
        });
        let rs = self.build_string;
        let root_fret = (1..=12u8)
            .find(|&f| pc_at(t, Pos::new(rs, f)) == chord.root_pc())
            .unwrap_or(1);
        let lo = root_fret.saturating_sub(2);
        let hi = (root_fret + 3).min(ctx.settings.frets);
        let root_pos = Pos::new(rs, root_fret);

        let mut dots = Vec::new();
        for s in rs..6 {
            for f in lo..=hi {
                let p = Pos::new(s, f);
                let pc = pc_at(t, p);
                if let Some(i) = chord.interval_of(pc) {
                    let mut d = ctx.interval_dot(p, chord.root, Some(i), chord.spell(pc));
                    if ctx.settings.labels == crate::settings::LabelMode::Notes {
                        d.label = format!("{}", i.label());
                    }
                    if p == root_pos {
                        d = d.emphasis();
                    }
                    dots.push(d);
                }
            }
        }
        let mut board = ctx.board(&dots);
        board.zone = Some((lo, hi));
        board.clickable = true;
        if let Some(p) = board.show(ui) {
            ctx.play_note(midi_at(t, p));
        }
        hint(ui, "All chord tones inside the zone, labelled by interval. The square (1) is the root. Pick one tone per string to build your own voicing.");

        heading(ui, "Where is each chord tone, seen from the root?");
        // string pairs that are not a 4th shift shapes; worked out from the tuning
        let names = ctx.string_names();
        let odd: Vec<String> = (0..5)
            .filter_map(|s| {
                let d = t.open_midi(s + 1) - t.open_midi(s);
                (d != 5).then(|| {
                    let shift = if d < 5 { "one fret higher" } else { format!("{} frets lower", d - 5).leak() };
                    format!("{}→{} ({} semitones): shapes move {shift}", names[s], names[s + 1], d)
                })
            })
            .collect();
        if !odd.is_empty() {
            hint(ui, &format!("Most string pairs are a 4th apart. Exceptions in this tuning – {}.", odd.join("; ")));
        }
        for (i, n) in chord.ty.intervals().iter().zip(chord.notes()) {
            if i.number == 1 && i.alter == 0 {
                continue;
            }
            // every occurrence on a higher string inside the zone, closest first
            let mut found: Vec<(i32, Pos)> = Vec::new();
            for st in rs + 1..6 {
                for f in lo..=hi {
                    let p = Pos::new(st, f);
                    if pc_at(t, p) == n.pc() {
                        found.push(((f as i32 - root_fret as i32).abs() + (st - rs) as i32, p));
                    }
                }
            }
            found.sort_by_key(|x| x.0);
            let describe = |p: Pos| {
                let ds = p.string - rs;
                let df = p.fret as i32 - root_fret as i32;
                let frets = match df {
                    0 => "same fret".to_string(),
                    d if d > 0 => format!("{d} fret{} higher", if d > 1 { "s" } else { "" }),
                    d => format!("{} fret{} lower", -d, if d < -1 { "s" } else { "" }),
                };
                format!("{} string{} up, {}", ds, if ds > 1 { "s" } else { "" }, frets)
            };
            let text = if found.is_empty() {
                format!("{} ({}): not inside this zone", i.label(), ctx.name(n))
            } else {
                let ways: Vec<String> = found.iter().take(3).map(|(_, p)| describe(*p)).collect();
                format!("{} ({}): {}", i.label(), ctx.name(n), ways.join("   or   "))
            };
            // colour only in a chip; the text itself stays high-contrast
            let c = rgb(interval_color(i.pc_offset()));
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(" {} ", i.label()))
                        .size(16.0)
                        .strong()
                        .background_color(c)
                        .color(crate::widgets::fretboard::contrast_text(c)),
                );
                ui.label(RichText::new(text).size(16.0));
            });
        }
    }

    // ------------------------------------------------------------------ drills
    fn drill_settings(&mut self, ui: &mut Ui, ctx: &Ctx) -> bool {
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("Chord types:");
            for (i, c) in ChordCategory::ALL.iter().enumerate() {
                changed |= ui.checkbox(&mut self.pool[i], c.label()).changed();
            }
            if !self.pool.iter().any(|x| *x) {
                self.pool[0] = true;
            }
            ui.separator();
            let naturals: Vec<String> = [0u8, 2, 4, 5, 7, 9, 11].iter().map(|&pc| ctx.pc_name(pc)).collect();
            changed |= ui
                .checkbox(&mut self.natural_roots, format!("only natural roots ({})", naturals.join(" ")))
                .changed();
            if self.tab == Tab::SightRead {
                ui.separator();
                ui.label("Prompt:");
                for (k, l) in [(PromptKind::Symbol, "chord symbol"), (PromptKind::Staff, "notation"), (PromptKind::Mixed, "mixed")] {
                    changed |= ui.selectable_value(&mut self.prompt_kind, k, l).changed();
                }
            }
        });
        changed
    }

    fn pool_chords(&self) -> Vec<Chord> {
        let mut v = Vec::new();
        for ty in CHORD_TYPES {
            let idx = ChordCategory::ALL.iter().position(|c| *c == ty.category).unwrap();
            if !self.pool[idx] {
                continue;
            }
            for r in drill_roots() {
                if self.natural_roots && r.acc != 0 {
                    continue;
                }
                v.push(Chord::new(r, ty));
            }
        }
        v
    }

    fn reset_q(&mut self, ctx: &Ctx) {
        self.stamp = Some(ctx.stamp());
        self.clicked = [None; 6];
        self.started = ctx.time;
        self.wrong_once = false;
        self.solved = false;
        self.revealed = false;
        self.feedback = None;
        self.advance_at = None;
        self.marks = Marks::default();
        self.wrong_type = None;
    }

    fn new_read_q(&mut self, ctx: &mut Ctx) {
        let t = ctx.tuning();
        let chords = self.pool_chords();
        let keys: Vec<String> = chords.iter().map(|c| format!("ch:read:{}:{}", c.root_pc(), c.ty.id)).collect();
        for _ in 0..30 {
            let c = chords[ctx.progress.pick(ctx.rng, &keys)];
            let vs = self.voicings(t, &c).clone();
            let zones: Vec<(u8, u8)> = DRILL_ZONES
                .iter()
                .copied()
                .filter(|z| vs.iter().any(|v| fits_zone(v, *z) && v.bass_pos().map(|p| pc_at(t, p)) == Some(c.root_pc())))
                .collect();
            if let Some(&zone) = ctx.rng.pick(&zones) {
                let staff = match self.prompt_kind {
                    PromptKind::Symbol => false,
                    PromptKind::Staff => true,
                    PromptKind::Mixed => ctx.rng.below(2) == 0,
                };
                self.read_q = Some(ReadQ { chord: c, staff, zone });
                self.reset_q(ctx);
                return;
            }
        }
        // nothing fitting found (very unlikely): a safe default question
        let am = Chord::new(Note::parse("A").unwrap(), ChordType::by_id("min").unwrap());
        self.read_q = Some(ReadQ { chord: am, staff: false, zone: (0, 4) });
        self.reset_q(ctx);
    }

    /// Drill options; folded away on short windows so the task keeps the space.
    fn options(&mut self, ui: &mut Ui, ctx: &Ctx, with_answer_mode: bool) -> bool {
        let mut changed = false;
        let mut body = |this: &mut Self, ui: &mut Ui| {
            changed |= this.drill_settings(ui, ctx);
            if with_answer_mode {
                changed |= answer_mode_picker(ui, &mut this.answer);
            }
        };
        if ui.available_height() < 600.0 {
            egui::CollapsingHeader::new(RichText::new("⚙ Drill options").size(16.0))
                .id_salt("ch_drill_opts")
                .show(ui, |ui| body(self, ui));
        } else {
            body(self, ui);
        }
        changed
    }

    fn sight_read(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        if self.options(ui, ctx, true) {
            self.read_q = None;
        }
        ui.separator();
        if self.read_q.is_none() {
            self.new_read_q(ctx);
        }
        let Some(q) = &self.read_q else {
            ui.label("No chord found for this selection.");
            return;
        };
        let (chord, staff, zone) = (q.chord, q.staff, q.zone);
        let t = ctx.tuning();
        let key = format!("ch:read:{}:{}", chord.root_pc(), chord.ty.id);

        if staff {
            prompt(ui, &format!("Play this chord between fret {} and {}:", zone.0, zone.1));
            let cols = [Column {
                pitches: Self::close_position(&chord),
                tab: Vec::new(),
                label: if self.solved || self.revealed { chord.symbol(ctx.settings.naming) } else { String::new() },
            }];
            let mut nv = NotationView::new(&cols);
            nv.tab = false;
            nv.staff_space = STAFF_DRILL;
            nv.show(ui);
        } else {
            ui.horizontal(|ui| {
                ui.label(RichText::new(chord.symbol(ctx.settings.naming)).size(44.0).strong());
                ui.label(RichText::new(format!("   between fret {} and {}", zone.0, zone.1)).size(22.0));
            });
        }
        hint(ui, "Root in the bass. Think: where is the root inside the zone? Then add the other chord tones on the next strings.");

        let show_answers = self.solved || self.revealed;
        let mut dots: Vec<Dot> = Vec::new();
        for s in 0..6u8 {
            if let Some(f) = self.clicked[s as usize] {
                let p = Pos::new(s, f);
                let pc = pc_at(t, p);
                if show_answers {
                    dots.push(ctx.interval_dot(p, chord.root, chord.interval_of(pc), chord.spell(pc)));
                } else if self.feedback.as_ref().is_some_and(|f| !f.ok) && chord.interval_of(pc).is_none() {
                    // after a failed check: point at the strings with a wrong note
                    dots.push(Dot::new(p, "✖", crate::theme::ERROR).hollow());
                } else {
                    dots.push(Dot::new(p, "", MARKER));
                }
            }
        }
        let fitting: Vec<Voicing> = self
            .voicings(t, &chord)
            .iter()
            .filter(|v| fits_zone(v, zone) && v.bass_pos().map(|p| pc_at(t, p)) == Some(chord.root_pc()))
            .take(6)
            .cloned()
            .collect();
        if self.revealed
            && let Some(v) = fitting.first()
        {
            dots = v
                .played()
                .map(|p| {
                    let pc = pc_at(t, p);
                    ctx.interval_dot(p, chord.root, chord.interval_of(pc), chord.spell(pc))
                })
                .collect();
        }
        let mut board = ctx.board(&dots);
        board.zone = Some(zone);
        board.clickable = self.answer == AnswerMode::OnScreen && !show_answers;
        board.flash = flash(&self.feedback);
        if let Some(p) = board.show(ui) {
            let s = p.string as usize;
            self.clicked[s] = if self.clicked[s] == Some(p.fret) { None } else { Some(p.fret) };
            ctx.play_note(midi_at(t, p));
        }

        if self.answer == AnswerMode::OnScreen {
            if !show_answers {
                hint(ui, "Click one note per string (click again to remove). Strings you don't click are muted.");
                ui.horizontal(|ui| {
                    let check = primary_button(ui, "✔ Check (Enter)").clicked() || key_once(ui, egui::Key::Enter);
                    if ui.button("Clear").clicked() {
                        self.clicked = [None; 6];
                    }
                    if ui.button("Show me").clicked() {
                        self.revealed = true;
                        self.wrong_once = true;
                        ctx.progress.record(&key, false, None);
                    }
                    if check {
                        let mut problems = voicing::check(t, &chord, &self.clicked, false);
                        if self.clicked.iter().flatten().any(|&f| !(f == 0 && zone.0 <= 1) && (f < zone.0.max(1) || f > zone.1)) {
                            problems.push("some notes are outside the zone".into());
                        }
                        let span_ok = voicing::from_frets(self.clicked, Source::Generated)
                            .map(|v| v.max_fret() - v.min_fret() <= 4)
                            .unwrap_or(false);
                        if !span_ok && problems.is_empty() {
                            problems.push("not playable with four fingers".into());
                        }
                        if self.clicked.iter().flatten().count() < 3 && !chord.ty.is_power_chord() {
                            problems.push("use at least 3 strings".into());
                        }
                        if problems.is_empty() {
                            self.solved = true;
                            let ms = ctx.ms_since(self.started);
                            ctx.progress.record(&key, !self.wrong_once, Some(ms));
                            let midis: Vec<i32> = (0..6)
                                .filter_map(|s| self.clicked[s].map(|f| midi_at(t, Pos::new(s as u8, f))))
                                .collect();
                            ctx.play_chord(&midis);
                            self.feedback = Some(Feedback {
                                text: format!("✔ correct  ({:.1} s)", ms / 1000.0),
                                ok: true,
                            });
                        } else {
                            self.wrong_once = true;
                            self.feedback = Some(Feedback {
                                text: format!("✖ {}", problems.join("; ")),
                                ok: false,
                            });
                        }
                    }
                });
            } else if next_pressed(ui) {
                self.new_read_q(ctx);
                return;
            }
        } else if !self.revealed {
            hint(ui, "Play it on your guitar, then reveal.");
            if reveal_button(ui) {
                self.revealed = true;
                if let Some(v) = fitting.first() {
                    ctx.play_chord(&v.midi_notes(t));
                }
            }
        } else if let Some(ok) = self_rate(ui) {
            ctx.progress.record(&key, ok, Some(ctx.ms_since(self.started)));
            self.new_read_q(ctx);
            return;
        }
        show_feedback(ui, &self.feedback);

        if show_answers {
            ui.label(RichText::new(format!(
                "{} = {}   ·   possible voicings in this zone:",
                chord.symbol(ctx.settings.naming),
                chord.notes().iter().map(|n| ctx.name(*n)).collect::<Vec<_>>().join(" ")
            )).size(16.0));
            ui.horizontal_wrapped(|ui| {
                for v in &fitting {
                    let (labels, colors) = self.diagram_labels(ctx, &chord, v, false);
                    let r = ChordDiagram {
                        voicing: v,
                        labels,
                        colors,
                        width: 112.0,
                        title: None,
                        selected: false,
                        subtitle: Some(v.tab_string()),
                    }
                    .show(ui);
                    if r.clicked() {
                        ctx.play_chord(&v.midi_notes(t));
                    }
                }
            });
        }
        stats_line(ui, ctx, &key);
    }

    fn new_name_q(&mut self, ctx: &mut Ctx) {
        let t = ctx.tuning();
        let chords = self.pool_chords();
        let keys: Vec<String> = chords.iter().map(|c| format!("ch:name:{}:{}", c.root_pc(), c.ty.id)).collect();
        let c = chords[ctx.progress.pick(ctx.rng, &keys)];
        let voicing = if ctx.rng.below(3) > 0 {
            let vs: Vec<Voicing> = self
                .voicings(t, &c)
                .iter()
                .filter(|v| v.bass_pos().map(|p| pc_at(t, p)) == Some(c.root_pc()))
                .take(8)
                .cloned()
                .collect();
            ctx.rng.pick(&vs).cloned()
        } else {
            None
        };
        self.name_q = Some(NameQ {
            chord: c,
            voicing,
            root_answer: None,
            type_answer: None,
        });
        self.reset_q(ctx);
    }

    fn pool_types(&self) -> Vec<&'static ChordType> {
        CHORD_TYPES
            .iter()
            .filter(|t| self.pool[ChordCategory::ALL.iter().position(|c| *c == t.category).unwrap()])
            .collect()
    }

    fn name_it(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        if self.options(ui, ctx, false) {
            self.name_q = None;
        }
        ui.separator();
        if self.name_q.is_none() {
            self.new_name_q(ctx);
        }
        if let Some(at) = self.advance_at
            && ctx.time >= at
        {
            self.new_name_q(ctx);
        }
        let t = ctx.tuning();
        let q = self.name_q.as_ref().unwrap();
        let chord = q.chord;
        let key = format!("ch:name:{}:{}", chord.root_pc(), chord.ty.id);
        prompt(ui, "Which chord is this?");
        if let Some(v) = &q.voicing {
            let v = v.clone();
            let mut labels: [String; 6] = Default::default();
            if self.solved {
                labels = self.diagram_labels(ctx, &chord, &v, false).0;
            }
            ui.horizontal(|ui| {
                ChordDiagram {
                    voicing: &v,
                    labels,
                    colors: [Color32::from_rgb(200, 200, 210); 6],
                    width: 150.0,
                    title: None,
                    selected: false,
                    subtitle: Some(v.tab_string()),
                }
                .show(ui);
                if ui.button(RichText::new("▶ hear it").size(16.0)).clicked() {
                    ctx.play_chord(&v.midi_notes(t));
                }
            });
        } else {
            let cols = [Column {
                pitches: Self::close_position(&chord),
                tab: Vec::new(),
                label: String::new(),
            }];
            let mut nv = NotationView::new(&cols);
            nv.tab = false;
            nv.staff_space = STAFF_DRILL;
            nv.show(ui);
        }
        let enabled = self.advance_at.is_none();
        ui.label(RichText::new("Root:").size(17.0).strong());
        if let Some(pc) = note_picker(ui, ctx.settings.naming, self.marks, enabled) {
            self.marks.wrong = None;
            self.name_q.as_mut().unwrap().root_answer = Some(pc);
        }
        let types = self.pool_types();
        let labels: Vec<String> = types.iter().map(|t| format!("{} ({})", if t.suffix.is_empty() { "maj" } else { t.suffix }, t.name)).collect();
        ui.label(RichText::new("Type:").size(17.0).strong());
        let q = self.name_q.as_ref().unwrap();
        let sel_type = q.type_answer;
        if let Some(i) = choice_buttons(ui, &labels, if self.solved { sel_type } else { None }, self.wrong_type, enabled) {
            self.wrong_type = None;
            self.name_q.as_mut().unwrap().type_answer = Some(i);
        }
        let q = self.name_q.as_ref().unwrap();
        if let (Some(r), Some(ti)) = (q.root_answer, q.type_answer) {
            ui.label(RichText::new(format!("Your answer: {}{}", ctx.pc_name(r), types[ti].suffix)).size(16.0));
            if enabled && !self.solved {
                let guess = Chord::new(Note::from_pc(r), types[ti]);
                let mut a = guess.pcs();
                let mut b = chord.pcs();
                a.sort();
                a.dedup();
                b.sort();
                b.dedup();
                let symmetric = matches!(chord.ty.id, "dim7" | "aug");
                let ok = a == b && (r == chord.root_pc() || symmetric);
                if ok {
                    self.solved = true;
                    let ms = ctx.ms_since(self.started);
                    ctx.progress.record(&key, !self.wrong_once, Some(ms));
                    self.marks.correct = Some(r);
                    self.feedback = Some(Feedback {
                        text: format!("✔ {}", chord.symbol(ctx.settings.naming)),
                        ok: true,
                    });
                    if let Some(v) = &q.voicing {
                        ctx.play_chord(&v.midi_notes(t));
                    }
                    self.advance_at = Some(ctx.time + 1.8);
                } else if self.wrong_once {
                    // second miss: show the answer
                    ctx.progress.record(&key, false, None);
                    self.solved = true;
                    if r != chord.root_pc() {
                        self.marks.wrong = Some(r);
                    }
                    self.marks.correct = Some(chord.root_pc());
                    let right = types.iter().position(|t| t.id == chord.ty.id);
                    if types[ti].id != chord.ty.id {
                        self.wrong_type = Some(ti);
                    }
                    self.name_q.as_mut().unwrap().type_answer = right;
                    self.feedback = Some(Feedback {
                        text: format!("✖ It is {} – listen to it once more", chord.symbol(ctx.settings.naming)),
                        ok: false,
                    });
                    if let Some(v) = &self.name_q.as_ref().unwrap().voicing {
                        ctx.play_chord(&v.midi_notes(t));
                    }
                    self.advance_at = Some(ctx.time + 3.5);
                } else {
                    self.wrong_once = true;
                    let root_ok = r == chord.root_pc();
                    let type_ok = types[ti].id == chord.ty.id;
                    if !root_ok {
                        self.marks.wrong = Some(r);
                    }
                    if !type_ok {
                        self.wrong_type = Some(ti);
                    }
                    let qm = self.name_q.as_mut().unwrap();
                    qm.root_answer = None;
                    qm.type_answer = None;
                    let what = match (root_ok, type_ok) {
                        (true, false) => "root is right, the type is wrong",
                        (false, true) => "type is right, the root is wrong",
                        _ => "not quite",
                    };
                    self.feedback = Some(Feedback {
                        text: format!("✖ {what} – try again"),
                        ok: false,
                    });
                }
            }
        }
        show_feedback(ui, &self.feedback);
        if self.advance_at.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        stats_line(ui, ctx, &key);
    }
}
