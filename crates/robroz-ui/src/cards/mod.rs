//! Client view models for the phase-1 cards (UI SPEC §4–§8).
//!
//! `choice` and the routines list are built from the contract types added in
//! robroz#15 (`Event::Choice`, `ScheduleEntry`). `approval` and `draft` have
//! no contract event yet, so their shapes follow SPEC §8 and stay client-side
//! until the backend adds them. None of this touches the frozen status card.

use crate::i18n::Key;
pub mod codes;
pub mod confirm;
pub mod group_approval;
pub mod interact;
pub mod mcp;
pub mod memories;

pub mod model;
pub mod review_rules;

pub mod reminder;
pub mod routines;
pub mod search;
pub mod secret;
pub mod skills;
pub mod status;

use robroz_contract::{ApprovalAction, ApprovalDecision, ChoiceOption, Event};

/// New card kinds, side by side with the frozen status and delivery cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Card {
    Choice(ChoiceCard),
    Approval(ApprovalCard),
    Draft(DraftCard),
    /// A bro's proposal to keep (robroz#66).
    Confirm(confirm::ConfirmCard),
}

impl Card {
    /// Key for in-place updates.
    pub fn id(&self) -> &str {
        match self {
            Card::Choice(c) => &c.id,
            Card::Approval(c) => &c.id,
            Card::Draft(c) => &c.id,
            Card::Confirm(c) => &c.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceCard {
    pub id: String,
    pub bot: String,
    pub prompt: String,
    /// 2..=4, in the order the bot gave them. `primary` is ignored on multi.
    pub options: Vec<ChoiceOption>,
    pub multi: bool,
    pub state: ChoiceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChoiceState {
    Open,
    Answered {
        selected: Vec<String>,
        /// ISO 8601 from the contract (`answeredAt`); shown as local HH:MM.
        at: Option<String>,
        /// Who answered, for the footnote: the host's `answeredBy`, with this
        /// client's own name shown as 「我」 (see [`ChoiceCard::seen_by`]).
        /// `None` when the host does not know.
        by: Option<String>,
    },
    /// No contract state for this yet; kept so the view is ready for it.
    Expired,
}

impl ChoiceCard {
    /// Build from the `choice` event. Returns `None` for any other event.
    pub fn from_event(event: &Event) -> Option<Self> {
        let Event::Choice {
            id,
            bot_name,
            prompt,
            options,
            multi,
            choice,
            answered_at,
            answered_by,
            ..
        } = event
        else {
            return None;
        };
        let state = match choice {
            Some(selected) => ChoiceState::Answered {
                selected: selected.clone(),
                at: answered_at.clone(),
                by: answered_by.clone(),
            },
            None => ChoiceState::Open,
        };
        Some(Self {
            id: id.clone(),
            bot: bot_name.clone(),
            prompt: prompt.clone(),
            options: options.clone(),
            multi: *multi,
            state,
        })
    }

    /// Show this client's own name (`me`) as 「我」.
    pub fn seen_by(mut self, me: &str) -> Self {
        if let ChoiceState::Answered { by, .. } = &mut self.state {
            *by = as_me(by.take(), me);
        }
        self
    }

    pub fn label_of(&self, option_id: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|o| o.id == option_id)
            .map(|o| o.label.as_str())
    }

    /// The option that may get the orange button: the first `primary`, and
    /// only on single choice. Guarantees at most one per card.
    pub fn primary_option(&self) -> Option<&str> {
        if self.multi {
            return None;
        }
        self.options
            .iter()
            .find(|o| o.primary)
            .map(|o| o.id.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    Delete,
    Send,
    Spend,
    Call,

    /// Propose adding an allow rule (`ApprovalAction::RuleAdd`).
    RuleAdd,
    /// Propose deleting an allow rule (`ApprovalAction::RuleDelete`).
    RuleDelete,
    /// An action value this client does not recognise. Shown with the
    /// generic verb 「操作」.
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalCard {
    pub id: String,
    pub bot: String,
    pub kind: ActionKind,
    pub summary: String,
    /// Relative path, display only (monospace, not a link).
    pub path: Option<String>,
    pub state: ApprovalState,
    /// MCP tool on a `call` card, used to set `{server}.{tool}` in the
    /// host summary in monospace. Absent on other actions.
    pub mcp_server: Option<String>,
    pub mcp_tool: Option<String>,
    /// Why a blocked action was stopped, or the expiry reason. Absent on
    /// older cards. Compared to [`review_rules::APPROVAL_EXPIRED_REASON`].
    pub reason: Option<String>,
    /// v0.2: the bro's owner, the only person who may decide this card
    /// (`ownerUserId`). `None` from an older host or on a settings card.
    pub owner_id: Option<String>,
    /// The owner's display name (`ownerName`), for "Waiting for {owner}".
    pub owner_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalState {
    Pending,
    /// `by`: who answered (the host's `answeredBy`, own name as 「我」),
    /// `None` when unknown. `at`: the host's `answeredAt` (unix seconds).
    Approved {
        by: Option<String>,
        at: i64,
    },
    Denied {
        by: Option<String>,
        at: i64,
    },
    NeedsOwner,
    /// `denied` + `reason == APPROVAL_EXPIRED_REASON`. The owner never
    /// clicked; do not present this as a rejection.
    Expired {
        at: i64,
    },
}

/// Unused leftover from the pre-contract `rule_*` stash. Callers still
/// thread this through event decode; it is always empty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApprovalExtras {}

impl ApprovalCard {
    /// Show this client's own name (`me`) as 「我」.
    pub fn seen_by(mut self, me: &str) -> Self {
        if let ApprovalState::Approved { by, .. } | ApprovalState::Denied { by, .. } =
            &mut self.state
        {
            *by = as_me(by.take(), me);
        }
        self
    }

    /// Build from the `approval` event. Returns `None` for any other event.
    /// `by` is the host's raw `answeredBy`; see [`ApprovalCard::seen_by`].
    pub fn from_event(event: &Event) -> Option<Self> {
        Self::from_event_with(event, None)
    }

    /// Like [`Self::from_event`]. `extras` is ignored; rule actions come
    /// from the contract enum.
    pub fn from_event_with(event: &Event, _extras: Option<&ApprovalExtras>) -> Option<Self> {
        let Event::Approval {
            id,
            bot_name,
            action,
            summary,
            path,
            decision,
            answered_at,
            answered_by,
            mcp_server,
            mcp_tool,
            reason,
            expires_at,
            owner_user_id,
            owner_name,
            ..
        } = event
        else {
            return None;
        };
        let reason = reason.clone();
        let expired = reason.as_deref() == Some(review_rules::APPROVAL_EXPIRED_REASON);
        let at = answered_at
            .as_deref()
            .or(expires_at.as_deref())
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map_or(0, |t| t.timestamp());
        let state = match decision {
            None => ApprovalState::Pending,
            Some(ApprovalDecision::Approved) => ApprovalState::Approved {
                by: answered_by.clone(),
                at,
            },
            Some(ApprovalDecision::Denied) if expired => ApprovalState::Expired { at },
            Some(ApprovalDecision::Denied) => ApprovalState::Denied {
                by: answered_by.clone(),
                at,
            },
        };
        let kind = match action {
            ApprovalAction::Delete => ActionKind::Delete,
            ApprovalAction::Send => ActionKind::Send,
            ApprovalAction::Spend => ActionKind::Spend,
            ApprovalAction::Call => ActionKind::Call,
            ApprovalAction::RuleAdd => ActionKind::RuleAdd,
            ApprovalAction::RuleDelete => ActionKind::RuleDelete,
            ApprovalAction::Unknown => ActionKind::Unknown,
        };
        Some(Self {
            id: id.clone(),
            bot: bot_name.clone(),
            kind,
            summary: summary.clone(),
            path: path.clone(),
            state,
            mcp_server: mcp_server.clone(),
            mcp_tool: mcp_tool.clone(),
            reason,
            owner_id: owner_user_id.clone(),
            owner_name: owner_name.clone(),
        })
    }

    pub fn is_rule(&self) -> bool {
        matches!(self.kind, ActionKind::RuleAdd | ActionKind::RuleDelete)
    }
}

/// Host `call` summary split so the view can set `{server}.{tool}` in
/// monospace and clip the argument tail to one line. The wording is the
/// host's; the client does not write a new sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSummaryParts<'a> {
    pub prefix: &'a str,
    pub tool: &'a str,
    /// Includes the leading full-width colon when the host added args.
    pub args: Option<&'a str>,
}

pub fn split_call_summary<'a>(
    summary: &'a str,
    server: Option<&str>,
    tool: Option<&str>,
) -> Option<CallSummaryParts<'a>> {
    let server = server.filter(|s| !s.is_empty())?;
    let tool = tool.filter(|t| !t.is_empty())?;
    let needle = format!("{server}.{tool}");
    let start = summary.find(&needle)?;
    let end = start + needle.len();
    let rest = &summary[end..];
    Some(CallSummaryParts {
        prefix: &summary[..start],
        tool: &summary[start..end],
        args: if rest.is_empty() { None } else { Some(rest) },
    })
}

/// The footnote name for `answeredBy`: [`ME_MARK`] when it is this client's
/// own name, else the name as sent; `None` stays `None` (no person shown).
/// The mark, not the word, is stored so a language switch re-renders it.
pub fn as_me(by: Option<String>, me: &str) -> Option<String> {
    by.map(|b| if b == me { ME_MARK.to_string() } else { b })
}

/// How a card names the person using this client.
pub const ME: Key = Key::CardsMe;

/// Stored in a card's `by` for "this client"; [`who`] turns it into [`ME`]
/// in the active language. A private-use character, so no real name can
/// collide with it.
pub const ME_MARK: &str = "\u{E000}me";

/// The name to show for a stored `by`.
pub fn who(by: &str) -> &str {
    if by == ME_MARK {
        ME.as_str()
    } else {
        by
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftChannel {
    Email,
    Message,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftCard {
    pub id: String,
    pub bot: String,
    pub channel: DraftChannel,
    /// Address, display only (monospace, not a link).
    pub to: String,
    pub subject: Option<String>,
    pub body: String,
    pub state: DraftState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftState {
    Pending,
    Sent { by: String, at: i64 },
    Discarded { by: String, at: i64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answered_by_shows_me_other_or_nobody() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let event = |decision, by: Option<&str>| Event::Approval {
            id: "a1".into(),
            bot_id: "bot-doc".into(),
            bot_name: "文档哥".into(),
            action: ApprovalAction::Send,
            summary: "发周报".into(),
            path: None,
            decision: Some(decision),
            created_at: "2026-11-20T08:40:00Z".into(),
            answered_at: Some("2026-11-20T08:48:00Z".into()),
            answered_by: by.map(Into::into),
            task_id: None,
            mcp_server: None,
            mcp_tool: None,
            reason: None,
            expires_at: None,
            owner_name: None,
            owner_user_id: None,
        };
        let utc = chrono::Utc;
        let me = "Panda 的 Mac";
        let foot = |text: &str, time: &str| status::ApprovalFootnote {
            text: text.into(),
            time: Some(time.into()),
        };
        for (d, by, want) in [
            (ApprovalDecision::Approved, None, foot("已批准", "08:48")),
            (ApprovalDecision::Denied, None, foot("已拒绝", "08:48")),
            (
                ApprovalDecision::Approved,
                Some(me),
                foot("我已批准", "08:48"),
            ),
            (ApprovalDecision::Denied, Some(me), foot("我 拒绝", "08:48")),
            (
                ApprovalDecision::Approved,
                Some("Panda 的手机"),
                foot("Panda 的手机已批准", "08:48"),
            ),
        ] {
            let card = ApprovalCard::from_event(&event(d, by)).unwrap().seen_by(me);
            assert_eq!(status::approval_footnote(&card.state, &utc).unwrap(), want);
        }
        assert_eq!(as_me(Some(me.into()), me).as_deref(), Some(ME_MARK));
        assert_eq!(who(ME_MARK), ME.as_str());
        assert_eq!(who("Panda 的手机"), "Panda 的手机");
        assert_eq!(as_me(None, me), None);
    }

    pub(crate) fn opt(id: &str, label: &str, primary: bool) -> ChoiceOption {
        ChoiceOption {
            id: id.into(),
            label: label.into(),
            primary,
        }
    }

    fn event(choice: Option<Vec<&str>>, multi: bool) -> Event {
        Event::Choice {
            id: "c1".into(),
            bot_id: "bot-doc".into(),
            bot_name: "文档哥".into(),
            prompt: "安装说明先写哪一种？".into(),
            options: vec![
                opt("docker", "Docker 一键", true),
                opt("cargo", "cargo install", false),
            ],
            multi,
            choice: choice.map(|v| v.into_iter().map(String::from).collect()),
            created_at: "2026-11-20T01:50:00.000Z".into(),
            answered_at: None,
            answered_by: None,
        }
    }

    #[test]
    fn open_and_answered_from_contract() {
        let open = ChoiceCard::from_event(&event(None, false)).unwrap();
        assert_eq!(open.state, ChoiceState::Open);
        assert_eq!(open.bot, "文档哥");
        let done = ChoiceCard::from_event(&event(Some(vec!["cargo"]), false)).unwrap();
        assert_eq!(done.clone().seen_by("me"), done, "null stays null");
        assert!(matches!(
            done.state,
            ChoiceState::Answered { ref selected, by: None, .. } if selected == &["cargo"]
        ));
        assert_eq!(done.label_of("cargo"), Some("cargo install"));
    }

    #[test]
    fn primary_only_on_single_and_at_most_one() {
        let single = ChoiceCard::from_event(&event(None, false)).unwrap();
        assert_eq!(single.primary_option(), Some("docker"));
        let multi = ChoiceCard::from_event(&event(None, true)).unwrap();
        assert_eq!(multi.primary_option(), None);
        let mut two = single.clone();
        two.options[1].primary = true;
        assert_eq!(two.primary_option(), Some("docker"), "first primary wins");
    }

    #[test]
    fn other_events_are_not_choices() {
        let msg = Event::Message {
            id: "m".into(),
            kind: robroz_contract::MessageKind::Text,
            body: "hi".into(),
            created_at: "2026-11-20T01:50:00.000Z".into(),
            author_name: None,
            author_user_id: None,
        };
        assert!(ChoiceCard::from_event(&msg).is_none());
    }

    fn call_event(summary: &str, server: Option<&str>, tool: Option<&str>) -> Event {
        Event::Approval {
            id: "a-call".into(),
            bot_id: "bot-doc".into(),
            bot_name: "文档哥".into(),
            action: ApprovalAction::Call,
            summary: summary.into(),
            path: None,
            decision: None,
            created_at: "2026-11-22T01:00:00Z".into(),
            answered_at: None,
            answered_by: None,
            task_id: Some("t1".into()),
            mcp_server: server.map(Into::into),
            mcp_tool: tool.map(Into::into),
            reason: None,
            expires_at: None,
            owner_name: None,
            owner_user_id: None,
        }
    }

    #[test]
    fn call_approval_keeps_the_host_summary_and_names_the_tool() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let summary = "调用 notes.write_note：{\"title\":\"hi\"}";
        let card =
            ApprovalCard::from_event(&call_event(summary, Some("notes"), Some("write_note")))
                .unwrap();
        assert_eq!(card.kind, ActionKind::Call);
        assert_eq!(card.summary, summary);
        assert_eq!(card.mcp_server.as_deref(), Some("notes"));
        assert_eq!(card.mcp_tool.as_deref(), Some("write_note"));
        assert_eq!(status::action_kind_label(card.kind), "调用");
        assert!(!card.summary.contains("要调用"));
        assert!(!card.summary.contains("要用"));
        let parts = split_call_summary(
            &card.summary,
            card.mcp_server.as_deref(),
            card.mcp_tool.as_deref(),
        )
        .unwrap();
        assert_eq!(parts.prefix, "调用 ");
        assert_eq!(parts.tool, "notes.write_note");
        assert_eq!(parts.args, Some("：{\"title\":\"hi\"}"));
    }

    #[test]
    fn call_summary_without_args_has_no_tail() {
        let parts = split_call_summary("调用 notes.ping", Some("notes"), Some("ping")).unwrap();
        assert_eq!(parts.prefix, "调用 ");
        assert_eq!(parts.tool, "notes.ping");
        assert_eq!(parts.args, None);
    }

    #[test]
    fn call_summary_is_left_alone_without_the_tool_fields() {
        assert!(split_call_summary("调用 notes.write_note：{}", None, None).is_none());
        let long = format!("：{}", "很长的参数".repeat(40));
        let cut = status::clamp_lines(&long, 20, 1);
        assert!(cut.ends_with('…'), "{cut}");
        assert!(!cut.contains('\n'), "{cut}");
        assert!(cut.chars().count() < long.chars().count());
    }

    fn approval_event() -> Event {
        Event::Approval {
            id: "a-exp".into(),
            bot_id: "bot-doc".into(),
            bot_name: "文档哥".into(),
            action: ApprovalAction::Call,
            summary: "允许合并已经过 CI 的 PR".into(),
            path: None,
            decision: Some(ApprovalDecision::Denied),
            created_at: "2026-11-20T08:00:00Z".into(),
            answered_at: Some("2026-11-21T08:00:00Z".into()),
            answered_by: None,
            task_id: None,
            mcp_server: None,
            mcp_tool: None,
            reason: None,
            expires_at: None,
            owner_name: None,
            owner_user_id: None,
        }
    }

    #[test]
    fn expired_reason_is_not_a_rejection() {
        let mut ev = approval_event();
        if let Event::Approval { reason, .. } = &mut ev {
            *reason = Some(review_rules::APPROVAL_EXPIRED_REASON.into());
        }
        let card = ApprovalCard::from_event(&ev).unwrap();
        assert!(matches!(card.state, ApprovalState::Expired { .. }));
        assert_eq!(
            status::approval_status(&card.state),
            (review_rules::EXPIRED.as_str(), status::Dot::Void)
        );
        let foot = status::approval_footnote(&card.state, &chrono::Utc).unwrap();
        assert_eq!(foot.text, review_rules::EXPIRED_FOOT);
        assert_ne!(foot.text.contains("拒绝"), true);
        assert_eq!(
            card.reason.as_deref(),
            Some(review_rules::APPROVAL_EXPIRED_REASON)
        );
    }

    #[test]
    fn denied_without_expired_reason_stays_denied() {
        let mut ev = approval_event();
        if let Event::Approval { reason, .. } = &mut ev {
            *reason = Some("这条会改生产数据".into());
        }
        let card = ApprovalCard::from_event(&ev).unwrap();
        assert!(matches!(card.state, ApprovalState::Denied { .. }));
        assert_eq!(card.reason.as_deref(), Some("这条会改生产数据"));
    }

    #[test]
    fn rule_add_from_contract_action() {
        let mut ev = approval_event();
        if let Event::Approval {
            action,
            decision,
            answered_at,
            bot_id,
            bot_name,
            task_id,
            ..
        } = &mut ev
        {
            *action = ApprovalAction::RuleAdd;
            *decision = None;
            *answered_at = None;
            *bot_id = robroz_contract::SETTINGS_APPROVAL_BOT_ID.into();
            *bot_name = robroz_contract::SETTINGS_APPROVAL_BOT_NAME.into();
            *task_id = None;
        }
        let card = ApprovalCard::from_event(&ev).unwrap();
        assert_eq!(card.kind, ActionKind::RuleAdd);
        assert!(card.is_rule());
        assert_eq!(card.bot, robroz_contract::SETTINGS_APPROVAL_BOT_NAME);
    }

    #[test]
    fn unknown_action_is_a_generic_card() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let event = Event::Approval {
            id: "a-future".into(),
            bot_id: "bot-doc".into(),
            bot_name: "文档哥".into(),
            action: ApprovalAction::Unknown,
            summary: "新增一条规则".into(),
            path: None,
            decision: None,
            created_at: "2026-11-24T12:00:00Z".into(),
            answered_at: None,
            answered_by: None,
            task_id: None,
            mcp_server: None,
            mcp_tool: None,
            reason: None,
            expires_at: None,
            owner_name: None,
            owner_user_id: None,
        };
        let card = ApprovalCard::from_event(&event).unwrap();
        assert_eq!(card.kind, ActionKind::Unknown);
        assert_eq!(status::action_kind_label(card.kind), "操作");
        assert_eq!(card.summary, "新增一条规则");
    }
}
