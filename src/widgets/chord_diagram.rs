//! Classic vertical chord box: low string on the left, nut at the top.

use super::fretboard::contrast_text;
use crate::theory::voicing::Voicing;
use egui::{Align2, Color32, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

pub struct ChordDiagram<'a> {
    pub voicing: &'a Voicing,
    /// Label per string (finger number, interval, note ...).
    pub labels: [String; 6],
    pub colors: [Color32; 6],
    pub width: f32,
    pub title: Option<String>,
    pub selected: bool,
    pub subtitle: Option<String>,
}

impl<'a> ChordDiagram<'a> {
    pub fn show(self, ui: &mut Ui) -> Response {
        let w = self.width;
        let sx = w / 7.3;
        let (first, count) = self.voicing.diagram_window();
        let count = count.min(6);
        let fy = sx * 1.25;
        let title_h = if self.title.is_some() { sx * 1.4 } else { 0.0 };
        let sub_h = if self.subtitle.is_some() { sx * 1.2 } else { 0.0 };
        let h = title_h + sx * 1.2 + fy * count as f32 + sx * 0.6 + sub_h;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, h), Sense::click());
        let p = ui.painter_at(rect);
        let ink = ui.visuals().text_color();
        if self.selected {
            p.rect_filled(rect, 6.0, ui.visuals().selection.bg_fill.gamma_multiply(0.5));
        } else if resp.hovered() {
            p.rect_filled(rect, 6.0, ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.5));
        }
        if let Some(t) = &self.title {
            p.text(
                Pos2::new(rect.center().x, rect.top() + title_h * 0.5),
                Align2::CENTER_CENTER,
                t,
                FontId::proportional(sx * 1.0),
                ink,
            );
        }
        let left = rect.left() + sx * 1.6;
        let top = rect.top() + title_h + sx * 1.2;
        let xs = |s: usize| left + s as f32 * sx;
        // frets
        for i in 0..=count {
            let y = top + i as f32 * fy;
            let thick = if i == 0 && first == 1 { 4.0 } else { 1.0 };
            p.line_segment([Pos2::new(xs(0), y), Pos2::new(xs(5), y)], Stroke::new(thick, ink));
        }
        for s in 0..6 {
            p.line_segment([Pos2::new(xs(s), top), Pos2::new(xs(s), top + count as f32 * fy)], Stroke::new(1.0, ink));
        }
        if first > 1 {
            p.text(
                Pos2::new(xs(0) - sx * 0.45, top + fy * 0.5),
                Align2::RIGHT_CENTER,
                format!("{first}"),
                FontId::proportional(sx * 0.85),
                ink,
            );
        }
        // barre
        if let Some((bf, lo, hi)) = self.voicing.barre
            && bf >= first
        {
            let y = top + (bf - first) as f32 * fy + fy * 0.5;
            let r = Rect::from_min_max(
                Pos2::new(xs(lo as usize) - sx * 0.35, y - sx * 0.3),
                Pos2::new(xs(hi as usize) + sx * 0.35, y + sx * 0.3),
            );
            p.rect_filled(r, sx * 0.3, ink.gamma_multiply(0.55));
        }
        let r = sx * 0.46;
        for s in 0..6 {
            let x = xs(s);
            match self.voicing.frets[s] {
                None => {
                    p.text(Pos2::new(x, top - sx * 0.55), Align2::CENTER_CENTER, "×", FontId::proportional(sx * 0.9), ink);
                }
                Some(0) => {
                    p.circle_stroke(Pos2::new(x, top - sx * 0.55), r * 0.75, Stroke::new(1.5, self.colors[s]));
                }
                Some(f) => {
                    let y = top + (f - first) as f32 * fy + fy * 0.5;
                    p.circle_filled(Pos2::new(x, y), r, self.colors[s]);
                    if !self.labels[s].is_empty() {
                        p.text(
                            Pos2::new(x, y),
                            Align2::CENTER_CENTER,
                            &self.labels[s],
                            FontId::proportional(sx * if self.labels[s].len() > 1 { 0.58 } else { 0.72 }),
                            contrast_text(self.colors[s]),
                        );
                    }
                }
            }
        }
        if let Some(sub) = &self.subtitle {
            p.text(
                Pos2::new(rect.center().x, rect.bottom() - sub_h * 0.5),
                Align2::CENTER_CENTER,
                sub,
                FontId::monospace(sx * 0.7),
                ink.gamma_multiply(0.8),
            );
        }
        resp
    }
}
