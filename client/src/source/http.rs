//! Live events from the host over plain HTTP: incremental polling.
//!
//! Base URL `ROBROZ_HOST` (default `http://127.0.0.1:8790`), token from
//! [`crate::conn`] (`ROBROZ_TOKEN` or the config-dir `token` file).
//! Groups come from `GET /api/groups`. `ROBROZ_GROUPS` (comma-separated `id`
//! or `id=名字`) still overrides the list, for pinning a test setup.
//! Each group keeps a cursor: the newest change time seen. Polls ask for
//! `GET /api/groups/{id}/events?since=<cursor − 1ms>`, so only changed cards
//! come back; the 1ms overlap catches rows written in the cursor's own
//! millisecond, and [`crate::model::Feed`] dedupes them by id.
//! The live stream ([`super::stream`]) is the fast path; this poll is the
//! fallback while the stream is down and a slow safety net while it is up.

use std::collections::HashMap;

use robroz_contract::{Event, Groups};
use robroz_ui::cards::ApprovalExtras;
use robroz_ui::i18n::Key as Text;

use super::{EventSource, GroupEvent, GroupTab, SourceError};
use crate::conn::Conn;
use crate::event_decode;
use crate::model::change_time;

pub struct HttpSource {
    conn: Conn,
    groups: Vec<GroupTab>,
    /// `ROBROZ_GROUPS` was set: do not replace the list from the host.
    pinned: bool,
    /// Group id → newest change time seen (RFC 3339).
    cursors: HashMap<String, String>,
}

impl HttpSource {
    pub fn new(conn: Conn, pinned: Vec<GroupTab>) -> Self {
        Self {
            conn,
            pinned: !pinned.is_empty(),
            groups: pinned,
            cursors: HashMap::new(),
        }
    }

    pub fn from_env() -> Self {
        let pinned = std::env::var("ROBROZ_GROUPS")
            .map(|v| parse_groups(&v))
            .unwrap_or_default();
        Self::new(Conn::from_env(), pinned)
    }

    pub fn base(&self) -> &str {
        self.conn.base()
    }

    pub fn conn(&self) -> &Conn {
        &self.conn
    }

    /// The `since` this group's next poll sends, if any.
    pub fn since(&self, group_id: &str) -> Option<String> {
        self.cursors.get(group_id).map(|at| overlap(at))
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, SourceError> {
        let url = self.conn.url(path);
        let mut response = self.conn.get(path).call().map_err(|err| {
            SourceError(Text::ClientConnectFailed.fill(&[&self.conn.base(), &err]))
        })?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|err| SourceError(Text::ClientReadFailed.fill(&[&url, &err])))?;
        if status != 200 {
            return Err(SourceError(crate::conn::error_text(
                "GET", &url, status, &body,
            )));
        }
        serde_json::from_str::<T>(&body).map_err(|err| {
            SourceError(format!(
                "GET {url} {}",
                Text::ClientDecodeFailed.fill(&[&err])
            ))
        })
    }

    /// `GET /api/groups/{id}/events[?since=…]` as the contract's [`Events`].
    /// Events of a type this client does not know are skipped (see
    /// [`decode_events`]); a broken response as a whole is still an error.
    #[cfg(test)]
    pub fn events(
        &self,
        group_id: &str,
        since: Option<&str>,
    ) -> Result<robroz_contract::Events, SourceError> {
        self.fetch(group_id, since)
            .map(|d| robroz_contract::Events { events: d.events })
    }

    fn fetch(&self, group_id: &str, since: Option<&str>) -> Result<Decoded, SourceError> {
        let path = match since {
            Some(s) => format!("/api/groups/{group_id}/events?since={}", query_escape(s)),
            None => format!("/api/groups/{group_id}/events"),
        };
        let raw: RawEvents = self.get_json(&path)?;
        Ok(decode_events(raw.events))
    }

    fn advance(&mut self, group_id: &str, decoded: &Decoded) {
        let times = decoded
            .events
            .iter()
            .map(change_time)
            .chain(decoded.skipped_at.as_deref());
        for at in times {
            let cur = self.cursors.entry(group_id.to_string()).or_default();
            if later(at, cur) {
                *cur = at.to_string();
            }
        }
    }
}

/// `{"events":[…]}` with each event left as raw JSON.
#[derive(serde::Deserialize)]
struct RawEvents {
    events: Vec<serde_json::Value>,
}

/// Known events, plus the newest change time among skipped ones.
#[derive(Debug, Default)]
struct Decoded {
    events: Vec<Event>,
    extras: Vec<Option<ApprovalExtras>>,
    skipped_at: Option<String>,
}

/// Decode one event at a time. A newer host may add event types (the
/// contract only grows); this client skips what it cannot read instead of
/// dropping the whole response. A skipped event still moves the `?since=`
/// cursor (its newest `updatedAt` / `answeredAt` / `createdAt`), so it is
/// not fetched again on every poll.
fn decode_events(raw: Vec<serde_json::Value>) -> Decoded {
    let mut out = Decoded::default();
    for value in raw {
        match event_decode::parse_event(value.clone()) {
            Ok((event, extras)) => {
                out.events.push(event);
                out.extras.push(extras);
            }
            Err(err) => {
                if cfg!(debug_assertions) {
                    eprintln!(
                        "skip event type {:?}: {err}",
                        value.get("type").and_then(|t| t.as_str()).unwrap_or("?")
                    );
                }
                let at = ["updatedAt", "answeredAt", "createdAt"]
                    .iter()
                    .filter_map(|k| value.get(*k).and_then(|v| v.as_str()))
                    .fold(None::<&str>, |best, t| match best {
                        Some(b) if !later(t, b) => Some(b),
                        _ => Some(t),
                    });
                if let Some(at) = at {
                    let newer = out.skipped_at.as_deref().is_none_or(|cur| later(at, cur));
                    if newer {
                        out.skipped_at = Some(at.to_string());
                    }
                }
            }
        }
    }
    out
}

/// `true` when `a` is after `b` (RFC 3339; empty `b` is "never").
fn later(a: &str, b: &str) -> bool {
    if b.is_empty() {
        return true;
    }
    match (
        chrono::DateTime::parse_from_rfc3339(a),
        chrono::DateTime::parse_from_rfc3339(b),
    ) {
        (Ok(x), Ok(y)) => x > y,
        _ => a > b,
    }
}

/// Cursor minus 1ms, in the host's own format.
fn overlap(at: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(at) {
        Ok(t) => (t - chrono::Duration::milliseconds(1))
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        Err(_) => at.to_string(),
    }
}

/// RFC 3339 in a query string: `+` and `:` must not be read as space/sep.
fn query_escape(s: &str) -> String {
    s.replace('%', "%25")
        .replace('+', "%2B")
        .replace(':', "%3A")
}

impl EventSource for HttpSource {
    fn groups(&mut self) -> Result<Vec<GroupTab>, SourceError> {
        if !self.pinned {
            let groups: Groups = self.get_json("/api/groups")?;
            self.groups = groups
                .groups
                .into_iter()
                .map(|g| GroupTab {
                    id: g.id,
                    name: g.name,
                })
                .collect();
        }
        Ok(self.groups.clone())
    }

    fn add_group(&mut self, group: GroupTab) {
        if !self.groups.iter().any(|g| g.id == group.id) {
            self.groups.push(group);
        }
    }

    fn poll(&mut self) -> Result<Vec<GroupEvent>, SourceError> {
        let mut out = Vec::new();
        for group in self.groups.clone() {
            let since = self.since(&group.id);
            let decoded = self.fetch(&group.id, since.as_deref())?;
            self.advance(&group.id, &decoded);
            for (event, extras) in decoded.events.into_iter().zip(decoded.extras) {
                out.push(GroupEvent {
                    group_id: group.id.clone(),
                    event,
                    approval_extras: extras,
                });
            }
        }
        Ok(out)
    }

    fn has_more(&self) -> bool {
        true
    }
}

fn parse_groups(value: &str) -> Vec<GroupTab> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|entry| match entry.split_once('=') {
            Some((id, name)) => GroupTab {
                id: id.trim().to_string(),
                name: name.trim().to_string(),
            },
            None => GroupTab {
                id: entry.to_string(),
                name: entry.to_string(),
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_group_list() {
        assert_eq!(
            parse_groups(" g1=官网 , g2 ,,"),
            vec![
                GroupTab {
                    id: "g1".into(),
                    name: "官网".into()
                },
                GroupTab {
                    id: "g2".into(),
                    name: "g2".into()
                },
            ]
        );
    }

    #[test]
    fn trailing_slash_is_trimmed() {
        assert_eq!(
            HttpSource::new(Conn::new("http://h:1/"), Vec::new()).base(),
            "http://h:1"
        );
    }

    #[test]
    fn unknown_event_types_are_skipped_and_still_move_the_cursor() {
        let raw: RawEvents = serde_json::from_str(
            r#"{"events":[
                {"type":"message","id":"m1","kind":"text","body":"一","createdAt":"2026-11-20T02:00:00.000Z"},
                {"type":"poll","id":"p1","question":"?","createdAt":"2026-11-20T02:00:01.000Z","updatedAt":"2026-11-20T02:00:09.000Z"},
                {"type":"message","id":"m2","kind":"text","body":"二","createdAt":"2026-11-20T02:00:02.000Z"},
                {"type":"message","id":"bad"}
            ]}"#,
        )
        .unwrap();
        let decoded = decode_events(raw.events);
        let ids: Vec<&str> = decoded
            .events
            .iter()
            .map(|e| match e {
                Event::Message { id, .. } => id.as_str(),
                _ => "?",
            })
            .collect();
        assert_eq!(ids, ["m1", "m2"], "known events kept, in order");
        assert_eq!(
            decoded.skipped_at.as_deref(),
            Some("2026-11-20T02:00:09.000Z")
        );
        let mut source = HttpSource::new(Conn::new("http://unused"), Vec::new());
        source.advance("g1", &decoded);
        assert_eq!(
            source.since("g1").as_deref(),
            Some("2026-11-20T02:00:08.999Z"),
            "cursor passes the skipped event"
        );
    }

    #[test]
    fn cursor_overlap_and_escape() {
        assert_eq!(
            overlap("2026-11-20T02:00:00.000Z"),
            "2026-11-20T01:59:59.999Z"
        );
        assert_eq!(
            overlap("2026-11-20T10:00:00.500+08:00"),
            "2026-11-20T02:00:00.499Z"
        );
        assert_eq!(
            query_escape("2026-11-20T02:00:00.000+08:00"),
            "2026-11-20T02%3A00%3A00.000%2B08%3A00"
        );
        assert!(later(
            "2026-11-20T02:00:00.001Z",
            "2026-11-20T02:00:00.000Z"
        ));
        assert!(!later(
            "2026-11-20T10:00:00.000+08:00",
            "2026-11-20T02:00:00.000Z"
        ));
        assert!(later("2026-11-20T02:00:00.000Z", ""));
    }

    /// Runs the real host router in-process and reads it through
    /// `HttpSource`, so the client decodes exactly what the host sends.
    /// No Docker: tasks stay queued.
    mod against_host {
        use super::*;
        use crate::model::{Feed, Row};
        use robroz_contract::{
            AddMember, CreateGroup, CreateTask, Group, MessageKind, PostMessage, Task, TaskState,
        };

        fn spawn_host() -> String {
            crate::source::http_test_host()
        }

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

        #[test]
        fn reads_groups_and_events_from_real_host() {
            let base = spawn_host();
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
            let _: serde_json::Value = post(
                &base,
                &format!("/api/groups/{}/messages", group.id),
                &PostMessage {
                    kind: MessageKind::Image,
                    body: "hero.png".into(),
                },
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
            let task: Task = post(
                &base,
                &format!("/api/groups/{}/tasks", group.id),
                &CreateTask {
                    assignee_bot_id: "bot-ada".into(),
                    prompt: "画草图".into(),
                },
            );

            // No ROBROZ_GROUPS: the list comes from GET /api/groups.
            let mut source = HttpSource::new(Conn::new(&base), Vec::new());
            let groups = source.groups().unwrap();
            assert_eq!(groups.len(), 1);
            assert_eq!(groups[0].id, group.id);
            assert_eq!(groups[0].name, "官网");

            let mut feed = Feed::default();
            let first = source.poll().unwrap();
            assert_eq!(first.len(), 2, "first poll: everything");
            for ge in &first {
                assert_eq!(ge.group_id, group.id);
                feed.apply(&ge.event);
            }
            assert!(source.since(&group.id).is_some(), "cursor set");

            // Second poll sends ?since= and gets only what changed: the new
            // text (plus at most the cursor's own millisecond), not hero.png.
            std::thread::sleep(std::time::Duration::from_millis(5));
            let _: serde_json::Value = post(
                &base,
                &format!("/api/groups/{}/messages", group.id),
                &PostMessage {
                    kind: MessageKind::Text,
                    body: "第二条".into(),
                },
            );
            let second = source.poll().unwrap();
            let bodies: Vec<_> = second
                .iter()
                .filter_map(|ge| match &ge.event {
                    robroz_contract::Event::Message { body, .. } => Some(body.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(bodies, ["第二条"], "{second:?}");
            for ge in &second {
                feed.apply(&ge.event);
            }
            assert!(
                source.poll().unwrap().len() <= 1,
                "nothing new: only the overlap"
            );
            let rows: Vec<Row> = feed.rows().cloned().collect();
            assert_eq!(rows.len(), 3, "no duplicates: {rows:?}");
            assert!(matches!(&rows[2], Row::Text { body } if body == "第二条"));
            assert!(
                matches!(&rows[0], Row::Attachment { kind: MessageKind::Image, name } if name == "hero.png")
            );
            match &rows[1] {
                Row::Status(card) => {
                    assert_eq!(card.task_id, task.id);
                    assert_eq!(card.bot_name, "Ada");
                    assert_eq!(
                        card.status_line,
                        robroz_card::status_line(TaskState::Queued, false)
                    );
                }
                other => panic!("expected status card, got {other:?}"),
            }
        }

        #[test]
        fn unknown_group_is_an_error() {
            let base = spawn_host();
            let mut source = HttpSource::new(Conn::new(base), parse_groups("nope"));
            let err = source.poll().unwrap_err();
            // The host's JSON `error` (「群不存在」), not a raw "returned 404: {body}" line.
            assert!(err.0.contains("群不存在") || err.0.contains("404"), "{err}");
        }

        #[test]
        fn host_down_is_an_error() {
            // Bind then drop to get a port nothing listens on.
            let port = std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port();
            let mut source = HttpSource::new(
                Conn::new(format!("http://127.0.0.1:{port}")),
                parse_groups("g"),
            );
            assert!(source.poll().is_err());
        }
    }
}
