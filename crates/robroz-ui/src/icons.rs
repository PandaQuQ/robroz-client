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
            p.push_circle(mid, mid, s * 0.12);
            for i in 0..6 {
                let a = (i as f32) * std::f32::consts::PI / 3.0;
                let inner = s * 0.20;
                let outer = s * 0.32;
                p.move_to(mid + a.cos() * inner, mid + a.sin() * inner);
                p.line_to(mid + a.cos() * outer, mid + a.sin() * outer);
            }
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

/// A 20px (logical) line icon, sharp at `scale` device pixels.
pub fn icon(glyph: Glyph, rgb: [u8; 3], size: f32, scale: f32) -> Image {
    let scale = scale.max(1.0);
    let px = (size * scale).round().max(1.0) as u32;
    let color = Color::from_rgb8(rgb[0], rgb[1], rgb[2]);
    let Some(mut pm) = tiny_skia::Pixmap::new(px, px) else {
        return Image::new(image::Handle::from_rgba(1, 1, vec![0, 0, 0, 0]))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size));
    };
    stroke(
        &mut pm,
        |p| draw(glyph, p, px as f32),
        color,
        (1.6 * scale).max(1.0),
    );
    Image::new(image::Handle::from_rgba(px, px, pack_rgba(&pm)))
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
        ] {
            let img = icon(g, [0, 0, 0], 20.0, 2.0);
            let _ = img;
        }
    }
}
