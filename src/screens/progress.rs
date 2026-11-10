//! Progress overview, heatmap, weakest items, and the mixed "Today's practice" session.

use super::common::*;
use super::fretboard_trainer::name_key;
use crate::srs::Progress;
use crate::theory::chord::{Chord, ChordType};
use crate::theory::fretboard::Pos;
use crate::theory::note::{Naming, Note, Pitch};
use crate::theory::scale::ScaleType;
use crate::widgets::{heading, hint};
use egui::{Color32, RichText, Ui};

/// Drills used by the mixed practice session.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Drill {
    FbName,
    FbFind,
    FbStaff,
    ChRead,
    ChName,
    ScDegree,
    ScFill,
}

impl Drill {
    pub const ALL: [Drill; 7] = [
        Drill::FbName,
        Drill::FbFind,
        Drill::FbStaff,
        Drill::ChRead,
        Drill::ChName,
        Drill::ScDegree,
        Drill::ScFill,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Drill::FbName => "Fretboard · name the note",
            Drill::FbFind => "Fretboard · find the note",
            Drill::FbStaff => "Fretboard · staff → fret",
            Drill::ChRead => "Chords · sight-read",
            Drill::ChName => "Chords · name this chord",
            Drill::ScDegree => "Scales · which degree?",
            Drill::ScFill => "Scales · fill the box",
        }
    }
    pub fn prefixes(self) -> &'static [&'static str] {
        match self {
            Drill::FbName => &["fb:name:"],
            Drill::FbFind => &["fb:find:", "fb:zone:"],
            Drill::FbStaff => &["fb:staff:"],
            Drill::ChRead => &["ch:read:"],
            Drill::ChName => &["ch:name:"],
            Drill::ScDegree => &["sc:deg:"],
            Drill::ScFill => &["sc:fill:"],
        }
    }
    /// Accuracy so far (None = never practised).
    pub fn accuracy(self, p: &Progress) -> Option<f32> {
        let mut a = 0;
        let mut c = 0.0;
        for pre in self.prefixes() {
            let (att, acc, _) = p.summary(pre);
            a += att;
            c += acc * att as f32;
        }
        (a > 0).then(|| c / a as f32)
    }
}

pub struct Session {
    pub plan: Vec<Drill>,
    pub step: usize,
    pub start_attempts: u32,
    pub block_start: u32,
    pub total: u32,
    pub per_block: u32,
}

impl Session {
    pub fn new(p: &Progress) -> Session {
        let mut plan: Vec<(f32, Drill)> = Drill::ALL
            .iter()
            .map(|d| (d.accuracy(p).unwrap_or(0.55), *d))
            .collect();
        plan.sort_by(|a, b| a.0.total_cmp(&b.0));
        let a = p.today().attempts;
        Session {
            plan: plan.into_iter().map(|x| x.1).take(5).collect(),
            step: 0,
            start_attempts: a,
            block_start: a,
            total: 30,
            per_block: 6,
        }
    }
    pub fn current(&self) -> Drill {
        self.plan[self.step % self.plan.len()]
    }
    pub fn answered(&self, p: &Progress) -> u32 {
        p.today().attempts.saturating_sub(self.start_attempts)
    }
}

pub fn describe_key(key: &str, naming: Naming) -> String {
    let parts: Vec<&str> = key.split(':').collect();
    let note = |pc: &str| Note::from_pc(pc.parse::<u8>().unwrap_or(0)).name(naming);
    let chord = |pc: &str, id: &str| {
        ChordType::by_id(id)
            .map(|t| Chord::new(Note::from_pc(pc.parse::<u8>().unwrap_or(0)), t).symbol(naming))
            .unwrap_or_else(|| id.to_string())
    };
    let scale = |id: &str| ScaleType::by_id(id).map(|s| s.name.to_string()).unwrap_or_else(|| id.to_string());
    match parts.as_slice() {
        ["fb", "name", sf] => {
            let s: u8 = sf[1..sf.find('f').unwrap_or(1)].parse().unwrap_or(0);
            let f: u8 = sf[sf.find('f').map(|i| i + 1).unwrap_or(0)..].parse().unwrap_or(0);
            format!("Name note on string {} fret {}", 6 - s, f)
        }
        ["fb", "find", pc, s] => format!("Find {} on string {}", note(pc), 6 - s[1..].parse::<u8>().unwrap_or(0)),
        ["fb", "zone", pc] => format!("Find all {} in a zone", note(pc)),
        ["fb", "staff", m] => format!("Staff → fret: {}", Pitch::from_midi(m.parse().unwrap_or(60)).name(naming)),
        ["ch", "read", pc, id] => format!("Sight-read {}", chord(pc, id)),
        ["ch", "name", pc, id] => format!("Name chord {}", chord(pc, id)),
        ["sc", "fill", id, pc, i] => format!("Fill {} {} position {}", note(pc), scale(id), i.parse::<usize>().unwrap_or(0) + 1),
        ["sc", "deg", id, d] => format!("Degree {} in {}", d, scale(id)),
        ["sc", "play", id, pc, i] => format!("Play {} {} position {}", note(pc), scale(id), i.parse::<usize>().unwrap_or(0) + 1),
        _ => key.to_string(),
    }
}

/// Returns a drill to start (session) if the user clicked the button.
pub fn ui(ui: &mut Ui, ctx: &mut Ctx) -> bool {
    let mut start = false;
    egui::ScrollArea::vertical().id_salt("progress").show(ui, |ui| {
        let d = ctx.progress.today();
        heading(ui, "Today");
        ui.label(
            RichText::new(format!(
                "{} answers · {:.0}% correct · streak: {} day{}",
                d.attempts,
                if d.attempts > 0 { d.correct as f32 * 100.0 / d.attempts as f32 } else { 0.0 },
                ctx.progress.streak(),
                if ctx.progress.streak() == 1 { "" } else { "s" }
            ))
            .size(18.0),
        );
        ui.add_space(6.0);
        if ui
            .button(RichText::new("▶ Start today's practice (30 questions, weakest areas first)").size(19.0))
            .clicked()
        {
            start = true;
        }
        hint(ui, "The session moves through your five weakest drills, 6 questions each. Inside every drill, weak and new items come up more often.");

        ui.add_space(10.0);
        heading(ui, "Areas");
        egui::Grid::new("areas").striped(true).show(ui, |ui| {
            for dr in Drill::ALL {
                ui.label(RichText::new(dr.label()).size(16.0));
                let mut att = 0;
                let mut items = 0;
                for pre in dr.prefixes() {
                    let (a, _, n) = ctx.progress.summary(pre);
                    att += a;
                    items += n;
                }
                match dr.accuracy(ctx.progress) {
                    Some(acc) => {
                        let c = Color32::from_rgb((230.0 * (1.0 - acc)) as u8 + 20, (200.0 * acc) as u8 + 30, 70);
                        ui.label(RichText::new(format!("{:.0}%", acc * 100.0)).size(16.0).color(c));
                        ui.label(format!("{att} answers · {items} items"));
                    }
                    None => {
                        ui.label("–");
                        ui.label("not practised yet");
                    }
                }
                ui.end_row();
            }
        });

        ui.add_space(10.0);
        heading(ui, "Fretboard heatmap (name the note)");
        hint(ui, "Green = you know it, red = weak, empty = not practised yet.");
        let progress = &*ctx.progress;
        let tint = |p: Pos| -> Option<Color32> {
            let it = progress.get(&name_key(p))?;
            let a = it.accuracy();
            Some(Color32::from_rgba_unmultiplied((255.0 * (1.0 - a)) as u8, (210.0 * a) as u8, 60, 170))
        };
        let mut board = ctx.board(&[]);
        board.tint = Some(&tint);
        board.height = 190.0;
        board.show(ui);

        ui.add_space(10.0);
        heading(ui, "Weakest items");
        let naming = ctx.settings.naming;
        for (k, it) in ctx.progress.weakest(15) {
            ui.label(format!(
                "{}  –  {}/{} correct, box {}/5{}",
                describe_key(k, naming),
                it.correct,
                it.attempts,
                it.level + 1,
                if it.avg_ms > 0.0 { format!(", {:.1} s", it.avg_ms / 1000.0) } else { String::new() }
            ));
        }
    });
    start
}
