//! Allow-rules settings page (docs/design/auto-review.md).
//!
//! Adding or deleting a rule on this page shows a confirm card inline.
//! The owner answers it here through the normal approvals endpoint.
//! The list shows confirmed rules only.

use crate::i18n::Key;
use chrono::{DateTime, Datelike, TimeZone};

/// Sidebar and page title, directly under 「MCP」.
pub const TITLE: Key = Key::ReviewRulesTitle;
pub const ADD: Key = Key::ReviewRulesAdd;
pub const DELETE: Key = Key::ReviewRulesDelete;
/// Empty state, wording from 最优雅UI姐.
pub const EMPTY: Key = Key::ReviewRulesEmpty;
/// Host limit on rule text.
pub use robroz_contract::REVIEW_RULE_MAX;

/// Red validation under the input when the draft is longer than
/// [`REVIEW_RULE_MAX`]. `over` is how many characters to delete.
/// Wording from 最优雅UI姐.
pub fn too_long_note(over: usize) -> String {
    Key::ReviewRulesTooLong.fill(&[&REVIEW_RULE_MAX, &over])
}

/// Whether `note` is a [`too_long_note`] in the active locale.
fn is_too_long_note(note: &str) -> bool {
    let template = Key::ReviewRulesTooLong.as_str();
    let head = template.split('{').next().unwrap_or(template);
    !head.is_empty() && note.starts_with(head)
}

/// Shown after approve/deny returns 409 on this page, in grey.
/// Wording from 最优雅UI姐.
pub const ALREADY_ANSWERED: Key = Key::ReviewRulesAlreadyAnswered;
/// Placeholder, wording from 最优雅UI姐.
pub const PLACEHOLDER: Key = Key::ReviewRulesPlaceholder;
pub const NEED_TEXT: Key = Key::ReviewRulesNeedText;
pub const LOADING: Key = Key::ReviewRulesLoading;
pub const LOAD_FAILED: Key = Key::ReviewRulesLoadFailed;

/// Grey line on an add confirm card, wording from 最优雅UI姐.
pub const ADD_HINT: Key = Key::ReviewRulesAddHint;
/// Grey line on a delete confirm card, wording from 最优雅UI姐.
pub const DELETE_HINT: Key = Key::ReviewRulesDeleteHint;
/// Primary button on an add card, wording from 最优雅UI姐.
pub const ADD_ACTION: Key = Key::ReviewRulesAddAction;
/// Primary (red) button on a delete card, wording from the design.
pub const DELETE_ACTION: Key = Key::ReviewRulesDeleteAction;
/// Deny button on rule confirm cards, wording from 最优雅UI姐.
pub const DROP: Key = Key::ReviewRulesDrop;

/// Status line on an open rule card for someone who may not answer it.
pub const WAITING_FOR_ADMIN: Key = Key::ReviewRulesWaitingForAdmin;

/// Whether this user may approve or deny a settings (rule) card. The host
/// (`may_answer` in host/src/approvals.rs) lets only an admin answer a
/// card from the settings bot; anyone may propose one. An unknown role
/// (`GET /api/me` not answered) gets no buttons, so no click is refused.
pub fn may_decide(role: Option<robroz_contract::UserRole>) -> bool {
    role == Some(robroz_contract::UserRole::Admin)
}

/// Add and Delete on the rules page: only an admin sees them. Members
/// do not get greyed controls (no dead UI).
pub fn shows_edit_controls(can_decide: bool) -> bool {
    can_decide
}

/// The "Confirm to add / delete" line on a rule card. Only someone who
/// can answer sees it; a member gets the waiting line and the rule text,
/// and not this hint.
pub fn confirm_hint(is_delete: bool, can_decide: bool) -> Option<Key> {
    can_decide.then_some(if is_delete { DELETE_HINT } else { ADD_HINT })
}

/// Prefix on a blocked-action DM card's reason line, wording from 最优雅UI姐.
pub const BLOCKED_REASON_PREFIX: Key = Key::ReviewRulesBlockedReasonPrefix;

/// Grey reason line for a blocked-action card. `shown` is already the
/// client copy from [`super::codes::reason_text`].
pub fn blocked_reason_line(shown: &str) -> String {
    format!("{BLOCKED_REASON_PREFIX}{shown}")
}

/// Group-thread line under 「等你批准」, wording from the design.
pub const SENT_TO_OWNER: Key = Key::ReviewRulesSentToOwner;
/// Top-right status on an expired card, wording from the design.
pub const EXPIRED: Key = Key::ReviewRulesExpired;
/// Footnote text on an expired card, wording from the design.
pub const EXPIRED_FOOT: Key = Key::ReviewRulesExpiredFoot;

/// Host writes this exact string on an expired card. Compare with `==`,
/// never hand-write the text at call sites.
pub use robroz_contract::APPROVAL_EXPIRED_REASON;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub id: String,
    pub text: String,
    pub created_at: String,
}

impl Rule {
    /// 「添加于 {日期}」, local `M月D日` from `createdAt`. Empty if unparsable.
    pub fn added_on<Tz: TimeZone>(&self, tz: &Tz) -> String
    where
        Tz::Offset: std::fmt::Display,
    {
        added_on(&self.created_at, tz)
    }
}

pub fn added_on<Tz: TimeZone>(created_at: &str, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    DateTime::parse_from_rfc3339(created_at)
        .map(|t| {
            let local = t.with_timezone(tz);
            Key::ReviewRulesAddedOn.fill(&[&crate::i18n::month_day(local.month(), local.day())])
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingKind {
    Add,
    Delete { rule_id: String },
}

/// In-page confirm card after POST/DELETE returns 202.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub id: String,
    pub kind: PendingKind,
    pub text: String,
    pub state: super::ApprovalState,
}

impl Pending {
    pub fn is_add(&self) -> bool {
        matches!(self.kind, PendingKind::Add)
    }

    pub fn is_delete_of(&self, rule_id: &str) -> bool {
        matches!(&self.kind, PendingKind::Delete { rule_id: id } if id == rule_id)
    }

    pub fn is_open(&self) -> bool {
        matches!(self.state, super::ApprovalState::Pending)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Page {
    /// `None` until the first successful load.
    pub rules: Option<Vec<Rule>>,
    pub draft: String,
    pub busy: bool,
    pub note: Option<String>,
    /// Confirm card shown on this page after a 202.
    pub pending: Option<Pending>,
    /// Stashed add/delete payload while the propose request is in flight.
    propose: Option<(PendingKind, String)>,
}

impl Page {
    pub fn load(&mut self, rules: Vec<Rule>) {
        self.rules = Some(rules);
        self.busy = false;
    }

    /// Fresh open of the page: drop a leftover confirm card and notes.
    pub fn reset_for_open(&mut self) {
        self.note = None;
        self.pending = None;
        self.propose = None;
        self.busy = true;
    }

    pub fn opened(&mut self, rules: Vec<Rule>) {
        self.note = None;
        self.pending = None;
        self.propose = None;
        self.load(rules);
    }

    pub fn load_failed(&mut self, reason: &str) {
        self.busy = false;
        self.note = Some(if reason.is_empty() {
            LOAD_FAILED.to_string()
        } else {
            Key::ReviewRulesLoadFailedBecause.fill(&[&crate::i18n::host_text(reason)])
        });
    }

    pub fn set_draft(&mut self, text: String) {
        self.draft = text;
        let n = self.draft.trim().chars().count();
        if n > REVIEW_RULE_MAX {
            self.note = Some(too_long_note(n - REVIEW_RULE_MAX));
        } else if self
            .note
            .as_deref()
            .is_some_and(|n| n == NEED_TEXT || n == ALREADY_ANSWERED || is_too_long_note(n))
        {
            self.note = None;
        }
    }

    /// Trimmed body for POST, or an inline note if empty or too long.
    pub fn to_add(&mut self) -> Option<String> {
        let text = self.draft.trim().to_string();
        if text.is_empty() {
            self.note = Some(NEED_TEXT.into());
            return None;
        }
        // One line only, as the design requires.
        if text.lines().count() > 1 {
            self.note = Some(NEED_TEXT.into());
            return None;
        }
        let n = text.chars().count();
        if n > REVIEW_RULE_MAX {
            self.note = Some(too_long_note(n - REVIEW_RULE_MAX));
            return None;
        }
        self.note = None;
        Some(text)
    }

    /// Remember the add text, then fire POST.
    pub fn begin_add(&mut self, text: String) {
        self.busy = true;
        self.propose = Some((PendingKind::Add, text));
    }

    /// Remember which rule is being deleted, then fire DELETE.
    pub fn begin_delete(&mut self, rule_id: String, text: String) {
        self.busy = true;
        self.propose = Some((PendingKind::Delete { rule_id }, text));
    }

    /// POST/DELETE returned 202: show the confirm card on this page.
    pub fn proposed(&mut self, approval_id: String) {
        self.busy = false;
        self.draft.clear();
        self.note = None;
        if let Some((kind, text)) = self.propose.take() {
            self.pending = Some(Pending {
                id: approval_id,
                kind,
                text,
                state: super::ApprovalState::Pending,
            });
        }
    }

    /// After 202, the confirm card loaded from `GET /api/approvals/{id}`.
    pub fn proposed_event(&mut self, event: &robroz_contract::Event) {
        let Some(card) = super::ApprovalCard::from_event(event) else {
            self.propose_failed(Key::ReviewRulesCardMissing.as_str());
            return;
        };
        self.busy = false;
        self.draft.clear();
        self.note = None;
        let kind = match self.propose.take() {
            Some((kind, _)) => kind,
            None => {
                if card.kind == super::ActionKind::RuleDelete {
                    PendingKind::Delete {
                        rule_id: String::new(),
                    }
                } else {
                    PendingKind::Add
                }
            }
        };
        self.pending = Some(Pending {
            id: card.id,
            kind,
            text: card.summary,
            state: card.state,
        });
    }

    pub fn propose_failed(&mut self, reason: &str) {
        self.busy = false;
        self.propose = None;
        self.note = Some(reason.to_string());
    }

    /// Owner approved or denied here: drop the card (caller refreshes GET).
    pub fn finish_pending(&mut self) {
        self.busy = false;
        self.pending = None;
        self.note = None;
        self.propose = None;
    }

    /// 409: already answered elsewhere.
    pub fn answered_elsewhere(&mut self) {
        self.busy = false;
        self.pending = None;
        self.propose = None;
        self.note = Some(ALREADY_ANSWERED.into());
    }

    pub fn expire_pending(&mut self, at: i64) {
        self.busy = false;
        if let Some(pending) = &mut self.pending {
            pending.state = super::ApprovalState::Expired { at };
        }
    }

    /// Input and row actions are locked while a request is in flight or a
    /// confirm card is still waiting for an answer.
    pub fn locked(&self) -> bool {
        self.busy || self.pending.as_ref().is_some_and(Pending::is_open)
    }

    pub fn is_empty(&self) -> bool {
        self.rules.as_ref().is_some_and(|r| r.is_empty())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_an_admin_may_decide_a_rule_card() {
        use robroz_contract::UserRole;
        assert!(may_decide(Some(UserRole::Admin)));
        assert!(!may_decide(Some(UserRole::Member)));
        assert!(!may_decide(None), "no buttons before /api/me answers");
    }

    #[test]
    fn only_an_admin_sees_add_and_delete_on_the_rules_page() {
        use robroz_contract::UserRole;
        assert!(shows_edit_controls(may_decide(Some(UserRole::Admin))));
        assert!(!shows_edit_controls(may_decide(Some(UserRole::Member))));
        assert!(!shows_edit_controls(may_decide(None)));
    }

    #[test]
    fn a_member_does_not_see_the_confirm_hint() {
        assert_eq!(confirm_hint(false, true), Some(ADD_HINT));
        assert_eq!(confirm_hint(true, true), Some(DELETE_HINT));
        assert_eq!(confirm_hint(false, false), None);
        assert_eq!(confirm_hint(true, false), None);
    }

    use super::*;
    use chrono::Utc;

    fn rule(id: &str, text: &str, at: &str) -> Rule {
        Rule {
            id: id.into(),
            text: text.into(),
            created_at: at.into(),
        }
    }

    #[test]
    fn empty_list_is_the_empty_state() {
        let mut page = Page::default();
        assert!(!page.is_empty());
        page.opened(Vec::new());
        assert!(page.is_empty());
        assert_eq!(page.note, None);
    }

    #[test]
    fn list_keeps_text_and_added_on() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let mut page = Page::default();
        page.opened(vec![rule(
            "r1",
            "允许合并已经过 CI 的 PR",
            "2026-11-20T08:00:00Z",
        )]);
        let r = &page.rules.as_ref().unwrap()[0];
        assert_eq!(r.text, "允许合并已经过 CI 的 PR");
        assert_eq!(r.added_on(&Utc), "添加于 11月20日");
    }

    #[test]
    fn add_requires_a_non_empty_line() {
        let mut page = Page::default();
        page.opened(Vec::new());
        assert_eq!(page.to_add(), None);
        assert_eq!(page.note.as_deref(), Some(NEED_TEXT.as_str()));
        page.set_draft("   ".into());
        assert_eq!(page.to_add(), None);
        page.set_draft("允许发周报\n第二行".into());
        assert_eq!(page.to_add(), None);
        page.set_draft("  允许发周报  ".into());
        assert_eq!(page.to_add().as_deref(), Some("允许发周报"));
        assert_eq!(page.note, None);
    }

    #[test]
    fn add_rejects_more_than_max_chars() {
        let mut page = Page::default();
        page.opened(Vec::new());
        page.set_draft("字".repeat(REVIEW_RULE_MAX + 1));
        assert_eq!(page.note.as_deref(), Some(too_long_note(1).as_str()));
        assert_eq!(page.to_add(), None);
        assert_eq!(page.note.as_deref(), Some(too_long_note(1).as_str()));
        page.set_draft("字".repeat(REVIEW_RULE_MAX + 5));
        assert_eq!(page.note.as_deref(), Some(too_long_note(5).as_str()));
        page.set_draft("字".repeat(REVIEW_RULE_MAX));
        assert_eq!(page.note, None);
        assert_eq!(
            page.to_add().as_deref(),
            Some("字".repeat(REVIEW_RULE_MAX).as_str())
        );
        assert_eq!(page.note, None);
    }

    #[test]
    fn proposed_add_shows_an_inline_card_and_does_not_list_the_rule() {
        let mut page = Page::default();
        page.opened(Vec::new());
        let text = page.to_add().or_else(|| {
            page.set_draft("允许发周报".into());
            page.to_add()
        });
        let text = text.unwrap();
        page.begin_add(text.clone());
        page.proposed("apr-1".into());
        assert!(page.draft.is_empty());
        assert_eq!(page.note, None);
        let pending = page.pending.as_ref().unwrap();
        assert_eq!(pending.id, "apr-1");
        assert_eq!(pending.text, "允许发周报");
        assert!(pending.is_add());
        assert!(pending.is_open());
        assert!(page.is_empty(), "unconfirmed rules stay off the list");
        assert!(page.locked());
        page.finish_pending();
        assert!(page.pending.is_none());
        assert!(!page.locked());
    }

    #[test]
    fn proposed_delete_leaves_the_rule_listed_and_pins_the_row() {
        let mut page = Page::default();
        page.opened(vec![rule("r1", "允许发周报", "2026-11-20T08:00:00Z")]);
        page.begin_delete("r1".into(), "允许发周报".into());
        page.proposed("apr-del-r1".into());
        assert_eq!(page.rules.as_ref().unwrap().len(), 1);
        assert_eq!(page.note, None);
        let pending = page.pending.as_ref().unwrap();
        assert!(pending.is_delete_of("r1"));
        assert_eq!(pending.text, "允许发周报");
        assert!(page.locked());
    }

    #[test]
    fn answered_elsewhere_drops_the_card() {
        let mut page = Page::default();
        page.opened(Vec::new());
        page.begin_add("允许发周报".into());
        page.proposed("apr-1".into());
        page.answered_elsewhere();
        assert!(page.pending.is_none());
        assert_eq!(page.note.as_deref(), Some(ALREADY_ANSWERED.as_str()));
        assert!(!page.locked());
    }

    #[test]
    fn expire_pending_keeps_the_settled_card() {
        let mut page = Page::default();
        page.opened(Vec::new());
        page.begin_add("允许发周报".into());
        page.proposed("apr-1".into());
        page.expire_pending(1_795_164_480);
        let pending = page.pending.as_ref().unwrap();
        assert!(matches!(
            pending.state,
            crate::cards::ApprovalState::Expired { at: 1_795_164_480 }
        ));
        assert!(!pending.is_open());
        assert!(!page.locked());
    }

    #[test]
    fn blocked_reason_line_uses_the_settled_prefix() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        assert_eq!(
            blocked_reason_line("这条操作会改生产数据"),
            "被拦原因：这条操作会改生产数据"
        );
    }

    #[test]
    fn confirm_hints_are_the_settled_copy() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        assert_eq!(ADD_HINT, "确认后加入放行规则");
        assert_eq!(DELETE_HINT, "确认后删除这条规则");
        assert_eq!(PLACEHOLDER, "用一句话写，比如：允许合并 CI 全绿的 PR");
        assert_eq!(too_long_note(3), "最多 200 字，还要删掉 3 字");
        assert_eq!(REVIEW_RULE_MAX, 200);
        assert_eq!(ALREADY_ANSWERED, "这张卡已经处理过了");
    }
}
