//! Connection form on the offline pane and in Settings.
//!
//! The token value never appears here as plaintext from storage: a saved
//! token is shown as eight bullets, and an untouched field sends
//! [`TokenEdit::Keep`].

use std::fmt;

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Padding, Theme as IcedTheme,
};
use robroz_ui::i18n::Key as Text;
use robroz_ui::view::{self, color};
use robroz_ui::Skin;

use super::{Source, TokenEdit, TokenState, DEFAULT_HOST};

/// Eight bullets: the only stand-in for a saved token.
const MASK: &str = "••••••••";
/// Error line under the Connect button (UI spec).
const ERROR_SIZE: f32 = 13.0;

/// User actions from the connection form.
#[derive(Clone)]
pub enum Event {
    Host(String),
    Token(String),
    ToggleShow,
    Clear,
    Connect,
}

impl fmt::Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Host(host) => f.debug_tuple("Host").field(host).finish(),
            Self::Token(_) => f.debug_tuple("Token").field(&"***").finish(),
            Self::ToggleShow => f.write_str("ToggleShow"),
            Self::Clear => f.write_str("Clear"),
            Self::Connect => f.write_str("Connect"),
        }
    }
}

/// A failed Connect, shown under the button. Never contains the token.
#[derive(Debug, Clone)]
pub enum FormError {
    Conn(super::ConnError),
    Save(String),
}

impl FormError {
    pub fn line(&self) -> String {
        match self {
            Self::Conn(err) => err.to_string(),
            Self::Save(err) => err.clone(),
        }
    }
}

/// Draft state for the Host / Token fields.
pub struct ConnForm {
    host: String,
    host_source: Source,
    token: TokenState,
    token_draft: String,
    token_dirty: bool,
    clear_token: bool,
    show_token: bool,
    pub connecting: bool,
    pub error: Option<FormError>,
}

impl fmt::Debug for ConnForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConnForm")
            .field("host", &self.host)
            .field("host_source", &self.host_source)
            .field("token", &self.token)
            .field(
                "token_draft",
                &if self.token_draft.is_empty() {
                    ""
                } else {
                    "***"
                },
            )
            .field("token_dirty", &self.token_dirty)
            .field("clear_token", &self.clear_token)
            .field("show_token", &self.show_token)
            .field("connecting", &self.connecting)
            .field("error", &self.error)
            .finish()
    }
}

impl ConnForm {
    pub fn from_config(cfg: super::ConnConfig) -> Self {
        Self {
            host: cfg.host,
            host_source: cfg.host_source,
            token: cfg.token,
            token_draft: String::new(),
            token_dirty: false,
            clear_token: false,
            show_token: false,
            connecting: false,
            error: None,
        }
    }

    /// Reload host and token state from [`super::load`] after a Connect.
    /// Keeps a connection error so the line still shows.
    pub fn reload(&mut self) {
        let cfg = super::load();
        self.host = cfg.host;
        self.host_source = cfg.host_source;
        self.token = cfg.token;
        self.token_draft.clear();
        self.token_dirty = false;
        self.clear_token = false;
        self.show_token = false;
    }

    pub fn host_for_save(&self) -> String {
        let trimmed = self.host.trim();
        if trimmed.is_empty() {
            DEFAULT_HOST.to_string()
        } else {
            trimmed.to_string()
        }
    }

    pub fn host_locked(&self) -> bool {
        self.host_source == Source::Env
    }

    pub fn token_locked(&self) -> bool {
        self.token == TokenState::Env
    }

    /// Both fields are set by the environment: Connect would write nothing.
    pub fn both_env_locked(&self) -> bool {
        self.host_locked() && self.token_locked()
    }

    pub fn connect_enabled(&self) -> bool {
        !self.connecting && !self.both_env_locked()
    }

    pub fn set_host(&mut self, host: String) {
        if !self.host_locked() {
            self.host = host;
        }
    }

    pub fn set_token(&mut self, value: String) {
        if self.token_locked() {
            return;
        }
        if !self.token_dirty {
            if value == MASK {
                return;
            }
            let next = value.strip_prefix(MASK).unwrap_or(&value).to_string();
            self.token_dirty = true;
            self.clear_token = false;
            self.token_draft = next;
            return;
        }
        self.token_draft = value;
    }

    pub fn request_clear(&mut self) {
        if self.token_locked() {
            return;
        }
        self.token_draft.clear();
        self.token_dirty = false;
        self.clear_token = true;
        self.show_token = false;
    }

    pub fn toggle_show(&mut self) {
        if self.token_locked() {
            return;
        }
        self.show_token = !self.show_token;
    }

    /// What `save` should do with the token. Env-locked tokens always Keep.
    pub fn token_edit(&self) -> TokenEdit {
        if self.token_locked() {
            return TokenEdit::Keep;
        }
        if self.token_dirty {
            let trimmed = self.token_draft.trim();
            if trimmed.is_empty() {
                if self.clear_token || matches!(self.token, TokenState::Saved) {
                    TokenEdit::Clear
                } else {
                    TokenEdit::Keep
                }
            } else {
                TokenEdit::Set(trimmed.to_string())
            }
        } else if self.clear_token {
            TokenEdit::Clear
        } else {
            TokenEdit::Keep
        }
    }

    fn token_value(&self) -> &str {
        if self.token_dirty {
            &self.token_draft
        } else if self.clear_token {
            ""
        } else if matches!(self.token, TokenState::Saved | TokenState::Env) {
            MASK
        } else {
            ""
        }
    }

    fn shows_clear(&self) -> bool {
        !self.token_locked()
            && (self.token_dirty && !self.token_draft.is_empty()
                || !self.token_dirty && !self.clear_token && self.token == TokenState::Saved)
    }

    fn shows_show(&self) -> bool {
        !self.token_locked() && (self.token_dirty || self.token == TokenState::None)
    }
}

/// Whether `host` is a remote `http://` address (not loopback).
pub fn is_insecure_remote(host: &str) -> bool {
    let host = host.trim();
    let Some(rest) = host.strip_prefix("http://") else {
        return false;
    };
    let rest = rest.split_once('@').map(|(_, h)| h).unwrap_or(rest);
    let hostname = if let Some(rest) = rest.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        rest.split(['/', ':', '?']).next().unwrap_or("")
    };
    let hostname = hostname.trim_end_matches('.').to_ascii_lowercase();
    !matches!(hostname.as_str(), "127.0.0.1" | "localhost" | "::1")
}

/// The connection form, as wide as its parent (Language row width).
/// `titled` adds a Model-card-style heading — Settings only.
pub fn view<'a>(t: &Skin, form: &'a ConnForm, titled: bool) -> Element<'a, Event> {
    let pad = Padding::from([t.metrics.row_pad_h * 0.75, t.metrics.row_pad_h]);
    let mut fields = column![].spacing(10);

    let mut host = text_input(DEFAULT_HOST, &form.host)
        .size(t.type_.body)
        .padding(field_padding())
        .width(Fill)
        .style(field_style(*t));
    if !form.host_locked() && !form.connecting {
        host = host.on_input(Event::Host).on_submit(Event::Connect);
    } else if !form.host_locked() {
        host = host.on_input(Event::Host);
    }
    let mut host_block = column![field_label(t, Text::ConnHostLabel.as_str()), host].spacing(2.0);
    if form.host_locked() {
        host_block = host_block.push(caption(t, Text::ConnSetByHost.as_str()));
    }
    if is_insecure_remote(&form.host) {
        host_block = host_block.push(caption(t, Text::ConnUnencryptedHint.as_str()));
    }
    fields = fields.push(host_block);

    let token_value = form.token_value();
    let mut token = text_input("", token_value)
        .secure(!form.show_token)
        .size(t.type_.body)
        .padding(field_padding())
        .width(Fill)
        .style(field_style(*t));
    if !form.token_locked() && !form.connecting {
        token = token.on_input(Event::Token).on_submit(Event::Connect);
    } else if !form.token_locked() {
        token = token.on_input(Event::Token);
    }
    let mut token_row = row![token]
        .spacing(t.controls.button_gap)
        .align_y(Alignment::Center);
    if form.shows_show() {
        let label = if form.show_token {
            Text::ConnHide
        } else {
            Text::ConnShow
        };
        token_row = token_row.push(text_action(t, label.as_str(), Some(Event::ToggleShow)));
    }
    if form.shows_clear() {
        token_row = token_row.push(text_action(
            t,
            Text::ConnClear.as_str(),
            (!form.connecting).then_some(Event::Clear),
        ));
    }
    let mut token_block =
        column![field_label(t, Text::ConnTokenLabel.as_str()), token_row].spacing(2.0);
    if form.token_locked() {
        token_block = token_block.push(caption(t, Text::ConnSetByToken.as_str()));
    }
    fields = fields.push(token_block);

    let (btn_label, btn_enabled) = if form.connecting {
        (Text::ConnConnecting.as_str(), false)
    } else {
        (Text::ConnConnect.as_str(), form.connect_enabled())
    };
    let mut action = column![connect_button(
        t,
        btn_label,
        btn_enabled.then_some(Event::Connect),
    )]
    .spacing(10);
    if let Some(err) = &form.error {
        action = action.push(
            text(err.line())
                .size(ERROR_SIZE)
                .color(color(t.palette.accent)),
        );
    }
    // 12px between the Token block and Connect (UI review).
    let body = column![
        fields,
        container(action).padding(Padding {
            top: 12.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0
        }),
    ];

    let fields = container(body).padding(pad).width(Fill);
    let card: Element<'a, Event> = if titled {
        column![section_heading(t), divider(t), fields].into()
    } else {
        fields.into()
    };
    container(card)
        .width(Fill)
        .style(view::card_style(*t))
        .into()
}

fn section_heading<'a>(t: &Skin) -> Element<'a, Event> {
    let m = t.metrics;
    let head = row![text(Text::ConnSectionTitle)
        .size(t.type_.list_title)
        .font(view::bold())
        .color(color(t.palette.text))]
    .align_y(Alignment::Center);
    container(head)
        .height(Length::Fixed(m.panel_head_height))
        .padding(Padding::from([0.0, m.row_pad_h]))
        .align_y(Alignment::Center)
        .into()
}

fn divider<'a>(t: &Skin) -> Element<'a, Event> {
    let c = t.palette.divider;
    container(
        container(Space::new().width(Fill).height(t.stroke.divider)).style(move |_| {
            container::Style {
                background: Some(color(c).into()),
                ..container::Style::default()
            }
        }),
    )
    .width(Fill)
    .into()
}

fn field_label<'a>(t: &Skin, label: &'static str) -> Element<'a, Event> {
    text(label)
        .size(t.type_.foot)
        .color(color(t.palette.text3))
        .into()
}

fn caption<'a>(t: &Skin, line: &'static str) -> Element<'a, Event> {
    text(line)
        .size(t.type_.tag)
        .color(color(t.palette.text3))
        .into()
}

fn field_padding() -> Padding {
    Padding::from([8.0, 10.0])
}

fn field_style(t: Skin) -> impl Fn(&IcedTheme, text_input::Status) -> text_input::Style {
    move |theme, status| {
        let border = match status {
            text_input::Status::Focused { .. } => t.palette.focus,
            text_input::Status::Hovered => t.palette.text,
            text_input::Status::Active | text_input::Status::Disabled => t.palette.border,
        };
        let disabled = matches!(status, text_input::Status::Disabled);
        let alpha = if disabled {
            t.palette.disabled_alpha
        } else {
            1.0
        };
        let sel = t.palette.selection;
        text_input::Style {
            background: theme.extended_palette().background.base.color.into(),
            border: Border {
                color: color(border),
                width: t.stroke.control,
                radius: 2.0.into(),
            },
            icon: fade(t.palette.text, alpha),
            placeholder: fade(t.palette.text3, alpha),
            value: fade(t.palette.text, alpha),
            selection: fade(sel.rgb, sel.alpha),
        }
    }
}

fn fade(c: [u8; 3], alpha: f32) -> Color {
    let mut c = color(c);
    c.a = alpha;
    c
}

fn text_action<'a>(t: &Skin, label: &'static str, on_press: Option<Event>) -> Element<'a, Event> {
    let mut btn = button(text(label).size(t.type_.foot).color(color(t.palette.text)))
        .padding(Padding::from([4.0, 8.0]))
        .style(view::nav_button_style(*t, false));
    if let Some(msg) = on_press {
        btn = btn.on_press(msg);
    }
    btn.into()
}

fn connect_button<'a>(
    t: &Skin,
    label: &'static str,
    on_press: Option<Event>,
) -> Element<'a, Event> {
    let t = *t;
    let enabled = on_press.is_some();
    let alpha = if enabled {
        1.0
    } else {
        t.palette.disabled_alpha
    };
    let label = text(label)
        .size(t.type_.button)
        .font(view::semibold())
        .color(fade(t.palette.on_accent, alpha));
    // Same height as Settings Save (`button_h` = 32). Centre the label
    // inside a padded container so the glyphs don't sit high.
    let visual = container(label)
        .height(Length::Fixed(t.controls.button_h))
        .padding(Padding::from([0.0, t.controls.button_px]))
        .align_y(Alignment::Center);
    let mut btn = button(visual)
        .padding(0)
        .height(Length::Fixed(t.controls.button_h))
        .style(move |_, status: button::Status| {
            let fill = match status {
                button::Status::Hovered | button::Status::Pressed => t.palette.accent_hover,
                _ => t.palette.accent,
            };
            button::Style {
                background: Some(Background::Color(fade(fill, alpha))),
                text_color: fade(t.palette.on_accent, alpha),
                border: Border {
                    radius: t.radii.button.into(),
                    ..Border::default()
                },
                ..button::Style::default()
            }
        });
    if let Some(msg) = on_press {
        btn = btn.on_press(msg);
    }
    btn.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conn::{ConnConfig, TokenState};

    fn form(host: &str, host_source: Source, token: TokenState) -> ConnForm {
        ConnForm::from_config(ConnConfig {
            host: host.to_string(),
            host_source,
            token,
        })
    }

    #[test]
    fn untouched_saved_token_sends_keep() {
        let f = form(DEFAULT_HOST, Source::Default, TokenState::Saved);
        assert!(matches!(f.token_edit(), TokenEdit::Keep));
        assert_eq!(f.token_value(), MASK);
    }

    #[test]
    fn typed_token_sends_set_without_the_mask() {
        let mut f = form(DEFAULT_HOST, Source::Default, TokenState::Saved);
        f.set_token(format!("{MASK}secret"));
        match f.token_edit() {
            TokenEdit::Set(value) => assert_eq!(value, "secret"),
            other => panic!("expected Set, got {other:?}"),
        }
        assert!(!format!("{f:?}").contains("secret"));
    }

    #[test]
    fn clear_sends_clear() {
        let mut f = form(DEFAULT_HOST, Source::Default, TokenState::Saved);
        f.request_clear();
        assert!(matches!(f.token_edit(), TokenEdit::Clear));
        assert_eq!(f.token_value(), "");
    }

    #[test]
    fn env_token_is_keep_even_after_edits() {
        let mut f = form(DEFAULT_HOST, Source::Default, TokenState::Env);
        f.set_token("should-be-ignored".into());
        f.request_clear();
        assert!(matches!(f.token_edit(), TokenEdit::Keep));
        assert_eq!(f.token_value(), MASK);
        assert!(f.token_locked());
    }

    #[test]
    fn empty_host_saves_as_the_default() {
        let mut f = form("", Source::Default, TokenState::None);
        f.set_host("  ".into());
        assert_eq!(f.host_for_save(), DEFAULT_HOST);
    }

    #[test]
    fn insecure_remote_is_non_loopback_http() {
        assert!(!is_insecure_remote("http://127.0.0.1:8790"));
        assert!(!is_insecure_remote("http://localhost:8790"));
        assert!(!is_insecure_remote("http://[::1]:8790"));
        assert!(!is_insecure_remote("https://example.invalid:8790"));
        assert!(is_insecure_remote("http://example.invalid:8790"));
        assert!(is_insecure_remote("http://192.168.1.8:8790"));
        assert!(!is_insecure_remote("example.invalid"));
    }

    #[test]
    fn connect_is_disabled_when_both_fields_are_env_locked() {
        let both = form(DEFAULT_HOST, Source::Env, TokenState::Env);
        assert!(both.both_env_locked());
        assert!(!both.connect_enabled());

        let host_only = form(DEFAULT_HOST, Source::Env, TokenState::None);
        assert!(!host_only.both_env_locked());
        assert!(host_only.connect_enabled());

        let token_only = form(DEFAULT_HOST, Source::Default, TokenState::Env);
        assert!(!token_only.both_env_locked());
        assert!(token_only.connect_enabled());

        let mut connecting = form(DEFAULT_HOST, Source::Default, TokenState::None);
        connecting.connecting = true;
        assert!(!connecting.connect_enabled());
    }

    #[test]
    fn section_title_is_connection_in_both_locales() {
        use robroz_ui::i18n::{text_in, Locale};
        assert_eq!(text_in(Locale::En, Text::ConnSectionTitle), "Connection");
        assert_eq!(text_in(Locale::Zh, Text::ConnSectionTitle), "连接");
    }

    #[test]
    fn event_debug_hides_the_token() {
        let ev = Event::Token("dummy-token-for-fixture-only".into());
        let dbg = format!("{ev:?}");
        assert!(!dbg.contains("dummy-token"));
        assert!(dbg.contains("***"));
    }
}
