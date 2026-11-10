//! Procedurally generated fretboard wood (no image files – our own code, nothing
//! to license). The texture is generated once per style and cached in egui memory.

use crate::settings::FretStyle;
use egui::{Color32, ColorImage, Context, Id, TextureHandle, TextureOptions};

const W: usize = 1024;
const H: usize = 128;

fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Smooth 2D value noise in 0..1.
fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (xf, yf) = (x - xi as f32, y - yi as f32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (u, v) = (s(xf), s(yf));
    let a = hash(xi, yi, seed);
    let b = hash(xi + 1, yi, seed);
    let c = hash(xi, yi + 1, seed);
    let d = hash(xi + 1, yi + 1, seed);
    let top = a + (b - a) * u;
    let bot = c + (d - c) * u;
    top + (bot - top) * v
}

fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

struct Palette {
    base: Color32,
    grain: Color32,
    light: Color32,
    pores: bool,
    contrast: f32,
}

fn palette(style: FretStyle) -> Palette {
    match style {
        FretStyle::Rosewood => Palette {
            base: Color32::from_rgb(0x3A, 0x24, 0x18),
            grain: Color32::from_rgb(0x1E, 0x11, 0x0B),
            light: Color32::from_rgb(0x5A, 0x3A, 0x28),
            pores: true,
            contrast: 1.0,
        },
        _ => Palette {
            base: Color32::from_rgb(0xE4, 0xC2, 0x8C),
            grain: Color32::from_rgb(0xC4, 0x97, 0x5A),
            light: Color32::from_rgb(0xF0, 0xD6, 0xA6),
            pores: false,
            contrast: 0.55,
        },
    }
}

fn generate(style: FretStyle) -> ColorImage {
    let p = palette(style);
    let mut px = Vec::with_capacity(W * H);
    for y in 0..H {
        for x in 0..W {
            let (fx, fy) = (x as f32, y as f32);
            // grain runs along the neck (x); slow warp makes the lines wander
            let warp = noise(fx * 0.004, 0.5, 7) * 6.0 + noise(fx * 0.015, 3.5, 9) * 1.5;
            let g1 = noise(fx * 0.01, (fy + warp * 4.0) * 0.18, 1);
            let g2 = noise(fx * 0.03, (fy + warp * 2.0) * 0.55, 2);
            let lines = ((fy * 0.9 + warp * 5.0 + g1 * 6.0).sin() * 0.5 + 0.5).powf(3.0);
            let dark = (g1 * 0.55 + g2 * 0.25 + lines * 0.35) * p.contrast;
            let mut c = lerp(p.base, p.grain, dark - 0.25);
            // occasional lighter streaks
            let streak = noise(fx * 0.006, fy * 0.08, 5);
            if streak > 0.62 {
                c = lerp(c, p.light, (streak - 0.62) * 2.2 * p.contrast);
            }
            // rosewood pores: short dark dashes along the grain
            if p.pores && hash(x as i32 / 7, y as i32, 11) > 0.985 {
                c = lerp(c, Color32::from_rgb(10, 5, 3), 0.55);
            }
            px.push(c);
        }
    }
    ColorImage::new([W, H], px)
}

/// The wood texture for a style (None for the flat Classic style).
pub fn texture(ctx: &Context, style: FretStyle) -> Option<TextureHandle> {
    if style == FretStyle::Classic {
        return None;
    }
    let id = Id::unique(("fretboard_wood", style as u8));
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(t);
    }
    let tex = ctx.load_texture(format!("wood-{style:?}"), generate(style), TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}
