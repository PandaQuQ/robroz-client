//! iced views for bro avatars: the shape glyph, the stacked title
//! avatars, the picker, and the create-bro and avatar pages. Geometry and
//! colours come from [`crate::avatar`]; sizes from `robroz-tokens`
//! (`base.avatar`).

use iced::widget::image;
use iced::widget::{button, column, container, row, space, text, text_input};
use iced::{
    Alignment, Border, Element, Fill, Length, Padding, Shadow as IcedShadow, Theme as IcedTheme,
};
use robroz_tokens::base::avatar as tok;

use crate::avatar::{self, BroAvatar, COLORS, SHAPES};
use crate::i18n::Key;
use crate::theme::{Mode, Rgb, Skin};
use crate::view::{bold, color, faded, Action};

/// Widget id of the create-bro name field.
pub const NEW_BRO_NAME: &str = "robroz-new-bro-name";

/// Raster scale: glyphs are drawn at this many pixels per logical pixel
/// and filtered down, so they stay sharp on 2x screens.
const RASTER_SCALE: f32 = 2.0;

type GlyphKey = (BroAvatar, bool, u32);

/// One bro avatar as an image: the filled shape and two round eyes,
/// drawn with tiny-skia ([`avatar::rasterize`]).
///
/// Not a `canvas`: iced 0.14's wgpu backend drops most canvas meshes when
/// several sit over quads in one layer (picker cells, preview, list rows),
/// while images draw reliably on both backends. Handles are cached so a
/// glyph is rasterised and uploaded once.
fn glyph_handle(a: BroAvatar, dark: bool, size: f32) -> image::Handle {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<GlyphKey, image::Handle>>> = OnceLock::new();
    let key: GlyphKey = (a, dark, size.to_bits());
    let cache = CACHE.get_or_init(Default::default);
    if let Some(h) = cache.lock().expect("glyph cache").get(&key) {
        return h.clone();
    }
    let px = (size * RASTER_SCALE).round().max(1.0) as u32;
    let rgba = avatar::rasterize(a, dark, None, px);
    let h = image::Handle::from_rgba(px, px, rgba);
    cache.lock().expect("glyph cache").insert(key, h.clone());
    h
}

fn glyph<'a, M: 'a>(a: BroAvatar, dark: bool, size: f32) -> Element<'a, M> {
    image(glyph_handle(a, dark, size))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

fn dark(t: &Skin) -> bool {
    t.mode == Mode::Dark
}

/// A bro avatar of `size` px.
pub fn bro_avatar<'a, M: 'a>(t: &Skin, a: BroAvatar, size: f32) -> Element<'a, M> {
    glyph(a, dark(t), size)
}

/// Title-capsule avatars: 24px, each overlapping the previous by 8px,
/// the first on top, with a 2px band in `bg` so overlaps stay readable;
/// then `+N`. Drawn as one bitmap so the stacking order is the same on
/// every renderer.
pub fn avatar_stack<'a, M: 'a>(
    t: &Skin,
    avatars: &[BroAvatar],
    more: usize,
    bg: Rgb,
) -> Element<'a, M> {
    avatar_stack_at(t, avatars, more, bg, tok::TITLE_SIZE, tok::TITLE_OVERLAP)
}

/// Stacked avatars at a given `size` and overlap. Used for the 16px
/// working-row stack as well as the 24px title capsule.
pub fn avatar_stack_at<'a, M: 'a>(
    t: &Skin,
    avatars: &[BroAvatar],
    more: usize,
    bg: Rgb,
    size: f32,
    overlap: f32,
) -> Element<'a, M> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    type Key = (Vec<BroAvatar>, bool, Rgb, u32, u32);
    static CACHE: OnceLock<Mutex<HashMap<Key, image::Handle>>> = OnceLock::new();
    let width = avatar::stack_width(avatars.len(), size, overlap);
    let key: Key = (
        avatars.to_vec(),
        dark(t),
        bg,
        size.to_bits(),
        overlap.to_bits(),
    );
    let cache = CACHE.get_or_init(Default::default);
    let cached = cache.lock().expect("stack cache").get(&key).cloned();
    let handle = cached.unwrap_or_else(|| {
        let px = (size * RASTER_SCALE).round() as u32;
        let step = ((size - overlap) * RASTER_SCALE).round() as u32;
        let (w, rgba) = avatar::rasterize_stack(
            avatars,
            dark(t),
            (
                bg,
                tok::TITLE_RING * RASTER_SCALE * (size / tok::TITLE_SIZE),
            ),
            px,
            step,
        );
        let h = image::Handle::from_rgba(w, px, rgba);
        cache.lock().expect("stack cache").insert(key, h.clone());
        h
    });
    let mut r = row![image(handle)
        .width(Length::Fixed(width))
        .height(Length::Fixed(size))]
    .spacing(4)
    .align_y(Alignment::Center);
    if more > 0 {
        r = r.push(
            text(format!("+{more}"))
                .size(t.type_.foot)
                .color(color(t.palette.text2)),
        );
    }
    r.into()
}

fn round_button_style(t: Skin) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => t.palette.control_hover,
            _ => t.palette.surface,
        };
        button::Style {
            background: Some(color(bg).into()),
            text_color: color(t.palette.text),
            border: Border::default()
                .width(t.stroke.card)
                .color(color(t.palette.border))
                .rounded(tok::MORE_BUTTON / 2.0),
            shadow: IcedShadow::default(),
            snap: false,
        }
    }
}

/// Round 32px button with a 16px line icon (`•••`, `‹`).
pub fn round_button<'a>(t: &Skin, glyph: crate::icons::Glyph, on: Action) -> Element<'a, Action> {
    let s = tok::MORE_BUTTON;
    button(
        container(crate::icons::icon(
            glyph,
            t.palette.text,
            robroz_tokens::home::MORE_ICON,
            RASTER_SCALE,
        ))
        .width(Length::Fixed(s))
        .height(Length::Fixed(s))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center),
    )
    .padding(0)
    .width(Length::Fixed(s))
    .height(Length::Fixed(s))
    .style(round_button_style(*t))
    .on_press(on)
    .into()
}

/// The capsule behind the chat title: surface fill, fully rounded.
pub fn capsule_style(t: Skin) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| container::Style {
        background: Some(color(t.palette.surface).into()),
        border: Border::default()
            .width(t.stroke.card)
            .color(color(t.palette.border))
            .rounded(tok::TITLE_HEIGHT / 2.0),
        ..container::Style::default()
    }
}

/// Inline note under the preview (save or create errors). One line.
fn preview_note<'a>(t: &Skin, note: Option<&'a str>) -> Element<'a, Action> {
    match note {
        Some(n) => container(
            text(n.to_string())
                .size(t.type_.foot)
                .color(color(crate::theme::DANGER)),
        )
        .width(Fill)
        .center_x(Fill)
        .into(),
        None => space::vertical().height(0).into(),
    }
}

/// 160px preview tile, radius 32, with a 112px avatar.
pub fn avatar_preview<'a, M: 'a>(t: &Skin, a: BroAvatar) -> Element<'a, M> {
    let sunken = t.palette.sunken;
    container(bro_avatar(t, a, tok::PREVIEW_GLYPH))
        .width(Length::Fixed(tok::PREVIEW))
        .height(Length::Fixed(tok::PREVIEW))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: Some(color(sunken).into()),
            border: Border::default().rounded(tok::PREVIEW_RADIUS),
            ..container::Style::default()
        })
        .into()
}

fn section_label<'a>(t: &Skin, key: Key) -> Element<'a, Action> {
    text(key.as_str())
        .size(t.type_.foot)
        .color(color(t.palette.text3))
        .into()
}

fn cell_style(t: Skin, selected: bool) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => t.palette.control_hover,
            _ => t.palette.surface,
        };
        let border = if selected {
            Border::default()
                .width(tok::SELECTED_RING)
                .color(color(t.palette.text))
        } else {
            Border::default()
                .width(t.stroke.card)
                .color(color(t.palette.border))
        };
        button::Style {
            background: Some(color(bg).into()),
            text_color: color(t.palette.text),
            border: border.rounded(tok::SHAPE_CELL / 4.0),
            shadow: IcedShadow::default(),
            snap: false,
        }
    }
}

/// 12 shapes, six per row, 56px cells (the selected one has a 2px `text`
/// ring); then 10 colour circles, 32px inside a 44px hit area. Each cell
/// shows the shape in the current colour.
pub fn avatar_picker<'a>(t: &Skin, current: BroAvatar) -> Element<'a, Action> {
    let cell = tok::SHAPE_CELL;
    let cols = tok::SHAPE_COLS as usize;
    let gap = (tok::PAGE_MAX - cell * cols as f32) / (cols as f32 - 1.0);
    let mut grid = column![].spacing(gap.min(16.0));
    let mut line = row![].spacing(gap);
    for s in 0..SHAPES {
        let selected = s == current.shape;
        let b = button(
            container(bro_avatar(
                t,
                BroAvatar {
                    shape: s,
                    color: current.color,
                },
                tok::SHAPE_GLYPH,
            ))
            .width(Fill)
            .height(Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        )
        .padding(0)
        .width(Length::Fixed(cell))
        .height(Length::Fixed(cell))
        .style(cell_style(*t, selected))
        .on_press(Action::AvatarShape(s));
        line = line.push(b);
        if (usize::from(s) + 1) % cols == 0 {
            grid = grid.push(line);
            line = row![].spacing(gap);
        }
    }
    // 12 shapes and 6 columns fill complete rows; don't push an empty
    // trailing row, or its column spacing adds a phantom 16px gap.
    if usize::from(SHAPES) % cols != 0 {
        grid = grid.push(line);
    }

    let swatch = tok::SWATCH;
    let hit = tok::SWATCH_HIT;
    let ring = tok::SELECTED_RING;
    let mut swatches = row![].spacing(0);
    for c in 0..COLORS {
        let selected = c == current.color;
        let fill = avatar::color(c, dark(t));
        let dot = container(space::horizontal().width(0))
            .width(Length::Fixed(swatch))
            .height(Length::Fixed(swatch))
            .style(move |_| container::Style {
                background: Some(color(fill).into()),
                border: Border::default().rounded(swatch / 2.0),
                ..container::Style::default()
            });
        let outer = swatch + 4.0 * ring;
        let text_c = t.palette.text;
        let ringed = container(dot)
            .width(Length::Fixed(outer))
            .height(Length::Fixed(outer))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(move |_| container::Style {
                border: if selected {
                    Border::default()
                        .width(ring)
                        .color(color(text_c))
                        .rounded(outer / 2.0)
                } else {
                    Border::default()
                },
                ..container::Style::default()
            });
        swatches = swatches.push(
            button(
                container(ringed)
                    .width(Fill)
                    .height(Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .padding(0)
            .width(Length::Fixed(hit))
            .height(Length::Fixed(hit))
            .style(|_, _| button::Style::default())
            .on_press(Action::AvatarColor(c)),
        );
    }
    column![
        section_label(t, Key::AvatarShape),
        grid,
        // Column spacing (8px) sits on both sides of this spacer, so
        // SECTION_GAP - 16 makes the group gap 24px. Label-to-content
        // stays the column's 8px.
        space::vertical().height(tok::SECTION_GAP - 16.0),
        section_label(t, Key::AvatarColor),
        swatches.wrap(),
    ]
    .spacing(8)
    .into()
}

fn name_input_style(t: Skin) -> impl Fn(&IcedTheme, text_input::Status) -> text_input::Style {
    move |_, status| {
        let border = match status {
            text_input::Status::Focused { .. } => t.palette.focus,
            _ => t.palette.border,
        };
        let sel = t.palette.selection;
        text_input::Style {
            background: color(t.palette.surface).into(),
            border: Border::default()
                .width(t.stroke.card)
                .color(color(border))
                .rounded(tok::NAME_RADIUS),
            icon: color(t.palette.text),
            placeholder: color(t.palette.text3),
            value: color(t.palette.text),
            selection: faded(sel.rgb, sel.alpha),
        }
    }
}

fn primary_style(t: Skin, alpha: f32) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, _| button::Style {
        background: Some(faded(t.palette.accent_strong, alpha).into()),
        text_color: faded(t.palette.on_accent, alpha),
        border: Border::default().rounded(tok::NAME_RADIUS),
        shadow: IcedShadow::default(),
        snap: false,
    }
}

fn secondary_style(t: Skin) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, status| {
        let bg = match status {
            button::Status::Hovered | button::Status::Pressed => t.palette.control_hover,
            _ => t.palette.surface,
        };
        button::Style {
            background: Some(color(bg).into()),
            text_color: color(t.palette.text),
            border: Border::default()
                .width(t.stroke.card)
                .color(color(t.palette.border))
                .rounded(tok::NAME_RADIUS),
            shadow: IcedShadow::default(),
            snap: false,
        }
    }
}

fn page_head<'a>(t: &Skin, title: String, trailing: Element<'a, Action>) -> Element<'a, Action> {
    row![
        text(title)
            .size(t.type_.list_title)
            .font(bold())
            .color(color(t.palette.text)),
        space::horizontal(),
        trailing,
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(tok::TITLE_HEIGHT))
    .into()
}

fn page_frame<'a>(t: &Skin, body: iced::widget::Column<'a, Action>) -> Element<'a, Action> {
    iced::widget::scrollable(
        container(container(body).max_width(tok::PAGE_MAX))
            .width(Fill)
            .center_x(Fill)
            .padding(t.metrics.stream_padding),
    )
    .height(Fill)
    .width(Fill)
    .into()
}

/// The create-bro page: preview, name, shape and colour, then 引入.
/// 引入 is at 40% opacity and inert while the name is empty.
pub fn create_bro_page<'a>(
    t: &Skin,
    name: &'a str,
    a: BroAvatar,
    note: Option<&'a str>,
    busy: bool,
) -> Element<'a, Action> {
    let can = !busy && !name.trim().is_empty();
    let mut input = text_input(Key::CreateBroNamePlaceholder.as_str(), name)
        .id(NEW_BRO_NAME)
        .size(16)
        .padding(Padding::from([0.0, 16.0]))
        .line_height(iced::widget::text::LineHeight::Absolute(iced::Pixels(
            tok::NAME_HEIGHT - 2.0 * t.stroke.card,
        )))
        .style(name_input_style(*t));
    if !busy {
        input = input.on_input(|text| Action::NewBroName { text });
        if can {
            input = input.on_submit(Action::NewBroCreate);
        }
    }
    let alpha = if can { 1.0 } else { tok::CREATE_EMPTY_OPACITY };
    let mut create = button(
        container(text(Key::CreateBroCreate.as_str()).size(t.type_.body))
            .width(Fill)
            .height(Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
    )
    .padding(0)
    .width(Fill)
    .height(Length::Fixed(tok::NAME_HEIGHT))
    .style(primary_style(*t, alpha));
    if can {
        create = create.on_press(Action::NewBroCreate);
    }
    let cancel = button(text(Key::ViewRenameCancel.as_str()).size(t.type_.body))
        .padding(Padding::from([6.0, 12.0]))
        .style(secondary_style(*t))
        .on_press(Action::AvatarPageClose);
    let body = column![
        page_head(
            t,
            Key::ConversationsNewBro.as_str().to_string(),
            cancel.into()
        ),
        container(avatar_preview(t, a)).width(Fill).center_x(Fill),
        preview_note(t, note),
        input,
        avatar_picker(t, a),
        create,
    ]
    .spacing(tok::SECTION_GAP);
    page_frame(t, body)
}

/// A bro's avatar page (owner only): preview, name, picker; each pick
/// saves at once. 「用默认头像」 only while an avatar is stored.
pub fn avatar_page<'a>(
    t: &Skin,
    name: &'a str,
    a: BroAvatar,
    stored: bool,
    note: Option<&'a str>,
) -> Element<'a, Action> {
    let back = round_button(t, crate::icons::Glyph::Back, Action::AvatarPageClose);
    let mut body = column![
        page_head(t, Key::AvatarTitle.as_str().to_string(), back),
        container(avatar_preview(t, a)).width(Fill).center_x(Fill),
        preview_note(t, note),
        container(
            text(name.to_string())
                .size(16)
                .font(bold())
                .color(color(t.palette.text))
        )
        .width(Fill)
        .center_x(Fill),
        avatar_picker(t, a),
    ]
    .spacing(tok::SECTION_GAP);
    if stored {
        body = body.push(
            button(
                container(text(Key::AvatarUseDefault.as_str()).size(t.type_.body))
                    .width(Fill)
                    .height(Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .padding(0)
            .width(Fill)
            .height(Length::Fixed(tok::NAME_HEIGHT))
            .style(secondary_style(*t))
            .on_press(Action::AvatarReset),
        );
    }
    page_frame(t, body)
}
