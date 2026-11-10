//! Scales: neck maps, positions (boxes / 3NPS / windows), tab + notation,
//! playback, harmonised chords, and drills for improvising.

use super::common::*;
use super::fretboard_trainer::stats_line;
use crate::theory::chord::common_roots;
use crate::theory::fretboard::{Pos, midi_at, pc_at};
use crate::theme;
use crate::theory::interval::interval_color;
use crate::theory::note::{Note, Pc, Pitch};
use crate::theory::scale::{SCALE_TYPES, Scale, ScaleCategory, ScaleType};
use crate::theory::scale_position::{ScalePosition, full_neck, positions, window};
use crate::widgets::fretboard::{Dot, rgb};
use crate::widgets::notation::{Column, NotationView};
use crate::widgets::{choice_buttons, heading, hint, tab_bar};
use egui::{Color32, RichText, ScrollArea, Ui};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Library,
    FillBox,
    WhichDegree,
    PlayPosition,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Full,
    Pattern(usize),
    Window,
}

pub struct Scales {
    pub tab: Tab,
    root: Note,
    ty: &'static ScaleType,
    view: View,
    window_start: u8,
    sevenths: bool,
    metronome: bool,
    play_list: Vec<Pos>,
    play_step: f32,
    play_id: Option<u64>,
    stamp: Option<Stamp>,
    random_key: bool,
    show_roots: bool,
    // drill state
    q_ready: bool,
    q_scale: Option<Scale>,
    q_pattern: usize,
    q_pos: ScalePosition,
    q_note: Option<Pos>,
    found: Vec<Pos>,
    wrong_clicks: Vec<(Pos, f64)>,
    wrong_choice: Option<usize>,
    right_choice: Option<usize>,
    started: f64,
    wrong_once: bool,
    revealed: bool,
    feedback: Option<Feedback>,
    advance_at: Option<f64>,
}

impl Default for Scales {
    fn default() -> Self {
        Scales {
            tab: Tab::Library,
            root: Note::parse("A").unwrap(),
            ty: ScaleType::by_id("minor_pentatonic").unwrap(),
            view: View::Pattern(0),
            window_start: 5,
            sevenths: false,
            metronome: false,
            play_list: Vec::new(),
            play_step: 0.25,
            play_id: None,
            stamp: None,
            random_key: false,
            show_roots: true,
            q_ready: false,
            q_scale: None,
            q_pattern: 0,
            q_pos: ScalePosition { name: String::new(), notes: Vec::new() },
            q_note: None,
            found: Vec::new(),
            wrong_clicks: Vec::new(),
            wrong_choice: None,
            right_choice: None,
            started: 0.0,
            wrong_once: false,
            revealed: false,
            feedback: None,
            advance_at: None,
        }
    }
}

impl Scales {
    /// Show a scale in the library (used by links from other screens).
    pub fn open(&mut self, root: Note, id: &str) {
        if let Some(ty) = ScaleType::by_id(id) {
            self.root = root;
            self.ty = ty;
            self.view = View::Pattern(0);
            self.tab = Tab::Library;
            self.q_ready = false;
            self.play_list.clear();
            self.play_id = None;
        }
    }

    pub fn set_tab(&mut self, t: Tab) {
        self.tab = t;
        self.q_ready = false;
    }

    /// Between questions (nothing half-answered on screen)?
    pub fn is_idle(&self) -> bool {
        self.advance_at.is_none() && !self.revealed
    }

    /// Forget the playback highlight when the sequence ended or was replaced.
    fn update_playback(&mut self, ctx: &Ctx) {
        if let Some(id) = self.play_id {
            let ended = ctx
                .audio
                .seq_elapsed(id)
                .is_some_and(|e| (e / self.play_step) as usize >= self.play_list.len());
            if !ctx.audio.is_current(id) || ended {
                self.play_id = None;
                self.play_list.clear();
            }
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let mut tab = self.tab;
        if tab_bar(
            ui,
            &mut tab,
            &[
                (Tab::Library, "📚 Library"),
                (Tab::FillBox, "📝 Fill the box"),
                (Tab::WhichDegree, "❓ Which degree?"),
                (Tab::PlayPosition, "🎸 Play the position"),
            ],
        ) {
            self.set_tab(tab);
        }
        // tuning or fret count changed → regenerate the drill question
        if self.stamp.is_some() && self.stamp != Some(ctx.stamp()) {
            self.q_ready = false;
            self.q_note = None;
            self.stamp = None;
            self.play_list.clear();
            self.play_id = None;
        }
        self.update_playback(ctx);
        if self.scale_picker(ui, ctx) {
            self.q_ready = false;
        }
        ui.separator();
        match self.tab {
            Tab::Library => self.library(ui, ctx),
            Tab::FillBox => self.fill_box(ui, ctx),
            Tab::WhichDegree => self.which_degree(ui, ctx),
            Tab::PlayPosition => self.play_position(ui, ctx),
        }
    }

    fn scale(&self) -> Scale {
        Scale::new(self.root, self.ty)
    }

    fn scale_picker(&mut self, ui: &mut Ui, ctx: &Ctx) -> bool {
        let mut changed = false;
        if self.tab != Tab::Library {
            // drills: one compact row, so the neck and the answer buttons keep the space
            ui.horizontal(|ui| {
                ui.label("Key:");
                egui::ComboBox::from_id_salt("sc_key")
                    .selected_text(RichText::new(ctx.name(self.root)).size(16.0))
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for r in common_roots() {
                            changed |= ui.selectable_value(&mut self.root, r, ctx.name(r)).changed();
                        }
                    });
                ui.label("Scale:");
                egui::ComboBox::from_id_salt("sc_type")
                    .selected_text(RichText::new(self.ty.name).size(16.0))
                    .width(220.0)
                    .show_ui(ui, |ui| {
                        for t in SCALE_TYPES.iter() {
                            if ui.selectable_label(std::ptr::eq(self.ty, t), t.name).clicked() {
                                self.ty = t;
                                self.view = View::Pattern(0);
                                changed = true;
                            }
                        }
                    });
            });
            if changed {
                self.play_list.clear();
            }
            return changed;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Key:");
            for r in common_roots() {
                if ui.selectable_label(self.root == r, RichText::new(ctx.name(r)).size(16.0)).clicked() {
                    self.root = r;
                    changed = true;
                }
            }
        });
        for cat in ScaleCategory::ALL {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{}:", cat.label())).strong());
                for t in SCALE_TYPES.iter().filter(|t| t.category == cat) {
                    if ui.selectable_label(std::ptr::eq(self.ty, t), RichText::new(t.name).size(15.0)).clicked() {
                        self.ty = t;
                        self.view = View::Pattern(0);
                        changed = true;
                    }
                }
            });
        }
        if changed {
            self.play_list.clear();
        }
        changed
    }

    fn header(&self, ui: &mut Ui, ctx: &Ctx, s: &Scale) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("{} {}", ctx.name(s.root), s.ty.name)).size(28.0).strong());
        });
        ui.horizontal_wrapped(|ui| {
            for (i, n) in s.ty.intervals().iter().zip(s.notes()) {
                let c = rgb(interval_color(i.pc_offset()));
                ui.label(
                    RichText::new(format!(" {} = {} ", i.label(), ctx.name(n)))
                        .size(17.0)
                        .background_color(c)
                        .color(crate::widgets::fretboard::contrast_text(c)),
                );
            }
        });
        ui.label(RichText::new(s.ty.info).size(15.0));
    }

    fn dot(&self, ctx: &Ctx, s: &Scale, p: Pos) -> Dot {
        let pc = pc_at(ctx.tuning(), p);
        ctx.interval_dot(p, s.root, s.interval_of(pc), s.spell(pc))
    }

    fn columns(&self, ctx: &Ctx, s: &Scale, notes: &[Pos]) -> Vec<Column> {
        let t = ctx.tuning();
        notes
            .iter()
            .map(|p| {
                let pc = pc_at(t, *p);
                Column {
                    pitches: vec![Pitch::from_midi_spelled(midi_at(t, *p), s.spell(pc))],
                    tab: vec![*p],
                    label: s.interval_of(pc).map(|i| i.label()).unwrap_or_default(),
                }
            })
            .collect()
    }

    fn up_down(notes: &[Pos]) -> Vec<Pos> {
        let mut v = notes.to_vec();
        v.extend(notes.iter().rev().skip(1));
        v
    }

    fn playing_index(&self, ctx: &Ctx) -> Option<usize> {
        let e = ctx.audio.seq_elapsed(self.play_id?)?;
        let i = (e / self.play_step) as usize;
        (i < self.play_list.len()).then_some(i)
    }

    fn transport(&mut self, ui: &mut Ui, ctx: &mut Ctx, notes: &[Pos]) {
        ui.horizontal(|ui| {
            if ui.button(RichText::new("▶ play up & down").size(16.0)).clicked() && ctx.settings.sound {
                self.play_list = Self::up_down(notes);
                let t = ctx.tuning();
                let midis: Vec<i32> = self.play_list.iter().map(|p| midi_at(t, *p)).collect();
                self.play_step = 60.0 / ctx.settings.bpm / 2.0;
                self.play_id = Some(ctx.audio.sequence(&midis, ctx.settings.bpm, 2.0));
            }
            if ui.button("■ stop").clicked() {
                ctx.audio.stop();
                self.play_list.clear();
                self.play_id = None;
            }
            let bpm_changed = ui
                .add(egui::Slider::new(&mut ctx.settings.bpm, 40.0..=220.0).text("BPM (8th notes)"))
                .changed();
            let toggled = ui.checkbox(&mut self.metronome, "metronome").changed();
            if toggled || (bpm_changed && self.metronome) {
                ctx.audio.metronome(self.metronome.then_some(ctx.settings.bpm), 4);
            }
        });
    }

    // ------------------------------------------------------------------ library
    fn library(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        let s = self.scale();
        let t = ctx.tuning();
        let pats = positions(t, &s, ctx.settings.frets);
        ScrollArea::vertical().id_salt("scale_lib").show(ui, |ui| {
            self.header(ui, ctx, &s);
            let before = (self.view, self.window_start);
            ui.horizontal_wrapped(|ui| {
                ui.label("Show:");
                ui.selectable_value(&mut self.view, View::Full, "whole neck");
                for (i, p) in pats.iter().enumerate() {
                    ui.selectable_value(&mut self.view, View::Pattern(i), &p.name);
                }
                ui.selectable_value(&mut self.view, View::Window, "fret window");
                if self.view == View::Window {
                    ui.add(egui::Slider::new(&mut self.window_start, 0..=ctx.settings.frets.saturating_sub(4)).text("start fret"));
                }
            });
            if before != (self.view, self.window_start) && self.play_id.is_some() {
                // a different position is shown: stop the old playback and its tab
                ctx.audio.stop();
                self.play_id = None;
                self.play_list.clear();
            }
            let shown: ScalePosition = match self.view {
                View::Full => ScalePosition {
                    name: "whole neck".into(),
                    notes: full_neck(t, &s, ctx.settings.frets),
                },
                View::Pattern(i) => pats.get(i).cloned().unwrap_or_else(|| pats[0].clone()),
                View::Window => window(t, &s, self.window_start, 5),
            };

            let playing = self.playing_index(ctx).map(|i| self.play_list[i]);
            let mut dots: Vec<Dot> = Vec::new();
            if self.view != View::Full {
                for p in full_neck(t, &s, ctx.settings.frets) {
                    if !shown.notes.contains(&p) {
                        dots.push(Dot::new(p, "", Color32::from_white_alpha(60)).hollow());
                    }
                }
            }
            for p in &shown.notes {
                let mut d = self.dot(ctx, &s, *p);
                if Some(*p) == playing {
                    d = d.emphasis();
                }
                dots.push(d);
            }
            let mut board = ctx.board(&dots);
            board.clickable = true;
            if self.view != View::Full {
                board.zone = Some(shown.fret_range());
            }
            if let Some(p) = board.show(ui) {
                ctx.play_note(midi_at(t, p));
            }
            if self.playing_index(ctx).is_some() {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
            }

            if self.view != View::Full {
                self.transport(ui, ctx, &shown.notes);
                let list = if self.play_list.is_empty() { Self::up_down(&shown.notes) } else { self.play_list.clone() };
                let cols = self.columns(ctx, &s, &list);
                let mut nv = NotationView::new(&cols);
                nv.whole_notes = false;
                nv.staff_space = 10.0;
                nv.highlight = self.playing_index(ctx);
                ScrollArea::horizontal().id_salt("scale_notation").show(ui, |ui| nv.show(ui));
                hint(ui, "Tab and notation of this position, up and down. Labels under the tab are the scale degrees.");
            }

            // harmonised chords
            let chords = s.harmonise(self.sevenths);
            if !chords.is_empty() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    heading(ui, "Chords in this scale");
                    ui.checkbox(&mut self.sevenths, "7th chords");
                });
                ui.horizontal_wrapped(|ui| {
                    for c in chords.into_iter().flatten() {
                        let text = format!("{}\n{}{}", c.roman, ctx.name(c.root), c.ty.suffix);
                        if ui
                            .button(RichText::new(text).size(17.0))
                            .on_hover_text("open in the chord library")
                            .clicked()
                        {
                            *ctx.nav = Some(Nav::Chord { root: c.root, ty: c.ty.id });
                        }
                    }
                });
                hint(ui, "Improvising: over each of these chords the scale fits. Land on that chord's tones (shown in the chord library) on strong beats.");
            } else if s.ty.len() <= 6 {
                hint(ui, "Pentatonic / blues scales: use them over a 12-bar blues (I7 IV7 V7) or over a minor (i iv v) progression in the same key.");
            }
        });
    }

    // ------------------------------------------------------------------ drills
    fn pick_pattern(&mut self, ctx: &mut Ctx, prefix: &str) {
        if self.random_key {
            let roots = common_roots();
            self.root = *ctx.rng.pick(&roots).unwrap();
        }
        let s = self.scale();
        let pats = positions(ctx.tuning(), &s, ctx.settings.frets);
        let keys: Vec<String> = (0..pats.len())
            .map(|i| format!("{prefix}:{}:{}:{}", s.ty.id, s.root.pc(), i))
            .collect();
        self.q_pattern = ctx.progress.pick(ctx.rng, &keys);
        self.q_pos = pats[self.q_pattern].clone();
        self.q_scale = Some(s);
        self.found.clear();
        self.wrong_clicks.clear();
        self.wrong_once = false;
        self.revealed = false;
        self.feedback = None;
        self.advance_at = None;
        self.wrong_choice = None;
        self.right_choice = None;
        self.started = ctx.time;
        self.q_ready = true;
        self.stamp = Some(ctx.stamp());
    }

    fn drill_options(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.random_key, "random key").changed() {
                self.q_ready = false;
            }
            if self.tab != Tab::PlayPosition && ui.checkbox(&mut self.show_roots, "show roots").changed() {
                self.q_ready = false;
            }
        });
    }

    fn fill_box(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        self.drill_options(ui);
        if !self.q_ready {
            self.pick_pattern(ctx, "sc:fill");
        }
        if let Some(at) = self.advance_at
            && ctx.time >= at
        {
            self.pick_pattern(ctx, "sc:fill");
        }
        let s = self.q_scale.unwrap();
        let t = ctx.tuning();
        let key = format!("sc:fill:{}:{}:{}", s.ty.id, s.root.pc(), self.q_pattern);
        let targets: Vec<Pos> = self
            .q_pos
            .notes
            .iter()
            .copied()
            .filter(|p| !(self.show_roots && pc_at(t, *p) == s.root.pc()))
            .collect();
        prompt(
            ui,
            &format!(
                "{} {} – {}: click all scale notes in the zone ({}/{})",
                ctx.name(s.root),
                s.ty.name,
                self.q_pos.name,
                self.found.len(),
                targets.len()
            ),
        );
        let done = self.advance_at.is_some() || self.revealed;
        let mut dots = Vec::new();
        for p in &self.q_pos.notes {
            let is_root = pc_at(t, *p) == s.root.pc();
            if (self.show_roots && is_root) || self.found.contains(p) || done {
                dots.push(self.dot(ctx, &s, *p));
            }
        }
        self.wrong_clicks.retain(|(_, at)| ctx.time - at < 1.6);
        for (p, _) in &self.wrong_clicks {
            dots.push(Dot::new(*p, "✖", theme::ERROR).hollow());
        }
        let mut board = ctx.board(&dots);
        board.zone = Some(self.q_pos.fret_range());
        board.clickable = !done;
        board.flash = flash(&self.feedback);
        if let Some(p) = board.show(ui) {
            if targets.contains(&p) {
                if !self.found.contains(&p) {
                    self.found.push(p);
                    ctx.play_note(midi_at(t, p));
                }
                if self.found.len() == targets.len() {
                    let ms = ctx.ms_since(self.started);
                    ctx.progress.record(&key, !self.wrong_once, Some(ms));
                    self.feedback = Some(Feedback {
                        text: format!("✔ position complete ({:.1} s)", ms / 1000.0),
                        ok: true,
                    });
                    self.advance_at = Some(ctx.time + 1.5);
                }
            } else if s.contains(pc_at(t, p)) && !self.q_pos.notes.contains(&p) {
                // a real scale note, just outside this position: no penalty
                self.feedback = Some(Feedback {
                    text: format!("{} is in the scale, but outside this position", ctx.pc_name(pc_at(t, p))),
                    ok: true,
                });
            } else if !self.q_pos.notes.contains(&p) {
                self.wrong_once = true;
                self.wrong_clicks.push((p, ctx.time));
                self.feedback = Some(Feedback {
                    text: format!("✖ {} is not in the scale", ctx.pc_name(pc_at(t, p))),
                    ok: false,
                });
            }
        }
        if !done && ui.button("Show me").clicked() {
            self.revealed = true;
            ctx.progress.record(&key, false, None);
            self.advance_at = Some(ctx.time + 3.0);
        }
        show_feedback(ui, &self.feedback);
        if self.advance_at.is_some() || !self.wrong_clicks.is_empty() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        stats_line(ui, ctx, &key);
    }

    fn which_degree(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        self.drill_options(ui);
        let new_q = |this: &mut Scales, ctx: &mut Ctx| {
            let keep_pattern = this.q_ready && ctx.rng.below(4) != 0;
            if !keep_pattern {
                this.pick_pattern(ctx, "sc:degpos");
            }
            let s = this.q_scale.unwrap();
            let t = ctx.tuning();
            let notes: Vec<Pos> = this
                .q_pos
                .notes
                .iter()
                .copied()
                .filter(|p| !(this.show_roots && pc_at(t, *p) == s.root.pc()))
                .collect();
            let keys: Vec<String> = notes
                .iter()
                .map(|p| {
                    let d = s.interval_of(pc_at(t, *p)).map(|i| i.formula_name()).unwrap_or_default();
                    format!("sc:deg:{}:{}", s.ty.id, d)
                })
                .collect();
            let mut i = ctx.progress.pick(ctx.rng, &keys);
            if Some(notes[i]) == this.q_note && notes.len() > 1 {
                i = (i + 1) % notes.len();
            }
            this.q_note = Some(notes[i]);
            this.wrong_choice = None;
            this.right_choice = None;
            this.wrong_once = false;
            this.feedback = None;
            this.advance_at = None;
            this.started = ctx.time;
        };
        if !self.q_ready || self.q_note.is_none() {
            self.pick_pattern(ctx, "sc:degpos");
            new_q(self, ctx);
        }
        if let Some(at) = self.advance_at
            && ctx.time >= at
        {
            new_q(self, ctx);
        }
        let s = self.q_scale.unwrap();
        let t = ctx.tuning();
        let q = self.q_note.unwrap();
        let pc = pc_at(t, q);
        let Some(interval) = s.interval_of(pc) else {
            // stale question (should not happen thanks to the stamp check): start over
            self.q_ready = false;
            self.q_note = None;
            return;
        };
        let key = format!("sc:deg:{}:{}", s.ty.id, interval.formula_name());
        prompt(ui, &format!("{} {}: which scale degree is the \"?\" note?", ctx.name(s.root), s.ty.name));
        hint(ui, "Find the nearest root (square, 1) and count the interval from it: that's how you target chord tones while improvising.");
        let mut dots = Vec::new();
        for p in &self.q_pos.notes {
            let is_root = pc_at(t, *p) == s.root.pc();
            if *p == q {
                let mut d = if self.advance_at.is_some() { self.dot(ctx, &s, *p) } else { Dot::question(*p) };
                d = d.emphasis();
                dots.push(d);
            } else if self.show_roots && is_root {
                dots.push(Dot::new(*p, "1", RED).root());
            } else {
                dots.push(Dot::new(*p, "", Color32::from_gray(150)).hollow());
            }
        }
        let mut board = ctx.board(&dots);
        board.zone = Some(self.q_pos.fret_range());
        board.flash = flash(&self.feedback);
        board.show(ui);
        let ints = s.ty.intervals();
        let labels: Vec<String> = ints.iter().map(|i| i.label()).collect();
        let enabled = self.advance_at.is_none();
        if let Some(i) = choice_buttons(ui, &labels, self.right_choice, self.wrong_choice, enabled) {
            ctx.play_note(midi_at(t, q));
            if ints[i] == interval {
                let ms = ctx.ms_since(self.started);
                ctx.progress.record(&key, !self.wrong_once, Some(ms));
                self.right_choice = Some(i);
                self.feedback = Some(Feedback {
                    text: format!("✔ {} = {} ({})", interval.label(), ctx.name(s.spell(pc)), interval.long_name()),
                    ok: true,
                });
                self.advance_at = Some(ctx.time + if self.wrong_once { 2.0 } else { 1.3 });
            } else if self.wrong_once {
                // second miss: show the answer instead of letting the user guess on
                ctx.progress.record(&key, false, None);
                self.wrong_choice = Some(i);
                self.right_choice = ints.iter().position(|x| *x == interval);
                self.feedback = Some(Feedback {
                    text: format!("✖ It is {} = {} ({})", interval.label(), ctx.name(s.spell(pc)), interval.long_name()),
                    ok: false,
                });
                self.advance_at = Some(ctx.time + 3.0);
            } else {
                self.wrong_once = true;
                self.wrong_choice = Some(i);
                self.feedback = Some(Feedback {
                    text: format!("✖ not {} – try again", ints[i].label()),
                    ok: false,
                });
            }
        }
        show_feedback(ui, &self.feedback);
        if self.advance_at.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
        }
        stats_line(ui, ctx, &key);
    }

    fn play_position(&mut self, ui: &mut Ui, ctx: &mut Ctx) {
        self.drill_options(ui);
        if !self.q_ready {
            self.pick_pattern(ctx, "sc:play");
        }
        let s = self.q_scale.unwrap();
        let key = format!("sc:play:{}:{}:{}", s.ty.id, s.root.pc(), self.q_pattern);
        prompt(
            ui,
            &format!(
                "Play {} {} – {} up and down, evenly, at {:.0} BPM (8th notes)",
                ctx.name(s.root),
                s.ty.name,
                self.q_pos.name,
                ctx.settings.bpm
            ),
        );
        hint(ui, "First look at the shape, then try it with the metronome. Rate yourself honestly: the app brings back what you miss.");
        let playing = self.playing_index(ctx).map(|i| self.play_list[i]);
        let dots: Vec<Dot> = self
            .q_pos
            .notes
            .iter()
            .map(|p| {
                let d = self.dot(ctx, &s, *p);
                if Some(*p) == playing { d.emphasis() } else { d }
            })
            .collect();
        let mut board = ctx.board(&dots);
        board.zone = Some(self.q_pos.fret_range());
        board.show(ui);
        let notes = self.q_pos.notes.clone();
        self.transport(ui, ctx, &notes);
        if self.playing_index(ctx).is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(30));
        }
        if let Some(ok) = self_rate(ui) {
            ctx.progress.record(&key, ok, None);
            ctx.audio.stop();
            self.play_list.clear();
            self.pick_pattern(ctx, "sc:play");
        }
        let _ = Pc::default;
        stats_line(ui, ctx, &key);
    }
}
