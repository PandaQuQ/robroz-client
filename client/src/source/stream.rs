//! Live events for one chat over the host's SSE stream (robroz#25):
//! `GET /api/groups/{id}/stream`.
//!
//! Each SSE message is one contract `Event` as JSON in `data:`. `id:` is
//! an opaque string (a decimal seq after host C1, or a legacy RFC3339
//! timestamp) and is sent back verbatim as `Last-Event-ID`. Only the last
//! frame of a batch carries an id; earlier frames leave the cursor still.
//!
//! A `event: resync` frame (contract-v0.3.1, no `id:`) means the cursor
//! could not be replayed. The client keeps showing existing cards,
//! buffers the snapshot that follows, and swaps once the last snapshot
//! frame (the one with an `id:`) arrives. An empty group has no such
//! frame; the UI completes after 300ms of silence.
//!
//! After 45s with no bytes the client reconnects (the host keepalives
//! every 15s). A clean close (the host ends the stream at 3600s) reconnects
//! immediately and does not count as a failure. 503 responses wait
//! `Retry-After`. Failed reconnects use 1s, 2s, 4s … up to 30s backoff.
//! The UI stays silent until more than 30s of continuous failed reconnects.
//! While the stream is down the app polls `…/events?since=` every tick
//! ([`poll_interval`]); while it is up, polling slows to a safety net.
//! Only the open chat is streamed; other chats update through that poll
//! and `GET /api/unread`.

use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::time::Duration;

use robroz_ui::i18n::Key as Text;

use robroz_contract::Event;
use robroz_ui::cards::ApprovalExtras;

use crate::conn::Conn;
use crate::event_decode;

/// What the reader reports.
// `Event` grew with `confirm` cards; items are short-lived on a channel, so
// boxing them buys nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamItem {
    /// 200 and `text/event-stream`: events flow.
    Connected,
    Event {
        event: Event,
        extras: Option<ApprovalExtras>,
    },
    /// `event: resync` (contract-v0.3.1). Start collecting a snapshot;
    /// keep showing the current cards until [`Self::BatchEnd`] or the
    /// UI's empty-snapshot timeout.
    Resync,
    /// The frame carried an `id:`: end of a host batch, and of a
    /// resync snapshot when one is in progress.
    BatchEnd,
    /// Keep-alive comment; lets the reader notice nobody is listening.
    Ping,
    /// The connection ended or could not be made; it will retry.
    Down(StreamDown),
}

/// Why a stream dropped, and whether the next reconnect is silent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamDown {
    /// 45s with no bytes. Reconnect immediately; not a failure.
    Idle,
    /// Clean end of stream (host max-age). Reconnect immediately; not a
    /// failure, so it does not start the 30s banner timer.
    CleanClose,
    /// HTTP 503 (e.g. `too_many_streams`). Wait `retry_after` when set.
    Unavailable {
        retry_after: Option<Duration>,
        body: String,
    },
    /// Connect or read failure.
    Error(String),
}

impl StreamDown {
    /// Only real failures count toward the 30s reconnecting banner.
    pub fn counts_as_failure(&self) -> bool {
        matches!(self, Self::Unavailable { .. } | Self::Error(_))
    }

    /// 503 body is `{"code":"too_many_streams"}`.
    pub fn is_too_many_streams(&self) -> bool {
        match self {
            Self::Unavailable { body, .. } => too_many_streams_body(body),
            _ => false,
        }
    }

    /// How long to wait before the next attempt.
    pub fn retry_wait(&self, backoff: &mut Backoff) -> Duration {
        match self {
            Self::Idle | Self::CleanClose => Duration::ZERO,
            Self::Unavailable { retry_after, .. } => retry_after.unwrap_or(Duration::from_secs(
                robroz_ui::process::TOO_MANY_STREAMS_RETRY_SECS,
            )),
            Self::Error(_) => backoff.next_delay(),
        }
    }
}

/// How often the app polls: fast while the stream is down, slow while up.
pub fn poll_interval(stream_up: bool, base: Duration) -> Duration {
    if stream_up {
        SAFETY_NET
    } else {
        base
    }
}

/// Poll period while the stream is up.
pub const SAFETY_NET: Duration = Duration::from_secs(10);

pub fn path(group_id: &str) -> String {
    format!("/api/groups/{group_id}/stream")
}

/// One complete SSE message.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Frame {
    pub event: Option<String>,
    pub id: Option<String>,
    pub data: String,
}

/// Line-by-line SSE parser (the subset the host sends: `event`, `id`,
/// `data`, comments).
#[derive(Debug, Default)]
pub struct Parser {
    event: Option<String>,
    id: Option<String>,
    data: Vec<String>,
}

pub enum Line {
    Frame(Frame),
    Comment,
    Nothing,
}

impl Parser {
    pub fn line(&mut self, raw: &str) -> Line {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() {
            if self.data.is_empty() {
                self.event = None;
                return Line::Nothing;
            }
            let frame = Frame {
                event: self.event.take(),
                id: self.id.take(),
                data: std::mem::take(&mut self.data).join("\n"),
            };
            return Line::Frame(frame);
        }
        if line.starts_with(':') {
            return Line::Comment;
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match field {
            "event" => self.event = Some(value.to_string()),
            "id" => self.id = Some(value.to_string()),
            "data" => self.data.push(value.to_string()),
            _ => {}
        }
        Line::Nothing
    }
}

/// Reconnect delays: doubles from `min` to `max`, back to `min` after a
/// successful connect.
#[derive(Debug, Clone)]
pub struct Backoff {
    min: Duration,
    max: Duration,
    next: Duration,
}

impl Backoff {
    pub fn new(min: Duration, max: Duration) -> Self {
        Self {
            min,
            max,
            next: min,
        }
    }

    pub fn next_delay(&mut self) -> Duration {
        let d = self.next;
        self.next = (self.next * 2).min(self.max);
        d
    }

    pub fn reset(&mut self) {
        self.next = self.min;
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(30))
    }
}

fn is_idle_timeout(err: &ureq::Error) -> bool {
    match err {
        ureq::Error::Timeout(_) => true,
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::TimedOut => true,
        other => {
            let s = other.to_string().to_lowercase();
            s.contains("timed out") || s.contains("timeout")
        }
    }
}

fn map_connect_err(conn: &Conn, err: ureq::Error) -> StreamDown {
    if is_idle_timeout(&err) {
        // A timeout before the body starts is a failed connect, not an
        // idle drop of a live stream.
        StreamDown::Error(Text::ClientConnectFailed.fill(&[&conn.base(), &err]))
    } else {
        StreamDown::Error(Text::ClientConnectFailed.fill(&[&conn.base(), &err]))
    }
}

/// `Retry-After` as a delay: a number of seconds, or an HTTP-date.
pub fn parse_retry_after(headers: &ureq::http::HeaderMap) -> Option<Duration> {
    let raw = headers.get("retry-after")?.to_str().ok()?.trim();
    if let Ok(secs) = raw.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    chrono::DateTime::parse_from_rfc2822(raw).ok().map(|t| {
        let secs = (t.with_timezone(&chrono::Utc) - chrono::Utc::now())
            .num_seconds()
            .max(0) as u64;
        Duration::from_secs(secs)
    })
}

pub fn too_many_streams_body(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("code")?.as_str().map(str::to_owned))
        .is_some_and(|c| c == robroz_contract::error_codes::TOO_MANY_STREAMS)
}

fn is_resync_frame(frame: &Frame) -> bool {
    if frame.event.as_deref() == Some(robroz_contract::STREAM_RESYNC) {
        return true;
    }
    serde_json::from_str::<serde_json::Value>(&frame.data)
        .ok()
        .and_then(|v| v.get("type")?.as_str().map(str::to_owned))
        .is_some_and(|t| t == robroz_contract::STREAM_RESYNC)
}

/// Turn one SSE frame into stream items and advance `last_id` when the
/// frame carries an `id:`.
pub fn dispatch_frame(frame: Frame, last_id: &mut Option<String>) -> Vec<StreamItem> {
    let has_id = frame.id.is_some();
    if let Some(id) = frame.id.clone() {
        *last_id = Some(id);
    }
    if is_resync_frame(&frame) {
        return vec![StreamItem::Resync];
    }
    match event_decode::parse_event_str(&frame.data) {
        Ok((event, extras)) => {
            let mut out = vec![StreamItem::Event { event, extras }];
            if has_id {
                out.push(StreamItem::BatchEnd);
            }
            out
        }
        Err(err) => {
            if cfg!(debug_assertions) {
                eprintln!("skip stream event: {err}");
            }
            if has_id {
                vec![StreamItem::BatchEnd]
            } else {
                vec![]
            }
        }
    }
}

/// One connection. Returns `Ok(true)` when the sink asked to stop,
/// `Ok(false)` on a clean end of stream, `Err` on a failed connect or read.
/// `last_id` is sent as `Last-Event-ID` verbatim and advanced only when a
/// frame carries an `id:` (the last frame of a host batch).
pub fn read_once(
    conn: &Conn,
    group_id: &str,
    last_id: &mut Option<String>,
    sink: &mut dyn FnMut(StreamItem) -> bool,
) -> Result<bool, StreamDown> {
    let mut req = conn
        .get(&path(group_id))
        .header("Accept", "text/event-stream");
    if let Some(id) = last_id.as_deref() {
        // Opaque: decimal seq or a legacy RFC3339 timestamp.
        req = req.header("Last-Event-ID", id);
    }
    let response = req.call().map_err(|e| map_connect_err(conn, e))?;
    let status = response.status().as_u16();
    if status == 503 {
        let retry_after = parse_retry_after(response.headers());
        let body = response.into_body().read_to_string().unwrap_or_default();
        return Err(StreamDown::Unavailable { retry_after, body });
    }
    if status != 200 {
        let url = conn.url(&path(group_id));
        let body = response.into_body().read_to_string().unwrap_or_default();
        return Err(StreamDown::Error(crate::conn::error_text(
            "GET", &url, status, &body,
        )));
    }
    if !sink(StreamItem::Connected) {
        return Ok(true);
    }
    let reader = BufReader::new(response.into_body().into_reader());
    let mut parser = Parser::default();
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                return Err(StreamDown::Idle);
            }
            Err(e) => {
                let msg = Text::ClientStreamDroppedBecause.fill(&[&e]);
                // ureq may surface its timeout as a non-TimedOut io error.
                if msg.to_lowercase().contains("timed out")
                    || msg.to_lowercase().contains("timeout")
                {
                    return Err(StreamDown::Idle);
                }
                return Err(StreamDown::Error(msg));
            }
        };
        let items = match parser.line(&line) {
            Line::Frame(frame) => dispatch_frame(frame, last_id),
            Line::Comment => vec![StreamItem::Ping],
            Line::Nothing => continue,
        };
        for item in items {
            if !sink(item) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Read, and on any drop wait and reconnect, until the sink returns false.
pub fn run(
    conn: &Conn,
    group_id: &str,
    mut backoff: Backoff,
    mut sink: impl FnMut(StreamItem) -> bool,
) {
    let mut last_id = None;
    loop {
        let mut connected = false;
        let result = read_once(conn, group_id, &mut last_id, &mut |item| {
            if item == StreamItem::Connected {
                connected = true;
            }
            sink(item)
        });
        let down = match result {
            Ok(true) => return,
            Ok(false) => StreamDown::CleanClose,
            Err(e) => e,
        };
        if connected {
            backoff.reset();
        }
        let wait = down.retry_wait(&mut backoff);
        if !sink(StreamItem::Down(down)) {
            return;
        }
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
    }
}

/// Subscription key: one stream per host and group. The token rides along
/// but is not part of the hash.
#[derive(Debug, Clone)]
pub struct Key {
    pub conn: Conn,
    pub group_id: String,
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.conn.base().hash(state);
        self.group_id.hash(state);
    }
}

/// `(group, item)` for the app.
pub type Live = (String, StreamItem);

/// iced stream for one chat: runs [`run`] on a blocking thread and forwards
/// items until the subscription is dropped.
pub fn subscribe(key: &Key) -> impl iced::futures::Stream<Item = Live> {
    let conn = key.conn.streaming();
    let group = key.group_id.clone();
    iced::stream::channel(
        64,
        async move |mut tx: iced::futures::channel::mpsc::Sender<Live>| {
            let _ = tokio::task::spawn_blocking(move || {
                run(&conn, &group, Backoff::default(), |item| {
                    if item == StreamItem::Ping {
                        return !tx.is_closed();
                    }
                    let mut msg = (group.clone(), item);
                    loop {
                        match tx.try_send(msg) {
                            Ok(()) => return true,
                            Err(e) if e.is_full() => {
                                msg = e.into_inner();
                                std::thread::sleep(Duration::from_millis(20));
                            }
                            Err(_) => return false,
                        }
                    }
                })
            })
            .await;
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    #[test]
    fn parses_host_frames() {
        let mut p = Parser::default();
        let mut frames = Vec::new();
        for l in [
            ": keep-alive",
            "event: message",
            "id: 2026-11-20T02:00:00.000Z",
            "data: {\"a\":1}",
            "",
            "",
            "data:x",
            "data: y",
            "",
        ] {
            if let Line::Frame(f) = p.line(l) {
                frames.push(f);
            }
        }
        assert_eq!(
            frames,
            vec![
                Frame {
                    event: Some("message".into()),
                    id: Some("2026-11-20T02:00:00.000Z".into()),
                    data: "{\"a\":1}".into()
                },
                Frame {
                    event: None,
                    id: None,
                    data: "x\ny".into()
                },
            ]
        );
    }

    #[test]
    fn backoff_doubles_caps_and_resets() {
        let mut b = Backoff::new(Duration::from_secs(1), Duration::from_secs(5));
        let got: Vec<u64> = (0..5).map(|_| b.next_delay().as_secs()).collect();
        assert_eq!(got, [1, 2, 4, 5, 5]);
        b.reset();
        assert_eq!(b.next_delay().as_secs(), 1);
    }

    #[test]
    fn polling_is_the_fallback() {
        let base = Duration::from_secs(1);
        assert_eq!(
            poll_interval(false, base),
            base,
            "stream down: poll every tick"
        );
        assert_eq!(
            poll_interval(true, base),
            SAFETY_NET,
            "stream up: slow safety net"
        );
    }

    fn message_json(id: &str, body: &str) -> String {
        serde_json::json!({
            "type": "message", "id": id, "kind": "text", "body": body,
            "createdAt": "2026-11-20T02:00:00.000Z"
        })
        .to_string()
    }

    /// A one-shot SSE server per connection: sends `frames`, then closes.
    /// Returns each request's raw head so the test can check headers.
    fn flaky_server(script: Vec<Vec<(String, String)>>) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for frames in script {
                let (mut sock, _) = listener.accept().unwrap();
                let mut head = Vec::new();
                let mut buf = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    if sock.read(&mut buf).unwrap() == 0 {
                        break;
                    }
                    head.push(buf[0]);
                }
                tx.send(String::from_utf8_lossy(&head).to_lowercase())
                    .unwrap();
                let mut body = String::from(": hi\n\n");
                for (id, data) in frames {
                    body.push_str(&format!("event: message\nid: {id}\ndata: {data}\n\n"));
                }
                let resp = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                sock.write_all(resp.as_bytes()).unwrap();
            }
        });
        (format!("http://{addr}"), rx)
    }

    #[test]
    fn reconnects_with_last_event_id_after_a_drop() {
        let (base, heads) = flaky_server(vec![
            vec![("t1".into(), message_json("m1", "一"))],
            vec![("t2".into(), message_json("m2", "二"))],
        ]);
        let conn = Conn::new(base).with_token(Some("tok-0123456789".into()));
        let mut got = Vec::new();
        let mut events = 0;
        run(
            &conn,
            "g1",
            Backoff::new(Duration::from_millis(10), Duration::from_millis(20)),
            |item| {
                if matches!(
                    item,
                    StreamItem::Ping | StreamItem::BatchEnd | StreamItem::Resync
                ) {
                    return true;
                }
                if matches!(item, StreamItem::Event { .. }) {
                    events += 1;
                }
                got.push(item);
                events < 2
            },
        );
        let kinds: Vec<&str> = got
            .iter()
            .map(|i| match i {
                StreamItem::Connected => "up",
                StreamItem::Event { .. } => "event",
                StreamItem::Down(_) => "down",
                StreamItem::Ping => "ping",
                StreamItem::Resync => "resync",
                StreamItem::BatchEnd => "batch",
            })
            .collect();
        assert_eq!(kinds, ["up", "event", "down", "up", "event"]);
        let first = heads.recv().unwrap();
        assert!(first.starts_with("get /api/groups/g1/stream "), "{first}");
        assert!(
            first.contains("authorization: bearer tok-0123456789"),
            "token sent"
        );
        assert!(!first.contains("last-event-id"), "fresh start: {first}");
        let second = heads.recv().unwrap();
        assert!(second.contains("last-event-id: t1"), "resume: {second}");
    }

    #[test]
    fn unknown_stream_events_are_skipped() {
        let unknown = serde_json::json!({
            "type": "poll", "id": "p1", "createdAt": "2026-11-20T02:00:01.000Z"
        })
        .to_string();
        let (base, heads) = flaky_server(vec![
            vec![
                ("t1".into(), message_json("m1", "一")),
                ("t2".into(), unknown),
                ("t3".into(), message_json("m3", "三")),
            ],
            vec![],
        ]);
        let conn = Conn::new(base);
        let mut bodies = Vec::new();
        let mut downs = 0;
        run(
            &conn,
            "g1",
            Backoff::new(Duration::from_millis(10), Duration::from_millis(20)),
            |item| {
                match item {
                    StreamItem::Event {
                        event: Event::Message { body, .. },
                        ..
                    } => bodies.push(body),
                    StreamItem::Event { event: other, .. } => panic!("unexpected {other:?}"),
                    StreamItem::Down(_) => downs += 1,
                    _ => {}
                }
                downs < 2
            },
        );
        assert_eq!(bodies, ["一", "三"], "known events on both sides arrive");
        heads.recv().unwrap();
        let second = heads.recv().unwrap();
        assert!(
            second.contains("last-event-id: t3"),
            "resume past the skipped frame: {second}"
        );
    }

    #[test]
    fn host_down_reports_down_and_retries() {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let conn = Conn::new(format!("http://127.0.0.1:{port}"));
        let mut downs = 0;
        run(
            &conn,
            "g",
            Backoff::new(Duration::from_millis(1), Duration::from_millis(2)),
            |item| {
                assert!(matches!(item, StreamItem::Down(_)), "{item:?}");
                downs += 1;
                downs < 3
            },
        );
        assert_eq!(downs, 3, "kept retrying");
    }

    /// The real host (robroz#25): new and changed cards arrive live.
    #[test]
    fn approval_cards_arrive_and_update_in_place_live() {
        use crate::api::{ApiError, CardApi, HttpApi};
        use crate::model::{Feed, Row};
        use robroz_contract::{ApprovalDecision, Group};
        use robroz_ui::cards::{ApprovalState, Card};
        let base = crate::source::http_test_host();
        let post = |path: &str, body: serde_json::Value| -> serde_json::Value {
            let mut r = ureq::post(&format!("{base}{path}"))
                .header("content-type", "application/json")
                .send(body.to_string())
                .unwrap();
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        let group: Group =
            serde_json::from_value(post("/api/groups", serde_json::json!({"name": "审批"})))
                .unwrap();
        post(
            &format!("/api/groups/{}/members", group.id),
            serde_json::json!({"botId": "bot-ops", "name": "运维"}),
        );
        let conn = Conn::new(&base).streaming();
        let (tx, rx) = mpsc::channel();
        let gid = group.id.clone();
        std::thread::spawn(move || {
            run(&conn, &gid, Backoff::default(), |item| {
                tx.send(item).is_ok()
            });
        });
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            StreamItem::Connected
        );
        let next_approval = || loop {
            match rx
                .recv_timeout(Duration::from_secs(5))
                .expect("event within 5s")
            {
                StreamItem::Event {
                    event: e @ Event::Approval { .. },
                    ..
                } => return e,
                StreamItem::Ping
                | StreamItem::BatchEnd
                | StreamItem::Resync
                | StreamItem::Event { .. } => continue,
                other => panic!("unexpected {other:?}"),
            }
        };
        post(
            &format!("/api/groups/{}/approvals", group.id),
            serde_json::json!({
                "botId": "bot-ops", "action": "delete",
                "summary": "删掉旧的构建缓存", "path": "target/"
            }),
        );
        let pending = next_approval();
        let Event::Approval { id, decision, .. } = &pending else {
            unreachable!()
        };
        assert_eq!(*decision, None);
        let mut feed = Feed::default();
        feed.apply(&pending);

        let api = HttpApi::new(Conn::new(&base));
        let (answered, _) = api.decide(id, true).unwrap().expect("host echoes the card");
        assert!(matches!(
            answered,
            Event::Approval {
                decision: Some(ApprovalDecision::Approved),
                ..
            }
        ));
        assert!(matches!(
            api.decide(id, false),
            Err(ApiError::ApprovalSettled)
        ));
        // The stream brings the answered card; it replaces the pending one.
        let live = next_approval();
        assert_eq!(live, answered);
        feed.apply(&live);
        let rows: Vec<_> = feed.rows().collect();
        assert_eq!(rows.len(), 1, "updated in place");
        match rows[0] {
            Row::Card(Card::Approval(c)) => {
                assert_eq!(c.summary, "删掉旧的构建缓存");
                assert!(matches!(c.state, ApprovalState::Approved { .. }));
            }
            other => panic!("expected approval card, got {other:?}"),
        }
    }

    #[test]
    fn live_events_from_real_host() {
        use robroz_contract::{CreateGroup, Group, MessageKind, PostMessage};
        let base = crate::source::http_test_host();
        let post = |path: &str, body: String| -> serde_json::Value {
            let mut r = ureq::post(&format!("{base}{path}"))
                .header("content-type", "application/json")
                .send(body)
                .unwrap();
            serde_json::from_str(&r.body_mut().read_to_string().unwrap()).unwrap()
        };
        let group: Group = serde_json::from_value(post(
            "/api/groups",
            serde_json::to_string(&CreateGroup {
                name: "直播".into(),
            })
            .unwrap(),
        ))
        .unwrap();
        let conn = Conn::new(&base).streaming();
        let (tx, rx) = mpsc::channel();
        let gid = group.id.clone();
        std::thread::spawn(move || {
            run(&conn, &gid, Backoff::default(), |item| {
                tx.send(item).is_ok()
            });
        });
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            StreamItem::Connected
        );
        post(
            &format!("/api/groups/{}/messages", group.id),
            serde_json::to_string(&PostMessage {
                kind: MessageKind::Text,
                body: "实时".into(),
            })
            .unwrap(),
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            match rx.recv_timeout(left).expect("event within 5s") {
                StreamItem::Event {
                    event: Event::Message { body, .. },
                    ..
                } => {
                    assert_eq!(body, "实时");
                    break;
                }
                StreamItem::Ping | StreamItem::BatchEnd | StreamItem::Resync => continue,
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    fn status_server(status: u16, extra_headers: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let headers = extra_headers.to_string();
        let body = body.to_string();
        std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut head = Vec::new();
            let mut buf = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if sock.read(&mut buf).unwrap() == 0 {
                    break;
                }
                head.push(buf[0]);
            }
            let resp = format!(
                "HTTP/1.1 {status} Error\r\ncontent-type: text/plain\r\ncontent-length: {}\r\nconnection: close\r\n{headers}\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes());
        });
        format!("http://{addr}")
    }

    #[test]
    fn parse_retry_after_accepts_seconds() {
        let mut headers = ureq::http::HeaderMap::new();
        headers.insert("retry-after", "5".parse().unwrap());
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(5)));
        headers.insert("retry-after", "0".parse().unwrap());
        assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(0)));
    }

    #[test]
    fn last_event_id_is_sent_verbatim_and_skips_frames_without_id() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut sock, _) = listener.accept().unwrap();
                let mut head = Vec::new();
                let mut buf = [0u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    if sock.read(&mut buf).unwrap() == 0 {
                        break;
                    }
                    head.push(buf[0]);
                }
                tx.send(String::from_utf8_lossy(&head).to_lowercase())
                    .unwrap();
                let d1 = message_json("m1", "一");
                let d2 = message_json("m2", "二");
                let body = format!(
                    ": hi\n\nevent: message\ndata: {d1}\n\nevent: message\nid: 42\ndata: {d2}\n\n"
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes());
            }
        });
        let conn = Conn::new(format!("http://{addr}"));
        let mut events = 0;
        run(
            &conn,
            "g1",
            Backoff::new(Duration::from_millis(10), Duration::from_millis(20)),
            |item| {
                if matches!(item, StreamItem::Event { .. }) {
                    events += 1;
                }
                events < 3
            },
        );
        let first = rx.recv().unwrap();
        assert!(!first.contains("last-event-id"), "fresh: {first}");
        let second = rx.recv().unwrap();
        assert!(
            second.contains("last-event-id: 42"),
            "decimal seq sent verbatim, not a missing id: {second}"
        );
    }

    #[test]
    fn a_clean_close_is_not_a_failure() {
        assert!(!StreamDown::CleanClose.counts_as_failure());
        assert!(!StreamDown::Idle.counts_as_failure());
        assert!(StreamDown::Error("x".into()).counts_as_failure());
        assert!(StreamDown::Unavailable {
            retry_after: Some(Duration::from_secs(5)),
            body: "too_many_streams".into(),
        }
        .counts_as_failure());
        let mut b = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
        assert_eq!(StreamDown::CleanClose.retry_wait(&mut b), Duration::ZERO);
        assert_eq!(StreamDown::Idle.retry_wait(&mut b), Duration::ZERO);
        assert_eq!(
            StreamDown::Unavailable {
                retry_after: Some(Duration::from_secs(5)),
                body: String::new(),
            }
            .retry_wait(&mut b),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn unavailable_503_waits_retry_after() {
        let base = status_server(503, "retry-after: 0\r\n", r#"{"code":"too_many_streams"}"#);
        let conn = Conn::new(base);
        let err = read_once(&conn, "g1", &mut None, &mut |_| true).unwrap_err();
        match err {
            StreamDown::Unavailable { retry_after, body } => {
                assert_eq!(retry_after, Some(Duration::from_secs(0)));
                assert!(body.contains("too_many_streams"), "{body}");
            }
            other => panic!("expected unavailable, got {other:?}"),
        }
    }

    #[test]
    fn unavailable_without_retry_after_defaults_to_five_seconds() {
        let mut b = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
        assert_eq!(
            StreamDown::Unavailable {
                retry_after: None,
                body: r#"{"code":"too_many_streams"}"#.into(),
            }
            .retry_wait(&mut b),
            Duration::from_secs(5)
        );
        assert!(too_many_streams_body(r#"{"code":"too_many_streams"}"#));
        assert!(!too_many_streams_body(r#"{"code":"other"}"#));
        assert!(!too_many_streams_body("no json"));
        assert!(StreamDown::Unavailable {
            retry_after: None,
            body: r#"{"code":"too_many_streams","error":"log only"}"#.into(),
        }
        .is_too_many_streams());
    }

    #[test]
    fn resync_then_snapshot_emits_resync_events_and_batch_end() {
        let mut last = None;
        let items = dispatch_frame(
            Frame {
                event: Some("resync".into()),
                id: None,
                data: r#"{"type":"resync"}"#.into(),
            },
            &mut last,
        );
        assert_eq!(items, vec![StreamItem::Resync]);
        assert_eq!(last, None, "resync has no id");

        let d1 = message_json("m1", "一");
        let first = dispatch_frame(
            Frame {
                event: Some("message".into()),
                id: None,
                data: d1,
            },
            &mut last,
        );
        assert!(matches!(first.as_slice(), [StreamItem::Event { .. }]));
        assert_eq!(last, None);

        let d2 = message_json("m2", "二");
        let last_frame = dispatch_frame(
            Frame {
                event: Some("message".into()),
                id: Some("42".into()),
                data: d2,
            },
            &mut last,
        );
        assert!(matches!(
            last_frame.as_slice(),
            [StreamItem::Event { .. }, StreamItem::BatchEnd]
        ));
        assert_eq!(last.as_deref(), Some("42"));
    }

    #[test]
    fn empty_resync_has_no_batch_end() {
        let mut last = Some("old".into());
        let items = dispatch_frame(
            Frame {
                event: Some(robroz_contract::STREAM_RESYNC.into()),
                id: None,
                data: r#"{"type":"resync"}"#.into(),
            },
            &mut last,
        );
        assert_eq!(items, vec![StreamItem::Resync]);
        assert_eq!(last.as_deref(), Some("old"));
    }

    #[test]
    fn idle_timeout_reconnects_when_no_bytes_arrive() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut head = Vec::new();
            let mut buf = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if sock.read(&mut buf).unwrap() == 0 {
                    break;
                }
                head.push(buf[0]);
            }
            let body = ": hi\n\n";
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n{body}"
            );
            let _ = sock.write_all(resp.as_bytes());
            std::thread::sleep(Duration::from_secs(3));
        });
        let conn =
            Conn::new(format!("http://{addr}")).streaming_with_idle(Duration::from_millis(300));
        let mut last = None;
        let mut saw_up = false;
        let start = std::time::Instant::now();
        let err = read_once(&conn, "g1", &mut last, &mut |item| {
            if item == StreamItem::Connected {
                saw_up = true;
            }
            true
        })
        .unwrap_err();
        assert!(saw_up);
        assert_eq!(err, StreamDown::Idle);
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "idle should trip in well under 2s, took {:?}",
            start.elapsed()
        );
    }

    /// The real host's SSE stream, read with the client's own reader.
    mod against_host {
        use super::*;
        use crate::conn::Conn;
        use robroz_contract::{
            AddMember, CreateGroup, CreateTask, Group, Message, MessageKind, PostMessage, Task,
        };

        fn post<T: serde::Serialize, R: serde::de::DeserializeOwned>(
            base: &str,
            path: &str,
            body: &T,
        ) -> R {
            let mut response = ureq::post(&format!("{base}{path}"))
                .header("content-type", "application/json")
                .send(serde_json::to_string(body).unwrap())
                .unwrap();
            serde_json::from_str(&response.body_mut().read_to_string().unwrap()).unwrap()
        }

        fn say(base: &str, group: &str, body: &str) -> Message {
            post(
                base,
                &format!("/api/groups/{group}/messages"),
                &PostMessage {
                    kind: MessageKind::Text,
                    body: body.into(),
                },
            )
        }

        /// One connection with `last_id`, read up to the end of the first
        /// batch (the frame that carries an `id:`). Returns the items.
        fn first_batch(conn: &Conn, group: &str, last_id: &mut Option<String>) -> Vec<StreamItem> {
            let mut items = Vec::new();
            let stopped = read_once(conn, group, last_id, &mut |item| {
                let end = item == StreamItem::BatchEnd;
                items.push(item);
                !end
            })
            .unwrap();
            assert!(stopped, "the batch ended before the stream did: {items:?}");
            items
        }

        fn card_ids(items: &[StreamItem]) -> Vec<String> {
            items
                .iter()
                .filter_map(|item| match item {
                    StreamItem::Event {
                        event: Event::Message { id, .. } | Event::Status { id, .. },
                        ..
                    } => Some(id.clone()),
                    _ => None,
                })
                .collect()
        }

        #[test]
        fn out_of_range_last_event_id_gets_resync_then_an_id_snapshot() {
            let (base, db) = crate::source::http_test_host_with_db();
            let group: Group = post(
                &base,
                "/api/groups",
                &CreateGroup {
                    name: "官网".into(),
                },
            );
            let _: serde_json::Value = post(
                &base,
                &format!("/api/groups/{}/members", group.id),
                &AddMember {
                    bot_id: "bot-ada".into(),
                    name: "Ada".into(),
                },
            );
            let first = say(&base, &group.id, "一");
            let second = say(&base, &group.id, "二");
            let task: Task = post(
                &base,
                &format!("/api/groups/{}/tasks", group.id),
                &CreateTask {
                    assignee_bot_id: "bot-ada".into(),
                    prompt: "画草图".into(),
                },
            );
            // The snapshot is ordered by created_at, then id. Give the two
            // messages the same time so the id decides, and the task a later one.
            let sql = rusqlite::Connection::open(&db).unwrap();
            for id in [&first.id, &second.id] {
                sql.execute(
                    "UPDATE messages SET created_at = '2026-11-20T08:00:00.000Z' WHERE id = ?1",
                    [id],
                )
                .unwrap();
            }
            sql.execute(
                "UPDATE tasks SET created_at = '2026-11-20T08:00:01.000Z' WHERE id = ?1",
                [&task.id],
            )
            .unwrap();
            let mut tied = [first.id.clone(), second.id.clone()];
            tied.sort();
            let snapshot = [tied[0].clone(), tied[1].clone(), task.id.clone()];

            // An id the host never handed out (far past its newest).
            let conn = Conn::new(&base).streaming();
            let mut last_id = Some("999999999".to_string());
            let items = first_batch(&conn, &group.id, &mut last_id);
            assert_eq!(items.first(), Some(&StreamItem::Connected), "{items:?}");
            assert_eq!(items.get(1), Some(&StreamItem::Resync), "{items:?}");
            assert_eq!(card_ids(&items), snapshot, "{items:?}");
            assert_eq!(items.last(), Some(&StreamItem::BatchEnd));
            assert_eq!(items.len(), 2 + 3 + 1, "{items:?}");
            let id = last_id
                .clone()
                .expect("the snapshot's last frame carries the id");
            assert!(
                id.parse::<i64>().is_ok() && id != "999999999",
                "a decimal id from the host: {id}"
            );

            // That id replays: only what changed, and no resync.
            let third = say(&base, &group.id, "三");
            let items = first_batch(&conn, &group.id, &mut last_id);
            assert!(!items.contains(&StreamItem::Resync), "{items:?}");
            assert_eq!(card_ids(&items), [third.id], "{items:?}");
        }
    }
}
