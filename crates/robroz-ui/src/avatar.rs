//! Bro avatars: a shape (0–11) and a colour (0–9).
//!
//! The host stores only two indices (robroz#194, `Bro.avatar`); every
//! client draws the shapes itself. This module is the one place that
//! knows the shape outlines, the eye positions and the palette, so the
//! iced desktop client and the Slint mobile client draw the same bro.
//!
//! When a bro has no stored avatar the client derives one from its id:
//! `shape = h % 12` and `color = (h / 12) % 8`, where `h` is the 64-bit
//! FNV-1a hash of the id. The default only uses the eight hues; brown and
//! grey (8, 9) are picked by hand.
//!
//! The wire types are the contract's: `robroz_contract::BroAvatar` on
//! `Bro.avatar`, `CreateBro.avatar` and `PatchBro.avatar` (robroz#194,
//! `contract-v0.3.0`).

use robroz_tokens::base::avatar as tok;
use robroz_tokens::Rgb;

pub use robroz_contract::BroAvatar;

/// `code` of the 400 the host sends for an out-of-range avatar (robroz#194).
pub const INVALID_AVATAR: &str = "invalid_avatar";

/// Number of shapes (`shape` is `0..SHAPES`).
pub const SHAPES: u8 = robroz_contract::AVATAR_SHAPES;
/// Number of colours (`color` is `0..COLORS`).
pub const COLORS: u8 = robroz_contract::AVATAR_COLORS;
/// Colours a default avatar can get: the hues, not brown or grey.
pub const DEFAULT_HUES: u8 = tok::DEFAULT_HUES as u8;

fn fnv1a(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// The avatar a bro shows when none is stored.
pub fn default_for(bro_id: &str) -> BroAvatar {
    let h = fnv1a(bro_id);
    BroAvatar {
        shape: (h % u64::from(SHAPES)) as u8,
        color: ((h / u64::from(SHAPES)) % u64::from(DEFAULT_HUES)) as u8,
    }
}

/// The adapter every view uses: the stored avatar when there is a valid
/// one, otherwise the id-based default.
pub fn resolve(stored: Option<BroAvatar>, bro_id: &str) -> BroAvatar {
    stored
        .filter(BroAvatar::is_valid)
        .unwrap_or_else(|| default_for(bro_id))
}

const LIGHT: [Rgb; 10] = [
    tok::light::C0,
    tok::light::C1,
    tok::light::C2,
    tok::light::C3,
    tok::light::C4,
    tok::light::C5,
    tok::light::C6,
    tok::light::C7,
    tok::light::C8,
    tok::light::C9,
];
const DARK: [Rgb; 10] = [
    tok::dark::C0,
    tok::dark::C1,
    tok::dark::C2,
    tok::dark::C3,
    tok::dark::C4,
    tok::dark::C5,
    tok::dark::C6,
    tok::dark::C7,
    tok::dark::C8,
    tok::dark::C9,
];

/// Fill of colour `index` (out of range wraps). The same value is used for
/// the bro's name text, so it is ≥4.5:1 on that theme's window and
/// surface.
pub fn color(index: u8, dark: bool) -> Rgb {
    let set = if dark { &DARK } else { &LIGHT };
    set[usize::from(index) % set.len()]
}

/// Eye colour: white on the deeper light-theme fills, ink on the lighter
/// dark-theme fills.
pub fn eye_color(dark: bool) -> Rgb {
    if dark {
        tok::EYE_DARK
    } else {
        tok::EYE_LIGHT
    }
}

/// One outline step in a unit box (0..1, y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    /// Control point, then end point.
    Quad(f32, f32, f32, f32),
    /// Two control points, then end point.
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

/// Two round eyes in the unit box: centres and radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Eyes {
    pub left: (f32, f32),
    pub right: (f32, f32),
    pub r: f32,
}

/// Cubic factor for a quarter circle.
const K: f32 = 0.552_284_8;

fn circle(out: &mut Vec<Seg>, cx: f32, cy: f32, r: f32) {
    let k = r * K;
    out.push(Seg::Move(cx + r, cy));
    out.push(Seg::Cubic(cx + r, cy + k, cx + k, cy + r, cx, cy + r));
    out.push(Seg::Cubic(cx - k, cy + r, cx - r, cy + k, cx - r, cy));
    out.push(Seg::Cubic(cx - r, cy - k, cx - k, cy - r, cx, cy - r));
    out.push(Seg::Cubic(cx + k, cy - r, cx + r, cy - k, cx + r, cy));
    out.push(Seg::Close);
}

/// Polygon with every corner rounded by a quadratic of radius `r`
/// (clamped to half of the shorter neighbouring edge).
fn rounded_poly(out: &mut Vec<Seg>, pts: &[(f32, f32)], r: f32) {
    let n = pts.len();
    let toward = |a: (f32, f32), b: (f32, f32), d: f32| {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(f32::EPSILON);
        let d = d.min(len / 2.0);
        (a.0 + dx / len * d, a.1 + dy / len * d)
    };
    for i in 0..n {
        let prev = pts[(i + n - 1) % n];
        let cur = pts[i];
        let next = pts[(i + 1) % n];
        let a = toward(cur, prev, r);
        let b = toward(cur, next, r);
        if i == 0 {
            out.push(Seg::Move(a.0, a.1));
        } else {
            out.push(Seg::Line(a.0, a.1));
        }
        out.push(Seg::Quad(cur.0, cur.1, b.0, b.1));
    }
    out.push(Seg::Close);
}

/// `n` round bumps around a circle: base radius `r`, bumps reach `r + bump`.
fn scallop(out: &mut Vec<Seg>, n: usize, r: f32, bump: f32) {
    let at = |i: f32, rad: f32| {
        let a = std::f32::consts::TAU * i / n as f32 - std::f32::consts::FRAC_PI_2;
        (0.5 + rad * a.cos(), 0.5 + rad * a.sin())
    };
    let start = at(0.0, r);
    out.push(Seg::Move(start.0, start.1));
    for i in 0..n {
        let c = at(i as f32 + 0.5, r + bump * 2.0);
        let e = at(i as f32 + 1.0, r);
        out.push(Seg::Quad(c.0, c.1, e.0, e.1));
    }
    out.push(Seg::Close);
}

/// The outline of `shape` in a unit box. Out of range wraps.
pub fn outline(shape: u8) -> Vec<Seg> {
    let mut o = Vec::with_capacity(32);
    match shape % SHAPES {
        // Round.
        0 => circle(&mut o, 0.5, 0.5, 0.46),
        // Soft square.
        1 => rounded_poly(
            &mut o,
            &[(0.06, 0.06), (0.94, 0.06), (0.94, 0.94), (0.06, 0.94)],
            0.3,
        ),
        // Arch: half-round top, flat base.
        2 => {
            let (l, r, top, mid, base) = (0.1, 0.9, 0.06, 0.46, 0.94);
            let k = 0.4 * K;
            o.push(Seg::Move(l, base - 0.08));
            o.push(Seg::Line(l, mid));
            o.push(Seg::Cubic(l, mid - k, 0.5 - k, top, 0.5, top));
            o.push(Seg::Cubic(0.5 + k, top, r, mid - k, r, mid));
            o.push(Seg::Line(r, base - 0.08));
            o.push(Seg::Quad(r, base, r - 0.08, base));
            o.push(Seg::Line(l + 0.08, base));
            o.push(Seg::Quad(l, base, l, base - 0.08));
            o.push(Seg::Close);
        }
        // Drop, point up.
        3 => {
            o.push(Seg::Move(0.5, 0.04));
            o.push(Seg::Cubic(0.62, 0.2, 0.9, 0.4, 0.9, 0.62));
            o.push(Seg::Cubic(0.9, 0.84, 0.72, 0.96, 0.5, 0.96));
            o.push(Seg::Cubic(0.28, 0.96, 0.1, 0.84, 0.1, 0.62));
            o.push(Seg::Cubic(0.1, 0.4, 0.38, 0.2, 0.5, 0.04));
            o.push(Seg::Close);
        }
        // Hexagon, flat top.
        4 => rounded_poly(
            &mut o,
            &[
                (0.27, 0.1),
                (0.73, 0.1),
                (0.97, 0.5),
                (0.73, 0.9),
                (0.27, 0.9),
                (0.03, 0.5),
            ],
            0.12,
        ),
        // Flower: six deep petals.
        5 => scallop(&mut o, 6, 0.36, 0.07),
        // Cookie: twelve small bumps.
        6 => scallop(&mut o, 12, 0.42, 0.035),
        // Ghost: round top, wavy hem.
        7 => {
            let (l, r, mid, hem) = (0.1, 0.9, 0.46, 0.94);
            let k = 0.4 * K;
            o.push(Seg::Move(l, hem));
            o.push(Seg::Line(l, mid));
            o.push(Seg::Cubic(l, mid - k, 0.5 - k, 0.06, 0.5, 0.06));
            o.push(Seg::Cubic(0.5 + k, 0.06, r, mid - k, r, mid));
            o.push(Seg::Line(r, hem));
            let w = (r - l) / 3.0;
            for i in 0..3 {
                let x1 = r - w * i as f32;
                let x0 = x1 - w;
                o.push(Seg::Quad((x0 + x1) / 2.0, hem - 0.14, x0, hem));
            }
            o.push(Seg::Close);
        }
        // Diamond.
        8 => rounded_poly(
            &mut o,
            &[(0.5, 0.02), (0.98, 0.5), (0.5, 0.98), (0.02, 0.5)],
            0.12,
        ),
        // Pill, lying down.
        9 => rounded_poly(
            &mut o,
            &[(0.02, 0.2), (0.98, 0.2), (0.98, 0.8), (0.02, 0.8)],
            0.3,
        ),
        // Triangle, point up.
        10 => rounded_poly(&mut o, &[(0.5, 0.06), (0.97, 0.9), (0.03, 0.9)], 0.16),
        // Star, five points.
        _ => {
            let mut pts = Vec::with_capacity(10);
            for i in 0..10 {
                let a = std::f32::consts::TAU * i as f32 / 10.0 - std::f32::consts::FRAC_PI_2;
                let rad = if i % 2 == 0 { 0.5 } else { 0.27 };
                pts.push((0.5 + rad * a.cos(), 0.54 + rad * a.sin()));
            }
            rounded_poly(&mut o, &pts, 0.06);
        }
    }
    o
}

/// Where the two eyes sit on `shape`.
pub fn eyes(shape: u8) -> Eyes {
    let r = tok::EYE_RADIUS;
    let (y, dx, r) = match shape % SHAPES {
        3 => (0.62, 0.13, r),
        9 => (0.5, 0.14, r),
        10 => (0.64, 0.1, r),
        11 => (0.56, 0.075, r * 0.85),
        2 | 7 => (0.44, 0.14, r),
        _ => (0.46, 0.14, r),
    };
    Eyes {
        left: (0.5 - dx, y),
        right: (0.5 + dx, y),
        r,
    }
}

fn num(v: f32) -> String {
    let s = format!("{:.2}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// SVG path data for `shape` in a `size`×`size` box (Slint `Path`).
pub fn svg_outline(shape: u8, size: f32) -> String {
    let p = |v: f32| num(v * size);
    let mut s = String::new();
    for seg in outline(shape) {
        let part = match seg {
            Seg::Move(x, y) => format!("M {} {} ", p(x), p(y)),
            Seg::Line(x, y) => format!("L {} {} ", p(x), p(y)),
            Seg::Quad(cx, cy, x, y) => format!("Q {} {} {} {} ", p(cx), p(cy), p(x), p(y)),
            Seg::Cubic(a, b, c, d, x, y) => {
                format!("C {} {} {} {} {} {} ", p(a), p(b), p(c), p(d), p(x), p(y))
            }
            Seg::Close => "Z ".to_string(),
        };
        s.push_str(&part);
    }
    s.trim_end().to_string()
}

/// SVG path data for both eyes of `shape` in a `size`×`size` box.
pub fn svg_eyes(shape: u8, size: f32) -> String {
    let e = eyes(shape);
    let mut o = Vec::new();
    circle(&mut o, e.left.0, e.left.1, e.r);
    circle(&mut o, e.right.0, e.right.1, e.r);
    let p = |v: f32| num(v * size);
    o.into_iter()
        .map(|seg| match seg {
            Seg::Move(x, y) => format!("M {} {}", p(x), p(y)),
            Seg::Cubic(a, b, c, d, x, y) => {
                format!("C {} {} {} {} {} {}", p(a), p(b), p(c), p(d), p(x), p(y))
            }
            Seg::Close => "Z".to_string(),
            Seg::Line(x, y) => format!("L {} {}", p(x), p(y)),
            Seg::Quad(cx, cy, x, y) => format!("Q {} {} {} {}", p(cx), p(cy), p(x), p(y)),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Title-capsule stack: at most [`title_max`] avatars, then `+N`.
/// Returns the ids to draw and how many are left over.
pub fn stack<'a>(ids: &'a [String]) -> (&'a [String], usize) {
    let max = tok::TITLE_MAX as usize;
    if ids.len() <= max {
        (ids, 0)
    } else {
        (&ids[..max], ids.len() - max)
    }
}

/// Width of a stack of `n` avatars of `size` that overlap by `overlap`.
pub fn stack_width(n: usize, size: f32, overlap: f32) -> f32 {
    if n == 0 {
        0.0
    } else {
        size + (n as f32 - 1.0) * (size - overlap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(c: Rgb) -> f64 {
        let ch = |v: u8| {
            let x = f64::from(v) / 255.0;
            if x <= 0.039_28 {
                x / 12.92
            } else {
                ((x + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
    }

    fn ratio(a: Rgb, b: Rgb) -> f64 {
        let (x, y) = (lum(a), lum(b));
        let (hi, lo) = if x > y { (x, y) } else { (y, x) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn counts_match_the_contract_and_tokens() {
        assert_eq!(SHAPES, 12);
        assert_eq!(COLORS, 10);
        assert_eq!(tok::SHAPES as u8, SHAPES);
        assert_eq!(tok::COLORS as u8, COLORS);
        assert_eq!(DEFAULT_HUES, 8);
        assert!(robroz_contract::error_codes::ERROR_CODES
            .iter()
            .any(|(_, code)| *code == INVALID_AVATAR));
    }

    #[test]
    fn default_is_stable_and_only_uses_hues() {
        let a = default_for("bot-ada");
        assert_eq!(a, default_for("bot-ada"));
        for i in 0..500 {
            let d = default_for(&format!("bro-{i}"));
            assert!(d.is_valid());
            assert!(d.color < DEFAULT_HUES, "{d:?}");
        }
        // The hash spreads: many distinct pairs over a few hundred ids.
        let distinct: std::collections::HashSet<_> =
            (0..500).map(|i| default_for(&format!("bro-{i}"))).collect();
        assert!(distinct.len() > 60, "{}", distinct.len());
    }

    #[test]
    fn resolve_prefers_a_valid_stored_avatar() {
        let av = |shape, color| BroAvatar { shape, color };
        let stored = av(11, 9);
        assert_eq!(resolve(Some(stored), "x"), stored);
        assert_eq!(resolve(None, "x"), default_for("x"));
        assert_eq!(resolve(Some(av(12, 0)), "x"), default_for("x"));
        assert_eq!(resolve(Some(av(0, 10)), "x"), default_for("x"));
    }

    #[test]
    fn name_text_contrast_on_both_themes() {
        let light = crate::Skin::light().palette;
        let dark = crate::Skin::dark().palette;
        for i in 0..COLORS {
            for bg in [light.window, light.surface] {
                assert!(ratio(color(i, false), bg) >= 4.5, "light c{i}");
            }
            for bg in [dark.window, dark.surface] {
                assert!(ratio(color(i, true), bg) >= 4.5, "dark c{i}");
            }
            // Eyes are graphics: ≥3:1 on the fill.
            assert!(ratio(eye_color(false), color(i, false)) >= 3.0);
            assert!(ratio(eye_color(true), color(i, true)) >= 3.0);
        }
    }

    #[test]
    fn palette_avoids_status_and_brand_colours_and_white() {
        let avoid: [Rgb; 4] = [
            robroz_tokens::BRAND_ORANGE,
            robroz_tokens::base::status_dot::light::DONE,
            robroz_tokens::base::status_dot::light::BLOCKED,
            [0xff, 0xff, 0xff],
        ];
        for i in 0..COLORS {
            for dark in [false, true] {
                assert!(!avoid.contains(&color(i, dark)));
            }
        }
    }

    #[test]
    fn every_shape_is_closed_and_inside_the_box() {
        for s in 0..SHAPES {
            let o = outline(s);
            assert!(matches!(o.first(), Some(Seg::Move(..))), "shape {s}");
            assert_eq!(o.last(), Some(&Seg::Close), "shape {s}");
            for seg in &o {
                let pts: Vec<(f32, f32)> = match *seg {
                    Seg::Move(x, y) | Seg::Line(x, y) => vec![(x, y)],
                    Seg::Quad(_, _, x, y) | Seg::Cubic(_, _, _, _, x, y) => vec![(x, y)],
                    Seg::Close => vec![],
                };
                for (x, y) in pts {
                    assert!((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y), "{s}");
                }
            }
            let e = eyes(s);
            assert!(e.left.0 < e.right.0 && e.r > 0.0);
            assert!(svg_outline(s, 100.0).starts_with('M'));
            assert_eq!(svg_eyes(s, 100.0).matches('M').count(), 2);
        }
    }

    #[test]
    fn stack_caps_at_three_then_counts_the_rest() {
        let ids: Vec<String> = (0..5).map(|i| i.to_string()).collect();
        let (shown, more) = stack(&ids);
        assert_eq!((shown.len(), more), (3, 2));
        let (shown, more) = stack(&ids[..3]);
        assert_eq!((shown.len(), more), (3, 0));
        assert_eq!(stack_width(3, 24.0, 8.0), 56.0);
        assert_eq!(stack_width(0, 24.0, 8.0), 0.0);
    }
}
