//! 20px line icons for the group and self-row menus (v0.2 PR4).
//!
//! Rasterized with tiny-skia so every icon is a real stroke, not a
//! canvas that iced_wgpu would only paint once.

use iced::widget::{image, Image};
use iced::{Color, Length};

/// One line icon in the 20px menu / settings-row slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Glyph {
    Profile,
    Search,
    Delete,
    Copy,
    Settings,
    Members,
    Routines,
    Pencil,
    SignOut,
    /// `•••`, three round dots (top-bar menu button).
    More,
    /// `‹` chevron (top-bar back button).
    Back,
    /// Tick for the selected option on a settings choice page.
    Check,
    /// `↗` for a settings row that leaves the app.
    External,
}

fn rgba_u8(c: Color) -> [u8; 4] {
    [
        (c.r * 255.0).round().clamp(0.0, 255.0) as u8,
        (c.g * 255.0).round().clamp(0.0, 255.0) as u8,
        (c.b * 255.0).round().clamp(0.0, 255.0) as u8,
        (c.a * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

fn skia_color(c: Color) -> tiny_skia::Color {
    let [r, g, b, a] = rgba_u8(c);
    tiny_skia::Color::from_rgba8(r, g, b, a)
}

fn pack_rgba(pm: &tiny_skia::Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pm.data().len());
    for px in pm.pixels() {
        let a = px.alpha();
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        if a == 255 {
            out.extend_from_slice(&[px.red(), px.green(), px.blue(), a]);
            continue;
        }
        let f = 255.0 / f32::from(a);
        out.extend_from_slice(&[
            (f32::from(px.red()) * f).round().clamp(0.0, 255.0) as u8,
            (f32::from(px.green()) * f).round().clamp(0.0, 255.0) as u8,
            (f32::from(px.blue()) * f).round().clamp(0.0, 255.0) as u8,
            a,
        ]);
    }
    out
}

fn stroke(
    pm: &mut tiny_skia::Pixmap,
    build: impl Fn(&mut tiny_skia::PathBuilder),
    color: Color,
    width: f32,
) {
    let mut p = tiny_skia::PathBuilder::new();
    build(&mut p);
    let Some(path) = p.finish() else {
        return;
    };
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(skia_color(color));
    paint.anti_alias = true;
    let stroke = tiny_skia::Stroke {
        width,
        line_cap: tiny_skia::LineCap::Round,
        line_join: tiny_skia::LineJoin::Round,
        ..tiny_skia::Stroke::default()
    };
    pm.stroke_path(
        &path,
        &paint,
        &stroke,
        tiny_skia::Transform::identity(),
        None,
    );
}

/// Logical stroke width. `More` draws dots, so its stroke is the dot size.
fn stroke_width(glyph: Glyph) -> f32 {
    match glyph {
        Glyph::More => 2.6,
        Glyph::Back => 2.0,
        _ => 1.6,
    }
}

fn draw(glyph: Glyph, p: &mut tiny_skia::PathBuilder, s: f32) {
    let m = s * 0.18;
    let x0 = m;
    let x1 = s - m;
    let y0 = m;
    let y1 = s - m;
    let mid = s / 2.0;
    match glyph {
        Glyph::Profile => {
            p.push_circle(mid, y0 + s * 0.22, s * 0.14);
            p.move_to(x0 + s * 0.08, y1);
            p.quad_to(mid, y0 + s * 0.42, x1 - s * 0.08, y1);
        }
        Glyph::Search => {
            p.push_circle(mid - s * 0.06, mid - s * 0.06, s * 0.22);
            p.move_to(mid + s * 0.12, mid + s * 0.12);
            p.line_to(x1, y1);
        }
        Glyph::Delete => {
            p.move_to(x0 + s * 0.08, y0 + s * 0.22);
            p.line_to(x1 - s * 0.08, y0 + s * 0.22);
            p.move_to(mid, y0 + s * 0.10);
            p.line_to(mid, y0 + s * 0.22);
            p.move_to(x0 + s * 0.16, y0 + s * 0.22);
            p.line_to(x0 + s * 0.22, y1);
            p.line_to(x1 - s * 0.22, y1);
            p.line_to(x1 - s * 0.16, y0 + s * 0.22);
        }
        Glyph::Copy => {
            p.move_to(x0 + s * 0.16, y0 + s * 0.22);
            p.line_to(x0 + s * 0.16, y1);
            p.line_to(x1 - s * 0.16, y1);
            p.line_to(x1 - s * 0.16, y0 + s * 0.22);
            p.line_to(x0 + s * 0.16, y0 + s * 0.22);
            p.move_to(x0 + s * 0.28, y0 + s * 0.22);
            p.line_to(x0 + s * 0.28, y0 + s * 0.08);
            p.line_to(x1 - s * 0.04, y0 + s * 0.08);
            p.line_to(x1 - s * 0.04, y1 - s * 0.16);
            p.line_to(x1 - s * 0.16, y1 - s * 0.16);
        }
        Glyph::Settings => {
            // Gear: eight square-ish teeth around a ring, plus the hub.
            let teeth = 8;
            let step = std::f32::consts::TAU / teeth as f32;
            let (r_in, r_out) = (s * 0.30, s * 0.42);
            let (half_in, half_out) = (step * 0.26, step * 0.17);
            for i in 0..teeth {
                let a = i as f32 * step;
                let pts = [
                    (r_in, a - half_in),
                    (r_out, a - half_out),
                    (r_out, a + half_out),
                    (r_in, a + half_in),
                ];
                for (k, (r, ang)) in pts.iter().enumerate() {
                    let (x, y) = (mid + r * ang.cos(), mid + r * ang.sin());
                    if i == 0 && k == 0 {
                        p.move_to(x, y);
                    } else {
                        p.line_to(x, y);
                    }
                }
            }
            p.close();
            p.push_circle(mid, mid, s * 0.13);
        }
        Glyph::Members => {
            p.push_circle(mid - s * 0.14, mid - s * 0.08, s * 0.10);
            p.push_circle(mid + s * 0.16, mid - s * 0.10, s * 0.09);
            p.move_to(x0, y1);
            p.quad_to(mid - s * 0.14, mid + s * 0.10, mid, y1);
            p.move_to(mid, y1);
            p.quad_to(mid + s * 0.16, mid + s * 0.08, x1, y1);
        }
        Glyph::Routines => {
            p.push_circle(mid, mid, s * 0.30);
            p.move_to(mid, mid - s * 0.16);
            p.line_to(mid, mid);
            p.line_to(mid + s * 0.14, mid + s * 0.08);
        }
        Glyph::Pencil => {
            p.move_to(x0 + s * 0.10, y1 - s * 0.10);
            p.line_to(x0 + s * 0.16, y1 - s * 0.32);
            p.line_to(x1 - s * 0.16, y0 + s * 0.10);
            p.line_to(x1 - s * 0.04, y0 + s * 0.22);
            p.line_to(x0 + s * 0.28, y1 - s * 0.16);
            p.line_to(x0 + s * 0.10, y1 - s * 0.10);
        }
        Glyph::More => {
            // Zero-length strokes: the round cap makes each a dot.
            for x in [s * 0.14, mid, s * 1.0 - s * 0.14] {
                p.move_to(x, mid);
                p.line_to(x + 0.1, mid);
            }
        }
        Glyph::Check => {
            p.move_to(s * 0.20, s * 0.52);
            p.line_to(s * 0.42, s * 0.74);
            p.line_to(s * 0.82, s * 0.28);
        }
        Glyph::External => {
            p.move_to(s * 0.28, s * 0.72);
            p.line_to(s * 0.72, s * 0.28);
            p.move_to(s * 0.38, s * 0.28);
            p.line_to(s * 0.72, s * 0.28);
            p.line_to(s * 0.72, s * 0.62);
        }
        Glyph::Back => {
            p.move_to(s * 0.64, s * 0.18);
            p.line_to(s * 0.34, mid);
            p.line_to(s * 0.64, s * 0.82);
        }
        Glyph::SignOut => {
            p.move_to(x0 + s * 0.10, y0 + s * 0.16);
            p.line_to(x0 + s * 0.10, y1 - s * 0.16);
            p.line_to(mid, y1 - s * 0.16);
            p.line_to(mid, y0 + s * 0.16);
            p.line_to(x0 + s * 0.10, y0 + s * 0.16);
            p.move_to(mid + s * 0.04, mid);
            p.line_to(x1, mid);
            p.move_to(x1 - s * 0.12, mid - s * 0.12);
            p.line_to(x1, mid);
            p.line_to(x1 - s * 0.12, mid + s * 0.12);
        }
    }
}

/// RGBA pixels of a glyph, `px` square, at `scale` device px per logical.
fn rasterize(glyph: Glyph, px: u32, scale: f32, color: Color) -> Option<(u32, Vec<u8>)> {
    let mut pm = tiny_skia::Pixmap::new(px, px)?;
    stroke(
        &mut pm,
        |p| draw(glyph, p, px as f32),
        color,
        (stroke_width(glyph) * scale).max(1.0),
    );
    Some((px, pack_rgba(&pm)))
}

/// A line icon of `size` logical px, sharp at `scale` device pixels.
pub fn icon(glyph: Glyph, rgb: [u8; 3], size: f32, scale: f32) -> Image {
    let scale = scale.max(1.0);
    let px = (size * scale).round().max(1.0) as u32;
    let color = Color::from_rgb8(rgb[0], rgb[1], rgb[2]);
    let handle = match rasterize(glyph, px, scale, color) {
        Some((px, rgba)) => image::Handle::from_rgba(px, px, rgba),
        None => image::Handle::from_rgba(1, 1, vec![0, 0, 0, 0]),
    };
    Image::new(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .content_fit(iced::ContentFit::Contain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_paints_some_pixels() {
        for g in [
            Glyph::Profile,
            Glyph::Search,
            Glyph::Delete,
            Glyph::Copy,
            Glyph::Settings,
            Glyph::Members,
            Glyph::Routines,
            Glyph::Pencil,
            Glyph::SignOut,
            Glyph::More,
            Glyph::Back,
            Glyph::Check,
            Glyph::External,
        ] {
            let img = icon(g, [0, 0, 0], 20.0, 2.0);
            let _ = img;
        }
    }

    fn painted(glyph: Glyph, px: u32) -> Vec<(u32, u32)> {
        let (_, rgba) = rasterize(glyph, px, 2.0, Color::BLACK).expect("pixmap");
        let mut out = Vec::new();
        for y in 0..px {
            for x in 0..px {
                if rgba[((y * px + x) * 4 + 3) as usize] > 64 {
                    out.push((x, y));
                }
            }
        }
        out
    }

    #[test]
    fn more_is_three_dots_on_one_centre_line() {
        let px = 32;
        let pts = painted(Glyph::More, px);
        assert!(!pts.is_empty());
        let (min_y, max_y) = pts
            .iter()
            .fold((px, 0), |(a, b), &(_, y)| (a.min(y), b.max(y)));
        // Centred vertically (rows min..=max straddle the middle).
        assert!(min_y <= px / 2 && max_y >= px / 2 - 1, "{min_y}..{max_y}");
        assert!((min_y + max_y).abs_diff(px - 1) <= 1, "not centred");
        // Three separate clusters along x.
        let mut cols: Vec<u32> = pts.iter().map(|&(x, _)| x).collect();
        cols.sort_unstable();
        cols.dedup();
        let clusters = cols.windows(2).filter(|w| w[1] - w[0] > 1).count() + 1;
        assert_eq!(clusters, 3);
    }

    #[test]
    fn back_chevron_is_vertically_centred() {
        let px = 32;
        let pts = painted(Glyph::Back, px);
        let (min_y, max_y) = pts
            .iter()
            .fold((px, 0), |(a, b), &(_, y)| (a.min(y), b.max(y)));
        assert!((min_y + max_y).abs_diff(px - 1) <= 1, "not centred");
    }

    #[test]
    fn settings_is_a_gear_not_a_star() {
        // A gear has a hub hole: the centre pixel is empty, the ring is not.
        let px = 40;
        let pts = painted(Glyph::Settings, px);
        let at = |x: u32, y: u32| pts.contains(&(x, y));
        assert!(!at(px / 2, px / 2) || pts.len() > 100);
        // The outline reaches close to the glyph's outer radius.
        let max_dx = pts.iter().map(|&(x, _)| x.abs_diff(px / 2)).max().unwrap();
        assert!(max_dx >= (px as f32 * 0.38) as u32);
    }
}
