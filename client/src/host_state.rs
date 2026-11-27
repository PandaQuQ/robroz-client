//! Whether the host has answered yet, and what the window shows until it
//! has. The empty-account state (「还没有 bro」 in the chat list, 「选一个聊天」
//! in the main pane) only makes sense once the host has listed the groups;
//! before that the sidebar stays empty.

use robroz_ui::i18n::Key as Text;

/// Where the first group listing stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostState {
    /// The first listing is still on its way.
    Waiting,
    /// The first listing failed: the host can't be reached.
    Unreachable,
    /// The host listed the groups at least once.
    Answered,
    /// The person signed out on this device: show the connection form,
    /// do not retry the previous token, and do not show a host-error headline.
    SignedOut,
}

impl HostState {
    /// `loaded`: the host listed the groups at least once. `load_failed`:
    /// the latest attempt at that first listing failed.
    pub fn of(loaded: bool, load_failed: bool) -> Self {
        Self::of_session(loaded, load_failed, false)
    }

    pub fn of_session(loaded: bool, load_failed: bool, signed_out: bool) -> Self {
        if signed_out {
            return Self::SignedOut;
        }
        match (loaded, load_failed) {
            (true, _) => Self::Answered,
            (false, true) => Self::Unreachable,
            (false, false) => Self::Waiting,
        }
    }

    /// The chat list and page buttons show only once the host answered.
    pub fn shows_sidebar(self) -> bool {
        self == Self::Answered
    }

    /// The main pane's headline before the host answered. A rejected token
    /// uses [`Text::ClientTokenRejectedTitle`]; any other failure uses
    /// [`Text::ClientHostUnreachable`]. The address sits on a lighter line
    /// below; the reason is shown once, under the connection form. `None`
    /// once answered, and while still waiting (the pane stays blank).
    pub fn notice(self, err: Option<&str>) -> Option<Text> {
        if self != Self::Unreachable {
            return None;
        }
        Some(if err.is_some_and(is_token_rejected) {
            Text::ClientTokenRejectedTitle
        } else {
            Text::ClientHostUnreachable
        })
    }

    /// Retry the first listing on the poll interval while unreachable.
    pub fn retries(self) -> bool {
        self == Self::Unreachable
    }

    /// The Language row sits on this pane, so the language can still be
    /// changed when the host has never answered (the Settings page lives
    /// in the sidebar, which is hidden here). The control is the same
    /// one as on the Model page.
    pub fn shows_language_row(self) -> bool {
        matches!(self, Self::Unreachable | Self::SignedOut)
    }
}

/// Short reason shown next to the host address on the disconnected pane.
/// Maps common transport errors from ureq / `std::io`; anything else is
/// omitted so the pane never shows `io:` or `os error`.
pub fn connect_reason(err: &str) -> Option<Text> {
    if is_token_rejected(err) {
        return Some(Text::ClientTokenRejected);
    }
    let s = err.to_ascii_lowercase();
    if s.contains("connection refused") {
        Some(Text::ClientConnectionRefused)
    } else if s.contains("timed out") || s.contains("timed-out") || s.contains("timeout") {
        Some(Text::ClientTimedOut)
    } else if is_host_not_found(&s) {
        Some(Text::ClientHostNotFound)
    } else {
        None
    }
}

pub(crate) fn is_token_rejected(err: &str) -> bool {
    let low = err.to_ascii_lowercase();
    err.contains(robroz_contract::host_text::TOKEN_REJECTED)
        || err.contains("The host rejected the token")
        || err.contains("宿主拒绝了令牌")
        || low.contains("returned 401")
        || low.contains(" 返回 401")
}

fn is_host_not_found(s: &str) -> bool {
    s.contains("host not found")
        || s.contains("failed to lookup")
        || s.contains("name or service not known")
        || s.contains("nodename nor servname")
        || s.contains("temporary failure in name resolution")
        || s.contains("no such host")
        || s.contains("dns error")
        || s.contains("could not resolve")
        || s.contains("failed to resolve")
}

#[cfg(test)]
mod tests {
    use super::HostState;
    use robroz_ui::i18n::{scoped, text_in, Key as Text, Locale};

    #[test]
    fn empty_account_waits_for_the_host_to_answer() {
        let waiting = HostState::of(false, false);
        assert_eq!(waiting, HostState::Waiting);
        assert!(!waiting.shows_sidebar());
        assert_eq!(waiting.notice(None), None);
        assert!(!waiting.shows_language_row());

        let down = HostState::of(false, true);
        assert_eq!(down, HostState::Unreachable);
        assert!(!down.shows_sidebar(), "no 「还没有 bro」 while unreachable");
        assert_eq!(down.notice(None), Some(Text::ClientHostUnreachable));
        assert!(down.retries());
        assert!(down.shows_language_row());

        // Once the host answered, a later failure keeps the account on
        // screen; the footer carries the error.
        for failed in [false, true] {
            let up = HostState::of(true, failed);
            assert_eq!(up, HostState::Answered);
            assert!(up.shows_sidebar());
            assert_eq!(up.notice(None), None);
            assert!(!up.shows_language_row());
            assert!(!up.retries());
        }
    }

    #[test]
    fn sign_out_shows_the_form_without_retry() {
        let out = HostState::of_session(true, false, true);
        assert_eq!(out, HostState::SignedOut);
        assert!(!out.shows_sidebar());
        assert_eq!(out.notice(None), None);
        assert!(out.shows_language_row());
        assert!(!out.retries());
    }

    #[test]
    fn unreachable_copy_in_both_locales() {
        assert_eq!(
            text_in(Locale::En, Text::ClientHostUnreachable),
            "Can't reach the host"
        );
        assert_eq!(
            text_in(Locale::Zh, Text::ClientHostUnreachable),
            "连不上宿主"
        );
        let _en = scoped(Locale::En);
        assert_eq!(
            HostState::Unreachable.notice(None).map(|k| k.as_str()),
            Some("Can't reach the host")
        );
    }

    #[test]
    fn connect_reason_maps_common_io_errors() {
        use super::connect_reason;

        let refused =
            "Can't reach the host http://127.0.0.1:1: io: Connection refused (os error 111)";
        assert_eq!(connect_reason(refused), Some(Text::ClientConnectionRefused));
        assert_eq!(
            connect_reason("io: Connection refused (os error 10061)"),
            Some(Text::ClientConnectionRefused)
        );
        assert_eq!(
            text_in(Locale::En, Text::ClientConnectionRefused),
            "Connection refused"
        );
        assert_eq!(
            text_in(Locale::Zh, Text::ClientConnectionRefused),
            "连接被拒绝"
        );

        assert_eq!(
            connect_reason("io: Connection timed out (os error 110)"),
            Some(Text::ClientTimedOut)
        );
        assert_eq!(
            connect_reason("timeout: Global"),
            Some(Text::ClientTimedOut)
        );
        assert_eq!(text_in(Locale::En, Text::ClientTimedOut), "Timed out");
        assert_eq!(text_in(Locale::Zh, Text::ClientTimedOut), "超时");

        assert_eq!(
            connect_reason("host not found"),
            Some(Text::ClientHostNotFound)
        );
        assert_eq!(
            connect_reason("io: failed to lookup address information: Name or service not known"),
            Some(Text::ClientHostNotFound)
        );
        assert_eq!(
            connect_reason("nodename nor servname provided, or not known"),
            Some(Text::ClientHostNotFound)
        );
        assert_eq!(
            text_in(Locale::En, Text::ClientHostNotFound),
            "Host not found"
        );
        assert_eq!(text_in(Locale::Zh, Text::ClientHostNotFound), "找不到主机");

        assert_eq!(
            connect_reason("The host rejected the token. Check ROBROZ_TOKEN."),
            Some(Text::ClientTokenRejected)
        );
        assert_eq!(
            connect_reason("宿主拒绝了令牌，请检查 ROBROZ_TOKEN。"),
            Some(Text::ClientTokenRejected)
        );
        assert_eq!(
            connect_reason(robroz_contract::host_text::TOKEN_REJECTED),
            Some(Text::ClientTokenRejected)
        );
        assert_eq!(
            connect_reason("GET http://127.0.0.1:8790/api/groups returned 401"),
            Some(Text::ClientTokenRejected)
        );
        assert_eq!(
            text_in(Locale::En, Text::ClientTokenRejected),
            "The host rejected the token. Check ROBROZ_TOKEN."
        );
        assert_eq!(
            text_in(Locale::Zh, Text::ClientTokenRejected),
            "宿主拒绝了令牌，请检查 ROBROZ_TOKEN。"
        );

        // Unknown transport errors are omitted, never echoed raw.
        for s in [
            "io: UnexpectedEof (os error 0)",
            "Can't reach the host http://127.0.0.1:1: io: Broken pipe (os error 32)",
            "http status: 500",
        ] {
            assert_eq!(connect_reason(s), None);
        }
        for key in [
            Text::ClientConnectionRefused,
            Text::ClientTimedOut,
            Text::ClientHostNotFound,
            Text::ClientTokenRejected,
        ] {
            let en = text_in(Locale::En, key);
            let zh = text_in(Locale::Zh, key);
            assert!(!en.contains("io:"), "{en}");
            assert!(!en.to_ascii_lowercase().contains("os error"), "{en}");
            assert!(!zh.contains("io:"), "{zh}");
            assert!(!zh.contains("os error"), "{zh}");
        }
    }

    #[test]
    fn offline_title_follows_the_error_type() {
        let down = HostState::Unreachable;
        assert_eq!(down.notice(None), Some(Text::ClientHostUnreachable));
        assert_eq!(
            down.notice(Some("io: Connection refused (os error 111)")),
            Some(Text::ClientHostUnreachable)
        );
        assert_eq!(
            down.notice(Some("GET http://127.0.0.1:8790/api/groups returned 401")),
            Some(Text::ClientTokenRejectedTitle)
        );
        assert_eq!(
            down.notice(Some("The host rejected the token. Check ROBROZ_TOKEN.")),
            Some(Text::ClientTokenRejectedTitle)
        );
        assert_eq!(
            text_in(Locale::En, Text::ClientTokenRejectedTitle),
            "Token rejected"
        );
        assert_eq!(
            text_in(Locale::Zh, Text::ClientTokenRejectedTitle),
            "令牌被拒绝"
        );
        let _en = scoped(Locale::En);
        assert_eq!(
            down.notice(Some("returned 401")).map(|k| k.as_str()),
            Some("Token rejected")
        );
    }

    #[test]
    fn connection_copy_has_no_markdown_backticks() {
        for key in [
            Text::ClientTokenRejected,
            Text::ClientTokenRejectedTitle,
            Text::ConnSetByHost,
            Text::ConnSetByToken,
            Text::ConnUnencryptedHint,
        ] {
            let en = text_in(Locale::En, key);
            let zh = text_in(Locale::Zh, key);
            assert!(!en.contains('`'), "{en}");
            assert!(!zh.contains('`'), "{zh}");
        }
        assert_eq!(
            text_in(Locale::En, Text::ClientTokenRejected),
            "The host rejected the token. Check ROBROZ_TOKEN."
        );
        assert_eq!(
            text_in(Locale::Zh, Text::ConnSetByHost),
            "由 ROBROZ_HOST 设置"
        );
        assert_eq!(
            text_in(Locale::Zh, Text::ConnSetByToken),
            "由 ROBROZ_TOKEN 设置"
        );
    }

    #[test]
    fn disconnected_pane_renders_the_shared_language_row() {
        // The pane in `page_view` must actually include the same Language
        // row as the Model page; a flag on HostState is not enough.
        let src = include_str!("main.rs");
        assert!(
            src.contains("state.shows_language_row()"),
            "page_view must gate the Language row on HostState"
        );
        assert_eq!(
            src.matches("self.language_row(t)").count(),
            2,
            "Model page and disconnected pane share language_row"
        );
        assert!(
            src.contains("Message::SetLocale") && src.contains("prefs::save(locale)"),
            "picking a language there persists through the same SetLocale path"
        );
    }
}
