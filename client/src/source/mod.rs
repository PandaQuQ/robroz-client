//! Where events come from.
//!
//! [`HttpSource`] polls the host's `GET /api/groups/{id}/events?since=…`.
//! [`stream`] reads `GET /api/groups/{id}/stream` (SSE) for the open chat.
//! [`FakeSource`] replays a script offline and backs the unit tests.
//! The app and [`crate::model::Feed`] only see [`EventSource`].

mod fake;
mod fake_approvals;
mod fake_codes;
mod http;
pub mod stream;

pub use fake::{fake_chats, fake_schedules, FakeSource, DRAFT_BODY};
pub use fake_approvals::{approvals_chats, approvals_source};
pub use fake_codes::{codes_chats, codes_source};
pub use http::HttpSource;

use std::fmt;
use std::time::Duration;

use robroz_contract::Event;
use robroz_ui::cards::{ApprovalExtras, Card};

/// One group or DM the client knows: id and display name, from
/// `GET /api/groups` (or `ROBROZ_GROUPS` when pinned).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupTab {
    pub id: String,
    pub name: String,
}

/// One contract event tagged with the group it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupEvent {
    pub group_id: String,
    pub event: Event,
    /// Client-side approval fields the contract event cannot carry yet.
    pub approval_extras: Option<ApprovalExtras>,
}

/// A new-kind card the contract has no event for yet (approval, draft),
/// tagged with its group. Only the fake source produces these today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupCard {
    pub group_id: String,
    pub card: Card,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceError(pub String);

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl SourceError {
    /// Map a failed first load or retry onto the connection form's two cases.
    #[allow(dead_code)] // connection form, follow-up PR
    pub fn as_conn_error(&self) -> crate::conn::ConnError {
        crate::conn::ConnError::classify(&self.0)
    }
}

/// Calls block (the HTTP source does network I/O); the app runs them off the
/// UI thread.
pub trait EventSource: Send {
    /// Groups to show in the left column. Called again when the host
    /// reports a group the client has not seen (e.g. a new DM).
    fn groups(&mut self) -> Result<Vec<GroupTab>, SourceError>;
    /// Start polling a group created by this client (a new bro's DM).
    fn add_group(&mut self, _group: GroupTab) {}
    /// Events since the last poll. Returning the full snapshot is also fine,
    /// because `Feed::apply` is idempotent.
    fn poll(&mut self) -> Result<Vec<GroupEvent>, SourceError>;
    /// False once nothing more will arrive. The fake source stops after its
    /// script; a live source always says true.
    fn has_more(&self) -> bool;
    /// Cards released with the last `poll` that have no contract event yet.
    fn poll_cards(&mut self) -> Vec<GroupCard> {
        Vec::new()
    }
    /// How often the app should call `poll`.
    fn interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}

/// A fresh path per call. Tests run in parallel and macOS clocks tick in
/// microseconds, so a timestamp alone let two tests share one database
/// ("database is locked"); the counter makes every name unique in the process.
#[cfg(test)]
fn temp_path(kind: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("robroz-client-{kind}-{}-{n}", std::process::id()))
}

#[cfg(test)]
fn temp_db() -> std::path::PathBuf {
    temp_path("test").with_extension("db")
}

#[cfg(test)]
fn temp_volumes() -> std::path::PathBuf {
    let path = temp_path("vol");
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// A real host on a random port with its own database and volumes.
#[cfg(test)]
pub fn http_test_host() -> String {
    serve(
        robroz_host::Host::open(&temp_db(), temp_volumes(), "robroz-runner:local")
            .unwrap()
            .router(),
    )
}

/// Same, with the host requiring `Authorization: Bearer <token>` (robroz#22).
#[cfg(test)]
pub fn http_test_host_with_token(token: &str) -> String {
    let volumes = temp_volumes();
    let token = robroz_host::auth::ApiToken::parse(token).unwrap();
    serve(
        robroz_host::Host::open(&temp_db(), volumes, "robroz-runner:local")
            .unwrap()
            .with_token(token)
            .router(),
    )
}

#[cfg(test)]
fn serve(router: axum::Router) -> String {
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    std_listener.set_nonblocking(true).unwrap();
    let addr = std_listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(std_listener).unwrap();
            axum::serve(listener, router).await.unwrap();
        });
    });
    format!("http://{addr}")
}

#[cfg(test)]
mod tests {
    use super::SourceError;
    use crate::conn::ConnError;

    #[test]
    fn as_conn_error_maps_401_and_unreachable() {
        assert_eq!(
            SourceError("GET http://h/api/groups returned 401: missing".into()).as_conn_error(),
            ConnError::Unauthorized
        );
        assert_eq!(
            SourceError("io: Connection refused (os error 111)".into()).as_conn_error(),
            ConnError::Unreachable(robroz_ui::i18n::Key::ClientConnectionRefused.to_string())
        );
    }
}
