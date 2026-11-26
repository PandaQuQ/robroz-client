//! One conversation as chat rows: text bubbles and status cards.
//!
//! Own messages are bubbles on the right, a bro's messages bubbles on the
//! left, grouped into runs: the name once above the first item in a group,
//! the avatar only at the last. A task's status card sits on the left
//! under the same rules. A card is keyed by task: later status events
//! update the card where it first appeared. Other card types (delivery,
//! choice, approval, ...) are not drawn in the spike.

use robroz_contract::Event;
use robroz_ui::cards::status as status_ui;
use robroz_ui::conversations;
use robroz_ui::runs::{self, Block, Meta, RunSender, GROUP_SECS};

use crate::card::StatusCard;

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
/// same task in place.
pub fn build(events: &[(Event, bool)]) -> Vec<ChatItem> {
    let mut items: Vec<ChatItem> = Vec::new();
    let mut card_at: Vec<(&str, usize)> = Vec::new();
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
                let Some(card) = StatusCard::from_event(event) else {
                    continue;
                };
                match card_at.iter().find(|(t, _)| *t == task_id.as_str()) {
                    Some(&(_, at)) => items[at] = ChatItem::Status(card),
                    None => {
                        card_at.push((task_id.as_str(), items.len()));
                        items.push(ChatItem::Status(card));
                    }
                }
            }
            _ => {}
        }
    }
    items
}

/// The name shown over a bro's bubbles: the bro on the newest status card,
/// else the conversation's own name.
pub fn bro_name<'a>(items: &'a [ChatItem], fallback: &'a str) -> &'a str {
    items
        .iter()
        .rev()
        .find_map(|i| match i {
            ChatItem::Status(c) => Some(c.bot_name.as_str()),
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
}
