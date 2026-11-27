//! The approval card in a group (v0.2): only the bro's owner decides.
//!
//! Pure rules for what the card shows, so the view only draws. See the
//! group approval spec: the owner sees Approve and Deny; everyone else sees
//! a read-only status and no buttons at all (not greyed out either).

use chrono::TimeZone;
use robroz_contract::ReminderState;

use super::interact::{ApprovalActions, CardUi, Note};
use super::review_rules;
use super::status::{self, clock_in, Dot};
use super::{ApprovalCard, ApprovalState};
use crate::i18n::{host_text, Key};
use robroz_contract::{UserRole, BOOTSTRAP_ADMIN_ID};

/// Who is looking at an approval card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viewer {
    /// The bro's owner.
    Owner,
    /// Anyone else. Also used while `GET /api/me` has not answered yet, so
    /// a button never shows for a click that would be refused.
    Other,
    /// No longer assigned by [`viewer`]: a missing owner is treated as
    /// the bootstrap admin, same as the host's `may_answer`. Kept so
    /// [`owner_layout`] still describes the old v0.1 card.
    Legacy,
}

/// Host `may_answer` treats a missing bro owner as the bootstrap admin
/// (`local-admin`). Empty or whitespace-only ids are missing too.
pub fn effective_owner_id(owner_id: Option<&str>) -> &str {
    owner_id
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .unwrap_or(BOOTSTRAP_ADMIN_ID)
}

/// Whether `me` is this bro's owner. Unknown `me` is not.
pub fn is_bro_owner(me: Option<&str>, owner_id: Option<&str>) -> bool {
    me.is_some_and(|id| id == effective_owner_id(owner_id))
}

/// Decide who `me` is for this card. `me` is the id from `GET /api/me`.
/// A missing owner is owned by [`BOOTSTRAP_ADMIN_ID`], not a Legacy card.
pub fn viewer(card: &ApprovalCard, me: Option<&str>) -> Viewer {
    if is_bro_owner(me, card.owner_id.as_deref()) {
        Viewer::Owner
    } else {
        Viewer::Other
    }
}

/// Whether a card uses the owner layout ([`lines`]): status line, and
/// buttons for the owner only. A group card always does (except a rule
/// card the owner answers in place). Anywhere else, a viewer who is not
/// the owner does too, because the host (`may_answer`) refuses everyone
/// but the bro's owner, in a DM as well. Legacy cards keep the v0.1 card.
pub fn owner_layout(in_group: bool, is_rule: bool, viewer: Viewer) -> bool {
    match viewer {
        Viewer::Legacy => false,
        Viewer::Other => true,
        Viewer::Owner => in_group && !is_rule,
    }
}

/// The owner's name for a sentence, or "its owner" when none is known.
pub fn owner_label(name: Option<&str>) -> String {
    match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => n.to_string(),
        None => Key::OwnerFallback.into(),
    }
}

/// Everything the group card draws that depends on state and viewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    /// Next to the dot at the top.
    pub status: String,
    pub dot: Dot,
    /// 11px `text3` line under the summary, when there is one.
    pub line: Option<String>,
    pub actions: ApprovalActions,
    /// 13px accent line (a 403 for a non-owner). Replaces the buttons.
    pub error: Option<String>,
}

/// Text and buttons for a group card. Not for [`Viewer::Legacy`] cards.
pub fn lines<Tz: TimeZone>(card: &ApprovalCard, ui: &CardUi, me: Option<&str>, tz: &Tz) -> Lines
where
    Tz::Offset: std::fmt::Display,
{
    let viewer = viewer(card, me);
    let owner = owner_label(card.owner_name.as_deref());
    let (status, dot) = status::approval_status(&card.state);
    let mut out = Lines {
        status: status.to_string(),
        dot,
        line: None,
        actions: ApprovalActions::None,
        error: None,
    };
    match &card.state {
        ApprovalState::Pending => {
            if viewer == Viewer::Owner {
                out.actions = super::interact::approval_actions(card, ui);
            } else {
                out.status = Key::ApprovalWaitingFor.into();
            }
        }
        ApprovalState::Approved { at, .. } => {
            let time = clock_in(*at, tz);
            out.line = Some(decided_line(true, viewer == Viewer::Owner, &owner, &time));
        }
        ApprovalState::Denied { at, .. } => {
            let time = clock_in(*at, tz);
            out.line = Some(decided_line(false, viewer == Viewer::Owner, &owner, &time));
        }
        // The status word already says "Expired"; nothing else to add.
        ApprovalState::Expired { .. } | ApprovalState::NeedsOwner => {}
    }
    if let Some(Note::OnlyOwner(named)) = &ui.note {
        let who = owner_label(named.as_deref().or(card.owner_name.as_deref()));
        out.error = Some(Key::ApprovalOnlyOwner.fill(&[&who]));
        out.actions = ApprovalActions::None;
    }
    out
}

/// 「Approved by you at 19:02」 and its three siblings. Without a time the
/// older, shorter copy is used so no sentence ends in a hole.
pub fn decided_line(approved: bool, by_you: bool, owner: &str, time: &str) -> String {
    match (approved, by_you, time.is_empty()) {
        (true, true, false) => Key::ApprovalApprovedByYou.fill(&[&time]),
        (false, true, false) => Key::ApprovalDeniedByYou.fill(&[&time]),
        (true, false, false) => Key::ApprovalApprovedBy.fill(&[&owner, &time]),
        (false, false, false) => Key::ApprovalDeniedBy.fill(&[&owner, &time]),
        (true, true, true) => Key::StatusApprovedFoot.into(),
        (false, true, true) => Key::StatusDeniedFoot.into(),
        (true, false, true) => Key::StatusApprovedByFoot.fill(&[&owner]),
        (false, false, true) => Key::StatusDeniedByFoot.fill(&[&owner]),
    }
}

/// Whether this user may approve or deny the card on desktop. A rule
/// (settings) card is admin-only; any other card is the bro's owner.
pub fn can_answer(card: &ApprovalCard, me: Option<&str>, role: Option<UserRole>) -> bool {
    if card.is_rule() {
        review_rules::may_decide(role)
    } else {
        viewer(card, me) == Viewer::Owner
    }
}

/// Read-only approval row on the phone (v0.2): status line, optional
/// muted second line, no buttons. Pending uses the yellow
/// `status.blocked` dot ([`Dot::Stuck`]); a resolved card uses the
/// same sentence as the desktop result line and its usual dot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneLine {
    pub status: String,
    pub line: Option<String>,
    pub dot: Dot,
}

/// Pending-owner status line: the waiting-for-you prefix plus the host
/// summary after [`host_text`], the same mapping the desktop approval
/// card uses (`view::approval_summary`). Empty summaries drop the
/// detail. A summary that already starts with the waiting-for-you
/// prefix is left as-is so the prefix is not doubled.
fn pending_waiting_status(summary: &str) -> String {
    let mapped = host_text(summary.trim());
    let mapped = mapped.trim();
    if mapped.is_empty() {
        return Key::ApprovalWaitingForYou.into();
    }
    let waiting = String::from(Key::ApprovalWaitingForYou);
    let already = mapped == waiting
        || mapped.starts_with(&format!("{waiting}: "))
        || mapped.starts_with(&format!("{waiting}:"))
        || mapped.starts_with(&format!("{waiting}："));
    if already {
        mapped.to_string()
    } else {
        Key::ApprovalWaitingForYouDetail.fill(&[&mapped])
    }
}

pub fn phone_line<Tz: TimeZone>(
    card: &ApprovalCard,
    me: Option<&str>,
    role: Option<UserRole>,
    tz: &Tz,
) -> PhoneLine
where
    Tz::Offset: std::fmt::Display,
{
    match &card.state {
        ApprovalState::Pending => {
            if can_answer(card, me, role) {
                let status = pending_waiting_status(&card.summary);
                PhoneLine {
                    status,
                    line: Some(Key::ApprovalHandleOnDesktop.into()),
                    dot: Dot::Stuck,
                }
            } else {
                let status = if card.is_rule() {
                    Key::ReviewRulesWaitingForAdmin.into()
                } else {
                    Key::ApprovalWaitingFor.into()
                };
                PhoneLine {
                    status,
                    line: None,
                    dot: Dot::Stuck,
                }
            }
        }
        ApprovalState::Approved { by, at } => resolved_phone_line(true, by, *at, card, me, tz),
        ApprovalState::Denied { by, at } => resolved_phone_line(false, by, *at, card, me, tz),
        other => {
            let (status, dot) = status::approval_status(other);
            PhoneLine {
                status: status.to_string(),
                line: None,
                dot,
            }
        }
    }
}

/// [`phone_line`] in the local timezone. The phone crate does not
/// depend on chrono directly.
pub fn phone_line_local(
    card: &ApprovalCard,
    me: Option<&str>,
    role: Option<UserRole>,
) -> PhoneLine {
    phone_line(card, me, role, &chrono::Local)
}

fn resolved_phone_line<Tz: TimeZone>(
    approved: bool,
    by: &Option<String>,
    at: i64,
    card: &ApprovalCard,
    me: Option<&str>,
    tz: &Tz,
) -> PhoneLine
where
    Tz::Offset: std::fmt::Display,
{
    let time = clock_in(at, tz);
    let by_you = match by.as_deref() {
        Some(super::ME_MARK) => true,
        Some(name) => me == Some(name),
        None => false,
    };
    let owner = owner_label(card.owner_name.as_deref());
    PhoneLine {
        status: decided_line(approved, by_you, &owner, &time),
        line: None,
        dot: if approved { Dot::Done } else { Dot::Void },
    }
}

/// Dot colour for a reminder row: the card's own dot colours.
pub fn reminder_dot(state: ReminderState) -> Dot {
    match state {
        ReminderState::Waiting => Dot::Waiting,
        ReminderState::Approved => Dot::Done,
        ReminderState::Denied | ReminderState::Expired => Dot::Void,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{scoped, Locale};
    use chrono::Utc;

    fn card(state: ApprovalState, owner: Option<(&str, &str)>) -> ApprovalCard {
        ApprovalCard {
            id: "a1".into(),
            bot: "Ada".into(),
            kind: super::super::ActionKind::Delete,
            summary: "Delete 12 old log files".into(),
            path: None,
            state,
            mcp_server: None,
            mcp_tool: None,
            reason_code: None,
            reason: None,
            owner_id: owner.map(|o| o.0.into()),
            owner_name: owner.map(|o| o.1.into()),
            task_id: None,
        }
    }

    /// 2026-10-07 19:02:00 UTC.
    fn at() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-10-07T19:02:00Z")
            .unwrap()
            .timestamp()
    }

    #[test]
    fn missing_owner_is_the_bootstrap_admin() {
        assert_eq!(effective_owner_id(None), BOOTSTRAP_ADMIN_ID);
        assert_eq!(effective_owner_id(Some("")), BOOTSTRAP_ADMIN_ID);
        assert_eq!(effective_owner_id(Some("  ")), BOOTSTRAP_ADMIN_ID);
        assert_eq!(effective_owner_id(Some("u-me")), "u-me");
        // owner null: only local-admin is the owner; unknown me is not.
        assert!(is_bro_owner(Some(BOOTSTRAP_ADMIN_ID), None));
        assert!(!is_bro_owner(Some("u-lin"), None));
        assert!(!is_bro_owner(None, None));
        // owner set: match / mismatch, unknown me is not.
        assert!(is_bro_owner(Some("u-me"), Some("u-me")));
        assert!(!is_bro_owner(Some("u-lin"), Some("u-me")));
        assert!(!is_bro_owner(None, Some("u-me")));
    }

    #[test]
    fn viewer_is_the_owner_only_when_the_ids_match() {
        let c = card(ApprovalState::Pending, Some(("u-me", "Panda")));
        assert_eq!(viewer(&c, Some("u-me")), Viewer::Owner);
        assert_eq!(viewer(&c, Some("u-lin")), Viewer::Other);
        assert_eq!(viewer(&c, None), Viewer::Other, "no buttons before /me");
        let old = card(ApprovalState::Pending, None);
        assert_eq!(
            viewer(&old, Some(BOOTSTRAP_ADMIN_ID)),
            Viewer::Owner,
            "null owner is local-admin's bro"
        );
        assert_eq!(viewer(&old, Some("u-lin")), Viewer::Other);
        assert_eq!(viewer(&old, None), Viewer::Other);
        assert_ne!(viewer(&old, Some("u-me")), Viewer::Legacy);
    }

    #[test]
    fn owner_waiting_gets_both_buttons_and_no_text_line() {
        let _en = scoped(Locale::En);
        let c = card(ApprovalState::Pending, Some(("u-me", "Panda")));
        let l = lines(&c, &CardUi::default(), Some("u-me"), &Utc);
        assert_eq!(l.actions, ApprovalActions::Decide { enabled: true });
        assert_eq!(l.status, "Needs your approval");
        assert_eq!(l.dot, Dot::Waiting);
        assert_eq!(l.line, None);
    }

    #[test]
    fn non_owner_waiting_has_no_buttons_and_asks_for_the_owner() {
        let c = card(ApprovalState::Pending, Some(("u-lin", "Lin")));
        let ui = CardUi::default();
        let en = {
            let _g = scoped(Locale::En);
            lines(&c, &ui, Some("u-me"), &Utc)
        };
        assert_eq!(en.actions, ApprovalActions::None);
        assert_eq!(en.status, "Waiting for this bro's owner");
        assert_eq!(en.dot, Dot::Waiting);
        let zh = {
            let _g = scoped(Locale::Zh);
            lines(&c, &ui, Some("u-me"), &Utc)
        };
        assert_eq!(zh.status, "等这个 bro 的主人审批");
        // Not even a busy flag or a stale note brings buttons back.
        let mut busy = CardUi::default();
        busy.busy = true;
        assert_eq!(
            lines(&c, &busy, Some("u-me"), &Utc).actions,
            ApprovalActions::None
        );
    }

    #[test]
    fn missing_owner_name_falls_back() {
        let mut c = card(ApprovalState::Pending, Some(("u-lin", "Lin")));
        c.owner_name = None;
        let en = {
            let _g = scoped(Locale::En);
            lines(&c, &CardUi::default(), Some("u-me"), &Utc)
        };
        assert_eq!(en.status, "Waiting for this bro's owner");
        let _g = scoped(Locale::Zh);
        assert_eq!(
            lines(&c, &CardUi::default(), Some("u-me"), &Utc).status,
            "等这个 bro 的主人审批"
        );
    }

    #[test]
    fn decided_lines_are_full_sentences_with_the_time() {
        let approved = ApprovalState::Approved { by: None, at: at() };
        let denied = ApprovalState::Denied { by: None, at: at() };
        let mine = |s: &ApprovalState| {
            lines(
                &card(s.clone(), Some(("u-me", "Panda"))),
                &CardUi::default(),
                Some("u-me"),
                &Utc,
            )
            .line
            .unwrap()
        };
        let theirs = |s: &ApprovalState| {
            lines(
                &card(s.clone(), Some(("u-lin", "Lin"))),
                &CardUi::default(),
                Some("u-me"),
                &Utc,
            )
            .line
            .unwrap()
        };
        {
            let _g = scoped(Locale::En);
            assert_eq!(mine(&approved), "Approved by you at 19:02");
            assert_eq!(mine(&denied), "Denied by you at 19:02");
            assert_eq!(theirs(&approved), "Approved by Lin at 19:02");
            assert_eq!(theirs(&denied), "Denied by Lin at 19:02");
        }
        let _g = scoped(Locale::Zh);
        assert_eq!(mine(&approved), "你于 19:02 批准");
        assert_eq!(mine(&denied), "你于 19:02 拒绝");
        assert_eq!(theirs(&approved), "Lin 于 19:02 批准");
        assert_eq!(theirs(&denied), "Lin 于 19:02 拒绝");
    }

    #[test]
    fn no_copy_uses_a_middle_dot() {
        for l in Locale::ALL {
            for k in [
                Key::ApprovalWaitingFor,
                Key::ApprovalApprovedBy,
                Key::ApprovalDeniedBy,
                Key::ApprovalApprovedByYou,
                Key::ApprovalDeniedByYou,
                Key::ApprovalDmReminder,
                Key::ApprovalOnlyOwner,
            ] {
                let s = crate::i18n::text_in(l, k);
                assert!(!s.contains('·') && !s.contains('`'), "{k:?} {l}: {s}");
            }
        }
    }

    #[test]
    fn expired_has_the_grey_dot_and_no_buttons_for_anyone() {
        let _g = scoped(Locale::En);
        for me in ["u-me", "u-lin"] {
            let c = card(ApprovalState::Expired { at: at() }, Some(("u-me", "Panda")));
            let l = lines(&c, &CardUi::default(), Some(me), &Utc);
            assert_eq!(l.status, "Expired");
            assert_eq!(l.dot, Dot::Void);
            assert_eq!(l.actions, ApprovalActions::None);
            assert_eq!(l.line, None);
        }
    }

    #[test]
    fn not_owner_403_drops_the_buttons_and_says_who_can() {
        let mut ui = CardUi::default();
        ui.action_done(Some(Note::OnlyOwner(None)));
        let c = card(ApprovalState::Pending, Some(("u-me", "Panda")));
        let en = {
            let _g = scoped(Locale::En);
            lines(&c, &ui, Some("u-me"), &Utc)
        };
        assert_eq!(en.actions, ApprovalActions::None);
        assert_eq!(en.error.as_deref(), Some("Only Panda can approve this."));
        let _g = scoped(Locale::Zh);
        assert_eq!(
            lines(&c, &ui, Some("u-me"), &Utc).error.as_deref(),
            Some("只有 Panda 能审批。")
        );
        // The host's own name wins when the card had none.
        let mut named = CardUi::default();
        named.action_done(Some(Note::OnlyOwner(Some("Lin".into()))));
        let mut c2 = c.clone();
        c2.owner_name = None;
        assert_eq!(
            lines(&c2, &named, Some("u-me"), &Utc).error.as_deref(),
            Some("只有 Lin 能审批。")
        );
    }

    #[test]
    fn decided_or_expired_403_shows_no_error_and_no_buttons() {
        let c = card(ApprovalState::Pending, Some(("u-me", "Panda")));
        let mut ui = CardUi::default();
        ui.action_done(Some(Note::Settled));
        let l = lines(&c, &ui, Some("u-me"), &Utc);
        assert_eq!(l.actions, ApprovalActions::None);
        assert_eq!(l.error, None);
        assert_eq!(Note::Settled.text(), "");
    }

    /// The server (`may_answer` in host/src/approvals.rs) lets only the
    /// bro's owner answer a bro's card. An admin who does not own the bro
    /// is refused too, so the client must not offer buttons by role.
    #[test]
    fn only_the_bro_owner_gets_buttons_not_an_admin_or_a_member() {
        let _en = scoped(Locale::En);
        // Bea belongs to the admin; Lin is a member.
        let c = card(ApprovalState::Pending, Some(("local-admin", "admin")));
        let ui = CardUi::default();
        let admin = lines(&c, &ui, Some("local-admin"), &Utc);
        assert_eq!(admin.actions, ApprovalActions::Decide { enabled: true });
        let lin = lines(&c, &ui, Some("local-member"), &Utc);
        assert_eq!(lin.actions, ApprovalActions::None, "no buttons, not greyed");
        assert_eq!(lin.status, "Waiting for this bro's owner");
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(
            lines(&c, &ui, Some("local-member"), &Utc).status,
            "等这个 bro 的主人审批"
        );

        assert_eq!(lin.error, None);
        // The other way round: Lin's bro, viewed by the admin.
        let lins = card(ApprovalState::Pending, Some(("local-member", "Lin")));
        assert_eq!(
            lines(&lins, &ui, Some("local-admin"), &Utc).actions,
            ApprovalActions::None
        );
        // Before /api/me answers nobody gets buttons.
        assert_eq!(lines(&c, &ui, None, &Utc).actions, ApprovalActions::None);
    }

    #[test]
    fn a_non_owner_never_gets_the_buttons_in_a_dm_or_on_a_rule_card() {
        // Group cards: owner layout for everyone.
        assert!(owner_layout(true, false, Viewer::Owner));
        assert!(owner_layout(true, false, Viewer::Other));
        // A rule card or a DM card: the owner keeps the full card...
        assert!(!owner_layout(true, true, Viewer::Owner));
        assert!(!owner_layout(false, false, Viewer::Owner));
        assert!(!owner_layout(false, true, Viewer::Owner));
        // ...anyone else gets the status line and no buttons.
        assert!(owner_layout(false, false, Viewer::Other));
        assert!(owner_layout(false, true, Viewer::Other));
        assert!(owner_layout(true, true, Viewer::Other));
        // A card without an owner (older host) keeps the v0.1 card.
        for (g, r) in [(true, false), (false, false), (true, true)] {
            assert!(!owner_layout(g, r, Viewer::Legacy));
        }
    }

    #[test]
    fn reminder_dots_follow_the_card() {
        assert_eq!(reminder_dot(ReminderState::Waiting), Dot::Waiting);
        assert_eq!(reminder_dot(ReminderState::Approved), Dot::Done);
        assert_eq!(reminder_dot(ReminderState::Denied), Dot::Void);
        assert_eq!(reminder_dot(ReminderState::Expired), Dot::Void);
    }

    fn pending_card(owner: Option<(&str, &str)>, summary: &str, rule: bool) -> ApprovalCard {
        ApprovalCard {
            id: "a1".into(),
            bot: if rule { "设置".into() } else { "Ada".into() },
            kind: if rule {
                super::super::ActionKind::RuleAdd
            } else {
                super::super::ActionKind::Delete
            },
            summary: summary.into(),
            path: None,
            state: ApprovalState::Pending,
            mcp_server: None,
            mcp_tool: None,
            reason_code: None,
            reason: None,
            owner_id: owner.map(|o| o.0.into()),
            owner_name: owner.map(|o| o.1.into()),
            task_id: None,
        }
    }

    #[test]
    fn phone_owner_pending_asks_to_decide_on_desktop() {
        let _en = scoped(Locale::En);
        let c = pending_card(Some(("u-me", "Panda")), "Delete 12 old log files", false);
        let l = phone_line(&c, Some("u-me"), None, &Utc);
        assert_eq!(
            l.status,
            "Waiting for your approval: Delete 12 old log files"
        );
        assert_eq!(l.line.as_deref(), Some("Approve or decline on desktop"));
        assert_eq!(l.dot, Dot::Stuck);
        let empty = pending_card(Some(("u-me", "Panda")), "  ", false);
        let l = phone_line(&empty, Some("u-me"), None, &Utc);
        assert_eq!(l.status, "Waiting for your approval");
        let _zh = scoped(Locale::Zh);
        let l = phone_line(&c, Some("u-me"), None, &Utc);
        assert_eq!(l.status, "等你批准：Delete 12 old log files");
        assert_eq!(l.line.as_deref(), Some("到桌面上批准或拒绝"));
    }

    #[test]
    fn phone_owner_pending_localizes_host_write_summary() {
        let c = pending_card(
            Some(("u-me", "Panda")),
            "要对外写入：Publish the release notes to the project blog",
            false,
        );
        {
            let _en = scoped(Locale::En);
            let l = phone_line(&c, Some("u-me"), None, &Utc);
            assert_eq!(
                l.status,
                "Waiting for your approval: Writes to an outside service: Publish the release notes to the project blog"
            );
            assert!(!l.status.contains("要对外写入"));
            assert_eq!(l.status.matches("Waiting for your approval").count(), 1);
            assert_eq!(l.line.as_deref(), Some("Approve or decline on desktop"));
        }
        let _zh = scoped(Locale::Zh);
        let l = phone_line(&c, Some("u-me"), None, &Utc);
        assert_eq!(
            l.status,
            "等你批准：要对外写入：Publish the release notes to the project blog"
        );
        assert_eq!(l.status.matches("等你批准").count(), 1);
        assert_eq!(l.line.as_deref(), Some("到桌面上批准或拒绝"));
    }

    #[test]
    fn phone_owner_pending_localizes_chinese_host_write_summary() {
        let c = pending_card(
            Some(("u-me", "Panda")),
            "要对外写入：把发布说明发到项目博客",
            false,
        );
        {
            let _en = scoped(Locale::En);
            let l = phone_line(&c, Some("u-me"), None, &Utc);
            assert_eq!(
                l.status,
                "Waiting for your approval: Writes to an outside service: 把发布说明发到项目博客"
            );
            assert!(!l.status.contains("要对外写入"));
            assert_eq!(l.status.matches("Waiting for your approval").count(), 1);
        }
        let _zh = scoped(Locale::Zh);
        let l = phone_line(&c, Some("u-me"), None, &Utc);
        assert_eq!(l.status, "等你批准：要对外写入：把发布说明发到项目博客");
        assert_eq!(l.status.matches("等你批准").count(), 1);
    }

    #[test]
    fn phone_owner_pending_does_not_double_a_waiting_prefix() {
        let _en = scoped(Locale::En);
        let already = pending_card(
            Some(("u-me", "Panda")),
            "Waiting for your approval: Delete 12 old log files",
            false,
        );
        let l = phone_line(&already, Some("u-me"), None, &Utc);
        assert_eq!(
            l.status,
            "Waiting for your approval: Delete 12 old log files"
        );
        assert_eq!(l.status.matches("Waiting for your approval").count(), 1);
    }

    #[test]
    fn phone_non_owner_pending_waits_for_the_bro_owner() {
        let c = pending_card(Some(("u-lin", "Lin")), "Delete 12 old log files", false);
        let en = {
            let _g = scoped(Locale::En);
            phone_line(&c, Some("u-me"), Some(UserRole::Admin), &Utc)
        };
        assert_eq!(en.status, "Waiting for this bro's owner");
        assert_eq!(en.line, None);
        assert_eq!(en.dot, Dot::Stuck);
        let zh = {
            let _g = scoped(Locale::Zh);
            phone_line(&c, Some("u-me"), Some(UserRole::Admin), &Utc)
        };
        assert_eq!(zh.status, "等这个 bro 的主人审批");
        assert_eq!(zh.line, None);
    }

    #[test]
    fn phone_member_pending_rule_waits_for_an_admin() {
        use robroz_contract::UserRole;
        let c = pending_card(None, "允许发周报", true);
        let en = {
            let _g = scoped(Locale::En);
            phone_line(&c, Some("u-lin"), Some(UserRole::Member), &Utc)
        };
        assert_eq!(en.status, "Waiting for an admin");
        assert_eq!(en.line, None);
        assert_eq!(en.dot, Dot::Stuck);
        let zh = {
            let _g = scoped(Locale::Zh);
            phone_line(&c, Some("u-lin"), Some(UserRole::Member), &Utc)
        };
        assert_eq!(zh.status, "等管理员确认");
        let admin = {
            let _g = scoped(Locale::Zh);
            phone_line(&c, Some("u-me"), Some(UserRole::Admin), &Utc)
        };
        assert_eq!(admin.status, "等你批准：允许发周报");
        assert_eq!(admin.line.as_deref(), Some("到桌面上批准或拒绝"));
    }

    #[test]
    fn phone_resolved_reuses_the_desktop_result_line() {
        let _en = scoped(Locale::En);
        let mut c = pending_card(Some(("u-me", "Panda")), "Delete 12 old log files", false);
        c.state = ApprovalState::Denied {
            by: Some(super::super::ME_MARK.into()),
            at: at(),
        };
        let l = phone_line(&c, Some("u-me"), None, &Utc);
        assert_eq!(
            l.status,
            decided_line(false, true, "Panda", &clock_in(at(), &Utc))
        );
        assert_eq!(l.line, None);
        assert_eq!(l.dot, Dot::Void);
        c.state = ApprovalState::Approved {
            by: Some("Lin".into()),
            at: at(),
        };
        let l = phone_line(&c, Some("u-me"), None, &Utc);
        assert_eq!(
            l.status,
            decided_line(true, false, "Panda", &clock_in(at(), &Utc))
        );
        assert_eq!(l.dot, Dot::Done);
    }
}
