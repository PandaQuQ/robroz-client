//! `ROBROZ_FAKE_SCENE=approvals`: every state of the v0.2 group approval
//! card and the owner's DM reminder row, for review screenshots. The cards
//! are real `approval` / `approval_reminder` events, so they go through the
//! same decoding and feed code as a host's. The person using the client is
//! [`FAKE_ME_ID`]; the other owner is "Lin".

use robroz_contract::{
    ApprovalAction, ApprovalDecision, Bro, Event, MessageKind, ReminderState, Unread,
};

use super::{FakeSource, GroupEvent, GroupTab};
use crate::api::{FAKE_ME_ID, FAKE_ME_NAME};

/// Ada's DM with the owner (Panda): the reminder rows.
pub const DM: &str = "dm-ada";
/// Owner view, waiting: Approve and Deny.
pub const OWN_WAITING: &str = "g-ap-own";
/// Owner view, after approving.
pub const OWN_DONE: &str = "g-ap-own-done";
/// Non-owner view, waiting.
pub const OTHER_WAITING: &str = "g-ap-other";
/// Non-owner view, approved by the owner.
pub const OTHER_DONE: &str = "g-ap-other-done";
/// Expired.
pub const EXPIRED: &str = "g-ap-expired";

const LIN_ID: &str = "u-lin";
const LIN_NAME: &str = "Lin";

fn tab(id: &str, name: &str) -> GroupTab {
    GroupTab {
        id: id.into(),
        name: name.into(),
    }
}

fn ts(hm: &str) -> String {
    format!("2026-10-07T{hm}:00.000Z")
}

#[allow(clippy::too_many_arguments)]
fn approval(
    group: &str,
    id: &str,
    bot: (&str, &str),
    owner: (&str, &str),
    decision: Option<ApprovalDecision>,
    answered: Option<&str>,
    expired: bool,
) -> GroupEvent {
    GroupEvent {
        group_id: group.into(),
        event: Event::Approval {
            id: id.into(),
            bot_id: bot.0.into(),
            bot_name: bot.1.into(),
            action: ApprovalAction::Delete,
            summary: "清理 12 个旧日志文件".into(),
            path: Some("logs/2026-10/".into()),
            decision,
            created_at: ts("18:50"),
            answered_at: answered.map(ts),
            answered_by: decision.map(|_| owner.1.to_string()),
            task_id: None,
            mcp_server: None,
            mcp_tool: None,
            reason: expired.then(|| robroz_contract::APPROVAL_EXPIRED_REASON.to_string()),
            expires_at: None,
            owner_user_id: Some(owner.0.into()),
            owner_name: Some(owner.1.into()),
        },
        approval_extras: None,
    }
}

fn say(group: &str, id: &str, body: &str) -> GroupEvent {
    GroupEvent {
        group_id: group.into(),
        event: Event::Message {
            id: id.into(),
            kind: MessageKind::Text,
            body: body.into(),
            created_at: ts("18:49"),
            author_user_id: Some(FAKE_ME_ID.into()),
            author_name: Some(FAKE_ME_NAME.into()),
        },
        approval_extras: None,
    }
}

fn reminder(
    id: &str,
    group: (&str, &str),
    state: ReminderState,
    answered: Option<&str>,
) -> GroupEvent {
    GroupEvent {
        group_id: DM.into(),
        event: Event::ApprovalReminder {
            id: format!("reminder:{id}"),
            approval_id: id.into(),
            group_id: group.0.into(),
            group_name: group.1.into(),
            bot_id: "bot-ada".into(),
            bot_name: "Ada".into(),
            owner_user_id: FAKE_ME_ID.into(),
            owner_name: FAKE_ME_NAME.into(),
            state,
            created_at: ts("18:50"),
            answered_at: answered.map(ts),
        },
        approval_extras: None,
    }
}

pub fn approvals_source() -> FakeSource {
    let ada = ("bot-ada", "Ada");
    let bea = ("bot-bea", "Bea");
    let me = (FAKE_ME_ID, FAKE_ME_NAME);
    let lin = (LIN_ID, LIN_NAME);
    let groups = vec![
        tab(OWN_WAITING, "官网上线"),
        tab(OWN_DONE, "周报发布"),
        tab(OTHER_WAITING, "数据迁移"),
        tab(OTHER_DONE, "日志清理"),
        tab(EXPIRED, "夜间构建"),
        tab(DM, "Ada"),
    ];
    let mut step = vec![
        say(OWN_WAITING, "m1", "把官网的旧日志清一下。"),
        approval(OWN_WAITING, "a-own", ada, me, None, None, false),
        say(OWN_DONE, "m2", "周报发出去前先清掉旧日志。"),
        approval(
            OWN_DONE,
            "a-own-done",
            ada,
            me,
            Some(ApprovalDecision::Approved),
            Some("19:02"),
            false,
        ),
        say(OTHER_WAITING, "m3", "迁移前先清一下旧日志。"),
        approval(OTHER_WAITING, "a-other", bea, lin, None, None, false),
        say(OTHER_DONE, "m4", "先清旧日志再备份。"),
        approval(
            OTHER_DONE,
            "a-other-done",
            bea,
            lin,
            Some(ApprovalDecision::Approved),
            Some("19:02"),
            false,
        ),
        say(EXPIRED, "m5", "夜里构建前清一下日志。"),
        approval(
            EXPIRED,
            "a-expired",
            ada,
            me,
            Some(ApprovalDecision::Denied),
            Some("19:30"),
            true,
        ),
        say(DM, "m-dm", "你好 Ada，在群里需要我批准的会在这里提醒我。"),
        reminder(
            "a-own-done",
            (OWN_DONE, "周报发布"),
            ReminderState::Approved,
            Some("19:02"),
        ),
        reminder(
            "a-own",
            (OWN_WAITING, "官网上线"),
            ReminderState::Waiting,
            None,
        ),
    ];
    // `ROBROZ_LONG_FEED=1`: many later messages in the owner-waiting
    // group, so a jump from the DM has to scroll up to the card.
    if std::env::var("ROBROZ_LONG_FEED").is_ok() {
        for i in 0..40 {
            let mm = 10 + (i % 50);
            step.push(say_at(
                OWN_WAITING,
                &format!("pad-{i}"),
                &format!("后面还有一条 {i}。"),
                ts(&format!("19:{mm:02}")),
            ));
        }
    }
    FakeSource::from_parts(groups, vec![step], Vec::new())
}

fn say_at(group: &str, id: &str, body: &str, created: String) -> GroupEvent {
    GroupEvent {
        group_id: group.into(),
        event: Event::Message {
            id: id.into(),
            kind: MessageKind::Text,
            body: body.into(),
            created_at: created,
            author_user_id: Some(FAKE_ME_ID.into()),
            author_name: Some(FAKE_ME_NAME.into()),
        },
        approval_extras: None,
    }
}

/// Unread counts and bros for the scene.
pub fn approvals_chats() -> (Vec<Unread>, Vec<Bro>) {
    let unread = |g: &str, n: u64, hm: &str| Unread {
        group_id: g.into(),
        read_at: None,
        unread: n,
        last_activity_at: Some(ts(hm)),
    };
    (
        vec![
            unread(DM, 1, "19:05"),
            unread(OWN_WAITING, 1, "18:50"),
            unread(OWN_DONE, 0, "19:02"),
            unread(OTHER_WAITING, 0, "18:50"),
            unread(OTHER_DONE, 0, "19:02"),
            unread(EXPIRED, 0, "19:30"),
        ],
        vec![Bro {
            id: "bot-ada".into(),
            name: "Ada".into(),
            dm_group_id: Some(DM.into()),
            owner_user_id: Some(FAKE_ME_ID.into()),
        }],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Feed, Row};
    use robroz_ui::cards::{ApprovalState, Card};
    use std::collections::HashMap;

    fn feeds() -> HashMap<String, Feed> {
        let mut source = approvals_source();
        let mut feeds: HashMap<String, Feed> = HashMap::new();
        use crate::source::EventSource;
        while source.has_more() {
            for ge in source.poll().unwrap() {
                feeds.entry(ge.group_id).or_default().apply(&ge.event);
            }
        }
        feeds
    }

    fn approval_of(feed: &Feed) -> robroz_ui::cards::ApprovalCard {
        feed.rows()
            .find_map(|r| match r {
                Row::Card(Card::Approval(c)) => Some(c.clone()),
                _ => None,
            })
            .expect("an approval card")
    }

    #[test]
    fn the_scene_has_every_state_once() {
        let f = feeds();
        assert_eq!(approval_of(&f[OWN_WAITING]).state, ApprovalState::Pending);
        assert_eq!(approval_of(&f[OTHER_WAITING]).state, ApprovalState::Pending);
        assert!(matches!(
            approval_of(&f[OWN_DONE]).state,
            ApprovalState::Approved { .. }
        ));
        assert!(matches!(
            approval_of(&f[OTHER_DONE]).state,
            ApprovalState::Approved { .. }
        ));
        assert!(matches!(
            approval_of(&f[EXPIRED]).state,
            ApprovalState::Expired { .. }
        ));
        assert_eq!(
            approval_of(&f[OWN_WAITING]).owner_id.as_deref(),
            Some(FAKE_ME_ID)
        );
        assert_eq!(
            approval_of(&f[OTHER_WAITING]).owner_id.as_deref(),
            Some(LIN_ID)
        );
    }

    #[test]
    fn the_dm_has_a_waiting_and_an_approved_reminder_and_no_card() {
        let f = feeds();
        let rows: Vec<_> = f[DM].rows().collect();
        let reminders: Vec<_> = rows
            .iter()
            .filter_map(|r| match r {
                Row::Reminder(r) => Some(r.state),
                _ => None,
            })
            .collect();
        assert_eq!(reminders, [ReminderState::Waiting, ReminderState::Approved]);
        assert!(!rows.iter().any(|r| matches!(r, Row::Card(_))));
    }
}
