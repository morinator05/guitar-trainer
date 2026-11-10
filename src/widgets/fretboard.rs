//! Horizontal fretboard: high e string on top (like tab), low string at the bottom.
//!
//! Three looks (Settings → Fretboard): realistic rosewood, realistic maple
//! (Strat-style), and the flat "classic" drawing. Everything is drawn in code;
//! the wood grain is a procedurally generated texture (see `wood.rs`).

use super::wood;
use crate::settings::FretStyle;
use crate::theme;
use crate::theory::fretboard::Pos;
use egui::{Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

#[derive(Clone, Debug)]
pub struct Dot {
    pub pos: Pos,
    pub label: String,
    pub fill: Color32,
    /// Draw only an outline (e.g. "hidden" or "hint" notes).
    pub hollow: bool,
    /// Bigger with a ring (current note, the question, ...).
    pub emphasis: bool,
}

impl Dot {
    pub fn new(pos: Pos, label: impl Into<String>, fill: Color32) -> Dot {
        Dot {
            pos,
            label: label.into(),
            fill,
            hollow: false,
            emphasis: false,
        }
    }
    /// A neutral "which note is this?" dot that gives nothing away.
    pub fn question(pos: Pos) -> Dot {
        Dot::new(pos, "?", theme::QUESTION).emphasis()
    }
    pub fn hollow(mut self) -> Dot {
        self.hollow = true;
        self
    }
    pub fn emphasis(mut self) -> Dot {
        self.emphasis = true;
        self
    }
}

pub struct FretboardView<'a> {
    pub max_fret: u8,
    pub dots: &'a [Dot],
    pub lines: &'a [(Pos, Pos, Color32)],
    /// Fret range (inclusive) of a drill zone; everything outside is dimmed.
    pub zone: Option<(u8, u8)>,
    /// Per-position background tint (heatmap).
    pub tint: Option<&'a dyn Fn(Pos) -> Option<Color32>>,
    pub left_handed: bool,
    pub clickable: bool,
    /// Fixed height; None = adapt to the available space.
    pub height: Option<f32>,
    pub string_names: Option<[String; 6]>,
    pub style: FretStyle,
}

const NUT: Color32 = Color32::from_rgb(0xEC, 0xE4, 0xD2);
const NUT_EDGE: Color32 = Color32::from_rgb(0xB5, 0xAA, 0x92);
const FRET_BODY: Color32 = Color32::from_rgb(0xA4, 0xA9, 0xAE);
const FRET_HI: Color32 = Color32::from_rgb(0xEE, 0xF1, 0xF3);
const PEARL: Color32 = Color32::from_rgb(0xEE, 0xE7, 0xDA);
const BLACK_DOT: Color32 = Color32::from_rgb(0x1E, 0x1A, 0x16);
const BINDING: Color32 = Color32::from_rgb(0xEA, 0xDF, 0xC8);
const WOUND: Color32 = Color32::from_rgb(0xB9, 0xB2, 0xA4);
const PLAIN: Color32 = Color32::from_rgb(0xD9, 0xDA, 0xDB);
const CLASSIC_WOOD: Color32 = Color32::from_rgb(58, 40, 28);

fn shade(a: u8) -> Color32 {
    Color32::from_black_alpha(a)
}

/// Vertical gradient rectangle (top colour → bottom colour).
fn v_gradient(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.left_bottom(), bottom);
    m.colored_vertex(r.right_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(1, 2, 3);
    p.add(m);
}

impl<'a> FretboardView<'a> {
    pub fn new(max_fret: u8) -> Self {
        FretboardView {
            max_fret,
            dots: &[],
            lines: &[],
            zone: None,
            tint: None,
            left_handed: false,
            clickable: false,
            height: None,
            string_names: None,
            style: FretStyle::Rosewood,
        }
    }

    /// Draws the neck; returns the clicked position, if any.
    pub fn show(self, ui: &mut Ui) -> Option<Pos> {
        let width = ui.available_width().max(300.0);
        let height = self
            .height
            .unwrap_or_else(|| (ui.available_height() * 0.45).clamp(220.0, 330.0));
        let sense = if self.clickable { Sense::click() } else { Sense::hover() };
        let (resp, painter) = ui.allocate_painter(Vec2::new(width, height), sense);
        let rect = resp.rect;
        let real = self.style != FretStyle::Classic;
        let maple = self.style == FretStyle::Maple;

        // ---------------------------------------------------------------- layout
        let name_w = 26.0;
        let numbers_h = 24.0;
        let neck_top = rect.top() + 2.0;
        let neck_bottom = rect.bottom() - numbers_h;
        let gap = (neck_bottom - neck_top) / 6.0;
        let top = neck_top + gap * 0.5;
        let bottom = neck_bottom - gap * 0.5;
        let frets = self.max_fret.max(1) as f32;
        let r_est = (gap * 0.42).clamp(8.0, 16.0);
        let open_w = 2.0 * r_est + 10.0;
        let nut_w = if real { 7.0 } else { 5.0 };
        let nut_x = rect.left() + name_w + open_w;
        let fw = (rect.right() - 8.0 - nut_x) / frets;
        let neck_right = nut_x + frets * fw;
        let r = (gap.min(fw) * 0.42).clamp(7.0, 16.0);

        let mirror = |x: f32| if self.left_handed { rect.left() + rect.right() - x } else { x };
        let mrect = |x0: f32, x1: f32, y0: f32, y1: f32| {
            let (a, b) = (mirror(x0), mirror(x1));
            Rect::from_min_max(Pos2::new(a.min(b), y0), Pos2::new(a.max(b), y1))
        };
        let string_y = |s: u8| bottom - s as f32 * gap;
        let fret_x = |f: u8| nut_x + f as f32 * fw;
        let fret_center_x = |f: u8| if f == 0 { nut_x - nut_w - (open_w - nut_w) * 0.5 } else { nut_x + (f as f32 - 0.5) * fw };
        let pt = |p: Pos| Pos2::new(mirror(fret_center_x(p.fret)), string_y(p.string));

        // ---------------------------------------------------------------- wood
        let neck = mrect(nut_x, neck_right, neck_top, neck_bottom);
        if let Some(tex) = wood::texture(ui.ctx(), self.style) {
            // stretch the grain along the neck; show a fret-count dependent slice
            let u1 = (frets / 22.0).clamp(0.5, 1.0);
            let uv = if self.left_handed {
                Rect::from_min_max(Pos2::new(u1, 0.0), Pos2::new(0.0, 1.0))
            } else {
                Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(u1, 1.0))
            };
            // soft shadow under the neck
            painter.rect_filled(neck.translate(Vec2::new(0.0, 3.0)).expand(2.0), 4.0, shade(70));
            painter.image(tex.id(), neck, uv, Color32::WHITE);
            // rounded edges: darker towards top and bottom
            let edge = gap * 0.45;
            v_gradient(&painter, Rect::from_min_max(neck.left_top(), Pos2::new(neck.right(), neck.top() + edge)), shade(if maple { 70 } else { 120 }), shade(0));
            v_gradient(&painter, Rect::from_min_max(Pos2::new(neck.left(), neck.bottom() - edge), neck.right_bottom()), shade(0), shade(if maple { 80 } else { 130 }));
            if maple {
                // lacquer gloss band
                let gy = neck.top() + gap * 1.2;
                let w = Color32::from_white_alpha(28);
                v_gradient(&painter, Rect::from_min_max(Pos2::new(neck.left(), gy - gap * 0.5), Pos2::new(neck.right(), gy)), Color32::TRANSPARENT, w);
                v_gradient(&painter, Rect::from_min_max(Pos2::new(neck.left(), gy), Pos2::new(neck.right(), gy + gap * 0.8)), w, Color32::TRANSPARENT);
            } else {
                // cream binding
                painter.line_segment([neck.left_top(), neck.right_top()], Stroke::new(2.0, BINDING));
                painter.line_segment([neck.left_bottom(), neck.right_bottom()], Stroke::new(2.0, BINDING));
            }
        } else {
            painter.rect_filled(neck, 3.0, CLASSIC_WOOD);
        }

        // ---------------------------------------------------------------- heatmap
        if let Some(tint) = self.tint {
            for s in 0..6u8 {
                for f in 0..=self.max_fret {
                    if let Some(c) = tint(Pos::new(s, f)) {
                        let w = if f == 0 { open_w - nut_w } else { fw };
                        let cell = Rect::from_center_size(pt(Pos::new(s, f)), Vec2::new(w - 2.0, gap - 2.0));
                        painter.rect_filled(cell, 3.0, c);
                    }
                }
            }
        }

        // ---------------------------------------------------------------- inlays
        let dot_at = |f: u8, strings: &[u8]| self.dots.iter().any(|d| d.pos.fret == f && strings.contains(&d.pos.string));
        let inlay_r = (gap.min(fw) * 0.17).clamp(3.0, 6.5);
        let inlay = |c: Pos2| {
            if real {
                let col = if maple { BLACK_DOT } else { PEARL };
                painter.circle_filled(c, inlay_r, col);
                if !maple {
                    painter.circle_filled(c + Vec2::new(-inlay_r * 0.3, -inlay_r * 0.3), inlay_r * 0.4, Color32::from_white_alpha(120));
                }
            } else {
                painter.circle_filled(c, inlay_r * 0.85, Color32::from_gray(150));
            }
        };
        let mid_y = (top + bottom) * 0.5;
        for f in 1..=self.max_fret {
            let x = mirror(fret_center_x(f));
            match f % 12 {
                3 | 5 | 7 | 9 if !dot_at(f, &[2, 3]) => inlay(Pos2::new(x, mid_y)),
                0 => {
                    if !dot_at(f, &[3, 4]) {
                        inlay(Pos2::new(x, mid_y - gap));
                    }
                    if !dot_at(f, &[1, 2]) {
                        inlay(Pos2::new(x, mid_y + gap));
                    }
                }
                _ => {}
            }
        }

        // ---------------------------------------------------------------- frets + nut
        for f in 1..=self.max_fret {
            let x = mirror(fret_x(f));
            let (y0, y1) = (neck_top, neck_bottom);
            if real {
                let sh = if self.left_handed { -2.0 } else { 2.0 };
                painter.line_segment([Pos2::new(x + sh, y0), Pos2::new(x + sh, y1)], Stroke::new(2.0, shade(90)));
                painter.line_segment([Pos2::new(x, y0), Pos2::new(x, y1)], Stroke::new(3.2, FRET_BODY));
                painter.line_segment([Pos2::new(x - 0.6, y0), Pos2::new(x - 0.6, y1)], Stroke::new(1.0, FRET_HI));
            } else {
                painter.line_segment([Pos2::new(x, y0), Pos2::new(x, y1)], Stroke::new(1.5, Color32::from_gray(170)));
            }
        }
        let nut = mrect(nut_x - nut_w, nut_x, neck_top - 1.0, neck_bottom + 1.0);
        if real {
            painter.rect_filled(nut, 1.5, NUT);
            let edge_x = mirror(nut_x);
            painter.line_segment([Pos2::new(edge_x, nut.top()), Pos2::new(edge_x, nut.bottom())], Stroke::new(1.5, NUT_EDGE));
        } else {
            painter.rect_filled(nut, 0.0, Color32::from_rgb(235, 225, 200));
        }

        // ---------------------------------------------------------------- zone
        if let Some((lo, hi)) = self.zone {
            let x0 = if lo == 0 { nut_x - open_w } else { fret_x(lo.saturating_sub(1)) };
            let x1 = fret_x(hi.min(self.max_fret));
            let dim = shade(105);
            if x0 > nut_x - open_w + 0.5 {
                painter.rect_filled(mrect(nut_x - open_w, x0, neck_top, neck_bottom), 0.0, dim);
            }
            if x1 < neck_right - 0.5 {
                painter.rect_filled(mrect(x1, neck_right, neck_top, neck_bottom), 0.0, dim);
            }
            painter.rect_stroke(
                mrect(x0, x1, neck_top - 1.0, neck_bottom + 1.0),
                3.0,
                Stroke::new(1.5, Color32::from_white_alpha(150)),
                StrokeKind::Outside,
            );
        }

        // ---------------------------------------------------------------- strings
        let (sx0, sx1) = (nut_x - open_w, neck_right);
        for s in 0..6u8 {
            let y = string_y(s);
            let thick = 1.0 + (5 - s) as f32 * 0.42;
            let a = Pos2::new(mirror(sx0), y);
            let b = Pos2::new(mirror(sx1), y);
            if real {
                let wound = s <= 2;
                painter.line_segment([a + Vec2::new(0.0, 2.0), b + Vec2::new(0.0, 2.0)], Stroke::new(thick, shade(75)));
                painter.line_segment([a, b], Stroke::new(thick, if wound { WOUND } else { PLAIN }));
                if wound {
                    // winding: fine dark ticks across the string
                    let (l, rr) = (a.x.min(b.x), a.x.max(b.x));
                    let mut x = l;
                    while x < rr {
                        painter.line_segment(
                            [Pos2::new(x, y - thick * 0.5), Pos2::new(x + 1.2, y + thick * 0.5)],
                            Stroke::new(0.8, shade(70)),
                        );
                        x += 3.0;
                    }
                }
                painter.line_segment(
                    [a - Vec2::new(0.0, thick * 0.25), b - Vec2::new(0.0, thick * 0.25)],
                    Stroke::new((thick * 0.35).max(0.6), Color32::from_white_alpha(140)),
                );
            } else {
                painter.line_segment([a, b], Stroke::new(thick, Color32::from_gray(210)));
            }
        }

        // ---------------------------------------------------------------- labels
        let text_col = theme::TEXT;
        for f in 1..=self.max_fret {
            let x = mirror(fret_center_x(f));
            let strong = matches!(f % 12, 0 | 3 | 5 | 7 | 9);
            painter.text(
                Pos2::new(x, neck_bottom + 4.0),
                Align2::CENTER_TOP,
                f.to_string(),
                FontId::proportional(if strong { 14.0 } else { 11.5 }),
                if strong { text_col } else { theme::TEXT_WEAK },
            );
        }
        if let Some(names) = &self.string_names {
            for s in 0..6u8 {
                let x = if self.left_handed { rect.right() - name_w * 0.5 } else { rect.left() + name_w * 0.5 };
                painter.text(Pos2::new(x, string_y(s)), Align2::CENTER_CENTER, &names[s as usize], FontId::monospace(13.0), theme::TEXT_WEAK);
            }
        }

        // connecting lines (octave shapes ...)
        for (a, b, c) in self.lines {
            painter.line_segment([pt(*a), pt(*b)], Stroke::new(3.0, *c));
        }

        // ---------------------------------------------------------------- hover
        let hovered = resp.hover_pos().and_then(|p| {
            let x = mirror(p.x);
            if p.y < neck_top || p.y > neck_bottom || x < nut_x - open_w || x > neck_right {
                return None;
            }
            let fret = if x < nut_x {
                0
            } else {
                (((x - nut_x) / fw).floor() as i32 + 1).clamp(1, self.max_fret as i32) as u8
            };
            let s = ((bottom - p.y) / gap).round().clamp(0.0, 5.0);
            Some(Pos::new(s as u8, fret))
        });
        if self.clickable && let Some(h) = hovered {
            painter.circle_stroke(pt(h), r, Stroke::new(2.0, Color32::from_white_alpha(130)));
        }

        // ---------------------------------------------------------------- dots
        let ring = if maple { Color32::from_rgb(0x2A, 0x1D, 0x14) } else { Color32::from_rgb(0xF2, 0xEC, 0xE8) };
        for d in self.dots {
            let c = pt(d.pos);
            let rr = if d.emphasis { r * 1.12 } else { r };
            if d.hollow {
                painter.circle_stroke(c, rr * 0.8, Stroke::new(1.8, d.fill));
            } else {
                painter.circle_filled(c + Vec2::new(1.0, 1.8), rr + 0.5, shade(90));
                painter.circle_filled(c, rr, d.fill);
                painter.circle_stroke(c, rr, Stroke::new(1.2, shade(150)));
            }
            if d.emphasis {
                painter.circle_stroke(c, rr + 3.0, Stroke::new(2.5, ring));
            }
            if !d.label.is_empty() {
                let tc = if d.hollow { if maple { BLACK_DOT } else { text_col } } else { contrast_text(d.fill) };
                let size = if d.label.chars().count() > 2 { r * 0.85 } else { r * 1.05 };
                painter.text(c, Align2::CENTER_CENTER, &d.label, FontId::proportional(size), tc);
            }
        }

        if self.clickable && resp.clicked() { hovered } else { None }
    }
}

pub fn contrast_text(bg: Color32) -> Color32 {
    let l = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if l > 140.0 { Color32::BLACK } else { Color32::WHITE }
}

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
