//! Minimal blocking host client for the spike: `GET /api/groups`,
//! `GET /api/unread`, `GET /api/groups/{id}/events`, `GET /api/me`, the
//! bro list, `POST /api/bros` and the avatar `PATCH /api/bros/{id}`, every
//! request with `Authorization: Bearer <token>`. It decodes with the `robroz-contract`
//! types, so the wire format is the one the desktop client uses.
//!
//! ureq runs with rustls so an `https://` tunnel works on Android and iOS.
//! The token is never printed: `Debug` hides it and error texts only carry
//! the base URL. A 401 is shown as
//! [`robroz_ui::i18n::Key::ClientTokenRejected`], the same line the desktop
//! client uses, and never includes the token or the raw host body.

use std::fmt;
use std::time::Duration;

use robroz_contract::{
    Bro, BroAvatar, Bros, CreateBro, ErrorBody, Event, Events, Group, GroupMembers, GroupSearch,
    Groups, Me, PatchBro, RenameGroup, ScheduleEntry, Schedules, Unread, Unreads, UserRole,
    FORBIDDEN_NOT_BRO_OWNER, GROUP_SEARCH_DEFAULT_LIMIT,
};
use robroz_ui::i18n::{host_text, Key};

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

/// What went wrong, already in the active language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError(pub String);

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// `https://` is assumed when the user types a bare host. Only http and
/// https are accepted. `None` for anything else.
pub fn normalize_base(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    let with_scheme = if s.contains("://") {
        s.to_string()
    } else {
        format!("https://{s}")
    };
    let rest = with_scheme
        .strip_prefix("https://")
        .or_else(|| with_scheme.strip_prefix("http://"))?;
    let rest = rest.trim_end_matches('/');
    (!rest.is_empty() && !rest.starts_with('/') && !rest.contains(char::is_whitespace))
        .then(|| with_scheme.trim_end_matches('/').to_string())
}

/// Whether `host` is a remote `http://` address (not loopback). Same rule
/// as the desktop connection form: only then is
/// [`Key::ConnUnencryptedHint`] shown.
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

/// The client-facing 401 line (UI-approved). Never includes the token.
pub fn unauthorized_message() -> String {
    Key::ClientTokenRejected.into()
}

impl Conn {
    pub fn new(base: &str, token: &str) -> Result<Self, ApiError> {
        if base.trim().is_empty() {
            return Err(ApiError(Key::HostNeedUrl.as_str().to_string()));
        }
        let base = normalize_base(base)
            .ok_or_else(|| ApiError(Key::HostUrlInvalid.fill(&[&base.trim()])))?;
        let token = Some(token.trim().to_string()).filter(|t| !t.is_empty());
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .timeout_connect(Some(Duration::from_secs(5)))
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self { base, token, agent })
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn get(&self, path: &str) -> Result<String, ApiError> {
        self.send("GET", path, None).map_err(|e| e.message)
    }

    /// One request; a non-2xx keeps its status and `code` so callers can
    /// pick their own copy (the avatar page does).
    fn send(&self, method: &str, path: &str, json: Option<String>) -> Result<String, Failure> {
        let url = format!("{}{path}", self.base);
        let auth = self.token.as_ref().map(|t| format!("Bearer {t}"));
        let failed = |message: String| Failure {
            status: 0,
            code: None,
            message: ApiError(message),
        };
        let sent = match (method, json) {
            ("GET", _) => {
                let mut req = self.agent.get(&url);
                if let Some(a) = &auth {
                    req = req.header("Authorization", a);
                }
                req.call()
            }
            ("DELETE", _) => {
                let mut req = self.agent.delete(&url);
                if let Some(a) = &auth {
                    req = req.header("Authorization", a);
                }
                req.call()
            }
            (_, body) => {
                let mut req = if method == "PATCH" {
                    self.agent.patch(&url)
                } else {
                    self.agent.post(&url)
                };
                if let Some(a) = &auth {
                    req = req.header("Authorization", a);
                }
                req.header("Content-Type", "application/json")
                    .send(body.unwrap_or_default())
            }
        };
        let mut response =
            sent.map_err(|e| failed(Key::ClientConnectFailed.fill(&[&self.base, &e])))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|e| failed(Key::ClientReadFailed.fill(&[&url, &e])))?;
        if (200..300).contains(&status) {
            return Ok(body);
        }
        let code = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v.get("code")?.as_str().map(str::to_string));
        let failure = |message: String| Failure {
            status,
            code: code.clone(),
            message: ApiError(message),
        };
        if let Some(key) = code.as_deref().and_then(copy_for_code) {
            return Err(failure(key.as_str().to_string()));
        }
        // Same as the desktop client: a 401 is always the approved token
        // line, never the host body (which might echo the token).
        if status == 401 {
            return Err(failure(unauthorized_message()));
        }
        let msg = serde_json::from_str::<ErrorBody>(&body)
            .map(|e| host_text(&e.error).into_owned())
            .unwrap_or_else(|_| Key::ClientHttpStatus.fill(&[&method, &url, &status]));
        Err(failure(msg))
    }

    /// `GET /api/me`: who this client is. An older host has no such route;
    /// then nobody owns anything here and the avatar edit entry stays hidden.
    pub fn me(&self) -> Result<Me, ApiError> {
        Self::decode(&self.get("/api/me")?)
    }

    /// `GET /api/bros`, oldest first.
    pub fn bros(&self) -> Result<Vec<Bro>, ApiError> {
        Ok(Self::decode::<Bros>(&self.get("/api/bros")?)?.bros)
    }

    /// `POST /api/bros` with a name and an avatar.
    pub fn create_bro(&self, name: &str, avatar: BroAvatar) -> Result<Bro, ApiError> {
        let body = serde_json::to_string(&CreateBro {
            name: Some(name.trim().to_string()).filter(|n| !n.is_empty()),
            avatar: Some(avatar),
        })
        .expect("serialize bro");
        let out = self
            .send("POST", "/api/bros", Some(body))
            .map_err(|e| e.message)?;
        Self::decode(&out)
    }

    /// `PATCH /api/bros/{id}` with `{"avatar":{…}}`, or `{"avatar":null}`
    /// to go back to the id default.
    pub fn set_avatar(&self, id: &str, avatar: Option<BroAvatar>) -> Result<Bro, AvatarError> {
        let out = self.send(
            "PATCH",
            &format!("/api/bros/{id}"),
            Some(avatar_body(avatar)),
        );
        match out {
            Ok(body) => Self::decode(&body).map_err(|_| AvatarError::Failed),
            Err(f) => Err(AvatarError::from_failure(f.status, f.code.as_deref())),
        }
    }

    fn decode<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, ApiError> {
        serde_json::from_str(body).map_err(|e| ApiError(Key::ClientDecodeFailed.fill(&[&e])))
    }

    /// `GET /api/groups`, oldest first.
    pub fn groups(&self) -> Result<Vec<Group>, ApiError> {
        Ok(Self::decode::<Groups>(&self.get("/api/groups")?)?.groups)
    }

    /// `GET /api/unread`, most recently active first.
    pub fn unread(&self) -> Result<Vec<Unread>, ApiError> {
        Ok(Self::decode::<Unreads>(&self.get("/api/unread")?)?.groups)
    }

    /// `GET /api/groups/{id}/events`, oldest first, each with whether a
    /// message is the user's own.
    ///
    /// The contract's `Event::Message` has no author field yet, so the side
    /// is read from a local `"from": "bro"` field on the raw JSON, which only
    /// the spike's fixture host sends. A real host omits it, so every message
    /// reads as the user's own (right side). The contract is not touched.
    pub fn events(&self, group_id: &str) -> Result<Vec<(Event, bool)>, ApiError> {
        let path = format!("/api/groups/{group_id}/events");
        let body = self.get(&path)?;
        let mut raw = Self::decode::<serde_json::Value>(&body)?;
        // Unknown codes are logged and dropped so the card still decodes.
        robroz_ui::cards::codes::strip_unknown_codes(&mut raw);
        let events = serde_json::from_value::<Events>(raw.clone())
            .map_err(|e| ApiError(Key::ClientDecodeFailed.fill(&[&e])))?
            .events;
        let sides: Vec<Option<String>> = raw
            .get("events")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .map(|e| e.get("from").and_then(|f| f.as_str()).map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        Ok(events
            .into_iter()
            .enumerate()
            .map(|(i, e)| {
                let from = sides.get(i).cloned().flatten();
                let own = from.as_deref() != Some("bro");
                (e, own)
            })
            .collect())
    }

    /// `PATCH /api/groups/{id}` with `{ "name": ... }`.
    pub fn rename_group(&self, id: &str, name: &str) -> Result<Group, ApiError> {
        let body = serde_json::to_string(&RenameGroup {
            name: name.trim().to_string(),
        })
        .expect("serialize rename group");
        Self::decode(
            &self
                .send("PATCH", &format!("/api/groups/{id}"), Some(body))
                .map_err(|e| e.message)?,
        )
    }

    /// `DELETE /api/groups/{id}`.
    pub fn delete_group(&self, id: &str) -> Result<(), ApiError> {
        self.send("DELETE", &format!("/api/groups/{id}"), None)
            .map(|_| ())
            .map_err(|e| e.message)
    }

    /// `GET /api/groups/{id}/members`.
    pub fn group_members(&self, id: &str) -> Result<GroupMembers, ApiError> {
        Self::decode(&self.get(&format!("/api/groups/{id}/members"))?)
    }

    /// `GET /api/groups/{id}/search?q=&limit=`.
    pub fn search_group(&self, id: &str, q: &str) -> Result<GroupSearch, ApiError> {
        let path = format!(
            "/api/groups/{id}/search?q={}&limit={GROUP_SEARCH_DEFAULT_LIMIT}",
            query_escape(q)
        );
        Self::decode(&self.get(&path)?)
    }

    /// `GET /api/groups/{id}/schedules`.
    pub fn schedules(&self, id: &str) -> Result<Vec<ScheduleEntry>, ApiError> {
        Ok(
            Self::decode::<Schedules>(&self.get(&format!("/api/groups/{id}/schedules"))?)?
                .schedules,
        )
    }
}

/// Approved copy for group error codes (same keys as the desktop client).
fn copy_for_code(code: &str) -> Option<Key> {
    Some(match code {
        "not_group_creator" => Key::HostNotGroupCreator,
        "group_is_dm" => Key::HostGroupIsDm,
        "not_group_member" => Key::HostNotGroupMember,
        _ => return None,
    })
}

fn query_escape(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Rename / delete / members: not a DM, and this user is admin or created it.
pub fn can_manage(me: Option<&Me>, group: &Group) -> bool {
    if group.dm {
        return false;
    }
    let Some(me) = me else {
        return false;
    };
    me.role == UserRole::Admin || group.created_by.as_deref() == Some(me.id.as_str())
}

/// A non-2xx with its status and error `code`.
struct Failure {
    status: u16,
    code: Option<String>,
    message: ApiError,
}

/// Why an avatar save failed, for UI姐's two inline lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarError {
    /// 403 `not_bro_owner`: 「只有这个 bro 的主人能改头像」.
    NotOwner,
    /// 400 `invalid_avatar` and everything else: 「头像没存上，再试一次」.
    Failed,
}

impl AvatarError {
    fn from_failure(status: u16, code: Option<&str>) -> Self {
        if status == 403 && code == Some(FORBIDDEN_NOT_BRO_OWNER) {
            AvatarError::NotOwner
        } else {
            AvatarError::Failed
        }
    }

    pub fn copy(self) -> String {
        match self {
            AvatarError::NotOwner => Key::AvatarNotOwner.as_str().to_string(),
            AvatarError::Failed => Key::AvatarSaveFailed.as_str().to_string(),
        }
    }
}

/// `PATCH /api/bros/{id}` body for an avatar change (contract `PatchBro`).
pub fn avatar_body(avatar: Option<BroAvatar>) -> String {
    serde_json::to_string(&PatchBro {
        name: None,
        avatar: Some(avatar),
    })
    .expect("serialize avatar")
}

/// Eight bullets: the only stand-in for a saved token, same as the desktop
/// connection form. The real value never goes in the field.
pub const TOKEN_MASK: &str = "••••••••";

/// What the token field should show: the mask when a token is saved and
/// the field has not been edited.
pub fn display_token(saved: Option<&str>, draft: &str, dirty: bool) -> String {
    if !dirty && saved.filter(|s| !s.is_empty()).is_some() {
        TOKEN_MASK.to_string()
    } else {
        draft.to_string()
    }
}

/// The token to send on Connect. Never treats the mask as the value.
pub fn token_for_connect(saved: Option<&str>, field: &str, dirty: bool) -> String {
    if !dirty {
        return saved.unwrap_or("").trim().to_string();
    }
    let trimmed = field.trim();
    if trimmed == TOKEN_MASK {
        return saved.unwrap_or("").trim().to_string();
    }
    trimmed.to_string()
}

/// Apply an edit to the token field. Typing over the mask starts a new
/// draft; leaving the mask in place keeps the saved token.
pub fn edit_token(dirty: bool, new_value: &str) -> (String, bool) {
    if !dirty {
        if new_value == TOKEN_MASK || new_value.is_empty() {
            return (String::new(), false);
        }
        let next = new_value
            .strip_prefix(TOKEN_MASK)
            .unwrap_or(new_value)
            .to_string();
        return (next, true);
    }
    (new_value.to_string(), true)
}

/// Sessions as the list shows them: most recently active first, with the
/// unread count. Groups without activity keep host order after the active
/// ones.
pub fn session_rows(groups: &[Group], unread: &[Unread]) -> Vec<(Group, u64)> {
    let mut rows: Vec<(Group, u64)> = Vec::new();
    for u in unread {
        if let Some(g) = groups.iter().find(|g| g.id == u.group_id) {
            rows.push((g.clone(), u.unread));
        }
    }
    for g in groups {
        if !rows.iter().any(|(r, _)| r.id == g.id) {
            rows.push((g.clone(), 0));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// One-request fake host; returns the raw request it saw.
    fn fake_host(status: &str, body: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let status = status.to_string();
        let handle = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).unwrap();
            let reply = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            sock.write_all(reply.as_bytes()).unwrap();
            String::from_utf8_lossy(&buf[..n]).to_string()
        });
        (format!("http://{addr}"), handle)
    }

    #[test]
    fn avatar_body_sets_or_clears() {
        assert_eq!(
            avatar_body(Some(BroAvatar { shape: 3, color: 9 })),
            r#"{"avatar":{"shape":3,"color":9}}"#
        );
        assert_eq!(avatar_body(None), r#"{"avatar":null}"#);
    }

    #[test]
    fn set_avatar_patches_and_maps_host_errors() {
        let (base, req) = fake_host(
            "400 Bad Request",
            r#"{"error":"头像不对","code":"invalid_avatar"}"#,
        );
        let conn = Conn::new(&base, "t").unwrap();
        let err = conn
            .set_avatar("b1", Some(BroAvatar { shape: 1, color: 2 }))
            .unwrap_err();
        assert_eq!(err, AvatarError::Failed);
        let sent = req.join().unwrap();
        assert!(sent.starts_with("PATCH /api/bros/b1 "), "{sent}");

        let (base, req) = fake_host(
            "403 Forbidden",
            r#"{"error":"x","code":"not_bro_owner","ownerName":"Bea"}"#,
        );
        let conn = Conn::new(&base, "t").unwrap();
        assert_eq!(
            conn.set_avatar("b1", None).unwrap_err(),
            AvatarError::NotOwner
        );
        assert!(req.join().unwrap().starts_with("PATCH /api/bros/b1 "));
    }

    #[test]
    fn avatar_errors_pick_the_two_lines() {
        assert_eq!(
            AvatarError::from_failure(403, Some("not_bro_owner")),
            AvatarError::NotOwner
        );
        assert_eq!(
            AvatarError::from_failure(400, Some("invalid_avatar")),
            AvatarError::Failed
        );
        assert_eq!(AvatarError::from_failure(0, None), AvatarError::Failed);
        assert_eq!(AvatarError::from_failure(500, None), AvatarError::Failed);
    }

    #[test]
    fn base_url_is_normalised() {
        assert_eq!(
            normalize_base(" example.com/ "),
            Some("https://example.com".into())
        );
        assert_eq!(
            normalize_base("http://10.0.2.2:8790"),
            Some("http://10.0.2.2:8790".into())
        );
        assert_eq!(normalize_base("ftp://x"), None);
        assert_eq!(normalize_base(""), None);
        assert_eq!(normalize_base("https://"), None);
    }

    #[test]
    fn sends_bearer_and_decodes_groups() {
        let (base, seen) = fake_host(
            "200 OK",
            r#"{"groups":[{"id":"g1","name":"Ada","tenantId":"t"}]}"#,
        );
        let conn = Conn::new(&base, "dummy-token-for-fixture-only").unwrap();
        let groups = conn.groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Ada");
        let request = seen.join().unwrap().to_ascii_lowercase();
        assert!(request.starts_with("get /api/groups "), "{request}");
        assert!(
            request.contains("authorization: bearer dummy-token-for-fixture-only"),
            "{request}"
        );
    }

    #[test]
    fn a_401_is_the_same_token_message_as_desktop() {
        use robroz_ui::i18n::{text_in, Locale};
        let body: &'static str = Box::leak(
            format!(
                r#"{{"error":"{}"}}"#,
                robroz_contract::host_text::TOKEN_REJECTED
            )
            .into_boxed_str(),
        );
        let (base, seen) = fake_host("401 Unauthorized", body);
        let conn = Conn::new(&base, "dummy-token-for-fixture-only").unwrap();
        let err = conn.groups().unwrap_err();
        seen.join().unwrap();
        assert_eq!(err.0, unauthorized_message());
        assert_eq!(
            text_in(Locale::En, Key::ClientTokenRejected),
            "The host rejected the token. Check ROBROZ_TOKEN."
        );
        assert_eq!(
            text_in(Locale::Zh, Key::ClientTokenRejected),
            "宿主拒绝了令牌，请检查 ROBROZ_TOKEN。"
        );
        assert!(!err.0.to_ascii_lowercase().contains("bearer"));
        assert!(!err.0.contains("dummy-token"));
        assert!(!err.0.contains(robroz_contract::host_text::TOKEN_REJECTED));
    }

    #[test]
    fn unencrypted_hint_matches_the_desktop_rule() {
        assert!(!is_insecure_remote("http://127.0.0.1:8790"));
        assert!(!is_insecure_remote("http://localhost:8790"));
        assert!(!is_insecure_remote("http://[::1]:8790"));
        assert!(!is_insecure_remote("https://example.invalid:8790"));
        assert!(is_insecure_remote("http://example.invalid:8790"));
        assert!(is_insecure_remote("http://192.168.1.8:8790"));
        assert!(!is_insecure_remote("example.invalid"));
    }

    #[test]
    fn message_side_comes_from_the_local_from_field_and_defaults_to_own() {
        let body = r#"{"events":[
            {"type":"message","id":"m1","kind":"text","body":"hi","createdAt":"2026-11-20T10:00:00.000Z"},
            {"type":"message","id":"m2","kind":"text","body":"yo","createdAt":"2026-11-20T10:00:01.000Z","from":"bro"}]}"#;
        let (base, seen) = fake_host("200 OK", body);
        let conn = Conn::new(&base, "t").unwrap();
        let events = conn.events("g1").unwrap();
        seen.join().unwrap();
        let own: Vec<bool> = events.iter().map(|(_, own)| *own).collect();
        assert_eq!(own, [true, false]);
    }

    #[test]
    fn token_stays_out_of_debug_output() {
        let conn = Conn::new("http://127.0.0.1:1", "dummy-token-for-fixture-only").unwrap();
        assert!(!format!("{conn:?}").contains("dummy-token"));
    }

    #[test]
    fn saved_token_is_exactly_eight_bullets() {
        assert_eq!(TOKEN_MASK, "••••••••");
        assert_eq!(TOKEN_MASK.chars().count(), 8);
        assert_eq!(
            display_token(Some("dummy-token-for-fixture-only"), "", false),
            TOKEN_MASK
        );
        assert_eq!(
            token_for_connect(Some("dummy-token-for-fixture-only"), TOKEN_MASK, false),
            "dummy-token-for-fixture-only"
        );
        assert_eq!(token_for_connect(Some("saved"), "typed", true), "typed");
        let (draft, dirty) = edit_token(false, TOKEN_MASK);
        assert!(!dirty, "{draft}");
        let (draft, dirty) = edit_token(false, format!("{TOKEN_MASK}abc").as_str());
        assert!(dirty);
        assert_eq!(draft, "abc");
    }

    #[test]
    fn rows_put_active_sessions_first() {
        let g = |id: &str| Group {
            created_by: None,
            dm: false,
            id: id.into(),
            name: id.into(),
            tenant_id: "t".into(),
        };
        let u = |id: &str, n: u64| Unread {
            group_id: id.into(),
            read_at: None,
            unread: n,
            last_activity_at: None,
        };
        let rows = session_rows(&[g("a"), g("b"), g("c")], &[u("c", 2), u("a", 0)]);
        let ids: Vec<_> = rows.iter().map(|(g, n)| (g.id.as_str(), *n)).collect();
        assert_eq!(ids, [("c", 2), ("a", 0), ("b", 0)]);
    }

    #[test]
    fn can_manage_hides_on_a_dm_and_shows_for_creator_or_admin() {
        let group = |dm: bool, created: Option<&str>| Group {
            created_by: created.map(str::to_string),
            dm,
            id: "g1".into(),
            name: "g1".into(),
            tenant_id: "t".into(),
        };
        let me = |admin: bool| Me {
            id: "u1".into(),
            name: "Panda".into(),
            role: if admin {
                UserRole::Admin
            } else {
                UserRole::Member
            },
        };
        let member = me(false);
        assert!(
            !can_manage(Some(&member), &group(true, Some("u1"))),
            "DMs never show manage entries"
        );
        assert!(can_manage(Some(&member), &group(false, Some("u1"))));
        assert!(!can_manage(Some(&member), &group(false, Some("other"))));
        let admin = me(true);
        assert!(can_manage(Some(&admin), &group(false, Some("other"))));
        assert!(
            can_manage(Some(&admin), &group(false, None)),
            "old groups: admin only"
        );
        assert!(!can_manage(Some(&member), &group(false, None)));
    }
}
