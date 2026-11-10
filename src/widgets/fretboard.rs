//! Horizontal fretboard: high e string on top (like tab), low string at the bottom.

use crate::theory::fretboard::Pos;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

#[derive(Clone, Debug)]
pub struct Dot {
    pub pos: Pos,
    pub label: String,
    pub fill: Color32,
    /// Draw only an outline (e.g. "hidden" or "hint" notes).
    pub hollow: bool,
    /// Bigger with a white ring (current note, the question, ...).
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
    /// Shaded fret range (inclusive), e.g. the zone of a drill.
    pub zone: Option<(u8, u8)>,
    /// Per-position background tint (heatmap).
    pub tint: Option<&'a dyn Fn(Pos) -> Option<Color32>>,
    pub left_handed: bool,
    pub clickable: bool,
    pub height: f32,
    pub string_names: Option<[String; 6]>,
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
            height: 210.0,
            string_names: None,
        }
    }

    /// Draws the neck; returns the clicked position, if any.
    pub fn show(self, ui: &mut Ui) -> Option<Pos> {
        let width = ui.available_width().max(300.0);
        let sense = if self.clickable {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (resp, painter) = ui.allocate_painter(Vec2::new(width, self.height), sense);
        let rect = resp.rect;
        let dark = ui.visuals().dark_mode;

        let name_w = 26.0;
        let open_w = 34.0;
        let numbers_h = 20.0;
        // the neck (wood) spans neck_top..neck_bottom; strings sit half a gap inside it
        let neck_top = rect.top() + 2.0;
        let neck_bottom = rect.bottom() - numbers_h;
        let gap = (neck_bottom - neck_top) / 6.0;
        let top = neck_top + gap * 0.5;
        let bottom = neck_bottom - gap * 0.5;
        let nut_x = rect.left() + name_w + open_w;
        let frets = self.max_fret.max(1) as f32;
        let fw = (rect.right() - 8.0 - nut_x) / frets;

        let mirror = |x: f32| {
            if self.left_handed {
                rect.left() + rect.right() - x
            } else {
                x
            }
        };
        // string 0 (lowest) at the bottom
        let string_y = |s: u8| bottom - s as f32 * gap;
        let fret_center_x = |f: u8| {
            if f == 0 {
                nut_x - open_w * 0.5
            } else {
                nut_x + (f as f32 - 0.5) * fw
            }
        };
        let pt = |p: Pos| Pos2::new(mirror(fret_center_x(p.fret)), string_y(p.string));

        // wood
        let wood = if dark {
            Color32::from_rgb(58, 40, 28)
        } else {
            Color32::from_rgb(222, 196, 160)
        };
        let neck = Rect::from_min_max(
            Pos2::new(mirror(nut_x).min(mirror(nut_x + frets * fw)), neck_top),
            Pos2::new(mirror(nut_x).max(mirror(nut_x + frets * fw)), neck_bottom),
        );
        painter.rect_filled(neck, 3.0, wood);

        // zone
        if let Some((lo, hi)) = self.zone {
            let x0 = if lo == 0 { nut_x - open_w } else { nut_x + (lo as f32 - 1.0) * fw };
            let x1 = nut_x + hi as f32 * fw;
            let (a, b) = (mirror(x0), mirror(x1));
            let r = Rect::from_min_max(
                Pos2::new(a.min(b), neck_top + 1.0),
                Pos2::new(a.max(b), neck_bottom - 1.0),
            );
            painter.rect_filled(r, 4.0, Color32::from_rgba_unmultiplied(80, 160, 255, 40));
            painter.rect_stroke(
                r,
                4.0,
                Stroke::new(1.5, Color32::from_rgb(80, 160, 255)),
                StrokeKind::Inside,
            );
        }

        // heatmap tint
        if let Some(tint) = self.tint {
            for s in 0..6u8 {
                for f in 0..=self.max_fret {
                    if let Some(c) = tint(Pos::new(s, f)) {
                        let w = if f == 0 { open_w } else { fw };
                        let r = Rect::from_center_size(pt(Pos::new(s, f)), Vec2::new(w - 2.0, gap - 2.0));
                        painter.rect_filled(r, 3.0, c);
                    }
                }
            }
        }

        // inlays
        let inlay = if dark {
            Color32::from_gray(150)
        } else {
            Color32::from_gray(250)
        };
        let mid_y = (top + bottom) * 0.5;
        for f in 1..=self.max_fret {
            let x = mirror(fret_center_x(f));
            match f % 12 {
                3 | 5 | 7 | 9 => {
                    painter.circle_filled(Pos2::new(x, mid_y), 5.0, inlay);
                }
                0 => {
                    painter.circle_filled(Pos2::new(x, mid_y - gap), 5.0, inlay);
                    painter.circle_filled(Pos2::new(x, mid_y + gap), 5.0, inlay);
                }
                _ => {}
            }
        }

        // frets + nut
        let fret_col = if dark {
            Color32::from_gray(170)
        } else {
            Color32::from_gray(120)
        };
        for f in 0..=self.max_fret {
            let x = mirror(nut_x + f as f32 * fw);
            let w = if f == 0 { 5.0 } else { 1.5 };
            let c = if f == 0 {
                Color32::from_rgb(235, 225, 200)
            } else {
                fret_col
            };
            painter.line_segment(
                [Pos2::new(x, neck_top), Pos2::new(x, neck_bottom)],
                Stroke::new(w, c),
            );
        }
        // fret numbers
        let text_col = ui.visuals().text_color();
        for f in 1..=self.max_fret {
            let x = mirror(fret_center_x(f));
            let strong = matches!(f % 12, 0 | 3 | 5 | 7 | 9);
            painter.text(
                Pos2::new(x, rect.bottom() - 2.0),
                Align2::CENTER_BOTTOM,
                f.to_string(),
                FontId::proportional(if strong { 13.0 } else { 11.0 }),
                if strong { text_col } else { text_col.gamma_multiply(0.6) },
            );
        }

        // strings
        let string_col = if dark {
            Color32::from_gray(210)
        } else {
            Color32::from_gray(90)
        };
        let names = self.string_names.clone();
        for s in 0..6u8 {
            let y = string_y(s);
            let thick = 1.0 + (5 - s) as f32 * 0.35;
            let (x0, x1) = (mirror(nut_x - open_w), mirror(nut_x + frets * fw));
            painter.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], Stroke::new(thick, string_col));
            if let Some(n) = &names {
                let x = if self.left_handed {
                    rect.right() - name_w * 0.5
                } else {
                    rect.left() + name_w * 0.5
                };
                painter.text(
                    Pos2::new(x, y),
                    Align2::CENTER_CENTER,
                    &n[s as usize],
                    FontId::monospace(13.0),
                    text_col,
                );
            }
        }

        // connecting lines (octave shapes ...)
        for (a, b, c) in self.lines {
            painter.line_segment([pt(*a), pt(*b)], Stroke::new(3.0, *c));
        }

        // hover
        let hovered = resp.hover_pos().and_then(|p| {
            let x = mirror(p.x);
            let fret = if x < nut_x {
                if x < nut_x - open_w {
                    return None;
                }
                0
            } else {
                (((x - nut_x) / fw).floor() as i32 + 1).clamp(1, self.max_fret as i32) as u8
            };
            let s = ((bottom - p.y) / gap).round();
            if !(0.0..=5.0).contains(&s) || (p.y < neck_top) || (p.y > neck_bottom) {
                return None;
            }
            Some(Pos::new(s as u8, fret))
        });
        if self.clickable
            && let Some(h) = hovered
        {
            painter.circle_stroke(pt(h), (gap.min(fw) * 0.42).max(8.0), Stroke::new(2.0, text_col.gamma_multiply(0.5)));
        }

        // dots
        let r = (gap.min(fw) * 0.42).clamp(7.0, 16.0);
        for d in self.dots {
            let c = pt(d.pos);
            let rr = if d.emphasis { r * 1.15 } else { r };
            if d.hollow {
                painter.circle_stroke(c, rr * 0.8, Stroke::new(1.6, d.fill));
            } else {
                painter.circle_filled(c, rr, d.fill);
            }
            if d.emphasis {
                painter.circle_stroke(c, rr + 2.5, Stroke::new(2.5, Color32::WHITE));
            }
            if !d.label.is_empty() {
                let tc = if d.hollow { text_col } else { contrast_text(d.fill) };
                let size = if d.label.chars().count() > 2 { r * 0.85 } else { r * 1.05 };
                painter.text(c, Align2::CENTER_CENTER, &d.label, FontId::proportional(size), tc);
            }
        }

        if self.clickable && resp.clicked() { hovered } else { None }
    }
}

pub fn contrast_text(bg: Color32) -> Color32 {
    let l = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if l > 140.0 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

pub fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
