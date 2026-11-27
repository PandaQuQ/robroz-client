//! One HTTP connection setup shared by the event source, the card API and
//! the live stream: base URL, timeouts and the optional host token.
//!
//! Token: `ROBROZ_TOKEN` for this run, otherwise the OS keychain, otherwise
//! the `token` file next to `lang` in the client config folder. Sent as
//! `Authorization: Bearer …` on every request when set (robroz#22), including
//! the live SSE stream. It is never printed: `Debug` hides it and error
//! texts only carry the base URL. A 401 is [`ConnError::Unauthorized`]
//! (shown as [`robroz_ui::i18n::Key::ClientTokenRejected`]). A host that
//! cannot be reached is [`ConnError::Unreachable`].
//!
//! The connection form uses [`load`] and [`save`]. After a successful
//! `save`, build a new [`Conn`] with [`Conn::from_env`].

use std::fmt;
use std::time::Duration;

use ureq::unversioned::transport::Connector;

mod config;
pub(crate) mod form;

pub use config::{load, save, ConnConfig, ConnError, SaveError, Source, TokenEdit, TokenState};

pub const DEFAULT_HOST: &str = "http://127.0.0.1:8790";

pub(super) fn normalize_host(base: &str) -> String {
    base.trim().trim_end_matches('/').to_string()
}

#[derive(Clone)]
pub struct Conn {
    base: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl fmt::Debug for Conn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Conn")
            .field("base", &self.base)
            .field("token", &self.token.as_ref().map(|_| "***"))
            .finish()
    }
}

fn agent(timeout: Option<Duration>) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(timeout)
        .timeout_connect(Some(Duration::from_secs(5)))
        .http_status_as_error(false)
        .build()
        .into()
}

/// Streaming agent: no overall timeout, but each socket read waits at
/// most `idle` (45s in production). A quiet stream then errors so the
/// reader can reconnect. Keepalives every 15s reset the wait.
fn streaming_agent(idle: Duration) -> ureq::Agent {
    let config = ureq::config::Config::builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .http_status_as_error(false)
        .build();
    let connector =
        ureq::unversioned::transport::DefaultConnector::default().chain(IdleLimit(idle));
    ureq::Agent::with_parts(
        config,
        connector,
        ureq::unversioned::resolver::DefaultResolver::default(),
    )
}

/// Caps each transport read at `0`, so a half-open SSE stream is dropped
/// after that idle period instead of hanging forever.
#[derive(Debug, Clone, Copy)]
struct IdleLimit(Duration);

impl<In: ureq::unversioned::transport::Transport> ureq::unversioned::transport::Connector<In>
    for IdleLimit
{
    type Out = IdleTransport<In>;

    fn connect(
        &self,
        _: &ureq::unversioned::transport::ConnectionDetails,
        chained: Option<In>,
    ) -> Result<Option<Self::Out>, ureq::Error> {
        Ok(chained.map(|inner| IdleTransport {
            inner,
            idle: self.0,
        }))
    }
}

#[derive(Debug)]
struct IdleTransport<T> {
    inner: T,
    idle: Duration,
}

fn cap_idle(
    timeout: ureq::unversioned::transport::NextTimeout,
    idle: Duration,
) -> ureq::unversioned::transport::NextTimeout {
    use ureq::unversioned::transport::time::Duration as UDuration;
    let cap: UDuration = idle.into();
    if timeout.after.is_not_happening() || timeout.after > cap {
        ureq::unversioned::transport::NextTimeout {
            after: cap,
            reason: timeout.reason,
        }
    } else {
        timeout
    }
}

impl<T: ureq::unversioned::transport::Transport> ureq::unversioned::transport::Transport
    for IdleTransport<T>
{
    fn buffers(&mut self) -> &mut dyn ureq::unversioned::transport::Buffers {
        self.inner.buffers()
    }

    fn transmit_output(
        &mut self,
        amount: usize,
        timeout: ureq::unversioned::transport::NextTimeout,
    ) -> Result<(), ureq::Error> {
        self.inner.transmit_output(amount, timeout)
    }

    fn await_input(
        &mut self,
        timeout: ureq::unversioned::transport::NextTimeout,
    ) -> Result<bool, ureq::Error> {
        self.inner.await_input(cap_idle(timeout, self.idle))
    }

    fn is_open(&mut self) -> bool {
        self.inner.is_open()
    }

    fn is_tls(&self) -> bool {
        self.inner.is_tls()
    }
}

impl Conn {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: normalize_host(&base.into()),
            token: None,
            agent: agent(Some(Duration::from_secs(5))),
        }
    }

    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.token = token
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
        self
    }

    /// Same host and token, no overall timeout: for the long-lived stream.
    /// Each read waits at most 45s with no bytes, then the stream errors
    /// so the client can reconnect.
    pub fn streaming(&self) -> Self {
        self.streaming_with_idle(Duration::from_secs(robroz_ui::process::SSE_IDLE_SECS))
    }

    /// [`streaming`] with a custom idle timeout (tests).
    pub fn streaming_with_idle(&self, idle: Duration) -> Self {
        Self {
            agent: streaming_agent(idle),
            ..self.clone()
        }
    }

    /// Whether this connection will send a Bearer token. The value itself
    /// is never exposed.
    #[cfg(test)]
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn auth<B>(&self, req: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        match &self.token {
            Some(t) => req.header("Authorization", &format!("Bearer {t}")),
            None => req,
        }
    }

    pub fn get(&self, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
        self.auth(self.agent.get(&self.url(path)))
    }

    pub fn post(&self, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
        self.auth(self.agent.post(&self.url(path)))
    }

    pub fn patch(&self, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
        self.auth(self.agent.patch(&self.url(path)))
    }

    pub fn put(&self, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithBody> {
        self.auth(self.agent.put(&self.url(path)))
    }

    pub fn delete(&self, path: &str) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
        self.auth(self.agent.delete(&self.url(path)))
    }
}

/// The client-facing 401 line (UI-approved). Never includes the token.
pub fn unauthorized_message() -> String {
    robroz_ui::i18n::Key::ClientTokenRejected.into()
}

/// Map a non-success HTTP response to a card or pane message. 401 always
/// becomes [`unauthorized_message`]; other statuses prefer the host's
/// `error` field when the body is JSON.
pub fn error_text(method: &str, url: &str, status: u16, body: &str) -> String {
    if status == 401 {
        return unauthorized_message();
    }
    #[derive(serde::Deserialize)]
    struct ErrorBody {
        error: String,
    }
    serde_json::from_str::<ErrorBody>(body)
        .ok()
        .map(|e| e.error)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| robroz_ui::i18n::Key::ClientHttpStatus.fill(&[&method, &url, &status]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_base_and_hides_token() {
        let c = Conn::new(" http://h:1/ ").with_token(Some(" sekrit-0123456789 ".into()));
        assert_eq!(c.base(), "http://h:1");
        assert_eq!(c.url("/api/unread"), "http://h:1/api/unread");
        let dbg = format!("{c:?}");
        assert!(!dbg.contains("sekrit"), "{dbg}");
        assert!(Conn::new("x").with_token(Some("  ".into())).token.is_none());
        assert!(c.has_token());
    }

    #[test]
    fn a_401_is_a_clear_token_message() {
        use robroz_ui::i18n::{text_in, Key, Locale};
        assert_eq!(
            error_text(
                "GET",
                "http://h/api/groups",
                401,
                r#"{"error":"缺少或错误的令牌"}"#
            ),
            unauthorized_message()
        );
        assert_eq!(
            unauthorized_message(),
            text_in(Locale::En, Key::ClientTokenRejected)
        );
        assert_eq!(
            text_in(Locale::En, Key::ClientTokenRejected),
            "The host rejected the token. Check ROBROZ_TOKEN."
        );
        assert_eq!(
            text_in(Locale::Zh, Key::ClientTokenRejected),
            "宿主拒绝了令牌，请检查 ROBROZ_TOKEN。"
        );
        // The token itself never appears in the message.
        assert!(!unauthorized_message()
            .to_ascii_lowercase()
            .contains("bearer"));
        assert!(
            !error_text("GET", "http://h/api/groups", 401, "sekrit-0123456789").contains("sekrit")
        );
    }

    #[test]
    fn connection_form_api_is_exported() {
        let _load: fn() -> ConnConfig = load;
        let _save: fn(&str, TokenEdit) -> Result<(), SaveError> = save;
        let cfg = ConnConfig {
            host: DEFAULT_HOST.to_string(),
            host_source: Source::Default,
            token: TokenState::None,
        };
        assert_eq!(cfg.token, TokenState::None);
        assert_eq!(ConnError::Unauthorized.to_string(), unauthorized_message());
    }
}
