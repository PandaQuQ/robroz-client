//! The one-line reminder in the owner's DM with a bro (v0.2): "{bro} is
//! waiting for your approval in {group}". Not a card and has no buttons;
//! the whole row opens the group and the approval card in it.

use chrono::TimeZone;
use robroz_contract::{Event, ReminderState};

use super::group_approval::reminder_dot;
use super::status::{clock_in, Dot};
use crate::i18n::Key;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reminder {
    /// `reminder:{approvalId}`; the feed updates the row in place by it.
    pub id: String,
    pub approval_id: String,
    pub group_id: String,
    pub group_name: String,
    pub bot_name: String,
    pub owner_name: String,
    /// The bro's owner (`ownerUserId`). Used so a non-owner never sees
    /// "waiting for your approval".
    pub owner_id: String,
    pub state: ReminderState,
    /// `createdAt`, unix seconds.
    pub created_at: i64,
    /// `answeredAt` once decided or expired, unix seconds.
    pub answered_at: Option<i64>,
}

fn unix(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|t| t.timestamp())
}

impl Reminder {
    /// Build from the `approval_reminder` event; `None` for any other.
    pub fn from_event(event: &Event) -> Option<Self> {
        let Event::ApprovalReminder {
            id,
            approval_id,
            group_id,
            group_name,
            bot_name,
            owner_name,
            owner_user_id,
            state,
            created_at,
            answered_at,
            ..
        } = event
        else {
            return None;
        };
        Some(Self {
            id: id.clone(),
            approval_id: approval_id.clone(),
            group_id: group_id.clone(),
            group_name: group_name.clone(),
            bot_name: bot_name.clone(),
            owner_name: owner_name.clone(),
            owner_id: owner_user_id.clone(),
            state: *state,
            created_at: unix(created_at).unwrap_or(0),
            answered_at: answered_at.as_deref().and_then(unix),
        })
    }

    pub fn dot(&self) -> Dot {
        reminder_dot(self.state)
    }

    /// Whether `me` is this card's owner. Unknown id (`None`) is not.
    /// A missing or empty `owner_id` is the bootstrap admin, same as
    /// the card and the list preview ([`super::is_bro_owner`]).
    pub fn is_owner(&self, me: Option<&str>) -> bool {
        super::is_bro_owner(me, Some(self.owner_id.as_str()))
    }

    /// The row's text: the ask while waiting, then the result.
    /// A non-owner waiting sees the same sentence as the card
    /// (`ApprovalWaitingFor`), not "waiting for your approval".
    pub fn text<Tz: TimeZone>(&self, _tz: &Tz, me: Option<&str>) -> String
    where
        Tz::Offset: std::fmt::Display,
    {
        match self.state {
            ReminderState::Waiting if self.is_owner(me) => {
                Key::ApprovalDmReminder.fill(&[&self.bot_name, &self.group_name])
            }
            ReminderState::Waiting => Key::ApprovalWaitingFor.into(),
            // The reminder only goes to the owner. Time sits on the right.
            ReminderState::Approved => Key::ApprovalDmApprovedByYou.to_string(),
            ReminderState::Denied => Key::ApprovalDmDeniedByYou.to_string(),
            ReminderState::Expired => Key::ReviewRulesExpired.into(),
        }
    }

    /// The 11px time on the right. Every row shows one: `createdAt` while
    /// waiting, `answeredAt` (or `createdAt` if that is missing) otherwise.
    pub fn time<Tz: TimeZone>(&self, tz: &Tz) -> Option<String>
    where
        Tz::Offset: std::fmt::Display,
    {
        let at = match self.state {
            ReminderState::Waiting => self.created_at,
            ReminderState::Approved | ReminderState::Denied | ReminderState::Expired => {
                self.answered_at.unwrap_or(self.created_at)
            }
        };
        Some(clock_in(at, tz)).filter(|t| !t.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{scoped, Locale};
    use chrono::Utc;

    fn event(state: ReminderState, answered: Option<&str>) -> Event {
        Event::ApprovalReminder {
            id: "reminder:a1".into(),
            approval_id: "a1".into(),
            group_id: "g1".into(),
            group_name: "Site".into(),
            bot_id: "bot-ada".into(),
            bot_name: "Ada".into(),
            owner_user_id: "u-me".into(),
            owner_name: "Panda".into(),
            state,
            created_at: "2026-10-07T19:00:00Z".into(),
            answered_at: answered.map(Into::into),
        }
    }

    #[test]
    fn waiting_row_reads_as_the_ask_with_the_time_on_the_right() {
        let r = Reminder::from_event(&event(ReminderState::Waiting, None)).unwrap();
        {
            let _g = scoped(Locale::En);
            assert_eq!(
                r.text(&Utc, Some("u-me")),
                "Ada is waiting for your approval in Site"
            );
            assert_eq!(r.time(&Utc).as_deref(), Some("19:00"));
        }
        let _g = scoped(Locale::Zh);
        assert_eq!(r.text(&Utc, Some("u-me")), "Ada 在 Site 等你审批");
        assert_eq!(r.dot(), Dot::Waiting);
    }

    #[test]
    fn decided_row_turns_into_the_result_with_the_card_dot() {
        let r = Reminder::from_event(&event(
            ReminderState::Approved,
            Some("2026-10-07T19:02:00Z"),
        ))
        .unwrap();
        let _g = scoped(Locale::En);
        assert_eq!(r.text(&Utc, Some("u-me")), "Approved by you");
        assert_eq!(r.time(&Utc).as_deref(), Some("19:02"));
        assert_eq!(r.dot(), Dot::Done);
        let d = Reminder::from_event(&event(ReminderState::Denied, Some("2026-10-07T19:03:00Z")))
            .unwrap();
        assert_eq!(d.text(&Utc, Some("u-me")), "Denied by you");
        assert_eq!(d.time(&Utc).as_deref(), Some("19:03"));
        assert_eq!(d.dot(), Dot::Void);
        let _g = scoped(Locale::Zh);
        assert_eq!(r.text(&Utc, Some("u-me")), "你已批准");
        assert_eq!(d.text(&Utc, Some("u-me")), "你已拒绝");
    }

    #[test]
    fn expired_row_is_grey_and_says_expired() {
        let r = Reminder::from_event(&event(ReminderState::Expired, Some("2026-10-07T19:30:00Z")))
            .unwrap();
        let _g = scoped(Locale::Zh);
        assert_eq!(r.text(&Utc, Some("u-me")), "已过期");
        assert_eq!(r.dot(), Dot::Void);
        assert_eq!(r.time(&Utc).as_deref(), Some("19:30"));
    }

    #[test]
    fn a_non_owner_waiting_sees_the_card_sentence() {
        let r = Reminder::from_event(&event(ReminderState::Waiting, None)).unwrap();
        {
            let _g = scoped(Locale::En);
            assert_eq!(r.text(&Utc, Some("u-lin")), "Waiting for this bro's owner");
            assert_eq!(r.text(&Utc, None), "Waiting for this bro's owner");
        }
        let _g = scoped(Locale::Zh);
        assert_eq!(r.text(&Utc, Some("u-lin")), "等这个 bro 的主人审批");
    }

    #[test]
    fn a_missing_owner_is_the_bootstrap_admin() {
        use robroz_contract::BOOTSTRAP_ADMIN_ID;
        let mut ev = event(ReminderState::Waiting, None);
        if let Event::ApprovalReminder { owner_user_id, .. } = &mut ev {
            *owner_user_id = String::new();
        }
        let r = Reminder::from_event(&ev).unwrap();
        assert!(r.is_owner(Some(BOOTSTRAP_ADMIN_ID)));
        assert!(!r.is_owner(Some("u-lin")));
        assert!(!r.is_owner(None));
        {
            let _g = scoped(Locale::En);
            assert_eq!(
                r.text(&Utc, Some(BOOTSTRAP_ADMIN_ID)),
                "Ada is waiting for your approval in Site"
            );
            assert_eq!(r.text(&Utc, Some("u-lin")), "Waiting for this bro's owner");
            assert_eq!(r.text(&Utc, None), "Waiting for this bro's owner");
        }
        // Owner set: match / mismatch stay as they were.
        let named = Reminder::from_event(&event(ReminderState::Waiting, None)).unwrap();
        assert!(named.is_owner(Some("u-me")));
        assert!(!named.is_owner(Some("u-lin")));
        assert!(!named.is_owner(None));
    }

    #[test]
    fn other_events_are_not_reminders() {
        let msg = Event::Message {
            id: "m".into(),
            kind: robroz_contract::MessageKind::Text,
            body: "x".into(),
            created_at: "2026-10-07T19:00:00Z".into(),
            author_user_id: None,
            author_name: None,
        };
        assert_eq!(Reminder::from_event(&msg), None);
    }
}
