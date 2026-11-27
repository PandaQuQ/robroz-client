//! iced widgets for the new cards and the routines panel.
//!
//! Every colour, radius, border, padding and size comes from [`Skin`].
//! Paths and addresses are plain monospace text, never links. Widgets only
//! emit [`Action`]s; the app decides what to call.

use chrono::Local;
use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::{
    button, canvas, column, container, hover, mouse_area, pick_list, row, scrollable, space, text,
    text_input, tooltip, Space,
};
use iced::{
    font, Alignment, Background, Border, Color, Element, Fill, Font, Length, Padding, Pixels,
    Shadow as IcedShadow, Theme as IcedTheme, Vector,
};

use crate::cards::confirm::{self, ConfirmCard};
use crate::cards::group_approval;
use crate::cards::interact::{self, ApprovalActions, CardUi, Look, Note, OptionButton};
use crate::cards::mcp::{self, Book as McpBook};
use crate::cards::memories::{self, Book};
use crate::cards::reminder::Reminder;
use crate::i18n::{host_text, Key};

use crate::cards::model;
use crate::cards::review_rules;

use crate::cards::routines::{self, Routines};
use crate::cards::search;
use crate::cards::secret;
use crate::cards::skills::{self, Book as SkillBook};
use crate::cards::status::{self, Dot};
use crate::cards::{split_call_summary, ApprovalCard, Card, ChoiceCard, DraftCard};
use crate::conversations::{self, Conversation, Conversations};
use robroz_contract::{McpSecretIn, McpServer};

use crate::theme::{Mode, Rgb, Skin};
use crate::widgets::{CheckBox, Rings, Toggle};

/// What a click on a new card or the routines panel asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Single choice: send this option.
    Choose {
        card: String,
        option: String,
    },
    /// Multi choice: flip a pick locally.
    TogglePick {
        card: String,
        option: String,
    },
    /// Multi choice: send the picks.
    ConfirmChoice {
        card: String,
    },
    Decide {
        card: String,
        approve: bool,
    },
    SendDraft {
        card: String,
    },
    /// Open the bot's DM with the draft; no editor in the group.
    EditDraftInDm {
        card: String,
    },
    /// Stuck card: go to the DM to take over.
    TakeOver {
        card: String,
    },
    /// The owner's DM reminder row: open the group and its approval card.
    OpenApproval {
        group: String,
        card: String,
    },
    TogglePause {
        id: String,
    },
    /// Narrow screens: show or hide the full bot name on a card.
    ToggleBotName {
        card: String,
    },
    /// Conversation list: open this group or DM.
    OpenChat {
        group: String,
    },
    /// 「引入一个 bro」: open the create-bro page.
    NewBro,
    /// Create-bro page: the name field changed.
    NewBroName {
        text: String,
    },
    /// Create-bro page: 引入 (only while the name is not empty).
    NewBroCreate,
    /// Leave the create-bro or avatar page.
    AvatarPageClose,
    /// Title capsule `…` in a DM you own: open that bro's avatar page.
    AvatarPageOpen,
    /// Picker: shape `0..12` (create page draft, or a save on the avatar page).
    AvatarShape(u8),
    /// Picker: colour `0..10`.
    AvatarColor(u8),
    /// Avatar page: clear the stored avatar (`null`) and use the id default.
    AvatarReset,
    /// Narrow screens: back from a chat to the list.
    BackToList,
    /// Chat title `…`: open or close the group / DM menu.
    GroupMenuOpen,
    GroupMenuClose,
    GroupProfileOpen,
    GroupProfileClose,
    FindInChatOpen,
    FindInChatClose,
    FindInChatEdit {
        text: String,
    },
    FindInChatSubmit,
    /// Group profile: rename the group (not a bro).
    GroupEditStart,
    GroupEdit {
        text: String,
    },
    GroupEditSave,
    GroupEditCancel,
    GroupDeleteAsk,
    /// Menu Rename: open the profile with the name box ready.
    GroupRenameOpen,
    /// Menu Members: open the member list (back goes to the profile).
    GroupMembersOpen,
    GroupDeleteConfirm,
    GroupDeleteCancel,
    GroupCopyId,
    MembersOpen,
    MembersClose,
    SelfMenuOpen,
    SelfMenuClose,
    OpenSettings,
    /// A Settings row (PR5).
    SettingsGo {
        dest: crate::settings::Dest,
    },
    /// Round ‹ on a Settings choice or connection page: back to Settings.
    SettingsBack,
    /// Option `index` picked on a Settings choice page.
    SettingsChoose {
        index: usize,
    },
    OpenRoutines,
    SignOut,
    HomeSearchEdit {
        text: String,
    },
    /// Composer text changed.
    ComposerEdit {
        text: String,
    },
    /// Enter or 发送 in the composer.
    ComposerSend,
    /// Bro DM header: click the name to rename the bro.
    RenameStart,
    RenameEdit {
        text: String,
    },
    /// Enter in the rename box.
    RenameSave,
    /// Esc or 取消.
    RenameCancel,
    /// 记忆 page: open or close the 记一条 form.
    MemoryAddOpen,
    MemoryAddClose,
    MemoryAddText {
        text: String,
    },
    MemoryAddScope {
        scope: String,
    },
    MemoryAddKind {
        kind: robroz_contract::MemoryKind,
    },
    MemoryAddSave,
    /// 改: edit this memory's text in place.
    MemoryEdit {
        id: String,
    },
    MemoryEditText {
        text: String,
    },
    MemoryEditSave,
    MemoryEditCancel,
    /// 忘掉 once asks; the second 忘掉 deletes.
    MemoryForgetAsk {
        id: String,
    },
    MemoryForgetConfirm,
    MemoryForgetCancel,
    /// 记忆 page, pending entry: 记下 (`keep`) or 不用, through its confirm card.
    MemoryDecide {
        id: String,
        keep: bool,
    },
    /// Skill confirm card: open or close the full instruction.
    ToggleInstruction {
        card: String,
    },
    /// 「还有 N 条待确认」: show every pending confirm card of this task.
    UnfoldConfirms {
        task: String,
    },
    /// 技能 page: open or close the 加一项 form.
    SkillAddOpen,
    SkillAddClose,
    SkillAddSlug {
        text: String,
    },
    SkillAddTitle {
        text: String,
    },
    SkillAddInstruction {
        text: String,
    },
    SkillAddScope {
        scope: String,
    },
    SkillAddSave,
    /// 改: edit this skill's title and instruction in place.
    SkillEdit {
        slug: String,
    },
    SkillEditTitle {
        text: String,
    },
    SkillEditInstruction {
        text: String,
    },
    SkillEditSave,
    SkillEditCancel,
    /// 删除 once asks; the second 删除 forgets the skill.
    SkillDeleteAsk {
        slug: String,
    },
    SkillDeleteConfirm,
    SkillDeleteCancel,
    /// 技能 page, pending entry: 保存 (`keep`) or 不用, through its confirm card.
    SkillDecide {
        slug: String,
        keep: bool,
    },
    /// Open or close a long instruction on the 技能 page.
    SkillToggleInstruction {
        slug: String,
    },
    /// Enable or disable a kept skill in this group.
    SkillToggleGroup {
        slug: String,
        group: String,
    },
    /// MCP page: open the 加一台 form.
    McpAddOpen,
    McpFormClose,
    McpFormName {
        text: String,
    },
    McpFormTransport {
        http: bool,
    },
    McpFormCommand {
        text: String,
    },
    McpFormArgs {
        text: String,
    },
    McpFormUrl {
        text: String,
    },
    McpSecretName {
        index: usize,
        text: String,
    },
    McpSecretPlacement {
        index: usize,
        header: bool,
    },
    McpSecretKey {
        index: usize,
        text: String,
    },
    McpSecretClear {
        index: usize,
    },
    McpSecretCancel {
        index: usize,
    },
    McpSecretAdd,
    McpSecretRemove {
        index: usize,
    },
    McpSave,
    McpEdit {
        id: String,
    },
    /// Open or close a server's tool list.
    McpOpen {
        id: String,
    },
    McpDeleteAsk {
        id: String,
    },
    McpDeleteConfirm,
    McpDeleteCancel,
    /// Flip the server's own enable switch.
    McpToggleEnabled {
        id: String,
    },
    McpToggleGroup {
        id: String,
        group: String,
    },
    /// Confirm the stdio command after 409 `command_not_approved`.
    McpApproveCommand {
        id: String,
    },
    McpCancelCommand {
        id: String,
    },
    /// 联网搜索 page.
    SearchPick {
        provider: search::Provider,
    },
    SearchUrl {
        text: String,
    },
    SearchKey {
        text: String,
    },
    SearchClearKey,
    SearchCancelKey,
    SearchSave,

    /// 放行规则 page.
    ReviewDraft {
        text: String,
    },
    ReviewAdd,
    ReviewDelete {
        id: String,
    },
    /// 模型 page.
    ModelPick {
        preset: model::Preset,
    },
    ModelUrl {
        text: String,
    },
    ModelName {
        text: String,
    },
    ModelKey {
        text: String,
    },
    ModelClearKey,
    ModelCancelKey,
    ModelSave,
}

/// Monospace advance as a fraction of font size (layout estimate only).
const MONO_ADVANCE: f32 = 0.6;

pub fn color(c: Rgb) -> Color {
    Color::from_rgb8(c[0], c[1], c[2])
}

pub(crate) fn faded(c: Rgb, alpha: f32) -> Color {
    let mut c = color(c);
    c.a = alpha;
    c
}

/// The UI font family. macOS and Windows name a Chinese-capable system
/// font that fontdb can actually index, so every glyph of a CJK string
/// comes from one font; left to the generic family (or a family fontdb
/// cannot open), the text engine falls back glyph by glyph and mixes
/// weights and even picks look-alike wrong characters (「发送」 → 「发遞」).
///
/// On macOS this is Hiragino Sans GB, from
/// `/System/Library/Fonts/Hiragino Sans GB.ttc` (W3 regular, W6 bold).
/// PingFang SC is not used: on recent macOS it lives in a hidden asset
/// path that fontdb does not index, so naming it fell back per glyph.
/// On Windows this is Microsoft YaHei UI. Elsewhere (Linux) it stays
/// the generic sans-serif.
pub const fn ui_family() -> font::Family {
    #[cfg(target_os = "macos")]
    {
        font::Family::Name("Hiragino Sans GB")
    }
    #[cfg(target_os = "windows")]
    {
        font::Family::Name("Microsoft YaHei UI")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        font::Family::SansSerif
    }
}

/// Regular-weight UI font; also the application's default font.
pub const fn regular() -> Font {
    Font {
        family: ui_family(),
        ..Font::DEFAULT
    }
}

/// Bold UI font, same family as [`regular`]. On macOS, `Weight::Bold`
/// should resolve to the W6 face in `Hiragino Sans GB.ttc`.
pub fn bold() -> Font {
    Font {
        weight: font::Weight::Bold,
        ..regular()
    }
}

/// Weight 600, same family as [`regular`]. Primary buttons use this so
/// the face stays the UI font (Bold can pick a heavier, different face).
pub fn semibold() -> Font {
    Font {
        weight: font::Weight::Semibold,
        ..regular()
    }
}

#[cfg(test)]
mod font_tests {
    use super::{bold, regular, ui_family};
    use iced::font::{self, Family};

    #[test]
    fn ui_family_is_a_fontdb_discoverable_system_font() {
        #[cfg(target_os = "macos")]
        assert_eq!(ui_family(), Family::Name("Hiragino Sans GB"));
        #[cfg(target_os = "windows")]
        assert_eq!(ui_family(), Family::Name("Microsoft YaHei UI"));
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        assert_eq!(ui_family(), Family::SansSerif);
    }

    #[test]
    fn bold_uses_the_same_family_as_regular() {
        assert_eq!(regular().family, ui_family());
        assert_eq!(bold().family, ui_family());
        assert_eq!(bold().weight, font::Weight::Bold);
    }
}

fn shadow(t: &Skin) -> IcedShadow {
    match t.shadows.card {
        Some(s) => IcedShadow {
            color: faded(s.color, s.alpha),
            offset: Vector::new(s.offset.0, s.offset.1),
            blur_radius: s.blur,
        },
        None => IcedShadow::default(),
    }
}

fn gap<'a, M: 'a>(h: f32) -> Element<'a, M> {
    Space::new().height(Length::Fixed(h)).into()
}

// ---------- Surfaces used by the app shell too ----------

/// Page background (message stream, routines page).
pub fn page_style(t: Skin) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| container::Style {
        background: Some(color(t.palette.window).into()),
        text_color: Some(color(t.palette.text)),
        ..container::Style::default()
    }
}

/// Sidebar: surface with a right divider look.
pub fn sidebar_style(t: Skin) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| container::Style {
        background: Some(color(t.palette.sidebar).into()),
        text_color: Some(color(t.palette.text)),
        border: Border::default()
            .width(t.stroke.divider)
            .color(color(t.palette.divider)),
        ..container::Style::default()
    }
}

/// Sidebar entry.
pub fn nav_button_style(
    t: Skin,
    selected: bool,
) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, status| {
        let bg = if selected {
            Some(Background::Color(color(t.palette.control_checked)))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(Background::Color(color(t.palette.control_hover)))
        } else {
            None
        };
        button::Style {
            background: bg,
            text_color: color(t.palette.text),
            border: Border::default().rounded(t.radii.row),
            ..button::Style::default()
        }
    }
}

/// Fill and text of a selected Language / Appearance segment. Dark uses
/// the solid-skin line colour ([`robroz_tokens::DARK`] `border`, the same
/// value as `divider` in the tokens) and `text`. Light keeps the skin's
/// `control_checked` and `text`.
fn segment_selected(t: &Skin) -> (Rgb, Rgb) {
    match t.mode {
        Mode::Dark => (robroz_tokens::DARK.border, robroz_tokens::DARK.text),
        Mode::Light => (t.palette.control_checked, t.palette.text),
    }
}

/// Segmented control (Language / Appearance rows). Same shape as
/// [`nav_button_style`]; the dark selected state uses
/// [`segment_selected`]. Sidebar nav keeps [`nav_button_style`].
pub fn segment_button_style(
    t: Skin,
    selected: bool,
) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, status| {
        let (bg, text) = if selected {
            let (fill, text) = segment_selected(&t);
            (Some(Background::Color(color(fill))), color(text))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            (
                Some(Background::Color(color(t.palette.control_hover))),
                color(t.palette.text),
            )
        } else {
            (None, color(t.palette.text))
        };
        button::Style {
            background: bg,
            text_color: text,
            border: Border::default().rounded(t.radii.row),
            ..button::Style::default()
        }
    }
}

#[cfg(test)]
mod segment_tests {
    use super::segment_selected;
    use crate::theme::Skin;

    #[test]
    fn dark_selected_segment_uses_the_dark_line_and_text_tokens() {
        let t = Skin::dark();
        let (fill, text) = segment_selected(&t);
        assert_eq!(fill, robroz_tokens::DARK.border);
        assert_eq!(text, robroz_tokens::DARK.text);
        // Brighter than both the card and the old near-black checked fill,
        // so the chip no longer reads as a hole.
        assert!(luma(fill) > luma(t.palette.surface));
        assert!(luma(fill) > luma(t.palette.control_checked));
    }

    #[test]
    fn light_selected_segment_keeps_the_existing_tokens() {
        let t = Skin::light();
        let (fill, text) = segment_selected(&t);
        assert_eq!(fill, t.palette.control_checked);
        assert_eq!(text, t.palette.text);
    }

    fn luma(c: [u8; 3]) -> u32 {
        // Rec. 709, integer; only used to compare relative brightness.
        u32::from(c[0]) * 299 + u32::from(c[1]) * 587 + u32::from(c[2]) * 114
    }
}

/// New-card frame: `surface`, 1px `border`, radius from the skin.
pub fn card_style(t: Skin) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| container::Style {
        background: Some(color(t.palette.surface).into()),
        text_color: Some(color(t.palette.text)),
        border: Border::default()
            .width(t.stroke.card)
            .color(color(t.palette.border))
            .rounded(t.radii.card),
        shadow: shadow(&t),
        ..container::Style::default()
    }
}

/// Frozen status card: fill and text from `robroz_card::palette()`
/// unchanged; the skin adds only the dark-mode outline.
pub fn status_card_style(
    t: Skin,
    fill: Rgb,
    text_rgb: Rgb,
) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| {
        let border = match t.palette.status_outline {
            Some(c) => Border::default()
                .width(t.stroke.status_outline)
                .color(color(c)),
            None => Border::default(),
        };
        container::Style {
            text_color: Some(color(text_rgb)),
            background: Some(color(fill).into()),
            border: border.rounded(t.radii.status_card),
            ..container::Style::default()
        }
    }
}

/// Chat bubble: own = accentStrong fill with white text; a bro's = surface
/// with a 1px border and a uniform 18px radius. Own bubbles keep a tighter
/// corner nearest the sender on the first of a run.
pub fn bubble_style(t: Skin, own: bool, first: bool) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| {
        let c = &t.chat;
        // Bro bubbles are a uniform 18px radius on surface. Own bubbles keep
        // the tighter corner nearest the sender on the first of a run.
        let mut radius = iced::border::radius(if own { c.own_radius } else { c.radius });
        if own && first {
            radius = radius.top_right(c.corner);
        }
        let (bg, fg, border) = if own {
            (c.own_bg, c.own_text, Border::default())
        } else {
            (
                c.bro_bg,
                t.palette.text,
                Border::default()
                    .width(t.stroke.card)
                    .color(color(c.bro_border)),
            )
        };
        container::Style {
            background: Some(color(bg).into()),
            text_color: Some(color(fg)),
            border: border.rounded(radius),
            ..container::Style::default()
        }
    }
}

/// Padding inside a bubble (10 / 14).
pub fn bubble_padding(t: &Skin) -> Padding {
    Padding::from([t.chat.pad_v, t.chat.pad_h])
}

/// Attachment chip inside an own bubble: pill with a thin outline.
pub fn chip_style(t: Skin) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| container::Style {
        border: Border::default()
            .width(1.0)
            .color(color(t.chat.own_chip))
            .rounded(999.0),
        ..container::Style::default()
    }
}

/// A bro's avatar in the chat feed (shape and eyes, `chat.avatar` px).
pub fn chat_avatar<'a, M: 'a>(t: &Skin, a: crate::avatar::BroAvatar) -> Element<'a, M> {
    crate::avatar_view::bro_avatar(t, a, t.chat.avatar)
}

pub fn status_card_padding(t: &Skin) -> Padding {
    Padding::from([t.metrics.status_pad_v, t.metrics.status_pad_h])
}

pub fn card_padding(t: &Skin) -> Padding {
    Padding::from([t.controls.card_pad_v, t.controls.card_pad_h])
}

// ---------- Pieces ----------

pub fn status_dot<'a, M: 'a>(t: &Skin, d: Dot) -> Element<'a, M> {
    dot(t, d)
}

/// A status dot in an arbitrary colour (the running card uses the chat accent).
pub fn rgb_dot<'a, M: 'a>(t: &Skin, rgb: Rgb) -> Element<'a, M> {
    let c = color(rgb);
    let size = t.metrics.dot;
    container(Space::new().width(size).height(size))
        .style(move |_| container::Style {
            background: Some(c.into()),
            border: Border::default().rounded(size / 2.0),
            ..container::Style::default()
        })
        .into()
}

/// Take over in DM: outlined like Deny / Edit, 1px line border, ink text.
pub fn takeover_button_style(t: Skin) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: hovered.then_some(color(t.palette.control_hover).into()),
            text_color: color(t.palette.text),
            border: Border::default()
                .width(t.stroke.card)
                .color(color(t.chat.bro_border))
                .rounded(t.radii.button),
            ..button::Style::default()
        }
    }
}

fn dot<'a, M: 'a>(t: &Skin, d: Dot) -> Element<'a, M> {
    let c = color(t.dot(d));
    let size = t.metrics.dot;
    container(Space::new().width(size).height(size))
        .style(move |_| container::Style {
            background: Some(c.into()),
            border: Border::default().rounded(size / 2.0),
            ..container::Style::default()
        })
        .into()
}

fn header<'a>(
    t: &Skin,
    card: &str,
    bot: &'a str,
    ui: &CardUi,
    label: impl Into<String>,
    d: Dot,
    hide_bot: bool,
) -> Element<'a, Action> {
    let status = row![
        dot(t, d),
        text(label.into())
            .size(t.type_.status)
            .color(color(t.palette.text2)),
    ]
    .spacing(t.metrics.dot_gap)
    .align_y(Alignment::Center);
    // In the chat feed the avatar already shows the bro; the first line
    // is only the status (and "Sent to owner's DM" under it on a group
    // approval card).
    if hide_bot {
        return status.into();
    }
    let name = |s: String| {
        text(s)
            .size(t.type_.bot)
            .font(bold())
            .color(color(t.palette.text))
    };
    // The settings page's own cards carry the host's 「设置」 as bot name.
    let bot = host_text(bot);
    let bot = bot.as_ref();
    // Narrow screens: names over 8 characters are cut; tapping toggles the
    // full name. Wide screens always show it whole.
    let bot_name: Element<'a, Action> = match status::short_bot_name(bot).filter(|_| t.narrow) {
        Some(short) => {
            let shown = if ui.name_expanded {
                bot.to_string()
            } else {
                short
            };
            let h = t.metrics.head_height;
            hit_area(
                t,
                name(shown).into(),
                None,
                h,
                Some(Action::ToggleBotName {
                    card: card.to_string(),
                }),
            )
        }
        None => name(bot.to_string()).into(),
    };
    row![bot_name, space::horizontal(), status,]
        .height(Length::Fixed(t.metrics.head_height.max(if t.narrow {
            t.controls.touch_target
        } else {
            0.0
        })))
        .align_y(Alignment::Center)
        .into()
}

fn body<'a, M: 'a>(t: &Skin, s: &'a str) -> Element<'a, M> {
    text(s)
        .size(t.type_.body)
        .line_height(LineHeight::Absolute(Pixels(t.type_.body_line)))
        .color(color(t.palette.text))
        .into()
}

/// Host `call` summary shown verbatim: `{server}.{tool}` in monospace,
/// argument tail clipped to one line with "…". Other actions stay as
/// ordinary body text.
fn approval_summary<'a>(t: &Skin, card: &'a ApprovalCard, inner: f32) -> Element<'a, Action> {
    let Some(parts) = split_call_summary(
        &card.summary,
        card.mcp_server.as_deref(),
        card.mcp_tool.as_deref(),
    ) else {
        return text(host_text(&card.summary).into_owned())
            .size(t.type_.body)
            .line_height(LineHeight::Absolute(Pixels(t.type_.body_line)))
            .color(color(t.palette.text))
            .into();
    };
    // The host writes `MCP_CALL_SUMMARY_PREFIX` server.tool
    // `MCP_CALL_ARGS_SEPARATOR` args; only the words around the tool name
    // are translated.
    let prefix: &str = if parts.prefix == robroz_contract::host_text::MCP_CALL_SUMMARY_PREFIX {
        Key::ViewCallPrefix.as_str()
    } else {
        parts.prefix
    };
    let mut line = row![
        text(prefix)
            .size(t.type_.body)
            .line_height(LineHeight::Absolute(Pixels(t.type_.body_line)))
            .color(color(t.palette.text)),
        text(parts.tool)
            .font(Font::MONOSPACE)
            .size(t.type_.mono)
            .color(color(t.palette.text)),
    ]
    .spacing(0)
    .align_y(Alignment::Center);
    if let Some(args) = parts.args {
        let cols = call_args_cols(t, inner, prefix, parts.tool);
        let args = match args.strip_prefix(robroz_contract::host_text::MCP_CALL_ARGS_SEPARATOR) {
            Some(rest) => format!("{}{rest}", Key::ViewCallArgsSep),
            None => args.to_string(),
        };
        let clipped = status::clamp_lines(&args, cols, 1);
        line = line.push(
            text(clipped)
                .size(t.type_.body)
                .line_height(LineHeight::Absolute(Pixels(t.type_.body_line)))
                .wrapping(Wrapping::None)
                .color(color(t.palette.text)),
        );
    }
    line.into()
}

fn call_args_cols(t: &Skin, inner: f32, prefix: &str, tool: &str) -> usize {
    let unit = |s: &str| -> f32 {
        s.chars()
            .map(|c| if c.is_ascii() { 1.0 } else { 2.0 })
            .sum()
    };
    // No action tag on call cards, so the summary can use the full width.
    let prefix_w = unit(prefix) * t.type_.body * t.type_.unit_width;
    let tool_w = tool.chars().count() as f32 * t.type_.mono * MONO_ADVANCE;
    let room = inner - prefix_w - tool_w;
    (room / (t.type_.body * t.type_.unit_width)).max(4.0) as usize
}

pub(crate) fn footnote<'a, M: 'a>(t: &Skin, s: String) -> Element<'a, M> {
    text(s)
        .size(t.type_.foot)
        .line_height(LineHeight::Absolute(Pixels(t.type_.foot_line)))
        .color(color(t.palette.text3))
        .into()
}

fn approval_footnote_view<'a, M: 'a>(t: &Skin, foot: status::ApprovalFootnote) -> Element<'a, M> {
    let mut r = row![footnote(t, foot.text)].align_y(Alignment::Center);
    if let Some(time) = foot.time {
        r = r.push(footnote(t, time));
    }
    r.spacing(t.controls.button_gap).into()
}

pub(crate) fn tag<'a, M: 'a>(t: &Skin, label: &'static str, fg: Rgb) -> Element<'a, M> {
    let border = t.palette.border2;
    let (bw, r) = (t.stroke.tag, t.radii.tag);
    container(text(label).size(t.type_.tag).color(color(fg)))
        .padding(Padding::from([0.0, t.metrics.tag_pad_h]))
        .height(Length::Fixed(t.metrics.tag_height))
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            border: Border::default().width(bw).color(color(border)).rounded(r),
            ..container::Style::default()
        })
        .into()
}

/// Relative path or address chip. Plain text: not a link, not a button.
/// Path shown in a card: monospace, never a link. On narrow screens it is
/// shortened from the middle to fit `width` (the card's content width).
fn path_chip<'a, M: 'a>(t: &Skin, s: &'a str, width: f32) -> Element<'a, M> {
    let s: String = if t.narrow {
        let room = width - 2.0 * t.metrics.path_pad_h;
        let max = (room / (t.type_.mono * MONO_ADVANCE)).floor().max(8.0) as usize;
        status::middle_ellipsis(s, max)
    } else {
        s.to_string()
    };
    let bg = t.palette.sunken;
    let r = t.radii.path_chip;
    container(
        text(s)
            .font(Font::MONOSPACE)
            .size(t.type_.mono)
            .wrapping(Wrapping::None)
            .color(color(t.palette.text2)),
    )
    .padding(Padding::from([0.0, t.metrics.path_pad_h]))
    .height(Length::Fixed(t.metrics.path_height))
    .align_y(Alignment::Center)
    .style(move |_| container::Style {
        background: Some(color(bg).into()),
        border: Border::default().rounded(r),
        ..container::Style::default()
    })
    .into()
}

/// A path for display in the given width: shortened from the middle on
/// narrow screens, unchanged otherwise. For paths outside the new cards
/// (the delivery card in the shell).
pub fn display_path(t: &Skin, path: &str, width: f32) -> String {
    if t.narrow {
        let max = (width / (t.type_.mono * MONO_ADVANCE)).floor().max(8.0) as usize;
        status::middle_ellipsis(path, max)
    } else {
        path.to_string()
    }
}

fn mono<'a, M: 'a>(t: &Skin, s: &'a str) -> Element<'a, M> {
    text(s)
        .font(Font::MONOSPACE)
        .size(t.type_.mono)
        .color(color(t.palette.text2))
        .into()
}

/// Command or arg: monospace, wraps instead of truncating.
fn mono_wrap<'a, M: 'a>(t: &Skin, s: &'a str) -> Element<'a, M> {
    text(s)
        .font(Font::MONOSPACE)
        .size(t.type_.mono)
        .wrapping(Wrapping::WordOrGlyph)
        .color(color(t.palette.text))
        .into()
}

fn field_label<'a, M: 'a>(t: &Skin, label: &'static str) -> Element<'a, M> {
    text(label)
        .size(t.type_.foot)
        .color(color(t.palette.text3))
        .into()
}

fn labelled<'a>(
    t: &Skin,
    label: &'static str,
    field: impl Into<Element<'a, Action>>,
) -> Element<'a, Action> {
    column![field_label(t, label), field.into()]
        .spacing(2.0)
        .into()
}

#[derive(Clone, Copy)]
enum Leading {
    None,
    Check,
    Box(bool),
}

/// Wrap a visual in a transparent button whose hit area is at least
/// `touch_target` on both axes, centred on the visual.
fn hit_area<'a>(
    t: &Skin,
    visual: Element<'a, Action>,
    visual_w: Option<f32>,
    visual_h: f32,
    on_press: Option<Action>,
) -> Element<'a, Action> {
    let pad_v = ((t.controls.touch_target - visual_h) / 2.0).max(0.0);
    let pad_h = visual_w.map_or(0.0, |w| ((t.controls.touch_target - w) / 2.0).max(0.0));
    button(visual)
        .padding(Padding::from([pad_v, pad_h]))
        .on_press_maybe(on_press)
        .style(|_, _| button::Style {
            background: None,
            ..button::Style::default()
        })
        .into()
}

/// The one button shape (SPEC §3): 32px visual inside a hit area of at
/// least `touch_target`.
/// `enabled` false fades to the disabled alpha unless the look is
/// `Selected` (an answered pick stays readable).
fn btn<'a>(
    t: &Skin,
    label: String,
    look: Look,
    leading: Leading,
    on_press: Option<Action>,
) -> Element<'a, Action> {
    btn_padded(t, label, look, leading, on_press, t.controls.button_px)
}

/// Padding of a 「去私聊接管」 / Take over in DM button: 6 vertical, 12
/// horizontal, on every platform.
pub const TAKE_OVER_PAD: [f32; 2] = [6.0, 12.0];

/// Gap between the status dot and the status word (or "Step n of m")
/// on a status card.
pub const STATUS_DOT_TEXT_GAP: f32 = 8.0;

fn btn_padded<'a>(
    t: &Skin,
    label: String,
    look: Look,
    leading: Leading,
    on_press: Option<Action>,
    pad_h: f32,
) -> Element<'a, Action> {
    let t = *t;
    let enabled = on_press.is_some();
    let strong = matches!(
        look,
        Look::Primary | Look::Ink | Look::Selected | Look::Danger
    );
    let faded_out = look == Look::Disabled
        || (!enabled && matches!(look, Look::Primary | Look::Ink | Look::Danger));
    let alpha = if faded_out {
        t.palette.disabled_alpha
    } else {
        1.0
    };
    let fg = match look {
        Look::Primary | Look::Danger => t.palette.on_accent,
        Look::Ink => t.chat.primary_text,
        _ => t.palette.text,
    };
    let fg_color = faded(fg, alpha);
    let visual = move |hovered: bool| -> Element<'a, Action> {
        let mut content = row![]
            .spacing(t.metrics.check_gap)
            .align_y(Alignment::Center);
        match leading {
            Leading::None => {}
            Leading::Check => {
                content = content.push(crate::widgets::check_mark(
                    fg_color,
                    t.metrics.check_stroke,
                    t.metrics.check_w,
                    t.metrics.check_h,
                ));
            }
            Leading::Box(checked) => {
                content = content
                    .spacing(t.metrics.checkbox_gap)
                    .push(crate::widgets::checkbox(
                        CheckBox {
                            checked,
                            fill: faded(t.palette.text, alpha),
                            mark: color(t.palette.surface),
                            border: faded(t.palette.border2, alpha),
                            border_width: t.stroke.checkbox,
                            radius: t.radii.checkbox,
                            mark_stroke: t.metrics.check_stroke,
                        },
                        t.controls.checkbox,
                    ));
            }
        }
        content = content.push(
            text(label.clone())
                .size(t.type_.button)
                .font(if strong { semibold() } else { regular() })
                .color(fg_color),
        );
        let c = t.palette;
        let (bg, border) = match look {
            Look::Primary => {
                let fill = if hovered { c.accent_hover } else { c.accent };
                (Some(faded(fill, alpha)), Border::default())
            }
            // Chat cards: ink on light, inverted on dark.
            Look::Ink => (Some(faded(t.chat.primary_bg, alpha)), Border::default()),
            // Proposed red for 「删除规则」; UI姐 to confirm the exact colour.
            Look::Danger => {
                let fill = if hovered {
                    crate::theme::DANGER_HOVER
                } else {
                    crate::theme::DANGER
                };
                (Some(faded(fill, alpha)), Border::default())
            }
            Look::Selected => (
                Some(color(c.control_checked)),
                Border::default()
                    .width(t.stroke.selected)
                    .color(color(c.sel_border)),
            ),
            Look::Secondary | Look::Disabled => (
                hovered.then(|| color(c.control_hover)),
                Border::default()
                    .width(t.stroke.control)
                    .color(faded(c.border2, alpha)),
            ),
        };
        container(content)
            .height(Length::Fixed(t.controls.button_h))
            .padding(Padding::from([0.0, pad_h]))
            .align_y(Alignment::Center)
            .style(move |_| container::Style {
                background: bg.map(Background::Color),
                border: border.rounded(t.radii.button),
                shadow: shadow(&t),
                ..container::Style::default()
            })
            .into()
    };
    let shown: Element<'a, Action> = if enabled {
        hover(visual(false), visual(true))
    } else {
        visual(false)
    };
    hit_area(&t, shown, None, t.controls.button_h, on_press)
}

fn buttons_row<'a>(t: &Skin, items: Vec<Element<'a, Action>>) -> Element<'a, Action> {
    row(items)
        .spacing(t.controls.button_gap)
        .wrap()
        // Keep the visual row gap at `button_gap`; hit-area padding already
        // counts toward it.
        .vertical_spacing(
            (t.controls.button_gap - (t.controls.touch_target - t.controls.button_h).max(0.0))
                .max(0.0),
        )
        .into()
}

fn frame<'a>(t: &Skin, content: Element<'a, Action>) -> Element<'a, Action> {
    container(content)
        .padding(card_padding(t))
        .width(Fill)
        .style(card_style(*t))
        .into()
}

/// 11px note line in a chat card (under the approval header).
const CHAT_NOTE_SIZE: f32 = 11.0;

/// An open approval card: the chat accent as its outline.
fn approval_frame<'a>(
    t: &Skin,
    card: &ApprovalCard,
    content: Element<'a, Action>,
) -> Element<'a, Action> {
    if card.state != crate::cards::ApprovalState::Pending {
        return frame(t, content);
    }
    let t = *t;
    container(content)
        .padding(card_padding(&t))
        .width(Fill)
        .style(move |th| container::Style {
            border: Border::default()
                .width(t.stroke.card)
                .color(color(t.chat.accent))
                .rounded(t.radii.card),
            ..card_style(t)(th)
        })
        .into()
}

// ---------- Cards ----------

/// What the connected backend can do, so the view never draws a button
/// whose endpoint does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Caps {
    /// Draft 发送 / 修改. Drafts are phase 2: only the fake source has them.
    pub draft_actions: bool,
}

impl Caps {
    /// Real host (phase 1): no draft buttons.
    pub const HOST: Caps = Caps {
        draft_actions: false,
    };
    /// `--fake` / `--gallery`: every card fully clickable.
    pub const FAKE: Caps = Caps {
        draft_actions: true,
    };
}

/// One new card. `width` is the stream's content width (for clamping the
/// draft preview); `ui` is that card's local state.
pub fn card<'a>(
    t: &Skin,
    card: &'a Card,
    ui: &CardUi,
    caps: Caps,
    width: f32,
    in_group: bool,
    hide_bot: bool,
    me: Option<&str>,
) -> Element<'a, Action> {
    match card {
        Card::Choice(c) => choice(t, c, ui, hide_bot),
        Card::Approval(c) => approval(t, c, ui, width, in_group, hide_bot, me),
        Card::Draft(c) => draft(t, c, ui, caps, width, hide_bot),
        Card::Confirm(c) => confirm_card(t, c, ui, width, hide_bot),
    }
}

/// A bro's proposal: tag 「记忆」/「技能」, the full text with who can use
/// it, then 记下 (保存) / 不用 until answered; the answer goes to the footnote.
pub fn confirm_card<'a>(
    t: &Skin,
    card: &'a ConfirmCard,
    ui: &CardUi,
    width: f32,
    hide_bot: bool,
) -> Element<'a, Action> {
    let (label, d) = card.status();
    let mut col = column![
        header(t, &card.id, &card.bot, ui, label, d, hide_bot),
        gap(t.metrics.head_gap)
    ];
    let tag_el = tag(t, card.tag(), t.palette.text2);
    match &card.title {
        // Skill: title + slug, instruction cut to three lines until opened.
        Some(title) => {
            let mut head = row![
                tag_el,
                text(title.as_str())
                    .size(t.type_.body)
                    .font(bold())
                    .color(color(t.palette.text)),
            ]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center);
            if let Some(slug) = &card.slug {
                head = head.push(mono(t, slug));
            }
            col = col.push(head);
            if let Some(instruction) = &card.instruction {
                let inner = width - 2.0 * (t.controls.card_pad_h + t.stroke.card);
                let cols = (inner / (t.type_.body * t.type_.unit_width)).max(8.0) as usize;
                let cut = status::clamp_lines(instruction, cols, confirm::INSTRUCTION_LINES);
                let long = cut.ends_with('…');
                let shown = if ui.more_open || !long {
                    instruction.clone()
                } else {
                    cut
                };
                col = col.push(gap(t.metrics.head_gap)).push(
                    text(shown)
                        .size(t.type_.body)
                        .line_height(LineHeight::Absolute(Pixels(t.type_.body_line)))
                        .color(color(t.palette.text))
                        .width(Fill),
                );
                if long {
                    col = col.push(btn(
                        t,
                        if ui.more_open {
                            confirm::SHOW_LESS
                        } else {
                            confirm::SHOW_ALL
                        }
                        .into(),
                        Look::Secondary,
                        Leading::None,
                        Some(Action::ToggleInstruction {
                            card: card.id.clone(),
                        }),
                    ));
                }
            }
        }
        // Memory: the whole proposed text, then who can use it.
        None => {
            col = col.push(
                row![
                    tag_el,
                    container(body(t, card.text.as_deref().unwrap_or_default())).width(Fill),
                ]
                .spacing(t.controls.button_gap)
                .align_y(Alignment::Center),
            );
        }
    }
    if let Some(scope) = &card.scope_line {
        col = col.push(gap(t.metrics.head_gap)).push(
            text(scope.as_str())
                .size(t.type_.foot)
                .color(color(t.palette.text2)),
        );
    }
    if let Some(enabled) = confirm::confirm_actions(card, ui) {
        let id = &card.id;
        col = col.push(gap(t.metrics.body_gap)).push(buttons_row(
            t,
            vec![
                btn(
                    t,
                    card.keep_label().into(),
                    Look::Primary,
                    Leading::None,
                    enabled.then(|| Action::Decide {
                        card: id.clone(),
                        approve: true,
                    }),
                ),
                btn(
                    t,
                    confirm::DROP.into(),
                    if enabled {
                        Look::Secondary
                    } else {
                        Look::Disabled
                    },
                    Leading::None,
                    enabled.then(|| Action::Decide {
                        card: id.clone(),
                        approve: false,
                    }),
                ),
            ],
        ));
    }
    let foot = ui
        .note
        .as_ref()
        .map(|n| n.text())
        .or_else(|| card.footnote_in(&Local));
    if let Some(foot) = foot {
        col = col.push(gap(t.metrics.foot_gap)).push(footnote(t, foot));
    }
    frame(t, col.into())
}

/// 「还有 N 条待确认」 in place of a task's folded confirm cards.
pub fn confirm_more<'a>(t: &Skin, task: &str, n: usize) -> Element<'a, Action> {
    btn(
        t,
        confirm::more_label(n),
        Look::Secondary,
        Leading::None,
        Some(Action::UnfoldConfirms {
            task: task.to_string(),
        }),
    )
}

pub fn choice<'a>(
    t: &Skin,
    card: &'a ChoiceCard,
    ui: &CardUi,
    hide_bot: bool,
) -> Element<'a, Action> {
    let (label, d) = status::choice_status(&card.state);
    let buttons = interact::choice_buttons(card, ui);
    let confirm = interact::confirm_button(card, ui);
    debug_assert!(interact::primary_count(&buttons, confirm.as_ref()) <= 1);

    let items = buttons
        .into_iter()
        .map(|b: OptionButton| {
            let leading = match (b.checkbox, b.look) {
                (Some(checked), _) => Leading::Box(checked),
                (None, Look::Selected) => Leading::Check,
                _ => Leading::None,
            };
            let action = b.clickable.then(|| {
                if card.multi {
                    Action::TogglePick {
                        card: card.id.clone(),
                        option: b.id.clone(),
                    }
                } else {
                    Action::Choose {
                        card: card.id.clone(),
                        option: b.id.clone(),
                    }
                }
            });
            btn(t, b.label, b.look, leading, action)
        })
        .collect();

    let mut col = column![
        header(t, &card.id, &card.bot, ui, label, d, hide_bot),
        gap(t.metrics.head_gap),
        body(t, &card.prompt),
        gap(t.metrics.body_gap),
        buttons_row(t, items),
    ];
    if let Some((confirm_label, enabled)) = confirm {
        col = col.push(gap(t.metrics.body_gap)).push(
            row![
                text(status::MULTI_HINT)
                    .size(t.type_.foot)
                    .color(color(t.palette.text3)),
                space::horizontal(),
                btn(
                    t,
                    confirm_label,
                    Look::Primary,
                    Leading::None,
                    enabled.then(|| Action::ConfirmChoice {
                        card: card.id.clone()
                    }),
                ),
            ]
            .align_y(Alignment::Center),
        );
    }
    let foot = ui
        .note
        .as_ref()
        .map(|n| n.text())
        .or_else(|| status::choice_footnote(card, &Local));
    if let Some(foot) = foot {
        col = col.push(gap(t.metrics.foot_gap)).push(footnote(t, foot));
    }
    frame(t, col.into())
}

pub fn approval<'a>(
    t: &Skin,
    card: &'a ApprovalCard,
    ui: &CardUi,
    width: f32,
    in_group: bool,
    hide_bot: bool,
    me: Option<&str>,
) -> Element<'a, Action> {
    let inner = width - 2.0 * (t.controls.card_pad_h + t.stroke.card);
    // v0.2 host: a group card belongs to the bro's owner, who alone decides.
    if group_approval::owner_layout(in_group, card.is_rule(), group_approval::viewer(card, me)) {
        return group_approval_card(t, card, ui, inner, hide_bot, me);
    }
    let (label, d) = status::approval_status(&card.state);
    let mut col = column![header(t, &card.id, &card.bot, ui, label, d, hide_bot)];
    // Group thread: same header and action line as a DM card, with
    // 「已发到主人私聊」 on its own 11px line under the header, no buttons.
    let group_pending = in_group && card.state == crate::cards::ApprovalState::Pending;
    if group_pending && !card.is_rule() {
        col = col.push(
            text(review_rules::SENT_TO_OWNER)
                .size(CHAT_NOTE_SIZE)
                .color(color(t.palette.text2)),
        );
    }
    col = col.push(gap(t.metrics.head_gap));
    if card.is_rule() && !group_pending {
        return rule_approval(t, card, ui, col);
    }
    let actions = interact::approval_actions(card, ui);
    if actions == ApprovalActions::TakeOver {
        // Stuck: one line and one secondary button. No approve/deny, no
        // code input, no terminal.
        col = col
            .push(body(t, status::NEEDS_OWNER_LINE.as_str()))
            .push(gap(t.metrics.body_gap))
            .push(btn_padded(
                t,
                status::TAKE_OVER.into(),
                Look::Secondary,
                Leading::None,
                Some(Action::TakeOver {
                    card: card.id.clone(),
                }),
                TAKE_OVER_PAD[1],
            ));
    } else {
        col = approval_what(t, card, inner, col);
        if group_pending {
            return approval_frame(t, card, col.into());
        }
        if let ApprovalActions::Decide { enabled } = actions {
            let id = &card.id;
            col = col.push(gap(t.metrics.body_gap)).push(buttons_row(
                t,
                vec![
                    btn(
                        t,
                        Key::ViewApprove.into(),
                        Look::Ink,
                        Leading::None,
                        enabled.then(|| Action::Decide {
                            card: id.clone(),
                            approve: true,
                        }),
                    ),
                    btn(
                        t,
                        Key::ViewDeny.into(),
                        if enabled {
                            Look::Secondary
                        } else {
                            Look::Disabled
                        },
                        Leading::None,
                        enabled.then(|| Action::Decide {
                            card: id.clone(),
                            approve: false,
                        }),
                    ),
                ],
            ));
        }
    }
    if let Some(note) = ui.note.as_ref() {
        col = col
            .push(gap(t.metrics.foot_gap))
            .push(footnote(t, note.text()));
    } else if let Some(foot) = status::approval_footnote(&card.state, &Local) {
        col = col
            .push(gap(t.metrics.foot_gap))
            .push(approval_footnote_view(t, foot));
    }
    approval_frame(t, card, col.into())
}

/// Id of a card's frame, so the client can find it to scroll to (jump from
/// the DM reminder row).
pub fn card_anchor(card: &str) -> iced::widget::Id {
    iced::widget::Id::from(format!("card:{card}"))
}

/// 13px accent line under a card (a refused click).
const ACCENT_NOTE_SIZE: f32 = 13.0;

/// Card frame: `surface`, card stroke; the stroke turns accent while
/// `flash` is set (a jump from the DM reminder row).
fn flash_frame<'a>(
    t: &Skin,
    id: &str,
    content: Element<'a, Action>,
    flash: bool,
) -> Element<'a, Action> {
    let t = *t;
    container(content)
        .id(card_anchor(id))
        .padding(card_padding(&t))
        .width(Fill)
        .style(move |th| container::Style {
            border: Border::default()
                .width(t.stroke.card)
                .color(color(if flash {
                    t.chat.accent
                } else {
                    t.palette.border
                }))
                .rounded(t.radii.card),
            ..card_style(t)(th)
        })
        .into()
}

/// What the bro wants to do: the kind tag and summary, the path and why it
/// was blocked. Shared by the DM card and the group card.
fn approval_what<'a>(
    t: &Skin,
    card: &'a ApprovalCard,
    inner: f32,
    mut col: iced::widget::Column<'a, Action>,
) -> iced::widget::Column<'a, Action> {
    // A call card's host summary already starts with 「调用」, so the
    // action tag would repeat it. Other kinds keep the tag.
    let summary = container(approval_summary(t, card, inner)).width(Fill);
    let line: Element<'a, Action> = if card.kind == crate::cards::ActionKind::Call {
        summary.into()
    } else {
        row![
            tag(t, status::action_kind_label(card.kind), t.palette.text2),
            summary,
        ]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center)
        .into()
    };
    col = col.push(line);
    if let Some(path) = &card.path {
        col = col
            .push(gap(t.metrics.head_gap))
            .push(path_chip(t, path, inner));
    }
    if let Some(shown) = crate::cards::codes::reason_text(card.reason_code, card.reason.as_deref())
    {
        col = col.push(gap(t.metrics.head_gap)).push(
            text(review_rules::blocked_reason_line(&shown))
                .size(t.type_.foot)
                .color(color(t.palette.text2)),
        );
    }
    col
}

/// An approval card in a group (v0.2): dot and status, what the bro wants,
/// one 11px line, and Approve / Deny for the owner only. Everyone else gets
/// no buttons at all, not even greyed ones.
fn group_approval_card<'a>(
    t: &Skin,
    card: &'a ApprovalCard,
    ui: &CardUi,
    inner: f32,
    hide_bot: bool,
    me: Option<&str>,
) -> Element<'a, Action> {
    let l = group_approval::lines(card, ui, me, &Local);
    let mut col = column![
        header(t, &card.id, &card.bot, ui, l.status, l.dot, hide_bot),
        gap(t.metrics.head_gap),
    ];
    col = approval_what(t, card, inner, col);
    if let Some(line) = l.line {
        col = col.push(gap(t.metrics.foot_gap)).push(footnote(t, line));
    }
    if let Some(error) = l.error {
        col = col.push(gap(t.metrics.foot_gap)).push(
            text(error)
                .size(ACCENT_NOTE_SIZE)
                .color(color(t.chat.accent)),
        );
    } else if let Some(note) = ui.note.as_ref().map(Note::text).filter(|n| !n.is_empty()) {
        col = col.push(gap(t.metrics.foot_gap)).push(footnote(t, note));
    }
    if let ApprovalActions::Decide { enabled } = l.actions {
        let id = &card.id;
        let pad = TAKE_OVER_PAD[1];
        let buttons = row![
            btn_padded(
                t,
                Key::ViewApprove.into(),
                Look::Primary,
                Leading::None,
                enabled.then(|| Action::Decide {
                    card: id.clone(),
                    approve: true,
                }),
                pad,
            ),
            btn_padded(
                t,
                Key::ViewDeny.into(),
                if enabled {
                    Look::Secondary
                } else {
                    Look::Disabled
                },
                Leading::None,
                enabled.then(|| Action::Decide {
                    card: id.clone(),
                    approve: false,
                }),
                pad,
            ),
        ]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center);
        col = col
            .push(gap(t.metrics.body_gap))
            .push(row![space::horizontal(), buttons]);
    }
    flash_frame(t, &card.id, col.into(), ui.flash)
}

/// Font size of the reminder row's text.
const REMINDER_TEXT: f32 = 13.0;

/// The owner's DM reminder: dot, one line of text, the time on the right.
/// Not a card and no buttons; the whole row is the click target.
pub fn reminder_row<'a>(t: &Skin, r: &'a Reminder) -> Element<'a, Action> {
    let t = *t;
    let mut line = row![
        dot(&t, r.dot()),
        text(r.text(&Local))
            .size(REMINDER_TEXT)
            .color(color(t.palette.text))
            .wrapping(Wrapping::None),
        space::horizontal(),
    ]
    .spacing(t.metrics.dot_gap)
    .align_y(Alignment::Center);
    if let Some(time) = r.time(&Local) {
        line = line.push(text(time).size(t.type_.foot).color(color(t.palette.text3)));
    }
    button(container(line).width(Fill).clip(true))
        .width(Fill)
        .padding(Padding::from([
            t.metrics.status_pad_v,
            t.metrics.status_pad_h,
        ]))
        .on_press(Action::OpenApproval {
            group: r.group_id.clone(),
            card: r.approval_id.clone(),
        })
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: hovered.then(|| color(t.palette.control_hover).into()),
                text_color: color(t.palette.text),
                border: Border::default().rounded(t.radii.button),
                ..button::Style::default()
            }
        })
        .into()
}

pub fn draft<'a>(
    t: &Skin,
    card: &'a DraftCard,
    ui: &CardUi,
    caps: Caps,
    width: f32,
    hide_bot: bool,
) -> Element<'a, Action> {
    let (label, d) = status::draft_status(&card.state);
    let lines = status::draft_preview_lines(&card.state);
    let inner = width - 2.0 * (t.controls.card_pad_h + t.metrics.preview_pad + t.stroke.card);
    let cols = (inner / (t.type_.preview * t.type_.unit_width)).max(8.0) as usize;
    let preview_text = status::clamp_lines(&card.body, cols, lines);

    let mut preview = column![].spacing(4);
    if let Some(subject) = &card.subject {
        preview = preview.push(
            text(subject.as_str())
                .size(t.type_.draft_title)
                .font(bold())
                .color(color(t.palette.text)),
        );
    }
    let mut body_text = text(preview_text)
        .size(t.type_.preview)
        .line_height(LineHeight::Absolute(Pixels(t.type_.preview_line)))
        .color(color(t.palette.text2));
    if lines == 1 {
        body_text = body_text.wrapping(Wrapping::None);
    }
    preview = preview.push(body_text);
    let (bg, r) = (t.palette.sunken, t.radii.preview);
    let preview_box = container(preview)
        .padding(t.metrics.preview_pad)
        .width(Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(color(bg).into()),
            border: Border::default().rounded(r),
            ..container::Style::default()
        });

    let mut col = column![
        header(t, &card.id, &card.bot, ui, label, d, hide_bot),
        gap(t.metrics.head_gap),
        row![
            tag(t, status::channel_label(card.channel), t.palette.text2),
            text(Key::ViewDraftTo)
                .size(t.type_.foot)
                .color(color(t.palette.text2)),
            mono(t, &card.to),
        ]
        .spacing(t.metrics.dot_gap)
        .align_y(Alignment::Center)
        .wrap(),
        gap(t.metrics.head_gap),
        preview_box,
    ];
    if let Some(enabled) = interact::draft_actions(card, ui, caps.draft_actions) {
        let id = &card.id;
        col = col.push(gap(t.metrics.body_gap)).push(buttons_row(
            t,
            vec![
                btn(
                    t,
                    Key::ViewDraftSend.into(),
                    Look::Primary,
                    Leading::None,
                    enabled.then(|| Action::SendDraft { card: id.clone() }),
                ),
                btn(
                    t,
                    Key::ViewDraftEdit.into(),
                    Look::Secondary,
                    Leading::None,
                    Some(Action::EditDraftInDm { card: id.clone() }),
                ),
            ],
        ));
    }
    let foot = ui
        .note
        .as_ref()
        .map(|n| n.text())
        .or_else(|| status::draft_footnote(&card.state, &Local));
    if let Some(foot) = foot {
        col = col.push(gap(t.metrics.foot_gap)).push(footnote(t, foot));
    }
    frame(t, col.into())
}

// ---------- Routines ----------

/// Full-width hairline (chat header / composer edges).
pub fn rule<'a, M: 'a>(t: &Skin) -> Element<'a, M> {
    divider(t, 0.0)
}

fn divider<'a, M: 'a>(t: &Skin, inset: f32) -> Element<'a, M> {
    let c = t.palette.divider;
    container(
        container(Space::new().width(Fill).height(t.stroke.divider)).style(move |_| {
            container::Style {
                background: Some(color(c).into()),
                ..container::Style::default()
            }
        }),
    )
    .padding(Padding::from([0.0, inset]))
    .width(Fill)
    .into()
}

/// The routines panel (SPEC §7). `now` is Unix seconds; times show in the
/// computer's local zone.
pub fn routines_panel<'a>(
    t: &Skin,
    routines: &'a Routines,
    now: i64,
    titled: bool,
) -> Element<'a, Action> {
    let m = t.metrics;
    let mut col = column![];
    if titled {
        let mut head = row![text(routines::TITLE)
            .size(t.type_.list_title)
            .font(bold())
            .color(color(t.palette.text))]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center);
        if !routines.rows.is_empty() {
            head = head.push(
                text(routines.rows.len().to_string())
                    .size(t.type_.list_count)
                    .color(color(t.palette.text3)),
            );
        }
        col = column![
            container(head)
                .height(Length::Fixed(m.panel_head_height))
                .padding(Padding::from([0.0, m.row_pad_h]))
                .align_y(Alignment::Center),
            divider(t, 0.0),
        ];
    }

    if routines.rows.is_empty() {
        let rings = Rings {
            color: color(t.palette.text3),
            radius: m.empty_ring_r,
            stroke: m.empty_ring_stroke,
        };
        let size = rings.size();
        col = col.push(
            container(
                column![
                    canvas(rings).width(size.width).height(size.height),
                    gap(m.head_gap),
                    text(routines::EMPTY_TITLE)
                        .size(t.type_.empty_title)
                        .font(bold())
                        .color(color(t.palette.text2)),
                    gap(m.dot_gap),
                    text(routines::EMPTY_HINT)
                        .size(t.type_.empty_hint)
                        .color(color(t.palette.text3)),
                ]
                .align_x(Alignment::Center),
            )
            .width(Fill)
            .height(Length::Fixed(m.empty_height))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        );
    } else {
        for (i, r) in routines.rows.iter().enumerate() {
            if i > 0 {
                col = col.push(divider(t, m.row_pad_h));
            }
            let name_color = if r.paused {
                t.palette.text3
            } else {
                t.palette.text
            };
            let mut right = row![].spacing(m.row_time_gap).align_y(Alignment::Center);
            if routines.failed.contains(&r.id) {
                right = right.push(
                    text(status::PAUSE_FAILED_NOTE)
                        .size(t.type_.foot)
                        .color(color(t.palette.text3)),
                );
            }
            right = right.push(match r.next_run_text(now) {
                Some(s) => {
                    Element::from(text(s).size(t.type_.row_time).color(color(t.palette.text2)))
                }
                None => tag(t, routines::PAUSED_TAG.as_str(), t.palette.text3),
            });
            let toggle = hit_area(
                t,
                canvas(Toggle {
                    on: !r.paused,
                    track_on: color(t.palette.accent),
                    track_off: color(t.palette.toggle_off),
                    knob: color(t.palette.knob),
                    knob_size: t.controls.knob,
                })
                .width(t.controls.switch_w)
                .height(t.controls.switch_h)
                .into(),
                Some(t.controls.switch_w),
                t.controls.switch_h,
                Some(Action::TogglePause { id: r.id.clone() }),
            );
            right = right.push(toggle);
            col = col.push(
                row![
                    text(r.name.as_str())
                        .size(t.type_.row_name)
                        .color(color(name_color))
                        .wrapping(Wrapping::None)
                        .width(Fill),
                    right,
                ]
                .spacing(m.row_time_gap)
                .height(Length::Fixed(t.controls.row_h))
                .padding(Padding::from([0.0, m.row_pad_h]))
                .align_y(Alignment::Center),
            );
        }
    }

    let tt = *t;
    container(col)
        .width(Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(color(tt.palette.surface).into()),
            text_color: Some(color(tt.palette.text)),
            border: Border::default()
                .width(tt.stroke.card)
                .color(color(tt.palette.border))
                .rounded(tt.radii.panel),
            shadow: shadow(&tt),
            ..container::Style::default()
        })
        .into()
}

// ---------- 记忆 page ----------

/// A failed save, shown where it happened: accent text, never silent.
fn inline_note<'a, M: 'a>(t: &Skin, note: &str) -> Element<'a, M> {
    text(note.to_string())
        .size(t.type_.foot)
        .color(color(t.palette.accent))
        .into()
}

/// The 记忆 page: one panel like the routines list. Groups by who may use a
/// memory (「所有 bro」 first, then each bro); pending apart from kept.
pub fn memories_panel<'a>(t: &Skin, book: &'a Book, now: i64, titled: bool) -> Element<'a, Action> {
    use robroz_contract::MemoryKind;
    let m = t.metrics;
    let busy = book.busy;
    let mut head = if titled {
        row![text(memories::TITLE)
            .size(t.type_.list_title)
            .font(bold())
            .color(color(t.palette.text))]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center)
    } else {
        row![]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center)
    };
    if titled && !book.memories.is_empty() {
        head = head.push(
            text(book.memories.len().to_string())
                .size(t.type_.list_count)
                .color(color(t.palette.text3)),
        );
    }
    head = head.push(space::horizontal());
    if book.adding.is_none() {
        head = head.push(btn(
            t,
            memories::ADD.into(),
            Look::Secondary,
            Leading::None,
            (!busy).then_some(Action::MemoryAddOpen),
        ));
    }
    let mut col = if titled || book.adding.is_none() {
        column![
            container(head)
                .height(Length::Fixed(m.panel_head_height))
                .padding(Padding::from([0.0, m.row_pad_h]))
                .align_y(Alignment::Center),
            divider(t, 0.0),
        ]
    } else {
        column![]
    };
    let pad = Padding::from([m.row_pad_h * 0.75, m.row_pad_h]);

    if let Some(note) = &book.note {
        col = col.push(
            container(
                text(note.as_str())
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
            )
            .padding(Padding::from([8.0, m.row_pad_h])),
        );
    }

    if let Some(new) = &book.adding {
        let mut input = text_input(memories::NEW_PLACEHOLDER.as_str(), &new.text)
            .size(t.type_.body)
            .padding(Padding::from([8.0, 10.0]));
        if !busy {
            input = input
                .on_input(|text| Action::MemoryAddText { text })
                .on_submit(Action::MemoryAddSave);
        }
        let mut scopes = row![].spacing(t.controls.button_gap);
        for (scope, label) in book.scope_choices() {
            let look = if new.scope.as_deref() == Some(scope.as_str()) {
                Look::Selected
            } else {
                Look::Secondary
            };
            scopes = scopes.push(btn(
                t,
                label,
                look,
                Leading::None,
                Some(Action::MemoryAddScope { scope }),
            ));
        }
        let mut kinds = row![].spacing(t.controls.button_gap);
        for kind in MemoryKind::ALL {
            let look = if kind == new.kind {
                Look::Selected
            } else {
                Look::Secondary
            };
            kinds = kinds.push(btn(
                t,
                memories::kind_label(kind).into(),
                look,
                Leading::None,
                Some(Action::MemoryAddKind { kind }),
            ));
        }
        let can_save = book.can_add();
        let mut form = column![input, scopes.wrap()].spacing(m.head_gap);
        // One grey row, always there: 「选一下谁能用这条」 until a scope is
        // picked, then 「只给 Bob 用」. Keeps 保存 from moving.
        if let Some(hint) = book.add_scope_hint() {
            form = form.push(text(hint).size(t.type_.foot).color(color(t.palette.text3)));
        }
        if let Some(note) = &book.add_note {
            form = form.push(inline_note(t, note));
        }
        col = col.push(
            container(
                column![
                    form,
                    kinds,
                    row![
                        btn(
                            t,
                            memories::SAVE.into(),
                            Look::Primary,
                            Leading::None,
                            can_save.then_some(Action::MemoryAddSave)
                        ),
                        btn(
                            t,
                            memories::CANCEL.into(),
                            Look::Secondary,
                            Leading::None,
                            (!busy).then_some(Action::MemoryAddClose)
                        ),
                    ]
                    .spacing(t.controls.button_gap),
                ]
                .spacing(m.head_gap),
            )
            .padding(pad),
        );
        col = col.push(divider(t, 0.0));
    }

    if book.loaded && book.is_empty() {
        let rings = Rings {
            color: color(t.palette.text3),
            radius: m.empty_ring_r,
            stroke: m.empty_ring_stroke,
        };
        let size = rings.size();
        col = col.push(
            container(
                column![
                    canvas(rings).width(size.width).height(size.height),
                    gap(m.head_gap),
                    text(memories::EMPTY_TITLE)
                        .size(t.type_.empty_title)
                        .font(bold())
                        .color(color(t.palette.text2)),
                    gap(m.dot_gap),
                    text(memories::EMPTY_HINT)
                        .size(t.type_.empty_hint)
                        .color(color(t.palette.text3)),
                ]
                .align_x(Alignment::Center),
            )
            .width(Fill)
            .height(Length::Fixed(m.empty_height))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        );
    }

    let entry = |mem: &'a robroz_contract::Memory, pending: bool| -> Element<'a, Action> {
        let mut top = row![tag(t, memories::kind_label(mem.kind), t.palette.text2)]
            .spacing(m.row_time_gap)
            .align_y(Alignment::Center);
        if pending {
            top = top.push(tag(t, memories::PENDING_TAG.as_str(), t.palette.accent));
        }
        top = top.push(space::horizontal());
        if book.just_saved.as_deref() == Some(mem.id.as_str()) {
            top = top.push(
                text(memories::SAVED)
                    .size(t.type_.row_time)
                    .color(color(t.palette.text2)),
            );
        }
        top = top.push(
            text(memories::updated_text(&mem.updated_at, now))
                .size(t.type_.row_time)
                .color(color(t.palette.text3)),
        );
        let editing = book.editing.as_ref().filter(|(id, _)| *id == mem.id);
        let asking = book.confirm_forget.as_deref() == Some(mem.id.as_str());
        let body: Element<'a, Action> = match editing {
            Some((_, draft)) => {
                let mut input = text_input("", draft)
                    .size(t.type_.body)
                    .padding(Padding::from([6.0, 8.0]));
                if !busy {
                    input = input
                        .on_input(|text| Action::MemoryEditText { text })
                        .on_submit(Action::MemoryEditSave);
                }
                input.into()
            }
            None => text(mem.text.as_str())
                .size(t.type_.body)
                .line_height(LineHeight::Absolute(Pixels(t.type_.body_line)))
                .color(color(t.palette.text))
                .width(Fill)
                .into(),
        };
        let actions: Element<'a, Action> = if pending {
            // 记下 / 不用 answer the entry's confirm card (robroz#66); an entry
            // without one (older host) stays display only. No 改/忘掉 here.
            match &mem.confirm_id {
                Some(_) => row![
                    btn(
                        t,
                        confirm::KEEP.into(),
                        Look::Primary,
                        Leading::None,
                        (!busy).then(|| Action::MemoryDecide {
                            id: mem.id.clone(),
                            keep: true,
                        })
                    ),
                    btn(
                        t,
                        confirm::DROP.into(),
                        if busy {
                            Look::Disabled
                        } else {
                            Look::Secondary
                        },
                        Leading::None,
                        (!busy).then(|| Action::MemoryDecide {
                            id: mem.id.clone(),
                            keep: false,
                        })
                    ),
                ]
                .spacing(t.controls.button_gap)
                .into(),
                None => Space::new().height(Length::Fixed(0.0)).into(),
            }
        } else if editing.is_some() {
            row![
                btn(
                    t,
                    memories::SAVE.into(),
                    Look::Primary,
                    Leading::None,
                    book.can_save_edit().then_some(Action::MemoryEditSave)
                ),
                btn(
                    t,
                    memories::CANCEL.into(),
                    Look::Secondary,
                    Leading::None,
                    (!busy).then_some(Action::MemoryEditCancel)
                ),
            ]
            .spacing(t.controls.button_gap)
            .into()
        } else if asking {
            row![
                text(memories::FORGET_ASK)
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
                space::horizontal(),
                btn(
                    t,
                    memories::FORGET.into(),
                    Look::Primary,
                    Leading::None,
                    (!busy).then_some(Action::MemoryForgetConfirm)
                ),
                btn(
                    t,
                    memories::CANCEL.into(),
                    Look::Secondary,
                    Leading::None,
                    (!busy).then_some(Action::MemoryForgetCancel)
                ),
            ]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center)
            .into()
        } else {
            row![
                btn(
                    t,
                    memories::EDIT.into(),
                    Look::Secondary,
                    Leading::None,
                    (!busy).then_some(Action::MemoryEdit { id: mem.id.clone() })
                ),
                btn(
                    t,
                    memories::FORGET.into(),
                    Look::Secondary,
                    Leading::None,
                    (!busy).then_some(Action::MemoryForgetAsk { id: mem.id.clone() })
                ),
            ]
            .spacing(t.controls.button_gap)
            .into()
        };
        let mut entry_col = column![top, body, actions].spacing(m.dot_gap + 2.0);
        if let Some(note) = book.note_for(&mem.id) {
            entry_col = entry_col.push(inline_note(t, note));
        }
        container(entry_col).padding(pad).width(Fill).into()
    };

    for (gi, group) in book.groups().into_iter().enumerate() {
        if gi > 0 {
            col = col.push(divider(t, 0.0));
        }
        col = col.push(
            container(
                text(group.title.clone())
                    .size(t.type_.row_name)
                    .font(bold())
                    .color(color(t.palette.text)),
            )
            .padding(Padding::from([m.row_pad_h * 0.75, m.row_pad_h])),
        );
        let sub = |label: &'static str, n: usize| -> Element<'a, Action> {
            container(
                text(format!("{label} {n}"))
                    .size(t.type_.foot)
                    .color(color(t.palette.text3)),
            )
            .padding(Padding::from([0.0, m.row_pad_h]))
            .into()
        };
        // 「待确认 N」 then 「已记下 N」; the kept head shows even when the
        // group has nothing pending, so every scope group reads the same.
        for (label, n) in group.heads() {
            if label == memories::KEPT_HEAD {
                if !group.pending.is_empty() {
                    col = col.push(divider(t, m.row_pad_h));
                    col = col.push(gap(m.dot_gap));
                }
                col = col.push(sub(label, n));
                break;
            }
            col = col.push(sub(label, n));
            for (i, mem) in group.pending.iter().enumerate() {
                if i > 0 {
                    col = col.push(divider(t, m.row_pad_h));
                }
                col = col.push(entry(mem, true));
            }
        }
        for (i, mem) in group.kept.iter().enumerate() {
            if i > 0 {
                col = col.push(divider(t, m.row_pad_h));
            }
            col = col.push(entry(mem, false));
        }
    }

    let tt = *t;
    container(col)
        .width(Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(color(tt.palette.surface).into()),
            text_color: Some(color(tt.palette.text)),
            border: Border::default()
                .width(tt.stroke.card)
                .color(color(tt.palette.border))
                .rounded(tt.radii.panel),
            shadow: shadow(&tt),
            ..container::Style::default()
        })
        .into()
}

// ---------- 技能 page ----------

/// The 技能 page: same grouping as the 记忆 page, plus a 3-line
/// instruction, edit of title and instruction, a confirm before 删除,
/// and a per-group enable toggle on kept entries.
pub fn skills_panel<'a>(
    t: &Skin,
    book: &'a SkillBook,
    now: i64,
    width: f32,
    titled: bool,
) -> Element<'a, Action> {
    let m = t.metrics;
    let busy = book.busy;
    let mut head = if titled {
        row![text(skills::TITLE)
            .size(t.type_.list_title)
            .font(bold())
            .color(color(t.palette.text))]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center)
    } else {
        row![]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center)
    };
    if titled && !book.skills.is_empty() {
        head = head.push(
            text(book.skills.len().to_string())
                .size(t.type_.list_count)
                .color(color(t.palette.text3)),
        );
    }
    head = head.push(space::horizontal());
    if book.adding.is_none() {
        head = head.push(btn(
            t,
            skills::ADD.into(),
            Look::Secondary,
            Leading::None,
            (!busy).then_some(Action::SkillAddOpen),
        ));
    }
    let mut col = if titled || book.adding.is_none() {
        column![
            container(head)
                .height(Length::Fixed(m.panel_head_height))
                .padding(Padding::from([0.0, m.row_pad_h]))
                .align_y(Alignment::Center),
            divider(t, 0.0),
        ]
    } else {
        column![]
    };
    let pad = Padding::from([m.row_pad_h * 0.75, m.row_pad_h]);

    if let Some(note) = &book.note {
        col = col.push(
            container(
                text(note.as_str())
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
            )
            .padding(Padding::from([8.0, m.row_pad_h])),
        );
    }

    if let Some(new) = &book.adding {
        let mut slug = text_input(skills::SLUG_PLACEHOLDER.as_str(), &new.slug)
            .size(t.type_.body)
            .padding(Padding::from([8.0, 10.0]));
        let mut title = text_input(skills::TITLE_PLACEHOLDER.as_str(), &new.title)
            .size(t.type_.body)
            .padding(Padding::from([8.0, 10.0]));
        let mut instruction =
            text_input(skills::INSTRUCTION_PLACEHOLDER.as_str(), &new.instruction)
                .size(t.type_.body)
                .padding(Padding::from([8.0, 10.0]));
        if !busy {
            slug = slug.on_input(|text| Action::SkillAddSlug { text });
            title = title.on_input(|text| Action::SkillAddTitle { text });
            instruction = instruction
                .on_input(|text| Action::SkillAddInstruction { text })
                .on_submit(Action::SkillAddSave);
        }
        let mut scopes = row![].spacing(t.controls.button_gap);
        for (scope, label) in book.scope_choices() {
            let look = if new.scope.as_deref() == Some(scope.as_str()) {
                Look::Selected
            } else {
                Look::Secondary
            };
            scopes = scopes.push(btn(
                t,
                label,
                look,
                Leading::None,
                Some(Action::SkillAddScope { scope }),
            ));
        }
        let can_save = book.can_add();
        let mut form = column![slug, title, instruction, scopes.wrap()].spacing(m.head_gap);
        if let Some(hint) = book.add_scope_hint() {
            form = form.push(text(hint).size(t.type_.foot).color(color(t.palette.text3)));
        }
        if let Some(note) = &book.add_note {
            form = form.push(inline_note(t, note));
        }
        col = col.push(
            container(
                column![
                    form,
                    row![
                        btn(
                            t,
                            skills::SAVE.into(),
                            Look::Primary,
                            Leading::None,
                            can_save.then_some(Action::SkillAddSave)
                        ),
                        btn(
                            t,
                            skills::CANCEL.into(),
                            Look::Secondary,
                            Leading::None,
                            (!busy).then_some(Action::SkillAddClose)
                        ),
                    ]
                    .spacing(t.controls.button_gap),
                ]
                .spacing(m.head_gap),
            )
            .padding(pad),
        );
        col = col.push(divider(t, 0.0));
    }

    if book.loaded && book.is_empty() {
        let rings = Rings {
            color: color(t.palette.text3),
            radius: m.empty_ring_r,
            stroke: m.empty_ring_stroke,
        };
        let size = rings.size();
        col = col.push(
            container(
                column![
                    canvas(rings).width(size.width).height(size.height),
                    gap(m.head_gap),
                    text(skills::EMPTY_TITLE)
                        .size(t.type_.empty_title)
                        .font(bold())
                        .color(color(t.palette.text2)),
                    gap(m.dot_gap),
                    text(skills::EMPTY_HINT)
                        .size(t.type_.empty_hint)
                        .color(color(t.palette.text3)),
                ]
                .align_x(Alignment::Center),
            )
            .width(Fill)
            .height(Length::Fixed(m.empty_height))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
        );
    }

    let inner = (width - 2.0 * (t.controls.card_pad_h + t.stroke.card)).max(80.0);
    let cols = (inner / (t.type_.body * t.type_.unit_width)).max(8.0) as usize;

    let entry = |skill: &'a robroz_contract::Skill, pending: bool| -> Element<'a, Action> {
        let mut top = row![
            text(skill.title.as_str())
                .size(t.type_.body)
                .font(bold())
                .color(color(t.palette.text)),
            mono(t, &skill.slug),
        ]
        .spacing(m.row_time_gap)
        .align_y(Alignment::Center);
        if pending {
            top = top.push(tag(t, skills::PENDING_TAG.as_str(), t.palette.accent));
        }
        top = top.push(space::horizontal());
        if book.just_saved.as_deref() == Some(skill.slug.as_str()) {
            top = top.push(
                text(skills::SAVED)
                    .size(t.type_.row_time)
                    .color(color(t.palette.text2)),
            );
        }
        top = top.push(
            text(skills::updated_text(&skill.updated_at, now))
                .size(t.type_.row_time)
                .color(color(t.palette.text3)),
        );
        let editing = book.editing.as_ref().filter(|e| e.slug == skill.slug);
        let asking = book.confirm_delete.as_deref() == Some(skill.slug.as_str());
        let body: Element<'a, Action> = match editing {
            Some(edit) => {
                let mut title = text_input(skills::TITLE_PLACEHOLDER.as_str(), &edit.title)
                    .size(t.type_.body)
                    .padding(Padding::from([6.0, 8.0]));
                let mut instruction =
                    text_input(skills::INSTRUCTION_PLACEHOLDER.as_str(), &edit.instruction)
                        .size(t.type_.body)
                        .padding(Padding::from([6.0, 8.0]));
                if !busy {
                    title = title.on_input(|text| Action::SkillEditTitle { text });
                    instruction = instruction
                        .on_input(|text| Action::SkillEditInstruction { text })
                        .on_submit(Action::SkillEditSave);
                }
                column![title, instruction].spacing(m.dot_gap).into()
            }
            None => {
                let cut = status::clamp_lines(&skill.instruction, cols, skills::INSTRUCTION_LINES);
                let long = cut.ends_with('…');
                let open = book.open.contains(&skill.slug);
                let shown = if open || !long {
                    skill.instruction.clone()
                } else {
                    cut
                };
                let mut body = column![text(shown)
                    .size(t.type_.body)
                    .line_height(LineHeight::Absolute(Pixels(t.type_.body_line)))
                    .color(color(t.palette.text))
                    .width(Fill)]
                .spacing(m.dot_gap);
                if long {
                    body = body.push(btn(
                        t,
                        if open {
                            confirm::SHOW_LESS
                        } else {
                            confirm::SHOW_ALL
                        }
                        .into(),
                        Look::Secondary,
                        Leading::None,
                        Some(Action::SkillToggleInstruction {
                            slug: skill.slug.clone(),
                        }),
                    ));
                }
                body.into()
            }
        };
        let actions: Element<'a, Action> = if pending {
            match &skill.confirm_id {
                Some(_) => row![
                    btn(
                        t,
                        confirm::SAVE.into(),
                        Look::Primary,
                        Leading::None,
                        (!busy).then(|| Action::SkillDecide {
                            slug: skill.slug.clone(),
                            keep: true,
                        })
                    ),
                    btn(
                        t,
                        confirm::DROP.into(),
                        if busy {
                            Look::Disabled
                        } else {
                            Look::Secondary
                        },
                        Leading::None,
                        (!busy).then(|| Action::SkillDecide {
                            slug: skill.slug.clone(),
                            keep: false,
                        })
                    ),
                ]
                .spacing(t.controls.button_gap)
                .into(),
                None => Space::new().height(Length::Fixed(0.0)).into(),
            }
        } else if editing.is_some() {
            row![
                btn(
                    t,
                    skills::SAVE.into(),
                    Look::Primary,
                    Leading::None,
                    book.can_save_edit().then_some(Action::SkillEditSave)
                ),
                btn(
                    t,
                    skills::CANCEL.into(),
                    Look::Secondary,
                    Leading::None,
                    (!busy).then_some(Action::SkillEditCancel)
                ),
            ]
            .spacing(t.controls.button_gap)
            .into()
        } else if asking {
            row![
                text(skills::DELETE_ASK)
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
                space::horizontal(),
                btn(
                    t,
                    skills::DELETE.into(),
                    Look::Primary,
                    Leading::None,
                    (!busy).then_some(Action::SkillDeleteConfirm)
                ),
                btn(
                    t,
                    skills::CANCEL.into(),
                    Look::Secondary,
                    Leading::None,
                    (!busy).then_some(Action::SkillDeleteCancel)
                ),
            ]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center)
            .into()
        } else {
            row![
                btn(
                    t,
                    skills::EDIT.into(),
                    Look::Secondary,
                    Leading::None,
                    (!busy).then_some(Action::SkillEdit {
                        slug: skill.slug.clone()
                    })
                ),
                btn(
                    t,
                    skills::DELETE.into(),
                    Look::Secondary,
                    Leading::None,
                    (!busy).then_some(Action::SkillDeleteAsk {
                        slug: skill.slug.clone()
                    })
                ),
            ]
            .spacing(t.controls.button_gap)
            .into()
        };
        let mut entry_col = column![top, body, actions].spacing(m.dot_gap + 2.0);
        if !pending && editing.is_none() && !asking && !book.chats.is_empty() {
            let mut toggles = row![text(skills::GROUP_ENABLE)
                .size(t.type_.foot)
                .color(color(t.palette.text3))]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center);
            for (group, name) in &book.chats {
                let load = book.group_load_state(group);
                let known = (load == skills::GroupLoad::Ready)
                    .then(|| book.enable_state(group, &skill.slug).known())
                    .flatten();
                let hint = match load {
                    skills::GroupLoad::Failed => Some(skills::LOAD_FAILED),
                    skills::GroupLoad::Ready if known.is_some() => None,
                    _ => Some(skills::LOADING),
                };
                let (track_on, track_off, knob) = if known.is_some() {
                    (
                        color(t.palette.accent),
                        color(t.palette.toggle_off),
                        color(t.palette.knob),
                    )
                } else {
                    let grey = color(t.palette.text3);
                    (grey, grey, color(t.palette.surface))
                };
                let toggle = hit_area(
                    t,
                    canvas(Toggle {
                        on: known.unwrap_or(false),
                        track_on,
                        track_off,
                        knob,
                        knob_size: t.controls.knob,
                    })
                    .width(t.controls.switch_w)
                    .height(t.controls.switch_h)
                    .into(),
                    Some(t.controls.switch_w),
                    t.controls.switch_h,
                    known.and_then(|_| {
                        (!busy).then(|| Action::SkillToggleGroup {
                            slug: skill.slug.clone(),
                            group: group.clone(),
                        })
                    }),
                );
                let mut cell = row![
                    text(name.as_str())
                        .size(t.type_.foot)
                        .color(color(t.palette.text2)),
                    toggle,
                ]
                .spacing(m.dot_gap)
                .align_y(Alignment::Center);
                if let Some(hint) = hint {
                    cell = cell.push(text(hint).size(t.type_.foot).color(color(t.palette.text3)));
                }
                toggles = toggles.push(cell);
            }
            entry_col = entry_col.push(toggles.wrap());
        }
        if let Some(note) = book.note_for(&skill.slug) {
            entry_col = entry_col.push(inline_note(t, note));
        }
        container(entry_col).padding(pad).width(Fill).into()
    };

    for (gi, group) in book.groups().into_iter().enumerate() {
        if gi > 0 {
            col = col.push(divider(t, 0.0));
        }
        col = col.push(
            container(
                text(group.title.clone())
                    .size(t.type_.row_name)
                    .font(bold())
                    .color(color(t.palette.text)),
            )
            .padding(Padding::from([m.row_pad_h * 0.75, m.row_pad_h])),
        );
        let sub = |label: &'static str, n: usize| -> Element<'a, Action> {
            container(
                text(format!("{label} {n}"))
                    .size(t.type_.foot)
                    .color(color(t.palette.text3)),
            )
            .padding(Padding::from([0.0, m.row_pad_h]))
            .into()
        };
        for (label, n) in group.heads() {
            if label == skills::KEPT_HEAD {
                if !group.pending.is_empty() {
                    col = col.push(divider(t, m.row_pad_h));
                    col = col.push(gap(m.dot_gap));
                }
                col = col.push(sub(label, n));
                break;
            }
            col = col.push(sub(label, n));
            for (i, skill) in group.pending.iter().enumerate() {
                if i > 0 {
                    col = col.push(divider(t, m.row_pad_h));
                }
                col = col.push(entry(skill, true));
            }
        }
        for (i, skill) in group.kept.iter().enumerate() {
            if i > 0 {
                col = col.push(divider(t, m.row_pad_h));
            }
            col = col.push(entry(skill, false));
        }
    }

    let tt = *t;
    container(col)
        .width(Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(color(tt.palette.surface).into()),
            text_color: Some(color(tt.palette.text)),
            border: Border::default()
                .width(tt.stroke.card)
                .color(color(tt.palette.border))
                .rounded(tt.radii.panel),
            shadow: shadow(&tt),
            ..container::Style::default()
        })
        .into()
}

// ---------- MCP page ----------

/// The MCP page: a list of servers, each with an enable switch and tool
/// count, plus a form to add or edit one, a tool list, and per-group toggles.
pub fn mcp_panel<'a>(
    t: &Skin,
    book: &'a McpBook,
    _now: i64,
    _width: f32,
    titled: bool,
) -> Element<'a, Action> {
    let m = t.metrics;
    let busy = book.busy;
    let mut head = if titled {
        row![text(mcp::TITLE)
            .size(t.type_.list_title)
            .font(bold())
            .color(color(t.palette.text))]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center)
    } else {
        row![]
            .spacing(t.controls.button_gap)
            .align_y(Alignment::Center)
    };
    if titled && !book.servers.is_empty() {
        head = head.push(
            text(book.servers.len().to_string())
                .size(t.type_.list_count)
                .color(color(t.palette.text3)),
        );
    }
    head = head.push(space::horizontal());
    if book.form.is_none() {
        head = head.push(btn(
            t,
            mcp::ADD.into(),
            Look::Secondary,
            Leading::None,
            (!busy).then_some(Action::McpAddOpen),
        ));
    }
    let mut col = if titled || book.form.is_none() {
        column![
            container(head)
                .height(Length::Fixed(m.panel_head_height))
                .padding(Padding::from([0.0, m.row_pad_h]))
                .align_y(Alignment::Center),
            divider(t, 0.0),
        ]
    } else {
        column![]
    };
    let pad = Padding::from([m.row_pad_h * 0.75, m.row_pad_h]);

    if let Some(note) = &book.note {
        col = col.push(
            container(
                text(note.as_str())
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
            )
            .padding(Padding::from([8.0, m.row_pad_h])),
        );
    }

    if book.form.as_ref().is_some_and(|f| f.id.is_none()) {
        col = col.push(container(mcp_form(t, book, busy)).padding(pad));
        col = col.push(divider(t, 0.0));
    }

    if book.loaded && book.is_empty() && book.form.is_none() {
        let rings = Rings {
            color: color(t.palette.text3),
            radius: m.empty_ring_r,
            stroke: m.empty_ring_stroke,
        };
        let size = rings.size();
        col = col.push(
            container(
                column![
                    canvas(rings).width(size.width).height(size.height),
                    gap(m.head_gap),
                    text(mcp::EMPTY_TITLE)
                        .size(t.type_.empty_title)
                        .font(bold())
                        .color(color(t.palette.text2)),
                    gap(m.dot_gap),
                    text(mcp::EMPTY_HINT)
                        .size(t.type_.empty_hint)
                        .color(color(t.palette.text3)),
                ]
                .align_x(Alignment::Center)
                .spacing(0),
            )
            .center_x(Fill)
            .padding(Padding::from([40.0, m.row_pad_h])),
        );
        return col.into();
    }

    let mut list = column![];
    for (i, server) in book.servers.iter().enumerate() {
        if i > 0 {
            list = list.push(divider(t, 0.0));
        }
        let editing = book
            .form
            .as_ref()
            .is_some_and(|f| f.id.as_deref() == Some(server.id.as_str()));
        if editing {
            list = list.push(container(mcp_form(t, book, busy)).padding(pad));
            continue;
        }
        list = list.push(container(mcp_row(t, book, server, busy)).padding(pad));
    }
    col = col.push(list);
    col.into()
}

fn mcp_form<'a>(t: &Skin, book: &'a McpBook, busy: bool) -> Element<'a, Action> {
    let m = t.metrics;
    let form = book.form.as_ref().expect("form open");
    let mut name = text_input(mcp::NAME_PLACEHOLDER.as_str(), &form.name)
        .size(t.type_.body)
        .padding(Padding::from([8.0, 10.0]));
    if !busy {
        name = name.on_input(|text| Action::McpFormName { text });
    }
    let mut transports = row![].spacing(t.controls.button_gap);
    for (http, label) in [(false, mcp::STDIO), (true, mcp::HTTP)] {
        let selected = form.transport == mcp::Transport::Http && http
            || form.transport == mcp::Transport::Stdio && !http;
        let look = if selected {
            Look::Selected
        } else {
            Look::Secondary
        };
        transports = transports.push(btn(
            t,
            label.into(),
            look,
            Leading::None,
            Some(Action::McpFormTransport { http }),
        ));
    }
    let mut fields =
        column![labelled(t, mcp::LABEL_NAME.as_str(), name), transports].spacing(m.head_gap);
    match form.transport {
        mcp::Transport::Stdio => {
            let mut command = text_input(mcp::COMMAND_PLACEHOLDER.as_str(), &form.command)
                .size(t.type_.body)
                .padding(Padding::from([8.0, 10.0]));
            let mut args = text_input(mcp::ARGS_PLACEHOLDER.as_str(), &form.args)
                .size(t.type_.body)
                .padding(Padding::from([8.0, 10.0]));
            if !busy {
                command = command.on_input(|text| Action::McpFormCommand { text });
                args = args.on_input(|text| Action::McpFormArgs { text });
            }
            fields = fields
                .push(labelled(t, mcp::LABEL_COMMAND.as_str(), command))
                .push(labelled(t, mcp::LABEL_ARGS.as_str(), args));
        }
        mcp::Transport::Http => {
            let mut url = text_input(mcp::URL_PLACEHOLDER.as_str(), &form.url)
                .size(t.type_.body)
                .padding(Padding::from([8.0, 10.0]));
            if !busy {
                url = url.on_input(|text| Action::McpFormUrl { text });
            }
            fields = fields.push(labelled(t, mcp::LABEL_URL.as_str(), url));
        }
    }
    for (index, secret) in form.secrets.iter().enumerate() {
        fields = fields.push(mcp_secret_row(t, index, secret, busy));
    }
    fields = fields.push(btn(
        t,
        mcp::ADD_SECRET.into(),
        Look::Secondary,
        Leading::None,
        (!busy).then_some(Action::McpSecretAdd),
    ));
    if let Some(note) = &book.form_note {
        fields = fields.push(inline_note(t, note));
    }
    let can_save = form.problem().is_none();
    column![
        fields,
        row![
            btn(
                t,
                mcp::SAVE.into(),
                Look::Primary,
                Leading::None,
                (can_save && !busy).then_some(Action::McpSave)
            ),
            btn(
                t,
                mcp::CANCEL.into(),
                Look::Secondary,
                Leading::None,
                (!busy).then_some(Action::McpFormClose)
            ),
        ]
        .spacing(t.controls.button_gap),
    ]
    .spacing(m.head_gap)
    .into()
}

fn masked_key_row<'a>(
    t: &Skin,
    key: &'a secret::MaskedKey,
    placeholder: &'static str,
    busy: bool,
    on_draft: impl Fn(String) -> Action + 'a,
    on_clear: Action,
    on_undo: Action,
    on_submit: Option<Action>,
) -> Element<'a, Action> {
    if key.shows_stored() {
        row![
            text(secret::filled(key.last4.as_deref()))
                .size(t.type_.body)
                .color(color(t.palette.text)),
            space::horizontal(),
            btn(
                t,
                secret::CLEAR.into(),
                Look::Secondary,
                Leading::None,
                (!busy).then_some(on_clear),
            ),
        ]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
    } else if key.clearing {
        row![
            text(secret::PENDING_CLEAR)
                .size(t.type_.body)
                .color(color(t.palette.text2)),
            space::horizontal(),
            btn(
                t,
                secret::UNDO.into(),
                Look::Secondary,
                Leading::None,
                (!busy).then_some(on_undo),
            ),
        ]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
    } else {
        let mut input = text_input(placeholder, &key.draft)
            .secure(true)
            .size(t.type_.body)
            .padding(field_padding())
            .style(field_input_style(*t));
        if !busy {
            input = input.on_input(on_draft);
            if let Some(submit) = on_submit {
                input = input.on_submit(submit);
            }
        }
        input.into()
    }
}

fn mcp_secret_row<'a>(
    t: &Skin,
    index: usize,
    secret: &'a mcp::SecretDraft,
    busy: bool,
) -> Element<'a, Action> {
    let mut name = text_input(mcp::SECRET_NAME_PLACEHOLDER.as_str(), &secret.name)
        .size(t.type_.body)
        .padding(Padding::from([8.0, 10.0]));
    if !busy {
        name = name.on_input(move |text| Action::McpSecretName { index, text });
    }
    let env_look = if secret.placement == McpSecretIn::Env {
        Look::Selected
    } else {
        Look::Secondary
    };
    let header_look = if secret.placement == McpSecretIn::Header {
        Look::Selected
    } else {
        Look::Secondary
    };
    let place = row![
        btn(
            t,
            mcp::ENV.into(),
            env_look,
            Leading::None,
            Some(Action::McpSecretPlacement {
                index,
                header: false
            }),
        ),
        btn(
            t,
            mcp::HEADER.into(),
            header_look,
            Leading::None,
            Some(Action::McpSecretPlacement {
                index,
                header: true
            }),
        ),
        btn(
            t,
            mcp::REMOVE_SECRET.into(),
            Look::Secondary,
            Leading::None,
            (!busy).then_some(Action::McpSecretRemove { index }),
        ),
    ]
    .spacing(t.controls.button_gap)
    .align_y(Alignment::Center);
    let key_row = masked_key_row(
        t,
        &secret.key,
        mcp::SECRET_KEY_PLACEHOLDER.as_str(),
        busy,
        move |text| Action::McpSecretKey { index, text },
        Action::McpSecretClear { index },
        Action::McpSecretCancel { index },
        None,
    );
    column![
        labelled(t, mcp::LABEL_SECRET_NAME.as_str(), name),
        place,
        labelled(t, mcp::LABEL_SECRET_KEY.as_str(), key_row),
    ]
    .spacing(t.metrics.head_gap)
    .into()
}

/// A switch drawn with containers, not `canvas(Toggle)`. Iced's canvas
/// cache can leave a stale geometry when the first row used to be the
/// empty-state Rings, or when a row is removed.
fn switch_control<'a>(
    t: &Skin,
    on: bool,
    track_on: iced::Color,
    track_off: iced::Color,
    knob: iced::Color,
    on_press: Option<Action>,
) -> Element<'a, Action> {
    let w = t.controls.switch_w;
    let h = t.controls.switch_h;
    let k = t.controls.knob;
    let inset = ((h - k) / 2.0).max(0.0);
    let knob_dot = container(space::horizontal())
        .width(Length::Fixed(k))
        .height(Length::Fixed(k))
        .style(move |_| container::Style {
            background: Some(knob.into()),
            border: Border::default().rounded(k / 2.0),
            ..container::Style::default()
        });
    let track = container(knob_dot)
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .padding(Padding::from([inset, inset]))
        .align_x(if on { Alignment::End } else { Alignment::Start })
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: Some(if on { track_on } else { track_off }.into()),
            border: Border::default().rounded(h / 2.0),
            ..container::Style::default()
        });
    hit_area(t, track.into(), Some(w), h, on_press)
}

fn mcp_row<'a>(
    t: &Skin,
    book: &'a McpBook,
    server: &'a McpServer,
    busy: bool,
) -> Element<'a, Action> {
    let m = t.metrics;
    let id = server.id.clone();
    let open = book.open.as_deref() == Some(server.id.as_str());
    let asking = book.confirm_delete.as_deref() == Some(server.id.as_str());
    let unapproved = mcp::command_unapproved(server);
    let toggle = switch_control(
        t,
        server.enabled,
        color(t.palette.accent),
        color(t.palette.toggle_off),
        color(t.palette.knob),
        (!busy).then(|| Action::McpToggleEnabled { id: id.clone() }),
    );
    let mut name = row![text(server.name.as_str())
        .size(t.type_.row_name)
        .font(bold())
        .color(color(t.palette.text))]
    .spacing(t.controls.button_gap)
    .align_y(Alignment::Center);
    if unapproved {
        name = name.push(tag(t, mcp::COMMAND_UNAPPROVED.as_str(), t.palette.accent));
    }
    let mut title = column![name].spacing(2.0);
    title = title.push(
        text(mcp::row_subtitle(server))
            .size(t.type_.foot)
            .color(color(t.palette.text3)),
    );
    let top = row![
        button(title)
            .padding(0)
            .style(|_, _| button::Style {
                background: None,
                ..button::Style::default()
            })
            .on_press(Action::McpOpen { id: id.clone() }),
        space::horizontal(),
        toggle,
    ]
    .spacing(t.controls.button_gap)
    .align_y(Alignment::Center)
    .width(Fill);

    let mut col = column![top].spacing(m.dot_gap + 2.0);
    if let Some(test) = book.test_for(&server.id) {
        col = col.push(
            text(test.text())
                .size(t.type_.foot)
                .color(color(match test {
                    mcp::TestState::Ok { .. } => t.palette.text2,
                    mcp::TestState::Failed(_) => t.palette.text2,
                    mcp::TestState::Running => t.palette.text3,
                })),
        );
    }
    if open {
        if book.command_prompt_for(&server.id).is_some() {
            col = col.push(mcp_command_prompt(t, book, server, busy));
        } else {
            col = col.push(mcp_tools(t, book, server));
        }
        if !book.chats.is_empty() {
            col = col.push(mcp_group_toggles(t, book, &server.id, busy));
        }
        if asking {
            col = col.push(
                column![
                    text(mcp::DELETE_ASK)
                        .size(t.type_.foot)
                        .color(color(t.palette.text2)),
                    row![
                        btn(
                            t,
                            mcp::DELETE.into(),
                            Look::Primary,
                            Leading::None,
                            (!busy).then_some(Action::McpDeleteConfirm),
                        ),
                        btn(
                            t,
                            mcp::CANCEL.into(),
                            Look::Secondary,
                            Leading::None,
                            Some(Action::McpDeleteCancel),
                        ),
                    ]
                    .spacing(t.controls.button_gap),
                ]
                .spacing(m.dot_gap),
            );
        } else {
            col = col.push(
                row![
                    btn(
                        t,
                        mcp::EDIT.into(),
                        Look::Secondary,
                        Leading::None,
                        (!busy).then_some(Action::McpEdit { id: id.clone() }),
                    ),
                    btn(
                        t,
                        mcp::DELETE.into(),
                        Look::Secondary,
                        Leading::None,
                        (!busy).then_some(Action::McpDeleteAsk { id: id.clone() }),
                    ),
                ]
                .spacing(t.controls.button_gap),
            );
        }
    }
    if let Some(note) = book.note_for(&server.id) {
        col = col.push(inline_note(t, note));
    }
    col.into()
}

fn mcp_command_prompt<'a>(
    t: &Skin,
    book: &'a McpBook,
    server: &'a McpServer,
    busy: bool,
) -> Element<'a, Action> {
    let prompt = book.command_prompt_for(&server.id).expect("prompt");
    let command_block = column![
        field_label(t, mcp::LABEL_PROMPT_COMMAND.as_str()),
        mono_wrap(t, prompt.command.as_str()),
    ]
    .spacing(2.0);
    let mut args_block = column![field_label(t, mcp::LABEL_PROMPT_ARGS.as_str())].spacing(2.0);
    for arg in &prompt.args {
        args_block = args_block.push(mono_wrap(t, arg.as_str()));
    }
    let id = server.id.clone();
    let mut col = column![
        command_block,
        args_block,
        text(mcp::COMMAND_RUNS_LOCAL)
            .size(t.type_.foot)
            .color(color(t.palette.text3)),
        row![
            btn(
                t,
                mcp::APPROVE_RUN.into(),
                Look::Primary,
                Leading::None,
                (!busy).then_some(Action::McpApproveCommand { id: id.clone() }),
            ),
            btn(
                t,
                mcp::CANCEL.into(),
                Look::Secondary,
                Leading::None,
                Some(Action::McpCancelCommand { id }),
            ),
        ]
        .spacing(t.controls.button_gap),
    ]
    .spacing(t.metrics.dot_gap);
    if let Some(err) = book.approve_note_for(&server.id) {
        col = col.push(text(err).size(t.type_.foot).color(color(t.palette.accent)));
    }
    col.into()
}

fn mcp_tools<'a>(t: &Skin, book: &'a McpBook, server: &'a McpServer) -> Element<'a, Action> {
    if mcp::command_unapproved(server) && book.command_prompt_for(&server.id).is_none() {
        return text(mcp::TOOLS_NEED_APPROVAL)
            .size(t.type_.foot)
            .color(color(t.palette.text3))
            .into();
    }
    match book.tools.get(&server.id) {
        None => text(mcp::LOADING)
            .size(t.type_.foot)
            .color(color(t.palette.text3))
            .into(),
        Some(Err(err)) => text(mcp::cannot_connect(err))
            .size(t.type_.foot)
            .color(color(t.palette.text2))
            .into(),
        Some(Ok(tools)) if tools.is_empty() => text(Key::ViewMcpNoTools)
            .size(t.type_.foot)
            .color(color(t.palette.text3))
            .into(),
        Some(Ok(tools)) => {
            let mut list = column![].spacing(t.metrics.dot_gap);
            for tool in tools {
                let mut row = row![text(tool.name.as_str())
                    .size(t.type_.body)
                    .color(color(t.palette.text))]
                .spacing(t.controls.button_gap)
                .align_y(Alignment::Center);
                if tool.needs_approval {
                    row = row.push(tag(t, mcp::NEEDS_APPROVAL.as_str(), t.palette.accent));
                }
                let mut item = column![row];
                if let Some(desc) = &tool.description {
                    item = item.push(
                        text(desc.as_str())
                            .size(t.type_.foot)
                            .color(color(t.palette.text3)),
                    );
                }
                list = list.push(item);
            }
            list.into()
        }
    }
}

fn mcp_group_toggles<'a>(
    t: &Skin,
    book: &'a McpBook,
    server_id: &str,
    busy: bool,
) -> Element<'a, Action> {
    let m = t.metrics;
    let mut toggles = row![text(mcp::GROUP_ENABLE)
        .size(t.type_.foot)
        .color(color(t.palette.text3))]
    .spacing(t.controls.button_gap)
    .align_y(Alignment::Center);
    for (group, name) in &book.chats {
        let load = book.group_load_state(group);
        let known = (load == mcp::GroupLoad::Ready)
            .then(|| book.enable_state(group, server_id))
            .flatten();
        let hint = match load {
            mcp::GroupLoad::Failed => Some(mcp::LOAD_FAILED),
            mcp::GroupLoad::Ready if known.is_some() => None,
            _ => Some(mcp::LOADING),
        };
        let (track_on, track_off, knob) = if known.is_some() {
            (
                color(t.palette.accent),
                color(t.palette.toggle_off),
                color(t.palette.knob),
            )
        } else {
            let grey = color(t.palette.text3);
            (grey, grey, color(t.palette.surface))
        };
        let toggle = switch_control(
            t,
            known.unwrap_or(false),
            track_on,
            track_off,
            knob,
            known.and_then(|_| {
                (!busy).then(|| Action::McpToggleGroup {
                    id: server_id.to_string(),
                    group: group.clone(),
                })
            }),
        );
        let mut cell = row![
            text(name.as_str())
                .size(t.type_.foot)
                .color(color(t.palette.text2)),
            toggle,
        ]
        .spacing(m.dot_gap)
        .align_y(Alignment::Center);
        if let Some(hint) = hint {
            cell = cell.push(text(hint).size(t.type_.foot).color(color(t.palette.text3)));
        }
        toggles = toggles.push(cell);
    }
    toggles.wrap().into()
}

// ---------- Conversation list ----------

fn avatar_at<'a, M: 'a>(t: &Skin, name: &str, size: f32) -> Element<'a, M> {
    let t = *t;
    container(
        text(conversations::initial(name))
            .size((size * 0.4).max(11.0))
            .font(bold())
            .color(color(t.palette.text2)),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(move |_| container::Style {
        background: Some(color(t.palette.sunken).into()),
        border: Border::default().rounded(size / 2.0),
        ..container::Style::default()
    })
    .into()
}

/// Time column width estimate (`10月7日`, `昨天`, `23:59`).
const TIME_W: f32 = 44.0;
/// Accent bar on the open chat's row (drawn in the row's left padding).
const SELECTED_BAR: f32 = 3.0;
const ROW_PAD_H: f32 = 8.0;
const ROW_GAP: f32 = 8.0;

/// A conversation row: the open chat sits on `surface` with a border, so
/// it stands out on the sidebar in both themes; hover uses `control_hover`.
pub fn conversation_row_style(
    t: Skin,
    selected: bool,
) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, status| {
        let (bg, border) = if selected {
            (
                Some(Background::Color(color(t.palette.surface))),
                Border::default()
                    .width(t.stroke.card)
                    .color(color(t.palette.border2))
                    .rounded(t.radii.row),
            )
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            (
                Some(Background::Color(color(t.palette.control_hover))),
                Border::default().rounded(t.radii.row),
            )
        } else {
            (None, Border::default().rounded(t.radii.row))
        };
        button::Style {
            background: bg,
            text_color: color(t.palette.text),
            border,
            ..button::Style::default()
        }
    }
}

/// Width units that fit in `px` at font size `size`.
fn units_in(t: &Skin, px: f32, size: f32) -> usize {
    (px / (size * t.type_.unit_width)).max(4.0) as usize
}

fn unread_dot<'a, M: 'a>(t: &Skin) -> Element<'a, M> {
    use robroz_tokens::home as h;
    let d = h::UNREAD_DOT;
    let accent = t.palette.accent;
    container(
        Space::new()
            .width(Length::Fixed(d))
            .height(Length::Fixed(d)),
    )
    .style(move |_| container::Style {
        background: Some(color(accent).into()),
        border: Border::default().rounded(d / 2.0),
        ..container::Style::default()
    })
    .into()
}

/// Up to three group avatars, stacked and overlapped like the chat title
/// capsule (24px, 8px overlap). The ring matches the row background.
fn group_face<'a>(
    t: &Skin,
    avatars: &[crate::avatar::BroAvatar],
    ring: crate::theme::Rgb,
) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let n = avatars.len().min(3);
    crate::avatar_view::avatar_stack_at(
        t,
        &avatars[..n],
        0,
        ring,
        h::GROUP_STACK,
        h::GROUP_STACK_OVERLAP,
    )
}

fn conversation_row<'a>(
    t: &Skin,
    c: &'a Conversation,
    selected: bool,
    now: i64,
    width: f32,
) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let time = c
        .last_at
        .as_deref()
        .map(|at| conversations::list_time(at, now, &Local))
        .unwrap_or_default();
    let ring = if selected {
        t.palette.surface
    } else {
        t.palette.window
    };
    let face: Element<'a, Action> = if !c.dm && !c.group_avatars.is_empty() {
        group_face(t, &c.group_avatars, ring)
    } else {
        match c.bro_avatar {
            Some(a) => crate::avatar_view::bro_avatar(t, a, h::ROW_AVATAR),
            None => avatar_at(t, &c.name, h::ROW_AVATAR),
        }
    };
    let face = container(face)
        .width(Length::Fixed(h::ROW_FACE_W))
        .align_x(Alignment::Center);
    let body_w = width - 2.0 * ROW_PAD_H - ROW_GAP - h::ROW_FACE_W;
    let name = conversations::fit(&c.name, units_in(t, body_w - TIME_W, h::ROW_NAME));
    let truncated = name != c.name;
    let has_preview = conversations::has_preview(c);
    let preview = conversations::fit(
        c.preview.as_deref().unwrap_or_default(),
        units_in(t, body_w, h::ROW_PREVIEW),
    );
    let mut time_col = column![text(time)
        .size(h::ROW_TIME)
        .wrapping(Wrapping::None)
        .color(color(t.palette.text3))]
    .spacing(4)
    .align_x(Alignment::End);
    if c.unread > 0 {
        time_col = time_col.push(unread_dot(t));
    }
    let top = row![
        container(
            text(name)
                .size(h::ROW_NAME)
                .font(bold())
                .wrapping(Wrapping::None)
                .color(color(t.palette.text)),
        )
        .width(Fill)
        .clip(true),
        time_col,
    ]
    .spacing(6)
    // No preview: the name sits on the row's centre line, no empty line
    // below it.
    .align_y(if has_preview {
        Alignment::Start
    } else {
        Alignment::Center
    });
    let mut body = column![top].spacing(2).width(Fill).clip(true);
    if has_preview {
        body = body.push(
            container(
                text(preview)
                    .size(h::ROW_PREVIEW)
                    .wrapping(Wrapping::None)
                    .color(color(t.palette.text2)),
            )
            .width(Fill)
            .clip(true),
        );
    }
    let content: Element<'a, Action> =
        container(row![face, body].spacing(ROW_GAP).align_y(Alignment::Center))
            .width(Fill)
            .height(Length::Fixed(h::ROW))
            .padding(Padding::from([0.0, ROW_PAD_H]))
            .align_y(Alignment::Center)
            .into();
    let inner: Element<'a, Action> = if selected {
        let accent = t.palette.accent;
        let bar = container(Space::new())
            .width(Length::Fixed(SELECTED_BAR))
            .height(Length::Fixed(h::ROW_AVATAR - 8.0))
            .style(move |_| container::Style {
                background: Some(color(accent).into()),
                border: Border::default().rounded(SELECTED_BAR / 2.0),
                ..container::Style::default()
            });
        iced::widget::stack![
            content,
            container(bar)
                .height(Fill)
                .align_y(Alignment::Center)
                .padding(Padding {
                    left: 2.0,
                    ..Padding::ZERO
                }),
        ]
        .into()
    } else {
        content
    };
    let row_btn: Element<'a, Action> = button(inner)
        .width(Fill)
        .padding(0)
        .style(conversation_row_style(*t, selected))
        .on_press(Action::OpenChat {
            group: c.group_id.clone(),
        })
        .into();
    // Truncated names show the full name on hover. The sidebar stays 220px.
    if truncated {
        name_tooltip(t, row_btn, c.name.clone())
    } else {
        row_btn
    }
}

fn name_tooltip<'a>(t: &Skin, content: Element<'a, Action>, full: String) -> Element<'a, Action> {
    let t = *t;
    tooltip(
        content,
        container(
            text(full)
                .size(t.type_.foot)
                .color(color(t.palette.text))
                .wrapping(Wrapping::None),
        )
        .padding(Padding::from([6.0, 8.0]))
        .style(move |_| container::Style {
            background: Some(color(t.palette.surface).into()),
            border: Border::default()
                .width(1.0)
                .color(color(t.palette.border2))
                .rounded(8.0),
            ..container::Style::default()
        }),
        tooltip::Position::FollowCursor,
    )
    .gap(8)
    .into()
}

/// Inputs for the v0.2 home list (sidebar or narrow full-screen).
pub struct HomeList<'a> {
    pub list: &'a Conversations,
    pub selected: Option<&'a str>,
    pub now: i64,
    pub width: f32,
    pub query: &'a str,
    pub me_name: Option<&'a str>,
    pub self_menu: bool,
    pub scale: f32,
}

fn translucent(rgb: crate::theme::Rgb, a: f32) -> Color {
    let mut c = color(rgb);
    c.a = a;
    c
}

fn float_button_style(t: Skin) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    move |_, status| {
        let a = match status {
            button::Status::Hovered | button::Status::Pressed => 0.94,
            _ => 0.82,
        };
        button::Style {
            background: Some(Background::Color(translucent(t.palette.surface, a))),
            text_color: color(t.palette.text),
            border: Border {
                color: color(t.palette.border2),
                width: 1.0,
                radius: (robroz_tokens::home::FLOAT_BTN / 2.0).into(),
            },
            ..button::Style::default()
        }
    }
}

fn plus_button(t: &Skin) -> Element<'static, Action> {
    use robroz_tokens::home as h;
    button(
        text("+")
            .size(22.0)
            .height(Fill)
            .width(Fill)
            .center()
            .color(color(t.palette.text)),
    )
    .width(Length::Fixed(h::FLOAT_BTN))
    .height(Length::Fixed(h::FLOAT_BTN))
    .padding(0)
    .style(float_button_style(*t))
    .on_press(Action::NewBro)
    .into()
}

/// Search capsule: the capsule draws the fill and border; the input is
/// borderless and transparent, 16px in from each side.
fn search_capsule<'a>(t: &Skin, query: &'a str) -> Element<'a, Action> {
    capsule_search(
        t,
        Key::HomeSearchPlaceholder.as_str(),
        query,
        Some(|text| Action::HomeSearchEdit { text }),
        None,
    )
}

/// The home search capsule: a borderless transparent input inside a
/// pill, 16px side padding. Find-in-chat uses the same component.
fn capsule_search<'a>(
    t: &Skin,
    placeholder: &'static str,
    query: &'a str,
    on_input: Option<fn(String) -> Action>,
    on_submit: Option<Action>,
) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let t = *t;
    let mut input = text_input(placeholder, query)
        .size(t.type_.body)
        .padding(0)
        .width(Fill)
        .style(composer_input_style(t));
    if let Some(on_input) = on_input {
        input = input.on_input(on_input);
    }
    if let Some(on_submit) = on_submit {
        input = input.on_submit(on_submit);
    }
    container(input)
        .width(Fill)
        .height(Length::Fixed(h::SEARCH))
        .padding(Padding::from([0.0, h::SEARCH_PAD]))
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(translucent(t.palette.surface, 0.92))),
            border: Border {
                color: color(t.palette.border2),
                width: 1.0,
                radius: (h::SEARCH / 2.0).into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// 32px circle with the first letter, on `surface` with a hairline so it
/// reads as a circle on the sidebar fill.
fn self_avatar<'a>(t: &Skin, name: &str) -> Element<'a, Action> {
    let t = *t;
    container(
        text(conversations::initial(name))
            .size(t.type_.row_name)
            .font(bold())
            .color(color(t.palette.text)),
    )
    .width(Length::Fixed(32.0))
    .height(Length::Fixed(32.0))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(move |_| container::Style {
        background: Some(color(t.palette.surface).into()),
        border: Border::default()
            .width(1.0)
            .color(color(t.palette.border2))
            .rounded(16.0),
        ..container::Style::default()
    })
    .into()
}

/// Self row: 32px avatar, name, chevron. It never changes look when its
/// menu is open: the menu floats above it (see `conversation_list`).
fn self_row<'a>(t: &Skin, name: &'a str, open: bool) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let row_el = container(
        row![
            self_avatar(t, name),
            text(name.to_string())
                .size(h::ROW_NAME)
                .color(color(t.palette.text))
                .width(Fill),
            text("›").size(18.0).color(color(t.palette.text3)),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .height(Fill)
    .align_y(Alignment::Center);
    button(row_el)
        .width(Fill)
        .height(Length::Fixed(h::CARD_ROW))
        .padding(Padding::from([0.0, 12.0]))
        .style(conversation_row_style(*t, false))
        .on_press(if open {
            Action::SelfMenuClose
        } else {
            Action::SelfMenuOpen
        })
        .into()
}

fn menu_row<'a>(
    t: &Skin,
    glyph: crate::icons::Glyph,
    label: String,
    danger: bool,
    scale: f32,
    on: Action,
) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let t = *t;
    let ink = if danger {
        crate::theme::DANGER
    } else {
        t.palette.text
    };
    let body = container(
        row![
            crate::icons::icon(glyph, ink, h::MENU_ICON, scale),
            text(label)
                .size(h::MENU_TEXT)
                .wrapping(Wrapping::None)
                .color(color(ink)),
        ]
        .spacing(h::MENU_ICON_GAP)
        .align_y(Alignment::Center),
    )
    .height(Fill)
    .align_y(Alignment::Center);
    button(body)
        .width(Fill)
        .height(Length::Fixed(h::MENU_ROW))
        .padding(Padding::from([0.0, 12.0]))
        .style(move |_, status| {
            let bg = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                Some(Background::Color(color(t.palette.control_hover)))
            } else {
                None
            };
            button::Style {
                background: bg,
                text_color: color(ink),
                border: Border::default().rounded(h::MENU_R - h::MENU_PAD),
                ..button::Style::default()
            }
        })
        .on_press(on)
        .into()
}

/// 1px divider with 4px above and 4px below.
fn menu_divider<'a>(t: &Skin) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let t = *t;
    container(
        container(Space::new().height(Length::Fixed(1.0)))
            .width(Fill)
            .style(move |_| container::Style {
                background: Some(color(t.palette.divider).into()),
                ..container::Style::default()
            }),
    )
    .width(Fill)
    .padding(Padding {
        top: h::MENU_DIVIDER_GAP,
        bottom: h::MENU_DIVIDER_GAP,
        left: 8.0,
        right: 8.0,
    })
    .into()
}

/// Popover surface: 4px inner padding, items 36px, rounded 16.
fn popover_box<'a>(t: &Skin, width: f32, rows: Vec<Element<'a, Action>>) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let t = *t;
    let mut col = column![].width(Fill);
    for r in rows {
        col = col.push(r);
    }
    container(col)
        .width(Length::Fixed(width))
        .padding(h::MENU_PAD)
        .style(move |_| container::Style {
            background: Some(color(t.palette.surface).into()),
            border: Border {
                color: color(t.palette.border2),
                width: 1.0,
                radius: h::MENU_R.into(),
            },
            shadow: IcedShadow {
                color: Color {
                    a: 0.16,
                    ..color(t.palette.text)
                },
                offset: Vector::new(0.0, 4.0),
                blur_radius: 16.0,
            },
            ..container::Style::default()
        })
        .into()
}

fn self_menu_popover<'a>(t: &Skin, width: f32, scale: f32) -> Element<'a, Action> {
    let t = *t;
    let rows = vec![
        menu_row(
            &t,
            crate::icons::Glyph::Settings,
            Key::SettingsTitle.into(),
            false,
            scale,
            Action::OpenSettings,
        ),
        menu_divider(&t),
        menu_row(
            &t,
            crate::icons::Glyph::SignOut,
            Key::SignOutOnThisDevice.into(),
            true,
            scale,
            Action::SignOut,
        ),
    ];
    popover_box(&t, width, rows)
}

/// Group / DM `•••` menu: View profile, Find in chat and, for people who
/// can manage the group, Rename, Members and (after a divider, red)
/// Delete group. The three manage entries share one condition, so they
/// always appear together.
pub fn group_menu_popover<'a>(t: &Skin, can_manage: bool, scale: f32) -> Element<'a, Action> {
    use crate::home::GroupMenuEntry as E;
    use crate::icons::Glyph;
    let t = *t;
    let mut rows = Vec::new();
    for entry in crate::home::group_menu_entries(can_manage) {
        let (glyph, key, danger, on) = match entry {
            E::Profile => (
                Glyph::Profile,
                Key::GroupMenuProfile,
                false,
                Action::GroupProfileOpen,
            ),
            E::Find => (
                Glyph::Search,
                Key::GroupMenuFind,
                false,
                Action::FindInChatOpen,
            ),
            E::Rename => (
                Glyph::Pencil,
                Key::GroupMenuRename,
                false,
                Action::GroupRenameOpen,
            ),
            E::Members => (
                Glyph::Members,
                Key::GroupProfileMembers,
                false,
                Action::GroupMembersOpen,
            ),
            E::Delete => {
                rows.push(menu_divider(&t));
                (
                    Glyph::Delete,
                    Key::GroupMenuDelete,
                    true,
                    Action::GroupDeleteAsk,
                )
            }
        };
        rows.push(menu_row(&t, glyph, key.into(), danger, scale, on));
    }
    popover_box(&t, robroz_tokens::home::MENU_W, rows)
}

/// The IM list: a `+` on top, 64px rows, a floating search capsule and
/// the self-row (Settings / sign out). `width` fits the preview line.
pub fn conversation_list<'a>(t: &Skin, home: HomeList<'a>) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let mut rows = column![].spacing(0);
    rows = rows.push(
        container(plus_button(t))
            .width(Fill)
            .align_x(Alignment::End)
            .padding(8.0),
    );
    let shown = home.list.matching(home.query);
    if home.list.is_empty() {
        rows = rows.push(
            container(
                text(conversations::EMPTY_HINT)
                    .size(t.type_.empty_hint)
                    .color(color(t.palette.text2)),
            )
            .padding(12.0),
        );
    } else if shown.is_empty() {
        rows = rows.push(
            container(
                text(Key::FindInChatNoResults)
                    .size(t.type_.empty_hint)
                    .color(color(t.palette.text2)),
            )
            .padding(12.0),
        );
    } else {
        for c in shown {
            rows = rows.push(conversation_row(
                t,
                c,
                home.selected == Some(c.group_id.as_str()),
                home.now,
                home.width,
            ));
        }
    }
    // Room to scroll the last row above the floating capsule and self-row.
    rows = rows.push(Space::new().height(h::SEARCH + h::SEARCH_LIFT + h::CARD_ROW + 12.0));
    let list = scrollable(rows).height(Fill).width(Fill);
    const FOOTER_GAP: f32 = 8.0;
    let mut footer = column![].spacing(FOOTER_GAP);
    if let Some(name) = home.me_name {
        footer = footer.push(self_row(t, name, home.self_menu));
    }
    footer = footer.push(search_capsule(t, home.query));
    let footer = container(footer).width(Fill).padding(Padding {
        bottom: h::SEARCH_LIFT,
        left: 12.0,
        right: 12.0,
        top: 0.0,
    });
    let footer = container(footer)
        .width(Fill)
        .height(Fill)
        .align_y(Alignment::End);
    if home.self_menu && home.me_name.is_some() {
        // The menu floats 4px above the self row: it is its own layer, so
        // the row and the search capsule do not move when it opens. The
        // dismiss layer sits under the footer so the row still toggles it.
        let width = h::MENU_W.min((home.width - 16.0).max(0.0));
        let menu = container(self_menu_popover(t, width, home.scale))
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::End)
            .padding(Padding {
                left: 12.0,
                bottom: h::SEARCH_LIFT + h::SEARCH + FOOTER_GAP + h::CARD_ROW + h::MENU_GAP,
                ..Padding::ZERO
            });
        return iced::widget::stack![
            list,
            menu_dismiss_layer(Action::SelfMenuClose),
            footer,
            menu
        ]
        .width(Fill)
        .height(Fill)
        .into();
    }
    iced::widget::stack![list, footer]
        .width(Fill)
        .height(Fill)
        .into()
}

// ---------- Chat page ----------

/// Rename state for a bro DM's header; `None` in groups.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeaderRename<'a> {
    /// `Some(draft)` while editing.
    pub draft: Option<&'a str>,
    pub saving: bool,
    /// Last rename error.
    pub note: Option<&'a str>,
}

/// Widget id of the rename box, focused when editing starts.
pub const RENAME_INPUT: &str = "robroz-rename-bro";

/// Reserved height of the chat title bar, not counting a rename error line.
/// The feed is padded by this so the first card sits below the bar, and
/// the bar itself is an opaque `window` fill so scrolled messages cannot
/// draw through it.
pub fn chat_header_height(t: &Skin) -> f32 {
    t.controls
        .touch_target
        .max(t.controls.button_h)
        .max(robroz_tokens::home::FLOAT_BTN)
        + 12.0
}

/// Who is in the title capsule: up to three avatars, then `+N`.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeaderBros<'a> {
    pub avatars: &'a [crate::avatar::BroAvatar],
    pub more: usize,
    /// You own this DM's bro: View profile opens its avatar page.
    pub can_edit: bool,
}

/// Top of the chat page: a floating bar with the stacked bro avatars
/// (24px, 8px overlap, at most three then `+N`), the group or bro name
/// and `私聊` for a DM; on the right a round `…` that opens the group
/// menu. `back` adds 「‹」 and the unread count from other chats, for
/// narrow screens. There is no divider under the bar.
pub fn chat_header<'a>(
    t: &Skin,
    name: &str,
    dm: bool,
    back: bool,
    unread: u64,
    rename: Option<HeaderRename<'_>>,
    bros: HeaderBros<'_>,
) -> Element<'a, Action> {
    use robroz_tokens::base::avatar as tok;
    use robroz_tokens::home as h;
    let hbtn = h::FLOAT_BTN;
    let mut bar = row![].spacing(8).align_y(Alignment::Center);
    if back {
        // Chevron icon and unread count share one centre line, 4px apart.
        let mut back_label = row![crate::icons::icon(
            crate::icons::Glyph::Back,
            t.palette.text,
            h::MORE_ICON,
            2.0,
        )]
        .spacing(h::BACK_GAP)
        .align_y(Alignment::Center);
        if unread > 0 {
            let n = if unread > 99 {
                "99+".into()
            } else {
                unread.to_string()
            };
            back_label = back_label.push(
                text(n)
                    .size(12.0)
                    .color(color(t.palette.text2))
                    .align_y(Alignment::Center),
            );
        }
        bar = bar.push(
            button(
                container(back_label)
                    .height(Fill)
                    .align_y(Alignment::Center),
            )
            .height(Length::Fixed(hbtn))
            .padding(Padding::from([0.0, 12.0]))
            .style(float_button_style(*t))
            .on_press(Action::BackToList),
        );
    }
    let title = || {
        text(name.to_string())
            .size(t.type_.list_title)
            .font(bold())
            .wrapping(Wrapping::None)
            .color(color(t.palette.text))
    };
    match rename {
        Some(HeaderRename {
            draft: Some(draft),
            saving,
            ..
        }) => {
            let mut input = text_input(conversations::RENAME_PLACEHOLDER.as_str(), draft)
                .id(RENAME_INPUT)
                .size(t.type_.row_name)
                .padding(Padding::from([6.0, 8.0]));
            if !saving {
                input = input
                    .on_input(|text| Action::RenameEdit { text })
                    .on_submit(Action::RenameSave);
            }
            let changed = !draft.trim().is_empty() && draft.trim() != name.trim();
            bar = bar
                .push(container(input).width(Fill))
                .push(btn(
                    t,
                    Key::ViewRenameSave.into(),
                    Look::Primary,
                    Leading::None,
                    (!saving && changed).then_some(Action::RenameSave),
                ))
                .push(btn(
                    t,
                    Key::ViewRenameCancel.into(),
                    Look::Secondary,
                    Leading::None,
                    (!saving).then_some(Action::RenameCancel),
                ));
        }
        _ => {
            let mut inner = row![].spacing(8).align_y(Alignment::Center);
            if !bros.avatars.is_empty() {
                inner = inner.push(crate::avatar_view::avatar_stack(
                    t,
                    bros.avatars,
                    bros.more,
                    t.palette.surface,
                ));
            }
            inner = match rename {
                Some(_) => inner.push(
                    button(
                        row![
                            title(),
                            text(conversations::RENAME_HINT)
                                .size(t.type_.tag)
                                .color(color(t.palette.text3)),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding(Padding::from([4.0, 6.0]))
                    .style(nav_button_style(*t, false))
                    .on_press(Action::RenameStart),
                ),
                None => inner.push(title()),
            };
            if dm {
                inner = inner.push(tag(t, Key::ViewDmTag.as_str(), t.palette.text2));
            }
            let left = if bros.avatars.is_empty() {
                14.0
            } else {
                tok::TITLE_PAD
            };
            bar = bar.push(
                container(inner)
                    .height(Length::Fixed(tok::TITLE_HEIGHT))
                    .align_y(Alignment::Center)
                    .padding(Padding {
                        top: 0.0,
                        right: 14.0,
                        bottom: 0.0,
                        left,
                    })
                    .style(crate::avatar_view::capsule_style(*t)),
            );
            bar = bar.push(space::horizontal());
            bar = bar.push(crate::avatar_view::round_button(
                t,
                crate::icons::Glyph::More,
                Action::GroupMenuOpen,
            ));
        }
    }
    let mut col = column![container(bar)
        .width(Fill)
        .height(Length::Fixed(chat_header_height(t)))
        .align_y(Alignment::Center)]
    .spacing(0);
    if let Some(note) = rename.and_then(|r| r.note) {
        col = col.push(container(footnote(&t, note.to_string())).padding(Padding {
            bottom: 6.0,
            ..Padding::ZERO
        }));
    }
    let bg = t.palette.window;
    container(col)
        .width(Fill)
        .padding(Padding::from([0.0, t.metrics.stream_padding]))
        .style(move |_| container::Style {
            background: Some(color(bg).into()),
            border: Border::default(),
            ..container::Style::default()
        })
        .into()
}

/// Centred date line between runs that fall on different local days.
pub fn date_separator<'a, M: 'a>(t: &Skin, label: String) -> Element<'a, M> {
    let gap = t.chat.date_gap;
    container(
        text(label)
            .size(t.chat.date_size)
            .color(color(t.palette.text3)),
    )
    .width(Fill)
    .center_x(Fill)
    .padding(Padding::from([gap, 0.0]))
    .into()
}

/// Bottom of the chat page: a radius-20 card with the input and a 32px
/// send button at the bottom right. An attachment-thumbnail row is
/// reserved above the input inside the card, and is omitted while there
/// are no attachments (the feature is not wired yet). Enter sends.
/// `note` is the last send error, shown above the card. There is no +
/// or microphone button until those actions exist.
pub fn composer<'a>(
    t: &Skin,
    draft: &str,
    hint: &str,
    sending: bool,
    note: Option<&str>,
) -> Element<'a, Action> {
    let c = &t.chat;
    let can_send = !sending && !draft.trim().is_empty();
    let mut input = text_input(hint, draft)
        .size(t.type_.body)
        .padding(Padding {
            top: 0.0,
            right: 8.0,
            bottom: 0.0,
            left: c.composer_pad_left,
        })
        .style(composer_input_style(*t));
    if !sending {
        input = input
            .on_input(|text| Action::ComposerEdit { text })
            .on_submit(Action::ComposerSend);
    }
    let arrow = text("↑").size(16).color(faded(
        t.palette.on_accent,
        if can_send {
            1.0
        } else {
            c.composer_send_empty_opacity
        },
    ));
    let mut send = button(
        container(arrow)
            .width(Length::Fixed(c.composer_send))
            .height(Length::Fixed(c.composer_send))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
    )
    .padding(0)
    .width(Length::Fixed(c.composer_send))
    .height(Length::Fixed(c.composer_send))
    .style(composer_send_style(*t, can_send));
    if can_send {
        send = send.on_press(Action::ComposerSend);
    }
    let input_row = row![
        container(input)
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::Center),
        send
    ]
    .spacing(0)
    .align_y(Alignment::End)
    .height(Length::Fixed(c.composer_height - c.composer_send_inset))
    .padding(Padding {
        top: 0.0,
        right: c.composer_send_inset,
        bottom: c.composer_send_inset,
        left: 0.0,
    });
    // The 56px thumbnail row is reserved above this input row. It is
    // not drawn while there are no attachments.
    let mut card_body = column![];
    card_body = push_composer_thumbs(card_body, *t, &[]);
    card_body = card_body.push(input_row);
    let card = container(card_body)
        .width(Fill)
        .style(composer_card_style(*t));
    let mut col = column![].spacing(4);
    if let Some(n) = note {
        col = col.push(footnote(t, n.to_string()));
    }
    col = col.push(card);
    container(col)
        .width(Fill)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: c.composer_bottom,
            left: 0.0,
        })
        .into()
}

/// A reserved thumbnail for the composer attachment row. The row stays
/// hidden until something is placed here.
struct ComposerThumb {
    name: String,
}

fn push_composer_thumbs<'a>(
    col: iced::widget::Column<'a, Action>,
    t: Skin,
    thumbs: &'a [ComposerThumb],
) -> iced::widget::Column<'a, Action> {
    if thumbs.is_empty() {
        return col;
    }
    let size = t.chat.thumb_size;
    col.push(
        row(thumbs.iter().map(|thumb| {
            container(
                text(thumb.name.as_str())
                    .size(t.type_.tag)
                    .color(color(t.palette.text)),
            )
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .padding(4)
            .style(move |_: &IcedTheme| container::Style {
                background: Some(color(t.palette.sunken).into()),
                border: Border::default()
                    .width(t.stroke.card)
                    .color(color(t.palette.border))
                    .rounded(8.0),
                ..container::Style::default()
            })
            .into()
        }))
        .spacing(8)
        .padding(Padding {
            top: t.chat.composer_send_inset,
            right: t.chat.composer_send_inset,
            bottom: 0.0,
            left: t.chat.composer_pad_left,
        }),
    )
}

fn composer_input_style(t: Skin) -> impl Fn(&IcedTheme, text_input::Status) -> text_input::Style {
    move |_, _| {
        let sel = t.palette.selection;
        text_input::Style {
            background: Color::TRANSPARENT.into(),
            border: Border::default(),
            icon: color(t.palette.text),
            placeholder: color(t.palette.text3),
            value: color(t.palette.text),
            selection: faded(sel.rgb, sel.alpha),
        }
    }
}

fn composer_card_style(t: Skin) -> impl Fn(&IcedTheme) -> container::Style {
    move |_| container::Style {
        background: Some(color(t.palette.surface).into()),
        border: Border::default()
            .width(t.stroke.card)
            .color(color(t.palette.border))
            .rounded(t.chat.composer_radius),
        ..container::Style::default()
    }
}

fn composer_send_style(
    t: Skin,
    can_send: bool,
) -> impl Fn(&IcedTheme, button::Status) -> button::Style {
    let alpha = if can_send {
        1.0
    } else {
        t.chat.composer_send_empty_opacity
    };
    let fill = faded(t.palette.accent_strong, alpha);
    let r = t.chat.composer_send / 2.0;
    move |_, _| button::Style {
        background: Some(fill.into()),
        text_color: faded(t.palette.on_accent, alpha),
        border: Border::default().rounded(r),
        shadow: IcedShadow::default(),
        snap: false,
    }
}

// ---------- 联网搜索 page ----------

/// The 联网搜索 page: provider, key or address, one test after each save.
pub fn search_panel<'a>(t: &Skin, page: &'a search::Page, titled: bool) -> Element<'a, Action> {
    use search::Provider;
    let m = t.metrics;
    let busy = page.busy;
    let mut col = column![];
    if titled {
        let mut head = row![text(search::TITLE)
            .size(t.type_.list_title)
            .font(bold())
            .color(color(t.palette.text))]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center);
        if page.from_env() {
            head = head.push(
                container(
                    text(search::FROM_ENV)
                        .size(t.type_.tag)
                        .color(color(t.palette.text2)),
                )
                .padding(Padding::from([2.0, m.tag_pad_h]))
                .style(card_style(*t)),
            );
        }
        col = column![
            container(head)
                .height(Length::Fixed(m.panel_head_height))
                .padding(Padding::from([0.0, m.row_pad_h]))
                .align_y(Alignment::Center),
            divider(t, 0.0),
        ];
    } else if page.from_env() {
        col = col.push(
            container(
                text(search::FROM_ENV)
                    .size(t.type_.tag)
                    .color(color(t.palette.text2)),
            )
            .padding(Padding::from([2.0, m.tag_pad_h])),
        );
    }
    let pad = Padding::from([m.row_pad_h * 0.75, m.row_pad_h]);
    let mut body = column![].spacing(10);
    if page.current.as_ref().is_some_and(|c| c.provider.is_none()) && page.provider.is_none() {
        body = body.push(text(search::NOT_SET).color(color(t.palette.text2)));
    }
    let mut picks = row![].spacing(t.controls.button_gap);
    for p in [Provider::Brave, Provider::Searxng] {
        let look = if page.provider == Some(p) {
            Look::Selected
        } else {
            Look::Secondary
        };
        picks = picks.push(btn(
            t,
            p.label().into(),
            look,
            Leading::None,
            (!busy).then_some(Action::SearchPick { provider: p }),
        ));
    }
    body = body.push(picks);
    match page.provider {
        Some(Provider::Brave) => {
            body = body.push(masked_key_row(
                t,
                &page.key,
                search::KEY_PLACEHOLDER.as_str(),
                busy,
                |text| Action::SearchKey { text },
                Action::SearchClearKey,
                Action::SearchCancelKey,
                Some(Action::SearchSave),
            ));
        }
        Some(Provider::Searxng) => {
            let mut input = text_input(search::URL_PLACEHOLDER.as_str(), &page.url)
                .size(t.type_.body)
                .padding(field_padding())
                .style(field_input_style(*t));
            if !busy {
                input = input
                    .on_input(|text| Action::SearchUrl { text })
                    .on_submit(Action::SearchSave);
            }
            body = body.push(input);
        }
        None => {}
    }
    let mut foot = row![btn(
        t,
        search::SAVE.into(),
        Look::Primary,
        Leading::None,
        page.can_save().then_some(Action::SearchSave),
    )]
    .spacing(t.controls.button_gap)
    .align_y(Alignment::Center);
    if let Some(line) = page.test_line() {
        let c = match page.test {
            Some(search::Test::Failed(_)) => t.palette.accent,
            _ => t.palette.text2,
        };
        foot = foot.push(text(line).size(t.type_.foot).color(color(c)));
    }
    body = body.push(foot);
    if let Some(note) = &page.note {
        body = body.push(inline_note(t, note));
    }
    col = col.push(container(body).padding(pad));
    container(col).width(Fill).style(card_style(*t)).into()
}

// ---------- 模型 page ----------

/// Shared fill for settings fields: the same white (or dark-theme base) as
/// iced's default text input, not the beige card `surface`.
fn field_fill(theme: &IcedTheme) -> Color {
    theme.extended_palette().background.base.color
}

fn field_padding() -> Padding {
    Padding::from([8.0, 10.0])
}

fn field_border(t: &Skin, c: Rgb) -> Border {
    Border {
        color: color(c),
        width: t.stroke.control,
        radius: 2.0.into(),
    }
}

fn field_pick_style(t: Skin) -> impl Fn(&IcedTheme, pick_list::Status) -> pick_list::Style {
    move |theme, status| {
        let border = match status {
            pick_list::Status::Hovered | pick_list::Status::Opened { .. } => t.palette.text,
            pick_list::Status::Active => t.palette.border,
        };
        pick_list::Style {
            text_color: color(t.palette.text),
            placeholder_color: color(t.palette.text3),
            handle_color: color(t.palette.text),
            background: field_fill(theme).into(),
            border: field_border(&t, border),
        }
    }
}

fn field_pick_menu_style(t: Skin) -> impl Fn(&IcedTheme) -> iced::widget::overlay::menu::Style {
    move |theme| iced::widget::overlay::menu::Style {
        background: field_fill(theme).into(),
        border: field_border(&t, t.palette.border),
        text_color: color(t.palette.text),
        selected_text_color: color(t.palette.text),
        selected_background: color(t.palette.control_hover).into(),
        shadow: IcedShadow::default(),
    }
}

fn field_input_style(t: Skin) -> impl Fn(&IcedTheme, text_input::Status) -> text_input::Style {
    move |theme, status| {
        let border = match status {
            text_input::Status::Focused { .. } => t.palette.focus,
            text_input::Status::Hovered => t.palette.text,
            text_input::Status::Active | text_input::Status::Disabled => t.palette.border,
        };
        let sel = t.palette.selection;
        text_input::Style {
            background: field_fill(theme).into(),
            border: field_border(&t, border),
            icon: color(t.palette.text),
            placeholder: color(t.palette.text3),
            value: color(t.palette.text),
            selection: faded(sel.rgb, sel.alpha),
        }
    }
}

/// Settings-page rule input: white fill even when disabled; text and
/// placeholder drop to 40% opacity. The 「添加」 button keeps its usual
/// disabled look.
fn review_rule_input_style(
    t: Skin,
) -> impl Fn(&IcedTheme, text_input::Status) -> text_input::Style {
    move |theme, status| {
        let disabled = matches!(status, text_input::Status::Disabled);
        let border = match status {
            text_input::Status::Focused { .. } => t.palette.focus,
            text_input::Status::Hovered => t.palette.text,
            text_input::Status::Active | text_input::Status::Disabled => t.palette.border,
        };
        let alpha = if disabled { 0.4 } else { 1.0 };
        let sel = t.palette.selection;
        text_input::Style {
            background: field_fill(theme).into(),
            border: field_border(&t, border),
            icon: faded(t.palette.text, alpha),
            placeholder: faded(t.palette.text3, alpha),
            value: faded(t.palette.text, alpha),
            selection: faded(sel.rgb, sel.alpha),
        }
    }
}

/// The 模型 page: preset, address, model name, masked key, one test after each save.
pub fn model_panel<'a>(t: &Skin, page: &'a model::Page, titled: bool) -> Element<'a, Action> {
    let m = t.metrics;
    let busy = page.busy;
    let mut col = column![];
    if titled {
        let mut head = row![text(model::TITLE)
            .size(t.type_.list_title)
            .font(bold())
            .color(color(t.palette.text))]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center);
        if page.from_env() {
            head = head.push(
                container(
                    text(model::FROM_ENV)
                        .size(t.type_.tag)
                        .color(color(t.palette.text2)),
                )
                .padding(Padding::from([2.0, m.tag_pad_h]))
                .style(card_style(*t)),
            );
        }
        col = column![
            container(head)
                .height(Length::Fixed(m.panel_head_height))
                .padding(Padding::from([0.0, m.row_pad_h]))
                .align_y(Alignment::Center),
            divider(t, 0.0),
        ];
    } else if page.from_env() {
        col = col.push(
            container(
                text(model::FROM_ENV)
                    .size(t.type_.tag)
                    .color(color(t.palette.text2)),
            )
            .padding(Padding::from([2.0, m.tag_pad_h])),
        );
    }
    let pad = Padding::from([m.row_pad_h * 0.75, m.row_pad_h]);
    let mut body = column![].spacing(10);
    if page.not_set() {
        body = body.push(text(model::NOT_SET).color(color(t.palette.text2)));
    }
    let preset = pick_list(model::Preset::ALL, page.preset, |preset| {
        Action::ModelPick { preset }
    })
    .placeholder(model::PRESET_PLACEHOLDER)
    .text_size(t.type_.body)
    .padding(field_padding())
    .width(Fill)
    .handle(pick_list::Handle::Arrow { size: None })
    .style(field_pick_style(*t))
    .menu_style(field_pick_menu_style(*t));
    body = body.push(labelled(t, model::LABEL_PRESET.as_str(), preset));

    let mut url = text_input(model::URL_PLACEHOLDER, &page.url)
        .size(t.type_.body)
        .padding(field_padding())
        .style(field_input_style(*t));
    if !busy {
        url = url.on_input(|text| Action::ModelUrl { text });
    }
    body = body.push(labelled(t, model::LABEL_URL.as_str(), url));

    let mut name = text_input(page.model_placeholder(), &page.model)
        .size(t.type_.body)
        .padding(field_padding())
        .style(field_input_style(*t));
    if !busy {
        name = name
            .on_input(|text| Action::ModelName { text })
            .on_submit(Action::ModelSave);
    }
    let mut name_block = column![labelled(t, model::LABEL_MODEL.as_str(), name)];
    if page.preset == Some(model::Preset::OpenRouter) {
        name_block = name_block.push(
            text(model::OPENROUTER_HINT)
                .size(t.type_.foot)
                .color(color(t.palette.text3)),
        );
    }
    body = body.push(name_block.spacing(2.0));

    if page.shows_key() {
        body = body.push(labelled(
            t,
            model::LABEL_KEY.as_str(),
            masked_key_row(
                t,
                &page.key,
                model::KEY_PLACEHOLDER,
                busy,
                |text| Action::ModelKey { text },
                Action::ModelClearKey,
                Action::ModelCancelKey,
                Some(Action::ModelSave),
            ),
        ));
    }

    let mut foot = row![btn(
        t,
        model::SAVE.into(),
        Look::Primary,
        Leading::None,
        page.can_save().then_some(Action::ModelSave),
    )]
    .spacing(t.controls.button_gap)
    .align_y(Alignment::Center);
    match &page.test {
        Some(model::Test::Ok) => {
            foot = foot.push(
                row![
                    dot(t, Dot::Done),
                    text(model::WORKS)
                        .size(t.type_.foot)
                        .color(color(t.palette.text)),
                ]
                .spacing(6.0)
                .align_y(Alignment::Center),
            );
        }
        Some(model::Test::Failed(_)) => {
            if let Some(line) = page.test_line() {
                foot = foot.push(text(line).size(t.type_.foot).color(color(t.palette.accent)));
            }
        }
        Some(model::Test::Running) => {
            foot = foot.push(
                text(model::TESTING)
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
            );
        }
        None => {}
    }
    body = body.push(foot);
    if let Some(note) = &page.note {
        body = body.push(inline_note(t, note));
    }
    col = col.push(container(body).padding(pad));
    container(col).width(Fill).style(card_style(*t)).into()
}
/// #68 confirm layout shared by the owner-DM card and the in-page card.
fn rule_confirm_fields<'a>(
    t: &Skin,
    kind: crate::cards::ActionKind,
    summary: &'a str,
    id: &'a str,
    state: &'a crate::cards::ApprovalState,
    show_buttons: bool,
    enabled: bool,
    note: Option<String>,
    mut col: iced::widget::Column<'a, Action>,
) -> iced::widget::Column<'a, Action> {
    let hint = if kind == crate::cards::ActionKind::RuleDelete {
        review_rules::DELETE_HINT
    } else {
        review_rules::ADD_HINT
    };
    if matches!(state, crate::cards::ApprovalState::Expired { .. }) {
        col = col.push(
            text(review_rules::EXPIRED)
                .size(t.type_.foot)
                .color(color(t.palette.text2)),
        );
        col = col.push(gap(t.metrics.head_gap));
    }
    col = col
        .push(body(t, summary))
        .push(gap(t.metrics.head_gap))
        .push(text(hint).size(t.type_.foot).color(color(t.palette.text2)));
    if show_buttons {
        let (ok, look) = if kind == crate::cards::ActionKind::RuleDelete {
            (review_rules::DELETE_ACTION, Look::Danger)
        } else {
            (review_rules::ADD_ACTION, Look::Primary)
        };
        col = col.push(gap(t.metrics.body_gap)).push(buttons_row(
            t,
            vec![
                btn(
                    t,
                    ok.into(),
                    look,
                    Leading::None,
                    enabled.then(|| Action::Decide {
                        card: id.to_string(),
                        approve: true,
                    }),
                ),
                btn(
                    t,
                    review_rules::DROP.into(),
                    if enabled {
                        Look::Secondary
                    } else {
                        Look::Disabled
                    },
                    Leading::None,
                    enabled.then(|| Action::Decide {
                        card: id.to_string(),
                        approve: false,
                    }),
                ),
            ],
        ));
    }
    if let Some(note) = note {
        col = col.push(gap(t.metrics.foot_gap)).push(footnote(t, note));
    } else if let Some(foot) = status::approval_footnote(state, &Local) {
        col = col
            .push(gap(t.metrics.foot_gap))
            .push(approval_footnote_view(t, foot));
    }
    col
}

/// In-page confirm card on the 放行规则 settings page.
fn rule_confirm_on_page<'a>(
    t: &Skin,
    pending: &'a review_rules::Pending,
    enabled: bool,
    can_decide: bool,
) -> Element<'a, Action> {
    let kind = if pending.is_add() {
        crate::cards::ActionKind::RuleAdd
    } else {
        crate::cards::ActionKind::RuleDelete
    };
    // Only an admin may answer; everyone else gets the status line and
    // no buttons at all (not greyed out).
    let show_buttons = pending.is_open() && can_decide;
    let mut head = column![];
    if pending.is_open() && !can_decide {
        head = head
            .push(
                text(review_rules::WAITING_FOR_ADMIN)
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
            )
            .push(gap(t.metrics.head_gap));
    }
    let col = rule_confirm_fields(
        t,
        kind,
        &pending.text,
        &pending.id,
        &pending.state,
        show_buttons,
        enabled,
        None,
        head,
    );
    container(col)
        .padding(card_padding(t))
        .width(Fill)
        .style(card_style(*t))
        .into()
}

/// Owner-DM card for adding or deleting a rule: #68 confirm layout.
fn rule_approval<'a>(
    t: &Skin,
    card: &'a ApprovalCard,
    ui: &CardUi,
    col: iced::widget::Column<'a, Action>,
) -> Element<'a, Action> {
    let actions = interact::approval_actions(card, ui);
    let show_buttons = matches!(actions, ApprovalActions::Decide { .. });
    let enabled = matches!(actions, ApprovalActions::Decide { enabled: true });
    let note = ui.note.as_ref().map(|n| n.text());
    frame(
        t,
        rule_confirm_fields(
            t,
            card.kind,
            &card.summary,
            &card.id,
            &card.state,
            show_buttons,
            enabled,
            note,
            col,
        )
        .into(),
    )
}

/// The 放行规则 page: add an input, list confirmed rules, confirm in place.
/// `can_decide`: [`review_rules::may_decide`] for the signed-in user.
pub fn review_rules_panel<'a>(
    t: &Skin,
    page: &'a review_rules::Page,
    can_decide: bool,
) -> Element<'a, Action> {
    let locked = page.locked();
    let mut body_col = column![].spacing(12).width(Fill);
    let mut input = text_input(review_rules::PLACEHOLDER.as_str(), &page.draft)
        .size(t.type_.body)
        .padding(field_padding())
        .width(Fill)
        .style(review_rule_input_style(*t));
    if !locked {
        input = input
            .on_input(|text| Action::ReviewDraft { text })
            .on_submit(Action::ReviewAdd);
    }
    body_col = body_col.push(
        row![
            input,
            btn(
                t,
                review_rules::ADD.into(),
                Look::Primary,
                Leading::None,
                (!locked).then_some(Action::ReviewAdd),
            ),
        ]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center),
    );
    if let Some(note) = &page.note {
        if *note == review_rules::ALREADY_ANSWERED {
            body_col = body_col.push(
                text(note.as_str())
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
            );
        } else {
            body_col = body_col.push(inline_note(t, note));
        }
    }
    let add_pending = page.pending.as_ref().filter(|p| p.is_add());
    if let Some(pending) = add_pending {
        body_col = body_col.push(rule_confirm_on_page(t, pending, !page.busy, can_decide));
    }
    match &page.rules {
        None => {
            body_col = body_col.push(
                text(review_rules::LOADING)
                    .size(t.type_.body)
                    .color(color(t.palette.text2)),
            );
        }
        Some(rules) if rules.is_empty() && add_pending.is_none() => {
            body_col = body_col.push(
                text(review_rules::EMPTY)
                    .size(t.type_.body)
                    .color(color(t.palette.text2)),
            );
        }
        Some(rules) => {
            for r in rules {
                if let Some(pending) = page.pending.as_ref().filter(|p| p.is_delete_of(&r.id)) {
                    body_col =
                        body_col.push(rule_confirm_on_page(t, pending, !page.busy, can_decide));
                    continue;
                }
                let date = r.added_on(&Local);
                let mut row_el = column![text(r.text.as_str())
                    .size(t.type_.body)
                    .color(color(t.palette.text)),]
                .spacing(4);
                if !date.is_empty() {
                    row_el =
                        row_el.push(text(date).size(t.type_.foot).color(color(t.palette.text2)));
                }
                let id = r.id.clone();
                body_col = body_col.push(
                    row![
                        container(row_el).width(Fill),
                        btn(
                            t,
                            review_rules::DELETE.into(),
                            Look::Secondary,
                            Leading::None,
                            (!locked).then_some(Action::ReviewDelete { id }),
                        ),
                    ]
                    .spacing(t.controls.button_gap)
                    .align_y(Alignment::Center),
                );
            }
        }
    }
    body_col.into()
}

/// Data for the group profile page.
pub struct GroupProfile<'a> {
    pub name: &'a str,
    pub avatars: &'a [crate::avatar::BroAvatar],
    pub more: usize,
    pub can_manage: bool,
    pub routines_on: Option<u64>,
    pub members: Option<u64>,
    pub edit: Option<&'a str>,
    pub saving: bool,
    pub note: Option<&'a str>,
    pub copied: bool,
    pub menu_open: bool,
    pub scale: f32,
}

fn profile_card_row<'a>(
    t: &Skin,
    glyph: crate::icons::Glyph,
    title: String,
    value: String,
    scale: f32,
    on: Option<Action>,
) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let body = container(
        row![
            crate::icons::icon(glyph, t.palette.text, h::CARD_ICON, scale),
            text(title)
                .size(t.type_.body)
                .color(color(t.palette.text))
                .width(Fill),
            text(value).size(t.type_.body).color(color(t.palette.text2)),
            text("›").size(18.0).color(color(t.palette.text3)),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .height(Fill)
    .align_y(Alignment::Center);
    let mut b = button(body)
        .width(Fill)
        .height(Length::Fixed(h::CARD_ROW))
        .padding(Padding::from([0.0, 16.0]))
        .style(conversation_row_style(*t, false));
    if let Some(on) = on {
        b = b.on_press(on);
    }
    b.into()
}

/// Top of a popover that belongs to a round top-bar button: 4px under it.
fn below_round_button(bar_top: f32, bar_h: f32) -> f32 {
    use robroz_tokens::base::avatar as tok;
    bar_top + (bar_h + tok::MORE_BUTTON) / 2.0 + robroz_tokens::home::MENU_GAP
}

fn profile_more_menu<'a>(t: &Skin, can_manage: bool, scale: f32) -> Element<'a, Action> {
    let t = *t;
    let mut rows = vec![menu_row(
        &t,
        crate::icons::Glyph::Copy,
        Key::GroupProfileCopyId.into(),
        false,
        scale,
        Action::GroupCopyId,
    )];
    if can_manage {
        rows.push(menu_divider(&t));
        rows.push(menu_row(
            &t,
            crate::icons::Glyph::Delete,
            Key::GroupMenuDelete.into(),
            true,
            scale,
            Action::GroupDeleteAsk,
        ));
    }
    container(popover_box(&t, robroz_tokens::home::MENU_W, rows))
        .width(Fill)
        .padding(Padding {
            top: below_round_button(PROFILE_BAR_PAD, robroz_tokens::base::avatar::MORE_BUTTON),
            right: PROFILE_BAR_PAD.max(t.metrics.stream_padding),
            ..Padding::ZERO
        })
        .align_x(Alignment::End)
        .into()
}

/// Vertical padding of the profile page's top bar.
const PROFILE_BAR_PAD: f32 = 8.0;

/// Group profile: 96px stacked avatars, name, Edit, Routines / Members
/// card, and a top-right `•••` with Copy ID and Delete group.
pub fn group_profile_page<'a>(t: &Skin, p: GroupProfile<'a>) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let t = *t;
    let back =
        crate::avatar_view::round_button(&t, crate::icons::Glyph::Back, Action::GroupProfileClose);
    let more =
        crate::avatar_view::round_button(&t, crate::icons::Glyph::More, Action::GroupMenuOpen);
    let bar = container(row![back, space::horizontal(), more].align_y(Alignment::Center))
        .width(Fill)
        .padding(Padding::from([PROFILE_BAR_PAD, t.metrics.stream_padding]));
    let face = if p.avatars.is_empty() {
        avatar_at(&t, p.name, h::PROFILE_AVATAR)
    } else {
        crate::avatar_view::avatar_stack_at(
            &t,
            p.avatars,
            p.more,
            t.palette.window,
            h::PROFILE_AVATAR,
            h::PROFILE_AVATAR * 8.0 / 24.0,
        )
    };
    let mut col = column![bar]
        .spacing(0)
        .align_x(Alignment::Center)
        .width(Fill);
    col = col.push(container(face).padding(Padding {
        top: 12.0,
        ..Padding::ZERO
    }));
    col = col.push(
        container(
            text(p.name.to_string())
                .size(h::PROFILE_NAME)
                .font(bold())
                .color(color(t.palette.text)),
        )
        .padding(Padding {
            top: h::PROFILE_NAME_GAP,
            ..Padding::ZERO
        }),
    );
    if let Some(draft) = p.edit {
        let mut input = text_input(conversations::RENAME_PLACEHOLDER.as_str(), draft)
            .size(t.type_.body)
            .padding(Padding::from([6.0, 12.0]));
        if !p.saving {
            input = input
                .on_input(|text| Action::GroupEdit { text })
                .on_submit(Action::GroupEditSave);
        }
        let changed = !draft.trim().is_empty() && draft.trim() != p.name.trim();
        col = col.push(
            container(
                row![
                    container(input).width(Fill),
                    btn(
                        &t,
                        Key::ViewRenameSave.into(),
                        Look::Primary,
                        Leading::None,
                        (!p.saving && changed).then_some(Action::GroupEditSave),
                    ),
                    btn(
                        &t,
                        Key::ViewRenameCancel.into(),
                        Look::Secondary,
                        Leading::None,
                        (!p.saving).then_some(Action::GroupEditCancel),
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .width(Length::Fixed(360.0))
            .padding(Padding {
                top: h::PROFILE_EDIT_GAP,
                ..Padding::ZERO
            }),
        );
    } else if p.can_manage {
        col = col.push(
            container(
                button(
                    text(Key::GroupProfileEdit)
                        .size(t.type_.foot)
                        .height(Fill)
                        .align_y(Alignment::Center)
                        .color(color(t.palette.text)),
                )
                .height(Length::Fixed(h::PROFILE_EDIT_H))
                .padding(Padding::from([0.0, 14.0]))
                .style(float_button_style(t))
                .on_press(Action::GroupEditStart),
            )
            .padding(Padding {
                top: h::PROFILE_EDIT_GAP,
                ..Padding::ZERO
            }),
        );
    }
    if let Some(note) = p.note {
        col = col.push(container(footnote(&t, note.to_string())).padding(Padding {
            top: 8.0,
            ..Padding::ZERO
        }));
    }
    if p.copied {
        col = col.push(
            container(
                text(Key::GroupProfileCopied)
                    .size(t.type_.foot)
                    .color(color(t.palette.text2)),
            )
            .padding(8.0),
        );
    }
    let mut card = column![];
    if let Some(n) = p.routines_on {
        card = card.push(profile_card_row(
            &t,
            crate::icons::Glyph::Routines,
            Key::RoutinesTitle.into(),
            Key::GroupProfileRoutinesOn.fill(&[&n]),
            p.scale,
            Some(Action::OpenRoutines),
        ));
    }
    if p.can_manage {
        if let Some(n) = p.members {
            card = card.push(profile_card_row(
                &t,
                crate::icons::Glyph::Members,
                Key::GroupProfileMembers.into(),
                n.to_string(),
                p.scale,
                Some(Action::MembersOpen),
            ));
        }
    }
    // The card is at most 720px (the chat column) and centred; narrower
    // windows keep a 16px margin each side. The 24px under Edit is outside
    // the card, so the card hugs its rows.
    let card = container(card)
        .width(Fill)
        .max_width(h::PROFILE_CARD_MAX)
        .style(move |_| container::Style {
            background: Some(color(t.palette.surface).into()),
            border: Border {
                color: color(t.palette.border2),
                width: 1.0,
                radius: h::CARD_R.into(),
            },
            ..container::Style::default()
        });
    col = col.push(container(card).width(Fill).center_x(Fill).padding(Padding {
        top: h::PROFILE_CARD_GAP,
        left: h::PROFILE_SIDE,
        right: h::PROFILE_SIDE,
        bottom: h::PROFILE_SIDE,
    }));
    let page = scrollable(col).height(Fill).width(Fill);
    if p.menu_open {
        iced::widget::stack![
            page,
            menu_dismiss_layer(Action::GroupMenuClose),
            profile_more_menu(&t, p.can_manage, p.scale)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    } else {
        page.into()
    }
}

fn menu_dismiss_layer<'a>(on: Action) -> Element<'a, Action> {
    mouse_area(container(Space::new()).width(Fill).height(Fill))
        .on_press(on)
        .into()
}

/// Click-outside layer for the chat `•••` menu; no scrim. The popover is
/// right-aligned to the button and sits 4px below it.
pub fn group_menu_overlay<'a>(
    t: &Skin,
    can_manage: bool,
    scale: f32,
    pad: f32,
) -> Element<'a, Action> {
    let bar_h = chat_header_height(t);
    let menu = container(group_menu_popover(t, can_manage, scale))
        .width(Fill)
        .height(Fill)
        .align_x(Alignment::End)
        .align_y(Alignment::Start)
        .padding(Padding {
            top: below_round_button(0.0, bar_h),
            right: pad,
            ..Padding::ZERO
        });
    iced::widget::stack![menu_dismiss_layer(Action::GroupMenuClose), menu]
        .width(Fill)
        .height(Fill)
        .into()
}

/// Desktop-centred delete confirmation, 400px wide. Cancel is on the
/// left; Delete is red and not the default focus.
pub fn delete_group_dialog<'a>(t: &Skin, name: &str) -> Element<'a, Action> {
    use robroz_tokens::home as h;
    let t = *t;
    let body = column![
        text(Key::GroupDeleteTitle.fill(&[&name]))
            .size(t.type_.list_title)
            .font(bold())
            .color(color(t.palette.text)),
        text(Key::GroupDeleteBody)
            .size(t.type_.body)
            .color(color(t.palette.text2)),
        row![
            container(btn(
                &t,
                Key::ViewRenameCancel.into(),
                Look::Secondary,
                Leading::None,
                Some(Action::GroupDeleteCancel),
            ))
            .width(Fill),
            container(btn(
                &t,
                Key::GroupDeleteConfirm.into(),
                Look::Danger,
                Leading::None,
                Some(Action::GroupDeleteConfirm),
            ))
            .width(Fill),
        ]
        .spacing(12),
    ]
    .spacing(16)
    .width(Length::Fixed(h::DIALOG_W));
    let card = container(body)
        .padding(24.0)
        .style(move |_| container::Style {
            background: Some(color(t.palette.surface).into()),
            border: Border {
                color: color(t.palette.border2),
                width: 1.0,
                radius: 16.0.into(),
            },
            shadow: IcedShadow {
                color: Color {
                    a: 0.20,
                    ..color(t.palette.text)
                },
                offset: Vector::new(0.0, 8.0),
                blur_radius: 24.0,
            },
            ..container::Style::default()
        });
    let dim = Color {
        a: 0.32,
        ..color(t.palette.text)
    };
    iced::widget::stack![
        mouse_area(
            container(Space::new())
                .width(Fill)
                .height(Fill)
                .style(move |_| container::Style {
                    background: Some(Background::Color(dim)),
                    ..container::Style::default()
                })
        )
        .on_press(Action::GroupDeleteCancel),
        container(card).width(Fill).height(Fill).center(Fill)
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

pub fn find_in_chat_page<'a>(
    t: &Skin,
    query: &'a str,
    hits: &'a [robroz_contract::GroupSearchHit],
    busy: bool,
    now: i64,
) -> Element<'a, Action> {
    let back =
        crate::avatar_view::round_button(t, crate::icons::Glyph::Back, Action::FindInChatClose);
    let title = text(Key::FindInChatTitle)
        .size(t.type_.list_title)
        .font(bold())
        .color(color(t.palette.text));
    let bar = container(row![back, title].spacing(12).align_y(Alignment::Center))
        .width(Fill)
        .padding(Padding::from([8.0, t.metrics.stream_padding]));
    let input = capsule_search(
        t,
        Key::FindInChatPlaceholder.as_str(),
        query,
        (!busy).then_some(|text| Action::FindInChatEdit { text }),
        Some(Action::FindInChatSubmit),
    );
    let mut col = column![
        bar,
        container(input).padding(Padding::from([0.0, t.metrics.stream_padding]))
    ]
    .spacing(12)
    .width(Fill);
    if query.trim().is_empty() {
        col = col.push(
            container(
                text(Key::FindInChatEmpty)
                    .size(t.type_.body)
                    .color(color(t.palette.text2)),
            )
            .padding(t.metrics.stream_padding),
        );
    } else if !busy && hits.is_empty() {
        col = col.push(
            container(
                text(Key::FindInChatNoResults)
                    .size(t.type_.body)
                    .color(color(t.palette.text2)),
            )
            .padding(t.metrics.stream_padding),
        );
    } else {
        for h in hits.iter() {
            let when = conversations::list_time(&h.created_at, now, &Local);
            let who = h.author_name.as_deref().unwrap_or("");
            let head = row![
                text(who.to_string())
                    .size(t.type_.foot)
                    .font(bold())
                    .color(color(t.palette.text)),
                space::horizontal(),
                text(when).size(t.type_.foot).color(color(t.palette.text3)),
            ]
            .align_y(Alignment::Center);
            col = col.push(
                container(
                    column![
                        head,
                        text(h.snippet.clone())
                            .size(t.type_.body)
                            .color(color(t.palette.text2)),
                    ]
                    .spacing(4),
                )
                .width(Fill)
                .padding(Padding::from([10.0, t.metrics.stream_padding])),
            );
        }
    }
    scrollable(col).height(Fill).width(Fill).into()
}

pub fn members_page<'a, F>(
    t: &Skin,
    members: Option<&'a robroz_contract::GroupMembers>,
    mut avatar_of: F,
    scale: f32,
) -> Element<'a, Action>
where
    F: FnMut(&str) -> crate::avatar::BroAvatar,
{
    let _ = scale;
    let back = crate::avatar_view::round_button(t, crate::icons::Glyph::Back, Action::MembersClose);
    let title = text(Key::MembersTitle)
        .size(t.type_.list_title)
        .font(bold())
        .color(color(t.palette.text));
    let mut col = column![
        container(row![back, title].spacing(12).align_y(Alignment::Center))
            .padding(Padding::from([8.0, t.metrics.stream_padding]))
    ]
    .spacing(0)
    .width(Fill);
    let Some(members) = members else {
        return scrollable(col).height(Fill).width(Fill).into();
    };
    for b in &members.bros {
        let face = crate::avatar_view::bro_avatar(t, avatar_of(&b.bot_id), 40.0);
        let mut body =
            column![text(b.name.clone()).size(16.0).color(color(t.palette.text))].spacing(2);
        if let Some(d) = b.owner_name.as_deref() {
            body = body.push(text(d.to_string()).size(12.0).color(color(t.palette.text3)));
        }
        col = col.push(
            container(row![face, body].spacing(12).align_y(Alignment::Center))
                .width(Fill)
                .padding(Padding::from([10.0, t.metrics.stream_padding])),
        );
    }
    for u in &members.users {
        let name = u.name.clone().unwrap_or_else(|| u.user_id.clone());
        let face = avatar_at(t, &name, 40.0);
        col = col.push(
            container(
                row![face, text(name).size(16.0).color(color(t.palette.text)),]
                    .spacing(12)
                    .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding(Padding::from([10.0, t.metrics.stream_padding])),
        );
    }
    scrollable(col).height(Fill).width(Fill).into()
}

/// One Settings row as drawn: label, optional `text2` value, then `›`.
pub struct SettingsRowView {
    pub label: String,
    pub value: Option<String>,
    pub on: Action,
}

/// A titled group of rows on one card.
pub struct SettingsGroupView {
    pub title: String,
    pub rows: Vec<SettingsRowView>,
}

/// Inputs for [`settings_page`].
pub struct SettingsPage {
    pub title: String,
    /// Round ‹ in the top-left; `None` on the desktop landing.
    pub back: Option<Action>,
    pub groups: Vec<SettingsGroupView>,
    /// Red row on its own card under the groups.
    pub sign_out: Option<(String, Action)>,
    /// 12px `text3` line, centred at the bottom.
    pub version: Option<String>,
    pub scale: f32,
}

fn settings_card<'a>(t: Skin, rows: Vec<Element<'a, Action>>) -> Element<'a, Action> {
    use robroz_tokens::settings as st;
    let mut col = column![];
    for (i, r) in rows.into_iter().enumerate() {
        if i > 0 {
            col = col.push(
                container(Space::new().height(Length::Fixed(st::DIVIDER)))
                    .width(Fill)
                    .style(move |_| container::Style {
                        background: Some(color(t.palette.border).into()),
                        ..container::Style::default()
                    }),
            );
        }
        col = col.push(r);
    }
    container(col)
        .width(Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(color(t.palette.surface).into()),
            border: Border {
                color: color(t.palette.border2),
                width: 1.0,
                radius: st::CARD_R.into(),
            },
            ..container::Style::default()
        })
        .into()
}

fn settings_row<'a>(
    t: Skin,
    label: String,
    label_rgb: crate::theme::Rgb,
    trailing: Option<Element<'a, Action>>,
    on: Action,
) -> Element<'a, Action> {
    use robroz_tokens::settings as st;
    let mut r = row![text(label)
        .size(t.type_.body)
        .color(color(label_rgb))
        .wrapping(Wrapping::None)
        .width(Fill)]
    .spacing(8)
    .align_y(Alignment::Center);
    if let Some(tr) = trailing {
        r = r.push(tr);
    }
    button(container(r).height(Fill).align_y(Alignment::Center))
        .width(Fill)
        .height(Length::Fixed(st::ROW))
        .padding(Padding::from([0.0, st::SIDE_PAD]))
        .style(conversation_row_style(t, false))
        .on_press(on)
        .into()
}

fn settings_trailing<'a>(t: Skin, value: Option<String>) -> Element<'a, Action> {
    let mut r = row![].spacing(8).align_y(Alignment::Center);
    if let Some(v) = value {
        r = r.push(
            text(v)
                .size(t.type_.body)
                .color(color(t.palette.text2))
                .wrapping(Wrapping::None),
        );
    }
    r = r.push(text("›").size(18.0).color(color(t.palette.text3)));
    r.into()
}

/// Shared frame of the Settings pages: top bar (round ‹ when given and the
/// title), then a column at most 720px wide, 16px from the sides. `ui`
/// lifts the ‹ [`Action`] into the page's message type.
fn settings_frame<'a, M: 'a>(
    t: Skin,
    title: String,
    back: Option<Action>,
    body: Element<'a, M>,
    ui: fn(Action) -> M,
) -> Element<'a, M> {
    use robroz_tokens::settings as st;
    let mut bar = row![].spacing(12).align_y(Alignment::Center);
    if let Some(b) = back {
        bar = bar.push(crate::avatar_view::round_button(&t, crate::icons::Glyph::Back, b).map(ui));
    }
    bar = bar.push(
        text(title)
            .size(st::TITLE)
            .font(bold())
            .color(color(t.palette.text)),
    );
    let inner = column![
        container(bar)
            .height(Length::Fixed(robroz_tokens::home::FLOAT_BTN + 16.0))
            .align_y(Alignment::Center),
        body,
    ]
    .spacing(8)
    .width(Fill)
    .max_width(st::MAX_W);
    scrollable(
        container(inner)
            .width(Fill)
            .padding(Padding {
                top: 0.0,
                right: st::SIDE_PAD,
                bottom: st::BOTTOM_PAD,
                left: st::SIDE_PAD,
            })
            .align_x(Alignment::Center),
    )
    .height(Fill)
    .width(Fill)
    .into()
}

/// Settings (PR5): grouped cards (r16, `surface`), 24px between groups,
/// each with a 12px `text3` title 8px above; 44px rows split by 1px
/// `border` lines, 16px side padding; value in `text2` then `›`; Sign
/// out on its own red card (44px, text centred); the version 12px
/// `text3`, centred, with 24px below it.
pub fn settings_page<'a>(t: &Skin, p: SettingsPage) -> Element<'a, Action> {
    use robroz_tokens::settings as st;
    let t = *t;
    let mut col = column![].spacing(st::GROUP_GAP).width(Fill);
    for g in p.groups {
        let rows = g
            .rows
            .into_iter()
            .map(|r| {
                let trailing = settings_trailing(t, r.value);
                settings_row(t, r.label, t.palette.text, Some(trailing), r.on)
            })
            .collect();
        col = col.push(
            column![
                container(
                    text(g.title)
                        .size(st::GROUP_TITLE)
                        .color(color(t.palette.text3)),
                )
                .padding(Padding::from([0.0, st::SIDE_PAD])),
                settings_card(t, rows),
            ]
            .spacing(st::GROUP_TITLE_GAP),
        );
    }
    if let Some((label, on)) = p.sign_out {
        col = col.push(settings_card(
            t,
            vec![settings_row(t, label, crate::theme::DANGER, None, on)],
        ));
    }
    if let Some(v) = p.version {
        col = col.push(
            text(v)
                .size(st::VERSION)
                .color(color(t.palette.text3))
                .width(Fill)
                .align_x(Alignment::Center),
        );
    }
    settings_frame(t, p.title, p.back, col.into(), std::convert::identity)
}

/// A Settings choice page (Language, Appearance): one card, the active
/// option ticked in the accent colour. Picking emits
/// [`Action::SettingsChoose`].
pub fn settings_choice_page<'a>(
    t: &Skin,
    title: String,
    options: Vec<(String, bool)>,
    scale: f32,
) -> Element<'a, Action> {
    let t = *t;
    let rows = options
        .into_iter()
        .enumerate()
        .map(|(index, (label, on))| {
            let tick: Element<'a, Action> = if on {
                crate::icons::icon(crate::icons::Glyph::Check, t.palette.accent, 16.0, scale).into()
            } else {
                Space::new().width(Length::Fixed(16.0)).into()
            };
            settings_row(
                t,
                label,
                t.palette.text,
                Some(tick),
                Action::SettingsChoose { index },
            )
        })
        .collect();
    settings_frame(
        t,
        title,
        Some(Action::SettingsBack),
        settings_card(t, rows),
        std::convert::identity,
    )
}

/// A Settings sub-page that hosts another view under the same top bar
/// (round ‹ and a 20px title) and 720px width cap.
pub fn settings_sub_page<'a, M: 'a>(
    t: &Skin,
    title: String,
    back: Action,
    body: Element<'a, M>,
    ui: fn(Action) -> M,
) -> Element<'a, M> {
    settings_frame(*t, title, Some(back), body, ui)
}
