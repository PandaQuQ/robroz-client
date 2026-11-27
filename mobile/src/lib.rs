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
pub mod mcp;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use robroz_card::Tone;
use robroz_contract::{Bro, BroAvatar, Group, GroupMembers, GroupSearchHit, Me, Unread};
use robroz_tokens::{ColorSet, DARK, LIGHT};
use robroz_ui::avatar;
use robroz_ui::conversations;
use robroz_ui::i18n::{self, Key, Locale};
use robroz_ui::runs;
use robroz_ui::theme::{Mode, Skin};

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
    brand.set_danger(color(robroz_ui::theme::DANGER));
    brand.set_stuck(color(Skin::for_mode(mode).status.blocked));
    brand.set_done(color(Skin::for_mode(mode).status.done));
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
    // Phone: 48px cells, 12px gap, six per row (two rows). Desktop keeps 56.
    am.set_cell(48.0);
    am.set_cell_glyph(32.0);
    am.set_swatch(a::SWATCH);
    am.set_swatch_hit(a::SWATCH_HIT);
    am.set_ring(a::SELECTED_RING);
    am.set_create_empty_opacity(a::CREATE_EMPTY_OPACITY);
    use robroz_tokens::home as h;
    let hm = ui.global::<HomeMetrics>();
    hm.set_row(h::ROW);
    hm.set_row_face_w(h::ROW_FACE_W);
    hm.set_row_avatar(h::ROW_AVATAR);
    hm.set_row_name(h::ROW_NAME);
    hm.set_row_time(h::ROW_TIME);
    hm.set_row_preview(h::ROW_PREVIEW);
    hm.set_unread_dot(h::UNREAD_DOT);
    hm.set_search(h::SEARCH);
    hm.set_search_lift(h::SEARCH_LIFT);
    hm.set_float_btn(h::FLOAT_BTN);
    hm.set_menu_w(h::MENU_W);
    hm.set_menu_r(h::MENU_R);
    hm.set_menu_gap(h::MENU_GAP);
    hm.set_menu_pad(h::MENU_PAD);
    hm.set_menu_divider_gap(h::MENU_DIVIDER_GAP);
    hm.set_more_icon(h::MORE_ICON);
    hm.set_back_gap(h::BACK_GAP);
    hm.set_profile_card_gap(h::PROFILE_CARD_GAP);
    hm.set_profile_side(h::PROFILE_SIDE);
    hm.set_menu_row(h::MENU_ROW);
    hm.set_menu_text(h::MENU_TEXT);
    hm.set_profile_avatar(h::PROFILE_AVATAR);
    hm.set_profile_name(h::PROFILE_NAME);
    hm.set_profile_name_gap(h::PROFILE_NAME_GAP);
    hm.set_profile_edit_h(h::PROFILE_EDIT_H);
    hm.set_card_r(h::CARD_R);
    hm.set_card_row(h::CARD_ROW);
    use robroz_tokens::mcp as k;
    let mm = ui.global::<McpMetrics>();
    mm.set_capsule(k::CAPSULE);
    mm.set_capsule_pad(k::CAPSULE_PAD);
    mm.set_capsule_gap(k::CAPSULE_GAP);
    mm.set_capsule_text(k::CAPSULE_TEXT);
    mm.set_row(k::ROW);
    mm.set_icon(k::ICON);
    mm.set_icon_r(k::ICON_R);
    mm.set_icon_gap(k::ICON_GAP);
    mm.set_icon_text(k::ICON_TEXT);
    mm.set_row_name(k::ROW_NAME);
    mm.set_row_sub(k::ROW_SUB);
    mm.set_connect_h(k::CONNECT_H);
    mm.set_count_text(k::COUNT_TEXT);
    mm.set_connected_dot(k::CONNECTED_DOT);
}

/// Push the MCP book into screen 9 (and its command sheet).
fn apply_mcp(ui: &MainWindow, st: &State) {
    let book = &st.mcp_book;
    let rows: Vec<McpLine> = mcp::lines(book)
        .into_iter()
        .map(|l| McpLine {
            id: l.id.into(),
            name: l.name.into(),
            letter: l.letter.into(),
            sub: l.sub.into(),
            connected: l.connected,
        })
        .collect();
    ui.set_mcp_rows(ModelRc::new(VecModel::from(rows)));
    ui.set_mcp_filter(mcp::filter_index(book.active_filter()));
    ui.set_mcp_connected_text(mcp::connected_text(book).into());
    ui.set_mcp_categories(mcp::show_categories(book));
    ui.set_mcp_note(book.note.clone().unwrap_or_default().into());
    ui.set_mcp_loaded(book.loaded);
    ui.set_mcp_busy(book.busy);
    let prompt = st
        .mcp_prompt
        .as_deref()
        .and_then(|id| book.command_prompt_for(id).map(|p| (id, p)));
    ui.set_mcp_prompt_open(prompt.is_some());
    if let Some((id, p)) = prompt {
        ui.set_mcp_prompt_command_text(p.command.clone().into());
        ui.set_mcp_prompt_args_text(p.args.join("\n").into());
        ui.set_mcp_prompt_note(book.approve_note_for(id).unwrap_or("").into());
    }
}

/// Apply a connection test result: a 409 opens the command sheet.
fn mcp_tested(
    st: &mut State,
    id: &str,
    result: Result<robroz_contract::McpTestResult, api::McpTestError>,
) {
    match result {
        Ok(r) => st.mcp_book.tested(id, &r),
        Err(api::McpTestError::NeedsApproval(d)) => {
            st.mcp_book.show_command_prompt(id, d.command, d.args);
            st.mcp_prompt = Some(id.to_string());
        }
        Err(api::McpTestError::Failed(e)) => st.mcp_book.test_failed(id, &e.0),
    }
}

/// Screen 9: load the server list.
fn open_mcp(ui: &MainWindow, state: &Arc<Mutex<State>>) {
    let Some(conn) = state.lock().unwrap().conn.clone() else {
        return;
    };
    {
        let mut st = state.lock().unwrap();
        st.mcp_book = Default::default();
        st.mcp_prompt = None;
        apply_mcp(ui, &st);
    }
    ui.set_screen(9);
    let weak = ui.as_weak();
    let state = state.clone();
    std::thread::spawn(move || {
        let result = conn.mcp_servers();
        let _ = slint::invoke_from_event_loop(move || {
            let ui = weak.unwrap();
            let mut st = state.lock().unwrap();
            match result {
                Ok(list) => {
                    st.mcp_book.load(list);
                    st.mcp_book.note = None;
                }
                Err(e) => {
                    st.mcp_book.loaded = true;
                    st.mcp_book.note = Some(e.0);
                }
            }
            apply_mcp(&ui, &st);
        });
    });
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
    tr.set_search_placeholder(Key::HomeSearchPlaceholder.as_str().into());
    tr.set_menu_profile(Key::GroupMenuProfile.as_str().into());
    tr.set_menu_find(Key::GroupMenuFind.as_str().into());
    tr.set_menu_delete(Key::GroupMenuDelete.as_str().into());
    tr.set_menu_rename(Key::GroupMenuRename.as_str().into());
    tr.set_delete_confirm(Key::GroupDeleteConfirm.as_str().into());
    tr.set_profile_edit(Key::GroupProfileEdit.as_str().into());
    tr.set_profile_members(Key::GroupProfileMembers.as_str().into());
    tr.set_profile_copy_id(Key::GroupProfileCopyId.as_str().into());
    tr.set_profile_copied(Key::GroupProfileCopied.as_str().into());
    tr.set_delete_body(Key::GroupDeleteBody.as_str().into());
    tr.set_find_title(Key::FindInChatTitle.as_str().into());
    tr.set_find_placeholder(Key::FindInChatPlaceholder.as_str().into());
    tr.set_find_empty(Key::FindInChatEmpty.as_str().into());
    tr.set_find_no_results(Key::FindInChatNoResults.as_str().into());
    tr.set_members_title(Key::MembersTitle.as_str().into());
    tr.set_settings_title(Key::SettingsTitle.as_str().into());
    tr.set_sign_out(Key::SignOutOnThisDevice.as_str().into());
    {
        use robroz_ui::cards::mcp as m;
        tr.set_mcp_title(m::TITLE.into());
        tr.set_mcp_all(m::FILTER_ALL.as_str().into());
        tr.set_mcp_connect(m::CONNECT.as_str().into());
        tr.set_mcp_connected(m::CONNECTED.as_str().into());
        tr.set_mcp_empty(m::EMPTY_TITLE.as_str().into());
        tr.set_mcp_prompt_command(m::LABEL_PROMPT_COMMAND.as_str().into());
        tr.set_mcp_prompt_args(m::LABEL_PROMPT_ARGS.as_str().into());
        tr.set_mcp_runs_local(m::COMMAND_RUNS_LOCAL.as_str().into());
        tr.set_mcp_approve_run(m::APPROVE_RUN.as_str().into());
    }
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

fn window_scale(ui: &MainWindow) -> f32 {
    ui.window().scale_factor().max(1.0)
}

fn items_from_events(
    prev: &[chat::ChatItem],
    events: &[(robroz_contract::Event, bool)],
) -> Vec<chat::ChatItem> {
    let mut items = chat::build_from(events);
    chat::carry_process(prev, &mut items);
    items
}

fn chat_rows(
    items: &[chat::ChatItem],
    fallback_name: &str,
    mode: Mode,
    in_group: bool,
    bros: &[Bro],
    scale: f32,
) -> Vec<ChatRow> {
    let now = runs::now_unix();
    chat::feed(items, fallback_name, in_group, now)
        .into_iter()
        .map(|entry| feed_entry_row(entry, fallback_name, mode, bros, scale))
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
        window_scale(ui),
    ))));
    render_title(ui, st);
    render_working(ui, st);
    render_banner(ui, st);
    render_page(ui, st);
    apply_chat_chrome(ui, st);
}

fn render_working(ui: &MainWindow, st: &State) {
    let working = chat::working_bros(&st.chat);
    let names: Vec<&str> = working.iter().map(|(_, n)| n.as_str()).collect();
    if let Some(line) = robroz_ui::process::working_row(&names) {
        let list: Vec<_> = working
            .iter()
            .take(3)
            .map(|(id, _)| avatars::avatar_of(&st.bros, id))
            .collect();
        let ring = tokens_for(st.theme).surface;
        let (img, width) = avatars::stack_at(
            &list,
            st.theme,
            ring,
            window_scale(ui),
            16.0,
            16.0 * 8.0 / 24.0,
        );
        ui.set_working_row(line.into());
        ui.set_working_avatars(img);
        ui.set_working_avatars_width(width);
    } else {
        ui.set_working_row("".into());
        ui.set_working_avatars(slint::Image::default());
        ui.set_working_avatars_width(0.0);
    }
}

fn render_banner(ui: &MainWindow, st: &State) {
    if robroz_ui::process::show_reconnect_banner(st.poll_fail_since, robroz_ui::process::now_unix())
    {
        ui.set_reconnect_banner(robroz_ui::process::reconnect_banner().into());
    } else {
        ui.set_reconnect_banner("".into());
    }
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
    let (img, width) = avatars::stack(&list, st.theme, ring, window_scale(ui));
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
    let scale = window_scale(ui);
    let t = robroz_tokens::base::avatar::PREVIEW_GLYPH;
    ui.set_avatar_preview(avatars::glyph(current, st.theme, t, scale));
    ui.set_avatar_note(note.unwrap_or_default().into());
    ui.set_avatar_stored(stored);
    ui.set_avatar_name(name.into());
    ui.set_avatar_busy(st.creating);
    let cell = 32.0; // matches AvatarMetrics.cell-glyph
    let shapes: Vec<PickerCell> = (0..avatar::SHAPES)
        .map(|shape| PickerCell {
            glyph: avatars::glyph(
                BroAvatar {
                    shape,
                    color: current.color,
                },
                st.theme,
                cell,
                scale,
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

fn status_runs_of(line: &str) -> ModelRc<StatusRun> {
    let runs: Vec<StatusRun> = robroz_ui::process::line_spans(line)
        .into_iter()
        .map(|(text, mono)| StatusRun {
            text: text.into(),
            mono,
        })
        .collect();
    ModelRc::new(VecModel::from(runs))
}

fn feed_entry_row(
    entry: chat::FeedEntry,
    fallback: &str,
    mode: Mode,
    bros: &[Bro],
    scale: f32,
) -> ChatRow {
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
                    scale,
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
                chat::ChatItem::Status(c) => {
                    let now = robroz_ui::process::now_unix();
                    let failed = c.failure.is_some();
                    let (status, step, progress, runs, shrink, expandable) = if failed {
                        (
                            c.shown_status(),
                            shown(&c.step_progress),
                            shown(&c.progress),
                            ModelRc::new(VecModel::<StatusRun>::from(vec![])),
                            false,
                            false,
                        )
                    } else {
                        let status = c.process_line(now).unwrap_or_else(|| c.shown_status());
                        let progress: SharedString = if c.expanded {
                            c.steps
                                .iter()
                                .map(|s| s.text())
                                .collect::<Vec<_>>()
                                .join("\n")
                                .into()
                        } else {
                            "".into()
                        };
                        let shrink = !c.expanded && progress.is_empty();
                        let runs = status_runs_of(&status);
                        (status, "".into(), progress, runs, shrink, c.expandable())
                    };
                    ChatRow {
                        kind: 2,
                        // #155: the bro name is not drawn inside the card.
                        name: if row.show_name {
                            name.into()
                        } else {
                            "".into()
                        },
                        status: status.into(),
                        detail: c.detail_line().unwrap_or_default().into(),
                        step,
                        progress,
                        dot: status_dot_color(mode, c.tone),
                        show_name: row.show_name,
                        show_avatar: row.show_avatar,
                        initial,
                        glyph,
                        has_glyph,
                        pad_top: row.pad_top,
                        time: row.time.into(),
                        task_id: c.task_id.clone().into(),
                        expandable,
                        status_runs: runs,
                        status_shrink: shrink,
                        ..ChatRow::default()
                    }
                }
            }
        }
    }
}

fn set_composer_hint(ui: &MainWindow, name: &str, dm: bool) {
    ui.set_composer_hint(conversations::composer_hint(name, dm).into());
}

fn current_group<'a>(st: &'a State) -> Option<&'a Group> {
    st.groups.iter().find(|g| g.id == st.chat_group)
}

fn close_menus(ui: &MainWindow) {
    ui.set_group_menu(false);
    ui.set_self_menu(false);
}

fn apply_sessions(ui: &MainWindow, st: &State) {
    let q = st.home_query.trim().to_lowercase();
    let rows = api::session_rows(&st.groups, &st.unread);
    let scale = window_scale(ui);
    let model: Vec<SessionRow> = rows
        .into_iter()
        .filter(|(g, _)| q.is_empty() || g.name.to_lowercase().contains(&q))
        .map(|(g, n)| {
            let (glyph, has_glyph) = row_glyph(st, &g, scale);
            SessionRow {
                id: g.id.clone().into(),
                name: g.name.clone().into(),
                unread: n.min(99) as i32,
                preview: SharedString::default(),
                time: SharedString::default(),
                glyph,
                has_glyph,
            }
        })
        .collect();
    ui.set_sessions(ModelRc::new(VecModel::from(model)));
    ui.set_me_name(
        st.me
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_default()
            .into(),
    );
    ui.set_me_initial(
        conversations::initial(st.me.as_ref().map(|m| m.name.as_str()).unwrap_or_default()).into(),
    );
    ui.set_home_query(st.home_query.clone().into());
}

fn row_glyph(st: &State, g: &Group, scale: f32) -> (slint::Image, bool) {
    if let Some(b) = avatars::dm_bro(&st.bros, &g.id) {
        return (
            avatars::glyph(
                avatars::avatar_of(&st.bros, &b.id),
                st.theme,
                robroz_tokens::home::ROW_AVATAR,
                scale,
            ),
            true,
        );
    }
    if let Some((faces, _)) = st.group_faces.get(&g.id) {
        if !faces.is_empty() {
            return (
                avatars::group_stack(faces, st.theme, tokens_for(st.theme).window, scale),
                true,
            );
        }
    }
    (slint::Image::default(), false)
}

fn apply_chat_chrome(ui: &MainWindow, st: &State) {
    let unread_other: u64 = st
        .unread
        .iter()
        .filter(|u| u.group_id != st.chat_group)
        .map(|u| u.unread)
        .sum();
    ui.set_unread_other(unread_other.min(99) as i32);
    let can = current_group(st).is_some_and(|g| api::can_manage(st.me.as_ref(), g));
    ui.set_can_manage(can);
    let name = current_group(st)
        .map(|g| g.name.as_str())
        .unwrap_or(st.chat_name.as_str());
    ui.global::<Tr>()
        .set_delete_title(Key::GroupDeleteTitle.fill(&[&name]).into());
    if let Some(n) = st.routines_on.get(&st.chat_group) {
        ui.set_routines_on(*n as i32);
        ui.global::<Tr>()
            .set_profile_routines(Key::GroupProfileRoutinesOn.fill(&[&n.to_string()]).into());
    } else {
        ui.set_routines_on(-1);
    }
    ui.set_member_count(
        st.member_count
            .get(&st.chat_group)
            .map(|n| *n as i32)
            .unwrap_or(-1),
    );
    ui.set_group_note(st.group_note.clone().unwrap_or_default().into());
    ui.set_group_edit(st.group_edit.clone().into());
    ui.set_group_editing(st.group_editing);
    ui.set_copied(st.copied_until.is_some_and(|u| now_unix() < u));
    let scale = window_scale(ui);
    let faces = st
        .group_faces
        .get(&st.chat_group)
        .cloned()
        .unwrap_or_default();
    let (img, w) = if faces.0.is_empty() {
        if let Some(b) = avatars::dm_bro(&st.bros, &st.chat_group) {
            avatars::stack_at(
                &[avatars::avatar_of(&st.bros, &b.id)],
                st.theme,
                tokens_for(st.theme).surface,
                scale,
                robroz_tokens::home::PROFILE_AVATAR,
                0.0,
            )
        } else {
            (slint::Image::default(), 0.0)
        }
    } else {
        avatars::stack_at(
            &faces.0,
            st.theme,
            tokens_for(st.theme).surface,
            scale,
            robroz_tokens::home::PROFILE_AVATAR,
            robroz_tokens::base::avatar::TITLE_OVERLAP
                * (robroz_tokens::home::PROFILE_AVATAR / robroz_tokens::base::avatar::TITLE_SIZE),
        )
    };
    ui.set_profile_avatars(img);
    ui.set_profile_avatars_width(w);
    ui.set_profile_more(faces.1 as i32);
}

fn apply_find(ui: &MainWindow, st: &State) {
    let hits: Vec<FindHit> = st
        .find_hits
        .iter()
        .map(|h| FindHit {
            message_id: h.message_id.clone().into(),
            snippet: h.snippet.clone().into(),
            when: h.created_at.clone().into(),
            author: h.author_name.clone().unwrap_or_default().into(),
        })
        .collect();
    ui.set_find_hits(ModelRc::new(VecModel::from(hits)));
    ui.set_find_query(st.find_query.clone().into());
    ui.set_find_busy(false);
}

fn apply_members(ui: &MainWindow, st: &State) {
    let scale = window_scale(ui);
    let mut rows: Vec<MemberLine> = Vec::new();
    if let Some(m) = &st.members {
        for b in &m.bros {
            rows.push(MemberLine {
                name: b.name.clone().into(),
                detail: b.owner_name.clone().unwrap_or_default().into(),
                glyph: avatars::glyph(
                    avatars::avatar_of(&st.bros, &b.bot_id),
                    st.theme,
                    40.0,
                    scale,
                ),
                has_glyph: true,
            });
        }
        for u in &m.users {
            rows.push(MemberLine {
                name: u.name.clone().unwrap_or_else(|| u.user_id.clone()).into(),
                detail: SharedString::default(),
                glyph: slint::Image::default(),
                has_glyph: false,
            });
        }
    }
    ui.set_members_list(ModelRc::new(VecModel::from(rows)));
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn store_groups(st: &mut State, groups: Vec<Group>, unread: Vec<Unread>) {
    st.groups = groups;
    st.unread = unread;
}

fn load_group_extras(
    ui: &MainWindow,
    state: &Arc<Mutex<State>>,
    conn: &api::Conn,
    groups: &[Group],
) {
    for g in groups.iter().filter(|g| !g.dm) {
        let conn = conn.clone();
        let id = g.id.clone();
        let weak = ui.as_weak();
        let state = state.clone();
        std::thread::spawn(move || {
            let members = conn.group_members(&id).ok();
            let schedules = conn.schedules(&id).ok();
            let _ = slint::invoke_from_event_loop(move || {
                let ui = weak.unwrap();
                let mut st = state.lock().unwrap();
                if let Some(m) = members {
                    let mut faces: Vec<BroAvatar> = m
                        .bros
                        .iter()
                        .map(|b| avatars::avatar_of(&st.bros, &b.bot_id))
                        .collect();
                    let more = faces.len().saturating_sub(4);
                    faces.truncate(4);
                    st.member_count.insert(id.clone(), m.bros.len() as u64);
                    st.group_faces.insert(id.clone(), (faces, more));
                    if st.chat_group == id && ui.get_screen() == 8 {
                        st.members = Some(m);
                        apply_members(&ui, &st);
                    }
                }
                if let Some(s) = schedules {
                    let on = s.iter().filter(|e| !e.paused).count() as u64;
                    st.routines_on.insert(id.clone(), on);
                }
                apply_sessions(&ui, &st);
                apply_chat_chrome(&ui, &st);
            });
        });
    }
}

fn copy_to_clipboard(text: &str) {
    let _ = std::process::Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            if let Some(stdin) = c.stdin.as_mut() {
                let _ = stdin.write_all(text.as_bytes());
            }
            c.wait()
        });
    let _ = std::process::Command::new("wl-copy").arg(text).status();
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
    /// Continuous poll/stream failure start, for the reconnecting banner.
    poll_fail_since: Option<i64>,
    groups: Vec<Group>,
    unread: Vec<Unread>,
    home_query: String,
    group_faces: HashMap<String, (Vec<BroAvatar>, usize)>,
    routines_on: HashMap<String, u64>,
    member_count: HashMap<String, u64>,
    members: Option<GroupMembers>,
    find_query: String,
    find_hits: Vec<GroupSearchHit>,
    group_note: Option<String>,
    group_edit: String,
    group_editing: bool,
    copied_until: Option<i64>,
    /// MCP page (screen 9) and the server whose command sheet is open.
    mcp_book: robroz_ui::cards::mcp::Book,
    mcp_prompt: Option<String>,
}

/// Desktop-only shot hook: pre-fill screens from a fixture host when
/// `ROBROZ_SHOT_SCREEN` is set (`connect`, `chats`, `chat-running`,
/// `chat-failed`, `settings`, `connect-401`; PR2: `chat-group`, `chat-dm`,
/// `chat-dm-other`, `create-bro`, `avatar`, `avatar-error`; PR6: `mcp`,
/// `mcp-prompt`).
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
            let mut st = state.lock().unwrap();
            st.conn = Some(conn.clone());
            st.bros = conn.bros().unwrap_or_default();
            st.me = conn.me().ok();
            store_groups(&mut st, groups.clone(), unread);
            apply_sessions(ui, &st);
            load_group_extras(ui, state, &conn, &groups);
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
            ui.set_mcp_available(true);
            return ui.set_screen(3);
        }
        "mcp" | "mcp-prompt" => {
            let mut st = state.lock().unwrap();
            st.before_settings = 1;
            ui.set_mcp_available(true);
            if let Some(conn) = st.conn.clone() {
                match conn.mcp_servers() {
                    Ok(list) => st.mcp_book.load(list),
                    Err(e) => {
                        st.mcp_book.loaded = true;
                        st.mcp_book.note = Some(e.0);
                    }
                }
                // `mcp-prompt`: Connect on the first stdio server whose
                // command is not approved, so the sheet shows.
                let first = st
                    .mcp_book
                    .servers
                    .iter()
                    .find(|s| robroz_ui::cards::mcp::command_unapproved(s))
                    .map(|s| s.id.clone());
                if let (true, Some(id)) = (shot == "mcp-prompt", first) {
                    if st.mcp_book.connect_to_send(&id).is_some() {
                        if let Ok(server) = conn.set_mcp_enabled(&id, true) {
                            st.mcp_book.connect_enabled(&server);
                        }
                        let result = conn.test_mcp(&id);
                        mcp_tested(&mut st, &id, result);
                    }
                }
            }
            ui.set_screen(9);
            apply_mcp(ui, &st);
            return;
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
                let items = items_from_events(&[], &events);
                ui.set_chat_title(name.clone().into());
                ui.set_chat_note("".into());
                let mut st = state.lock().unwrap();
                st.chat = items;
                st.poll_fail_since = None;
                st.chat_name = name.clone();
                st.chat_group = group_id.to_string();
                st.chat_in_group = avatars::dm_bro(&st.bros, &st.chat_group).is_none();
                set_composer_hint(ui, &name, !st.chat_in_group);
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

    if std::env::var("ROBROZ_SHOT_SCREEN").is_ok()
        || std::env::var("ROBROZ_FAKE_RECONNECTING").is_ok()
    {
        if let Some(unix) = robroz_ui::process::parse_unix(robroz_ui::process::FAKE_NOW_ISO) {
            robroz_ui::process::freeze_clock(unix);
        }
    }
    if std::env::var("ROBROZ_FAKE_RECONNECTING").as_deref() == Ok("1") {
        state.lock().unwrap().poll_fail_since =
            Some(robroz_ui::process::now_unix() - (robroz_ui::process::SSE_BANNER_SECS as i64 + 1));
    }
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
                let result = conn.groups().map(|groups| {
                    let unread = conn.unread().unwrap_or_default();
                    (groups, unread)
                });
                // Avatars and who owns which bro; an older host has neither.
                let bros = conn.bros().unwrap_or_default();
                let me = conn.me().ok();
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    ui.set_busy(false);
                    match result {
                        Ok((groups, unread)) => {
                            {
                                let mut st = state.lock().unwrap();
                                st.conn = Some(conn.clone());
                                st.bros = bros;
                                st.me = me;
                                store_groups(&mut st, groups.clone(), unread);
                                if !token.is_empty() {
                                    st.token_from_env = from_env;
                                    st.token_saved = Some(token);
                                    st.token_dirty = false;
                                    apply_token_field(&ui, &st);
                                }
                                apply_sessions(&ui, &st);
                            }
                            load_group_extras(&ui, &state, &conn, &groups);
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
            close_menus(&ui);
            ui.set_delete_open(false);
            ui.set_screen(2);
            {
                let mut st = state.lock().unwrap();
                st.chat.clear();
                st.chat_group = id.to_string();
                st.group_editing = false;
                st.group_note = None;
                render_title(&ui, &st);
                apply_chat_chrome(&ui, &st);
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
                            let items = {
                                let st = state.lock().unwrap();
                                items_from_events(&st.chat, &events)
                            };
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
                            st.poll_fail_since = None;
                            st.chat_name = name.clone();
                            st.chat_in_group = avatars::dm_bro(&st.bros, &id).is_none();
                            set_composer_hint(&ui, &name, !st.chat_in_group);
                            ui.set_composer_draft("".into());
                            ui.set_composer_focused(false);
                            render_chat(&ui, &st);
                        }
                        Err(e) => {
                            ui.set_chat_note(e.0.into());
                            let mut st = state.lock().unwrap();
                            if st.poll_fail_since.is_none() {
                                st.poll_fail_since = Some(robroz_ui::process::now_unix());
                            }
                            render_banner(&ui, &st);
                        }
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
            close_menus(&ui);
            ui.set_delete_open(false);
            let screen = ui.get_screen();
            let to = match screen {
                3 => state.lock().unwrap().before_settings,
                9 => 3,
                6 | 7 => 2,
                8 => 6,
                _ => 1,
            };
            ui.set_screen(to);
            if to == 2 || to == 6 {
                let st = state.lock().unwrap();
                apply_chat_chrome(&ui, &st);
                if to == 6 {
                    apply_members(&ui, &st);
                }
            }
        }
    });

    ui.on_open_settings({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            {
                let mut st = state.lock().unwrap();
                st.before_settings = ui.get_screen();
                ui.set_mcp_available(st.conn.is_some());
            }
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
                                let groups: Vec<Group> =
                                    rows.iter().map(|(g, _)| g.clone()).collect();
                                let unread: Vec<Unread> = rows
                                    .iter()
                                    .map(|(g, n)| Unread {
                                        group_id: g.id.clone(),
                                        read_at: None,
                                        unread: *n,
                                        last_activity_at: None,
                                    })
                                    .collect();
                                store_groups(&mut st, groups, unread);
                                apply_sessions(&ui, &st);
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
            if to != 2 {
                ui.set_working_row("".into());
                ui.set_reconnect_banner("".into());
            }
        }
    });

    ui.on_toggle_process({
        let weak = ui.as_weak();
        let state = state.clone();
        move |task_id: slint::SharedString| {
            let ui = weak.unwrap();
            let mut st = state.lock().unwrap();
            for item in &mut st.chat {
                if let chat::ChatItem::Status(c) = item {
                    if c.task_id == task_id.as_str() && c.expandable() {
                        c.expanded = !c.expanded;
                    }
                }
            }
            render_chat(&ui, &st);
        }
    });

    ui.on_open_group_menu({
        let weak = ui.as_weak();
        move || {
            let ui = weak.unwrap();
            ui.set_self_menu(false);
            ui.set_group_menu(true);
        }
    });
    ui.on_close_menus({
        let weak = ui.as_weak();
        move || close_menus(&weak.unwrap())
    });
    ui.on_open_group_profile({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            close_menus(&ui);
            let st = state.lock().unwrap();
            let owned_dm = avatars::dm_bro(&st.bros, &st.chat_group)
                .is_some_and(|b| avatars::owns(st.me.as_ref(), b));
            drop(st);
            if owned_dm {
                open_avatar(&ui, &state);
                return;
            }
            {
                let st = state.lock().unwrap();
                apply_chat_chrome(&ui, &st);
            }
            ui.set_screen(6);
        }
    });
    ui.on_open_find({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            close_menus(&ui);
            {
                let mut st = state.lock().unwrap();
                st.find_query.clear();
                st.find_hits.clear();
                apply_find(&ui, &st);
            }
            ui.set_screen(7);
        }
    });
    ui.on_find_edited({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let q = ui.get_find_query().to_string();
            let Some(conn) = state.lock().unwrap().conn.clone() else {
                return;
            };
            let id = state.lock().unwrap().chat_group.clone();
            if q.trim().is_empty() {
                let mut st = state.lock().unwrap();
                st.find_query = q;
                st.find_hits.clear();
                apply_find(&ui, &st);
                return;
            }
            ui.set_find_busy(true);
            let weak = ui.as_weak();
            let state = state.clone();
            std::thread::spawn(move || {
                let result = conn.search_group(&id, &q);
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    let mut st = state.lock().unwrap();
                    st.find_query = q;
                    st.find_hits = result.map(|s| s.results).unwrap_or_default();
                    apply_find(&ui, &st);
                });
            });
        }
    });
    ui.on_open_mcp({
        let weak = ui.as_weak();
        let state = state.clone();
        move || open_mcp(&weak.unwrap(), &state)
    });
    ui.on_mcp_pick_filter({
        let weak = ui.as_weak();
        let state = state.clone();
        move |i| {
            let mut st = state.lock().unwrap();
            st.mcp_book.set_filter(mcp::filter_from_index(i));
            apply_mcp(&weak.unwrap(), &st);
        }
    });
    ui.on_mcp_connect_server({
        let weak = ui.as_weak();
        let state = state.clone();
        move |id| {
            let id = id.to_string();
            let ui = weak.unwrap();
            let (conn, step) = {
                let mut st = state.lock().unwrap();
                let step = st.mcp_book.connect_to_send(&id);
                apply_mcp(&ui, &st);
                (st.conn.clone(), step)
            };
            let (Some(conn), Some(step)) = (conn, step) else {
                return;
            };
            let weak = ui.as_weak();
            let state = state.clone();
            std::thread::spawn(move || {
                use robroz_ui::cards::mcp::ConnectStep;
                let enabled = match step {
                    ConnectStep::Enable => Some(conn.set_mcp_enabled(&id, true)),
                    ConnectStep::Test => None,
                };
                let test = match &enabled {
                    Some(Err(_)) => None,
                    _ => Some(conn.test_mcp(&id)),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    let mut st = state.lock().unwrap();
                    match enabled {
                        Some(Ok(server)) => st.mcp_book.connect_enabled(&server),
                        Some(Err(e)) => st.mcp_book.entry_failed(&id, &e.0),
                        None => {}
                    }
                    if let Some(result) = test {
                        mcp_tested(&mut st, &id, result);
                    }
                    apply_mcp(&ui, &st);
                });
            });
        }
    });
    ui.on_mcp_approve({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let (conn, id) = {
                let mut st = state.lock().unwrap();
                let id = st
                    .mcp_prompt
                    .clone()
                    .and_then(|id| st.mcp_book.approve_command_to_send(&id));
                apply_mcp(&ui, &st);
                (st.conn.clone(), id)
            };
            let (Some(conn), Some(id)) = (conn, id) else {
                return;
            };
            let weak = ui.as_weak();
            let state = state.clone();
            std::thread::spawn(move || {
                let approved = conn.approve_mcp_command(&id);
                let test = approved.is_ok().then(|| conn.test_mcp(&id));
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    let mut st = state.lock().unwrap();
                    match approved {
                        Ok(server) => {
                            st.mcp_book.command_approved(server);
                            st.mcp_prompt = None;
                        }
                        Err(e) => st.mcp_book.approve_command_failed(&id, &e.0),
                    }
                    if let Some(result) = test {
                        mcp_tested(&mut st, &id, result);
                    }
                    apply_mcp(&ui, &st);
                });
            });
        }
    });
    ui.on_mcp_cancel_prompt({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let mut st = state.lock().unwrap();
            if let Some(id) = st.mcp_prompt.take() {
                st.mcp_book.cancel_command_prompt(&id);
            }
            apply_mcp(&weak.unwrap(), &st);
        }
    });
    ui.on_open_members({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            close_menus(&ui);
            let Some(conn) = state.lock().unwrap().conn.clone() else {
                return;
            };
            let id = state.lock().unwrap().chat_group.clone();
            ui.set_screen(8);
            let weak = ui.as_weak();
            let state = state.clone();
            std::thread::spawn(move || {
                let result = conn.group_members(&id);
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    let mut st = state.lock().unwrap();
                    st.members = result.ok();
                    apply_members(&ui, &st);
                });
            });
        }
    });
    ui.on_start_edit({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let mut st = state.lock().unwrap();
            st.group_edit = current_group(&st)
                .map(|g| g.name.clone())
                .unwrap_or_else(|| st.chat_name.clone());
            st.group_editing = true;
            st.group_note = None;
            apply_chat_chrome(&ui, &st);
        }
    });
    ui.on_save_edit({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let name = ui.get_group_edit().to_string();
            let Some(conn) = state.lock().unwrap().conn.clone() else {
                return;
            };
            let id = state.lock().unwrap().chat_group.clone();
            ui.set_group_edit_saving(true);
            let weak = ui.as_weak();
            let state = state.clone();
            std::thread::spawn(move || {
                let result = conn.rename_group(&id, &name);
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    ui.set_group_edit_saving(false);
                    let mut st = state.lock().unwrap();
                    match result {
                        Ok(g) => {
                            if let Some(old) = st.groups.iter_mut().find(|x| x.id == g.id) {
                                *old = g.clone();
                            }
                            st.chat_name = g.name.clone();
                            st.group_editing = false;
                            st.group_note = None;
                            ui.set_chat_title(g.name.into());
                            apply_sessions(&ui, &st);
                            apply_chat_chrome(&ui, &st);
                        }
                        Err(e) => {
                            st.group_note = Some(e.0);
                            apply_chat_chrome(&ui, &st);
                        }
                    }
                });
            });
        }
    });
    ui.on_copy_id({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            close_menus(&ui);
            let id = state.lock().unwrap().chat_group.clone();
            copy_to_clipboard(&id);
            let mut st = state.lock().unwrap();
            st.copied_until = Some(now_unix() + 2);
            apply_chat_chrome(&ui, &st);
        }
    });
    ui.on_ask_delete({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            close_menus(&ui);
            let st = state.lock().unwrap();
            apply_chat_chrome(&ui, &st);
            ui.set_delete_open(true);
        }
    });
    ui.on_cancel_delete({
        let weak = ui.as_weak();
        move || weak.unwrap().set_delete_open(false)
    });
    ui.on_confirm_delete({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let Some(conn) = state.lock().unwrap().conn.clone() else {
                return;
            };
            let id = state.lock().unwrap().chat_group.clone();
            let weak = ui.as_weak();
            let state = state.clone();
            std::thread::spawn(move || {
                let result = conn.delete_group(&id);
                let _ = slint::invoke_from_event_loop(move || {
                    let ui = weak.unwrap();
                    let mut st = state.lock().unwrap();
                    match result {
                        Ok(()) => {
                            st.groups.retain(|g| g.id != id);
                            st.unread.retain(|u| u.group_id != id);
                            st.chat_group.clear();
                            st.chat.clear();
                            ui.set_delete_open(false);
                            apply_sessions(&ui, &st);
                            ui.set_screen(1);
                        }
                        Err(e) => {
                            st.group_note = Some(e.0);
                            ui.set_delete_open(false);
                            apply_chat_chrome(&ui, &st);
                        }
                    }
                });
            });
        }
    });
    ui.on_open_self_menu({
        let weak = ui.as_weak();
        move || {
            let ui = weak.unwrap();
            ui.set_group_menu(false);
            ui.set_self_menu(true);
        }
    });
    ui.on_sign_out({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            close_menus(&ui);
            let mut st = state.lock().unwrap();
            if !st.token_from_env {
                st.token_saved = None;
            }
            st.token_dirty = false;
            st.conn = None;
            st.groups.clear();
            st.unread.clear();
            st.bros.clear();
            st.me = None;
            st.chat.clear();
            st.chat_group.clear();
            apply_sessions(&ui, &st);
            apply_token_field(&ui, &st);
            drop(st);
            ui.set_error_title("".into());
            ui.set_error("".into());
            ui.set_sessions(ModelRc::new(VecModel::from(Vec::<SessionRow>::new())));
            ui.set_screen(0);
        }
    });
    ui.on_home_search_edited({
        let weak = ui.as_weak();
        let state = state.clone();
        move || {
            let ui = weak.unwrap();
            let mut st = state.lock().unwrap();
            st.home_query = ui.get_home_query().to_string();
            apply_sessions(&ui, &st);
        }
    });
    ui.on_open_hit({
        let weak = ui.as_weak();
        move |_id| {
            // Jump-to-message is not wired yet; close find and return to the chat.
            let ui = weak.unwrap();
            ui.set_screen(2);
        }
    });

    let clock = slint::Timer::default();
    clock.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(1),
        {
            let weak = ui.as_weak();
            let state = state.clone();
            move || {
                let Some(ui) = weak.upgrade() else { return };
                if ui.get_screen() != 2 && ui.get_screen() != 6 {
                    return;
                }
                let mut st = state.lock().unwrap();
                if st.copied_until.is_some_and(|u| now_unix() >= u) {
                    st.copied_until = None;
                    apply_chat_chrome(&ui, &st);
                }
                if ui.get_screen() != 2 {
                    return;
                }
                let now = robroz_ui::process::now_unix();
                let ticking = st.poll_fail_since.is_some()
                    || st.chat.iter().any(|item| {
                        matches!(
                            item,
                            chat::ChatItem::Status(c)
                                if c.timer(now).is_running()
                        )
                    });
                if ticking {
                    render_chat(&ui, &st);
                }
            }
        },
    );
    std::mem::forget(clock);

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
