//! robroz design tokens.
//!
//! The source of truth is `tokens.json` (UI's file, copied byte-for-byte
//! from the brand kit). From it, `cargo run -p robroz-tokens --bin gen`
//! writes:
//! - `src/generated.rs`: Rust constants for `base` and the default `solid`
//!   skin (re-exported here as [`base`] and [`solid`]);
//! - `generated/tokens.css`: CSS custom properties for a future web skin.
//!
//! Both files are checked in and a test fails when either is stale. Brand
//! colours exist only in `tokens.json`; nothing here restates a hex value.
//!
//! The modules below the generated part ([`radius`], [`border`], [`space`],
//! [`font`], [`motion`]) are the card SPEC's component metrics. Where
//! `tokens.json` has the same value they read it from there; the rest
//! (check-mark size, panel heights, preview line height…) are SPEC-only.
//!
//! Platform skin crates (`robroz-skin-macos`, `-fluent`, `-adwaita`,
//! `-web`, `-material`) will read their own `skins.<name>` from the same
//! file.

pub mod codegen;
#[rustfmt::skip]
mod generated;

pub use generated::{base, solid, VERSION};

/// sRGB.
pub type Rgb = [u8; 3];

/// sRGB with alpha (`rgba()` / `transparent` in `tokens.json`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub rgb: Rgb,
    pub alpha: f32,
}

/// The only brand orange. Not re-tinted per theme.
pub const BRAND_ORANGE: Rgb = base::brand::ORANGE;

/// One colour scheme of the solid skin, as robroz-ui consumes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorSet {
    pub window: Rgb,
    pub sidebar: Rgb,
    pub content: Rgb,
    pub surface: Rgb,
    pub sunken: Rgb,
    pub border: Rgb,
    pub border2: Rgb,
    pub text: Rgb,
    pub text2: Rgb,
    pub text3: Rgb,
    pub accent: Rgb,
    /// My-bubble and send-button fill. White text on this is ≥4.5:1
    /// (5.10:1). Same value in light and dark. Links still use `accent`.
    pub accent_strong: Rgb,
    pub accent_hover: Rgb,
    pub accent_pressed: Rgb,
    pub on_accent: Rgb,
    pub control: Rgba,
    pub control_hover: Rgb,
    pub control_checked: Rgb,
    pub selection: Rgba,
    pub toggle_off: Rgb,
    pub knob: Rgb,
    pub divider: Rgb,
    pub focus: Rgb,
    /// Border of an answered pick (SPEC §3: 1.5px in the text colour).
    pub sel_border: Rgb,
    pub dot_waiting: Rgb,
    pub dot_done: Rgb,
    pub dot_void: Rgb,
    pub dot_stuck: Rgb,
    /// Client-side outline around the frozen status card; dark only,
    /// because the "running" fill is 1.1:1 on the dark background.
    pub status_outline: Option<Rgb>,
}

macro_rules! color_set {
    ($p:path, $d:path, $outline:expr) => {{
        use $d as d;
        use $p as p;
        ColorSet {
            window: p::WINDOW,
            sidebar: p::SIDEBAR,
            content: p::CONTENT,
            surface: p::SURFACE,
            sunken: p::SUNKEN,
            border: p::BORDER,
            border2: p::BORDER2,
            text: p::TEXT,
            text2: p::TEXT2,
            text3: p::TEXT3,
            accent: p::ACCENT,
            accent_strong: p::ACCENT_STRONG,
            accent_hover: p::ACCENT_HOVER,
            accent_pressed: p::ACCENT_PRESSED,
            on_accent: p::ON_ACCENT,
            control: p::CONTROL,
            control_hover: p::CONTROL_HOVER,
            control_checked: p::CONTROL_CHECKED,
            selection: p::SELECTION,
            toggle_off: p::TOGGLE_OFF,
            knob: p::KNOB,
            divider: p::DIVIDER,
            focus: p::FOCUS,
            sel_border: p::TEXT,
            dot_waiting: d::WAITING,
            dot_done: d::DONE,
            dot_void: d::EXPIRED,
            dot_stuck: d::BLOCKED,
            status_outline: $outline,
        }
    }};
}

pub const LIGHT: ColorSet = color_set!(solid::light::palette, base::status_dot::light, None);
pub const DARK: ColorSet = color_set!(
    solid::dark::palette,
    base::status_dot::dark,
    Some(base::status_card::DARK_STROKE)
);

/// Whole-control opacity when disabled.
pub const DISABLED_ALPHA: f32 = 0.4;

/// Corner radii, logical px.
pub mod radius {
    use crate::solid::light::radii as r;
    pub const CARD: f32 = r::R_CARD;
    pub const PANEL: f32 = r::R_CARD;
    pub const BUTTON: f32 = r::R_BUTTON;
    pub const TAG: f32 = r::R_TAG;
    pub const PATH_CHIP: f32 = r::R_PATH;
    pub const PREVIEW: f32 = r::R_PREVIEW;
    pub const SWITCH: f32 = r::R_SWITCH;
    pub const ROW: f32 = r::R_ROW;
    pub const CHECKBOX: f32 = 3.0;
    pub const STATUS_CARD: f32 = 8.0;
}

/// Border widths, logical px.
pub mod border {
    use crate::solid::light::stroke as w;
    pub const CARD: f32 = w::W_CARD;
    pub const BUTTON: f32 = w::W_CONTROL;
    pub const SELECTED: f32 = 1.5;
    pub const TAG: f32 = 1.0;
    pub const CHECKBOX: f32 = 1.2;
    pub const DIVIDER: f32 = w::W_DIVIDER;
    pub const STATUS_OUTLINE: f32 = 1.0;
    pub const FOCUS_RING: f32 = w::W_FOCUS;
    pub const FOCUS_OFFSET: f32 = w::FOCUS_OFFSET;
}

/// Spacing and control sizes, logical px.
pub mod space {
    use crate::base::layout as l;
    use crate::solid::light::controls as c;
    pub const STREAM_PADDING: f32 = l::STREAM_PADDING;
    pub const STREAM_SPACING: f32 = l::CARD_GAP;
    pub const CARD_PAD_V: f32 = 12.0;
    pub const CARD_PAD_H: f32 = 14.0;
    pub const HEAD_HEIGHT: f32 = 20.0;
    pub const HEAD_GAP: f32 = 8.0;
    pub const BODY_GAP: f32 = 12.0;
    pub const FOOT_GAP: f32 = 10.0;
    pub const DOT: f32 = 8.0;
    pub const DOT_GAP: f32 = 6.0;
    pub const BUTTON_HEIGHT: f32 = c::BUTTON_H;
    pub const BUTTON_PAD_H: f32 = c::BUTTON_PX;
    pub const BUTTON_GAP: f32 = c::BUTTON_GAP;
    pub const CHECK_W: f32 = 12.0;
    pub const CHECK_H: f32 = 8.0;
    pub const CHECK_STROKE: f32 = 1.8;
    pub const CHECK_GAP: f32 = 6.0;
    pub const CHECKBOX: f32 = 14.0;
    pub const CHECKBOX_GAP: f32 = 8.0;
    pub const TAG_HEIGHT: f32 = 18.0;
    pub const TAG_PAD_H: f32 = 6.0;
    pub const PATH_HEIGHT: f32 = 24.0;
    pub const PATH_PAD_H: f32 = 8.0;
    pub const PREVIEW_PAD: f32 = 12.0;
    pub const SIDEBAR_WIDTH: f32 = c::SIDEBAR_W;
    pub const PANEL_HEAD_HEIGHT: f32 = 48.0;
    pub const ROW_HEIGHT: f32 = c::ROW_H;
    pub const ROW_PAD_H: f32 = 16.0;
    pub const ROW_TIME_GAP: f32 = 16.0;
    pub const TOGGLE_W: f32 = 36.0;
    pub const TOGGLE_H: f32 = 20.0;
    pub const KNOB: f32 = 16.0;
    pub const EMPTY_HEIGHT: f32 = 168.0;
    pub const EMPTY_RING_R: f32 = 7.0;
    pub const EMPTY_RING_STROKE: f32 = 2.0;
    pub const STATUS_PAD_V: f32 = 10.0;
    pub const STATUS_PAD_H: f32 = 14.0;
    /// Solid (desktop, mouse) skin's minimum interactive size. Each skin
    /// sets its own; robroz-ui raises it to `TOUCH_FLOOR` on touch input or
    /// windows no wider than `NARROW_BREAKPOINT`.
    pub const TOUCH_TARGET: f32 = c::TOUCH_TARGET;
    pub const TOUCH_FLOOR: f32 = l::TOUCH_FLOOR_NARROW;
    pub const NARROW_BREAKPOINT: f32 = l::NARROW_BREAKPOINT;
    /// Narrowest width cards must lay out at without horizontal overflow.
    pub const NARROW_WIDTH: f32 = l::MIN_WIDTH;
}

/// Font sizes and line heights, logical px. Weights are 400 and 700 only.
pub mod font {
    use crate::base::type_scale as ts;
    use crate::solid::light::type_ as t;
    pub const WEIGHT_REGULAR: u16 = 400;
    pub const WEIGHT_BOLD: u16 = 700;
    pub const BOT: f32 = t::F_NAME_SIZE;
    pub const STATUS: f32 = t::F_STATUS;
    pub const BODY: f32 = t::F_BODY;
    pub const BODY_LINE: f32 = ts::BODY_LINE_HEIGHT;
    pub const BUTTON: f32 = t::F_BUTTON;
    pub const DRAFT_TITLE: f32 = 14.0;
    pub const PREVIEW: f32 = 13.0;
    pub const PREVIEW_LINE: f32 = 20.0;
    pub const MONO: f32 = ts::META;
    pub const FOOT: f32 = 12.0;
    pub const FOOT_LINE: f32 = 16.0;
    pub const TAG: f32 = ts::CAPTION;
    pub const LIST_TITLE: f32 = t::F_TITLE_SIZE;
    pub const LIST_COUNT: f32 = 13.0;
    pub const ROW_NAME: f32 = 14.0;
    pub const ROW_TIME: f32 = 13.0;
    pub const EMPTY_TITLE: f32 = 14.0;
    pub const EMPTY_HINT: f32 = 13.0;
    /// Body family preference (CSS form). Native skins resolve their own.
    pub const FAMILY_SANS: &str =
        "system-ui, -apple-system, \"Segoe UI\", \"Noto Sans SC\", \"PingFang SC\", sans-serif";
    pub const FAMILY_MONO: &str = "\"IBM Plex Mono\", ui-monospace, monospace";
}

/// Motion durations, milliseconds. `HOVER` is the skin's `motion`
/// (120ms ease-out); the others are SPEC-side defaults until UI sets them.
/// The solid skin uses `NOTE_VISIBLE` for the 没改成功 note.
pub mod motion {
    use crate::solid::light::motion as m;
    pub const HOVER: u32 = m::MOTION_MS;
    pub const REDUCED: u32 = m::REDUCED_MOTION_MS;
    pub const PRESS: u32 = 80;
    pub const TOGGLE: u32 = 160;
    pub const CARD_UPDATE: u32 = 200;
    pub const NOTE_VISIBLE: u32 = 3000;
}

pub fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn px(v: f32) -> String {
    let s = format!("{v}");
    format!("{s}px")
}

fn scalar_vars() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    let mut add = |prefix: &str, items: &[(&str, f32)]| {
        for (name, value) in items {
            v.push((format!("--robroz-spec-{prefix}-{name}"), px(*value)));
        }
    };
    use self::{border as b, font as f, radius as r, space as s};
    add(
        "radius",
        &[
            ("card", r::CARD),
            ("panel", r::PANEL),
            ("button", r::BUTTON),
            ("tag", r::TAG),
            ("path-chip", r::PATH_CHIP),
            ("preview", r::PREVIEW),
            ("checkbox", r::CHECKBOX),
            ("status-card", r::STATUS_CARD),
        ],
    );
    add(
        "border",
        &[
            ("card", b::CARD),
            ("button", b::BUTTON),
            ("selected", b::SELECTED),
            ("tag", b::TAG),
            ("checkbox", b::CHECKBOX),
            ("divider", b::DIVIDER),
            ("status-outline", b::STATUS_OUTLINE),
            ("focus-ring", b::FOCUS_RING),
            ("focus-offset", b::FOCUS_OFFSET),
        ],
    );
    add(
        "space",
        &[
            ("stream-padding", s::STREAM_PADDING),
            ("stream-spacing", s::STREAM_SPACING),
            ("card-pad-v", s::CARD_PAD_V),
            ("card-pad-h", s::CARD_PAD_H),
            ("head-height", s::HEAD_HEIGHT),
            ("head-gap", s::HEAD_GAP),
            ("body-gap", s::BODY_GAP),
            ("foot-gap", s::FOOT_GAP),
            ("dot", s::DOT),
            ("dot-gap", s::DOT_GAP),
            ("button-height", s::BUTTON_HEIGHT),
            ("button-pad-h", s::BUTTON_PAD_H),
            ("button-gap", s::BUTTON_GAP),
            ("check-w", s::CHECK_W),
            ("check-h", s::CHECK_H),
            ("check-stroke", s::CHECK_STROKE),
            ("check-gap", s::CHECK_GAP),
            ("checkbox", s::CHECKBOX),
            ("checkbox-gap", s::CHECKBOX_GAP),
            ("tag-height", s::TAG_HEIGHT),
            ("tag-pad-h", s::TAG_PAD_H),
            ("path-height", s::PATH_HEIGHT),
            ("path-pad-h", s::PATH_PAD_H),
            ("preview-pad", s::PREVIEW_PAD),
            ("sidebar-width", s::SIDEBAR_WIDTH),
            ("panel-head-height", s::PANEL_HEAD_HEIGHT),
            ("row-height", s::ROW_HEIGHT),
            ("row-pad-h", s::ROW_PAD_H),
            ("row-time-gap", s::ROW_TIME_GAP),
            ("toggle-w", s::TOGGLE_W),
            ("toggle-h", s::TOGGLE_H),
            ("knob", s::KNOB),
            ("empty-height", s::EMPTY_HEIGHT),
            ("empty-ring-r", s::EMPTY_RING_R),
            ("empty-ring-stroke", s::EMPTY_RING_STROKE),
            ("status-pad-v", s::STATUS_PAD_V),
            ("status-pad-h", s::STATUS_PAD_H),
            ("touch-target", s::TOUCH_TARGET),
            ("touch-floor", s::TOUCH_FLOOR),
            ("narrow-breakpoint", s::NARROW_BREAKPOINT),
            ("narrow-width", s::NARROW_WIDTH),
        ],
    );
    add(
        "font",
        &[
            ("bot", f::BOT),
            ("status", f::STATUS),
            ("body", f::BODY),
            ("body-line", f::BODY_LINE),
            ("button", f::BUTTON),
            ("draft-title", f::DRAFT_TITLE),
            ("preview", f::PREVIEW),
            ("preview-line", f::PREVIEW_LINE),
            ("mono", f::MONO),
            ("foot", f::FOOT),
            ("foot-line", f::FOOT_LINE),
            ("tag", f::TAG),
            ("list-title", f::LIST_TITLE),
            ("list-count", f::LIST_COUNT),
            ("row-name", f::ROW_NAME),
            ("row-time", f::ROW_TIME),
            ("empty-title", f::EMPTY_TITLE),
            ("empty-hint", f::EMPTY_HINT),
        ],
    );
    v.push((
        "--robroz-spec-font-weight-regular".into(),
        f::WEIGHT_REGULAR.to_string(),
    ));
    v.push((
        "--robroz-spec-font-weight-bold".into(),
        f::WEIGHT_BOLD.to_string(),
    ));
    v.push((
        "--robroz-spec-font-family-sans".into(),
        f::FAMILY_SANS.into(),
    ));
    v.push((
        "--robroz-spec-font-family-mono".into(),
        f::FAMILY_MONO.into(),
    ));
    for (name, ms) in [
        ("hover", motion::HOVER),
        ("reduced", motion::REDUCED),
        ("press", motion::PRESS),
        ("toggle", motion::TOGGLE),
        ("card-update", motion::CARD_UPDATE),
        ("note-visible", motion::NOTE_VISIBLE),
    ] {
        v.push((format!("--robroz-spec-motion-{name}"), format!("{ms}ms")));
    }
    v.push((
        "--robroz-spec-disabled-alpha".into(),
        format!("{DISABLED_ALPHA}"),
    ));
    v
}

/// The `tokens.json` this crate was built from.
pub const TOKENS_JSON: &str = include_str!("../tokens.json");

/// CSS custom properties: `base` + solid skin from `tokens.json` (light by
/// default, dark with `data-theme="dark"` or the system preference unless
/// `data-theme="light"`), followed by the SPEC component metrics.
pub fn css_from(json: &str) -> String {
    codegen::css(json, &scalar_vars())
}

/// [`css_from`] for the built-in `tokens.json`.
pub fn css() -> String {
    css_from(TOKENS_JSON)
}

/// Where the checked-in CSS lives, relative to this crate.
pub const CSS_PATH: &str = "generated/tokens.css";
/// Where the generated Rust constants live, relative to this crate.
pub const RUST_PATH: &str = "src/generated.rs";
/// The source file, relative to this crate.
pub const JSON_PATH: &str = "tokens.json";

#[cfg(test)]
mod tests {
    use super::*;

    fn on_disk(rel: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .replace("\r\n", "\n")
    }

    #[test]
    fn generated_rust_is_up_to_date() {
        assert!(
            on_disk(RUST_PATH) == codegen::rust(TOKENS_JSON),
            "{RUST_PATH} is stale; run `cargo run -p robroz-tokens --bin gen`"
        );
    }

    #[test]
    fn generated_css_is_up_to_date() {
        assert!(
            on_disk(CSS_PATH) == css(),
            "{CSS_PATH} is stale; run `cargo run -p robroz-tokens --bin gen`"
        );
    }

    #[test]
    fn brand_orange_is_the_accent_in_both_schemes() {
        assert_eq!(hex(BRAND_ORANGE), "#c8551f");
        assert_eq!(LIGHT.accent, BRAND_ORANGE);
        assert_eq!(DARK.accent, BRAND_ORANGE);
        assert_eq!(LIGHT.dot_waiting, BRAND_ORANGE);
    }

    #[test]
    fn solid_values_match_the_card_spec() {
        assert_eq!(hex(LIGHT.window), "#f4f1ea");
        assert_eq!(hex(DARK.surface), "#1e2127");
        assert_eq!(DARK.status_outline.map(hex).as_deref(), Some("#3f444d"));
        assert_eq!(LIGHT.status_outline, None);
        assert_eq!(radius::CARD, 8.0);
        assert_eq!(space::BUTTON_HEIGHT, 32.0);
        assert_eq!(space::TOUCH_TARGET, 32.0);
        assert_eq!(space::TOUCH_FLOOR, 44.0);
        assert_eq!(space::NARROW_BREAKPOINT, 480.0);
        assert_eq!(font::BOT, 13.0);
        assert_eq!(motion::HOVER, 120);
        assert_eq!(LIGHT.control.alpha, 0.0);
    }

    #[test]
    fn css_has_both_schemes_and_metrics() {
        let css = css();
        assert!(css.contains("--robroz-palette-accent: #c8551f;"));
        assert!(css.contains("--robroz-palette-accent-strong: #b84d1b;"));
        assert!(css.contains("--robroz-chat-composer-radius: 20px;"));
        assert_eq!(hex(LIGHT.accent_strong), "#b84d1b");
        assert_eq!(LIGHT.accent_strong, DARK.accent_strong);
        assert_eq!(hex(LIGHT.accent), "#c8551f");
        assert!(css.contains("--robroz-palette-window: #f4f1ea;"));
        assert!(css.contains("--robroz-palette-window: #16181d;"));
        assert!(css.contains("--robroz-status-card-dark-stroke: #3f444d;"));
        assert!(css.contains("--robroz-radii-r-card: 8px;"));
        assert!(css.contains("--robroz-controls-touch-target: 32px;"));
        assert!(css.contains("--robroz-motion-motion: 120ms;"));
        assert!(css.contains("--robroz-spec-border-selected: 1.5px;"));
    }
}
