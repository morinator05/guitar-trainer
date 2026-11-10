//! Dark theme with a muted, pastel dark-red accent.
//!
//! Rule: the accent is a UI colour only. It is never used on the fretboard, so a
//! red root note can never be mistaken for a selected button.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Theme, ThemePreference};

const fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

pub const PANEL: Color32 = hex(0x1B1819);
pub const SIDEBAR: Color32 = hex(0x151213);
pub const WINDOW: Color32 = hex(0x231F20);
pub const EXTREME: Color32 = hex(0x100E0F);
pub const FAINT: Color32 = hex(0x211D1E);
pub const W_INACTIVE: Color32 = hex(0x322C2D);
pub const W_HOVERED: Color32 = hex(0x372F30);
pub const W_ACTIVE: Color32 = hex(0x433839);
pub const STROKE: Color32 = hex(0x4A4243);
/// Outline of every interactive widget: >= 3:1 against the panel.
pub const OUTLINE: Color32 = hex(0x6E6566);

pub const ACCENT: Color32 = hex(0xB86B6F);
pub const ACCENT_HOVER: Color32 = hex(0xCC8588);
pub const SELECTION: Color32 = hex(0x8A4549);
pub const SELECTION_TEXT: Color32 = hex(0xF2DADB);

pub const TEXT: Color32 = hex(0xE7DFDE);
pub const TEXT_STRONG: Color32 = hex(0xF6F0EF);
pub const TEXT_WEAK: Color32 = hex(0xB3A8A7);
pub const LINK: Color32 = hex(0xD79A9D);

/// Right/wrong: blue vs orange stays distinct for red-green colour blindness,
/// and is never the only cue (✔/✖ glyphs, filled vs outlined).
pub const SUCCESS: Color32 = hex(0x5FA8E8);
pub const ERROR: Color32 = hex(0xE8A33C);
/// Fill of the main action button (white text >= 4.5:1).
pub const PRIMARY: Color32 = hex(0x8A4549);
pub const WARN: Color32 = hex(0xD8B26A);

/// Fill of a "question" dot on the fretboard: neutral, says nothing about the answer.
pub const QUESTION: Color32 = hex(0xEDE6E4);
/// Neutral marker for notes the user placed (sight-read drill) before checking.
pub const MARKER: Color32 = hex(0xC9C1BF);
/// Neutral grey for context notes.
pub const NEUTRAL: Color32 = hex(0x8E8586);

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(ThemePreference::Dark);
    ctx.style_mut_of(Theme::Dark, |s| {
        let v = &mut s.visuals;
        v.dark_mode = true;
        v.panel_fill = PANEL;
        v.window_fill = WINDOW;
        v.window_stroke = Stroke::new(1.0, STROKE);
        v.extreme_bg_color = EXTREME;
        v.faint_bg_color = FAINT;
        v.code_bg_color = EXTREME;
        v.hyperlink_color = LINK;
        v.warn_fg_color = WARN;
        v.error_fg_color = ERROR;
        v.weak_text_color = Some(TEXT_WEAK);
        v.override_text_color = None;
        v.window_corner_radius = CornerRadius::same(8);
        v.menu_corner_radius = CornerRadius::same(6);
        v.slider_trailing_fill = true;
        v.interact_cursor = Some(egui::CursorIcon::PointingHand);
        v.striped = true;
        v.selection.bg_fill = SELECTION;
        v.selection.stroke = Stroke::new(1.5, SELECTION_TEXT);

        let r = CornerRadius::same(6);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = PANEL;
        w.noninteractive.weak_bg_fill = PANEL;
        w.noninteractive.bg_stroke = Stroke::new(1.0, STROKE);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
        w.noninteractive.corner_radius = r;

        w.inactive.bg_fill = W_INACTIVE;
        w.inactive.weak_bg_fill = W_INACTIVE;
        w.inactive.bg_stroke = Stroke::new(1.0, OUTLINE);
        w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        w.inactive.corner_radius = r;

        w.hovered.bg_fill = W_HOVERED;
        w.hovered.weak_bg_fill = W_HOVERED;
        // hover: visible outline, but weaker than the selected state
        w.hovered.bg_stroke = Stroke::new(1.5, TEXT_WEAK);
        w.hovered.fg_stroke = Stroke::new(1.5, TEXT_STRONG);
        w.hovered.corner_radius = r;
        w.hovered.expansion = 1.0;

        w.active.bg_fill = W_ACTIVE;
        w.active.weak_bg_fill = W_ACTIVE;
        w.active.bg_stroke = Stroke::new(2.0, TEXT_STRONG);
        w.active.fg_stroke = Stroke::new(2.0, TEXT_STRONG);
        w.active.corner_radius = r;

        w.open.bg_fill = W_ACTIVE;
        w.open.weak_bg_fill = W_ACTIVE;
        w.open.bg_stroke = Stroke::new(1.0, ACCENT);
        w.open.fg_stroke = Stroke::new(1.0, TEXT_STRONG);
        w.open.corner_radius = r;

        let sp = &mut s.spacing;
        sp.item_spacing = egui::vec2(8.0, 6.0);
        sp.button_padding = egui::vec2(10.0, 5.0);
        sp.slider_width = 180.0;
        sp.interact_size.y = 24.0;

        use FontFamily::{Monospace, Proportional};
        s.text_styles = [
            (TextStyle::Small, FontId::new(13.5, Proportional)),
            (TextStyle::Body, FontId::new(16.0, Proportional)),
            (TextStyle::Button, FontId::new(16.0, Proportional)),
            (TextStyle::Heading, FontId::new(22.0, Proportional)),
            (TextStyle::Monospace, FontId::new(14.0, Monospace)),
        ]
        .into();
    });
}
