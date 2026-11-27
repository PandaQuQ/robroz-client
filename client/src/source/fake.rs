//! Scripted events for offline runs (`--fake`) and unit tests. No network.

use std::time::Duration;

use robroz_contract::{
    Bro, ChoiceOption, Event, MessageKind, Schedule, ScheduleEntry, TaskState, Unread,
};
use robroz_ui::cards::{
    ActionKind, ApprovalCard, ApprovalState, Card, DraftCard, DraftChannel, DraftState,
};

use super::{EventSource, GroupCard, GroupEvent, GroupTab, SourceError};

/// Scripted events, released one step per `poll`. No network.
pub struct FakeSource {
    groups: Vec<GroupTab>,
    steps: Vec<Vec<GroupEvent>>,
    /// Approval/draft cards released with step `.0`.
    cards: Vec<(usize, GroupCard)>,
    next: usize,
}

const SITE: &str = "g-site";
const OPS: &str = "g-ops";
/// Two bros delivering at the same time, plus one unattributed delivery.
const TWO: &str = "g-two";
/// A bro's private chat: a group whose only member is that bro.
const DM: &str = "dm-ada";

impl FakeSource {
    pub fn new() -> Self {
        let groups = vec![
            group(SITE, "公司官网建设"),
            group(OPS, "运营日报"),
            group(TWO, "同时交付"),
            group(DM, "Ada"),
        ];
        let steps = vec![
            vec![
                msg(
                    SITE,
                    "m1",
                    MessageKind::Text,
                    "首页 hero 换成大透明 logo，手机上也要清楚。",
                ),
                msg(SITE, "m2", MessageKind::Image, "hero-draft.png"),
                status(SITE, "t-hero", "bot-ada", "Ada", TaskState::Queued, false),
                msg(
                    OPS,
                    "m3",
                    MessageKind::Text,
                    "每天早上 8 点出一份询价简报。",
                ),
                status(OPS, "t-crawl", "bot-cy", "Cy", TaskState::Queued, false),
                msg(
                    DM,
                    "m-dm1",
                    MessageKind::Text,
                    "你好 Ada，以后有事我直接在这里找你。",
                ),
            ],
            vec![status(
                SITE,
                "t-hero",
                "bot-ada",
                "Ada",
                TaskState::Running,
                true,
            )],
            vec![
                msg(SITE, "m4", MessageKind::File, "brand-guide.pdf"),
                status(SITE, "t-copy", "bot-bea", "Bea", TaskState::Queued, false),
            ],
            vec![status(
                OPS,
                "t-crawl",
                "bot-cy",
                "Cy",
                TaskState::Running,
                true,
            )],
            vec![status(
                SITE,
                "t-copy",
                "bot-bea",
                "Bea",
                TaskState::Blocked,
                false,
            )],
            vec![
                status(SITE, "t-hero", "bot-ada", "Ada", TaskState::Done, false),
                attributed_delivery(
                    SITE,
                    "d1",
                    "已写入结果",
                    "tasks/t-hero/result.txt",
                    false,
                    Some("t-hero"),
                    Some("bot-ada"),
                    Some("Ada"),
                ),
            ],
            vec![status(
                OPS,
                "t-crawl",
                "bot-cy",
                "Cy",
                TaskState::Running,
                false,
            )],
            // Node 5: a schedule fires a normal task. Its card is a normal
            // status card and its result is a normal delivery with
            // `briefing: true`. No separate card kind.
            vec![status(
                OPS,
                "t-brief",
                "bot-dee",
                "Dee",
                TaskState::Queued,
                false,
            )],
            vec![status(
                OPS,
                "t-brief",
                "bot-dee",
                "Dee",
                TaskState::Running,
                true,
            )],
            vec![
                status(OPS, "t-brief", "bot-dee", "Dee", TaskState::Done, false),
                attributed_delivery(
                    OPS,
                    "d2",
                    "已写入结果",
                    "tasks/t-brief/result.txt",
                    true,
                    Some("t-brief"),
                    Some("bot-dee"),
                    Some("Dee"),
                ),
            ],
            // Two bros deliver at the same instant, then a delivery with
            // no bot_name, so attribution can be checked at a glance.
            vec![
                msg(TWO, "m-two", MessageKind::Text, "Ada 和 Bea 同时交结果。"),
                attributed_delivery(
                    TWO,
                    "d-ada",
                    "已写入结果",
                    "tasks/t-ada/result.txt",
                    false,
                    Some("t-ada"),
                    Some("bot-ada"),
                    Some("Ada"),
                ),
                attributed_delivery(
                    TWO,
                    "d-bea",
                    "已写入结果",
                    "tasks/t-bea/result.txt",
                    false,
                    Some("t-bea"),
                    Some("bot-bea"),
                    Some("Bea"),
                ),
                delivery(
                    TWO,
                    "d-orphan",
                    "已写入结果",
                    "tasks/t-orphan/result.txt",
                    false,
                ),
            ],
            // Phase-1 cards: a choice (contract event from robroz#15) plus
            // an approval and a draft (client-side until the backend adds
            // them). Clicking them in `--fake` answers locally.
            vec![
                choice(SITE, "c-install", false),
                choice(SITE, "c-pages", true),
                // Six minutes later: a new group of the person's own.
                msg_at(
                    SITE,
                    "m5",
                    MessageKind::Text,
                    "收到。CI 变绿就发。",
                    "19:06:00",
                ),
                // A person's message and a bro's approval in the DM.
                msg_at(
                    DM,
                    "m-dm2",
                    MessageKind::Text,
                    "帮我把旧日志清一下。",
                    "19:00:40",
                ),
            ],
        ];
        let last = steps.len() - 1;
        let cards = vec![
            (
                last,
                GroupCard {
                    group_id: SITE.into(),
                    card: Card::Approval(ApprovalCard {
                        id: "a-logs".into(),
                        bot: "后端".into(),
                        kind: ActionKind::Delete,
                        summary: "清理 12 个旧日志文件".into(),
                        path: Some("logs/2026-10/".into()),
                        state: ApprovalState::Pending,
                        mcp_server: None,
                        mcp_tool: None,
                        reason: None,
                        reason_code: None,
                        owner_id: Some(crate::api::FAKE_ME_ID.into()),
                        owner_name: Some(crate::api::FAKE_ME_NAME.into()),
                    }),
                },
            ),
            (
                last,
                GroupCard {
                    group_id: DM.into(),
                    card: Card::Approval(ApprovalCard {
                        id: "a-dm-logs".into(),
                        bot: "Ada".into(),
                        kind: ActionKind::Delete,
                        summary: "清理 12 个旧日志文件".into(),
                        path: Some("logs/2026-10/".into()),
                        state: ApprovalState::Pending,
                        mcp_server: None,
                        mcp_tool: None,
                        reason: None,
                        reason_code: None,
                        owner_id: Some(crate::api::FAKE_ME_ID.into()),
                        owner_name: Some(crate::api::FAKE_ME_NAME.into()),
                    }),
                },
            ),
            (
                last,
                GroupCard {
                    group_id: SITE.into(),
                    card: Card::Draft(DraftCard {
                        id: "d-progress".into(),
                        bot: "文档哥".into(),
                        channel: DraftChannel::Email,
                        to: "team@example.com".into(),
                        subject: Some("robroz 第一期进度".into()),
                        body: DRAFT_BODY.into(),
                        state: DraftState::Pending,
                    }),
                },
            ),
        ];
        Self {
            groups,
            steps,
            cards,
            next: 0,
        }
    }
}

impl FakeSource {
    /// A scripted run from parts; used by the approval scene.
    pub(super) fn from_parts(
        groups: Vec<GroupTab>,
        steps: Vec<Vec<GroupEvent>>,
        cards: Vec<(usize, GroupCard)>,
    ) -> Self {
        Self {
            groups,
            steps,
            cards,
            next: 0,
        }
    }
}

impl Default for FakeSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSource for FakeSource {
    fn groups(&mut self) -> Result<Vec<GroupTab>, SourceError> {
        Ok(self.groups.clone())
    }

    fn poll(&mut self) -> Result<Vec<GroupEvent>, SourceError> {
        Ok(match self.steps.get(self.next) {
            Some(step) => {
                self.next += 1;
                step.clone()
            }
            None => Vec::new(),
        })
    }

    fn has_more(&self) -> bool {
        self.next < self.steps.len()
    }

    fn poll_cards(&mut self) -> Vec<GroupCard> {
        let Some(released) = self.next.checked_sub(1) else {
            return Vec::new();
        };
        let out = self
            .cards
            .iter()
            .filter(|(step, _)| *step == released)
            .map(|(_, c)| c.clone())
            .collect();
        // Release each card once.
        self.cards.retain(|(step, _)| *step != released);
        out
    }

    fn interval(&self) -> Duration {
        Duration::from_millis(900)
    }
}

fn group(id: &str, name: &str) -> GroupTab {
    GroupTab {
        id: id.into(),
        name: name.into(),
    }
}

fn at(seq: &str) -> String {
    format!("2026-10-07T19:00:{seq}.000Z")
}

pub const DRAFT_BODY: &str = "大家好，第一期的 7 个节点里已经合并了 5 个：任务队列、Docker 沙箱、三级权限、定时任务和客户端外壳。剩下的是联调和真机测试，预计下周开始。";

fn option(id: &str, label: &str, primary: bool) -> ChoiceOption {
    ChoiceOption {
        id: id.into(),
        label: label.into(),
        primary,
    }
}

fn choice(group: &str, id: &str, multi: bool) -> GroupEvent {
    let (bot_id, bot_name, prompt, options) = if multi {
        (
            "bot-fe",
            "前端",
            "第一期客户端先做哪几页？",
            vec![
                option("chat", "群聊", false),
                option("routines", "例行任务", false),
                option("settings", "设置", false),
                option("files", "文件", false),
            ],
        )
    } else {
        (
            "bot-doc",
            "文档哥",
            "安装说明先写哪一种？",
            vec![
                option("docker", "Docker 一键", true),
                option("cargo", "cargo install", false),
                option("both", "两种都写", false),
            ],
        )
    };
    GroupEvent {
        group_id: group.into(),
        event: Event::Choice {
            id: id.into(),
            bot_id: bot_id.into(),
            bot_name: bot_name.into(),
            prompt: prompt.into(),
            options,
            multi,
            choice: None,
            created_at: at("04"),
            answered_at: None,
            answered_by: None,
        },
        approval_extras: None,
    }
}

/// Schedules for the offline API, one per fake group plus a paused one.
/// Unread counts and bros for the fake chats (`--fake`).
pub fn fake_chats() -> (Vec<Unread>, Vec<Bro>) {
    let unread = |g: &str, n: u64, sec: &str| Unread {
        group_id: g.into(),
        read_at: None,
        unread: n,
        last_activity_at: Some(at(sec)),
    };
    (
        vec![
            unread(SITE, 3, "40"),
            unread(DM, 1, "30"),
            unread(OPS, 0, "20"),
            unread(TWO, 0, "03"),
        ],
        vec![Bro {
            avatar: None,
            id: "bot-ada".into(),
            name: "Ada".into(),
            dm_group_id: Some(DM.into()),
            owner_user_id: None,
        }],
    )
}

pub fn fake_schedules() -> Vec<ScheduleEntry> {
    let now = chrono::Utc::now();
    let entry = |id: &str, group: &str, prompt: &str, hours: i64, paused: bool| ScheduleEntry {
        schedule: Schedule {
            id: id.into(),
            group_id: group.into(),
            assignee_bot_id: "bot-dee".into(),
            cron: "0 8 * * *".into(),
            prompt: prompt.into(),
            next_run_at: (now + chrono::Duration::hours(hours))
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            last_task_id: None,
        },
        paused,
    };
    vec![
        entry("s-brief", OPS, "每日简报", 30, false),
        entry("s-deps", SITE, "依赖安全检查", 80, false),
        entry("s-weekly", OPS, "周报草稿", 50, true),
        entry("s-clean", SITE, "清理沙箱缓存", 3, false),
    ]
}

fn msg(group: &str, id: &str, kind: MessageKind, body: &str) -> GroupEvent {
    msg_at(group, id, kind, body, "19:00:00")
}

/// A person's message sent at `hms` on the fake day (UTC).
fn msg_at(group: &str, id: &str, kind: MessageKind, body: &str, hms: &str) -> GroupEvent {
    GroupEvent {
        group_id: group.into(),
        event: Event::Message {
            id: id.into(),
            kind,
            body: body.into(),
            created_at: format!("2026-10-07T{hms}.000Z"),
            author_name: None,
            author_user_id: None,
        },
        approval_extras: None,
    }
}

fn status(
    group: &str,
    task: &str,
    bot_id: &str,
    bot_name: &str,
    state: TaskState,
    container_up: bool,
) -> GroupEvent {
    GroupEvent {
        group_id: group.into(),
        event: Event::Status {
            progress_code: None,
            progress_params: None,
            id: task.into(),
            task_id: task.into(),
            bot_id: bot_id.into(),
            bot_name: bot_name.into(),
            state,
            label: state.label().into(),
            created_at: at("01"),
            updated_at: at("02"),
            container_up,
            failure: None,
            progress: None,
            waiting_for: None,
            status_code: None,
            failure_code: None,
            failure_detail: None,
        },
        approval_extras: None,
    }
}

fn delivery(group: &str, id: &str, summary: &str, path: &str, briefing: bool) -> GroupEvent {
    attributed_delivery(group, id, summary, path, briefing, None, None, None)
}

fn attributed_delivery(
    group: &str,
    id: &str,
    summary: &str,
    path: &str,
    briefing: bool,
    task_id: Option<&str>,
    bot_id: Option<&str>,
    bot_name: Option<&str>,
) -> GroupEvent {
    GroupEvent {
        group_id: group.into(),
        event: Event::Delivery {
            id: id.into(),
            summary: summary.into(),
            path: path.into(),
            briefing,
            created_at: at("03"),
            failure: None,
            needs_setup: None,
            task_id: task_id.map(str::to_string),
            bot_id: bot_id.map(str::to_string),
            bot_name: bot_name.map(str::to_string),
            badge: None,
            failure_code: None,
            failure_detail: None,
        },
        approval_extras: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{group_items, Feed, Row};
    use std::collections::HashMap;

    fn drain() -> HashMap<String, Feed> {
        let mut source = FakeSource::new();
        let mut feeds: HashMap<String, Feed> = HashMap::new();
        while source.has_more() {
            for ge in source.poll().unwrap() {
                feeds.entry(ge.group_id).or_default().apply(&ge.event);
            }
            for gc in source.poll_cards() {
                feeds.entry(gc.group_id).or_default().apply_card(gc.card);
            }
        }
        assert!(source.poll().unwrap().is_empty());
        feeds
    }

    #[test]
    fn fake_script_covers_every_case() {
        let mut source = FakeSource::new();
        assert_eq!(source.groups().unwrap().len(), 4);
        let feeds = drain();
        let site: Vec<_> = feeds[SITE].rows().cloned().collect();
        let ops: Vec<_> = feeds[OPS].rows().cloned().collect();

        let status_of = |rows: &[Row], bot: &str| {
            rows.iter()
                .find_map(|r| match r {
                    Row::Status(c) if c.bot_name == bot => Some(c.clone()),
                    _ => None,
                })
                .unwrap()
        };
        let count_status = |rows: &[Row], bot: &str| {
            rows.iter()
                .filter(|r| matches!(r, Row::Status(c) if c.bot_name == bot))
                .count()
        };

        assert_eq!(count_status(&site, "Ada"), 1);
        assert_eq!(
            status_of(&site, "Ada").status_line,
            robroz_card::status_line(TaskState::Done, false)
        );
        assert_eq!(
            status_of(&site, "Bea").action,
            robroz_card::appearance(TaskState::Blocked, false).action
        );
        assert_eq!(
            status_of(&ops, "Cy").status_line,
            robroz_card::status_line(TaskState::Running, false)
        );
        assert!(site
            .iter()
            .any(|r| matches!(r, Row::Delivery(d) if d.badge.is_none())));
        assert!(ops.iter().any(
            |r| matches!(r, Row::Delivery(d) if d.badge == Some(robroz_card::briefing_badge()))
        ));
        assert_eq!(count_status(&ops, "Dee"), 1);
        assert_eq!(
            status_of(&ops, "Dee").status_line,
            robroz_card::status_line(TaskState::Done, false)
        );
        assert!(site.iter().any(|r| matches!(
            r,
            Row::Attachment {
                kind: MessageKind::Image,
                ..
            }
        )));
        assert!(site.iter().any(|r| matches!(
            r,
            Row::Attachment {
                kind: MessageKind::File,
                ..
            }
        )));
        let site_feed = &feeds[SITE];
        for id in ["c-install", "c-pages", "a-logs", "d-progress"] {
            assert!(site_feed.card(id).is_some(), "fake script has card {id}");
        }

        let two_feed = &feeds[TWO];
        let items: Vec<_> = two_feed.items().collect();
        let groups = group_items(items.iter().map(|(r, m)| (*r, (*m).clone())), 120);
        assert_eq!(groups.len(), 4);
        assert!(groups[0].sender.is_own());
        assert_eq!(groups[1].sender.name(), "Ada");
        assert_eq!(groups[2].sender.name(), "Bea");
        assert!(groups[3].sender.is_delivery());
    }

    #[test]
    fn fake_schedules_cover_running_and_paused() {
        let all = fake_schedules();
        assert!(all.iter().any(|s| s.paused));
        assert!(all.iter().any(|s| !s.paused));
    }
}
