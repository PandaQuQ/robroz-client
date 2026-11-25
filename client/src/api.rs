//! Calls the client makes when someone clicks a card, a routine toggle, a
//! chat or 「＋ 新 bro」.
//!
//! All endpoint paths live in [`paths`], so switching to final backend names
//! is a one-place change.
//! - `choice` and routines use robroz#15: `POST /api/choices/:id/answer`,
//!   `GET /api/groups/:id/schedules`, `PATCH /api/schedules/:id`.
//! - Unread uses robroz#26: `GET /api/unread`, `POST /api/groups/:id/read`.
//! - Bros and DMs use robroz#28: `GET /api/bros`, `POST /api/bros`.
//! - The composer uses robroz#39: `POST /api/groups/:id/say` stores the
//!   text and, in a DM or after `@名字` in a group, queues a task (the host
//!   decides who, in one transaction).
//! - `approval` uses robroz#32: `POST /api/approvals/:id/approve` and
//!   `/deny`, no body, answered once (409 after).
//! - `draft` has no backend endpoint yet (phase 2). It uses the UI SPEC §9
//!   suggestion `POST /api/cards/:id/send` until the backend defines it.

use std::sync::Mutex;

use robroz_contract::{
    next_bro_name, parse_skill_slug, route_mention, Answer, AnswerChoice, ApprovalAction,
    ApprovalDecision, Bro, Bros, CreateBro, CreateMemory, CreateReviewRule, CreateSkill, Event,
    McpCommandNotApproved, McpServer, McpServers, McpTestResult, McpTool, McpTools, Memories,
    Memory, MemoryKind, MemorySource, MemoryStatus, Message, MessageKind, RenameBro,
    ReviewRuleCreated, ReviewRules, Said, Say, ScheduleEntry, Schedules, Skill, Skills, Task,
    TaskState, Unread, Unreads, UpdateMemory, UpdateSchedule, UpdateSkill,
    MCP_COMMAND_NOT_APPROVED, SETTINGS_APPROVAL_BOT_ID, SETTINGS_APPROVAL_BOT_NAME,
};
use robroz_contract::{
    ApprovalForbidden, ApprovalForbiddenReason, Me, APPROVAL_FORBIDDEN, FORBIDDEN_NOT_BRO_OWNER,
};
use robroz_contract::{
    ModelSettings, ModelSettingsSource, ModelTestResult, SearchSettings, SearchTestResult,
};
use robroz_ui::cards::group_approval::owner_label;
use robroz_ui::cards::interact::Note;
use robroz_ui::cards::mcp::{Save as McpSave, SecretPut};
use robroz_ui::i18n::{host_text, Key as Text};

use robroz_ui::cards::model as model_page;
use robroz_ui::cards::review_rules;

use crate::conn::Conn;
use crate::event_decode;
use robroz_ui::cards::search;
use robroz_ui::cards::ApprovalExtras;

/// The signed-in person in `--fake` runs; fake cards owned by this id show
/// the owner's buttons, any other id the read-only view.
pub const FAKE_ME_ID: &str = "u-me";
pub const FAKE_ME_NAME: &str = "Panda";

pub mod paths {
    /// v0.2: who this client is signed in as.
    pub const ME: &str = "/api/me";
    /// robroz#62: list and write memories.
    pub const MEMORIES: &str = "/api/memories";
    /// robroz#62: edit (PATCH) or forget (DELETE) one memory.
    pub fn memory(id: &str) -> String {
        format!("/api/memories/{id}")
    }
    /// robroz#71: list and write skills.
    pub const SKILLS: &str = "/api/skills";
    /// robroz#71: edit (PATCH) or forget (DELETE) one skill.
    pub fn skill(slug: &str) -> String {
        format!("/api/skills/{slug}")
    }
    /// robroz#71: enable (PUT) or disable (DELETE) a skill in a group.
    pub fn group_skill(group_id: &str, slug: &str) -> String {
        format!("/api/groups/{group_id}/skills/{slug}")
    }
    /// robroz#75: list skills enabled in this group.
    pub fn group_skills(group_id: &str) -> String {
        format!("/api/groups/{group_id}/skills")
    }
    /// robroz#81: list and create MCP servers.
    pub const MCP_SERVERS: &str = "/api/mcp/servers";
    /// robroz#81: patch or delete one MCP server.
    pub fn mcp_server(id: &str) -> String {
        format!("/api/mcp/servers/{id}")
    }
    pub fn mcp_tools(id: &str) -> String {
        format!("/api/mcp/servers/{id}/tools")
    }
    pub fn mcp_test(id: &str) -> String {
        format!("/api/mcp/servers/{id}/test")
    }
    pub fn mcp_approve_command(id: &str) -> String {
        format!("/api/mcp/servers/{id}/approve-command")
    }
    /// robroz#81: MCP servers enabled in this group.
    pub fn group_mcp(group_id: &str) -> String {
        format!("/api/groups/{group_id}/mcp")
    }
    pub fn group_mcp_server(group_id: &str, server_id: &str) -> String {
        format!("/api/groups/{group_id}/mcp/{server_id}")
    }
    /// robroz#15.
    pub fn answer_choice(id: &str) -> String {
        format!("/api/choices/{id}/answer")
    }
    /// robroz#15.
    pub fn group_schedules(group_id: &str) -> String {
        format!("/api/groups/{group_id}/schedules")
    }
    /// robroz#15.
    pub fn schedule(id: &str) -> String {
        format!("/api/schedules/{id}")
    }
    /// robroz#26.
    pub const UNREAD: &str = "/api/unread";
    /// robroz#26.
    pub fn mark_read(group_id: &str) -> String {
        format!("/api/groups/{group_id}/read")
    }
    /// robroz#28.
    pub const BROS: &str = "/api/bros";
    /// robroz#28: rename a bro (its DM follows).
    pub fn bro(id: &str) -> String {
        format!("/api/bros/{id}")
    }
    /// robroz#39: what the person typed; the host routes it.
    pub fn say(group_id: &str) -> String {
        format!("/api/groups/{group_id}/say")
    }
    /// robroz#32: approve or deny an approval card, once.
    pub fn approval(id: &str) -> String {
        format!("/api/approvals/{id}")
    }
    pub fn approval_decision(id: &str, approve: bool) -> String {
        let verb = if approve { "approve" } else { "deny" };
        format!("/api/approvals/{id}/{verb}")
    }
    /// robroz#76: read (GET) or change (PUT) the search config.
    pub const SEARCH_SETTINGS: &str = "/api/settings/search";
    /// robroz#76: try the effective config once.
    pub const SEARCH_TEST: &str = "/api/settings/search/test";

    /// GET confirmed rules; POST proposes an add (202 + approvalId).
    pub const REVIEW_RULES: &str = "/api/settings/review-rules";
    /// DELETE proposes removing a confirmed rule (202 + approvalId).
    pub fn review_rule(id: &str) -> String {
        format!("/api/settings/review-rules/{id}")
    }
    /// robroz#102: read (GET) or change (PUT) the model config.
    pub const MODEL_SETTINGS: &str = "/api/settings/model";
    /// robroz#102: try the effective config once.
    pub const MODEL_TEST: &str = "/api/settings/model/test";

    /// SPEC §9 suggestion; not in the backend yet.
    pub fn send_draft(id: &str) -> String {
        format!("/api/cards/{id}/send")
    }
}

/// Request bodies, kept as pure functions so their shape is tested.
pub mod bodies {
    use super::*;

    /// `{"choice":[ids],"answeredBy":…}`; an array even for single choice,
    /// `answeredBy` left out when unknown (robroz#43).
    pub fn answer(choice: &[String], by: Option<&str>) -> String {
        serde_json::to_string(&AnswerChoice {
            choice: choice.to_vec(),
            answered_by: by.map(str::to_string),
        })
        .expect("serialize answer")
    }

    /// `/approve` and `/deny`: `{"answeredBy":…}`, or no body when unknown
    /// (the old request, robroz#43).
    pub fn decision(by: Option<&str>) -> Option<String> {
        by.map(|b| {
            serde_json::to_string(&Answer {
                answered_by: Some(b.to_string()),
            })
            .expect("serialize decision")
        })
    }

    /// `{"body":…}`.
    pub fn say(body: &str) -> String {
        serde_json::to_string(&Say {
            body: body.to_string(),
        })
        .expect("serialize say")
    }

    /// `{"name":…}`.
    pub fn rename(name: &str) -> String {
        serde_json::to_string(&RenameBro {
            name: name.to_string(),
        })
        .expect("serialize rename")
    }

    pub fn paused(paused: bool) -> String {
        serde_json::to_string(&UpdateSchedule { paused }).expect("serialize patch")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// 409, e.g. the choice was already answered.
    Conflict(String),
    /// 409 `command_not_approved` from MCP `/test` or `/tools`.
    CommandNotApproved(McpCommandNotApproved),
    /// 400, e.g. an option that is not on the card.
    BadRequest(String),
    NotFound(String),
    Status(u16, String),
    Transport(String),
    Decode(String),
    /// 403 `approval_forbidden` + `not_owner`: only the bro's owner may
    /// decide. The host's owner name when it sent one.
    NotApprovalOwner {
        owner: Option<String>,
    },
    /// 403 `approval_forbidden` + `decided` / `expired`: not an error to
    /// show; the card is fetched again and shows what happened.
    ApprovalSettled,
    /// 403 `not_bro_owner`: only this bro's owner may rename it, open its
    /// DM, or add it to a group. The host's owner name when it sent one.
    NotBroOwner {
        owner: Option<String>,
    },
    /// 401 `token_rejected`.
    TokenRejected,
}

/// A v0.2 error body with a `code`. The client never shows the host's
/// `error` or `message`; it picks its own copy from the code (and
/// `reason` / `ownerName` when those are present).
fn coded_error(body: &str) -> Option<ApiError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CodedBody {
        code: String,
        #[serde(default)]
        owner_name: Option<String>,
    }
    let coded = serde_json::from_str::<CodedBody>(body).ok()?;
    if coded.code.trim().is_empty() {
        return None;
    }
    let owner = coded.owner_name.filter(|n| !n.trim().is_empty());
    Some(match coded.code.as_str() {
        APPROVAL_FORBIDDEN => {
            let refused = serde_json::from_str::<ApprovalForbidden>(body).ok()?;
            match refused.reason {
                ApprovalForbiddenReason::NotOwner => ApiError::NotApprovalOwner { owner },
                ApprovalForbiddenReason::Decided | ApprovalForbiddenReason::Expired => {
                    ApiError::ApprovalSettled
                }
            }
        }
        FORBIDDEN_NOT_BRO_OWNER => ApiError::NotBroOwner { owner },
        "token_rejected" => ApiError::TokenRejected,
        _ => return None,
    })
}

/// The `code` of a v0.2 error body, when it has one.
fn error_code(body: &str) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct CodeOnly {
        code: String,
    }
    let code = serde_json::from_str::<CodeOnly>(body).ok()?.code;
    (!code.trim().is_empty()).then_some(code)
}

/// i18n for a host `code` the client shows. Unknown codes get a generic
/// sentence, never the host's `error` text.
fn copy_for_code(code: &str) -> String {
    let key = match code {
        "token_rejected" => Text::ClientTokenRejected,
        "group_not_found" => Text::HostGroupNotFound,
        "bot_not_in_group" => Text::HostBotNotInGroup,
        "need_text" => Text::HostNeedText,
        "need_name" => Text::HostNeedName,
        "need_title" => Text::HostNeedTitle,
        "need_instruction" => Text::HostNeedInstruction,
        "need_url" => Text::HostNeedUrl,
        "need_model_name" => Text::HostNeedModelName,
        "need_command" => Text::HostNeedCommand,
        "need_secret_name" => Text::HostNeedSecretName,
        "need_secret_value" => Text::HostNeedSecretValue,
        "need_rule" => Text::HostNeedRule,
        "need_action" => Text::HostNeedAction,
        "need_searxng_url" => Text::HostNeedSearxngUrl,
        "bad_request_format" => Text::HostBadRequestFormat,
        "body_not_json" => Text::HostBodyNotJson,
        "body_invalid" => Text::HostBodyInvalid,
        "nothing_to_change" => Text::HostNothingToChange,
        "database_error" => Text::HostDatabaseError,
        "already_answered" => Text::HostAlreadyAnswered,
        "name_too_long" => Text::HostNameTooLong,
        "name_taken" => Text::HostNameTaken,
        "slug_taken" => Text::HostSlugTaken,
        "bad_slug" => Text::HostBadSlug,
        "bro_not_found" => Text::HostBroNotFound,
        "unknown_bro" => Text::HostUnknownBro,
        "task_not_found" => Text::HostTaskNotFound,
        "approval_not_found" => Text::HostApprovalNotFound,
        "memory_not_found" => Text::HostMemoryNotFound,
        "skill_not_found" => Text::HostSkillNotFound,
        "mcp_server_not_found" => Text::HostMcpNotFound,
        "mcp_stdio_only_approve" => Text::HostStdioOnlyApprove,
        "action_not_supported" => Text::HostUnsupportedAction,
        "resume_needs_approval" => Text::HostResumeNeedsApproval,
        "resume_no_intent" => Text::HostResumeNoIntent,
        "resume_marker_failed" => Text::HostResumeMarkerFailed,
        _ => Text::ClientRequestFailed,
    };
    key.into()
}

impl ApiError {
    /// Map a failed request onto the connection form's two cases:
    /// a rejected token, or a host that cannot be reached.
    #[allow(dead_code)] // connection form, follow-up PR
    pub fn as_conn_error(&self) -> crate::conn::ConnError {
        use crate::conn::ConnError;
        match self {
            ApiError::Status(401, _) | ApiError::TokenRejected => ConnError::Unauthorized,
            ApiError::Transport(msg)
            | ApiError::Status(_, msg)
            | ApiError::Conflict(msg)
            | ApiError::BadRequest(msg)
            | ApiError::NotFound(msg)
            | ApiError::Decode(msg) => ConnError::classify(msg),
            ApiError::CommandNotApproved(_)
            | ApiError::NotApprovalOwner { .. }
            | ApiError::NotBroOwner { .. }
            | ApiError::ApprovalSettled => ConnError::classify(&self.reason()),
        }
    }

    /// Plain reason for notes outside cards (send, rename, new bro):
    /// the host's message, or 「连不上主机」.
    pub fn reason(&self) -> String {
        match self {
            ApiError::Transport(_) => Text::ClientNoHost.into(),
            ApiError::CommandNotApproved(_) => robroz_ui::cards::mcp::COMMAND_UNAPPROVED.into(),
            ApiError::NotApprovalOwner { owner } => {
                Text::ApprovalOnlyOwner.fill(&[&owner_label(owner.as_deref())])
            }
            ApiError::NotBroOwner { owner } => {
                Text::BroOnlyOwner.fill(&[&owner_label(owner.as_deref())])
            }
            ApiError::TokenRejected => Text::ClientTokenRejected.into(),
            // Never shown; the card is fetched again instead.
            ApiError::ApprovalSettled => Text::InteractAlreadyDecided.into(),
            ApiError::Conflict(msg)
            | ApiError::BadRequest(msg)
            | ApiError::NotFound(msg)
            | ApiError::Status(_, msg)
            | ApiError::Decode(msg) => host_text(msg).into_owned(),
        }
    }

    /// Footnote for a card whose action failed.
    pub fn note(&self) -> Note {
        match self {
            ApiError::Conflict(_) => Note::AlreadyAnswered,
            ApiError::CommandNotApproved(_) => {
                Note::Failed(robroz_ui::cards::mcp::COMMAND_UNAPPROVED.into())
            }
            ApiError::BadRequest(msg) => Note::Rejected(msg.clone()),
            ApiError::NotApprovalOwner { owner } => Note::OnlyOwner(owner.clone()),
            ApiError::NotBroOwner { owner } => {
                Note::Copy(Text::BroOnlyOwner.fill(&[&owner_label(owner.as_deref())]))
            }
            ApiError::TokenRejected => Note::Copy(Text::ClientTokenRejected.into()),
            ApiError::ApprovalSettled => Note::Settled,
            ApiError::Transport(_) => Note::Retry,
            ApiError::NotFound(msg) | ApiError::Status(_, msg) | ApiError::Decode(msg) => {
                Note::Failed(msg.clone())
            }
        }
    }

    pub fn command_not_approved(&self) -> Option<&McpCommandNotApproved> {
        match self {
            ApiError::CommandNotApproved(denied) => Some(denied),
            _ => None,
        }
    }
}

/// Blocking calls; the app runs them off the UI thread.
pub trait CardApi: Send + Sync {
    /// v0.2: the user this client is signed in as. Older hosts have no
    /// such route; the caller treats any error as "unknown".
    fn me(&self) -> Result<Me, ApiError>;
    /// Every group's unread count and last activity, newest first.
    fn unread(&self) -> Result<Vec<Unread>, ApiError>;
    /// Mark a chat read up to now; returns its new unread state.
    fn mark_read(&self, group_id: &str) -> Result<Unread, ApiError>;
    /// All bros with their DM group.
    fn bros(&self) -> Result<Vec<Bro>, ApiError>;
    /// New bro with the next default name and its DM.
    fn create_bro(&self) -> Result<Bro, ApiError>;
    /// Rename a bro; the host renames its DM too.
    fn rename_bro(&self, id: &str, name: &str) -> Result<Bro, ApiError>;
    /// Composer: store the text; the host queues a task when a bro is meant.
    fn say(&self, group_id: &str, body: &str) -> Result<Said, ApiError>;
    /// Returns the updated `choice` event when the host sends it back.
    fn answer_choice(&self, id: &str, choice: &[String]) -> Result<Option<Event>, ApiError>;
    /// Returns the answered `approval` event when the host sends it back.
    fn decide(
        &self,
        id: &str,
        approve: bool,
    ) -> Result<Option<(Event, Option<ApprovalExtras>)>, ApiError>;
    fn send_draft(&self, id: &str) -> Result<(), ApiError>;
    fn schedules(&self, group_id: &str) -> Result<Vec<ScheduleEntry>, ApiError>;
    fn set_paused(&self, id: &str, paused: bool) -> Result<ScheduleEntry, ApiError>;
    /// 记忆 page: kept and pending memories, newest first (robroz#62).
    fn memories(&self) -> Result<Vec<Memory>, ApiError>;
    /// The person writes one down; the host keeps it at once.
    fn create_memory(&self, scope: &str, kind: MemoryKind, text: &str) -> Result<Memory, ApiError>;
    /// Change a memory's text.
    fn update_memory(&self, id: &str, text: &str) -> Result<Memory, ApiError>;
    /// 忘掉: soft delete; already gone is fine (204).
    fn forget_memory(&self, id: &str) -> Result<(), ApiError>;
    /// 技能 page: kept and pending skills, newest first (robroz#71).
    fn skills(&self) -> Result<Vec<Skill>, ApiError>;
    /// The person writes one down; the host keeps it at once.
    fn create_skill(
        &self,
        scope: &str,
        slug: &str,
        title: &str,
        instruction: &str,
    ) -> Result<Skill, ApiError>;
    /// Change a skill's title and instruction.
    fn update_skill(&self, slug: &str, title: &str, instruction: &str) -> Result<Skill, ApiError>;
    /// 删除: soft delete; already gone is fine (204).
    fn forget_skill(&self, slug: &str) -> Result<(), ApiError>;
    /// Enable or disable a kept skill in this group.
    fn set_skill_enabled(&self, group_id: &str, slug: &str, on: bool) -> Result<(), ApiError>;
    /// Skills this group has turned on (robroz#75). Missing group is 404.
    fn group_skills(&self, group_id: &str) -> Result<Vec<Skill>, ApiError>;
    /// MCP servers, minus secret values (robroz#81).
    fn mcp_servers(&self) -> Result<Vec<McpServer>, ApiError>;
    fn create_mcp_server(&self, save: &McpSave) -> Result<McpServer, ApiError>;
    fn update_mcp_server(&self, save: &McpSave) -> Result<McpServer, ApiError>;
    fn delete_mcp_server(&self, id: &str) -> Result<(), ApiError>;
    fn mcp_tools(&self, id: &str) -> Result<Vec<McpTool>, ApiError>;
    fn test_mcp_server(&self, id: &str) -> Result<McpTestResult, ApiError>;
    fn approve_mcp_command(&self, id: &str) -> Result<McpServer, ApiError>;
    fn set_mcp_enabled(&self, id: &str, on: bool) -> Result<McpServer, ApiError>;
    fn group_mcp(&self, group_id: &str) -> Result<Vec<McpServer>, ApiError>;
    fn set_group_mcp(&self, group_id: &str, server_id: &str, on: bool) -> Result<(), ApiError>;
    /// 联网搜索 page: what is in effect; never the key (robroz#76).
    fn search_settings(&self) -> Result<search::Current, ApiError>;
    fn put_search(&self, put: &search::Put) -> Result<search::Current, ApiError>;
    /// `{ok, error?}`: 「能用了」 or 「连不上：原因」.
    fn test_search(&self) -> Result<(bool, Option<String>), ApiError>;

    /// Confirmed allow rules only.
    fn review_rules(&self) -> Result<Vec<review_rules::Rule>, ApiError>;
    /// Propose adding a rule. Returns the pending approval id (202).
    fn propose_review_rule(&self, text: &str) -> Result<String, ApiError>;
    /// Propose deleting a confirmed rule. Returns the pending approval id (202).
    fn propose_delete_review_rule(&self, id: &str) -> Result<String, ApiError>;
    /// `GET /api/approvals/{id}`: the confirm card, same JSON as the event.
    fn approval(&self, id: &str) -> Result<Event, ApiError>;
    /// 模型 page: what is in effect; never the key (robroz#102).
    fn model_settings(&self) -> Result<model_page::Current, ApiError>;
    fn put_model(&self, put: &model_page::Put) -> Result<model_page::Current, ApiError>;
    fn test_model(&self) -> Result<(bool, Option<String>), ApiError>;
}

/// `GET`/`PUT /api/settings/search` body. `source` is read only when the
/// host sends it; absent or unknown stays `None` (no label, no guess).
fn decode_review_rules(body: &str) -> Result<Vec<review_rules::Rule>, ApiError> {
    let list: ReviewRules = decode(body)?;
    Ok(list
        .rules
        .into_iter()
        .map(|r| review_rules::Rule {
            id: r.id,
            text: r.text,
            created_at: r.created_at,
        })
        .collect())
}

fn decode_approval_id(body: &str) -> Result<String, ApiError> {
    let created: ReviewRuleCreated = decode(body)?;
    if created.approval_id.is_empty() {
        return Err(ApiError::Decode(Text::ClientDecodeNeedsApprovalId.into()));
    }
    Ok(created.approval_id)
}

fn decode_approval_event(body: &str) -> Result<Event, ApiError> {
    event_decode::parse_event_str(body)
        .map(|(event, _)| event)
        .map_err(|e| ApiError::Decode(Text::ClientDecodeFailed.fill(&[&e])))
}

pub fn decode_search(body: &str) -> Result<search::Current, ApiError> {
    let s: SearchSettings = decode(body)?;
    let raw: serde_json::Value = decode(body)?;
    Ok(search::Current {
        provider: s.provider.as_deref().and_then(search::Provider::parse),
        url: s.url,
        has_key: s.has_key,
        key_last4: s.key_last4,
        source: search::Source::parse(raw.get("source").and_then(|v| v.as_str())),
    })
}

pub fn decode_model(body: &str) -> Result<model_page::Current, ApiError> {
    let s: ModelSettings = decode(body)?;
    Ok(model_page::Current {
        base_url: s.base_url,
        model: s.model,
        has_key: s.has_key,
        key_last4: s.key_last4,
        source: s.source.map(|src| match src {
            ModelSettingsSource::Env => model_page::Source::Env,
            ModelSettingsSource::Settings => model_page::Source::Settings,
        }),
    })
}

pub mod model_body {
    use super::model_page::Put;
    /// Missing field: leave; `null`: clear (falls back to env).
    pub fn put(p: &Put) -> String {
        let mut m = serde_json::Map::new();
        if let Some(u) = &p.base_url {
            m.insert(
                "baseUrl".into(),
                u.clone().map_or(serde_json::Value::Null, Into::into),
            );
        }
        if let Some(model) = &p.model {
            m.insert(
                "model".into(),
                model.clone().map_or(serde_json::Value::Null, Into::into),
            );
        }
        if let Some(k) = &p.key {
            m.insert(
                "key".into(),
                k.clone().map_or(serde_json::Value::Null, Into::into),
            );
        }
        serde_json::Value::Object(m).to_string()
    }
}

pub mod search_body {
    use super::search::Put;
    /// Missing `key` leaves it; `null` clears it.
    pub fn put(p: &Put) -> String {
        let mut m = serde_json::Map::new();
        if p.clear {
            return serde_json::json!({"provider": null, "url": null, "key": null}).to_string();
        }
        m.insert("provider".into(), p.provider.as_str().into());
        if let Some(u) = &p.url {
            m.insert("url".into(), u.clone().into());
        }
        if let Some(k) = &p.key {
            m.insert(
                "key".into(),
                k.clone().map_or(serde_json::Value::Null, Into::into),
            );
        }
        serde_json::Value::Object(m).to_string()
    }
}

pub struct HttpApi {
    conn: Conn,
    /// Sent as `answeredBy` when answering cards.
    name: Option<String>,
}

impl HttpApi {
    pub fn new(conn: Conn) -> Self {
        Self { conn, name: None }
    }

    /// Name this client answers cards as (`answeredBy`).
    pub fn answering_as(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    fn finish(
        &self,
        method: &str,
        url: &str,
        result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
    ) -> Result<String, ApiError> {
        let mut response = result.map_err(|e| {
            ApiError::Transport(Text::ClientConnectFailed.fill(&[&self.conn.base(), &e]))
        })?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|e| ApiError::Transport(Text::ClientReadFailed.fill(&[&url, &e])))?;
        if (200..300).contains(&status) {
            return Ok(body);
        }
        if status == 409 {
            if let Ok(denied) = serde_json::from_str::<McpCommandNotApproved>(&body) {
                if denied.error == MCP_COMMAND_NOT_APPROVED {
                    return Err(ApiError::CommandNotApproved(denied));
                }
            }
        }
        if let Some(err) = coded_error(&body) {
            return Err(err);
        }
        let msg = match error_code(&body) {
            // A known-shape body: show the code's i18n, never the host text.
            Some(code) => copy_for_code(&code),
            None => crate::conn::error_text(method, url, status, &body),
        };
        Err(match status {
            400 => ApiError::BadRequest(msg),
            401 => ApiError::Status(401, msg),
            404 => ApiError::NotFound(msg),
            409 => ApiError::Conflict(msg),
            _ => ApiError::Status(status, msg),
        })
    }

    fn post(&self, path: &str, body: Option<String>) -> Result<String, ApiError> {
        let url = self.conn.url(path);
        let req = self.conn.post(path);
        let result = match body {
            Some(b) => req.header("content-type", "application/json").send(b),
            None => req.send_empty(),
        };
        self.finish("POST", &url, result)
    }

    fn patch(&self, path: &str, body: String) -> Result<String, ApiError> {
        let url = self.conn.url(path);
        let result = self
            .conn
            .patch(path)
            .header("content-type", "application/json")
            .send(body);
        self.finish("PATCH", &url, result)
    }

    fn put(&self, path: &str) -> Result<String, ApiError> {
        let url = self.conn.url(path);
        let result = self.conn.put(path).send_empty();
        self.finish("PUT", &url, result)
    }

    fn put_json(&self, path: &str, body: String) -> Result<String, ApiError> {
        let url = self.conn.url(path);
        let result = self
            .conn
            .put(path)
            .header("content-type", "application/json")
            .send(body);
        self.finish("PUT", &url, result)
    }

    fn get(&self, path: &str) -> Result<String, ApiError> {
        let url = self.conn.url(path);
        let result = self.conn.get(path).call();
        self.finish("GET", &url, result)
    }

    fn delete(&self, path: &str) -> Result<String, ApiError> {
        let url = self.conn.url(path);
        let result = self.conn.delete(path).call();
        self.finish("DELETE", &url, result)
    }
}

fn decode<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, ApiError> {
    serde_json::from_str(body).map_err(|e| ApiError::Decode(Text::ClientDecodeFailed.fill(&[&e])))
}

fn mcp_secret_json(s: &SecretPut) -> serde_json::Value {
    let mut v = serde_json::json!({
        "name": s.name,
        "in": s.placement.as_str(),
    });
    match &s.key {
        None => {}
        Some(None) => v["key"] = serde_json::Value::Null,
        Some(Some(k)) => v["key"] = serde_json::json!(k),
    }
    v
}

/// Build a create or patch body. Secret `key` is omitted when unchanged,
/// `null` when cleared — never rely on the contract struct's default
/// serialize, which would turn "leave as-is" into `null`.
fn mcp_save_body(save: &McpSave, patch: bool) -> String {
    let mut v = serde_json::json!({
        "name": save.name,
        "transport": save.transport.as_str(),
    });
    match save.transport {
        robroz_ui::cards::mcp::Transport::Stdio => {
            if let Some(command) = &save.command {
                v["command"] = serde_json::json!(command);
            }
            if let Some(args) = &save.args {
                v["args"] = serde_json::json!(args);
            }
            if patch {
                v["url"] = serde_json::Value::Null;
            }
        }
        robroz_ui::cards::mcp::Transport::Http => {
            if let Some(url) = &save.url {
                v["url"] = serde_json::json!(url);
            }
            if patch {
                v["command"] = serde_json::Value::Null;
            }
        }
    }
    if !save.secrets.is_empty() || patch {
        v["secrets"] = serde_json::Value::Array(save.secrets.iter().map(mcp_secret_json).collect());
    }
    v.to_string()
}

impl CardApi for HttpApi {
    fn me(&self) -> Result<Me, ApiError> {
        decode(&self.get(paths::ME)?)
    }

    fn unread(&self) -> Result<Vec<Unread>, ApiError> {
        Ok(decode::<Unreads>(&self.get(paths::UNREAD)?)?.groups)
    }

    fn mark_read(&self, group_id: &str) -> Result<Unread, ApiError> {
        decode(&self.post(&paths::mark_read(group_id), None)?)
    }

    fn bros(&self) -> Result<Vec<Bro>, ApiError> {
        Ok(decode::<Bros>(&self.get(paths::BROS)?)?.bros)
    }

    fn create_bro(&self) -> Result<Bro, ApiError> {
        let body = serde_json::to_string(&CreateBro::default()).expect("serialize bro");
        decode(&self.post(paths::BROS, Some(body))?)
    }

    fn rename_bro(&self, id: &str, name: &str) -> Result<Bro, ApiError> {
        decode(&self.patch(&paths::bro(id), bodies::rename(name))?)
    }

    fn say(&self, group_id: &str, body: &str) -> Result<Said, ApiError> {
        decode(&self.post(&paths::say(group_id), Some(bodies::say(body)))?)
    }

    fn answer_choice(&self, id: &str, choice: &[String]) -> Result<Option<Event>, ApiError> {
        let body = self.post(
            &paths::answer_choice(id),
            Some(bodies::answer(choice, self.name.as_deref())),
        )?;
        Ok(decode::<Event>(&body).ok())
    }

    fn decide(
        &self,
        id: &str,
        approve: bool,
    ) -> Result<Option<(Event, Option<ApprovalExtras>)>, ApiError> {
        let body = self.post(
            &paths::approval_decision(id, approve),
            bodies::decision(self.name.as_deref()),
        )?;
        if body.trim().is_empty() {
            return Ok(None);
        }
        event_decode::parse_event_str(&body)
            .map(Some)
            .map_err(|e| ApiError::Decode(Text::ClientDecodeFailed.fill(&[&e])))
    }

    fn send_draft(&self, id: &str) -> Result<(), ApiError> {
        self.post(&paths::send_draft(id), None).map(|_| ())
    }

    fn schedules(&self, group_id: &str) -> Result<Vec<ScheduleEntry>, ApiError> {
        let body = self.get(&paths::group_schedules(group_id))?;
        Ok(decode::<Schedules>(&body)?.schedules)
    }

    fn set_paused(&self, id: &str, paused: bool) -> Result<ScheduleEntry, ApiError> {
        decode(&self.patch(&paths::schedule(id), bodies::paused(paused))?)
    }

    fn memories(&self) -> Result<Vec<Memory>, ApiError> {
        Ok(decode::<Memories>(&self.get(paths::MEMORIES)?)?.memories)
    }

    fn create_memory(&self, scope: &str, kind: MemoryKind, text: &str) -> Result<Memory, ApiError> {
        let body = serde_json::to_string(&CreateMemory {
            scope: scope.to_string(),
            kind,
            text: text.to_string(),
        })
        .expect("serialize memory");
        decode(&self.post(paths::MEMORIES, Some(body))?)
    }

    fn update_memory(&self, id: &str, text: &str) -> Result<Memory, ApiError> {
        let body = serde_json::to_string(&UpdateMemory {
            text: Some(text.to_string()),
            ..UpdateMemory::default()
        })
        .expect("serialize memory edit");
        decode(&self.patch(&paths::memory(id), body)?)
    }

    fn forget_memory(&self, id: &str) -> Result<(), ApiError> {
        self.delete(&paths::memory(id)).map(|_| ())
    }

    fn skills(&self) -> Result<Vec<Skill>, ApiError> {
        Ok(decode::<Skills>(&self.get(paths::SKILLS)?)?.skills)
    }

    fn create_skill(
        &self,
        scope: &str,
        slug: &str,
        title: &str,
        instruction: &str,
    ) -> Result<Skill, ApiError> {
        let body = serde_json::to_string(&CreateSkill {
            scope: scope.to_string(),
            slug: slug.to_string(),
            title: title.to_string(),
            instruction: instruction.to_string(),
        })
        .expect("serialize skill");
        decode(&self.post(paths::SKILLS, Some(body))?)
    }

    fn update_skill(&self, slug: &str, title: &str, instruction: &str) -> Result<Skill, ApiError> {
        let body = serde_json::to_string(&UpdateSkill {
            title: Some(title.to_string()),
            instruction: Some(instruction.to_string()),
            ..UpdateSkill::default()
        })
        .expect("serialize skill edit");
        decode(&self.patch(&paths::skill(slug), body)?)
    }

    fn forget_skill(&self, slug: &str) -> Result<(), ApiError> {
        self.delete(&paths::skill(slug)).map(|_| ())
    }

    fn set_skill_enabled(&self, group_id: &str, slug: &str, on: bool) -> Result<(), ApiError> {
        let path = paths::group_skill(group_id, slug);
        if on {
            self.put(&path).map(|_| ())
        } else {
            self.delete(&path).map(|_| ())
        }
    }

    fn group_skills(&self, group_id: &str) -> Result<Vec<Skill>, ApiError> {
        Ok(decode::<Skills>(&self.get(&paths::group_skills(group_id))?)?.skills)
    }

    fn mcp_servers(&self) -> Result<Vec<McpServer>, ApiError> {
        Ok(decode::<McpServers>(&self.get(paths::MCP_SERVERS)?)?.servers)
    }

    fn create_mcp_server(&self, save: &McpSave) -> Result<McpServer, ApiError> {
        decode(&self.post(paths::MCP_SERVERS, Some(mcp_save_body(save, false)))?)
    }

    fn update_mcp_server(&self, save: &McpSave) -> Result<McpServer, ApiError> {
        let id = save
            .id
            .as_deref()
            .ok_or_else(|| ApiError::BadRequest(Text::ClientNoSuchServer.into()))?;
        decode(&self.patch(&paths::mcp_server(id), mcp_save_body(save, true))?)
    }

    fn delete_mcp_server(&self, id: &str) -> Result<(), ApiError> {
        self.delete(&paths::mcp_server(id)).map(|_| ())
    }

    fn mcp_tools(&self, id: &str) -> Result<Vec<McpTool>, ApiError> {
        Ok(decode::<McpTools>(&self.get(&paths::mcp_tools(id))?)?.tools)
    }

    fn test_mcp_server(&self, id: &str) -> Result<McpTestResult, ApiError> {
        decode(&self.post(&paths::mcp_test(id), None)?)
    }

    fn approve_mcp_command(&self, id: &str) -> Result<McpServer, ApiError> {
        decode(&self.post(&paths::mcp_approve_command(id), None)?)
    }

    fn set_mcp_enabled(&self, id: &str, on: bool) -> Result<McpServer, ApiError> {
        decode(&self.patch(&paths::mcp_server(id), format!(r#"{{"enabled":{on}}}"#))?)
    }

    fn group_mcp(&self, group_id: &str) -> Result<Vec<McpServer>, ApiError> {
        Ok(decode::<McpServers>(&self.get(&paths::group_mcp(group_id))?)?.servers)
    }

    fn set_group_mcp(&self, group_id: &str, server_id: &str, on: bool) -> Result<(), ApiError> {
        let path = paths::group_mcp_server(group_id, server_id);
        if on {
            self.put(&path).map(|_| ())
        } else {
            self.delete(&path).map(|_| ())
        }
    }

    fn search_settings(&self) -> Result<search::Current, ApiError> {
        decode_search(&self.get(paths::SEARCH_SETTINGS)?)
    }

    fn put_search(&self, put: &search::Put) -> Result<search::Current, ApiError> {
        decode_search(&self.put_json(paths::SEARCH_SETTINGS, search_body::put(put))?)
    }

    fn test_search(&self) -> Result<(bool, Option<String>), ApiError> {
        let r: SearchTestResult = decode(&self.post(paths::SEARCH_TEST, None)?)?;
        Ok((r.ok, r.error))
    }

    fn review_rules(&self) -> Result<Vec<review_rules::Rule>, ApiError> {
        decode_review_rules(&self.get(paths::REVIEW_RULES)?)
    }

    fn propose_review_rule(&self, text: &str) -> Result<String, ApiError> {
        let n = text.chars().count();
        if n > review_rules::REVIEW_RULE_MAX {
            return Err(ApiError::BadRequest(review_rules::too_long_note(
                n - review_rules::REVIEW_RULE_MAX,
            )));
        }
        let payload = serde_json::to_string(&CreateReviewRule {
            text: text.to_string(),
        })
        .expect("serialize CreateReviewRule");
        let body = self.post(paths::REVIEW_RULES, Some(payload))?;
        decode_approval_id(&body)
    }

    fn propose_delete_review_rule(&self, id: &str) -> Result<String, ApiError> {
        let body = self.delete(&paths::review_rule(id))?;
        decode_approval_id(&body)
    }

    fn approval(&self, id: &str) -> Result<Event, ApiError> {
        decode_approval_event(&self.get(&paths::approval(id))?)
    }

    fn model_settings(&self) -> Result<model_page::Current, ApiError> {
        decode_model(&self.get(paths::MODEL_SETTINGS)?)
    }

    fn put_model(&self, put: &model_page::Put) -> Result<model_page::Current, ApiError> {
        decode_model(&self.put_json(paths::MODEL_SETTINGS, model_body::put(put))?)
    }

    fn test_model(&self) -> Result<(bool, Option<String>), ApiError> {
        let r: ModelTestResult = decode(&self.post(paths::MODEL_TEST, None)?)?;
        Ok((r.ok, r.error))
    }
}

/// Offline API for `--fake` and the gallery: accepts everything and keeps
/// schedules, unread counts and bros in memory.
pub struct FakeApi {
    memories: Mutex<Vec<Memory>>,
    skills: Mutex<Vec<Skill>>,
    skill_enabled: Mutex<std::collections::HashSet<(String, String)>>,
    mcp_servers: Mutex<Vec<McpServer>>,
    mcp_group: Mutex<std::collections::HashSet<(String, String)>>,
    search: Mutex<search::Current>,

    review_rules: Mutex<Vec<review_rules::Rule>>,
    pending_review: Mutex<Vec<FakeRuleApproval>>,
    model: Mutex<model_page::Current>,

    schedules: Mutex<Vec<ScheduleEntry>>,
    unread: Mutex<Vec<Unread>>,
    bros: Mutex<Vec<Bro>>,
}

impl FakeApi {
    pub fn new(schedules: Vec<ScheduleEntry>) -> Self {
        Self {
            memories: Mutex::new(Vec::new()),
            skills: Mutex::new(Vec::new()),
            skill_enabled: Mutex::new(std::collections::HashSet::new()),
            mcp_servers: Mutex::new(Vec::new()),
            mcp_group: Mutex::new(std::collections::HashSet::new()),
            search: Mutex::new(search::Current::default()),

            review_rules: Mutex::new(Vec::new()),
            pending_review: Mutex::new(Vec::new()),
            model: Mutex::new(model_page::Current::default()),

            schedules: Mutex::new(schedules),
            unread: Mutex::new(Vec::new()),
            bros: Mutex::new(Vec::new()),
        }
    }

    /// Seed unread counts and DMs for the fake chats.
    pub fn with_chats(self, unread: Vec<Unread>, bros: Vec<Bro>) -> Self {
        *self.unread.lock().expect("fake unread lock") = unread;
        *self.bros.lock().expect("fake bros lock") = bros;
        self
    }

    /// Seed the 模型 page for `--fake` screenshots or a review script.
    #[allow(dead_code)]
    pub fn with_model(self, cur: model_page::Current) -> Self {
        *self.model.lock().expect("fake model lock") = cur;
        self
    }

    /// Seed confirmed allow-rules for `--fake` screenshots or a review script.
    #[allow(dead_code)]
    pub fn with_review_rules(self, rules: Vec<review_rules::Rule>) -> Self {
        *self.review_rules.lock().expect("fake review rules lock") = rules;
        self
    }
}

impl CardApi for FakeApi {
    fn me(&self) -> Result<Me, ApiError> {
        Ok(Me {
            id: FAKE_ME_ID.into(),
            name: FAKE_ME_NAME.into(),
            role: robroz_contract::UserRole::Admin,
        })
    }

    fn unread(&self) -> Result<Vec<Unread>, ApiError> {
        let mut all = self.unread.lock().expect("fake unread lock").clone();
        all.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));
        Ok(all)
    }

    fn mark_read(&self, group_id: &str) -> Result<Unread, ApiError> {
        let mut all = self.unread.lock().expect("fake unread lock");
        let u = all
            .iter_mut()
            .find(|u| u.group_id == group_id)
            .ok_or_else(|| ApiError::NotFound("群不存在".into()))?;
        u.unread = 0;
        u.read_at = Some(chrono::Utc::now().to_rfc3339());
        Ok(u.clone())
    }

    fn bros(&self) -> Result<Vec<Bro>, ApiError> {
        Ok(self.bros.lock().expect("fake bros lock").clone())
    }

    fn create_bro(&self) -> Result<Bro, ApiError> {
        let mut bros = self.bros.lock().expect("fake bros lock");
        let n = bros.len() + 1;
        let bro = Bro {
            id: format!("fake-bro-{n}"),
            name: next_bro_name(bros.iter().map(|b| b.name.as_str())),
            dm_group_id: Some(format!("fake-dm-{n}")),
            owner_user_id: None,
        };
        bros.push(bro.clone());
        self.unread.lock().expect("fake unread lock").push(Unread {
            group_id: format!("fake-dm-{n}"),
            read_at: None,
            unread: 0,
            last_activity_at: Some(chrono::Utc::now().to_rfc3339()),
        });
        Ok(bro)
    }

    fn rename_bro(&self, id: &str, name: &str) -> Result<Bro, ApiError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(ApiError::BadRequest("请填写名称".into()));
        }
        if name.chars().count() > robroz_contract::BRO_NAME_MAX {
            return Err(ApiError::BadRequest("名字太长".into()));
        }
        let mut bros = self.bros.lock().expect("fake bros lock");
        let bro = bros
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or_else(|| ApiError::NotFound("bro 不存在".into()))?;
        bro.name = name.to_string();
        Ok(bro.clone())
    }

    /// Same rule as the host: a DM goes to its bro; a group routes a
    /// leading `@名字` (the fake treats every known bro as a member).
    fn say(&self, group_id: &str, body: &str) -> Result<Said, ApiError> {
        let body = body.trim();
        if body.is_empty() {
            return Err(ApiError::BadRequest("请填写内容".into()));
        }
        let bros = self.bros.lock().expect("fake bros lock");
        let route = match bros
            .iter()
            .find(|b| b.dm_group_id.as_deref() == Some(group_id))
        {
            Some(bro) => Some((bro.id.clone(), body.to_string())),
            None => route_mention(body, bros.iter().map(|b| (b.id.as_str(), b.name.as_str()))),
        };
        Ok(Said {
            message: Message {
                id: format!("fake-msg-{}", uuid_ish()),
                kind: MessageKind::Text,
                body: body.to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                author_name: None,
                author_user_id: None,
            },
            task: route.map(|(bot, prompt)| Task {
                id: format!("fake-task-{}", uuid_ish()),
                assignee_bot_id: bot,
                prompt,
                state: TaskState::Queued,
            }),
            appended_to_task_id: None,
        })
    }

    fn answer_choice(&self, _: &str, _: &[String]) -> Result<Option<Event>, ApiError> {
        Ok(None)
    }

    fn decide(
        &self,
        id: &str,
        approve: bool,
    ) -> Result<Option<(Event, Option<ApprovalExtras>)>, ApiError> {
        if let Some(reply) = decide_fake_review_rule(self, id, approve)? {
            return Ok(Some(reply));
        }
        Ok(None)
    }

    fn send_draft(&self, _: &str) -> Result<(), ApiError> {
        Ok(())
    }

    fn schedules(&self, group_id: &str) -> Result<Vec<ScheduleEntry>, ApiError> {
        let all = self.schedules.lock().expect("fake schedules lock");
        Ok(all
            .iter()
            .filter(|s| s.schedule.group_id == group_id)
            .cloned()
            .collect())
    }

    fn set_paused(&self, id: &str, paused: bool) -> Result<ScheduleEntry, ApiError> {
        let mut all = self.schedules.lock().expect("fake schedules lock");
        let entry = all
            .iter_mut()
            .find(|s| s.schedule.id == id)
            .ok_or_else(|| ApiError::NotFound("例行任务不存在".into()))?;
        entry.paused = paused;
        Ok(entry.clone())
    }

    fn memories(&self) -> Result<Vec<Memory>, ApiError> {
        let mut all = self.memories.lock().expect("fake memories lock").clone();
        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(all)
    }

    fn create_memory(&self, scope: &str, kind: MemoryKind, text: &str) -> Result<Memory, ApiError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let memory = Memory {
            id: format!("m-{}", uuid_ish()),
            scope: scope.to_string(),
            kind,
            text: text.to_string(),
            source: MemorySource::User,
            status: MemoryStatus::Kept,
            created_at: now.clone(),
            updated_at: now,
            confirm_id: None,
        };
        self.memories
            .lock()
            .expect("fake memories lock")
            .push(memory.clone());
        Ok(memory)
    }

    fn update_memory(&self, id: &str, text: &str) -> Result<Memory, ApiError> {
        let mut all = self.memories.lock().expect("fake memories lock");
        let m = all
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| ApiError::NotFound("没有这条记忆".into()))?;
        m.text = text.to_string();
        m.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Ok(m.clone())
    }

    fn forget_memory(&self, id: &str) -> Result<(), ApiError> {
        self.memories
            .lock()
            .expect("fake memories lock")
            .retain(|m| m.id != id);
        Ok(())
    }

    fn skills(&self) -> Result<Vec<Skill>, ApiError> {
        let mut all = self.skills.lock().expect("fake skills lock").clone();
        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(all)
    }

    fn create_skill(
        &self,
        scope: &str,
        slug: &str,
        title: &str,
        instruction: &str,
    ) -> Result<Skill, ApiError> {
        let slug = parse_skill_slug(slug)
            .ok_or_else(|| ApiError::BadRequest("slug 只能是小写字母、数字和连字符".into()))?
            .to_string();
        let title = title.trim();
        let instruction = instruction.trim();
        if title.is_empty() {
            return Err(ApiError::BadRequest("请填写标题".into()));
        }
        if instruction.is_empty() {
            return Err(ApiError::BadRequest("请填写说明".into()));
        }
        let mut all = self.skills.lock().expect("fake skills lock");
        if all.iter().any(|s| s.slug == slug) {
            return Err(ApiError::Conflict("这个 slug 已经有了".into()));
        }
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let skill = Skill {
            id: format!("s-{}", uuid_ish()),
            scope: scope.to_string(),
            slug,
            title: title.to_string(),
            instruction: instruction.to_string(),
            source: MemorySource::User,
            status: MemoryStatus::Kept,
            created_at: now.clone(),
            updated_at: now,
            confirm_id: None,
        };
        all.push(skill.clone());
        Ok(skill)
    }

    fn update_skill(&self, slug: &str, title: &str, instruction: &str) -> Result<Skill, ApiError> {
        let mut all = self.skills.lock().expect("fake skills lock");
        let s = all
            .iter_mut()
            .find(|s| s.slug == slug)
            .ok_or_else(|| ApiError::NotFound("技能不存在".into()))?;
        s.title = title.to_string();
        s.instruction = instruction.to_string();
        s.updated_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        Ok(s.clone())
    }

    fn forget_skill(&self, slug: &str) -> Result<(), ApiError> {
        let mut all = self.skills.lock().expect("fake skills lock");
        let before = all.len();
        all.retain(|s| s.slug != slug);
        if all.len() == before {
            // Already gone is fine, like the host's second DELETE.
            return Ok(());
        }
        self.skill_enabled
            .lock()
            .expect("fake skill enable lock")
            .retain(|(_, s)| s != slug);
        Ok(())
    }

    fn set_skill_enabled(&self, group_id: &str, slug: &str, on: bool) -> Result<(), ApiError> {
        let all = self.skills.lock().expect("fake skills lock");
        if !all.iter().any(|s| s.slug == slug) {
            return Err(ApiError::NotFound("技能不存在".into()));
        }
        drop(all);
        let mut en = self.skill_enabled.lock().expect("fake skill enable lock");
        let key = (group_id.to_string(), slug.to_string());
        if on {
            en.insert(key);
        } else {
            en.remove(&key);
        }
        Ok(())
    }

    fn group_skills(&self, group_id: &str) -> Result<Vec<Skill>, ApiError> {
        let enabled = self
            .skill_enabled
            .lock()
            .expect("fake skill enable lock")
            .clone();
        let mut all = self.skills.lock().expect("fake skills lock").clone();
        all.retain(|s| enabled.contains(&(group_id.to_string(), s.slug.clone())));
        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(all)
    }

    fn mcp_servers(&self) -> Result<Vec<McpServer>, ApiError> {
        let mut all = self.mcp_servers.lock().expect("fake mcp lock").clone();
        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(all)
    }

    fn create_mcp_server(&self, save: &McpSave) -> Result<McpServer, ApiError> {
        let mut all = self.mcp_servers.lock().expect("fake mcp lock");
        if all.iter().any(|s| s.name == save.name) {
            return Err(ApiError::Conflict("这个名字已经有了".into()));
        }
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let server = fake_mcp_from_save(save, format!("mcp-{}", uuid_ish()), now);
        all.push(server.clone());
        Ok(server)
    }

    fn update_mcp_server(&self, save: &McpSave) -> Result<McpServer, ApiError> {
        let id = save
            .id
            .as_deref()
            .ok_or_else(|| ApiError::BadRequest("没有这台服务器".into()))?;
        let mut all = self.mcp_servers.lock().expect("fake mcp lock");
        let pos = all
            .iter()
            .position(|s| s.id == id)
            .ok_or_else(|| ApiError::NotFound("MCP 服务器不存在".into()))?;
        if all.iter().any(|s| s.name == save.name && s.id != id) {
            return Err(ApiError::Conflict("这个名字已经有了".into()));
        }
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let mut server = fake_mcp_from_save(save, id.to_string(), now);
        let prev = &all[pos];
        server.created_at = prev.created_at.clone();
        server.enabled = prev.enabled;
        let command_changed = server.transport != prev.transport
            || server.command != prev.command
            || server.args != prev.args;
        if server.transport.eq_ignore_ascii_case("stdio") && !command_changed {
            server.command_approved = prev.command_approved;
            server.command_approved_at = prev.command_approved_at.clone();
        }
        // merge secrets: omitted keys keep last4
        let prev = all[pos].secrets.clone();
        for (i, put) in save.secrets.iter().enumerate() {
            if put.key.is_none() {
                let kept = prev
                    .iter()
                    .find(|s| s.name == put.name)
                    .or(server.secrets.get(i))
                    .cloned();
                if let Some(old) = kept {
                    if let Some(slot) = server.secrets.iter_mut().find(|s| s.name == put.name) {
                        slot.has_key = old.has_key;
                        slot.key_last4 = old.key_last4.clone();
                    }
                }
            }
        }
        all[pos] = server.clone();
        Ok(server)
    }

    fn delete_mcp_server(&self, id: &str) -> Result<(), ApiError> {
        let mut all = self.mcp_servers.lock().expect("fake mcp lock");
        let before = all.len();
        all.retain(|s| s.id != id);
        if all.len() == before {
            return Ok(());
        }
        self.mcp_group
            .lock()
            .expect("fake mcp group lock")
            .retain(|(_, s)| s != id);
        Ok(())
    }

    fn mcp_tools(&self, id: &str) -> Result<Vec<McpTool>, ApiError> {
        let all = self.mcp_servers.lock().expect("fake mcp lock");
        let s = all
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| ApiError::NotFound("MCP 服务器不存在".into()))?;
        if let Some(err) = fake_mcp_unapproved(s) {
            return Err(err);
        }
        if s.command.as_deref() == Some("/fail") {
            return Err(ApiError::Status(502, "连不上这台服务器".into()));
        }
        Ok(fake_mcp_tools())
    }

    fn test_mcp_server(&self, id: &str) -> Result<McpTestResult, ApiError> {
        let all = self.mcp_servers.lock().expect("fake mcp lock");
        let s = all
            .iter()
            .find(|s| s.id == id)
            .ok_or_else(|| ApiError::NotFound("MCP 服务器不存在".into()))?;
        if let Some(err) = fake_mcp_unapproved(s) {
            return Err(err);
        }
        if s.command.as_deref() == Some("/fail") {
            return Ok(McpTestResult {
                ok: false,
                error: Some("连不上这台服务器".into()),
                tool_count: None,
            });
        }
        Ok(McpTestResult {
            ok: true,
            error: None,
            tool_count: Some(2),
        })
    }

    fn approve_mcp_command(&self, id: &str) -> Result<McpServer, ApiError> {
        let mut all = self.mcp_servers.lock().expect("fake mcp lock");
        let s = all
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| ApiError::NotFound("MCP 服务器不存在".into()))?;
        if !s.transport.eq_ignore_ascii_case("stdio") {
            return Err(ApiError::BadRequest(
                "只有 stdio 服务器需要批准启动命令".into(),
            ));
        }
        if s.command
            .as_ref()
            .map(|c| c.trim().is_empty())
            .unwrap_or(true)
        {
            return Err(ApiError::BadRequest("请填写启动命令".into()));
        }
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        s.command_approved = Some(true);
        s.command_approved_at = Some(now.clone());
        s.updated_at = now;
        Ok(s.clone())
    }

    fn set_mcp_enabled(&self, id: &str, on: bool) -> Result<McpServer, ApiError> {
        let mut all = self.mcp_servers.lock().expect("fake mcp lock");
        let s = all
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| ApiError::NotFound("MCP 服务器不存在".into()))?;
        s.enabled = on;
        Ok(s.clone())
    }

    fn group_mcp(&self, group_id: &str) -> Result<Vec<McpServer>, ApiError> {
        let enabled = self.mcp_group.lock().expect("fake mcp group lock").clone();
        let mut all = self.mcp_servers.lock().expect("fake mcp lock").clone();
        all.retain(|s| enabled.contains(&(group_id.to_string(), s.id.clone())));
        Ok(all)
    }

    fn set_group_mcp(&self, group_id: &str, server_id: &str, on: bool) -> Result<(), ApiError> {
        let all = self.mcp_servers.lock().expect("fake mcp lock");
        if !all.iter().any(|s| s.id == server_id) {
            return Err(ApiError::NotFound("MCP 服务器不存在".into()));
        }
        drop(all);
        let mut en = self.mcp_group.lock().expect("fake mcp group lock");
        let key = (group_id.to_string(), server_id.to_string());
        if on {
            en.insert(key);
        } else {
            en.remove(&key);
        }
        Ok(())
    }

    fn search_settings(&self) -> Result<search::Current, ApiError> {
        Ok(self.search.lock().expect("fake search lock").clone())
    }

    fn put_search(&self, put: &search::Put) -> Result<search::Current, ApiError> {
        let mut cur = self.search.lock().expect("fake search lock");
        if put.clear {
            *cur = search::Current::default();
            return Ok(cur.clone());
        }
        cur.provider = Some(put.provider);
        if put.url.is_some() {
            cur.url = put.url.clone();
        }
        match &put.key {
            Some(Some(k)) => {
                cur.has_key = true;
                cur.key_last4 = Some(
                    k.chars()
                        .rev()
                        .take(4)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect(),
                );
            }
            Some(None) => {
                cur.has_key = false;
                cur.key_last4 = None;
            }
            None => {}
        }
        cur.source = Some(search::Source::Settings);
        Ok(cur.clone())
    }

    fn test_search(&self) -> Result<(bool, Option<String>), ApiError> {
        let cur = self.search.lock().expect("fake search lock");
        Ok(match cur.provider {
            Some(search::Provider::Brave) if !cur.has_key => (false, Some("还没填 key".into())),
            Some(_) => (true, None),
            None => (false, Some(search::NOT_SET.into())),
        })
    }

    fn review_rules(&self) -> Result<Vec<review_rules::Rule>, ApiError> {
        Ok(self
            .review_rules
            .lock()
            .expect("fake review rules lock")
            .clone())
    }

    fn propose_review_rule(&self, text: &str) -> Result<String, ApiError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ApiError::BadRequest(review_rules::NEED_TEXT.into()));
        }
        let n = text.chars().count();
        if n > review_rules::REVIEW_RULE_MAX {
            return Err(ApiError::BadRequest(review_rules::too_long_note(
                n - review_rules::REVIEW_RULE_MAX,
            )));
        }
        let id = format!("apr-add-{}", uuid_ish());
        self.pending_review
            .lock()
            .expect("fake pending review lock")
            .push(FakeRuleApproval {
                id: id.clone(),
                text: text.to_string(),
                delete_id: None,
                decided: false,
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            });
        Ok(id)
    }

    fn propose_delete_review_rule(&self, id: &str) -> Result<String, ApiError> {
        let all = self.review_rules.lock().expect("fake review rules lock");
        let Some(rule) = all.iter().find(|r| r.id == id) else {
            return Err(ApiError::NotFound("规则不存在".into()));
        };
        let approval_id = format!("apr-del-{}", uuid_ish());
        self.pending_review
            .lock()
            .expect("fake pending review lock")
            .push(FakeRuleApproval {
                id: approval_id.clone(),
                text: rule.text.clone(),
                delete_id: Some(id.to_string()),
                decided: false,
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            });
        Ok(approval_id)
    }

    fn approval(&self, id: &str) -> Result<Event, ApiError> {
        let pending = self
            .pending_review
            .lock()
            .expect("fake pending review lock");
        let Some(p) = pending.iter().find(|p| p.id == id) else {
            return Err(ApiError::NotFound("审批卡不存在".into()));
        };
        if p.decided {
            // Host returns the settled card; we only need pending cards on
            // the settings page after 202, so treat decided as gone.
            return Err(ApiError::NotFound("审批卡不存在".into()));
        }
        Ok(fake_review_event(
            &p.id,
            &p.text,
            p.delete_id.is_some(),
            &p.created_at,
            None,
        ))
    }

    fn model_settings(&self) -> Result<model_page::Current, ApiError> {
        Ok(self.model.lock().expect("fake model lock").clone())
    }

    fn put_model(&self, put: &model_page::Put) -> Result<model_page::Current, ApiError> {
        let mut cur = self.model.lock().expect("fake model lock");
        if let Some(url) = &put.base_url {
            match url {
                None => cur.base_url = None,
                Some(raw) => {
                    let url = model_page::normalize_url(raw).to_string();
                    if url.is_empty() {
                        return Err(ApiError::BadRequest(model_page::NEED_URL.into()));
                    }
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        return Err(ApiError::BadRequest(model_page::BAD_URL.into()));
                    }
                    cur.base_url = Some(url);
                }
            }
        }
        if let Some(name) = &put.model {
            match name {
                None => cur.model = None,
                Some(raw) => {
                    let name = raw.trim().to_string();
                    if name.is_empty() {
                        return Err(ApiError::BadRequest(model_page::NEED_MODEL.into()));
                    }
                    cur.model = Some(name);
                }
            }
        }
        match &put.key {
            Some(Some(k)) => {
                if k.is_empty() {
                    return Err(ApiError::BadRequest("密钥不能为空".into()));
                }
                if k.chars().any(char::is_whitespace) {
                    return Err(ApiError::BadRequest("密钥里不能有空白".into()));
                }
                cur.has_key = true;
                cur.key_last4 = Some(
                    k.chars()
                        .rev()
                        .take(4)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect(),
                );
            }
            Some(None) => {
                cur.has_key = false;
                cur.key_last4 = None;
            }
            None => {}
        }
        cur.source = if cur.base_url.is_none() && cur.model.is_none() && !cur.has_key {
            None
        } else {
            Some(model_page::Source::Settings)
        };
        Ok(cur.clone())
    }

    fn test_model(&self) -> Result<(bool, Option<String>), ApiError> {
        let cur = self.model.lock().expect("fake model lock");
        Ok(if cur.base_url.is_some() && cur.model.is_some() {
            (true, None)
        } else {
            (false, Some(model_page::NOT_SET.into()))
        })
    }
}

fn fake_mcp_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "ping".into(),
            description: Some("Check the server is up".into()),
            needs_approval: false,
        },
        McpTool {
            name: "write_note".into(),
            description: None,
            needs_approval: true,
        },
    ]
}

fn fake_mcp_from_save(save: &McpSave, id: String, now: String) -> McpServer {
    use robroz_contract::McpSecretPublic;
    let secrets = save
        .secrets
        .iter()
        .filter_map(|s| match &s.key {
            Some(None) => None,
            Some(Some(k)) => Some(McpSecretPublic {
                name: s.name.clone(),
                placement: s.placement,
                has_key: true,
                key_last4: last4(k),
            }),
            None => Some(McpSecretPublic {
                name: s.name.clone(),
                placement: s.placement,
                has_key: false,
                key_last4: None,
            }),
        })
        .collect();
    McpServer {
        id,
        name: save.name.clone(),
        transport: save.transport.as_str().into(),
        command: save.command.clone(),
        args: save.args.clone(),
        url: save.url.clone(),
        secrets,
        enabled: true,
        tool_count: None,
        created_at: now.clone(),
        updated_at: now,
        command_approved: if save.transport.as_str() == "stdio" {
            Some(false)
        } else {
            None
        },
        command_approved_at: None,
    }
}

fn fake_mcp_unapproved(s: &McpServer) -> Option<ApiError> {
    if s.transport.eq_ignore_ascii_case("stdio") && s.command_approved != Some(true) {
        Some(ApiError::CommandNotApproved(McpCommandNotApproved::new(
            s.command.clone().unwrap_or_default(),
            s.args.clone().unwrap_or_default(),
        )))
    } else {
        None
    }
}

fn last4(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let n = value.chars().count();
    Some(value.chars().skip(n.saturating_sub(4)).collect())
}

struct FakeRuleApproval {
    id: String,
    text: String,
    delete_id: Option<String>,
    decided: bool,
    created_at: String,
}

fn decide_fake_review_rule(
    api: &FakeApi,
    id: &str,
    approve: bool,
) -> Result<Option<(Event, Option<ApprovalExtras>)>, ApiError> {
    let mut pending = api.pending_review.lock().expect("fake pending review lock");
    let Some(idx) = pending.iter().position(|p| p.id == id) else {
        return Ok(None);
    };
    if pending[idx].decided {
        return Err(ApiError::Conflict(review_rules::ALREADY_ANSWERED.into()));
    }
    pending[idx].decided = true;
    let text = pending[idx].text.clone();
    let delete_id = pending[idx].delete_id.clone();
    drop(pending);
    if approve {
        let mut rules = api.review_rules.lock().expect("fake review rules lock");
        if let Some(del) = &delete_id {
            rules.retain(|r| &r.id != del);
        } else {
            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            rules.push(review_rules::Rule {
                id: format!("r-{}", uuid_ish()),
                text: text.clone(),
                created_at: now,
            });
        }
    }
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    Ok(Some((
        fake_review_event(id, &text, delete_id.is_some(), &now, Some(approve)),
        None,
    )))
}

fn fake_review_event(
    id: &str,
    text: &str,
    delete: bool,
    now: &str,
    approve: Option<bool>,
) -> Event {
    let expires = chrono::Utc::now() + chrono::Duration::seconds(86_400);
    Event::Approval {
        id: id.to_string(),
        bot_id: SETTINGS_APPROVAL_BOT_ID.into(),
        bot_name: SETTINGS_APPROVAL_BOT_NAME.into(),
        action: if delete {
            ApprovalAction::RuleDelete
        } else {
            ApprovalAction::RuleAdd
        },
        summary: text.to_string(),
        path: None,
        decision: approve.map(|ok| {
            if ok {
                ApprovalDecision::Approved
            } else {
                ApprovalDecision::Denied
            }
        }),
        created_at: now.to_string(),
        answered_at: approve.map(|_| now.to_string()),
        answered_by: approve.map(|_| "我".into()),
        task_id: None,
        mcp_server: None,
        mcp_tool: None,
        reason: None,
        expires_at: Some(expires.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)),
        owner_name: None,
        owner_user_id: None,
    }
}

/// Unique-enough id for fake echoes.
fn uuid_ish() -> u128 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    nanos ^ u128::from(N.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_403_is_told_apart_by_code_and_reason() {
        let body = |reason: &str| {
            format!(
                r#"{{"error":"x","code":"approval_forbidden","reason":"{reason}","ownerName":"Lin"}}"#
            )
        };
        assert_eq!(
            coded_error(&body("not_owner")).map(|e| e.note()),
            Some(Note::OnlyOwner(Some("Lin".into())))
        );
        for settled in ["decided", "expired"] {
            assert_eq!(
                coded_error(&body(settled)).map(|e| e.note()),
                Some(Note::Settled)
            );
        }
        // Any other code, or not JSON: left to the HTTP-status mapping.
        assert!(coded_error(r#"{"error":"x","code":"other"}"#).is_none());
        assert!(coded_error("nope").is_none());
        // No owner name: the note's copy falls back to "its owner".
        let none = r#"{"error":"x","code":"approval_forbidden","reason":"not_owner"}"#;
        assert_eq!(
            coded_error(none).map(|e| e.note()),
            Some(Note::OnlyOwner(None))
        );
        assert_eq!(
            Note::OnlyOwner(None).text(),
            "Only its owner can approve this."
        );
    }

    #[test]
    fn a_coded_error_never_shows_the_host_message() {
        let not_owner =
            r#"{"error":"只有这个 bro 的主人能这样做","code":"not_bro_owner","ownerName":"Lin"}"#;
        match coded_error(not_owner) {
            Some(ApiError::NotBroOwner { owner }) => {
                assert_eq!(owner.as_deref(), Some("Lin"));
                assert_eq!(
                    ApiError::NotBroOwner { owner }.reason(),
                    "Only Lin can change this bro."
                );
            }
            other => panic!("{other:?}"),
        }
        let missing = r#"{"error":"只有这个 bro 的主人能这样做","code":"not_bro_owner"}"#;
        assert_eq!(
            coded_error(missing).map(|e| e.reason()).as_deref(),
            Some("Only its owner can change this bro.")
        );
        let token = r#"{"error":"缺少或错误的令牌","code":"token_rejected"}"#;
        match coded_error(token) {
            Some(ApiError::TokenRejected) => {
                assert_eq!(
                    ApiError::TokenRejected.reason(),
                    "The host rejected the token. Check ROBROZ_TOKEN."
                );
                assert!(!ApiError::TokenRejected.reason().contains("令牌"));
            }
            other => panic!("{other:?}"),
        }
        let other = r#"{"error":"这句主机文案绝不能出现","code":"group_not_found"}"#;
        assert!(
            coded_error(other).is_none(),
            "ordinary codes stay on the HTTP variant"
        );
        assert_eq!(error_code(other).as_deref(), Some("group_not_found"));
        assert_eq!(copy_for_code("group_not_found"), "Group not found");
        assert!(!copy_for_code("group_not_found").contains("绝不能"));
        assert_eq!(copy_for_code("no_such_code"), "The request failed.");
        assert!(!copy_for_code("no_such_code").contains("主机"));
        assert!(coded_error("not json").is_none());
        assert!(coded_error(r#"{"error":"x"}"#).is_none());
    }

    #[test]
    fn the_fake_api_signs_in_as_the_fake_owner() {
        let me = FakeApi::new(Vec::new()).me().unwrap();
        assert_eq!(me.id, FAKE_ME_ID);
    }

    use super::*;

    #[test]
    fn paths_follow_backend_and_spec() {
        assert_eq!(paths::answer_choice("c1"), "/api/choices/c1/answer");
        assert_eq!(paths::group_schedules("g1"), "/api/groups/g1/schedules");
        assert_eq!(paths::schedule("s1"), "/api/schedules/s1");
        assert_eq!(
            paths::approval_decision("a1", true),
            "/api/approvals/a1/approve"
        );
        assert_eq!(
            paths::approval_decision("a1", false),
            "/api/approvals/a1/deny"
        );
        assert_eq!(paths::send_draft("d1"), "/api/cards/d1/send");
        assert_eq!(paths::SKILLS, "/api/skills");
        assert_eq!(paths::skill("weekly-report"), "/api/skills/weekly-report");
        assert_eq!(
            paths::group_skill("g1", "weekly-report"),
            "/api/groups/g1/skills/weekly-report"
        );
        assert_eq!(paths::group_skills("g1"), "/api/groups/g1/skills");
        assert_eq!(paths::MCP_SERVERS, "/api/mcp/servers");
        assert_eq!(paths::mcp_server("s1"), "/api/mcp/servers/s1");
        assert_eq!(paths::mcp_tools("s1"), "/api/mcp/servers/s1/tools");
        assert_eq!(paths::mcp_test("s1"), "/api/mcp/servers/s1/test");
        assert_eq!(
            paths::mcp_approve_command("s1"),
            "/api/mcp/servers/s1/approve-command"
        );
        assert_eq!(paths::group_mcp("g1"), "/api/groups/g1/mcp");
        assert_eq!(paths::group_mcp_server("g1", "s1"), "/api/groups/g1/mcp/s1");
        assert_eq!(paths::UNREAD, "/api/unread");
        assert_eq!(paths::mark_read("g1"), "/api/groups/g1/read");
        assert_eq!(paths::BROS, "/api/bros");
        assert_eq!(paths::say("g1"), "/api/groups/g1/say");
        assert_eq!(paths::bro("b1"), "/api/bros/b1");
        assert_eq!(bodies::rename("Ada"), r#"{"name":"Ada"}"#);
        assert_eq!(paths::SEARCH_SETTINGS, "/api/settings/search");
        assert_eq!(paths::SEARCH_TEST, "/api/settings/search/test");

        assert_eq!(paths::REVIEW_RULES, "/api/settings/review-rules");
        assert_eq!(paths::review_rule("r1"), "/api/settings/review-rules/r1");
        assert_eq!(paths::approval("apr-1"), "/api/approvals/apr-1");
        assert_eq!(paths::MODEL_SETTINGS, "/api/settings/model");
        assert_eq!(paths::MODEL_TEST, "/api/settings/model/test");
    }

    #[test]
    fn review_rules_decode_and_approval_id() {
        let rules = decode_review_rules(
            r#"{"rules":[{"id":"r1","text":"允许发周报","createdAt":"2026-11-20T08:00:00Z"}]}"#,
        )
        .unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "r1");
        assert_eq!(rules[0].text, "允许发周报");
        assert_eq!(rules[0].created_at, "2026-11-20T08:00:00Z");
        assert_eq!(
            decode_approval_id(r#"{"approvalId":"apr-1"}"#).unwrap(),
            "apr-1"
        );
        assert!(decode_approval_id(r#"{"id":"apr-1"}"#).is_err());
        assert!(
            decode_review_rules(r#"[{"id":"r1","text":"允许发周报"}]"#).is_err(),
            "bare array is not the contract wrapper"
        );
    }

    #[test]
    fn fake_review_rules_confirm_then_apply() {
        let api = FakeApi::new(Vec::new()).with_review_rules(vec![review_rules::Rule {
            id: "r1".into(),
            text: "允许发周报".into(),
            created_at: "2026-11-20T08:00:00Z".into(),
        }]);
        assert_eq!(api.review_rules().unwrap().len(), 1);

        let add_id = api.propose_review_rule("允许合并 CI 全绿的 PR").unwrap();
        assert!(add_id.starts_with("apr-add-"));
        assert_eq!(
            api.review_rules().unwrap().len(),
            1,
            "unconfirmed add stays off the list"
        );

        let event = api.approval(&add_id).unwrap();
        assert!(event_decode::is_settings_approval(&event));
        assert!(matches!(
            event,
            Event::Approval {
                action: ApprovalAction::RuleAdd,
                bot_id,
                task_id: None,
                ..
            } if bot_id == SETTINGS_APPROVAL_BOT_ID
        ));

        let (event, extras) = api.decide(&add_id, true).unwrap().unwrap();
        assert!(extras.is_none());
        assert!(matches!(
            event,
            Event::Approval {
                decision: Some(ApprovalDecision::Approved),
                action: ApprovalAction::RuleAdd,
                ..
            }
        ));
        let rules = api.review_rules().unwrap();
        assert_eq!(rules.len(), 2);
        assert!(rules.iter().any(|r| r.text == "允许合并 CI 全绿的 PR"));
        assert!(matches!(
            api.decide(&add_id, true),
            Err(ApiError::Conflict(_))
        ));

        let del_id = api.propose_delete_review_rule("r1").unwrap();
        assert_eq!(api.review_rules().unwrap().len(), 2);
        api.decide(&del_id, false).unwrap();
        assert_eq!(
            api.review_rules().unwrap().len(),
            2,
            "denied delete leaves the rule"
        );
        let del_id = api.propose_delete_review_rule("r1").unwrap();
        api.decide(&del_id, true).unwrap();
        let rules = api.review_rules().unwrap();
        assert_eq!(rules.len(), 1);
        assert!(!rules.iter().any(|r| r.id == "r1"));

        assert!(matches!(
            api.propose_review_rule(&"字".repeat(review_rules::REVIEW_RULE_MAX + 1)),
            Err(ApiError::BadRequest(msg)) if msg == review_rules::too_long_note(1)
        ));
    }

    #[test]
    fn search_source_env_settings_or_absent() {
        let env = decode_search(
            r#"{"provider":"brave","url":null,"hasKey":true,"keyLast4":"XXXX","source":"env"}"#,
        )
        .unwrap();
        assert_eq!(env.source, Some(search::Source::Env));
        assert_eq!(env.provider, Some(search::Provider::Brave));
        assert_eq!(env.key_last4.as_deref(), Some("XXXX"));
        let saved = decode_search(
            r#"{"provider":"searxng","url":"http://s","hasKey":false,"keyLast4":null,"source":"settings"}"#,
        )
        .unwrap();
        assert_eq!(saved.source, Some(search::Source::Settings));
        // Today's host (#76): no `source` at all, so no label.
        let old =
            decode_search(r#"{"provider":"brave","url":null,"hasKey":true,"keyLast4":"XXXX"}"#)
                .unwrap();
        assert_eq!(old.source, None);
        let none = decode_search(r#"{"provider":null,"url":null,"hasKey":false,"keyLast4":null}"#)
            .unwrap();
        assert_eq!(none, search::Current::default());
    }

    #[test]
    fn search_put_body_key_rules() {
        use search::{Provider, Put};
        let p = |key| {
            search_body::put(&Put {
                provider: Provider::Brave,
                url: None,
                key,
                clear: false,
            })
        };
        assert_eq!(p(None), r#"{"provider":"brave"}"#);
        assert_eq!(p(Some(None)), r#"{"key":null,"provider":"brave"}"#);
        assert_eq!(
            p(Some(Some("k1".into()))),
            r#"{"key":"k1","provider":"brave"}"#
        );
        let sx = search_body::put(&Put {
            provider: Provider::Searxng,
            url: Some("http://s".into()),
            key: None,
            clear: false,
        });
        assert_eq!(sx, r#"{"provider":"searxng","url":"http://s"}"#);
        assert_eq!(
            search_body::put(&Put::clear()),
            r#"{"key":null,"provider":null,"url":null}"#
        );
    }

    #[test]
    fn model_source_env_settings_or_absent() {
        let env = decode_model(
            r#"{"baseUrl":"https://generativelanguage.googleapis.com/v1beta/openai","model":"gemini-3.8-flash","hasKey":true,"keyLast4":"ABCD","source":"env"}"#,
        )
        .unwrap();
        assert_eq!(env.source, Some(model_page::Source::Env));
        assert_eq!(
            env.base_url.as_deref(),
            Some("https://generativelanguage.googleapis.com/v1beta/openai")
        );
        assert_eq!(env.model.as_deref(), Some("gemini-3.8-flash"));
        assert_eq!(env.key_last4.as_deref(), Some("ABCD"));
        let saved = decode_model(
            r#"{"baseUrl":"http://localhost:11434/v1","model":"llama3.2","hasKey":true,"keyLast4":"lama","source":"settings"}"#,
        )
        .unwrap();
        assert_eq!(saved.source, Some(model_page::Source::Settings));
        let old = decode_model(
            r#"{"baseUrl":"http://localhost:11434/v1","model":"llama3.2","hasKey":false,"keyLast4":null}"#,
        )
        .unwrap();
        assert_eq!(old.source, None);
        let none = decode_model(r#"{"baseUrl":null,"model":null,"hasKey":false,"keyLast4":null}"#)
            .unwrap();
        assert_eq!(none, model_page::Current::default());
    }

    #[test]
    fn model_put_body_key_rules() {
        use model_page::Put;
        let p = |key| {
            model_body::put(&Put {
                base_url: Some(Some("https://api.groq.com/openai/v1".into())),
                model: Some(Some("llama".into())),
                key,
            })
        };
        assert_eq!(
            p(None),
            r#"{"baseUrl":"https://api.groq.com/openai/v1","model":"llama"}"#
        );
        assert_eq!(
            p(Some(None)),
            r#"{"baseUrl":"https://api.groq.com/openai/v1","key":null,"model":"llama"}"#
        );
        assert_eq!(
            p(Some(Some("k1".into()))),
            r#"{"baseUrl":"https://api.groq.com/openai/v1","key":"k1","model":"llama"}"#
        );
        assert_eq!(
            model_body::put(&Put {
                base_url: None,
                model: None,
                key: Some(None)
            }),
            r#"{"key":null}"#
        );
    }

    #[test]
    fn fake_model_put_and_test() {
        let api = FakeApi::new(Vec::new());
        assert_eq!(
            api.model_settings().unwrap(),
            model_page::Current::default()
        );
        assert_eq!(
            api.test_model().unwrap(),
            (false, Some(model_page::NOT_SET.into()))
        );
        let saved = api
            .put_model(&model_page::Put {
                base_url: Some(Some("https://api.groq.com/openai/v1/".into())),
                model: Some(Some("llama-3.1-8b-instant".into())),
                key: Some(Some("gsk_xxxx".into())),
            })
            .unwrap();
        assert_eq!(
            saved.base_url.as_deref(),
            Some("https://api.groq.com/openai/v1")
        );
        assert_eq!(saved.key_last4.as_deref(), Some("xxxx"));
        assert_eq!(saved.source, Some(model_page::Source::Settings));
        assert_eq!(api.test_model().unwrap(), (true, None));
        let cleared_key = api
            .put_model(&model_page::Put {
                base_url: None,
                model: None,
                key: Some(None),
            })
            .unwrap();
        assert!(!cleared_key.has_key);
        assert_eq!(
            api.test_model().unwrap(),
            (true, None),
            "url + model is enough; key is optional"
        );
        let without_key = api
            .put_model(&model_page::Put {
                base_url: Some(Some("http://localhost:11434/v1".into())),
                model: Some(Some("llama3.2".into())),
                key: None,
            })
            .unwrap();
        assert!(!without_key.has_key);
        assert_eq!(api.test_model().unwrap(), (true, None));
        assert!(matches!(
            api.put_model(&model_page::Put {
                base_url: Some(Some("not-a-url".into())),
                model: None,
                key: None,
            }),
            Err(ApiError::BadRequest(m)) if m == model_page::BAD_URL
        ));
        let seeded = FakeApi::new(Vec::new()).with_model(model_page::Current {
            base_url: Some("https://api.groq.com/openai/v1".into()),
            model: Some("llama".into()),
            has_key: true,
            key_last4: Some("SEED".into()),
            source: Some(model_page::Source::Env),
        });
        let cur = seeded.model_settings().unwrap();
        assert_eq!(cur.key_last4.as_deref(), Some("SEED"));
        assert_eq!(cur.source, Some(model_page::Source::Env));
    }

    #[test]
    fn bodies() {
        assert_eq!(
            bodies::answer(&["cargo".into()], None),
            r#"{"choice":["cargo"]}"#
        );
        assert_eq!(
            bodies::answer(&["cargo".into()], Some("Mac")),
            r#"{"choice":["cargo"],"answeredBy":"Mac"}"#
        );
        assert_eq!(bodies::decision(None), None);
        assert_eq!(
            bodies::decision(Some("Mac")).as_deref(),
            Some(r#"{"answeredBy":"Mac"}"#)
        );
        assert_eq!(
            bodies::answer(&["a".into(), "b".into()], None),
            r#"{"choice":["a","b"]}"#
        );
        assert_eq!(bodies::paused(false), r#"{"paused":false}"#);
    }

    #[test]
    fn fake_say_routes_like_the_host() {
        let bro = |id: &str, name: &str| Bro {
            id: id.into(),
            name: name.into(),
            dm_group_id: Some(format!("dm-{id}")),
            owner_user_id: None,
        };
        let api = FakeApi::new(Vec::new())
            .with_chats(Vec::new(), vec![bro("b1", "bro"), bro("b2", "bro 2")]);
        let said = api.say("dm-b1", " 整理周报 ").unwrap();
        assert_eq!(said.message.body, "整理周报");
        let task = said.task.unwrap();
        assert_eq!(
            (task.assignee_bot_id.as_str(), task.prompt.as_str()),
            ("b1", "整理周报")
        );
        let task = api.say("g1", "@bro 2 写 README").unwrap().task.unwrap();
        assert_eq!(
            (task.assignee_bot_id.as_str(), task.prompt.as_str()),
            ("b2", "写 README")
        );
        assert_eq!(api.say("g1", "大家早").unwrap().task, None);
        assert!(matches!(api.say("g1", "  "), Err(ApiError::BadRequest(_))));
    }

    #[test]
    fn errors_map_to_notes() {
        assert_eq!(ApiError::Conflict("x".into()).note(), Note::AlreadyAnswered);
        assert_eq!(
            ApiError::BadRequest("没有这个选项".into()).note(),
            Note::Rejected("没有这个选项".into())
        );
        assert_eq!(ApiError::Transport("down".into()).note(), Note::Retry);
    }

    #[test]
    fn as_conn_error_maps_401_and_transport() {
        use crate::conn::ConnError;
        assert_eq!(
            ApiError::Status(401, "缺少或错误的令牌".into()).as_conn_error(),
            ConnError::Unauthorized
        );
        let err =
            ApiError::Transport("io: Connection refused (os error 111)".into()).as_conn_error();
        assert_eq!(
            err,
            ConnError::Unreachable(Text::ClientConnectionRefused.to_string())
        );
    }

    #[test]
    fn plain_reasons() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        assert_eq!(ApiError::BadRequest("名字太长".into()).reason(), "名字太长");
        assert_eq!(ApiError::Transport("refused".into()).reason(), "连不上主机");
    }

    #[test]
    fn host_down_is_transport_error() {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let api = HttpApi::new(Conn::new(format!("http://127.0.0.1:{port}")));
        assert!(matches!(
            api.answer_choice("c", &["a".into()]),
            Err(ApiError::Transport(_))
        ));
    }

    /// The real host router in-process (robroz#15 endpoints).
    mod against_host {
        use super::*;
        use robroz_contract::{
            AddMember, ChoiceOption, CreateChoice, CreateGroup, CreateSchedule, Group, Schedule,
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

        fn group_with_bot(base: &str) -> Group {
            let group: Group = post(
                base,
                "/api/groups",
                &CreateGroup {
                    name: "官网".into(),
                },
            );
            let _: serde_json::Value = post(
                base,
                &format!("/api/groups/{}/members", group.id),
                &AddMember {
                    bot_id: "bot-doc".into(),
                    name: "文档哥".into(),
                },
            );
            group
        }

        #[test]
        fn memories_list_write_edit_forget() {
            let base = spawn_host();
            let _ = group_with_bot(&base);
            let api = HttpApi::new(Conn::new(&base));
            assert!(api.memories().unwrap().is_empty());

            let mine = api
                .create_memory("user", MemoryKind::Profile, "我在伦敦，用英国时间")
                .unwrap();
            assert_eq!(mine.status, MemoryStatus::Kept);
            let doc = api
                .create_memory("bot:bot-doc", MemoryKind::Habit, "周报写短一点")
                .unwrap();
            assert!(matches!(
                api.create_memory("bot:nobody", MemoryKind::Habit, "x"),
                Err(ApiError::BadRequest(_))
            ));
            assert!(matches!(
                api.create_memory("user", MemoryKind::Habit, &"字".repeat(2001)),
                Err(ApiError::BadRequest(_))
            ));
            assert_eq!(api.memories().unwrap().len(), 2);

            let edited = api.update_memory(&mine.id, "我在伦敦").unwrap();
            assert_eq!(edited.text, "我在伦敦");
            assert_eq!(edited.scope, "user");

            api.forget_memory(&doc.id).unwrap();
            api.forget_memory(&doc.id).unwrap();
            assert!(matches!(
                api.forget_memory("nope"),
                Err(ApiError::NotFound(_))
            ));
            let left = api.memories().unwrap();
            assert_eq!(left.len(), 1);
            assert_eq!(left[0].id, mine.id);
        }

        #[test]
        fn skills_list_write_edit_forget_toggle_and_409() {
            let base = spawn_host();
            let group = group_with_bot(&base);
            let api = HttpApi::new(Conn::new(&base));
            assert!(api.skills().unwrap().is_empty());

            let weekly = api
                .create_skill("user", "weekly-report", "写周报", "先列三点")
                .unwrap();
            assert_eq!(weekly.status, MemoryStatus::Kept);
            let ada = api
                .create_skill("bot:bot-doc", "ada-notes", "Ada 的笔记", "写短一点")
                .unwrap();
            assert!(matches!(
                api.create_skill("bot:nobody", "x", "x", "x"),
                Err(ApiError::BadRequest(_))
            ));
            assert!(matches!(
                api.create_skill("user", "Weekly", "x", "x"),
                Err(ApiError::BadRequest(_))
            ));
            assert!(matches!(
                api.create_skill("user", "weekly-report", "又一条", "说明"),
                Err(ApiError::Conflict(_))
            ));
            assert_eq!(api.skills().unwrap().len(), 2);

            let edited = api
                .update_skill(&weekly.slug, "周报", "用中文，先结论")
                .unwrap();
            assert_eq!(edited.title, "周报");
            assert_eq!(edited.instruction, "用中文，先结论");
            assert_eq!(edited.scope, "user");

            assert!(api.group_skills(&group.id).unwrap().is_empty());
            api.set_skill_enabled(&group.id, &weekly.slug, true)
                .unwrap();
            api.set_skill_enabled(&group.id, &weekly.slug, true)
                .unwrap();
            let on = api.group_skills(&group.id).unwrap();
            assert_eq!(on.len(), 1);
            assert_eq!(on[0].slug, weekly.slug);
            api.set_skill_enabled(&group.id, &weekly.slug, false)
                .unwrap();
            assert!(api.group_skills(&group.id).unwrap().is_empty());
            assert!(matches!(
                api.set_skill_enabled(&group.id, "nope", true),
                Err(ApiError::NotFound(_))
            ));
            assert!(matches!(
                api.group_skills("missing"),
                Err(ApiError::NotFound(_))
            ));

            api.forget_skill(&ada.slug).unwrap();
            api.forget_skill(&ada.slug).unwrap();
            assert!(matches!(
                api.forget_skill("nope"),
                Err(ApiError::NotFound(_))
            ));
            let left = api.skills().unwrap();
            assert_eq!(left.len(), 1);
            assert_eq!(left[0].slug, weekly.slug);
        }

        #[test]
        fn mcp_crud_secrets_group_enable_and_test() {
            use robroz_contract::McpSecretIn;
            use robroz_ui::cards::mcp::{Save as McpSave, SecretPut, Transport};

            let base = spawn_host();
            let group = group_with_bot(&base);
            let api = HttpApi::new(Conn::new(&base));
            assert!(api.mcp_servers().unwrap().is_empty());

            let key = "secret-key-WXYZ";
            let save = McpSave {
                id: None,
                name: "文档".into(),
                transport: Transport::Stdio,
                command: Some("/bin/true".into()),
                args: Some(vec!["--ok".into()]),
                url: None,
                secrets: vec![SecretPut {
                    name: "TOKEN".into(),
                    placement: McpSecretIn::Env,
                    key: Some(Some(key.into())),
                }],
            };
            let created = api.create_mcp_server(&save).unwrap();
            assert_eq!(created.name, "文档");
            assert!(created.enabled);
            assert_eq!(created.command_approved, Some(false));
            assert!(created.secrets[0].has_key);
            assert_eq!(created.secrets[0].key_last4.as_deref(), Some("WXYZ"));
            let raw = serde_json::to_string(&created).unwrap();
            assert!(!raw.contains(key), "{raw}");

            let id = created.id.clone();
            let mut patch = save.clone();
            patch.id = Some(id.clone());
            patch.name = "文档工具".into();
            patch.secrets[0].key = None;
            let edited = api.update_mcp_server(&patch).unwrap();
            assert_eq!(edited.name, "文档工具");
            assert_eq!(edited.secrets[0].key_last4.as_deref(), Some("WXYZ"));
            let raw = serde_json::to_string(&edited).unwrap();
            assert!(!raw.contains(key), "{raw}");

            assert!(api.group_mcp(&group.id).unwrap().is_empty());
            api.set_group_mcp(&group.id, &id, true).unwrap();
            assert_eq!(api.group_mcp(&group.id).unwrap()[0].id, id);
            api.set_group_mcp(&group.id, &id, false).unwrap();
            assert!(api.group_mcp(&group.id).unwrap().is_empty());
            assert!(matches!(
                api.group_mcp("missing"),
                Err(ApiError::NotFound(_))
            ));

            match api.test_mcp_server(&id) {
                Err(ApiError::CommandNotApproved(denied)) => {
                    assert_eq!(denied.command, "/bin/true");
                    assert_eq!(denied.args, vec!["--ok"]);
                }
                other => panic!("expected command_not_approved, got {other:?}"),
            }
            let approved = api.approve_mcp_command(&id).unwrap();
            assert_eq!(approved.command_approved, Some(true));
            assert!(approved.command_approved_at.is_some());

            let test = api.test_mcp_server(&id).unwrap();
            assert!(!test.ok, " /bin/true is not an MCP server");
            assert!(test.error.as_ref().is_some());

            let mut changed = patch.clone();
            changed.args = Some(vec!["--ok".into(), "--again".into()]);
            let after = api.update_mcp_server(&changed).unwrap();
            assert_eq!(after.command_approved, Some(false));
            assert!(matches!(
                api.test_mcp_server(&id),
                Err(ApiError::CommandNotApproved(_))
            ));
            api.approve_mcp_command(&id).unwrap();

            assert!(!api.set_mcp_enabled(&id, false).unwrap().enabled);
            api.delete_mcp_server(&id).unwrap();
            assert!(api.mcp_servers().unwrap().is_empty());
            let a = api.create_mcp_server(&save).unwrap();
            assert!(matches!(
                api.create_mcp_server(&save),
                Err(ApiError::Conflict(_))
            ));
            api.delete_mcp_server(&a.id).unwrap();
        }

        #[test]
        fn answer_choice_then_409_and_400() {
            let base = spawn_host();
            let group = group_with_bot(&base);
            let option = |id: &str| ChoiceOption {
                id: id.into(),
                label: id.into(),
                primary: false,
            };
            let created: Event = post(
                &base,
                &format!("/api/groups/{}/choices", group.id),
                &CreateChoice {
                    bot_id: "bot-doc".into(),
                    prompt: "先写哪一种？".into(),
                    options: vec![option("docker"), option("cargo")],
                    multi: false,
                },
            );
            let Event::Choice { id, .. } = created else {
                panic!("expected choice event");
            };
            let api = HttpApi::new(Conn::new(&base));
            assert!(matches!(
                api.answer_choice(&id, &["nope".into()]),
                Err(ApiError::BadRequest(_))
            ));
            let answered = api.answer_choice(&id, &["cargo".into()]).unwrap();
            match answered {
                Some(Event::Choice { choice, .. }) => {
                    assert_eq!(choice, Some(vec!["cargo".to_string()]))
                }
                other => panic!("expected answered choice, got {other:?}"),
            }
            assert!(matches!(
                api.answer_choice(&id, &["docker".into()]),
                Err(ApiError::Conflict(_))
            ));
        }

        #[test]
        fn answers_carry_answered_by() {
            let base = spawn_host();
            let group = group_with_bot(&base);
            let approval = |summary: &str| -> String {
                let created: Event = post(
                    &base,
                    &format!("/api/groups/{}/approvals", group.id),
                    &serde_json::json!({
                        "botId": "bot-doc", "action": "send", "summary": summary
                    }),
                );
                let Event::Approval { id, .. } = created else {
                    panic!("expected approval event");
                };
                id
            };
            let named = HttpApi::new(Conn::new(&base)).answering_as("Panda 的 Mac");
            let a1 = approval("一");
            match named.decide(&a1, true).unwrap() {
                Some((Event::Approval { answered_by, .. }, _)) => {
                    assert_eq!(answered_by.as_deref(), Some("Panda 的 Mac"))
                }
                other => panic!("expected answered approval, got {other:?}"),
            }
            // No name: the old empty request, `answeredBy` stays null.
            let a2 = approval("二");
            match HttpApi::new(Conn::new(&base)).decide(&a2, false).unwrap() {
                Some((Event::Approval { answered_by, .. }, _)) => assert_eq!(answered_by, None),
                other => panic!("expected answered approval, got {other:?}"),
            }
            let created: Event = post(
                &base,
                &format!("/api/groups/{}/choices", group.id),
                &CreateChoice {
                    bot_id: "bot-doc".into(),
                    prompt: "先写哪一种？".into(),
                    options: ["docker", "cargo"]
                        .map(|o| ChoiceOption {
                            id: o.into(),
                            label: o.into(),
                            primary: false,
                        })
                        .to_vec(),
                    multi: false,
                },
            );
            let Event::Choice { id, .. } = created else {
                panic!("expected choice event");
            };
            match named.answer_choice(&id, &["cargo".into()]).unwrap() {
                Some(Event::Choice { answered_by, .. }) => {
                    assert_eq!(answered_by.as_deref(), Some("Panda 的 Mac"))
                }
                other => panic!("expected answered choice, got {other:?}"),
            }
        }

        #[test]
        fn list_and_pause_schedules() {
            let base = spawn_host();
            let group = group_with_bot(&base);
            let created: Schedule = post(
                &base,
                "/api/schedules",
                &CreateSchedule {
                    group_id: group.id.clone(),
                    assignee_bot_id: "bot-doc".into(),
                    cron: "0 8 * * *".into(),
                    prompt: "每日简报".into(),
                },
            );
            let api = HttpApi::new(Conn::new(&base));
            let list = api.schedules(&group.id).unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(list[0].schedule.id, created.id);
            assert!(!list[0].paused);
            let paused = api.set_paused(&created.id, true).unwrap();
            assert!(paused.paused);
            assert!(api.schedules(&group.id).unwrap()[0].paused);
            assert!(matches!(
                api.set_paused("missing", true),
                Err(ApiError::NotFound(_))
            ));
        }

        #[test]
        fn unread_list_and_mark_read() {
            let base = spawn_host();
            let quiet = group_with_bot(&base);
            std::thread::sleep(std::time::Duration::from_millis(5));
            let busy = group_with_bot(&base);
            // A choice card from a bro counts as unread.
            let _: Event = post(
                &base,
                &format!("/api/groups/{}/choices", busy.id),
                &CreateChoice {
                    bot_id: "bot-doc".into(),
                    prompt: "装法？".into(),
                    options: vec![
                        ChoiceOption {
                            id: "a".into(),
                            label: "A".into(),
                            primary: false,
                        },
                        ChoiceOption {
                            id: "b".into(),
                            label: "B".into(),
                            primary: false,
                        },
                    ],
                    multi: false,
                },
            );
            let api = HttpApi::new(Conn::new(&base));
            let list = api.unread().unwrap();
            assert_eq!(list.len(), 2);
            assert_eq!(list[0].group_id, busy.id, "most recently active first");
            assert_eq!(list[0].unread, 1);
            assert_eq!(list[1].group_id, quiet.id);
            assert_eq!(list[1].unread, 0);

            let read = api.mark_read(&busy.id).unwrap();
            assert_eq!(read.unread, 0);
            assert!(read.read_at.is_some());
            assert_eq!(api.unread().unwrap()[0].unread, 0);
            assert!(matches!(
                api.mark_read("missing"),
                Err(ApiError::NotFound(_))
            ));
        }

        #[test]
        fn new_bro_gets_default_name_and_a_dm_in_the_list() {
            let base = spawn_host();
            let api = HttpApi::new(Conn::new(&base));
            let first = api.create_bro().unwrap();
            let second = api.create_bro().unwrap();
            assert_eq!(first.name, "bro");
            assert_eq!(second.name, "bro 2");
            let dm = first.dm_group_id.clone().expect("DM created");
            // The DM is an ordinary group: listed, in unread, readable.
            let mut source = crate::source::HttpSource::new(Conn::new(&base), Vec::new());
            use crate::source::EventSource;
            let groups = source.groups().unwrap();
            assert!(groups.iter().any(|g| g.id == dm && g.name == "bro"));
            assert!(api.unread().unwrap().iter().any(|u| u.group_id == dm));
            assert_eq!(api.mark_read(&dm).unwrap().unread, 0);
            let bros = api.bros().unwrap();
            assert_eq!(bros.len(), 2);
            assert_eq!(bros[0].dm_group_id.as_deref(), Some(dm.as_str()));
        }

        /// Events of a group as raw JSON (`type` + fields).
        fn events(base: &str, group: &str) -> Vec<serde_json::Value> {
            let source = crate::source::HttpSource::new(Conn::new(base), Vec::new());
            source
                .events(group, None)
                .unwrap()
                .events
                .into_iter()
                .map(|e| serde_json::to_value(e).unwrap())
                .collect()
        }

        #[test]
        fn composer_says_and_the_host_routes() {
            let base = spawn_host();
            let group = group_with_bot(&base);
            let api = HttpApi::new(Conn::new(&base));
            // Group, no @: stored, nobody asked.
            let said = api.say(&group.id, "大家早").unwrap();
            assert_eq!(said.message.body, "大家早");
            assert_eq!(said.task, None);
            // Group, @member: stored as typed, the rest queued for that bro.
            let said = api.say(&group.id, "  @文档哥 写 README ").unwrap();
            assert_eq!(said.message.body, "@文档哥 写 README");
            let task = said.task.expect("task for the mentioned bro");
            assert_eq!(task.assignee_bot_id, "bot-doc");
            assert_eq!(task.prompt, "写 README");
            assert_eq!(task.state, TaskState::Queued);
            // @ someone not in the group: just a message.
            assert_eq!(api.say(&group.id, "@路人 帮忙").unwrap().task, None);
            let err = api.say(&group.id, "  ").unwrap_err();
            assert!(matches!(err, ApiError::BadRequest(_)));
            // The host's own constant, shown in the active (default) language.
            assert_eq!(err.reason(), "Enter some text");
            assert!(matches!(&err, ApiError::BadRequest(r) if r == Text::HostNeedText.as_str()));
            assert!(matches!(
                api.say("no-such-group", "hi"),
                Err(ApiError::NotFound(_))
            ));
            let seen = events(&base, &group.id);
            assert!(seen
                .iter()
                .any(|e| e["type"] == "message" && e["body"] == "@文档哥 写 README"));
            assert!(seen
                .iter()
                .any(|e| e["type"] == "status" && e["botId"] == "bot-doc"));

            // DM: the bro behind it gets the task.
            let bro = api.create_bro().unwrap();
            let dm = bro.dm_group_id.clone().unwrap();
            let said = api.say(&dm, "整理一下周报").unwrap();
            let task = said.task.expect("a DM always asks its bro");
            assert_eq!(task.assignee_bot_id, bro.id);
            assert_eq!(task.prompt, "整理一下周报");
            let seen = events(&base, &dm);
            assert!(seen
                .iter()
                .any(|e| e["type"] == "message" && e["body"] == "整理一下周报"));
            assert!(seen
                .iter()
                .any(|e| e["type"] == "status" && e["botId"] == bro.id.as_str()));
        }

        #[test]
        fn rename_bro_renames_its_dm() {
            let base = spawn_host();
            let api = HttpApi::new(Conn::new(&base));
            let bro = api.create_bro().unwrap();
            let dm = bro.dm_group_id.clone().unwrap();
            let renamed = api.rename_bro(&bro.id, "  文档哥 ").unwrap();
            assert_eq!(renamed.name, "文档哥");
            assert_eq!(renamed.dm_group_id.as_deref(), Some(dm.as_str()));
            // The list and the header read the DM group's name.
            let mut source = crate::source::HttpSource::new(Conn::new(&base), Vec::new());
            use crate::source::EventSource;
            let groups = source.groups().unwrap();
            assert!(groups.iter().any(|g| g.id == dm && g.name == "文档哥"));
            assert_eq!(api.bros().unwrap()[0].name, "文档哥");
            // The next default name is free again.
            assert_eq!(api.create_bro().unwrap().name, "bro");
            assert!(matches!(
                api.rename_bro(&bro.id, "   "),
                Err(ApiError::BadRequest(_))
            ));
            assert!(matches!(
                api.rename_bro("nope", "x"),
                Err(ApiError::NotFound(_))
            ));
        }

        #[test]
        fn approve_once_then_409_deny_and_404() {
            let base = spawn_host();
            let group = group_with_bot(&base);
            let created: Event = post(
                &base,
                &format!("/api/groups/{}/approvals", group.id),
                &serde_json::json!({
                    "botId": "bot-doc", "action": "send", "summary": "把周报发给团队"
                }),
            );
            let Event::Approval { id, decision, .. } = created else {
                panic!("expected approval event");
            };
            assert_eq!(decision, None);
            let api = HttpApi::new(Conn::new(&base));
            match api.decide(&id, false).unwrap() {
                Some((
                    Event::Approval {
                        decision,
                        answered_at,
                        ..
                    },
                    _,
                )) => {
                    assert_eq!(decision, Some(robroz_contract::ApprovalDecision::Denied));
                    assert!(answered_at.is_some());
                }
                other => panic!("expected answered approval, got {other:?}"),
            }
            assert!(matches!(
                api.decide(&id, true),
                Err(ApiError::ApprovalSettled)
            ));
            assert!(matches!(
                api.decide("missing", true),
                Err(ApiError::NotFound(_))
            ));
            // Polling (`?since=` path) sees the answered card too.
            let seen = events(&base, &group.id);
            assert!(seen
                .iter()
                .any(|e| e["type"] == "approval" && e["decision"] == "denied"));
        }

        #[test]
        fn token_is_sent_on_every_call() {
            let token = "client-test-token-0123456789";
            let base = crate::source::http_test_host_with_token(token);
            let without = HttpApi::new(Conn::new(&base));
            match without.unread() {
                Err(ApiError::TokenRejected) => {
                    assert_eq!(
                        without.unread().unwrap_err().reason(),
                        Text::ClientTokenRejected.as_str()
                    );
                }
                other => panic!("expected token_rejected, got {other:?}"),
            }
            let api = HttpApi::new(Conn::new(&base).with_token(Some(token.into())));
            let bro = api.create_bro().unwrap();
            let dm = bro.dm_group_id.unwrap();
            assert_eq!(api.mark_read(&dm).unwrap().unread, 0);
            let mut source = crate::source::HttpSource::new(
                Conn::new(&base).with_token(Some(token.into())),
                Vec::new(),
            );
            use crate::source::EventSource;
            assert!(source.groups().unwrap().iter().any(|g| g.id == dm));
            assert!(source.poll().is_ok());

            // The live SSE stream uses the same Conn auth. Stop as soon as
            // it is up, so we don't sit on an idle stream.
            let mut last_id = None;
            let stream = Conn::new(&base).with_token(Some(token.into())).streaming();
            let up = crate::source::stream::read_once(&stream, &dm, &mut last_id, &mut |item| {
                !matches!(item, crate::source::stream::StreamItem::Connected)
            });
            assert_eq!(up, Ok(true), "{up:?}");

            let no_token = Conn::new(&base).streaming();
            let down =
                crate::source::stream::read_once(&no_token, &dm, &mut last_id, &mut |_| true);
            assert_eq!(down, Err(crate::conn::unauthorized_message()));
        }
    }
}
