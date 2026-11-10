//! Shared state passed to every screen + small drill helpers.

use crate::audio::Audio;
use crate::settings::{LabelMode, Settings};
use crate::srs::Progress;
use crate::theory::fretboard::{Pos, pc_at};
use crate::theory::interval::{Interval, interval_color};
use crate::theory::note::{Note, Pc};
use crate::theory::tuning::Tuning;
use crate::util::Rng;
use crate::widgets::fretboard::{Dot, FretboardView, rgb};
use egui::{Color32, Key, RichText, Ui};

pub enum Nav {
    Chord { root: Note, ty: &'static str },
}

pub struct Ctx<'a> {
    pub settings: &'a mut Settings,
    pub progress: &'a mut Progress,
    pub audio: &'a mut Audio,
    pub rng: &'a mut Rng,
    pub nav: &'a mut Option<Nav>,
    /// UI time in seconds
    pub time: f64,
}

impl Ctx<'_> {
    pub fn tuning(&self) -> &'static Tuning {
        self.settings.tuning()
    }

    pub fn play_note(&mut self, midi: i32) {
        if self.settings.sound {
            self.audio.note(midi);
        }
    }

    pub fn play_chord(&mut self, midis: &[i32]) {
        if self.settings.sound {
            self.audio.strum(midis);
        }
    }

    pub fn name(&self, n: Note) -> String {
        n.name(self.settings.naming)
    }

    pub fn pc_name(&self, pc: Pc) -> String {
        Note::from_pc(pc).name(self.settings.naming)
    }

    pub fn string_names(&self) -> [String; 6] {
        let t = self.tuning();
        std::array::from_fn(|s| t.strings[s].note.name(self.settings.naming))
    }

    /// Fretboard pre-configured with the user's settings.
    pub fn board<'b>(&self, dots: &'b [Dot]) -> FretboardView<'b> {
        let mut v = FretboardView::new(self.settings.frets);
        v.dots = dots;
        v.left_handed = self.settings.left_handed;
        v.string_names = Some(self.string_names());
        v
    }

    /// Dot for a position relative to a root, labelled according to settings.
    pub fn interval_dot(&self, pos: Pos, root: Note, interval: Option<Interval>, spelled: Note) -> Dot {
        let pc = pc_at(self.tuning(), pos);
        let semis = (pc as i32 - root.pc() as i32).rem_euclid(12) as Pc;
        let label = match self.settings.labels {
            LabelMode::Notes => spelled.name(self.settings.naming),
            LabelMode::Intervals => interval.map(|i| i.label()).unwrap_or_default(),
            LabelMode::None => String::new(),
        };
        Dot::new(pos, label, rgb(interval_color(semis)))
    }

    pub fn ms_since(&self, start: f64) -> f32 {
        ((self.time - start) * 1000.0) as f32
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AnswerMode {
    /// Answer by clicking in the app (checked automatically).
    OnScreen,
    /// Play it on the guitar, reveal, rate yourself.
    OnGuitar,
}

pub fn answer_mode_picker(ui: &mut Ui, m: &mut AnswerMode) -> bool {
    let before = *m;
    ui.horizontal(|ui| {
        ui.label("Answer:");
        ui.selectable_value(m, AnswerMode::OnScreen, "🖱 on screen");
        ui.selectable_value(m, AnswerMode::OnGuitar, "🎸 on the guitar (self-check)");
    });
    before != *m
}

/// Self rating after "reveal": returns Some(correct).
pub fn self_rate(ui: &mut Ui) -> Option<bool> {
    let mut r = None;
    ui.horizontal(|ui| {
        ui.label("How did it go?");
        if ui.button(RichText::new("1 · missed").size(16.0)).clicked() {
            r = Some(false);
        }
        if ui.button(RichText::new("2 · got it, slowly").size(16.0)).clicked() {
            r = Some(true);
        }
        if ui.button(RichText::new("3 · easy").size(16.0)).clicked() {
            r = Some(true);
        }
    });
    ui.input(|i| {
        if i.key_pressed(Key::Num1) {
            r = Some(false);
        } else if i.key_pressed(Key::Num2) || i.key_pressed(Key::Num3) {
            r = Some(true);
        }
    });
    r
}

pub fn reveal_button(ui: &mut Ui) -> bool {
    let b = ui.button(RichText::new("Reveal answer  (Space)").size(17.0)).clicked();
    b || ui.input(|i| i.key_pressed(Key::Space))
}

pub fn next_pressed(ui: &mut Ui) -> bool {
    let b = ui.button(RichText::new("Next  (Space / Enter)").size(17.0)).clicked();
    b || ui.input(|i| i.key_pressed(Key::Space) || i.key_pressed(Key::Enter))
}

/// Timed feedback message ("✔ correct", "✖ that was G").
pub struct Feedback {
    pub text: String,
    pub ok: bool,
}

pub fn show_feedback(ui: &mut Ui, f: &Option<Feedback>) {
    if let Some(f) = f {
        let c = if f.ok {
            Color32::from_rgb(90, 200, 110)
        } else {
            Color32::from_rgb(230, 80, 80)
        };
        ui.label(RichText::new(&f.text).size(20.0).strong().color(c));
    } else {
        ui.label(RichText::new(" ").size(20.0));
    }
}

/// Big prompt text.
pub fn prompt(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(26.0).strong());
}

pub const RED: Color32 = Color32::from_rgb(220, 50, 47);
pub const GREEN: Color32 = Color32::from_rgb(80, 190, 100);
pub const GREY: Color32 = Color32::from_rgb(130, 130, 130);
pub const BLUE: Color32 = Color32::from_rgb(70, 140, 230);
