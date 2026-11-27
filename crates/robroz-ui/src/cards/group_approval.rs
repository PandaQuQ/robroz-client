//! The approval card in a group (v0.2): only the bro's owner decides.
//!
//! Pure rules for what the card shows, so the view only draws. See the
//! group approval spec: the owner sees Approve and Deny; everyone else sees
//! a read-only status and no buttons at all (not greyed out either).

use chrono::TimeZone;
use robroz_contract::ReminderState;

use super::interact::{ApprovalActions, CardUi, Note};
use super::status::{self, clock_in, Dot};
use super::{ApprovalCard, ApprovalState};
use crate::i18n::Key;

/// Who is looking at an approval card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Viewer {
    /// The bro's owner.
    Owner,
    /// Anyone else. Also used while `GET /api/me` has not answered yet, so
    /// a button never shows for a click that would be refused.
    Other,
    /// The host sent no owner (an older host): the v0.1 card.
    Legacy,
}

/// Decide who `me` is for this card. `me` is the id from `GET /api/me`.
pub fn viewer(card: &ApprovalCard, me: Option<&str>) -> Viewer {
    match (card.owner_id.as_deref(), me) {
        (None, _) => Viewer::Legacy,
        (Some(owner), Some(me)) if owner == me => Viewer::Owner,
        (Some(_), _) => Viewer::Other,
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
        }
    }

    /// 2026-10-07 19:02:00 UTC.
    fn at() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-10-07T19:02:00Z")
            .unwrap()
            .timestamp()
    }

    #[test]
    fn viewer_is_the_owner_only_when_the_ids_match() {
        let c = card(ApprovalState::Pending, Some(("u-me", "Panda")));
        assert_eq!(viewer(&c, Some("u-me")), Viewer::Owner);
        assert_eq!(viewer(&c, Some("u-lin")), Viewer::Other);
        assert_eq!(viewer(&c, None), Viewer::Other, "no buttons before /me");
        let old = card(ApprovalState::Pending, None);
        assert_eq!(viewer(&old, Some("u-me")), Viewer::Legacy);
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
    fn reminder_dots_follow_the_card() {
        assert_eq!(reminder_dot(ReminderState::Waiting), Dot::Waiting);
        assert_eq!(reminder_dot(ReminderState::Approved), Dot::Done);
        assert_eq!(reminder_dot(ReminderState::Denied), Dot::Void);
        assert_eq!(reminder_dot(ReminderState::Expired), Dot::Void);
    }
}
