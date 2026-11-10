//! Fretboard trainer: learn where every note is, and that every note exists
//! in several places (octave shapes).

use super::common::*;
use crate::theme;
use crate::theory::fretboard::{Pos, midi_at, octave_pairs, pc_at, positions_of_midi, positions_of_pc};
use crate::theory::note::{Letter, Note, Pc, Pitch, is_natural};
use crate::theory::tuning::{Tuning, TuningId};
use crate::widgets::fretboard::Dot;
use crate::widgets::notation::{Column, NotationView, STAFF_DRILL};
use crate::widgets::{Marks, heading, hint, note_picker, tab_bar};
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
    /// wrong answers on the current question; after 2 the answer is shown
    misses: u8,
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
    /// settings the current question was made for
    stamp: Option<Stamp>,
}

impl Default for FretboardTrainer {
    fn default() -> Self {
        FretboardTrainer {
            mode: Mode::Explore,
            level: 0,
            answer: AnswerMode::OnScreen,
            explore_pc: Some(0),
            show_all: false,
            naturals_only: false,
            octave_lines: true,
            heatmap: false,
            started: 0.0,
            wrong_once: false,
            misses: 0,
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
            stamp: None,
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

    /// Between questions (nothing half-answered on screen)?
    pub fn is_idle(&self) -> bool {
        self.advance_at.is_none() && !self.revealed
    }

    fn max_fret(&self, ctx: &Ctx) -> u8 {
        LEVELS[self.level].max_fret.unwrap_or(ctx.settings.frets).min(ctx.settings.frets)
    }

    fn reset_question(&mut self, ctx: &Ctx) {
        self.stamp = Some(ctx.stamp());
        self.started = ctx.time;
        self.wrong_once = false;
        self.misses = 0;
        self.feedback = None;
        self.advance_at = None;
        self.revealed = false;
        self.marks = Marks::default();
        self.found.clear();
        self.wrong_clicks.clear();
    }

    pub fn ui(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let mut m = self.mode;
        if tab_bar(
            ui,
            &mut m,
            &[
                (Mode::Explore, "🔍 Explore"),
                (Mode::NameNote, "❓ Name the note"),
                (Mode::FindNote, "🎯 Find the note"),
                (Mode::StaffToFret, "🎼 Staff → fret"),
            ],
        ) {
            self.set_mode(m);
        }
        // tuning or fret count changed since the question was made → new question
        if self.stamp.is_some() && self.stamp != Some(ctx.stamp()) {
            self.q_pos = None;
            self.q_pitch = None;
            self.targets.clear();
            self.stamp = None;
        }
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
                    dots.push(Dot::new(p, ctx.pc_name(pc), RED).root().emphasis());
                } else if self.show_all && (!self.naturals_only || is_natural(pc)) {
                    // context stays recessive: naturals light discs, sharps/flats outlined
                    let d = Dot::new(p, ctx.pc_name(pc), Color32::from_rgb(200, 200, 205));
                    dots.push(if is_natural(pc) { d } else { d.hollow() });
                }
            }
        }
        let mut lines = Vec::new();
        if let (Some(pc), true) = (self.explore_pc, self.octave_lines) {
            let pos = positions_of_pc(t, pc, 0, max);
            for (a, b) in octave_pairs(t, &pos) {
                lines.push((a, b, Color32::from_rgba_unmultiplied(242, 236, 232, 210)));
            }
        }
        let progress = &*ctx.progress;
        let tuning_id = ctx.settings.tuning;
        let tint = |p: Pos| -> Option<Color32> {
            let it = progress.get(&name_key(tuning_id, p))?;
            let a = it.accuracy();
            Some(heat(a))
        };
        let mut board = ctx.board(&dots);
        board.lines = &lines;
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
        heading(ui, &format!("How to find the copies of a note in {}", ctx.tuning_name()));
        for l in octave_rules(t, &ctx.string_names()) {
            ui.label(RichText::new(format!("• {l}")).size(16.0));
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
        let tid = ctx.settings.tuning;
        let keys: Vec<String> = cands.iter().map(|p| name_key(tid, *p)).collect();
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
        prompt(
            ui,
            &format!(
                "Which note is this?  (string {} – {}, fret {})",
                Tuning::string_number(q.string as usize),
                ctx.string_names()[q.string as usize],
                q.fret
            ),
        );
        hint(ui, "Try to answer without counting frets from the nut: use the dots, octave shapes and the notes you already know.");
        let dots = if self.advance_at.is_some() {
            vec![Dot::new(q, ctx.pc_name(pc), GREEN).emphasis()]
        } else {
            vec![Dot::question(q)]
        };
        let mut board = ctx.board(&dots);
        // only the level's frets: fewer, bigger frets to read
        board.max_fret = self.max_fret(ctx);
        board.zone = Some((0, self.max_fret(ctx)));
        board.flash = flash(&self.feedback);
        board.show(ui);
        ui.add_space(6.0);
        let enabled = self.advance_at.is_none();
        if let Some(a) = note_picker(ui, ctx.settings.naming, self.marks, enabled) {
            ctx.play_note(midi_at(t, q));
            if a == pc {
                let ms = ctx.ms_since(self.started);
                ctx.progress.record(&name_key(ctx.settings.tuning, q), !self.wrong_once, Some(ms));
                self.marks.correct = Some(pc);
                self.feedback = Some(Feedback {
                    text: format!("✔ {}  ({:.1} s)", ctx.pc_name(pc), ms / 1000.0),
                    ok: true,
                });
                self.advance_at = Some(ctx.time + if self.wrong_once { 2.0 } else { 1.2 });
            } else {
                self.wrong_once = true;
                self.misses += 1;
                self.marks.wrong = Some(a);
                if self.misses >= 2 {
                    // don't make the user click through all 12: show the answer
                    ctx.progress.record(&name_key(ctx.settings.tuning, q), false, None);
                    self.marks.correct = Some(pc);
                    self.feedback = Some(Feedback {
                        text: format!("✖ It is {} – remember this one", ctx.pc_name(pc)),
                        ok: false,
                    });
                    self.advance_at = Some(ctx.time + 3.0);
                } else {
                    self.feedback = Some(Feedback {
                        text: format!("✖ not {} – try again", ctx.pc_name(a)),
                        ok: false,
                    });
                }
            }
        }
        show_feedback(ui, &self.feedback);
        if self.advance_at.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        stats_line(ui, ctx, &name_key(ctx.settings.tuning, q));
    }

    // ------------------------------------------------------------------ find the note
    fn new_find_question(&mut self, ctx: &mut Ctx) {
        let t = ctx.tuning();
        let l = &LEVELS[self.level];
        let max = self.max_fret(ctx);
        let pcs: Vec<Pc> = (0..12).filter(|&pc| !l.naturals || is_natural(pc)).collect();
        let tid = ctx.settings.tuning;
        match self.variant {
            FindVariant::OnString => {
                let keys: Vec<(Pc, u8)> = pcs
                    .iter()
                    .flat_map(|&pc| l.strings.iter().map(move |&s| (pc, s)))
                    .collect();
                let names: Vec<String> = keys.iter().map(|(pc, s)| format!("fb:find:{tid:?}:{pc}:s{s}")).collect();
                let i = ctx.progress.pick(ctx.rng, &names);
                (self.q_pc, self.q_string) = keys[i];
                self.zone = (0, max);
                self.targets = positions_of_pc(t, self.q_pc, 0, max)
                    .into_iter()
                    .filter(|p| p.string == self.q_string)
                    .collect();
            }
            FindVariant::AllInZone => {
                let names: Vec<String> = pcs.iter().map(|pc| format!("fb:zone:{tid:?}:{pc}")).collect();
                // a few tries to find a zone that contains the note (capped, no recursion)
                for _ in 0..50 {
                    let i = ctx.progress.pick(ctx.rng, &names);
                    self.q_pc = pcs[i];
                    let start = ctx.rng.range(0, (max as i32 - 4).max(0)) as u8;
                    self.zone = (start, (start + 4).min(max));
                    self.targets = positions_of_pc(t, self.q_pc, self.zone.0, self.zone.1)
                        .into_iter()
                        .filter(|p| l.strings.contains(&p.string))
                        .collect();
                    if !self.targets.is_empty() {
                        break;
                    }
                }
            }
        }
        self.reset_question(ctx);
    }

    fn find_key(&self) -> String {
        let tid = self.stamp.map(|s| s.tuning).unwrap_or_default();
        match self.variant {
            FindVariant::OnString => format!("fb:find:{tid:?}:{}:s{}", self.q_pc, self.q_string),
            FindVariant::AllInZone => format!("fb:zone:{tid:?}:{}", self.q_pc),
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
        board.max_fret = self.max_fret(ctx);
        board.zone = Some(self.zone);
        board.clickable = self.answer == AnswerMode::OnScreen && !done;
        board.flash = flash(&self.feedback);
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
                self.advance_at = Some(ctx.time + 1.5);
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
                text: format!(
                    "✖ that is {} (string {}, fret {})",
                    ctx.name(n),
                    Tuning::string_number(p.string as usize),
                    p.fret
                ),
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
                dots.push(Dot::new(*p, label, GREEN).emphasis());
            } else if done {
                // missed targets: outlined (found ones are filled)
                dots.push(if self.revealed { Dot::new(*p, label, MARKER) } else { Dot::new(*p, label, theme::ERROR).hollow() });
            }
        }
        self.wrong_clicks.retain(|(_, at)| ctx.time - at < 1.6);
        for (p, _) in &self.wrong_clicks {
            dots.push(Dot::new(*p, "✖", theme::ERROR).hollow());
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
        let tid = ctx.settings.tuning;
        let keys: Vec<String> = midis.iter().map(|m| format!("fb:staff:{tid:?}:{m}")).collect();
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
        nv.staff_space = STAFF_DRILL;
        nv.tab = done;
        nv.show(ui);

        let dots = self.board_dots(ctx, done);
        let mut board = ctx.board(&dots);
        board.max_fret = self.max_fret(ctx);
        board.zone = Some(self.zone);
        board.clickable = self.answer == AnswerMode::OnScreen && !done;
        board.flash = flash(&self.feedback);
        let clicked = board.show(ui);
        let key = format!("fb:staff:{:?}:{}", ctx.settings.tuning, p.midi());
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

pub fn name_key(t: TuningId, p: Pos) -> String {
    format!("fb:name:{t:?}:s{}f{}", p.string, p.fret)
}

pub fn stats_line(ui: &mut Ui, ctx: &Ctx, key: &str) {
    if let Some(it) = ctx.progress.get(key) {
        hint(
            ui,
            &format!(
                "This question: {} of {} correct, memory level {}/5, avg {:.1} s",
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

/// Octave-shape rules worked out from the actual tuning (no E-standard assumptions).
pub fn octave_rules(t: &Tuning, names: &[String; 6]) -> Vec<String> {
    let open: Vec<i32> = (0..6).map(|s| t.open_midi(s)).collect();
    let num = |s: usize| Tuning::string_number(s);
    let frets = |n: i32| match n {
        0 => "the same fret".to_string(),
        1 => "1 fret higher".to_string(),
        n => format!("{n} frets higher"),
    };
    let mut out = vec!["Every note repeats 12 frets higher on the same string.".to_string()];
    // octave two strings up, grouped by fret offset
    let mut groups: Vec<(i32, Vec<usize>)> = Vec::new();
    for s in 0..4 {
        let off = 12 - (open[s + 2] - open[s]);
        match groups.last_mut() {
            Some((o, v)) if *o == off => v.push(s),
            _ => groups.push((off, vec![s])),
        }
    }
    for (off, ss) in groups {
        let list: Vec<String> = ss.iter().map(|&s| format!("{} ({})", num(s), names[s])).collect();
        out.push(format!(
            "Root on string {} → the octave is 2 strings higher and {}.",
            list.join(" or "),
            frets(off)
        ));
    }
    // same note one string thinner
    let mut odd = Vec::new();
    for s in 0..5 {
        let d = open[s + 1] - open[s];
        if d != 5 {
            odd.push(format!("{} → {} ({}→{}): {d} frets", num(s), num(s + 1), names[s], names[s + 1]));
        }
    }
    let mut same = "Same note one string thinner (e.g. string 6 → 5): 5 frets lower.".to_string();
    if !odd.is_empty() {
        same += &format!(" Exceptions: {}.", odd.join(", "));
    }
    out.push(same);
    let d61 = open[5] - open[0];
    if d61 % 12 == 0 {
        out.push(format!(
            "Strings 6 and 1 are both {}: learn string 6 and you know string 1.",
            names[0]
        ));
    } else {
        let diff = d61.rem_euclid(12);
        let (dir, n) = if diff <= 6 { ("higher", diff) } else { ("lower", 12 - diff) };
        out.push(format!(
            "String 6 ({}) and string 1 ({}) differ: on string 6 every note sits {n} frets {dir} than on string 1.",
            names[0], names[5]
        ));
    }
    out
}

/// Heatmap tint: one hue from dark indigo (weak) to bright yellow (known), so it
/// reads by lightness alone (works with colour blindness).
pub fn heat(accuracy: f32) -> Color32 {
    let a = accuracy.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * a) as u8;
    Color32::from_rgba_unmultiplied(lerp(0x6A, 0xF2), lerp(0x5A, 0xD0), lerp(0xCD, 0x6B), 210)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::note::Naming;
    use crate::theory::tuning::*;

    fn names(t: &Tuning) -> [String; 6] {
        std::array::from_fn(|s| t.strings[s].note.name(Naming::English))
    }

    #[test]
    fn rules_e_standard() {
        let r = octave_rules(&E_STANDARD, &names(&E_STANDARD));
        assert!(r[1].contains("6 (E) or 5 (A)") && r[1].contains("2 frets higher"), "{r:?}");
        assert!(r[2].contains("4 (D) or 3 (G)") && r[2].contains("3 frets higher"), "{r:?}");
        assert!(r[3].contains("G→B): 4 frets"), "{r:?}");
        assert!(r[4].contains("both E"), "{r:?}");
    }

    #[test]
    fn rules_drop_c() {
        let r = octave_rules(&DROP_C, &names(&DROP_C));
        assert!(r[1].contains("6 (C)") && r[1].contains("the same fret"), "{r:?}");
        assert!(r.iter().any(|l| l.contains("C→G): 7 frets")), "{r:?}");
        assert!(r.last().unwrap().contains("2 frets higher"), "{r:?}");
    }
}
