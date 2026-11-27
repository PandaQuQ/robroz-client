//! The skin: every visual parameter the components read.
//!
//! Components read colours, radii, border widths, shadows, paddings, sizes
//! and type only from [`Skin`]; none are written into a component. Today
//! there are two constructors, [`Skin::light`] and [`Skin::dark`], which
//! are the default solid skin from the UI SPEC (§1–§3, §7), filled from
//! `robroz-tokens` (the only place brand colours are defined). Per-platform
//! looks (macOS Liquid Glass, Windows Fluent/Mica, GNOME/Adwaita) will be
//! skin crates (`robroz-skin-macos`, `-fluent`, `-adwaita`, `-web`) filling the same struct: same layout and cards, other
//! surface material, radii and control feel; colours stay ours.
//!
//! The frozen status card keeps `robroz_card::palette()` for its fill and
//! text; only its client-side outline lives here.

use robroz_tokens as tokens;

/// sRGB, same shape as `robroz_card::Palette`.
pub use robroz_tokens::Rgb;

/// Light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

impl Mode {
    pub fn toggled(self) -> Self {
        match self {
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::Light,
        }
    }

    /// `ROBROZ_THEME=dark` starts in dark mode; anything else is light.
    pub fn from_env_value(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some(v) if v.eq_ignore_ascii_case("dark") => Mode::Dark,
            _ => Mode::Light,
        }
    }
}

/// Surface material (THEMES.md `material.material_kind`). The solid skin
/// is `Solid`; platform skins set the others and do the system calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialKind {
    Solid,
    Vibrancy,
    Mica,
    Backdrop,
    Tonal,
}

/// sRGB with alpha.
pub use robroz_tokens::Rgba;

/// Destructive actions and inline error lines (`danger`). Same red in
/// both themes; used for 「删除规则」 and the avatar save-failure line.
pub const DANGER: Rgb = [0xc4, 0x47, 0x3a];
/// Hover fill for a danger button.
pub const DANGER_HOVER: Rgb = [0xb0, 0x3c, 0x32];

/// Colours (THEMES.md `palette.*`; SPEC §1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
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
    /// My-bubble and send-button fill (`accentStrong`). White 15px
    /// regular text. Links and selected states still use `accent`.
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
    /// Border of an answered pick (SPEC §3).
    pub sel_border: Rgb,
    /// Outline for the frozen status card. Dark only: its "running" fill
    /// `#1f2328` is 1.1:1 on the dark window colour.
    pub status_outline: Option<Rgb>,
    /// Whole-control opacity when disabled.
    pub disabled_alpha: f32,
}

/// Status dots on the new cards (THEMES.md `status`, from `base.status_dot`;
/// same on every skin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusDots {
    pub waiting: Rgb,
    pub done: Rgb,
    pub expired: Rgb,
    pub blocked: Rgb,
}

/// Drop shadow. `None` in the solid skin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub color: Rgb,
    pub alpha: f32,
    pub offset: (f32, f32),
    pub blur: f32,
}

/// THEMES.md `shadow.*`. Window shadows are drawn by the system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadows {
    pub card: Option<Shadow>,
    pub flyout: Option<Shadow>,
}

/// Corner radii (THEMES.md `radii.*`, plus SPEC-only `panel`, `checkbox`,
/// `status_card`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Radii {
    pub card: f32,
    pub panel: f32,
    pub button: f32,
    pub tag: f32,
    pub path_chip: f32,
    pub preview: f32,
    pub switch: f32,
    pub row: f32,
    pub checkbox: f32,
    pub status_card: f32,
}

/// Line widths (THEMES.md `stroke.*`, plus SPEC-only `selected`, `tag`,
/// `checkbox`, `status_outline`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub card: f32,
    pub control: f32,
    pub focus: f32,
    pub focus_offset: f32,
    pub divider: f32,
    pub selected: f32,
    pub tag: f32,
    pub checkbox: f32,
    pub status_outline: f32,
}

/// Control sizes and feel (THEMES.md `controls.*`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Controls {
    pub button_h: f32,
    pub button_px: f32,
    pub button_gap: f32,
    pub checkbox: f32,
    pub switch_w: f32,
    pub switch_h: f32,
    pub knob: f32,
    pub row_h: f32,
    pub sidebar_w: f32,
    pub card_pad_v: f32,
    pub card_pad_h: f32,
    /// Minimum interactive size (both axes), set by the skin (solid 32).
    /// Components read only this; [`Skin::for_input`] applies the floor.
    pub touch_target: f32,
}

/// Layout sizes the card SPEC owns (not per-skin in THEMES.md).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    pub stream_padding: f32,
    pub stream_spacing: f32,
    pub head_height: f32,
    pub head_gap: f32,
    pub body_gap: f32,
    pub foot_gap: f32,
    pub dot: f32,
    pub dot_gap: f32,
    pub check_w: f32,
    pub check_h: f32,
    pub check_stroke: f32,
    pub check_gap: f32,
    pub checkbox_gap: f32,
    pub tag_height: f32,
    pub tag_pad_h: f32,
    pub path_height: f32,
    pub path_pad_h: f32,
    pub preview_pad: f32,
    pub panel_head_height: f32,
    pub row_pad_h: f32,
    pub row_time_gap: f32,
    pub empty_height: f32,
    pub empty_ring_r: f32,
    pub empty_ring_stroke: f32,
    pub status_pad_v: f32,
    pub status_pad_h: f32,
}

/// Font sizes and line heights (THEMES.md `type.*`, SPEC §2). Weights are
/// 400 and 700 only.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Type {
    pub bot: f32,
    pub status: f32,
    pub body: f32,
    pub body_line: f32,
    pub button: f32,
    pub draft_title: f32,
    pub preview: f32,
    pub preview_line: f32,
    pub mono: f32,
    pub foot: f32,
    pub foot_line: f32,
    pub tag: f32,
    pub list_title: f32,
    pub list_count: f32,
    pub row_name: f32,
    pub row_time: f32,
    pub empty_title: f32,
    pub empty_hint: f32,
    /// Rough average glyph width as a fraction of font size for one
    /// "unit" (ASCII); CJK counts two. Used only to clamp preview lines.
    pub unit_width: f32,
}

/// Chat feed colours and sizes (designer spec, chat bubbles). Colours are
/// the designer's exact values per mode; sizes are the same in both.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chat {
    /// Own message bubble: `accentStrong` fill, white 15px regular text.
    pub own_bg: Rgb,
    pub own_text: Rgb,
    /// Size of the body text inside an own bubble.
    pub mine_size: f32,
    /// Dimmed text and chip outline inside an own bubble (attachment chip).
    pub own_chip: Rgb,
    /// A bro's bubble and cards: fill and 1px border.
    pub bro_bg: Rgb,
    pub bro_border: Rgb,
    /// Running status card: the only status card with a fill. Every other
    /// state is a bro card with a small status dot.
    pub running_fill: Rgb,
    pub running_text: Rgb,
    /// Chat accent: avatar ring and approval card outline. Brighter than
    /// the brand orange on dark.
    pub accent: Rgb,
    /// Primary button inside chat cards: ink on light, inverted on dark.
    pub primary_bg: Rgb,
    pub primary_text: Rgb,
    /// Bro bubble corner radius (`base.chat.bubble_radius`).
    pub radius: f32,
    /// Own bubble radius and the corner nearest the sender.
    pub own_radius: f32,
    pub corner: f32,
    pub pad_v: f32,
    pub pad_h: f32,
    /// Bubble max width: this share of the available text column.
    pub max_frac: f32,
    /// Share used instead of `max_frac` in a window narrower than
    /// [`CHAT_COMPACT_WIDTH`].
    pub compact_frac: f32,
    /// Spacing inside a run and between runs.
    pub gap_in: f32,
    pub gap_between: f32,
    /// Same sender within this many seconds forms a run.
    pub group_secs: i64,
    pub time_size: f32,
    pub avatar: f32,
    pub avatar_gap: f32,
    pub column_max: f32,
    pub column_margin: f32,
    pub date_size: f32,
    pub date_gap: f32,
    pub composer_height: f32,
    pub composer_radius: f32,
    pub composer_pad_left: f32,
    pub composer_send: f32,
    pub composer_send_inset: f32,
    pub composer_bottom: f32,
    pub composer_send_empty_opacity: f32,
    /// Reserved attachment-thumbnail size inside the composer card.
    pub thumb_size: f32,
}

/// Windows narrower than this hide the sidebar and widen bubbles.
pub const CHAT_COMPACT_WIDTH: f32 = 760.0;

/// Motion durations in ms (THEMES.md `motion`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Motion {
    pub hover_ms: u32,
    pub reduced_ms: u32,
    pub toggle_ms: u32,
    pub note_visible_ms: u32,
}

/// The skin interface (THEMES.md "Skin 接口参数表"): everything a component
/// may read. robroz-ui ships the solid skin ([`Skin::light`],
/// [`Skin::dark`]); `robroz-skin-*` crates will build the same struct.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Skin {
    pub mode: Mode,
    pub material: MaterialKind,
    pub palette: Palette,
    pub status: StatusDots,
    pub radii: Radii,
    pub stroke: Stroke,
    pub shadows: Shadows,
    pub controls: Controls,
    pub metrics: Metrics,
    pub type_: Type,
    pub motion: Motion,
    pub chat: Chat,
    /// Not a skin value: set by [`Skin::for_input`] when the window is no
    /// wider than [`NARROW_MAX_WIDTH`]. Turns on the narrow-screen rules
    /// (middle-ellipsis paths, 8-character bot names). Does not control
    /// whether settings can be edited.
    pub narrow: bool,
    /// Not a skin value and not derived from window width. True only on
    /// the phone (Slint) target. A desktop window under 480px stays
    /// editable: [`Self::narrow`] only changes layout.
    pub phone: bool,
}

/// robroz-ui's floor for interactive size on touch or narrow windows.
pub const TOUCH_FLOOR: f32 = tokens::space::TOUCH_FLOOR;
/// Windows at most this wide count as narrow (phone-sized).
pub const NARROW_MAX_WIDTH: f32 = tokens::space::NARROW_BREAKPOINT;

impl Controls {
    /// Hit size: on touch input or a narrow window at least [`TOUCH_FLOOR`].
    pub fn hit(&self, touch_or_narrow: bool) -> f32 {
        if touch_or_narrow {
            self.touch_target.max(TOUCH_FLOOR)
        } else {
            self.touch_target
        }
    }
}

fn palette(set: &tokens::ColorSet) -> Palette {
    Palette {
        window: set.window,
        sidebar: set.sidebar,
        content: set.content,
        surface: set.surface,
        sunken: set.sunken,
        border: set.border,
        border2: set.border2,
        text: set.text,
        text2: set.text2,
        text3: set.text3,
        accent: set.accent,
        accent_strong: set.accent_strong,
        accent_hover: set.accent_hover,
        accent_pressed: set.accent_pressed,
        on_accent: set.on_accent,
        control: set.control,
        control_hover: set.control_hover,
        control_checked: set.control_checked,
        selection: set.selection,
        toggle_off: set.toggle_off,
        knob: set.knob,
        divider: set.divider,
        focus: set.focus,
        sel_border: set.sel_border,
        status_outline: set.status_outline,
        disabled_alpha: tokens::DISABLED_ALPHA,
    }
}

fn status(set: &tokens::ColorSet) -> StatusDots {
    StatusDots {
        waiting: set.dot_waiting,
        done: set.dot_done,
        expired: set.dot_void,
        blocked: set.dot_stuck,
    }
}

fn radii() -> Radii {
    use tokens::radius as r;
    Radii {
        card: r::CARD,
        panel: r::PANEL,
        button: r::BUTTON,
        tag: r::TAG,
        path_chip: r::PATH_CHIP,
        preview: r::PREVIEW,
        switch: r::SWITCH,
        row: r::ROW,
        checkbox: r::CHECKBOX,
        status_card: r::STATUS_CARD,
    }
}

fn stroke() -> Stroke {
    use tokens::border as b;
    Stroke {
        card: b::CARD,
        control: b::BUTTON,
        focus: b::FOCUS_RING,
        focus_offset: b::FOCUS_OFFSET,
        divider: b::DIVIDER,
        selected: b::SELECTED,
        tag: b::TAG,
        checkbox: b::CHECKBOX,
        status_outline: b::STATUS_OUTLINE,
    }
}

fn controls() -> Controls {
    use tokens::space as s;
    Controls {
        button_h: s::BUTTON_HEIGHT,
        button_px: s::BUTTON_PAD_H,
        button_gap: s::BUTTON_GAP,
        checkbox: s::CHECKBOX,
        switch_w: s::TOGGLE_W,
        switch_h: s::TOGGLE_H,
        knob: s::KNOB,
        row_h: s::ROW_HEIGHT,
        sidebar_w: s::SIDEBAR_WIDTH,
        card_pad_v: s::CARD_PAD_V,
        card_pad_h: s::CARD_PAD_H,
        touch_target: s::TOUCH_TARGET,
    }
}

fn metrics() -> Metrics {
    use tokens::space as s;
    Metrics {
        stream_padding: s::STREAM_PADDING,
        stream_spacing: s::STREAM_SPACING,
        head_height: s::HEAD_HEIGHT,
        head_gap: s::HEAD_GAP,
        body_gap: s::BODY_GAP,
        foot_gap: s::FOOT_GAP,
        dot: s::DOT,
        dot_gap: s::DOT_GAP,
        check_w: s::CHECK_W,
        check_h: s::CHECK_H,
        check_stroke: s::CHECK_STROKE,
        check_gap: s::CHECK_GAP,
        checkbox_gap: s::CHECKBOX_GAP,
        tag_height: s::TAG_HEIGHT,
        tag_pad_h: s::TAG_PAD_H,
        path_height: s::PATH_HEIGHT,
        path_pad_h: s::PATH_PAD_H,
        preview_pad: s::PREVIEW_PAD,
        panel_head_height: s::PANEL_HEAD_HEIGHT,
        row_pad_h: s::ROW_PAD_H,
        row_time_gap: s::ROW_TIME_GAP,
        empty_height: s::EMPTY_HEIGHT,
        empty_ring_r: s::EMPTY_RING_R,
        empty_ring_stroke: s::EMPTY_RING_STROKE,
        status_pad_v: s::STATUS_PAD_V,
        status_pad_h: s::STATUS_PAD_H,
    }
}

fn fonts() -> Type {
    use tokens::font as f;
    Type {
        bot: f::BOT,
        status: f::STATUS,
        body: f::BODY,
        body_line: f::BODY_LINE,
        button: f::BUTTON,
        draft_title: f::DRAFT_TITLE,
        preview: f::PREVIEW,
        preview_line: f::PREVIEW_LINE,
        mono: f::MONO,
        foot: f::FOOT,
        foot_line: f::FOOT_LINE,
        tag: f::TAG,
        list_title: f::LIST_TITLE,
        list_count: f::LIST_COUNT,
        row_name: f::ROW_NAME,
        row_time: f::ROW_TIME,
        empty_title: f::EMPTY_TITLE,
        empty_hint: f::EMPTY_HINT,
        unit_width: UNIT_WIDTH,
    }
}

/// Rough ASCII glyph width as a fraction of font size; a layout heuristic
/// for clamping preview lines, not a design token.
const UNIT_WIDTH: f32 = 0.56;

fn chat(mode: Mode, set: &tokens::ColorSet) -> Chat {
    use tokens::base::chat as c;
    // Own bubbles use `accentStrong` + white in both modes (5.10:1).
    // Running-card colours and the chat accent (avatar ring) are still
    // a designer overlay. Bro bubbles use `surface` / `border`.
    let (own_chip, running, accent, pb, pt) = match mode {
        Mode::Light => (
            [0xf0, 0xd4, 0xc8],
            ([0x1f, 0x21, 0x24], [0xf4, 0xf1, 0xea]),
            [0xc8, 0x57, 0x2a],
            [0x1f, 0x21, 0x24],
            [0xf4, 0xf1, 0xea],
        ),
        Mode::Dark => (
            [0xf0, 0xd4, 0xc8],
            ([0x2c, 0x2f, 0x33], [0xec, 0xe8, 0xdf]),
            [0xe0, 0x74, 0x3f],
            [0xec, 0xe8, 0xdf],
            [0x1f, 0x21, 0x24],
        ),
    };
    Chat {
        own_bg: set.accent_strong,
        own_text: set.on_accent,
        mine_size: c::BUBBLE_MINE_SIZE,
        own_chip,
        bro_bg: set.surface,
        bro_border: set.border,
        running_fill: running.0,
        running_text: running.1,
        accent,
        primary_bg: pb,
        primary_text: pt,
        radius: c::BUBBLE_RADIUS,
        own_radius: c::OWN_RADIUS,
        corner: c::OWN_CORNER,
        pad_v: c::BUBBLE_PAD_V,
        pad_h: c::BUBBLE_PAD_H,
        max_frac: c::BUBBLE_MAX_FRAC,
        compact_frac: c::BUBBLE_COMPACT_FRAC,
        gap_in: c::GAP_IN,
        gap_between: c::GAP_BETWEEN,
        group_secs: c::GROUP_SECS as i64,
        time_size: c::TIME_SIZE,
        avatar: c::AVATAR,
        avatar_gap: c::AVATAR_GAP,
        column_max: c::COLUMN_MAX,
        column_margin: c::COLUMN_MARGIN,
        date_size: c::DATE_SIZE,
        date_gap: c::DATE_GAP,
        composer_height: c::COMPOSER_HEIGHT,
        composer_radius: c::COMPOSER_RADIUS,
        composer_pad_left: c::COMPOSER_PAD_LEFT,
        composer_send: c::COMPOSER_SEND,
        composer_send_inset: c::COMPOSER_SEND_INSET,
        composer_bottom: c::COMPOSER_BOTTOM,
        composer_send_empty_opacity: c::COMPOSER_SEND_EMPTY_OPACITY,
        thumb_size: c::THUMB_SIZE,
    }
}

impl Skin {
    /// Width of the centred chat column in a content area `content_w` wide:
    /// at most `column_max`, with at least `column_margin` on each side.
    pub fn chat_column_width(&self, content_w: f32) -> f32 {
        let c = &self.chat;
        (content_w - 2.0 * c.column_margin)
            .min(c.column_max)
            .max(0.0)
    }

    /// Avatar column: the avatar plus the gap to the bubbles.
    pub fn chat_gutter(&self) -> f32 {
        self.chat.avatar + self.chat.avatar_gap
    }

    /// Bubble and card width limits for a chat column of `column_w` in a
    /// window `window_w` wide: `(own_bubble_max, bro_text_column)`.
    /// Cards fill the bro text column (the column minus the avatar gutter).
    pub fn chat_widths(&self, window_w: f32, column_w: f32) -> (f32, f32) {
        let c = &self.chat;
        let frac = if window_w < CHAT_COMPACT_WIDTH {
            c.compact_frac
        } else {
            c.max_frac
        };
        let text_w = (column_w - self.chat_gutter()).max(0.0);
        let own_bubble = (column_w * frac).min(column_w);
        let _ = text_w * frac;
        (own_bubble, text_w)
    }

    pub fn for_mode(mode: Mode) -> Self {
        match mode {
            Mode::Light => Self::light(),
            Mode::Dark => Self::dark(),
        }
    }

    /// Solid skin, light (`tokens.json` skins.solid.light).
    pub fn light() -> Self {
        Self::solid(Mode::Light, &tokens::LIGHT)
    }

    /// Solid skin, dark (`tokens.json` skins.solid.dark).
    pub fn dark() -> Self {
        Self::solid(Mode::Dark, &tokens::DARK)
    }

    fn solid(mode: Mode, set: &tokens::ColorSet) -> Self {
        let mut palette = palette(set);
        let chat = chat(mode, set);
        // Dark page colours are a designer overlay on the solid skin,
        // still waiting on a token.
        if mode == Mode::Dark {
            palette.window = [0x17, 0x18, 0x1a];
            palette.content = [0x17, 0x18, 0x1a];
            palette.sidebar = [0x1d, 0x1e, 0x21];
            palette.sunken = [0x1d, 0x1e, 0x21];
            palette.divider = [0x2f, 0x32, 0x36];
            palette.text = [0xec, 0xe8, 0xdf];
            palette.text2 = [0x8f, 0x8b, 0x83];
        }
        Self {
            mode,
            material: MaterialKind::Solid,
            palette,
            chat,
            status: status(set),
            radii: radii(),
            stroke: stroke(),
            shadows: Shadows {
                card: None,
                flyout: None,
            },
            controls: controls(),
            metrics: metrics(),
            type_: fonts(),
            motion: Motion {
                hover_ms: tokens::motion::HOVER,
                reduced_ms: tokens::motion::REDUCED,
                toggle_ms: tokens::motion::TOGGLE,
                note_visible_ms: tokens::motion::NOTE_VISIBLE,
            },
            narrow: false,
            phone: false,
        }
    }

    /// The skin adjusted for how it is used right now: on touch input or a
    /// window no wider than [`NARROW_MAX_WIDTH`], `controls.touch_target`
    /// becomes `max(touch_target, TOUCH_FLOOR)`; otherwise it stands.
    /// Never sets [`Self::phone`]: editability is a platform flag, not a
    /// window-width one.
    pub fn for_input(mut self, touch: bool, window_width: f32) -> Self {
        self.narrow = window_width <= NARROW_MAX_WIDTH;
        self.controls.touch_target = self.controls.hit(touch || self.narrow);
        self
    }

    /// Whether add / edit / disconnect / delete controls may be shown.
    /// False only on the phone target. Independent of [`Self::narrow`].
    pub fn can_edit(&self) -> bool {
        !self.phone
    }

    /// Colour for a status dot.
    pub fn dot(&self, dot: crate::cards::status::Dot) -> Rgb {
        use crate::cards::status::Dot;
        match dot {
            Dot::Waiting => self.status.waiting,
            Dot::Done => self.status.done,
            Dot::Void => self.status.expired,
            Dot::Stuck => self.status.blocked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::status::Dot;
    use robroz_tokens as tokens;

    #[test]
    fn single_brand_orange_in_both_modes() {
        for t in [Skin::light(), Skin::dark()] {
            assert_eq!(t.palette.accent, [0xc8, 0x55, 0x1f]);
            assert_eq!(t.dot(Dot::Waiting), [0xc8, 0x55, 0x1f]);
            assert_eq!(t.dot(Dot::Stuck), [0xf5, 0xb7, 0x00]);
        }
    }

    #[test]
    fn light_dots_reuse_the_frozen_palette() {
        let t = Skin::light();
        assert_eq!(
            t.dot(Dot::Done),
            robroz_card::palette(robroz_card::Tone::Green).fill
        );
        assert_eq!(
            t.dot(Dot::Stuck),
            robroz_card::palette(robroz_card::Tone::Amber).fill
        );
        assert_eq!(t.dot(Dot::Void), [0x9a, 0x97, 0x8f]);
        let d = Skin::dark();
        assert_eq!(d.dot(Dot::Done), [0x4f, 0x9a, 0x53]);
        assert_eq!(d.dot(Dot::Void), [0x6f, 0x6c, 0x66]);
    }

    #[test]
    fn status_outline_only_in_dark() {
        assert_eq!(Skin::light().palette.status_outline, None);
        assert_eq!(
            Skin::dark().palette.status_outline,
            Some([0x3f, 0x44, 0x4d])
        );
        assert_eq!(Skin::dark().stroke.status_outline, 1.0);
    }

    #[test]
    fn chat_palette_is_the_designers_in_both_modes() {
        let l = Skin::light().chat;
        assert_eq!(l.own_bg, [0xb8, 0x4d, 0x1b]);
        assert_eq!(l.own_text, [0xff, 0xff, 0xff]);
        assert_eq!(l.mine_size, 15.0);
        assert_eq!(l.composer_radius, 20.0);
        assert_eq!(l.thumb_size, 56.0);
        assert_eq!(l.bro_bg, tokens::LIGHT.surface);
        assert_eq!(l.bro_border, tokens::LIGHT.border);
        assert_eq!(l.accent, [0xc8, 0x57, 0x2a]);
        assert_eq!(l.radius, 18.0);
        assert_eq!(l.avatar, 24.0);
        assert_eq!(l.group_secs, 300);
        let d = Skin::dark();
        assert_eq!(d.palette.accent, [0xc8, 0x55, 0x1f]);
        assert_eq!(d.palette.accent_strong, [0xb8, 0x4d, 0x1b]);
        assert_eq!(d.chat.own_bg, [0xb8, 0x4d, 0x1b]);
        assert_eq!(d.chat.own_text, [0xff, 0xff, 0xff]);
        assert_eq!(d.chat.bro_bg, tokens::DARK.surface);
        assert_eq!(d.chat.running_fill, [0x2c, 0x2f, 0x33]);
        assert_eq!(d.chat.running_text, [0xec, 0xe8, 0xdf]);
        assert_eq!(l.running_fill, [0x1f, 0x21, 0x24]);
        assert_eq!(d.chat.accent, [0xe0, 0x74, 0x3f]);
        assert_eq!(d.palette.window, [0x17, 0x18, 0x1a]);
        assert_eq!(d.palette.sidebar, [0x1d, 0x1e, 0x21]);
        assert_eq!(d.palette.divider, [0x2f, 0x32, 0x36]);
        assert_eq!(d.palette.text, [0xec, 0xe8, 0xdf]);
        assert_eq!(d.palette.text2, [0x8f, 0x8b, 0x83]);
    }

    #[test]
    fn chat_column_is_centred_at_720_with_24px_margins() {
        let t = Skin::light();
        assert_eq!(t.chat_column_width(1000.0), 720.0);
        assert_eq!(t.chat_column_width(400.0), 352.0);
        assert_eq!(t.chat_gutter(), 32.0);
        // 1280 window, 720 column: own bubble 68% of the column; cards fill
        // the column minus the 24+8 avatar gutter.
        assert_eq!(t.chat_widths(1280.0, 720.0), (489.6, 688.0));
        // Below 760: 80% for the own bubble; cards still fill the text column.
        assert_eq!(t.chat_widths(720.0, 352.0), (281.6, 320.0));
    }

    #[test]
    fn spec_metrics() {
        let t = Skin::light();
        assert_eq!(t.radii.card, 8.0);
        assert_eq!(t.radii.button, 6.0);
        assert_eq!(t.controls.button_h, 32.0);
        assert_eq!(t.controls.switch_w, 36.0);
        assert_eq!(t.stroke.selected, 1.5);
        assert_eq!(t.material, MaterialKind::Solid);
        assert_eq!(t.controls.sidebar_w, 220.0);
        assert!(t.shadows.card.is_none(), "solid version has no shadow");
        assert_eq!(t.controls.touch_target, 32.0, "desktop skin");
    }

    #[test]
    fn touch_floor_only_on_touch_or_narrow() {
        let t = Skin::light();
        assert_eq!(t.for_input(false, 1040.0).controls.touch_target, 32.0);
        assert_eq!(t.for_input(false, 481.0).controls.touch_target, 32.0);
        assert_eq!(t.for_input(false, 480.0).controls.touch_target, TOUCH_FLOOR);
        assert_eq!(t.for_input(false, 390.0).controls.touch_target, TOUCH_FLOOR);
        assert_eq!(t.for_input(true, 1040.0).controls.touch_target, TOUCH_FLOOR);
        assert!(
            !t.for_input(true, 1040.0).narrow,
            "touch alone is not narrow"
        );
        assert!(t.for_input(false, 480.0).narrow);
        let narrow_desktop = t.for_input(false, 390.0);
        assert!(narrow_desktop.narrow && !narrow_desktop.phone && narrow_desktop.can_edit());
        let mut phone = Skin::light();
        phone.phone = true;
        assert!(!phone.for_input(false, 1040.0).can_edit());
        assert!(!phone.for_input(false, 1040.0).narrow);
        // A skin above the floor (material 48) keeps its own value.
        let mut material = Skin::light();
        material.controls.touch_target = 48.0;
        assert_eq!(material.for_input(true, 390.0).controls.touch_target, 48.0);
    }

    #[test]
    fn mode_from_env() {
        assert_eq!(Mode::from_env_value(Some("dark")), Mode::Dark);
        assert_eq!(Mode::from_env_value(Some(" DARK ")), Mode::Dark);
        assert_eq!(Mode::from_env_value(Some("light")), Mode::Light);
        assert_eq!(Mode::from_env_value(None), Mode::Light);
        assert_eq!(Mode::Light.toggled(), Mode::Dark);
        assert_eq!(Skin::for_mode(Mode::Dark).mode, Mode::Dark);
    }
}
