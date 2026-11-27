//! robroz mobile client spike (Slint). Connect screen, session list, one
//! status card, Settings (language, About with the Slint attribution).
//!
//! Strings come from the `robroz-ui` i18n keys (no Slint copy of their own);
//! the host talk and the status card rules are in [`api`] and [`card`].
//! Colours come from `robroz_tokens` via [`robroz_ui::theme::Skin`].

pub mod api;
pub mod avatars;
pub mod card;
pub mod chat;

use std::sync::{Arc, Mutex};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use robroz_card::Tone;
use robroz_contract::{Bro, BroAvatar, Me};
use robroz_tokens::{ColorSet, DARK, LIGHT};
use robroz_ui::avatar;
use robroz_ui::conversations;
use robroz_ui::i18n::{self, Key, Locale};
use robroz_ui::runs;
use robroz_ui::theme::Mode;

slint::include_modules!();

fn color(rgb: [u8; 3]) -> slint::Color {
    slint::Color::from_rgb_u8(rgb[0], rgb[1], rgb[2])
}

/// Appearance choice, same order as the desktop Settings row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ThemePref {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePref {
    const ALL: [ThemePref; 3] = [ThemePref::System, ThemePref::Light, ThemePref::Dark];

    fn from_index(index: i32) -> Self {
        Self::ALL
            .get(index as usize)
            .copied()
            .unwrap_or(ThemePref::System)
    }

    fn index(self) -> i32 {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0) as i32
    }

    fn resolve(self, system: Mode) -> Mode {
        match self {
            ThemePref::System => system,
            ThemePref::Light => Mode::Light,
            ThemePref::Dark => Mode::Dark,
        }
    }

    fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        Self::ALL.into_iter().find(|p| {
            let code = match p {
                ThemePref::System => "system",
                ThemePref::Light => "light",
                ThemePref::Dark => "dark",
            };
            code.eq_ignore_ascii_case(text)
        })
    }
}

fn initial_pref() -> ThemePref {
    std::env::var("ROBROZ_THEME")
        .ok()
        .as_deref()
        .and_then(ThemePref::parse)
        .unwrap_or(ThemePref::System)
}

fn system_mode(ui: &MainWindow) -> Mode {
    if ui.get_os_dark() {
        Mode::Dark
    } else {
        Mode::Light
    }
}

/// 401 copy on the connect form: the desktop title, and the env hint only
/// when the token really came from the environment. The body never includes
/// `ROBROZ_TOKEN` unless that env hint is shown.
fn unauthorized_copy(from_env: bool) -> (String, String) {
    let title = Key::ClientTokenRejectedTitle.as_str().to_string();
    let body = if from_env {
        Key::ConnSetByToken.as_str().to_string()
    } else {
        String::new()
    };
    (title, body)
}

fn apply_connect_error(ui: &MainWindow, err: &str, from_env: bool) {
    if err == api::unauthorized_message() {
        let (title, body) = unauthorized_copy(from_env);
        ui.set_error_title(title.into());
        ui.set_error(body.into());
    } else {
        ui.set_error_title("".into());
        ui.set_error(err.into());
    }
}

fn clear_connect_error(ui: &MainWindow) {
    ui.set_error_title("".into());
    ui.set_error("".into());
}

fn status_dot_color(mode: Mode, tone: Tone) -> slint::Color {
    let p = tokens_for(mode);
    color(match tone {
        Tone::Ink => p.accent,
        Tone::Green => p.dot_done,
        Tone::Amber => p.dot_stuck,
        Tone::Gray => p.dot_void,
    })
}

/// The token set for a mode. Used directly so bubble and chrome colours
/// stay on `robroz_tokens` (the desktop `Skin` still overwrites a few
/// fields with designer literals that have not landed in tokens.json).
fn tokens_for(mode: Mode) -> ColorSet {
    match mode {
        Mode::Dark => DARK,
        Mode::Light => LIGHT,
    }
}

/// Selected Language / Appearance segment: same rule as the desktop
/// client. Dark uses the border and text tokens; light uses
/// `control_checked` and text.
fn segment_selected(mode: Mode) -> ([u8; 3], [u8; 3]) {
    let p = tokens_for(mode);
    match mode {
        Mode::Dark => (p.border, p.text),
        Mode::Light => (p.control_checked, p.text),
    }
}

fn apply_theme(ui: &MainWindow, mode: Mode) {
    let p = tokens_for(mode);
    let brand = ui.global::<Brand>();
    brand.set_paper(color(p.window));
    brand.set_ink(color(p.text));
    brand.set_muted(color(p.text2));
    brand.set_rule(color(p.divider));
    brand.set_accent(color(p.accent));
    brand.set_accent_strong(color(p.accent_strong));
    brand.set_on_accent(color(p.on_accent));
    brand.set_surface(color(p.surface));
    brand.set_border(color(p.border));
    brand.set_text3(color(p.text3));
    // Own bubbles and the send button use accentStrong + white in both
    // modes. Bro bubbles use surface + border.
    brand.set_own_bg(color(p.accent_strong));
    brand.set_own_text(color(p.on_accent));
    brand.set_bro_bg(color(p.surface));
    brand.set_bro_border(color(p.border));
    let (fill, text) = segment_selected(mode);
    brand.set_segment_fill(color(fill));
    brand.set_segment_text(color(text));
}

fn apply_metrics(ui: &MainWindow) {
    use robroz_tokens::base::chat as c;
    let m = ui.global::<Metrics>();
    m.set_column_margin(c::COLUMN_MARGIN);
    m.set_bubble_radius(c::BUBBLE_RADIUS);
    m.set_bubble_pad_v(c::BUBBLE_PAD_V);
    m.set_bubble_pad_h(c::BUBBLE_PAD_H);
    m.set_gap_in(c::GAP_IN);
    m.set_gap_between(c::GAP_BETWEEN);
    // The generic "monospace" alias is not resolved on every platform.
    m.set_mono_font(if cfg!(target_os = "ios") || cfg!(target_os = "macos") {
        "Menlo".into()
    } else if cfg!(target_os = "android") {
        "monospace".into()
    } else if cfg!(target_os = "windows") {
        "Consolas".into()
    } else {
        "DejaVu Sans Mono".into()
    });
    m.set_avatar(c::AVATAR);
    m.set_avatar_gap(c::AVATAR_GAP);
    m.set_date_size(c::DATE_SIZE);
    m.set_date_gap(c::DATE_GAP);
    m.set_composer_height(c::COMPOSER_HEIGHT);
    m.set_composer_radius(c::COMPOSER_RADIUS);
    m.set_composer_pad_left(c::COMPOSER_PAD_LEFT);
    m.set_composer_send(c::COMPOSER_SEND);
    m.set_composer_send_inset(c::COMPOSER_SEND_INSET);
    m.set_composer_bottom(c::COMPOSER_BOTTOM);
    m.set_composer_send_empty_opacity(c::COMPOSER_SEND_EMPTY_OPACITY);
    m.set_bubble_mine_size(c::BUBBLE_MINE_SIZE);
    m.set_thumb_size(c::THUMB_SIZE);
    m.set_time_size(c::TIME_SIZE);
    use robroz_tokens::base::avatar as a;
    let am = ui.global::<AvatarMetrics>();
    am.set_title_height(a::TITLE_HEIGHT);
    am.set_title_size(a::TITLE_SIZE);
    am.set_title_overlap(a::TITLE_OVERLAP);
    am.set_title_pad(a::TITLE_PAD);
    am.set_more_button(a::MORE_BUTTON);
    am.set_preview(a::PREVIEW);
    am.set_preview_glyph(a::PREVIEW_GLYPH);
    am.set_preview_radius(a::PREVIEW_RADIUS);
    am.set_name_height(a::NAME_HEIGHT);
    am.set_name_radius(a::NAME_RADIUS);
    am.set_cell(a::SHAPE_CELL);
    am.set_cell_glyph(a::SHAPE_GLYPH);
    am.set_swatch(a::SWATCH);
    am.set_swatch_hit(a::SWATCH_HIT);
    am.set_ring(a::SELECTED_RING);
    am.set_create_empty_opacity(a::CREATE_EMPTY_OPACITY);
}

/// Open the avatar page for the open DM's bro, owner only.
fn open_avatar(ui: &MainWindow, state: &Arc<Mutex<State>>) {
    let mut st = state.lock().unwrap();
    let Some(bro) = avatars::dm_bro(&st.bros, &st.chat_group).cloned() else {
        return;
    };
    if !avatars::owns(st.me.as_ref(), &bro) {
        return;
    }
    st.editor = Some(avatars::Editor::open(&bro));
    ui.set_screen(5);
    render_page(ui, &st);
}

/// Avatar page pick: show it at once, then save; a failure reverts.
fn save_avatar(ui: &MainWindow, state: &Arc<Mutex<State>>, next: Option<BroAvatar>) {
    let mut guard = state.lock().unwrap();
    let st = &mut *guard;
    let Some(conn) = st.conn.clone() else {
        return;
    };
    let Some(ed) = st.editor.as_mut() else {
        return;
    };
    let Some(seq) = ed.begin(&mut st.bros, next) else {
        return;
    };
    let bro = ed.bro.clone();
    render_chat(ui, st);
    let weak = ui.as_weak();
    let state = state.clone();
    std::thread::spawn(move || {
        let result = conn.set_avatar(&bro, next);
        let _ = slint::invoke_from_event_loop(move || {
            let ui = weak.unwrap();
            let mut guard = state.lock().unwrap();
            let st = &mut *guard;
            match st.editor.as_mut() {
                Some(ed) if ed.bro == bro => ed.finish(&mut st.bros, seq, result),
                // The page moved on to another bro: keep the host's answer.
                _ => {
                    if let Ok(saved) = result {
                        if let Some(b) = st.bros.iter_mut().find(|b| b.id == saved.id) {
                            *b = saved;
                        }
                    }
                }
            }
            render_chat(&ui, st);
        });
    });
}

fn apply_pref(ui: &MainWindow, state: &mut State, pref: ThemePref) {
    state.theme_pref = pref;
    state.theme = pref.resolve(system_mode(ui));
    apply_theme(ui, state.theme);
    ui.set_theme_index(pref.index());
}

fn apply_strings(ui: &MainWindow) {
    let tr = ui.global::<Tr>();
    tr.set_title_chats(Key::ConversationsTitle.as_str().into());
    tr.set_empty_chats(Key::ConversationsEmptyHint.as_str().into());
    tr.set_label_host(Key::ConnHostLabel.as_str().into());
    tr.set_label_token(Key::ConnTokenLabel.as_str().into());
    tr.set_connect(Key::ConnConnect.as_str().into());
    tr.set_connecting(Key::ConnConnecting.as_str().into());
    tr.set_show(Key::ConnShow.as_str().into());
    tr.set_hide(Key::ConnHide.as_str().into());
    tr.set_back(Key::ViewBackToChats.as_str().into());
    tr.set_settings(Key::HostSettingsBotName.as_str().into());
    tr.set_language(Key::SettingsLanguage.as_str().into());
    tr.set_locale_en(Key::LocaleNameEn.as_str().into());
    tr.set_locale_zh(Key::LocaleNameZh.as_str().into());
    tr.set_appearance(Key::SettingsAppearance.as_str().into());
    tr.set_theme_system(Key::SettingsThemeSystem.as_str().into());
    tr.set_theme_light(Key::ClientThemeLight.as_str().into());
    tr.set_theme_dark(Key::ClientThemeDark.as_str().into());
    tr.set_about(Key::MobileAbout.as_str().into());
    tr.set_send(Key::ConversationsSend.as_str().into());
    tr.set_add_bro(Key::ConversationsNewBro.as_str().into());
    tr.set_name_placeholder(Key::CreateBroNamePlaceholder.as_str().into());
    tr.set_create(Key::CreateBroCreate.as_str().into());
    tr.set_cancel(Key::ViewRenameCancel.as_str().into());
    tr.set_avatar_title(Key::AvatarTitle.as_str().into());
    tr.set_shape(Key::AvatarShape.as_str().into());
    tr.set_color(Key::AvatarColor.as_str().into());
    tr.set_use_default(Key::AvatarUseDefault.as_str().into());
    ui.set_locale_index(match i18n::locale() {
        Locale::En => 0,
        Locale::Zh => 1,
    });
    ui.set_version_text(
        Key::MobileVersion
            .fill(&[&env!("CARGO_PKG_VERSION")])
            .into(),
    );
}

fn apply_unencrypted_hint(ui: &MainWindow) {
    let hint = if api::is_insecure_remote(&ui.get_host_url()) {
        Key::ConnUnencryptedHint.as_str().to_string()
    } else {
        String::new()
    };
    ui.set_unencrypted_hint(hint.into());
}

fn apply_token_field(ui: &MainWindow, st: &State) {
    let masked = !st.token_dirty && st.token_saved.as_deref().is_some_and(|s| !s.is_empty());
    ui.set_token_masked(masked);
    if masked {
        ui.set_show_token(false);
        ui.set_token(api::TOKEN_MASK.into());
    } else if !st.token_dirty {
        ui.set_token("".into());
    }
}

fn chat_rows(
    items: &[chat::ChatItem],
    fallback_name: &str,
    mode: Mode,
    in_group: bool,
    bros: &[Bro],
) -> Vec<ChatRow> {
    let now = runs::now_unix();
    chat::feed(items, fallback_name, in_group, now)
        .into_iter()
        .map(|entry| feed_entry_row(entry, fallback_name, mode, bros))
        .collect()
}

/// The open chat, its title capsule and the avatar pages, from `st`.
fn render_chat(ui: &MainWindow, st: &State) {
    ui.set_chat(ModelRc::new(VecModel::from(chat_rows(
        &st.chat,
        &st.chat_name,
        st.theme,
        st.chat_in_group,
        &st.bros,
    ))));
    render_title(ui, st);
    render_page(ui, st);
}

/// Title capsule: up to three avatars then "+N"; the `…` entry only for
/// the owner of a DM's bro.
fn render_title(ui: &MainWindow, st: &State) {
    let dm = avatars::dm_bro(&st.bros, &st.chat_group);
    let ids = avatars::title_ids(dm, &st.chat);
    let (shown, more) = avatar::stack(&ids);
    let ring = tokens_for(st.theme).surface;
    let list: Vec<BroAvatar> = shown
        .iter()
        .map(|id| avatars::avatar_of(&st.bros, id))
        .collect();
    let (img, width) = avatars::stack(&list, st.theme, ring);
    ui.set_title_avatars(img);
    ui.set_title_avatars_width(width);
    ui.set_title_more(more as i32);
    ui.set_can_edit_avatar(dm.is_some_and(|b| avatars::owns(st.me.as_ref(), b)));
}

/// The create-bro (4) or avatar (5) page: preview, picker, note.
fn render_page(ui: &MainWindow, st: &State) {
    let (current, note, stored, name) = match (ui.get_screen(), &st.editor) {
        (4, _) => (
            st.new_bro.unwrap_or_else(|| avatar::default_for("new-bro")),
            st.new_bro_note.clone(),
            false,
            String::new(),
        ),
        (5, Some(ed)) => {
            let b = st.bros.iter().find(|b| b.id == ed.bro);
            (
                avatars::avatar_of(&st.bros, &ed.bro),
                ed.note.clone(),
                b.is_some_and(|b| b.avatar.is_some()),
                b.map(|b| b.name.clone()).unwrap_or_default(),
            )
        }
        _ => return,
    };
    let t = robroz_tokens::base::avatar::PREVIEW_GLYPH;
    ui.set_avatar_preview(avatars::glyph(current, st.theme, t));
    ui.set_avatar_note(note.unwrap_or_default().into());
    ui.set_avatar_stored(stored);
    ui.set_avatar_name(name.into());
    ui.set_avatar_busy(st.creating);
    let cell = robroz_tokens::base::avatar::SHAPE_GLYPH;
    let shapes: Vec<PickerCell> = (0..avatar::SHAPES)
        .map(|shape| PickerCell {
            glyph: avatars::glyph(
                BroAvatar {
                    shape,
                    color: current.color,
                },
                st.theme,
                cell,
            ),
            selected: shape == current.shape,
            ..PickerCell::default()
        })
        .collect();
    let swatches: Vec<PickerCell> = (0..avatar::COLORS)
        .map(|c| PickerCell {
            fill: color(avatar::color(c, st.theme == Mode::Dark)),
            selected: c == current.color,
            ..PickerCell::default()
        })
        .collect();
    ui.set_picker_shapes(ModelRc::new(VecModel::from(shapes)));
    ui.set_picker_swatches(ModelRc::new(VecModel::from(swatches)));
}

fn feed_entry_row(entry: chat::FeedEntry, fallback: &str, mode: Mode, bros: &[Bro]) -> ChatRow {
    let shown = |p: &Option<robroz_ui::cards::codes::ProgressLine>| -> SharedString {
        p.as_ref().map(|p| p.text()).unwrap_or_default().into()
    };
    match entry {
        chat::FeedEntry::Date { label, pad_top } => ChatRow {
            kind: 3,
            text: label.into(),
            pad_top,
            ..ChatRow::default()
        },
        chat::FeedEntry::Row(row) => {
            let name = match &row.item {
                chat::ChatItem::Text { side, .. } => {
                    let n = side.name();
                    if n.is_empty() {
                        fallback.to_string()
                    } else {
                        n.to_string()
                    }
                }
                chat::ChatItem::Status(c) => {
                    if c.bot_name.is_empty() {
                        fallback.to_string()
                    } else {
                        c.bot_name.clone()
                    }
                }
            };
            let initial: SharedString = conversations::initial(&name).into();
            let bro_id = match &row.item {
                chat::ChatItem::Text {
                    side: chat::Side::Bro { id, .. },
                    ..
                } => id.as_str(),
                chat::ChatItem::Status(c) => c.bot_id.as_str(),
                _ => "",
            };
            let has_glyph = !bro_id.is_empty();
            let glyph = if has_glyph {
                avatars::glyph(
                    avatars::avatar_of(bros, bro_id),
                    mode,
                    robroz_tokens::base::chat::AVATAR,
                )
            } else {
                slint::Image::default()
            };
            match row.item {
                chat::ChatItem::Text { body, side, .. } => ChatRow {
                    kind: if side.is_own() { 0 } else { 1 },
                    name: if side.is_own() {
                        "".into()
                    } else {
                        name.into()
                    },
                    text: body.into(),
                    show_name: row.show_name,
                    show_avatar: row.show_avatar,
                    initial,
                    glyph: glyph.clone(),
                    has_glyph,
                    pad_top: row.pad_top,
                    time: row.time.into(),
                    ..ChatRow::default()
                },
                chat::ChatItem::Status(c) => ChatRow {
                    kind: 2,
                    // #155: the bro name is not drawn inside the card.
                    name: if row.show_name {
                        name.into()
                    } else {
                        "".into()
                    },
                    status: c.shown_status().into(),
                    detail: c.detail_line().unwrap_or_default().into(),
                    step: shown(&c.step_progress),
                    progress: shown(&c.progress),
                    dot: status_dot_color(mode, c.tone),
                    show_name: row.show_name,
                    show_avatar: row.show_avatar,
                    initial,
                    glyph,
                    has_glyph,
                    pad_top: row.pad_top,
                    time: row.time.into(),
                    ..ChatRow::default()
                },
            }
        }
    }
}

fn set_composer_hint(ui: &MainWindow, name: &str, dm: bool) {
    ui.set_composer_hint(conversations::composer_hint(name, dm).into());
}

#[derive(Default)]
struct State {
    conn: Option<api::Conn>,
    chat: Vec<chat::ChatItem>,
    /// The open conversation's name, for the bubbles of a bro without a card.
    chat_name: String,
    /// Names are shown once per bro run in a group chat.
    chat_in_group: bool,
    /// Where Back from Settings returns to.
    before_settings: i32,
    theme: Mode,
    theme_pref: ThemePref,
    /// The last saved (or env) token. Never shown in the field.
    token_saved: Option<String>,
    token_dirty: bool,
    /// True while the current token is the one from `ROBROZ_TOKEN`.
    token_from_env: bool,
    /// `GET /api/bros` and `GET /api/me`, loaded on connect.
    bros: Vec<Bro>,
    me: Option<Me>,
    /// The open chat's group id.
    chat_group: String,
    /// The avatar page's bro and save state.
    editor: Option<avatars::Editor>,
    /// Create-bro page: the avatar `引入` will store, its error line, and
    /// whether the request is out.
    new_bro: Option<BroAvatar>,
    new_bro_note: Option<String>,
    creating: bool,
}

/// Desktop-only shot hook: pre-fill screens from a fixture host when
/// `ROBROZ_SHOT_SCREEN` is set (`connect`, `chats`, `chat-running`,
/// `chat-failed`, `settings`, `connect-401`; PR2: `chat-group`, `chat-dm`,
/// `chat-dm-other`, `create-bro`, `avatar`, `avatar-error`).
fn prepare_shot(ui: &MainWindow, state: &Arc<Mutex<State>>, shot: &str) {
    let shot = shot.trim();
    if shot == "connect" || shot == "connect-hint" {
        return;
    }
    let host = ui.get_host_url().to_string();
    let token = {
        let st = state.lock().unwrap();
        if shot == "connect-401" {
            "wrong-token".to_string()
        } else {
            api::token_for_connect(st.token_saved.as_deref(), &ui.get_token(), st.token_dirty)
        }
    };
    let conn = match api::Conn::new(&host, &token) {
        Ok(c) => c,
        Err(e) => {
            apply_connect_error(ui, &e.0, false);
            return;
        }
    };
    let mut names: Vec<(String, String)> = Vec::new();
    match conn.groups() {
        Ok(groups) => {
            let unread = conn.unread().unwrap_or_default();
            let rows = api::session_rows(&groups, &unread);
            names = rows
                .iter()
                .map(|(g, _)| (g.id.clone(), g.name.clone()))
                .collect();
            ui.set_sessions(ModelRc::new(VecModel::from(
                rows.into_iter()
                    .map(|(g, n)| SessionRow {
                        id: g.id.into(),
                        name: g.name.into(),
                        unread: n.min(99) as i32,
                    })
                    .collect::<Vec<_>>(),
            )));
            let mut st = state.lock().unwrap();
            st.conn = Some(conn.clone());
            st.bros = conn.bros().unwrap_or_default();
            st.me = conn.me().ok();
            if !token.is_empty() && shot != "connect-401" {
                st.token_saved = Some(token);
                st.token_dirty = false;
                apply_token_field(ui, &st);
            }
        }
        Err(e) => {
            // connect-401 types a wrong token, so this is not the env token.
            apply_connect_error(ui, &e.0, false);
            if shot == "connect-401" {
                return;
            }
        }
    }
    let (group_id, screen) = match shot {
        "chats" => return ui.set_screen(1),
        "settings" => {
            state.lock().unwrap().before_settings = 1;
            return ui.set_screen(3);
        }
        "chat-running" => ("g1", 2),
        "chat-failed" => ("g2", 2),
        "chat-group" => ("g-site", 2),
        "chat-dm" | "avatar" | "avatar-error" => ("dm-ada", 2),
        // Someone else's bro: no avatar entry.
        "chat-dm-other" => ("dm-bea", 2),
        "create-bro" => {
            let mut st = state.lock().unwrap();
            st.new_bro = Some(avatar::default_for("new-bro"));
            ui.set_new_bro_name(
                std::env::var("ROBROZ_NEW_BRO_NAME")
                    .unwrap_or_default()
                    .into(),
            );
            ui.set_screen(4);
            render_page(ui, &st);
            return;
        }
        _ => return,
    };
    // Clone the conn first so the mutex is not held across events() or
    // later UI updates (if-let would otherwise extend the guard's lifetime
    // over the whole block).
    let conn = state.lock().unwrap().conn.clone();
    if let Some(conn) = conn {
        match conn.events(group_id) {
            Ok(events) => {
                let name = names
                    .into_iter()
                    .find(|(id, _)| id == group_id)
                    .map(|(_, n)| n)
                    .unwrap_or_else(|| group_id.to_string());
                let items = chat::build(&events);
                ui.set_chat_title(name.clone().into());
                ui.set_chat_note("".into());
                let mut st = state.lock().unwrap();
                st.chat = items;
                st.chat_name = name.clone();
                st.chat_in_group = true;
                st.chat_group = group_id.to_string();
                set_composer_hint(ui, &name, false);
                ui.set_composer_draft("".into());
                ui.set_composer_focused(false);
                ui.set_screen(screen);
                render_chat(ui, &st);
                if shot == "avatar" || shot == "avatar-error" {
                    drop(st);
                    open_avatar(ui, state);
                }
                if shot == "avatar-error" {
                    // A real save against the fixture, which refuses it:
                    // the preview goes back and the line shows.
                    let (conn, next, seq) = {
                        let mut st = state.lock().unwrap();
                        let cur = st
                            .editor
                            .as_ref()
                            .map(|e| avatars::avatar_of(&st.bros, &e.bro));
                        let next = cur.map(|c| BroAvatar {
                            shape: (c.shape + 1) % avatar::SHAPES,
                            color: c.color,
                        });
                        let st = &mut *st;
                        let seq = match (st.editor.as_mut(), next) {
                            (Some(ed), Some(n)) => ed.begin(&mut st.bros, Some(n)),
                            _ => None,
                        };
                        (st.conn.clone(), next, seq)
                    };
                    if let (Some(conn), Some(seq)) = (conn, seq) {
                        let bro = state.lock().unwrap().editor.as_ref().map(|e| e.bro.clone());
                        if let Some(bro) = bro {
                            let result = conn.set_avatar(&bro, next);
                            let mut st = state.lock().unwrap();
                            let st = &mut *st;
                            if let Some(ed) = st.editor.as_mut() {
                                ed.finish(&mut st.bros, seq, result);
                            }
                            render_chat(ui, st);
                        }
                    }
                }
            }
            Err(e) => ui.set_chat_note(e.0.into()),
        }
    }
}

/// Build the window and run until it closes.
pub fn run() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let state = Arc::new(Mutex::new(State::default()));

    // Desktop runs only; on a phone the environment is empty.
    if let Some(l) = std::env::var("ROBROZ_LANG")
        .ok()
        .and_then(|v| Locale::parse(&v))
    {
        i18n::set_locale(l);
    }
    if let Ok(h) = std::env::var("ROBROZ_HOST") {
        ui.set_host_url(h.into());
    }
    if let Ok(t) = std::env::var("ROBROZ_TOKEN") {
        let trimmed = t.trim().to_string();
        if !trimmed.is_empty() {
            let mut st = state.lock().unwrap();
            st.token_saved = Some(trimmed);
            st.token_from_env = true;
        }
    }
    if let Ok(v) = std::env::var("ROBROZ_SHOT_SAFE") {
        if v != "0" {
            // Typical iPhone inset, so desktop shots can show the safe-area
            // padding (the desktop backend reports 0).
            ui.set_extra_safe_top(47.0);
            ui.set_extra_safe_bottom(34.0);
        }
    }
    {
        let mut st = state.lock().unwrap();
        apply_pref(&ui, &mut st, initial_pref());
    }
    apply_strings(&ui);
    apply_metrics(&ui);
    apply_unencrypted_hint(&ui);
    apply_token_field(&ui, &state.lock().unwrap());

    if let Ok(shot) = std::env::var("ROBROZ_SHOT_SCREEN") {
        prepare_shot(&ui, &state, &shot);
    }

    ui.on_host_changed({
        let weak = ui.as_weak();
        move || {
            apply_unencrypted_hint(&weak.unwrap());
        }
    });

    ui.on_token_changed({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let mut st = state.lock().unwrap();
            let (draft, dirty) = api::edit_token(st.token_dirty, &ui.get_token());
            st.token_dirty = dirty;
            if dirty {
                st.token_from_env = false;
                ui.set_token_masked(false);
                if ui.get_token() != draft.as_str() && !draft.is_empty() {
                    ui.set_token(draft.into());
                }
            } else {
                apply_token_field(&ui, &st);
            }
        }
    });

    ui.on_toggle_show_token({
        let weak = ui.as_weak();
        move || {
            let ui = weak.unwrap();
            if ui.get_token_masked() {
                return;
            }
            ui.set_show_token(!ui.get_show_token());
        }
    });

    ui.on_connect({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            if ui.get_busy() {
                return;
            }
            clear_connect_error(&ui);
            apply_unencrypted_hint(&ui);
            let (host, token, from_env) = {
                let st = state.lock().unwrap();
                (
                    ui.get_host_url().to_string(),
                    api::token_for_connect(
                        st.token_saved.as_deref(),
                        &ui.get_token(),
                        st.token_dirty,
                    ),
                    st.token_from_env && !st.token_dirty,
                )
            };
            let conn = match api::Conn::new(&host, &token) {
                Ok(c) => c,
                Err(e) => {
                    apply_connect_error(&ui, &e.0, from_env);
                    return;
                }
            };
            ui.set_busy(true);
            let weak = ui.as_weak();
            let state = state.clone();
            std::thread::spawn(move || {
                let result = conn.groups().and_then(|groups| {
                    // The unread counts only decorate the list.
                    let unread = conn.unread().unwrap_or_default();
                    Ok(api::session_rows(&groups, &unread))
                });
                // Avatars and who owns which bro; an older host has neither.
                let bros = conn.bros().unwrap_or_default();
                let me = conn.me().ok();
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    ui.set_busy(false);
                    match result {
                        Ok(rows) => {
                            let rows: Vec<SessionRow> = rows
                                .into_iter()
                                .map(|(g, n)| SessionRow {
                                    id: g.id.into(),
                                    name: g.name.into(),
                                    unread: n.min(99) as i32,
                                })
                                .collect();
                            ui.set_sessions(ModelRc::new(VecModel::from(rows)));
                            {
                                let mut st = state.lock().unwrap();
                                st.conn = Some(conn);
                                st.bros = bros;
                                st.me = me;
                                if !token.is_empty() {
                                    st.token_from_env = from_env;
                                    st.token_saved = Some(token);
                                    st.token_dirty = false;
                                    apply_token_field(&ui, &st);
                                }
                            }
                            ui.set_screen(1);
                        }
                        Err(e) => apply_connect_error(&ui, &e.0, from_env),
                    }
                });
            });
        }
    });

    ui.on_open_session({
        let weak = ui.as_weak();
        let state = state.clone();
        move |id, name| {
            let Some(conn) = state.lock().unwrap().conn.clone() else {
                return;
            };
            let ui = weak.unwrap();
            ui.set_chat(ModelRc::new(VecModel::from(Vec::<ChatRow>::new())));
            ui.set_chat_title(name.clone());
            ui.set_chat_note("…".into());
            ui.set_screen(2);
            {
                let mut st = state.lock().unwrap();
                st.chat.clear();
                st.chat_group = id.to_string();
                render_title(&ui, &st);
            }
            let weak = ui.as_weak();
            let state = state.clone();
            let id = id.to_string();
            let name = name.to_string();
            std::thread::spawn(move || {
                let result = conn.events(&id);
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    match result {
                        Ok(events) => {
                            let items = chat::build(&events);
                            ui.set_chat_note(if items.is_empty() {
                                Key::ConversationsEmptyHint.as_str().into()
                            } else {
                                "".into()
                            });
                            let mut st = state.lock().unwrap();
                            if st.chat_group != id {
                                return;
                            }
                            st.chat = items;
                            st.chat_name = name.clone();
                            st.chat_in_group = true;
                            set_composer_hint(&ui, &name, false);
                            ui.set_composer_draft("".into());
                            ui.set_composer_focused(false);
                            render_chat(&ui, &st);
                        }
                        Err(e) => ui.set_chat_note(e.0.into()),
                    }
                });
            });
        }
    });

    ui.on_back({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let to = if ui.get_screen() == 3 {
                state.lock().unwrap().before_settings
            } else {
                1
            };
            ui.set_screen(to);
        }
    });

    ui.on_open_settings({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            state.lock().unwrap().before_settings = ui.get_screen();
            ui.set_screen(3);
        }
    });

    ui.on_set_locale({
        let weak = ui.as_weak();
        let state = state.clone();
        move |index| {
            let ui = weak.unwrap();
            let locale = if index == 1 { Locale::Zh } else { Locale::En };
            i18n::set_locale(locale);
            apply_strings(&ui);
            apply_unencrypted_hint(&ui);
            clear_connect_error(&ui);
            let st = state.lock().unwrap();
            render_chat(&ui, &st);
        }
    });

    ui.on_set_theme({
        let weak = ui.as_weak();
        let state = state.clone();
        move |index| {
            let ui = weak.unwrap();
            let mut st = state.lock().unwrap();
            apply_pref(&ui, &mut st, ThemePref::from_index(index));
            render_chat(&ui, &st);
        }
    });

    ui.on_theme_follow_system({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let mut st = state.lock().unwrap();
            if st.theme_pref != ThemePref::System {
                return;
            }
            apply_pref(&ui, &mut st, ThemePref::System);
            render_chat(&ui, &st);
        }
    });

    ui.on_send({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let body = ui.get_composer_draft().trim().to_string();
            if body.is_empty() {
                return;
            }
            ui.set_composer_draft("".into());
            ui.set_composer_focused(false);
            let mut st = state.lock().unwrap();
            st.chat.push(chat::ChatItem::Text {
                body,
                side: chat::Side::Own,
                at: Some(runs::now_unix()),
            });
            render_chat(&ui, &st);
        }
    });

    ui.on_open_new_bro({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let mut st = state.lock().unwrap();
            // A random start; the preview shows exactly what 引入 stores.
            let seed = format!(
                "{:?}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
            );
            st.new_bro = Some(avatar::default_for(&seed));
            st.new_bro_note = None;
            ui.set_new_bro_name("".into());
            ui.set_screen(4);
            render_page(&ui, &st);
        }
    });

    ui.on_open_avatar({
        let weak = ui.as_weak();
        let state = state.clone();
        move || open_avatar(&weak.unwrap(), &state)
    });

    ui.on_pick_shape({
        let weak = ui.as_weak();
        let state = state.clone();
        move |i| {
            let ui = weak.unwrap();
            let shape = (i.max(0) as u8) % avatar::SHAPES;
            if ui.get_screen() == 4 {
                let mut st = state.lock().unwrap();
                if !st.creating {
                    let cur = st.new_bro.unwrap_or_else(|| avatar::default_for("new-bro"));
                    st.new_bro = Some(BroAvatar { shape, ..cur });
                    render_page(&ui, &st);
                }
                return;
            }
            let next = {
                let st = state.lock().unwrap();
                st.editor.as_ref().map(|e| BroAvatar {
                    shape,
                    ..avatars::avatar_of(&st.bros, &e.bro)
                })
            };
            if let Some(n) = next {
                save_avatar(&ui, &state, Some(n));
            }
        }
    });

    ui.on_pick_color({
        let weak = ui.as_weak();
        let state = state.clone();
        move |i| {
            let ui = weak.unwrap();
            let color = (i.max(0) as u8) % avatar::COLORS;
            if ui.get_screen() == 4 {
                let mut st = state.lock().unwrap();
                if !st.creating {
                    let cur = st.new_bro.unwrap_or_else(|| avatar::default_for("new-bro"));
                    st.new_bro = Some(BroAvatar { color, ..cur });
                    render_page(&ui, &st);
                }
                return;
            }
            let next = {
                let st = state.lock().unwrap();
                st.editor.as_ref().map(|e| BroAvatar {
                    color,
                    ..avatars::avatar_of(&st.bros, &e.bro)
                })
            };
            if let Some(n) = next {
                save_avatar(&ui, &state, Some(n));
            }
        }
    });

    ui.on_avatar_reset({
        let weak = ui.as_weak();
        let state = state.clone();
        move || save_avatar(&weak.unwrap(), &state, None)
    });

    ui.on_create_bro({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let name = ui.get_new_bro_name().trim().to_string();
            let (conn, a) = {
                let mut st = state.lock().unwrap();
                if st.creating || name.is_empty() {
                    return;
                }
                let Some(conn) = st.conn.clone() else {
                    return;
                };
                st.creating = true;
                st.new_bro_note = None;
                render_page(&ui, &st);
                let a = st.new_bro.unwrap_or_else(|| avatar::default_for("new-bro"));
                (conn, a)
            };
            let weak = ui.as_weak();
            let state = state.clone();
            std::thread::spawn(move || {
                let result = conn.create_bro(&name, a);
                // The new DM shows up in the list right away.
                let rows = conn.groups().ok().map(|groups| {
                    let unread = conn.unread().unwrap_or_default();
                    api::session_rows(&groups, &unread)
                });
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    let mut st = state.lock().unwrap();
                    st.creating = false;
                    match result {
                        Ok(bro) => {
                            if let Some(rows) = rows {
                                ui.set_sessions(ModelRc::new(VecModel::from(
                                    rows.into_iter()
                                        .map(|(g, n)| SessionRow {
                                            id: g.id.into(),
                                            name: g.name.into(),
                                            unread: n.min(99) as i32,
                                        })
                                        .collect::<Vec<_>>(),
                                )));
                            }
                            let dm = bro.dm_group_id.clone();
                            let name = bro.name.clone();
                            if !st.bros.iter().any(|b| b.id == bro.id) {
                                st.bros.push(bro);
                            }
                            st.new_bro = None;
                            drop(st);
                            match dm {
                                Some(dm) => ui.invoke_open_session(dm.into(), name.into()),
                                None => ui.set_screen(1),
                            }
                        }
                        Err(e) => {
                            st.new_bro_note = Some(Key::ClientNewBroFailed.fill(&[&e.0]));
                            render_page(&ui, &st);
                        }
                    }
                });
            });
        }
    });

    ui.on_page_back({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let to = if ui.get_screen() == 5 { 2 } else { 1 };
            ui.set_screen(to);
            let st = state.lock().unwrap();
            render_title(&ui, &st);
        }
    });

    ui.run()
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: slint::android::AndroidApp) {
    slint::android::init(app).expect("init android backend");
    run().expect("run");
}

#[cfg(test)]
mod theme_tests {
    use super::*;

    #[test]
    fn segment_selected_matches_desktop_tokens() {
        assert_eq!(
            segment_selected(Mode::Light),
            (LIGHT.control_checked, LIGHT.text)
        );
        // Same values the desktop client uses (DARK.border / DARK.text).
        assert_eq!(segment_selected(Mode::Dark), (DARK.border, DARK.text));
    }

    #[test]
    fn appearance_defaults_to_system_then_light_and_dark() {
        assert_eq!(ThemePref::default(), ThemePref::System);
        assert_eq!(ThemePref::from_index(0).index(), 0);
        assert_eq!(ThemePref::from_index(1), ThemePref::Light);
        assert_eq!(ThemePref::from_index(2), ThemePref::Dark);
        assert_eq!(ThemePref::System.resolve(Mode::Dark), Mode::Dark);
        assert_eq!(ThemePref::Light.resolve(Mode::Dark), Mode::Light);
        assert_eq!(ThemePref::parse("system"), Some(ThemePref::System));
        assert_eq!(ThemePref::parse("DARK"), Some(ThemePref::Dark));
    }

    #[test]
    fn running_dot_is_accent_and_failed_dot_is_grey() {
        assert_eq!(
            status_dot_color(Mode::Light, Tone::Ink),
            color(LIGHT.accent)
        );
        assert_eq!(status_dot_color(Mode::Dark, Tone::Ink), color(DARK.accent));
        assert_eq!(
            status_dot_color(Mode::Light, Tone::Gray),
            color(LIGHT.dot_void)
        );
        assert_eq!(
            status_dot_color(Mode::Dark, Tone::Gray),
            color(DARK.dot_void)
        );
    }

    #[test]
    fn rejected_token_title_omits_env_name_unless_the_token_is_from_env() {
        use robroz_ui::i18n::{scoped, text_in, Locale};
        let _en = scoped(Locale::En);
        let (title, body) = unauthorized_copy(false);
        assert_eq!(title, text_in(Locale::En, Key::ClientTokenRejectedTitle));
        assert_eq!(title, "Token rejected");
        assert!(body.is_empty());
        assert!(!title.contains("ROBROZ_TOKEN"));
        let (title, body) = unauthorized_copy(true);
        assert_eq!(title, "Token rejected");
        assert_eq!(body, text_in(Locale::En, Key::ConnSetByToken));
        assert!(body.contains("ROBROZ_TOKEN"));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        let (title, body) = unauthorized_copy(false);
        assert_eq!(title, "令牌被拒绝");
        assert!(body.is_empty());
        assert!(!title.contains("ROBROZ_TOKEN"));
    }
}
