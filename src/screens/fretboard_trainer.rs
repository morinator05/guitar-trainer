//! Fretboard trainer: learn where every note is, and that every note exists
//! in several places (octave shapes).

use super::common::*;
use crate::theory::fretboard::{Pos, midi_at, octave_pairs, pc_at, positions_of_midi, positions_of_pc};
use crate::theory::note::{Letter, Note, Pc, Pitch, is_natural};
use crate::theory::tuning::{Tuning, TuningId};
use crate::widgets::fretboard::Dot;
use crate::widgets::notation::{Column, NotationView};
use crate::widgets::{Marks, heading, hint, note_picker};
use egui::{Color32, RichText, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Explore,
    NameNote,
    FindNote,
    StaffToFret,
}

struct Level {
    name: &'static str,
    strings: &'static [u8],
    naturals: bool,
    max_fret: Option<u8>,
}

const LEVELS: [Level; 4] = [
    Level {
        name: "1 · Strings 6 + 5, natural notes, frets 0–12",
        strings: &[0, 1],
        naturals: true,
        max_fret: Some(12),
    },
    Level {
        name: "2 · All strings, natural notes, frets 0–12",
        strings: &[0, 1, 2, 3, 4, 5],
        naturals: true,
        max_fret: Some(12),
    },
    Level {
        name: "3 · All strings, all notes, frets 0–12",
        strings: &[0, 1, 2, 3, 4, 5],
        naturals: false,
        max_fret: Some(12),
    },
    Level {
        name: "4 · Whole neck, all notes",
        strings: &[0, 1, 2, 3, 4, 5],
        naturals: false,
        max_fret: None,
    },
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum FindVariant {
    OnString,
    AllInZone,
}

pub struct FretboardTrainer {
    pub mode: Mode,
    level: usize,
    answer: AnswerMode,
    // explore
    explore_pc: Option<Pc>,
    show_all: bool,
    naturals_only: bool,
    octave_lines: bool,
    heatmap: bool,
    // shared question state
    started: f64,
    wrong_once: bool,
    feedback: Option<Feedback>,
    advance_at: Option<f64>,
    revealed: bool,
    // name note
    q_pos: Option<Pos>,
    marks: Marks,
    // find note
    variant: FindVariant,
    q_pc: Pc,
    q_string: u8,
    zone: (u8, u8),
    targets: Vec<Pos>,
    found: Vec<Pos>,
    wrong_clicks: Vec<(Pos, f64)>,
    // staff
    q_pitch: Option<Pitch>,
}

impl Default for FretboardTrainer {
    fn default() -> Self {
        FretboardTrainer {
            mode: Mode::Explore,
            level: 0,
            answer: AnswerMode::OnScreen,
            explore_pc: Some(0),
            show_all: true,
            naturals_only: false,
            octave_lines: true,
            heatmap: false,
            started: 0.0,
            wrong_once: false,
            feedback: None,
            advance_at: None,
            revealed: false,
            q_pos: None,
            marks: Marks::default(),
            variant: FindVariant::OnString,
            q_pc: 0,
            q_string: 0,
            zone: (0, 4),
            targets: Vec::new(),
            found: Vec::new(),
            wrong_clicks: Vec::new(),
            q_pitch: None,
        }
    }
}

impl FretboardTrainer {
    pub fn set_mode(&mut self, m: Mode) {
        self.mode = m;
        self.q_pos = None;
        self.q_pitch = None;
        self.targets.clear();
    }

    fn max_fret(&self, ctx: &Ctx) -> u8 {
        LEVELS[self.level].max_fret.unwrap_or(ctx.settings.frets).min(ctx.settings.frets)
    }

    fn reset_question(&mut self, ctx: &Ctx) {
        self.started = ctx.time;
        self.wrong_once = false;
        self.feedback = None;
        self.advance_at = None;
        self.revealed = false;
        self.marks = Marks::default();
        self.found.clear();
        self.wrong_clicks.clear();
    }

    pub fn ui(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        ui.horizontal(|ui| {
            for (m, label) in [
                (Mode::Explore, "🔍 Explore"),
                (Mode::NameNote, "❓ Name the note"),
                (Mode::FindNote, "🎯 Find the note"),
                (Mode::StaffToFret, "🎼 Staff → fret"),
            ] {
                if ui.selectable_label(self.mode == m, RichText::new(label).size(17.0)).clicked() {
                    self.set_mode(m);
                }
            }
        });
        ui.separator();
        if self.mode != Mode::Explore {
            ui.horizontal(|ui| {
                ui.label("Level:");
                let before = self.level;
                egui::ComboBox::from_id_salt("fb_level")
                    .selected_text(LEVELS[self.level].name)
                    .width(360.0)
                    .show_ui(ui, |ui| {
                        for (i, l) in LEVELS.iter().enumerate() {
                            ui.selectable_value(&mut self.level, i, l.name);
                        }
                    });
                if before != self.level {
                    self.q_pos = None;
                    self.q_pitch = None;
                    self.targets.clear();
                }
            });
        }
        match self.mode {
            Mode::Explore => self.explore(ui, ctx),
            Mode::NameNote => self.name_note(ui, ctx),
            Mode::FindNote => self.find_note(ui, ctx),
            Mode::StaffToFret => self.staff_to_fret(ui, ctx),
        }
    }

    // ------------------------------------------------------------------ explore
    fn explore(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let t = ctx.tuning();
        ui.horizontal_wrapped(|ui| {
            ui.label("Highlight note:");
            for pc in 0..12u8 {
                let sel = self.explore_pc == Some(pc);
                if ui
                    .selectable_label(sel, RichText::new(ctx.pc_name(pc)).size(17.0))
                    .clicked()
                {
                    self.explore_pc = if sel { None } else { Some(pc) };
                }
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_all, "show all other notes");
            ui.checkbox(&mut self.naturals_only, "naturals only");
            ui.checkbox(&mut self.octave_lines, "octave shapes");
            ui.checkbox(&mut self.heatmap, "my weak spots (heatmap)");
        });

        let mut dots = Vec::new();
        let max = ctx.settings.frets;
        for s in 0..6u8 {
            for f in 0..=max {
                let p = Pos::new(s, f);
                let pc = pc_at(t, p);
                let is_q = self.explore_pc == Some(pc);
                if is_q {
                    dots.push(Dot::new(p, ctx.pc_name(pc), RED));
                } else if self.show_all && (!self.naturals_only || is_natural(pc)) {
                    let c = if is_natural(pc) {
                        Color32::from_rgb(200, 200, 205)
                    } else {
                        Color32::from_rgb(110, 110, 120)
                    };
                    dots.push(Dot::new(p, ctx.pc_name(pc), c));
                }
            }
        }
        let mut lines = Vec::new();
        if let (Some(pc), true) = (self.explore_pc, self.octave_lines) {
            let pos = positions_of_pc(t, pc, 0, max);
            for (a, b) in octave_pairs(t, &pos) {
                lines.push((a, b, Color32::from_rgba_unmultiplied(220, 50, 47, 150)));
            }
        }
        let progress = &*ctx.progress;
        let tint = |p: Pos| -> Option<Color32> {
            let it = progress.get(&name_key(p))?;
            let a = it.accuracy();
            Some(Color32::from_rgba_unmultiplied(
                (255.0 * (1.0 - a)) as u8,
                (200.0 * a) as u8,
                60,
                110,
            ))
        };
        let mut board = ctx.board(&dots);
        board.lines = &lines;
        board.height = 240.0;
        board.clickable = true;
        if self.heatmap {
            board.tint = Some(&tint);
        }
        if let Some(p) = board.show(ui) {
            let m = midi_at(t, p);
            ctx.play_note(m);
            self.explore_pc = Some(pc_at(t, p));
        }

        ui.add_space(8.0);
        if let Some(pc) = self.explore_pc {
            let n = positions_of_pc(t, pc, 0, 12).len();
            ui.label(RichText::new(format!("{} appears {} times between fret 0 and 12.", ctx.pc_name(pc), n)).size(16.0));
        }
        ui.add_space(4.0);
        heading(ui, "How to find the copies of a note (octave shapes)");
        let drop = matches!(ctx.settings.tuning, TuningId::DropD | TuningId::DropC);
        let text = [
            "• Every note repeats 12 frets higher on the same string.",
            "• Root on string 6 or 5 → the octave is 2 strings higher and 2 frets higher.",
            "• Root on string 4 or 3 → 2 strings higher and 3 frets higher (crossing the B string costs one fret: G→B is a major 3rd, all other pairs are 4ths).",
            "• Same note on the next lower string: 5 frets higher (4 frets when going from B down to G).",
            "• Strings 6 and 1 have the same note names: learn string 6 and you know string 1.",
        ];
        for l in text {
            ui.label(RichText::new(l).size(15.0));
        }
        if drop {
            ui.label(
                RichText::new("• Drop tuning: string 6 is a whole step lower than string 1 – on string 6 every note sits 2 frets higher than on string 1. The other strings are unchanged.")
                    .size(15.0)
                    .color(Color32::from_rgb(240, 170, 60)),
            );
        }
        hint(ui, "Tip: click any position to hear it and select its note.");
    }

    // ------------------------------------------------------------------ name the note
    fn candidates_name(&self, ctx: &Ctx) -> Vec<Pos> {
        let t = ctx.tuning();
        let l = &LEVELS[self.level];
        let mut v = Vec::new();
        for &s in l.strings {
            for f in 0..=self.max_fret(ctx) {
                let p = Pos::new(s, f);
                if !l.naturals || is_natural(pc_at(t, p)) {
                    v.push(p);
                }
            }
        }
        v
    }

    fn new_name_question(&mut self, ctx: &mut Ctx) {
        let cands = self.candidates_name(ctx);
        let keys: Vec<String> = cands.iter().map(|p| name_key(*p)).collect();
        let mut i = ctx.progress.pick(ctx.rng, &keys);
        // avoid repeating the same position twice in a row
        if Some(cands[i]) == self.q_pos && cands.len() > 1 {
            i = (i + 1 + ctx.rng.below(cands.len() - 1)) % cands.len();
        }
        self.q_pos = Some(cands[i]);
        self.reset_question(ctx);
    }

    fn name_note(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        if self.q_pos.is_none() {
            self.new_name_question(ctx);
        }
        if let Some(t) = self.advance_at
            && ctx.time >= t
        {
            self.new_name_question(ctx);
        }
        let t = ctx.tuning();
        let q = self.q_pos.unwrap();
        let pc = pc_at(t, q);
        prompt(ui, &format!("Which note is this?  (string {}, fret {})", Tuning::string_number(q.string as usize), q.fret));
        hint(ui, "Try to answer without counting frets from the nut: use the dots, octave shapes and the notes you already know.");
        let mut dots = vec![Dot::new(q, if self.advance_at.is_some() { ctx.pc_name(pc) } else { "?".into() }, BLUE).emphasis()];
        if self.advance_at.is_some() {
            dots[0].fill = GREEN;
        }
        let mut board = ctx.board(&dots);
        board.zone = Some((0, self.max_fret(ctx)));
        board.show(ui);
        ui.add_space(6.0);
        let enabled = self.advance_at.is_none();
        if let Some(a) = note_picker(ui, ctx.settings.naming, self.marks, enabled) {
            ctx.play_note(midi_at(t, q));
            if a == pc {
                let ms = ctx.ms_since(self.started);
                ctx.progress.record(&name_key(q), !self.wrong_once, Some(ms));
                self.marks.correct = Some(pc);
                self.feedback = Some(Feedback {
                    text: format!("✔ {}  ({:.1} s)", ctx.pc_name(pc), ms / 1000.0),
                    ok: true,
                });
                self.advance_at = Some(ctx.time + if self.wrong_once { 1.6 } else { 0.7 });
            } else {
                self.wrong_once = true;
                self.marks.wrong = Some(a);
                self.feedback = Some(Feedback {
                    text: format!("✖ not {} – try again", ctx.pc_name(a)),
                    ok: false,
                });
            }
        }
        show_feedback(ui, &self.feedback);
        if self.advance_at.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        stats_line(ui, ctx, &name_key(q));
    }

    // ------------------------------------------------------------------ find the note
    fn new_find_question(&mut self, ctx: &mut Ctx) {
        let t = ctx.tuning();
        let l = &LEVELS[self.level];
        let max = self.max_fret(ctx);
        let pcs: Vec<Pc> = (0..12).filter(|&pc| !l.naturals || is_natural(pc)).collect();
        match self.variant {
            FindVariant::OnString => {
                let keys: Vec<(Pc, u8)> = pcs
                    .iter()
                    .flat_map(|&pc| l.strings.iter().map(move |&s| (pc, s)))
                    .collect();
                let names: Vec<String> = keys.iter().map(|(pc, s)| format!("fb:find:{pc}:s{s}")).collect();
                let i = ctx.progress.pick(ctx.rng, &names);
                (self.q_pc, self.q_string) = keys[i];
                self.zone = (0, max);
                self.targets = positions_of_pc(t, self.q_pc, 0, max)
                    .into_iter()
                    .filter(|p| p.string == self.q_string)
                    .collect();
            }
            FindVariant::AllInZone => {
                let names: Vec<String> = pcs.iter().map(|pc| format!("fb:zone:{pc}")).collect();
                let i = ctx.progress.pick(ctx.rng, &names);
                self.q_pc = pcs[i];
                let start = ctx.rng.range(0, (max as i32 - 4).max(0)) as u8;
                self.zone = (start, (start + 4).min(max));
                self.targets = positions_of_pc(t, self.q_pc, self.zone.0, self.zone.1)
                    .into_iter()
                    .filter(|p| l.strings.contains(&p.string))
                    .collect();
                if self.targets.is_empty() {
                    return self.new_find_question(ctx);
                }
            }
        }
        self.reset_question(ctx);
    }

    fn find_key(&self) -> String {
        match self.variant {
            FindVariant::OnString => format!("fb:find:{}:s{}", self.q_pc, self.q_string),
            FindVariant::AllInZone => format!("fb:zone:{}", self.q_pc),
        }
    }

    fn find_note(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        ui.horizontal(|ui| {
            let before = self.variant;
            ui.label("Task:");
            ui.selectable_value(&mut self.variant, FindVariant::OnString, "one string");
            ui.selectable_value(&mut self.variant, FindVariant::AllInZone, "all copies in a 5-fret zone");
            if before != self.variant {
                self.targets.clear();
            }
        });
        if answer_mode_picker(ui, &mut self.answer) {
            self.targets.clear();
        }
        if self.targets.is_empty() {
            self.new_find_question(ctx);
        }
        if let Some(t) = self.advance_at
            && ctx.time >= t
        {
            self.new_find_question(ctx);
        }
        let t = ctx.tuning();
        let name = ctx.pc_name(self.q_pc);
        let text = match self.variant {
            FindVariant::OnString => format!(
                "Find {name} on string {} ({})",
                Tuning::string_number(self.q_string as usize),
                ctx.string_names()[self.q_string as usize]
            ),
            FindVariant::AllInZone => format!(
                "Find every {name} between fret {} and {}  ({} to find)",
                self.zone.0,
                self.zone.1,
                self.targets.len()
            ),
        };
        prompt(ui, &text);
        let done = self.advance_at.is_some() || self.revealed;
        let dots = self.board_dots(ctx, done);
        let mut board = ctx.board(&dots);
        board.zone = Some(self.zone);
        board.clickable = self.answer == AnswerMode::OnScreen && !done;
        let clicked = board.show(ui);

        if self.answer == AnswerMode::OnScreen {
            if let Some(p) = clicked {
                self.on_target_click(ctx, p, &self.find_key());
            }
        } else if !self.revealed {
            hint(ui, "Play it on your guitar first, then reveal.");
            if reveal_button(ui) {
                self.revealed = true;
                if let Some(p) = self.targets.first() {
                    ctx.play_note(midi_at(t, *p));
                }
            }
        } else if let Some(ok) = self_rate(ui) {
            ctx.progress.record(&self.find_key(), ok, Some(ctx.ms_since(self.started)));
            self.new_find_question(ctx);
        }
        show_feedback(ui, &self.feedback);
        if self.advance_at.is_some() || !self.wrong_clicks.is_empty() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        stats_line(ui, ctx, &self.find_key());
    }

    /// Common click handling for "find all targets" style questions.
    fn on_target_click(&mut self, ctx: &mut Ctx, p: Pos, key: &str) {
        let t = ctx.tuning();
        if self.targets.contains(&p) {
            if !self.found.contains(&p) {
                self.found.push(p);
                ctx.play_note(midi_at(t, p));
            }
            if self.found.len() == self.targets.len() {
                let ms = ctx.ms_since(self.started);
                ctx.progress.record(key, !self.wrong_once, Some(ms));
                self.feedback = Some(Feedback {
                    text: if self.wrong_once {
                        "✔ all found (with mistakes)".into()
                    } else {
                        format!("✔ perfect  ({:.1} s)", ms / 1000.0)
                    },
                    ok: !self.wrong_once,
                });
                self.advance_at = Some(ctx.time + 1.2);
            } else {
                self.feedback = Some(Feedback {
                    text: format!("✔ {} of {}", self.found.len(), self.targets.len()),
                    ok: true,
                });
            }
        } else {
            self.wrong_once = true;
            self.wrong_clicks.push((p, ctx.time));
            let n = Note::from_pc(pc_at(t, p));
            self.feedback = Some(Feedback {
                text: format!("✖ that is {}", ctx.name(n)),
                ok: false,
            });
        }
    }

    fn board_dots(&mut self, ctx: &Ctx, done: bool) -> Vec<Dot> {
        let t = ctx.tuning();
        let mut dots = Vec::new();
        for p in &self.targets {
            let label = ctx.pc_name(pc_at(t, *p));
            if self.found.contains(p) {
                dots.push(Dot::new(*p, label, GREEN));
            } else if done {
                dots.push(Dot::new(*p, label, if self.revealed { BLUE } else { RED }));
            }
        }
        self.wrong_clicks.retain(|(_, at)| ctx.time - at < 1.0);
        for (p, _) in &self.wrong_clicks {
            dots.push(Dot::new(*p, ctx.pc_name(pc_at(t, *p)), RED).hollow());
        }
        dots
    }

    // ------------------------------------------------------------------ staff → fret
    fn new_staff_question(&mut self, ctx: &mut Ctx) {
        let t = ctx.tuning();
        let l = &LEVELS[self.level];
        let max = self.max_fret(ctx);
        let lo = l.strings.iter().map(|&s| t.open_midi(s as usize)).min().unwrap();
        let hi = l.strings.iter().map(|&s| t.open_midi(s as usize)).max().unwrap() + max as i32;
        let midis: Vec<i32> = (lo..=hi)
            .filter(|&m| !l.naturals || is_natural(m.rem_euclid(12) as Pc))
            .collect();
        let keys: Vec<String> = midis.iter().map(|m| format!("fb:staff:{m}")).collect();
        let m = midis[ctx.progress.pick(ctx.rng, &keys)];
        let pc = m.rem_euclid(12) as Pc;
        // accidentals: sometimes sharp, sometimes flat spelling
        let note = if is_natural(pc) {
            Note::from_pc(pc)
        } else if ctx.rng.below(2) == 0 {
            Note::with_letter(sharp_letter(pc), pc)
        } else {
            Note::with_letter(Letter::from_index(sharp_letter(pc).index() + 1), pc)
        };
        self.q_pitch = Some(Pitch::from_midi_spelled(m, note));
        self.targets = positions_of_midi(t, m, max)
            .into_iter()
            .filter(|p| l.strings.contains(&p.string))
            .collect();
        self.zone = (0, max);
        self.reset_question(ctx);
    }

    fn staff_to_fret(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        if answer_mode_picker(ui, &mut self.answer) {
            self.q_pitch = None;
        }
        if self.q_pitch.is_none() || self.targets.is_empty() {
            self.new_staff_question(ctx);
        }
        if let Some(t) = self.advance_at
            && ctx.time >= t
        {
            self.new_staff_question(ctx);
        }
        let p = self.q_pitch.unwrap();
        let done = self.advance_at.is_some() || self.revealed;
        prompt(
            ui,
            &format!(
                "Where can you play this note?  ({} place{})",
                self.targets.len(),
                if self.targets.len() == 1 { "" } else { "s" }
            ),
        );
        hint(ui, "Guitar music is written one octave higher than it sounds (treble clef with a small 8 below). Exact pitch counts: only one octave is correct.");
        let col = [Column {
            pitches: vec![p],
            tab: if done { self.targets.clone() } else { Vec::new() },
            label: if done { p.name(ctx.settings.naming) } else { String::new() },
        }];
        let mut nv = NotationView::new(&col);
        nv.staff_space = 11.0;
        nv.tab = done;
        nv.show(ui);

        let dots = self.board_dots(ctx, done);
        let mut board = ctx.board(&dots);
        board.zone = Some(self.zone);
        board.clickable = self.answer == AnswerMode::OnScreen && !done;
        let clicked = board.show(ui);
        let key = format!("fb:staff:{}", p.midi());
        if self.answer == AnswerMode::OnScreen {
            if let Some(c) = clicked {
                self.on_target_click(ctx, c, &key);
            }
        } else if !self.revealed {
            hint(ui, "Play it on your guitar first, then reveal.");
            if reveal_button(ui) {
                self.revealed = true;
                ctx.play_note(p.midi());
            }
        } else if let Some(ok) = self_rate(ui) {
            ctx.progress.record(&key, ok, Some(ctx.ms_since(self.started)));
            self.new_staff_question(ctx);
        }
        show_feedback(ui, &self.feedback);
        if self.advance_at.is_some() || !self.wrong_clicks.is_empty() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        stats_line(ui, ctx, &key);
    }
}

fn sharp_letter(pc: Pc) -> Letter {
    Letter::from_index([0, 0, 1, 1, 2, 3, 3, 4, 4, 5, 5, 6][pc as usize % 12])
}

pub fn name_key(p: Pos) -> String {
    format!("fb:name:s{}f{}", p.string, p.fret)
}

pub fn stats_line(ui: &mut Ui, ctx: &Ctx, key: &str) {
    if let Some(it) = ctx.progress.get(key) {
        hint(
            ui,
            &format!(
                "This question: {} of {} correct, box {}/5, avg {:.1} s",
                it.correct,
                it.attempts,
                it.level + 1,
                it.avg_ms / 1000.0
            ),
        );
    }
    let d = ctx.progress.today();
    hint(ui, &format!("Today: {} answers, {} correct", d.attempts, d.correct));
}
