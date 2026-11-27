//! One conversation as chat rows: text bubbles, status cards, and
//! read-only approval rows.
//!
//! Own messages are bubbles on the right, a bro's messages bubbles on the
//! left, grouped into runs: the name once above the first item in a group,
//! the avatar only at the last. A task's status card sits on the left
//! under the same rules. A card is keyed by task: later status events
//! update the card where it first appeared. An approval event becomes a
//! read-only row (no buttons on the phone in v0.2), keyed by approval
//! id, and later events for the same id update it in place.

use robroz_contract::Event;
use robroz_ui::cards::status as status_ui;
use robroz_ui::cards::ApprovalCard;
use robroz_ui::conversations;
use robroz_ui::runs::{self, Block, Meta, RunSender, GROUP_SECS};

use crate::card::StatusCard;

/// An approval event in the feed, with the bot id and created time used
/// to place it in a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalItem {
    pub card: ApprovalCard,
    pub bot_id: String,
    pub at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
    Own,
    Bro { id: String, name: String },
}

impl RunSender for Side {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Side::Own, Side::Own) => true,
            (Side::Bro { id: a, name: x }, Side::Bro { id: b, name: y }) => {
                if a.is_empty() || b.is_empty() {
                    x == y
                } else {
                    a == b
                }
            }
            _ => false,
        }
    }
}

impl Side {
    pub fn is_own(&self) -> bool {
        matches!(self, Side::Own)
    }

    pub fn name(&self) -> &str {
        match self {
            Side::Bro { name, .. } => name,
            Side::Own => "",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatItem {
    Text {
        body: String,
        side: Side,
        at: Option<i64>,
    },
    Status(StatusCard),
    Approval(ApprovalItem),
}

/// One row in the feed, after run grouping.
#[derive(Debug, Clone, PartialEq)]
pub struct FeedRow {
    pub item: ChatItem,
    pub show_name: bool,
    pub show_avatar: bool,
    pub pad_top: f32,
    /// Clock under the last item of a run; empty on earlier items.
    pub time: String,
}

/// A date separator or a message/status row.
#[derive(Debug, Clone, PartialEq)]
pub enum FeedEntry {
    Date { label: String, pad_top: f32 },
    Row(FeedRow),
}

/// `events` is oldest first, each with whether a message is the user's own
/// (see `Conn::events`). A status event replaces the earlier card of the
/// same task in place; an approval event does the same by approval id.
pub fn build(events: &[(Event, bool)]) -> Vec<ChatItem> {
    build_from(events)
}

pub fn build_from(events: &[(Event, bool)]) -> Vec<ChatItem> {
    let mut items: Vec<ChatItem> = Vec::new();
    let mut card_at: Vec<(String, usize)> = Vec::new();
    let mut approval_at: Vec<(String, usize)> = Vec::new();
    for (event, own) in events {
        match event {
            Event::Message {
                body, created_at, ..
            } => {
                let at = runs::parse_unix(created_at);
                let side = if *own {
                    Side::Own
                } else {
                    Side::Bro {
                        id: String::new(),
                        name: String::new(),
                    }
                };
                items.push(ChatItem::Text {
                    body: body.clone(),
                    side,
                    at,
                });
            }
            Event::Status { task_id, .. } => {
                let Some(mut card) = StatusCard::from_event(event) else {
                    continue;
                };
                match card_at.iter().find(|(t, _)| t == task_id) {
                    Some((_, at)) => {
                        let at = *at;
                        if let ChatItem::Status(old) = &items[at] {
                            card = merge_status(old, card);
                        }
                        items[at] = ChatItem::Status(card);
                    }
                    None => {
                        card_at.push((task_id.clone(), items.len()));
                        items.push(ChatItem::Status(card));
                    }
                }
            }
            Event::Approval {
                id,
                bot_id,
                created_at,
                ..
            } => {
                let Some(card) = ApprovalCard::from_event(event) else {
                    continue;
                };
                let item = ChatItem::Approval(ApprovalItem {
                    card,
                    bot_id: bot_id.clone(),
                    at: runs::parse_unix(created_at),
                });
                match approval_at.iter().find(|(t, _)| t == id) {
                    Some((_, at)) => {
                        items[*at] = item;
                    }
                    None => {
                        approval_at.push((id.clone(), items.len()));
                        items.push(item);
                    }
                }
            }
            _ => {}
        }
    }
    items
}

/// Keep steps, the local timer, and expand state across a status update.
fn merge_status(old: &StatusCard, mut card: StatusCard) -> StatusCard {
    card.steps = old.steps.clone();
    card.local_started = old.local_started;
    card.expanded = old.expanded;
    if card.is_live() && !old.is_live() && card.local_started.is_none() {
        card.local_started = Some(robroz_ui::process::now_unix());
    }
    if let Some(line) = card.progress.clone().or(card.step_progress.clone()) {
        robroz_ui::process::push_step(&mut card.steps, line);
    }
    card
}

/// Live-running bros in this chat, first-seen order, unique by bot id.
pub fn working_bros(items: &[ChatItem]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for item in items {
        if let ChatItem::Status(card) = item {
            if card.is_live() && !out.iter().any(|(id, _)| id == &card.bot_id) {
                out.push((card.bot_id.clone(), card.bot_name.clone()));
            }
        }
    }
    out
}

/// Carry expand/timer state from a previous build of the same chat.
pub fn carry_process(old: &[ChatItem], new: &mut [ChatItem]) {
    for item in new.iter_mut() {
        let ChatItem::Status(card) = item else {
            continue;
        };
        let Some(ChatItem::Status(prev)) = old
            .iter()
            .find(|o| matches!(o, ChatItem::Status(c) if c.task_id == card.task_id))
        else {
            continue;
        };
        *card = merge_status(prev, card.clone());
    }
}

/// The name shown over a bro's bubbles: the bro on the newest status card,
/// else the conversation's own name.
pub fn bro_name<'a>(items: &'a [ChatItem], fallback: &'a str) -> &'a str {
    items
        .iter()
        .rev()
        .find_map(|i| match i {
            ChatItem::Status(c) => Some(c.bot_name.as_str()),
            ChatItem::Approval(a) => Some(a.card.bot.as_str()),
            ChatItem::Text {
                side: Side::Bro { name, .. },
                ..
            } if !name.is_empty() => Some(name.as_str()),
            _ => None,
        })
        .unwrap_or(fallback)
}

fn item_meta(item: &ChatItem, fallback: &str) -> (Side, Option<i64>) {
    match item {
        ChatItem::Text { side, at, .. } => {
            let side = match side {
                Side::Bro { id, name } if name.is_empty() => Side::Bro {
                    id: id.clone(),
                    name: fallback.to_string(),
                },
                other => other.clone(),
            };
            (side, *at)
        }
        ChatItem::Status(c) => (
            Side::Bro {
                id: c.bot_id.clone(),
                name: c.bot_name.clone(),
            },
            c.at,
        ),
        ChatItem::Approval(a) => (
            Side::Bro {
                id: a.bot_id.clone(),
                name: a.card.bot.clone(),
            },
            a.at,
        ),
    }
}

/// Group items into runs and date separators for the feed.
/// `in_group` shows the bro name once at the start of each bro run.
pub fn feed(items: &[ChatItem], fallback: &str, in_group: bool, now: i64) -> Vec<FeedEntry> {
    let gap_in = robroz_tokens::base::chat::GAP_IN;
    let gap_between = robroz_tokens::base::chat::GAP_BETWEEN;
    let date_gap = robroz_tokens::base::chat::DATE_GAP;
    let prepared: Vec<(usize, Meta<Side>)> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let (sender, at) = item_meta(item, fallback);
            (i, Meta { sender, at })
        })
        .collect();
    let blocks = runs::blocks_local(prepared, GROUP_SECS);
    let mut out = Vec::new();
    let mut prev_date = false;
    for (i, block) in blocks.into_iter().enumerate() {
        match block {
            Block::Date(at) => {
                let pad_top = if i == 0 { date_gap } else { date_gap };
                out.push(FeedEntry::Date {
                    label: runs::date_label_local(at, now),
                    pad_top,
                });
                prev_date = true;
            }
            Block::Run(run) => {
                let n = run.items.len();
                let show_name = in_group && !run.sender.is_own() && !run.sender.name().is_empty();
                let last_time = run.last_at.and_then(|at| {
                    let s = status_ui::clock_local(at);
                    (!s.is_empty()).then_some(s)
                });
                for (j, idx) in run.items.into_iter().enumerate() {
                    let pad_top = if j == 0 {
                        if i == 0 || prev_date {
                            if i == 0 && !prev_date {
                                gap_between
                            } else {
                                0.0
                            }
                        } else {
                            gap_between
                        }
                    } else {
                        gap_in
                    };
                    out.push(FeedEntry::Row(FeedRow {
                        item: items[idx].clone(),
                        show_name: show_name && j == 0,
                        show_avatar: !run.sender.is_own() && j + 1 == n,
                        pad_top,
                        time: if j + 1 == n {
                            last_time.clone().unwrap_or_default()
                        } else {
                            String::new()
                        },
                    }));
                }
                prev_date = false;
            }
        }
    }
    out
}

/// Avatar letter: the first character of the name, upper-cased.
pub fn initial(name: &str) -> String {
    conversations::initial(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use robroz_contract::{MessageKind, TaskState};

    fn msg_at(id: &str, body: &str, at: &str) -> Event {
        Event::Message {
            id: id.into(),
            kind: MessageKind::Text,
            body: body.into(),
            created_at: at.into(),
            author_user_id: None,
            author_name: None,
        }
    }

    fn msg(id: &str, body: &str) -> Event {
        msg_at(id, body, "2026-11-20T10:00:00.000Z")
    }

    fn status(task: &str, state: TaskState) -> Event {
        Event::Status {
            progress_code: None,
            progress_params: None,
            id: format!("s-{task}"),
            task_id: task.into(),
            bot_id: "b".into(),
            bot_name: "Ada".into(),
            state,
            label: String::new(),
            created_at: "2026-11-20T10:00:00.000Z".into(),
            updated_at: "2026-11-20T10:00:00.000Z".into(),
            container_up: true,
            failure: None,
            waiting_for: None,
            progress: None,
            status_code: None,
            failure_code: None,
            failure_detail: None,
            started_at: None,
        }
    }

    #[test]
    fn card_updates_in_place_and_sides_are_kept() {
        let events = vec![
            (msg("m1", "write the report"), true),
            (status("t1", TaskState::Running), false),
            (msg("m2", "on it"), false),
            (status("t1", TaskState::Done), false),
        ];
        let items = build(&events);
        assert_eq!(items.len(), 3, "one card per task");
        assert!(matches!(
            &items[0],
            ChatItem::Text {
                side: Side::Own,
                ..
            }
        ));
        assert!(matches!(
            &items[2],
            ChatItem::Text {
                side: Side::Own,
                ..
            } | ChatItem::Text {
                side: Side::Bro { .. },
                ..
            }
        ));
        assert!(matches!(
            &items[2],
            ChatItem::Text {
                side: Side::Bro { .. },
                ..
            }
        ));
        let ChatItem::Status(card) = &items[1] else {
            panic!("card stays where it first appeared")
        };
        assert_eq!(
            card.status_line,
            robroz_card::status_line(TaskState::Done, true)
        );
        assert_eq!(bro_name(&items, "chat"), "Ada");
        assert_eq!(bro_name(&items[..1], "chat"), "chat");
    }

    #[test]
    fn a_run_shows_the_name_once_and_the_avatar_on_the_last() {
        let t0 = "2026-11-20T10:00:00.000Z";
        let t1 = "2026-11-20T10:01:00.000Z";
        let events = vec![
            (msg_at("m1", "one", t0), false),
            (msg_at("m2", "two", t1), false),
        ];
        let items = build(&events);
        let rows = feed(&items, "Ada", true, runs::parse_unix(t1).unwrap());
        let row_flags: Vec<(bool, bool)> = rows
            .iter()
            .filter_map(|e| match e {
                FeedEntry::Row(r) => Some((r.show_name, r.show_avatar)),
                _ => None,
            })
            .collect();
        assert_eq!(row_flags, vec![(true, false), (false, true)]);
    }

    #[test]
    fn own_messages_have_no_avatar() {
        let events = vec![(msg("m1", "hello"), true), (msg("m2", "again"), true)];
        let items = build(&events);
        let rows = feed(&items, "Ada", true, 1_795_000_000);
        let mut times = Vec::new();
        for e in &rows {
            if let FeedEntry::Row(r) = e {
                assert!(!r.show_avatar);
                assert!(!r.show_name);
                times.push(r.time.is_empty());
            }
        }
        // Time only under the last of the run.
        if times.len() >= 2 {
            assert!(times[..times.len() - 1].iter().all(|empty| *empty));
            assert!(!times[times.len() - 1]);
        }
    }

    fn approval(
        id: &str,
        bot_id: &str,
        bot_name: &str,
        summary: &str,
        decision: Option<robroz_contract::ApprovalDecision>,
        owner: Option<(&str, &str)>,
    ) -> Event {
        Event::Approval {
            reason_code: None,
            id: id.into(),
            bot_id: bot_id.into(),
            bot_name: bot_name.into(),
            action: robroz_contract::ApprovalAction::Delete,
            summary: summary.into(),
            path: None,
            decision,
            created_at: "2026-11-20T10:00:00.000Z".into(),
            answered_at: decision.map(|_| "2026-11-20T10:02:00.000Z".into()),
            answered_by: None,
            task_id: None,
            mcp_server: None,
            mcp_tool: None,
            reason: None,
            expires_at: None,
            owner_user_id: owner.map(|o| o.0.into()),
            owner_name: owner.map(|o| o.1.into()),
        }
    }

    #[test]
    fn an_approval_event_is_drawn_and_updates_in_place() {
        let events = vec![
            (msg("m1", "please delete the logs"), true),
            (
                approval(
                    "a1",
                    "b",
                    "Ada",
                    "Delete 12 old log files",
                    None,
                    Some(("u-me", "Panda")),
                ),
                false,
            ),
            (msg("m2", "ok"), true),
        ];
        let items = build(&events);
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[1], ChatItem::Approval(a) if a.card.id == "a1"));
        let ChatItem::Approval(first) = &items[1] else {
            panic!("approval is kept");
        };
        assert_eq!(first.card.state, robroz_ui::cards::ApprovalState::Pending);

        let mut later = events.clone();
        later.push((
            approval(
                "a1",
                "b",
                "Ada",
                "Delete 12 old log files",
                Some(robroz_contract::ApprovalDecision::Denied),
                Some(("u-me", "Panda")),
            ),
            false,
        ));
        let items = build(&later);
        assert_eq!(items.len(), 3, "the decided card stays where it appeared");
        let ChatItem::Approval(decided) = &items[1] else {
            panic!("approval updates in place");
        };
        assert!(matches!(
            decided.card.state,
            robroz_ui::cards::ApprovalState::Denied { .. }
        ));
    }
}
