//! Standard notation (treble clef 8vb, as guitar music is written) with an
//! optional tab staff below, sharing the same horizontal positions.
//!
//! Guitar sounds one octave lower than written, so a sounding E2 (low open E)
//! is written as E3 - three ledger lines below the staff.

use crate::theory::fretboard::Pos;
use crate::theory::note::Pitch;
use egui::{Align2, Color32, FontFamily, FontId, Painter, Pos2, Rect, Sense, Stroke, Ui, Vec2};

pub const MUSIC_FONT: &str = "bravura";

/// Staff sizes (space between two staff lines, in points). Drills, where the staff
/// *is* the question, use the big one; libraries and reference views the small one.
pub const STAFF_DRILL: f32 = 15.0;
pub const STAFF_LIB: f32 = 11.0;

const G_CLEF_8VB: char = '\u{E052}';
const NOTEHEAD_WHOLE: char = '\u{E0A2}';
const NOTEHEAD_BLACK: char = '\u{E0A4}';
const FLAT: char = '\u{E260}';
const SHARP: char = '\u{E262}';
const DOUBLE_SHARP: char = '\u{E263}';
const DOUBLE_FLAT: char = '\u{E264}';

/// One column: notes sounding together (a chord or a single note) and their tab.
#[derive(Clone, Debug, Default)]
pub struct Column {
    pub pitches: Vec<Pitch>,
    pub tab: Vec<Pos>,
    pub label: String,
}

pub struct NotationView<'a> {
    pub columns: &'a [Column],
    pub staff: bool,
    pub tab: bool,
    /// Whole notes (chords, single prompts) vs. black noteheads (scale runs).
    pub whole_notes: bool,
    pub highlight: Option<usize>,
    pub staff_space: f32,
    /// Hide the tab numbers (quiz mode).
    pub hide_tab: bool,
}

impl<'a> NotationView<'a> {
    pub fn new(columns: &'a [Column]) -> Self {
        NotationView {
            columns,
            staff: true,
            tab: true,
            whole_notes: true,
            highlight: None,
            staff_space: 9.0,
            hide_tab: false,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let sp = self.staff_space;
        let col_w = if self.whole_notes { sp * 5.0 } else { sp * 3.2 };
        // room above/below the 5 lines grows with the highest/lowest note: nothing is clipped
        let positions = self.columns.iter().flat_map(|c| c.pitches.iter()).map(|p| {
            let written = Pitch { note: p.note, octave: p.octave + 1 };
            written.diatonic_step() - E4_STEP
        });
        let (lo_all, hi_all) = positions.fold((0, 8), |(lo, hi), p| (lo.min(p), hi.max(p)));
        let above = (4.5f32).max((hi_all - 8) as f32 * 0.5 + 2.5);
        let below = (5.0f32).max(-lo_all as f32 * 0.5 + 2.5);
        let staff_h = if self.staff { sp * (above + 4.0 + below) } else { 0.0 };
        let tab_gap = sp * 1.4;
        let tab_h = if self.tab { tab_gap * 5.0 + sp * 4.5 } else { 0.0 };
        let left_w = sp * 5.0;
        let needed = left_w + col_w * self.columns.len().max(1) as f32 + sp * 2.0;
        let width = ui.available_width().max(needed.min(ui.available_width().max(200.0)));
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width.max(needed), staff_h + tab_h), Sense::hover());
        let painter = ui.painter_at(rect.expand(2.0));
        let ink = ui.visuals().text_color();
        let accent = Color32::from_rgb(255, 140, 40);
        let music = FontFamily::Name(MUSIC_FONT.into());
        let em_size = calibrate(&painter, &music) * 4.0 * sp;

        let x_of = |i: usize| rect.left() + left_w + col_w * (i as f32 + 0.5);
        let right = (rect.left() + left_w + col_w * self.columns.len() as f32 + sp).min(rect.right());

        if self.staff {
            let bottom_line = rect.top() + sp * (above + 4.0);
            let line_y = |p: i32| bottom_line - p as f32 * sp * 0.5;
            for l in 0..5 {
                let y = line_y(l * 2);
                painter.line_segment(
                    [Pos2::new(rect.left(), y), Pos2::new(right, y)],
                    Stroke::new(1.3, ink.gamma_multiply(0.85)),
                );
            }
            // clef: baseline on the G line (p = 2)
            glyph(&painter, G_CLEF_8VB, rect.left() + sp * 0.5, line_y(2), em_size, &music, ink, false);

            for (i, c) in self.columns.iter().enumerate() {
                let col = if self.highlight == Some(i) { accent } else { ink };
                let x = x_of(i);
                let mut notes: Vec<(i32, i8)> = c
                    .pitches
                    .iter()
                    .map(|p| {
                        let written = Pitch { note: p.note, octave: p.octave + 1 };
                        (written.diatonic_step() - E4_STEP, p.note.acc)
                    })
                    .collect();
                notes.sort();
                notes.dedup();
                let head = if self.whole_notes { NOTEHEAD_WHOLE } else { NOTEHEAD_BLACK };
                let head_w = if self.whole_notes { sp * 1.69 } else { sp * 1.18 };
                // ledger lines
                let lo = notes.iter().map(|n| n.0).min().unwrap_or(0);
                let hi = notes.iter().map(|n| n.0).max().unwrap_or(0);
                let led = |p: i32| {
                    let y = line_y(p);
                    painter.line_segment(
                        [Pos2::new(x - head_w * 0.95, y), Pos2::new(x + head_w * 0.95, y)],
                        Stroke::new(1.6, ink),
                    );
                };
                let mut p = -2;
                while p >= lo {
                    led(p);
                    p -= 2;
                }
                let mut p = 10;
                while p <= hi {
                    led(p);
                    p += 2;
                }
                // noteheads (seconds are shifted right)
                let mut last: Option<(i32, bool)> = None;
                let mut acc_col = 0;
                let mut last_acc_p: Option<i32> = None;
                for &(p, acc) in notes.iter() {
                    let shifted = matches!(last, Some((lp, ls)) if p - lp == 1 && !ls);
                    let hx = x - head_w * 0.5 + if shifted { head_w * 0.95 } else { 0.0 };
                    glyph(&painter, head, hx, line_y(p), em_size, &music, col, false);
                    last = Some((p, shifted));
                    if acc != 0 {
                        let ch = match acc {
                            -2 => DOUBLE_FLAT,
                            -1 => FLAT,
                            1 => SHARP,
                            _ => DOUBLE_SHARP,
                        };
                        if matches!(last_acc_p, Some(lp) if p - lp < 6) {
                            acc_col += 1;
                        } else {
                            acc_col = 0;
                        }
                        last_acc_p = Some(p);
                        let ax = x - head_w * 0.5 - sp * 1.3 * (acc_col as f32 + 1.0);
                        glyph(&painter, ch, ax, line_y(p), em_size, &music, col, false);
                    }
                }
            }
        }

        if self.tab {
            let top = rect.top() + staff_h + sp * 1.5;
            let y_of = |s: u8| top + (5 - s) as f32 * tab_gap;
            for s in 0..6 {
                let y = y_of(s);
                painter.line_segment(
                    [Pos2::new(rect.left(), y), Pos2::new(right, y)],
                    Stroke::new(1.0, ink.gamma_multiply(0.6)),
                );
            }
            for (k, ch) in ["T", "A", "B"].iter().enumerate() {
                painter.text(
                    Pos2::new(rect.left() + sp * 1.5, top + tab_gap * (1.0 + k as f32 * 1.5)),
                    Align2::CENTER_CENTER,
                    *ch,
                    FontId::proportional(sp * 1.5),
                    ink,
                );
            }
            let bg = ui.visuals().panel_fill;
            for (i, c) in self.columns.iter().enumerate() {
                let col = if self.highlight == Some(i) { accent } else { ink };
                let x = x_of(i);
                for p in &c.tab {
                    let y = y_of(p.string);
                    let t = if self.hide_tab { "?".to_string() } else { p.fret.to_string() };
                    let r = Rect::from_center_size(Pos2::new(x, y), Vec2::new(sp * 1.1 * t.len() as f32 + 2.0, sp * 1.3));
                    painter.rect_filled(r, 0.0, bg);
                    painter.text(Pos2::new(x, y), Align2::CENTER_CENTER, t, FontId::monospace(sp * 1.35), col);
                }
                if !c.label.is_empty() {
                    painter.text(
                        Pos2::new(x, y_of(0) + sp * 1.8),
                        Align2::CENTER_CENTER,
                        &c.label,
                        FontId::proportional(sp * 1.25),
                        ink.gamma_multiply(0.8),
                    );
                }
            }
        } else {
            for (i, c) in self.columns.iter().enumerate() {
                if !c.label.is_empty() {
                    painter.text(
                        Pos2::new(x_of(i), rect.bottom() - sp),
                        Align2::CENTER_CENTER,
                        &c.label,
                        FontId::proportional(sp * 1.25),
                        ink.gamma_multiply(0.8),
                    );
                }
            }
        }
    }
}

/// diatonic step of E4 (bottom line of the treble staff)
const E4_STEP: i32 = 4 * 7 + 2;

/// Font size → em size ratio of the music font (measured from the notehead's
/// advance width, which is 1.18 staff spaces = 0.295 em in Bravura).
fn calibrate(painter: &Painter, family: &FontFamily) -> f32 {
    let probe = 100.0;
    let g = painter.layout_no_wrap(NOTEHEAD_BLACK.to_string(), FontId::new(probe, family.clone()), Color32::WHITE);
    let adv = g
        .rows
        .first()
        .and_then(|r| r.row.glyphs.first())
        .map(|g| g.advance_width)
        .unwrap_or(29.5);
    let em_at_probe = adv / 0.295;
    if em_at_probe <= 1.0 { 1.0 } else { probe / em_at_probe }
}

/// Draw a music glyph with its baseline at `baseline_y`, left edge at `x`.
#[allow(clippy::too_many_arguments)]
fn glyph(
    painter: &Painter,
    ch: char,
    x: f32,
    baseline_y: f32,
    size: f32,
    family: &FontFamily,
    color: Color32,
    center: bool,
) {
    let g = painter.layout_no_wrap(ch.to_string(), FontId::new(size, family.clone()), color);
    let (off, w) = g
        .rows
        .first()
        .and_then(|r| r.row.glyphs.first().map(|gl| (r.pos.y + gl.pos.y, gl.advance_width)))
        .unwrap_or((size * 0.5, 0.0));
    let x = if center { x - w * 0.5 } else { x };
    painter.galley(Pos2::new(x, baseline_y - off), g, color);
}
