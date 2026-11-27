//! Small drawings: check mark, checkbox, toggle, the brand rings.
//! Sizes and colours are passed in from the skin; nothing is fixed here.
//!
//! Check marks and checkboxes are rasterized with tiny-skia and drawn as
//! images: several `canvas` widgets in one iced_wgpu layer only show the
//! last one, which is why only the last option on a choice card had a box.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Program, Stroke};
use iced::widget::image;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme as IcedTheme};

pub struct CheckBox {
    pub checked: bool,
    pub fill: Color,
    pub mark: Color,
    pub border: Color,
    pub border_width: f32,
    pub radius: f32,
    pub mark_stroke: f32,
}

impl<M> Program<M> for CheckBox {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &IcedTheme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let size = bounds.size();
        if self.checked {
            let rect = Path::rounded_rectangle(Point::ORIGIN, size, self.radius.into());
            frame.fill(&rect, self.fill);
            let (w, h) = (size.width, size.height);
            let mark = Path::new(|b| {
                b.move_to(Point::new(w * 0.24, h * 0.52));
                b.line_to(Point::new(w * 0.43, h * 0.70));
                b.line_to(Point::new(w * 0.77, h * 0.32));
            });
            frame.stroke(
                &mark,
                Stroke::default()
                    .with_color(self.mark)
                    .with_width(self.mark_stroke)
                    .with_line_cap(canvas::LineCap::Round)
                    .with_line_join(canvas::LineJoin::Round),
            );
        } else {
            let i = self.border_width / 2.0;
            let rect = Path::rounded_rectangle(
                Point::new(i, i),
                Size::new(size.width - 2.0 * i, size.height - 2.0 * i),
                self.radius.into(),
            );
            frame.stroke(
                &rect,
                Stroke::default()
                    .with_color(self.border)
                    .with_width(self.border_width),
            );
        }
        vec![frame.into_geometry()]
    }
}

pub struct Toggle {
    pub on: bool,
    pub track_on: Color,
    pub track_off: Color,
    pub knob: Color,
    pub knob_size: f32,
}

impl<M> Program<M> for Toggle {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &IcedTheme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let track = Path::rounded_rectangle(Point::ORIGIN, size, (size.height / 2.0).into());
        frame.fill(
            &track,
            if self.on {
                self.track_on
            } else {
                self.track_off
            },
        );
        let r = self.knob_size / 2.0;
        let inset = (size.height - self.knob_size) / 2.0;
        let cx = if self.on {
            size.width - inset - r
        } else {
            inset + r
        };
        frame.fill(
            &Path::circle(Point::new(cx, size.height / 2.0), r),
            self.knob,
        );
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _: &(),
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

/// Two outline rings side by side (the brand mark, SPEC §7 empty state).
pub struct Rings {
    pub color: Color,
    pub radius: f32,
    pub stroke: f32,
}

impl Rings {
    pub fn size(&self) -> Size {
        let d = 2.0 * self.radius + self.stroke;
        Size::new(2.0 * d + self.stroke, d)
    }
}

impl<M> Program<M> for Rings {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &IcedTheme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let d = 2.0 * self.radius + self.stroke;
        let cy = bounds.height / 2.0;
        for cx in [d / 2.0, d * 1.5 + self.stroke] {
            frame.stroke(
                &Path::circle(Point::new(cx, cy), self.radius),
                Stroke::default()
                    .with_color(self.color)
                    .with_width(self.stroke),
            );
        }
        vec![frame.into_geometry()]
    }
}

// --- Rasterized check / checkbox (images, not canvas) ---

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
    // Straight alpha: iced image::Handle is not premultiplied.
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

fn stroke_check(p: &mut tiny_skia::PathBuilder, w: f32, h: f32) {
    p.move_to(w * 0.24, h * 0.52);
    p.line_to(w * 0.43, h * 0.70);
    p.line_to(w * 0.77, h * 0.32);
}

fn push_rounded_rect(p: &mut tiny_skia::PathBuilder, x: f32, y: f32, w: f32, h: f32, r: f32) {
    let r = r.max(0.0).min(w / 2.0).min(h / 2.0);
    p.move_to(x + r, y);
    p.line_to(x + w - r, y);
    p.quad_to(x + w, y, x + w, y + r);
    p.line_to(x + w, y + h - r);
    p.quad_to(x + w, y + h, x + w - r, y + h);
    p.line_to(x + r, y + h);
    p.quad_to(x, y + h, x, y + h - r);
    p.line_to(x, y + r);
    p.quad_to(x, y, x + r, y);
    p.close();
}

fn paint_stroke(pm: &mut tiny_skia::Pixmap, path: tiny_skia::Path, color: Color, width: f32) {
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

fn raster_check(color: Color, stroke: f32, w: u32, h: u32) -> Vec<u8> {
    let Some(mut pm) = tiny_skia::Pixmap::new(w, h) else {
        return vec![0; (w * h * 4) as usize];
    };
    let mut p = tiny_skia::PathBuilder::new();
    stroke_check(&mut p, w as f32, h as f32);
    if let Some(path) = p.finish() {
        paint_stroke(&mut pm, path, color, stroke);
    }
    pack_rgba(&pm)
}

fn raster_checkbox(box_: &CheckBox, px: u32) -> Vec<u8> {
    let Some(mut pm) = tiny_skia::Pixmap::new(px, px) else {
        return vec![0; (px * px * 4) as usize];
    };
    let size = px as f32;
    let radius = box_.radius;
    if box_.checked {
        let mut p = tiny_skia::PathBuilder::new();
        push_rounded_rect(&mut p, 0.0, 0.0, size, size, radius);
        if let Some(path) = p.finish() {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color(skia_color(box_.fill));
            paint.anti_alias = true;
            pm.fill_path(
                &path,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }
        let mut mark = tiny_skia::PathBuilder::new();
        stroke_check(&mut mark, size, size);
        if let Some(path) = mark.finish() {
            paint_stroke(&mut pm, path, box_.mark, box_.mark_stroke);
        }
    } else {
        let i = box_.border_width / 2.0;
        let mut p = tiny_skia::PathBuilder::new();
        push_rounded_rect(&mut p, i, i, size - 2.0 * i, size - 2.0 * i, radius);
        if let Some(path) = p.finish() {
            paint_stroke(&mut pm, path, box_.border, box_.border_width);
        }
    }
    pack_rgba(&pm)
}

/// A leading check mark, as an image (not a canvas).
pub fn check_mark<'a, M: 'a>(color: Color, stroke: f32, w: f32, h: f32) -> Element<'a, M> {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct Key {
        color: [u8; 4],
        stroke: u16,
        w: u16,
        h: u16,
    }
    static CACHE: OnceLock<Mutex<HashMap<Key, image::Handle>>> = OnceLock::new();
    let scale = 2u32;
    let pw = (w.round() as u32).max(1) * scale;
    let ph = (h.round() as u32).max(1) * scale;
    let key = Key {
        color: rgba_u8(color),
        stroke: (stroke * scale as f32 * 10.0).round() as u16,
        w: pw as u16,
        h: ph as u16,
    };
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap();
    let handle = cache
        .entry(key)
        .or_insert_with(|| {
            image::Handle::from_rgba(pw, ph, raster_check(color, stroke * scale as f32, pw, ph))
        })
        .clone();
    drop(cache);
    iced::widget::image(handle)
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .content_fit(iced::ContentFit::Fill)
        .into()
}

/// A checkbox, as an image (not a canvas).
pub fn checkbox<'a, M: 'a>(box_: CheckBox, size: f32) -> Element<'a, M> {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    struct Key {
        checked: bool,
        fill: [u8; 4],
        mark: [u8; 4],
        border: [u8; 4],
        border_width: u16,
        radius: u16,
        mark_stroke: u16,
        px: u16,
    }
    static CACHE: OnceLock<Mutex<HashMap<Key, image::Handle>>> = OnceLock::new();
    let scale = 2u32;
    let px = (size.round() as u32).max(1) * scale;
    let key = Key {
        checked: box_.checked,
        fill: rgba_u8(box_.fill),
        mark: rgba_u8(box_.mark),
        border: rgba_u8(box_.border),
        border_width: (box_.border_width * scale as f32 * 10.0).round() as u16,
        radius: (box_.radius * scale as f32 * 10.0).round() as u16,
        mark_stroke: (box_.mark_stroke * scale as f32 * 10.0).round() as u16,
        px: px as u16,
    };
    // Scale stroke/radius for the 2x pixmap.
    let scaled = CheckBox {
        checked: box_.checked,
        fill: box_.fill,
        mark: box_.mark,
        border: box_.border,
        border_width: box_.border_width * scale as f32,
        radius: box_.radius * scale as f32,
        mark_stroke: box_.mark_stroke * scale as f32,
    };
    let mut cache = CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap();
    let handle = cache
        .entry(key)
        .or_insert_with(|| image::Handle::from_rgba(px, px, raster_checkbox(&scaled, px)))
        .clone();
    drop(cache);
    iced::widget::image(handle)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .content_fit(iced::ContentFit::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque_count(rgba: &[u8]) -> usize {
        rgba.chunks_exact(4).filter(|p| p[3] > 8).count()
    }

    #[test]
    fn checkbox_bitmap_has_a_border_when_empty_and_a_fill_when_checked() {
        let empty = CheckBox {
            checked: false,
            fill: Color::BLACK,
            mark: Color::WHITE,
            border: Color::from_rgb(0.2, 0.2, 0.2),
            border_width: 1.0,
            radius: 3.0,
            mark_stroke: 1.5,
        };
        let on = CheckBox {
            checked: true,
            ..empty
        };
        let a = raster_checkbox(&empty, 28);
        let b = raster_checkbox(&on, 28);
        // Empty: a ring, not a fill. Mid pixel stays clear.
        let mid = 14 * 28 * 4 + 14 * 4;
        assert_eq!(a[mid + 3], 0, "empty checkbox mid should be clear");
        assert!(opaque_count(&a) > 20, "empty checkbox should draw a border");
        // Checked: mid is filled, and more pixels are opaque than the ring.
        assert!(b[mid + 3] > 200, "checked checkbox mid should be filled");
        assert!(opaque_count(&b) > opaque_count(&a));
    }
}
