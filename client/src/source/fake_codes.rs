//! `ROBROZ_FAKE_SCENE=status-codes`: failed status and delivery cards
//! driven by v0.2 codes, for review screenshots.

use robroz_contract::{
    Bro, CardStatusCode, DeliveryBadge, Event, FailureCode, MessageKind, TaskState, Unread,
};

use super::{FakeSource, GroupEvent, GroupTab};
use crate::api::{FAKE_ME_ID, FAKE_ME_NAME};

pub const GROUP: &str = "g-fail";

fn tab(id: &str, name: &str) -> GroupTab {
    GroupTab::new(id, name)
}

fn ts(hm: &str) -> String {
    format!("2026-10-07T{hm}:00.000Z")
}

fn say(id: &str, body: &str, hm: &str) -> GroupEvent {
    GroupEvent {
        group_id: GROUP.into(),
        event: Event::Message {
            id: id.into(),
            kind: MessageKind::Text,
            body: body.into(),
            created_at: ts(hm),
            author_user_id: Some(FAKE_ME_ID.into()),
            author_name: Some(FAKE_ME_NAME.into()),
        },
        approval_extras: None,
    }
}

fn failed(id: &str, code: FailureCode, detail: Option<&str>, legacy: &str, hm: &str) -> GroupEvent {
    GroupEvent {
        group_id: GROUP.into(),
        event: Event::Status {
            progress_code: None,
            progress_params: None,
            id: id.into(),
            task_id: id.into(),
            bot_id: "bot-ada".into(),
            bot_name: "Ada".into(),
            state: TaskState::Done,
            label: TaskState::Done.label().into(),
            created_at: ts(hm),
            updated_at: ts(hm),
            container_up: false,
            failure: Some(legacy.into()),
            progress: None,
            waiting_for: None,
            status_code: Some(CardStatusCode::Done),
            failure_code: Some(code),
            failure_detail: detail.map(str::to_string),
        },
        approval_extras: None,
    }
}

fn legacy_failed(id: &str, legacy: &str, hm: &str) -> GroupEvent {
    GroupEvent {
        group_id: GROUP.into(),
        event: Event::Status {
            progress_code: None,
            progress_params: None,
            id: id.into(),
            task_id: id.into(),
            bot_id: "bot-ada".into(),
            bot_name: "Ada".into(),
            state: TaskState::Done,
            label: TaskState::Done.label().into(),
            created_at: ts(hm),
            updated_at: ts(hm),
            container_up: false,
            failure: Some(legacy.into()),
            progress: None,
            waiting_for: None,
            status_code: None,
            failure_code: None,
            failure_detail: None,
        },
        approval_extras: None,
    }
}

fn delivery_fail(
    id: &str,
    code: FailureCode,
    detail: Option<&str>,
    legacy: &str,
    hm: &str,
) -> GroupEvent {
    GroupEvent {
        group_id: GROUP.into(),
        event: Event::Delivery {
            id: id.into(),
            summary: "周报".into(),
            path: "tasks/t/weekly.md".into(),
            briefing: false,
            created_at: ts(hm),
            failure: Some(legacy.into()),
            needs_setup: None,
            task_id: Some(id.into()),
            bot_id: Some("bot-ada".into()),
            bot_name: Some("Ada".into()),
            badge: None,
            failure_code: Some(code),
            failure_detail: detail.map(str::to_string),
        },
        approval_extras: None,
    }
}

fn briefing(id: &str, hm: &str) -> GroupEvent {
    GroupEvent {
        group_id: GROUP.into(),
        event: Event::Delivery {
            id: id.into(),
            summary: "今日简报".into(),
            path: "tasks/t/briefing.md".into(),
            briefing: true,
            created_at: ts(hm),
            failure: None,
            needs_setup: None,
            task_id: Some(id.into()),
            bot_id: Some("bot-ada".into()),
            bot_name: Some("Ada".into()),
            badge: Some(DeliveryBadge::Briefing),
            failure_code: None,
            failure_detail: None,
        },
        approval_extras: None,
    }
}

pub fn codes_source() -> FakeSource {
    let groups = vec![tab(GROUP, "失败卡")];
    let step = vec![
        say("m1", "看一下这几种失败是怎么写的。", "18:40"),
        failed(
            "f-model",
            FailureCode::ModelNoReply,
            Some("401"),
            "模型没回：模型服务返回 401",
            "18:41",
        ),
        failed(
            "f-rounds",
            FailureCode::ToolRoundLimit,
            None,
            "工具调用超过 8 轮",
            "18:42",
        ),
        failed("f-none", FailureCode::NoResult, None, "没写出结果", "18:43"),
        failed(
            "f-quota",
            FailureCode::OutboundOverQuota,
            Some("3/2"),
            "出网次数已达上限：已用 3 次，上限 2",
            "18:44",
        ),
        failed(
            "f-denied",
            FailureCode::Denied,
            Some("mail.send"),
            "你拒绝了：发送周报",
            "18:45",
        ),
        legacy_failed("f-legacy", "模型服务返回 500", "18:46"),
        delivery_fail(
            "d-tool",
            FailureCode::ToolCallFailed,
            Some("connection refused"),
            "调用失败：模型服务返回 401",
            "18:47",
        ),
        briefing("d-brief", "18:48"),
    ];
    FakeSource::from_parts(groups, vec![step], Vec::new())
}

pub fn codes_chats() -> (Vec<Unread>, Vec<Bro>) {
    (
        vec![Unread {
            group_id: GROUP.into(),
            read_at: None,
            unread: 0,
            last_activity_at: Some(ts("18:48")),
        }],
        vec![Bro {
            avatar: None,
            id: "bot-ada".into(),
            name: "Ada".into(),
            dm_group_id: None,
            owner_user_id: Some(FAKE_ME_ID.into()),
        }],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{preview, Feed, Row};
    use crate::source::EventSource;
    use robroz_ui::i18n::{scoped, Locale};

    #[test]
    fn the_scene_covers_every_prefix_and_language_case() {
        let mut src = codes_source();
        let mut feed = Feed::default();
        for ge in src.poll().unwrap() {
            if ge.group_id == GROUP {
                feed.apply(&ge.event);
            }
        }
        let failures: Vec<_> = feed
            .rows()
            .filter_map(|r| match r {
                Row::Status(c) => Some(c.failure_code),
                Row::Delivery(c) => Some(c.failure_code),
                _ => None,
            })
            .collect();
        assert!(failures.contains(&Some(FailureCode::ModelNoReply)));
        assert!(failures.contains(&Some(FailureCode::ToolRoundLimit)));
        assert!(failures.contains(&Some(FailureCode::NoResult)));
        assert!(failures.contains(&Some(FailureCode::OutboundOverQuota)));
        assert!(failures.contains(&None)); // legacy card
        let ev = {
            let mut src = codes_source();
            src.poll()
                .unwrap()
                .into_iter()
                .find(|g| matches!(g.event, Event::Status { ref task_id, .. } if task_id == "f-model"))
                .unwrap()
                .event
        };
        {
            let _g = scoped(Locale::Zh);
            assert_eq!(preview(&ev), "Ada：没做成：模型没回（HTTP 401）");
            assert!(!preview(&ev).contains("模型服务返回"));
        }
        let _g = scoped(Locale::En);
        assert_eq!(
            preview(&ev),
            "Ada: Couldn't finish: the model didn't reply (HTTP 401)"
        );
    }
}
