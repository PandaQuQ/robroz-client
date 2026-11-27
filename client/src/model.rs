//! Maps contract events to the card models the view draws.
//!
//! Every colour, status line, action and badge comes from `robroz-card`.
//! This module only decides where a card sits in the stream.

use robroz_card::{Palette, Tone};
use robroz_contract::{
    CardStatusCode, DeliveryBadge, Event, FailureCode, MessageKind, TaskState, WaitingFor,
};
use robroz_ui::cards::codes::{self, FailureLine, ProgressLine};
use robroz_ui::cards::confirm::ConfirmCard;
use robroz_ui::cards::reminder::Reminder;
use robroz_ui::cards::{ApprovalCard, ApprovalExtras, Card, ChoiceCard};
use robroz_ui::i18n::{host_text, Key as Text};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCard {
    pub task_id: String,
    pub bot_id: String,
    pub bot_name: String,
    /// From `robroz-card`, or 「没做成：…」 when the host sent a `failure`.
    pub status_line: String,
    pub tone: Tone,
    pub palette: Palette,
    pub action: Option<&'static str>,
    /// What the bro is doing right now, one line under the bro name.
    /// Live run or a card blocked on approval; not used for Router step
    /// progress, which sits left of the status word.
    pub progress: Option<ProgressLine>,
    /// `progressCode: router_step`. Shown to the left of the status word
    /// on a live run only. Rendered in the active locale when drawn.
    pub step_progress: Option<ProgressLine>,
    /// v0.2: the code form of `label`. When set, the drawn word comes from
    /// i18n, not `status_line`.
    pub status_code: Option<CardStatusCode>,
    /// The code form of `failure`. The drawn line comes from i18n plus
    /// `failure_detail`, never from `status_line` or `failure`.
    pub failure_code: Option<FailureCode>,
    /// Raw `failureDetail` (`500`, `3/2`, `POST`, `server.tool`, …): a
    /// parameter of the code's copy or the raw detail line, never a sentence.
    pub failure_detail: Option<String>,
    /// Raw host `failure`. Without a code it draws the unknown-code copy.
    pub failure: Option<String>,
    pub state: TaskState,
    pub container_up: bool,
    pub waiting_for: Option<WaitingFor>,
    pub created_at: String,
    pub updated_at: String,
    /// First transition into `running` seen this session.
    pub local_started: Option<i64>,
    /// Host `startedAt` for this round, RFC 3339 when present.
    pub started_at: Option<String>,
    /// Progress lines recorded this session, in order.
    pub steps: Vec<ProgressLine>,
    /// The process line is expanded to show [`steps`].
    pub expanded: bool,
}

impl StatusCard {
    /// Live run: `running` and the container is up, not a failure.
    pub fn is_live(&self) -> bool {
        self.failure.is_none() && self.state == TaskState::Running && self.container_up
    }

    pub fn timer(&self, now: i64) -> robroz_ui::process::TimerPhase {
        robroz_ui::process::timer_phase(robroz_ui::process::TimerInput {
            state: self.state,
            live: self.is_live(),
            failed: self.failure.is_some() || self.failure_code.is_some(),
            local_started: self.local_started,
            started_at: self
                .started_at
                .as_deref()
                .and_then(robroz_ui::process::parse_unix),
            created_at: robroz_ui::process::parse_unix(&self.created_at),
            updated_at: robroz_ui::process::parse_unix(&self.updated_at),
            now,
        })
    }

    /// Collapsed process line, if this card is a process (not a failure).
    pub fn process_line(&self, now: i64) -> Option<String> {
        if self.failure.is_some() || self.failure_code.is_some() {
            return None;
        }
        if let Some(line) = self.timer(now).line() {
            return Some(line);
        }
        // Blocked / queued: status word, no timer.
        Some(self.shown_line())
    }

    pub fn expandable(&self) -> bool {
        robroz_ui::process::expandable(&self.steps)
    }
}

impl StatusCard {
    /// The status or failure line in the active locale.
    pub fn shown_line(&self) -> String {
        if let Some(line) = self.failure_display() {
            return line.text;
        }
        if let Some(code) = self.status_code {
            return codes::status_text(code);
        }
        display(&self.status_line).into_owned()
    }

    /// Raw detail line under the failure (monospace, muted, selectable).
    pub fn detail_line(&self) -> Option<String> {
        self.failure_display().and_then(|line| line.detail)
    }

    fn failure_display(&self) -> Option<FailureLine> {
        FailureLine::from_wire(
            self.failure_code,
            self.failure_detail.as_deref(),
            self.failure.as_deref(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryCard {
    pub id: String,
    pub summary: String,
    /// Volume-relative path, shown as-is. Display only.
    pub path: String,
    pub badge: Option<&'static str>,
    /// Why the task did not get done. Drawn grey like 「没写出结果」, no buttons.
    pub failure: Option<String>,
    /// `needsSetup: "search"` (robroz#76): offer 「去设置」 on the card.
    pub needs_search_setup: bool,
    /// `needsSetup: "model"` (robroz#102): offer 「去设置」 on the card.
    pub needs_model_setup: bool,
    /// v0.2: the code form of the briefing badge.
    pub badge_code: Option<DeliveryBadge>,
    pub failure_code: Option<FailureCode>,
    pub failure_detail: Option<String>,
}

impl DeliveryCard {
    pub fn shown_badge(&self) -> Option<String> {
        if let Some(code) = self.badge_code {
            return Some(codes::badge_text(code));
        }
        self.badge.map(|b| display(b).into_owned())
    }

    pub fn shown_failure(&self) -> Option<String> {
        self.failure_line().map(|l| l.text)
    }

    /// Raw detail line under the failure (monospace, muted, selectable).
    pub fn detail_line(&self) -> Option<String> {
        self.failure_line().and_then(|l| l.detail)
    }

    fn failure_line(&self) -> Option<FailureLine> {
        FailureLine::from_wire(
            self.failure_code,
            self.failure_detail.as_deref(),
            self.failure.as_deref(),
        )
    }
}

/// Status line while the task waits on an approval card (`waitingFor:
/// "approval"`). Amber like other blocked cards, no button: the approval
/// card is the only way to act.
pub const AWAITING_APPROVAL: &str = "等你批准";

/// Blocked on an unanswered approval card (`waitingFor: "approval"`, #57).
/// The stuck dot stays amber (`status.blocked`); only the takeover button
/// is omitted. `takeover` or an absent field (older host) keeps amber 卡住
/// + 去私聊接管.
fn awaits_approval(state: TaskState, waiting_for: Option<WaitingFor>) -> bool {
    state == TaskState::Blocked && waiting_for == Some(WaitingFor::Approval)
}

/// How much a preview candidate counts when two events share a change time.
/// An outcome (`failure`) beats the answer that caused it, and 「等你批准」
/// beats the approval card it points at; an answered card is the weakest
/// because its result follows it.
pub fn preview_rank(event: &Event) -> u8 {
    match event {
        Event::Status {
            failure: Some(_), ..
        }
        | Event::Delivery {
            failure: Some(_), ..
        } => 2,
        // The status summarises the wait; its approval card is created at
        // the same instant and must not take over the preview.
        Event::Status {
            state, waiting_for, ..
        } if awaits_approval(*state, *waiting_for) => 2,
        Event::Choice {
            answered_at: Some(_),
            ..
        }
        | Event::Approval {
            answered_at: Some(_),
            ..
        } => 0,
        _ => 1,
    }
}

/// Whether a new preview `(change time, rank)` replaces the current one.
/// Event order across types is not guaranteed (README, #54), so the newest
/// change time wins and a tie goes to the higher rank, not to arrival order.
pub fn preview_wins(new: (&str, u8), cur: Option<(&str, u8)>) -> bool {
    let Some((cur_at, cur_rank)) = cur else {
        return true;
    };
    let parse = |t: &str| chrono::DateTime::parse_from_rfc3339(t).ok();
    match (parse(new.0), parse(cur_at)) {
        (Some(a), Some(b)) if a != b => a > b,
        (Some(_), Some(_)) => new.1 >= cur_rank,
        // Unparseable time: keep the old "last one wins".
        _ => true,
    }
}

/// The event behind each chat's list preview. The event is kept, not its
/// text, so a language switch can redraw every preview at once.
/// `owner_id` is the bro's owner for a status waiting on approval, looked
/// up when the event arrived.
#[derive(Debug, Default)]
pub struct Previews {
    by_group: std::collections::HashMap<String, (String, u8, Event, Option<String>)>,
}

impl Previews {
    /// Offer `event` for `group`'s preview; the preview text if it wins.
    /// `me` is the signed-in user id. `owner_id` is the bro's owner, used
    /// when a status is waiting on approval (reminders carry their own).
    pub fn offer(
        &mut self,
        group: &str,
        event: &Event,
        me: Option<&str>,
        owner_id: Option<&str>,
    ) -> Option<String> {
        let at = change_time(event);
        let rank = preview_rank(event);
        let cur = self.by_group.get(group);
        if !preview_wins((at, rank), cur.map(|(t, r, _, _)| (t.as_str(), *r))) {
            return None;
        }
        self.by_group.insert(
            group.to_string(),
            (
                at.to_string(),
                rank,
                event.clone(),
                owner_id.map(str::to_owned),
            ),
        );
        Some(preview_of(event, me, owner_id))
    }

    /// Every chat's preview, drawn again in the active language.
    pub fn redraw(&self, me: Option<&str>) -> impl Iterator<Item = (&str, String)> + '_ {
        let me = me.map(str::to_owned);
        self.by_group.iter().map(move |(g, (_, _, ev, owner))| {
            (g.as_str(), preview_of(ev, me.as_deref(), owner.as_deref()))
        })
    }
}

/// The grey line for a task the host says did not get done. A dead run reads
/// 「没写出结果」, anything else 「没做成：原因」 (from `robroz-card`).
///
/// Cards keep this Chinese line as built; [`display`] translates it when
/// drawn, so a language switch applies to cards already on screen.
pub fn failure_line(failure: &str) -> String {
    robroz_card::failure_line(failure)
}

/// A status-card line (`status_line`, `progress`, a failure) in the active
/// locale: [`AWAITING_APPROVAL`], `robroz-card` copy and known host strings
/// are translated, anything else is shown as sent.
pub fn display(s: &str) -> std::borrow::Cow<'_, str> {
    if s == AWAITING_APPROVAL {
        std::borrow::Cow::Borrowed(Text::ClientAwaitingApproval.as_str())
    } else {
        host_text(s)
    }
}

/// The failure line in the active locale: from the code when the host
/// sent one, otherwise the unknown-code copy.
fn failure_shown(code: Option<FailureCode>, detail: Option<&str>, legacy: Option<&str>) -> String {
    FailureLine::from_wire(code, detail, Some(legacy.unwrap_or_default()))
        .map(|l| l.text)
        .unwrap_or_default()
}

fn status_shown(code: Option<CardStatusCode>, state: TaskState, container_up: bool) -> String {
    match code {
        Some(code) => codes::status_text(code),
        None => display(robroz_card::status_line(state, container_up)).into_owned(),
    }
}

/// 「名字：内容」 for the chat list preview.
fn line(bot_name: &str, rest: &str) -> String {
    Text::ClientPreviewLine.fill(&[&bot_name, &rest])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Text {
        body: String,
    },
    Attachment {
        kind: MessageKind,
        name: String,
    },
    Status(StatusCard),
    Delivery(DeliveryCard),
    /// New card kinds (`choice`, `approval`, `draft`), drawn by `robroz-ui`.
    Card(Card),
    /// The owner's DM line "{bro} is waiting for your approval in {group}":
    /// one clickable row, not a card.
    Reminder(Reminder),
}

/// Who a feed item belongs to, for the chat bubbles. The host only writes
/// `messages` from a person (`post_message`, `say`), so a `Message` is
/// always [`Sender::Own`] and goes on the right; status and card events
/// belong to a bro and go on the left. A delivery with `bot_name` belongs
/// to that bro; without it, the card is drawn alone ([`Sender::Delivery`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sender {
    Own,
    Bro {
        id: String,
        name: String,
    },
    /// A delivery with no `bot_name`; shown alone, with no avatar.
    Delivery,
    /// An approval reminder row in the owner's DM: full width, no avatar,
    /// no name, no clock of its own (the row shows its time).
    Reminder,
}

impl Sender {
    /// Same sender: both own, or the same bro (by id when both have one,
    /// else by name).
    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Sender::Own, Sender::Own) => true,
            (Sender::Bro { id: a, name: x }, Sender::Bro { id: b, name: y }) => {
                if a.is_empty() || b.is_empty() {
                    x == y
                } else {
                    a == b
                }
            }
            _ => false,
        }
    }

    pub fn is_own(&self) -> bool {
        matches!(self, Sender::Own)
    }

    pub fn is_delivery(&self) -> bool {
        matches!(self, Sender::Delivery)
    }

    pub fn is_reminder(&self) -> bool {
        matches!(self, Sender::Reminder)
    }

    /// The bro's name; empty for own or an unattributed delivery.
    pub fn name(&self) -> &str {
        match self {
            Sender::Bro { name, .. } => name,
            Sender::Own | Sender::Delivery | Sender::Reminder => "",
        }
    }
}

/// Client-side facts about a feed item, kept beside its [`Row`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta {
    pub sender: Sender,
    /// When it was sent (`createdAt`, Unix seconds); `None` for a card the
    /// client made itself.
    pub at: Option<i64>,
}

/// Consecutive items of one sender, drawn as one block: name on the first
/// item (in a group chat), avatar at the last, the time at the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group<T> {
    pub sender: Sender,
    pub items: Vec<T>,
    /// Time of the first item that has one.
    pub first_at: Option<i64>,
    /// Time of the last item that has one.
    pub last_at: Option<i64>,
}

/// Split items into groups: a new group starts when the sender changes or
/// the gap to the previous timed item in the group is over `window` seconds.
/// Items without a time never start a group on their own.
pub fn group_items<T>(items: impl IntoIterator<Item = (T, Meta)>, window: i64) -> Vec<Group<T>> {
    let mut out: Vec<Group<T>> = Vec::new();
    for (item, meta) in items {
        let joins = out.last().is_some_and(|g| {
            g.sender.same(&meta.sender)
                && match (g.last_at, meta.at) {
                    (Some(prev), Some(at)) => (at - prev).abs() <= window,
                    _ => true,
                }
        });
        if joins {
            let g = out.last_mut().expect("joins implies a group");
            g.items.push(item);
            if meta.at.is_some() {
                if g.first_at.is_none() {
                    g.first_at = meta.at;
                }
                g.last_at = meta.at;
            }
        } else {
            out.push(Group {
                sender: meta.sender,
                items: vec![item],
                first_at: meta.at,
                last_at: meta.at,
            });
        }
    }
    out
}

impl robroz_ui::runs::RunSender for Sender {
    fn same(&self, other: &Self) -> bool {
        Sender::same(self, other)
    }
}

/// Runs that also break at a local day change, with a date separator at
/// the start of each day.
pub fn chat_blocks<T, Tz: chrono::TimeZone>(
    items: impl IntoIterator<Item = (T, Meta)>,
    window: i64,
    tz: &Tz,
) -> Vec<robroz_ui::runs::Block<T, Sender>> {
    robroz_ui::runs::blocks(
        items.into_iter().map(|(item, meta)| {
            (
                item,
                robroz_ui::runs::Meta {
                    sender: meta.sender,
                    at: meta.at,
                },
            )
        }),
        window,
        tz,
    )
}

fn unix(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|t| t.timestamp())
}

fn bro(id: &str, name: &str) -> Sender {
    Sender::Bro {
        id: id.to_string(),
        name: name.to_string(),
    }
}

/// A delivery belongs to the named bro; a missing or blank `bot_name`
/// stays unattributed. A missing `bot_id` still groups by name.
fn delivery_sender(bot_id: Option<&str>, bot_name: Option<&str>) -> Sender {
    match bot_name.map(str::trim) {
        Some(name) if !name.is_empty() => bro(bot_id.unwrap_or(""), name),
        _ => Sender::Delivery,
    }
}

/// Who owns a card and when it was made. A delivery with `bot_name` is
/// that bro's; without it, [`Sender::Delivery`].
fn meta_of(event: &Event) -> Meta {
    let (sender, at) = match event {
        Event::Message { created_at, .. } => (Sender::Own, unix(created_at)),
        Event::Delivery {
            created_at,
            bot_id,
            bot_name,
            ..
        } => (
            delivery_sender(bot_id.as_deref(), bot_name.as_deref()),
            unix(created_at),
        ),
        Event::Status {
            bot_id,
            bot_name,
            created_at,
            ..
        }
        | Event::Choice {
            bot_id,
            bot_name,
            created_at,
            ..
        }
        | Event::Approval {
            bot_id,
            bot_name,
            created_at,
            ..
        }
        | Event::Confirm {
            bot_id,
            bot_name,
            created_at,
            ..
        } => (bro(bot_id, bot_name), unix(created_at)),
        Event::ApprovalReminder {
            created_at,
            answered_at,
            ..
        } => (
            Sender::Reminder,
            unix(answered_at.as_deref().unwrap_or(created_at)),
        ),
    };
    Meta { sender, at }
}

fn card_bot(card: &Card) -> &str {
    match card {
        Card::Choice(c) => &c.bot,
        Card::Approval(c) => &c.bot,
        Card::Draft(c) => &c.bot,
        Card::Confirm(c) => &c.bot,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    Event(String),
    Task(String),
}

/// Rows for one group, in arrival order. Applying an event is idempotent, so
/// a source may hand over the full snapshot from
/// `GET /api/groups/:id/events` or only what is new.
#[derive(Debug, Default, Clone)]
pub struct Feed {
    rows: Vec<(Key, Row, Meta)>,
}

impl Feed {
    #[cfg(test)]
    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter().map(|(_, row, _)| row)
    }

    /// Rows with who sent them and when, in arrival order.
    pub fn items(&self) -> impl Iterator<Item = (&Row, &Meta)> {
        self.rows.iter().map(|(_, row, meta)| (row, meta))
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn apply(&mut self, event: &Event) {
        self.apply_with(event, None);
    }

    /// Swap this feed for a resync snapshot in one go. Cards that
    /// disappeared on the host are dropped; matching status cards keep
    /// this session's local timer, recorded steps and expanded state.
    pub fn replace_from_snapshot(&mut self, events: &[(Event, Option<ApprovalExtras>)]) {
        let old = std::mem::take(self);
        for (event, extras) in events {
            if crate::event_decode::is_settings_approval(event) {
                continue;
            }
            let (key, row) = to_row(event, extras.as_ref());
            let row = match row {
                Row::Status(card) => Row::Status(old.merge_status(card)),
                other => other,
            };
            let meta = meta_of(event);
            self.upsert(key, row, Some(meta));
        }
    }

    /// Apply an event, optionally with leftover approval extras (always
    /// empty now that `rule_*` is on the contract). Settings-page rule
    /// cards are ignored: they do not belong in a chat feed.
    pub fn apply_with(&mut self, event: &Event, extras: Option<&ApprovalExtras>) {
        if crate::event_decode::is_settings_approval(event) {
            return;
        }
        let (key, row) = to_row(event, extras);
        let row = match row {
            Row::Status(card) => Row::Status(self.merge_status(card)),
            other => other,
        };
        let meta = meta_of(event);
        self.upsert(key, row, Some(meta));
    }

    fn merge_status(&self, mut card: StatusCard) -> StatusCard {
        let key = Key::Task(card.task_id.clone());
        if let Some((_, Row::Status(old), _)) = self.rows.iter().find(|(k, _, _)| *k == key) {
            card.steps = old.steps.clone();
            card.local_started = old.local_started;
            card.expanded = old.expanded;
            // Only start the local timer when we see the transition into a
            // live run this session, not when a snapshot was already running.
            if card.is_live() && !old.is_live() && card.local_started.is_none() {
                card.local_started = Some(robroz_ui::process::now_unix());
            }
        }
        if let Some(line) = card.progress.clone().or(card.step_progress.clone()) {
            robroz_ui::process::push_step(&mut card.steps, line);
        }
        card
    }

    /// Live-running bros in this feed, first-seen order, unique by bot id.
    pub fn working_bros(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (_, row, _) in &self.rows {
            if let Row::Status(card) = row {
                if card.is_live() && !out.iter().any(|(id, _)| id == &card.bot_id) {
                    out.push((card.bot_id.clone(), card.bot_name.clone()));
                }
            }
        }
        out
    }

    /// Toggle the process line for this task. No-op when it has no steps.
    pub fn toggle_process(&mut self, task_id: &str) {
        let key = Key::Task(task_id.to_string());
        if let Some((_, Row::Status(card), _)) = self.rows.iter_mut().find(|(k, _, _)| *k == key) {
            if card.expandable() {
                card.expanded = !card.expanded;
            }
        }
    }

    pub fn any_live_timer(&self) -> bool {
        self.rows.iter().any(|(_, row, _)| {
            matches!(row, Row::Status(c) if c.timer(robroz_ui::process::now_unix()).is_running())
        })
    }

    /// Insert or replace a new-kind card by its id. Used for card kinds the
    /// contract has no event for yet (approval, draft) and for applying the
    /// host's reply to an answer before the next poll.
    pub fn apply_card(&mut self, card: Card) {
        let key = Key::Event(card.id().to_string());
        let meta = Meta {
            sender: bro("", card_bot(&card)),
            at: None,
        };
        self.upsert(key, Row::Card(card), Some(meta));
    }

    pub fn card(&self, id: &str) -> Option<&Card> {
        self.rows.iter().find_map(|(k, row, _)| match (k, row) {
            (Key::Event(k), Row::Card(c)) if k == id => Some(c),
            _ => None,
        })
    }

    /// A replaced row keeps its place and its time (an answered card was
    /// still sent when it was first sent). Reminder rows are the exception:
    /// they move to their latest change time, so an approved row at 19:02
    /// sits after a waiting row at 18:50.
    fn upsert(&mut self, key: Key, row: Row, meta: Option<Meta>) {
        let reminder = matches!(row, Row::Reminder(_));
        if let Some(idx) = self.rows.iter().position(|(k, _, _)| *k == key) {
            self.rows[idx].1 = row;
            if let Some(m) = meta {
                self.rows[idx].2.sender = m.sender;
                if reminder {
                    self.rows[idx].2.at = m.at;
                    let item = self.rows.remove(idx);
                    let pos = reminder_index(&self.rows, item.2.at);
                    self.rows.insert(pos, item);
                } else {
                    self.rows[idx].2.at = self.rows[idx].2.at.or(m.at);
                }
            }
        } else {
            let meta = meta.expect("a new row has meta");
            if reminder {
                let pos = reminder_index(&self.rows, meta.at);
                self.rows.insert(pos, (key, row, meta));
            } else {
                self.rows.push((key, row, meta));
            }
        }
    }
}

/// Where a reminder with time `at` belongs: after earlier rows, before
/// later ones. A row with no time stays where it is (treated as earliest).
fn reminder_index(rows: &[(Key, Row, Meta)], at: Option<i64>) -> usize {
    rows.iter()
        .position(|(_, _, meta)| meta.at > at)
        .unwrap_or(rows.len())
}

/// When this card last changed, as the host defines it for `?since=`:
/// `updatedAt` for status cards, `answeredAt` for answered choices and approvals,
/// otherwise `createdAt`.
pub fn change_time(event: &Event) -> &str {
    match event {
        Event::Status { updated_at, .. } => updated_at,
        Event::Choice {
            answered_at: Some(at),
            ..
        }
        | Event::Approval {
            answered_at: Some(at),
            ..
        }
        | Event::Confirm {
            answered_at: Some(at),
            ..
        }
        | Event::ApprovalReminder {
            answered_at: Some(at),
            ..
        } => at,
        Event::ApprovalReminder { created_at, .. } => created_at,
        Event::Choice { created_at, .. }
        | Event::Approval { created_at, .. }
        | Event::Confirm { created_at, .. }
        | Event::Message { created_at, .. }
        | Event::Delivery { created_at, .. } => created_at,
    }
}

/// One-line preview of an event for the conversation list.
pub fn preview(event: &Event) -> String {
    preview_of(event, None, None)
}

/// Like [`preview`], with the signed-in user id and, for a status waiting
/// on approval, the bro's owner id. A reminder already carries its owner
/// on the event. A missing `owner_id` is the bootstrap admin
/// (`local-admin`), same as the card; unknown `me` uses the non-owner
/// sentence.
pub fn preview_of(event: &Event, me: Option<&str>, owner_id: Option<&str>) -> String {
    match event {
        Event::Message { kind, body, .. } => match kind {
            MessageKind::Text => body.clone(),
            MessageKind::Image | MessageKind::File => {
                format!("[{}] {body}", attachment_label(*kind))
            }
        },
        Event::Status {
            bot_name,
            failure: Some(failure),
            failure_code,
            failure_detail,
            ..
        } => line(
            bot_name,
            &failure_shown(*failure_code, failure_detail.as_deref(), Some(failure)),
        ),
        Event::Status {
            bot_name,
            state,
            waiting_for,
            ..
        } if awaits_approval(*state, *waiting_for) => {
            // Same rule as the card and the reminder: [`is_bro_owner`].
            // A missing owner is `local-admin`; unknown me is not the owner.
            let text = if robroz_ui::cards::is_bro_owner(me, owner_id) {
                display(AWAITING_APPROVAL)
            } else {
                std::borrow::Cow::Borrowed(Text::ApprovalWaitingFor.as_str())
            };
            line(bot_name, &text)
        }
        // A live run with a progress line: say what the bro is doing, not 进行中.
        Event::Status {
            bot_name,
            state,
            container_up,
            failure,
            progress,
            progress_code,
            progress_params,
            waiting_for,
            ..
        } if progress_line(
            *state,
            *container_up,
            failure.as_deref(),
            *waiting_for,
            (
                *progress_code,
                progress_params.as_ref(),
                progress.as_deref(),
            ),
        )
        .is_some() =>
        {
            let progress = progress_line(
                *state,
                *container_up,
                failure.as_deref(),
                *waiting_for,
                (
                    *progress_code,
                    progress_params.as_ref(),
                    progress.as_deref(),
                ),
            );
            line(bot_name, &progress.map(|p| p.text()).unwrap_or_default())
        }
        Event::Status {
            bot_name,
            state,
            container_up,
            status_code,
            ..
        } => line(bot_name, &status_shown(*status_code, *state, *container_up)),
        Event::Delivery {
            failure: Some(failure),
            failure_code,
            failure_detail,
            ..
        } => failure_shown(*failure_code, failure_detail.as_deref(), Some(failure)),
        Event::Delivery { summary, .. } => host_text(summary).into_owned(),
        Event::Choice {
            bot_name, prompt, ..
        } => line(bot_name, prompt),
        Event::Approval {
            bot_name, summary, ..
        } => line(bot_name, &host_text(summary)),
        Event::Confirm {
            bot_name,
            text,
            title,
            ..
        } => line(
            bot_name,
            text.as_deref().or(title.as_deref()).unwrap_or_default(),
        ),
        // The same sentence the row shows: the ask, then the result.
        Event::ApprovalReminder { .. } => Reminder::from_event(event)
            .map(|r| r.text(&chrono::Local, me))
            .unwrap_or_default(),
    }
}

fn to_row(event: &Event, extras: Option<&ApprovalExtras>) -> (Key, Row) {
    match event {
        Event::Message { id, kind, body, .. } => {
            let row = match kind {
                MessageKind::Text => Row::Text { body: body.clone() },
                MessageKind::Image | MessageKind::File => Row::Attachment {
                    kind: *kind,
                    name: body.clone(),
                },
            };
            (Key::Event(id.clone()), row)
        }
        Event::Status {
            task_id,
            bot_id,
            bot_name,
            state,
            container_up,
            failure,
            progress,
            progress_code,
            progress_params,
            waiting_for,
            status_code,
            failure_code,
            failure_detail,
            created_at,
            updated_at,
            started_at,
            ..
        } => {
            // A failed task is grey with no action, like a dead run, never
            // the green 完成.
            let (look, status_line) = match failure {
                Some(f) => (
                    robroz_card::appearance(TaskState::Running, false),
                    failure_line(f),
                ),
                // Waiting on an approval card: amber, no takeover button.
                // Amber 卡住 + 去私聊接管 stays for real takeovers.
                None if awaits_approval(*state, *waiting_for) => (
                    robroz_card::awaiting_approval(),
                    AWAITING_APPROVAL.to_string(),
                ),
                None => (
                    robroz_card::appearance(*state, *container_up),
                    robroz_card::status_line(*state, *container_up).to_string(),
                ),
            };
            let shown = progress_line(
                *state,
                *container_up,
                failure.as_deref(),
                *waiting_for,
                (
                    *progress_code,
                    progress_params.as_ref(),
                    progress.as_deref(),
                ),
            );
            let (progress, step_progress) = split_progress(*state, *container_up, shown);
            let card = StatusCard {
                task_id: task_id.clone(),
                bot_id: bot_id.clone(),
                bot_name: bot_name.clone(),
                status_line,
                tone: look.tone,
                palette: robroz_card::palette(look.tone),
                action: look.action,
                progress,
                step_progress,
                status_code: *status_code,
                failure_code: *failure_code,
                failure_detail: failure_detail.clone(),
                failure: failure.clone(),
                state: *state,
                container_up: *container_up,
                waiting_for: *waiting_for,
                created_at: created_at.clone(),
                updated_at: updated_at.clone(),
                local_started: None,
                started_at: started_at.clone(),
                steps: Vec::new(),
                expanded: false,
            };
            (Key::Task(task_id.clone()), Row::Status(card))
        }
        // A briefing is the same delivery event with `briefing: true`; the
        // badge is driven only by that contract field.
        Event::Delivery {
            id,
            summary,
            path,
            briefing,
            failure,
            needs_setup,
            badge,
            failure_code,
            failure_detail,
            ..
        } => {
            let card = DeliveryCard {
                id: id.clone(),
                summary: summary.clone(),
                path: path.clone(),
                badge: briefing.then(robroz_card::briefing_badge),
                failure: failure.clone(),
                needs_search_setup: needs_setup.as_deref() == Some("search"),
                needs_model_setup: needs_setup.as_deref() == Some("model"),
                badge_code: *badge,
                failure_code: *failure_code,
                failure_detail: failure_detail.clone(),
            };
            (Key::Event(id.clone()), Row::Delivery(card))
        }
        // Keyed by id, so the answered snapshot replaces the open card in place.
        Event::Choice { id, .. } => {
            let card = ChoiceCard::from_event(event)
                .expect("choice event")
                .seen_by(crate::user_name());
            (Key::Event(id.clone()), Row::Card(Card::Choice(card)))
        }
        // Same as choice: the answered snapshot replaces the pending card.
        Event::Approval { id, .. } => {
            let card = ApprovalCard::from_event_with(event, extras)
                .expect("approval event")
                .seen_by(crate::user_name());
            (Key::Event(id.clone()), Row::Card(Card::Approval(card)))
        }
        // A bro's proposal; the answered snapshot replaces the pending card.
        Event::Confirm { id, .. } => {
            let card = ConfirmCard::from_event(event)
                .expect("confirm event")
                .seen_by(crate::user_name());
            (Key::Event(id.clone()), Row::Card(Card::Confirm(card)))
        }
        // v0.2: one line in the owner's DM; the same id updates it in place.
        Event::ApprovalReminder { id, .. } => (
            Key::Event(id.clone()),
            Row::Reminder(Reminder::from_event(event).expect("reminder event")),
        ),
    }
}

/// The progress line a status card shows, if any, from `progressCode` +
/// `progressParams` (`progress` text alone draws the unknown-code copy).
/// The host already sends it only on a live run, but the card also checks,
/// so an old or odd event never puts a stale line next to 完成 or 没做成.
fn progress_line(
    state: TaskState,
    container_up: bool,
    failure: Option<&str>,
    waiting_for: Option<WaitingFor>,
    (code, params, legacy): (
        Option<robroz_contract::ProgressCode>,
        Option<&robroz_contract::ProgressParams>,
        Option<&str>,
    ),
) -> Option<ProgressLine> {
    let live = state == TaskState::Running && container_up;
    if failure.is_some() || !(live || awaits_approval(state, waiting_for)) {
        return None;
    }
    ProgressLine::from_wire(code, params, legacy)
}

/// A live-run Router step goes left of the status word; everything else
/// that `progress_line` accepted stays under the bro name.
fn split_progress(
    state: TaskState,
    container_up: bool,
    shown: Option<ProgressLine>,
) -> (Option<ProgressLine>, Option<ProgressLine>) {
    match shown {
        Some(line) if state == TaskState::Running && container_up && line.is_step() => {
            (None, Some(line))
        }
        other => (other, None),
    }
}

/// Plain label for an attachment row. Not a task state, so it lives here.
pub fn attachment_label(kind: MessageKind) -> &'static str {
    match kind {
        MessageKind::Image => Text::ClientAttachImage.as_str(),
        MessageKind::File => Text::ClientAttachFile.as_str(),
        MessageKind::Text => Text::ClientAttachText.as_str(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn display_translates_card_lines_and_keeps_the_rest() {
        use robroz_ui::i18n::{scoped, Locale};
        let _en = scoped(Locale::En);
        assert_eq!(display(AWAITING_APPROVAL), "Needs your approval");
        assert_eq!(display(robroz_card::NO_RESULT), "Finished with no result.");
        assert_eq!(
            display(robroz_contract::APPROVAL_SENT_TO_OWNER_DM_NOTICE),
            "Sent to owner's DM"
        );
        // Host-crate text is matched by value; pin it to the host constant.
        assert_eq!(
            display(robroz_host::review::NO_MODEL_REASON),
            "No model set up, so auto-review can't run"
        );
        assert_eq!(
            display(&failure_line("工具调用超过 8 轮")),
            "Couldn't finish: Stopped after 8 rounds of tool calls"
        );
        assert_eq!(display("Ada 的草稿"), "Ada 的草稿");
        assert_eq!(
            preview(&delivery("d-written", "tasks/t1/result.txt", false)),
            "Result written"
        );
        assert_eq!(
            host_text(robroz_contract::host_text::DELIVERY_NOT_RUN).as_ref(),
            "Didn't run"
        );
        let _zh = scoped(Locale::Zh);
        assert_eq!(display(AWAITING_APPROVAL), AWAITING_APPROVAL);
        assert_eq!(
            preview(&delivery("d-written", "tasks/t1/result.txt", false)),
            robroz_contract::host_text::DELIVERY_WRITTEN
        );
    }

    use super::*;

    fn confirm(decision: Option<robroz_contract::ApprovalDecision>) -> Event {
        Event::Confirm {
            id: "c1".into(),
            bot_id: "bot-ada".into(),
            bot_name: "Ada".into(),
            subject: robroz_contract::ConfirmSubject::Memory,
            task_id: Some("t1".into()),
            memory_id: Some("m1".into()),
            kind: Some(robroz_contract::MemoryKind::Habit),
            text: Some("周报先说结论".into()),
            scope: Some("user".into()),
            slug: None,
            title: None,
            instruction: None,
            decision,
            created_at: "2026-11-20T11:00:00.000Z".into(),
            answered_at: decision.map(|_| "2026-11-20T11:05:00.000Z".into()),
            answered_by: decision.map(|_| "Panda 的手机".into()),
        }
    }

    #[test]
    fn confirm_card_is_drawn_and_answered_in_place() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        use robroz_ui::cards::confirm::ConfirmState;
        let mut feed = Feed::default();
        feed.apply(&confirm(None));
        let Some(Card::Confirm(c)) = feed.card("c1") else {
            panic!("confirm card expected");
        };
        assert!(c.is_pending());
        assert_eq!(c.scope_line.as_deref(), Some("所有 bro"));
        assert_eq!(preview(&confirm(None)), "Ada：周报先说结论");
        // The answered snapshot (here: someone else, after our 409) replaces it.
        feed.apply(&confirm(Some(robroz_contract::ApprovalDecision::Denied)));
        assert_eq!(feed.len(), 1, "one card, updated in place");
        let Some(Card::Confirm(c)) = feed.card("c1") else {
            panic!("confirm card expected");
        };
        assert!(
            matches!(&c.state, ConfirmState::Dropped { by: Some(b), .. } if b == "Panda 的手机")
        );
    }

    fn status(task: &str, bot: &str, state: TaskState, container_up: bool) -> Event {
        Event::Status {
            progress_code: None,
            progress_params: None,
            id: task.into(),
            task_id: task.into(),
            bot_id: format!("bot-{bot}"),
            bot_name: bot.into(),
            state,
            label: state.label().into(),
            created_at: "2026-10-07T19:00:00.000Z".into(),
            updated_at: "2026-10-07T19:00:00.000Z".into(),
            container_up,
            failure: None,
            progress: None,
            waiting_for: None,
            status_code: None,
            failure_code: None,
            failure_detail: None,
            started_at: None,
        }
    }

    fn text(id: &str, body: &str) -> Event {
        Event::Message {
            id: id.into(),
            kind: MessageKind::Text,
            body: body.into(),
            created_at: "2026-10-07T19:00:00.000Z".into(),
            author_name: None,
            author_user_id: None,
        }
    }

    fn delivery(id: &str, path: &str, briefing: bool) -> Event {
        delivery_from(id, path, briefing, "2026-10-07T19:00:00.000Z", None, None)
    }

    fn delivery_from(
        id: &str,
        path: &str,
        briefing: bool,
        created_at: &str,
        bot_id: Option<&str>,
        bot_name: Option<&str>,
    ) -> Event {
        Event::Delivery {
            id: id.into(),
            summary: "已写入结果".into(),
            path: path.into(),
            briefing,
            created_at: created_at.into(),
            failure: None,
            needs_setup: None,
            task_id: None,
            bot_id: bot_id.map(str::to_string),
            bot_name: bot_name.map(str::to_string),
            badge: None,
            failure_code: None,
            failure_detail: None,
        }
    }

    fn status_cards(feed: &Feed) -> Vec<&StatusCard> {
        feed.rows()
            .filter_map(|r| match r {
                Row::Status(c) => Some(c),
                _ => None,
            })
            .collect()
    }

    fn expected(
        state: TaskState,
        container_up: bool,
    ) -> (&'static str, Tone, Palette, Option<&'static str>) {
        let look = robroz_card::appearance(state, container_up);
        (
            robroz_card::status_line(state, container_up),
            look.tone,
            robroz_card::palette(look.tone),
            look.action,
        )
    }

    fn actual(card: &StatusCard) -> (&str, Tone, Palette, Option<&'static str>) {
        (&card.status_line, card.tone, card.palette, card.action)
    }

    #[test]
    fn status_updates_in_place_by_task_id() {
        let mut feed = Feed::default();
        feed.apply(&text("m1", "开始"));
        feed.apply(&status("t1", "Ada", TaskState::Queued, false));
        feed.apply(&text("m2", "中间插一句"));
        assert_eq!(
            actual(status_cards(&feed)[0]),
            expected(TaskState::Queued, false)
        );

        feed.apply(&status("t1", "Ada", TaskState::Running, true));
        assert_eq!(
            actual(status_cards(&feed)[0]),
            expected(TaskState::Running, true)
        );

        feed.apply(&status("t1", "Ada", TaskState::Done, false));
        feed.apply(&delivery("d1", "tasks/t1/result.txt", false));

        assert_eq!(feed.len(), 4, "one status card, not three");
        let cards = status_cards(&feed);
        assert_eq!(cards.len(), 1);
        assert_eq!(actual(cards[0]), expected(TaskState::Done, false));
        assert_eq!(cards[0].bot_name, "Ada");
        // Still in its original slot, between the two messages.
        assert!(matches!(feed.rows().nth(1), Some(Row::Status(_))));
        assert!(matches!(feed.rows().nth(3), Some(Row::Delivery(_))));
    }

    /// A v0.3 host event: the legacy `progress` text plus the code and
    /// params the host stores for it (a bro's own line is `custom`).
    fn with_progress(mut e: Event, text: Option<&str>) -> Event {
        use robroz_contract::{ProgressCode as P, ProgressParams};
        if let Event::Status {
            progress,
            progress_code,
            progress_params,
            ..
        } = &mut e
        {
            *progress = text.map(Into::into);
            let none = ProgressParams::default();
            let coded = text.filter(|t| !t.trim().is_empty()).map(|t| {
                if let Some((n, m)) = robroz_contract::host_text::parse_router_step_progress(t) {
                    let p = ProgressParams {
                        step: Some(n as u32),
                        steps: Some(m as u32),
                        ..none.clone()
                    };
                    return (P::RouterStep, p);
                }
                match t {
                    "开始干活" => (P::Started, none.clone()),
                    "在想怎么回" => (P::Thinking, none.clone()),
                    "在沙箱里跑" => (P::InSandbox, none.clone()),
                    _ => match t.strip_prefix("在用 ") {
                        Some(tool) => (
                            P::UsingTool,
                            ProgressParams {
                                tool: Some(tool.into()),
                                ..none.clone()
                            },
                        ),
                        None => (
                            P::Custom,
                            ProgressParams {
                                text: Some(t.into()),
                                ..none.clone()
                            },
                        ),
                    },
                }
            });
            *progress_code = coded.as_ref().map(|c| c.0);
            *progress_params = coded.map(|c| c.1).filter(|p| !p.is_empty());
        }
        e
    }

    /// A progress line as drawn in Chinese.
    fn zh(p: &Option<ProgressLine>) -> Option<String> {
        p.as_ref().map(|p| p.text_in(robroz_ui::i18n::Locale::Zh))
    }

    #[test]
    fn progress_shows_on_a_live_run_and_updates_in_place() {
        let mut feed = Feed::default();
        let run = status("t1", "Ada", TaskState::Running, true);
        feed.apply(&with_progress(run.clone(), Some("开始干活")));
        assert_eq!(
            zh(&status_cards(&feed)[0].progress).as_deref(),
            Some("开始干活")
        );

        feed.apply(&with_progress(run.clone(), Some("在沙箱里跑")));
        assert_eq!(feed.len(), 1, "same card, new line");
        assert_eq!(
            zh(&status_cards(&feed)[0].progress).as_deref(),
            Some("在沙箱里跑")
        );

        // The host drops the field: the card goes back to one line.
        feed.apply(&with_progress(run, None));
        assert_eq!(status_cards(&feed)[0].progress, None);
        assert_eq!(
            actual(status_cards(&feed)[0]),
            expected(TaskState::Running, true)
        );
    }

    #[test]
    fn progress_is_one_line_and_never_on_a_finished_or_stuck_card() {
        let line = |e: Event| match to_row(&e, None).1 {
            Row::Status(c) => zh(&c.progress),
            _ => unreachable!(),
        };
        let p = Some("  在想\n怎么   回 ");
        assert_eq!(
            line(with_progress(
                status("t", "Ada", TaskState::Running, true),
                p
            ))
            .as_deref(),
            Some("在想 怎么 回")
        );
        assert_eq!(
            line(with_progress(
                status("t", "Ada", TaskState::Running, true),
                Some(" \n ")
            )),
            None,
            "blank progress takes no room"
        );
        for (state, up) in [
            (TaskState::Queued, false),
            (TaskState::Running, false),
            (TaskState::Blocked, false),
            (TaskState::Done, false),
        ] {
            assert_eq!(line(with_progress(status("t", "Ada", state, up), p)), None);
        }
        let mut failed = with_progress(status("t", "Ada", TaskState::Running, true), p);
        if let Event::Status { failure, .. } = &mut failed {
            *failure = Some("模型没回：超时".into());
        }
        assert_eq!(line(failed), None, "没做成 never carries a progress line");
    }

    #[test]
    fn list_preview_shows_progress_while_running() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        let run = status("t1", "Ada", TaskState::Running, true);
        assert_eq!(
            preview(&with_progress(run.clone(), Some("在查上周的\n工单"))),
            "Ada：在查上周的 工单"
        );
        // No progress: the displayed state word (「进行中」, not the card
        // crate's raw 「进行」).
        assert_eq!(preview(&run), "Ada：进行中");
        // Never on a finished, stuck or failed card.
        let done = with_progress(status("t1", "Ada", TaskState::Done, false), Some("在查"));
        assert_eq!(
            preview(&done),
            format!("Ada：{}", robroz_card::status_line(TaskState::Done, false))
        );
        let mut failed = with_progress(run, Some("在查"));
        if let Event::Status { failure, .. } = &mut failed {
            *failure = Some("模型没回：超时".into());
        }
        // Legacy text with no code is the unknown-code copy, never host text.
        assert_eq!(preview(&failed), "Ada：出错了");
        if let Event::Status {
            failure_code,
            failure_detail,
            ..
        } = &mut failed
        {
            *failure_code = Some(FailureCode::ModelNoReply);
            *failure_detail = Some("504".into());
        }
        assert_eq!(preview(&failed), "Ada：没做成：模型没回（HTTP 504）");
    }

    #[test]
    fn long_progress_preview_is_cut_with_an_ellipsis() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        let long = "在查上周的工单，按客户和优先级整理成一份周报草稿，再把超时没回的单子单独列出来，最后发给你确认";
        let run = with_progress(status("t1", "Ada", TaskState::Running, true), Some(long));
        // The list row fits the stored preview to its width with `…`.
        let shown = robroz_ui::conversations::fit(&preview(&run), 30);
        assert!(shown.starts_with("Ada：在查上周的"));
        assert!(shown.ends_with('…'), "{shown}");
    }

    #[test]
    fn progress_from_host_json() {
        let json = serde_json::json!({
            "type": "status", "id": "t1", "taskId": "t1", "botId": "bot-ada",
            "botName": "Ada", "state": "running", "label": "进行中",
            "createdAt": "2026-11-20T10:00:00.000Z", "updatedAt": "2026-11-20T10:00:01.000Z",
            "containerUp": true, "progress": "在沙箱里跑", "progressCode": "in_sandbox"
        });
        let event: Event = serde_json::from_value(json).unwrap();
        let mut feed = Feed::default();
        feed.apply(&event);
        assert_eq!(
            zh(&status_cards(&feed)[0].progress).as_deref(),
            Some("在沙箱里跑")
        );
        assert_eq!(status_cards(&feed)[0].step_progress, None);
    }

    fn card_progress(e: Event) -> (Option<String>, Option<String>) {
        match to_row(&e, None).1 {
            Row::Status(c) => (zh(&c.progress), zh(&c.step_progress)),
            _ => unreachable!(),
        }
    }

    #[test]
    fn router_step_progress_sits_left_of_status_only_on_a_live_run() {
        let step = robroz_contract::host_text::router_step_progress(2, 3);
        assert_eq!(step, "2/3 步");
        let live = with_progress(status("t", "Ada", TaskState::Running, true), Some(&step));
        assert_eq!(
            card_progress(live.clone()),
            (None, Some("第 2/3 步".into()))
        );

        // Ordinary progress and a finished or stuck card never take the slot.
        let ordinary = with_progress(
            status("t", "Ada", TaskState::Running, true),
            Some("在沙箱里跑"),
        );
        assert_eq!(card_progress(ordinary), (Some("在沙箱里跑".into()), None));
        for (state, up) in [
            (TaskState::Queued, false),
            (TaskState::Running, false),
            (TaskState::Done, false),
        ] {
            assert_eq!(
                card_progress(with_progress(status("t", "Ada", state, up), Some(&step))),
                (None, None),
                "{state:?} container_up={up}"
            );
        }
        let mut failed = live;
        if let Event::Status { failure, .. } = &mut failed {
            *failure = Some("模型没回：模型服务返回 503".into());
        }
        assert_eq!(
            card_progress(failed),
            (None, None),
            "a failed card does not keep the step text"
        );

        // A card blocked on approval keeps its progress under the bro name,
        // even if the host text happens to look like a step.
        let blocked = with_progress(
            blocked_waiting(Some(WaitingFor::Approval)),
            Some("在用 notes.write_note"),
        );
        assert_eq!(
            card_progress(blocked),
            (Some("在用 notes.write_note".into()), None)
        );
        let blocked_step = with_progress(blocked_waiting(Some(WaitingFor::Approval)), Some(&step));
        assert_eq!(
            card_progress(blocked_step),
            (Some("第 2/3 步".into()), None),
            "approval-blocked progress stays under the name"
        );
        let sent = with_progress(
            blocked_waiting(Some(WaitingFor::Approval)),
            Some(robroz_contract::APPROVAL_SENT_TO_OWNER_DM_NOTICE),
        );
        assert_eq!(
            card_progress(sent),
            (
                Some(robroz_contract::APPROVAL_SENT_TO_OWNER_DM_NOTICE.into()),
                None
            )
        );
        // Translated English is not a match; we parse the host form only.
        assert_eq!(
            card_progress(with_progress(
                status("t", "Ada", TaskState::Running, true),
                Some("Step 2 of 3"),
            )),
            (Some("Step 2 of 3".into()), None)
        );
    }

    #[test]
    fn router_step_progress_is_translated_on_the_card_and_in_the_list_preview() {
        let ev = with_progress(
            status("t", "Ada", TaskState::Running, true),
            Some(&robroz_contract::host_text::router_step_progress(2, 3)),
        );
        {
            let _en = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::En);
            let Row::Status(c) = to_row(&ev, None).1 else {
                unreachable!()
            };
            assert_eq!(c.progress, None);
            assert_eq!(
                c.step_progress.map(|p| p.text()).as_deref(),
                Some("Step 2 of 3")
            );
            assert_eq!(preview(&ev), "Ada: Step 2 of 3");
        }
        {
            let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
            let (progress, step) = card_progress(ev.clone());
            assert_eq!(progress, None);
            assert_eq!(step.as_deref(), Some("第 2/3 步"));
            assert_eq!(preview(&ev), "Ada：第 2/3 步");
        }
    }

    #[test]
    fn dead_running_renders_no_result_gray_without_action() {
        let mut feed = Feed::default();
        feed.apply(&status("t2", "Cy", TaskState::Running, true));
        feed.apply(&status("t2", "Cy", TaskState::Running, false));
        let cards = status_cards(&feed);
        assert_eq!(cards.len(), 1);
        assert_eq!(actual(cards[0]), expected(TaskState::Running, false));
        assert_eq!(cards[0].status_line, "没写出结果");
        assert_eq!(cards[0].palette, expected(TaskState::Queued, false).2);
        assert_eq!(cards[0].action, None);
    }

    #[test]
    fn failed_task_is_grey_without_action_or_done() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        let mut ev = status("t4", "Ada", TaskState::Done, false);
        if let Event::Status { failure, .. } = &mut ev {
            *failure = Some("模型没回：模型服务返回 500".into());
        }
        let mut feed = Feed::default();
        feed.apply(&status("t4", "Ada", TaskState::Running, true));
        feed.apply(&ev);
        let cards = status_cards(&feed);
        assert_eq!(cards.len(), 1, "updates in place");
        assert_eq!(cards[0].status_line, "没做成：模型没回：模型服务返回 500");
        assert_eq!(cards[0].tone, Tone::Gray);
        assert_eq!(cards[0].palette, expected(TaskState::Running, false).2);
        assert_ne!(cards[0].palette, expected(TaskState::Done, false).2);
        assert_eq!(cards[0].action, None);
        // No `failureCode`: the unknown-code copy, never the host text.
        assert_eq!(preview(&ev), "Ada：出错了");
    }

    #[test]
    fn dead_run_ended_as_done_still_reads_no_result() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        let mut ev = status("t5", "Cy", TaskState::Done, false);
        if let Event::Status { failure, .. } = &mut ev {
            *failure = Some("没写出结果".into());
        }
        let mut feed = Feed::default();
        feed.apply(&status("t5", "Cy", TaskState::Running, true));
        feed.apply(&ev);
        let cards = status_cards(&feed);
        assert_eq!(cards[0].status_line, "没写出结果");
        assert_eq!(cards[0].tone, Tone::Gray);
        assert_eq!(cards[0].palette, expected(TaskState::Running, false).2);
        assert_eq!(cards[0].action, None);
        assert_eq!(preview(&ev), "Cy：没写出结果");
    }

    #[test]
    fn failed_delivery_carries_failure_and_old_json_is_unchanged() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        let mut ev = delivery("d9", "tasks/t9/result.txt", false);
        if let Event::Delivery { failure, .. } = &mut ev {
            *failure = Some("你拒绝了：发送周报".into());
        }
        let mut feed = Feed::default();
        feed.apply(&ev);
        assert!(matches!(
            feed.rows().next(),
            Some(Row::Delivery(DeliveryCard { failure: Some(f), .. })) if f == "你拒绝了：发送周报"
        ));
        assert_eq!(preview(&ev), "出错了");
        if let Event::Delivery {
            failure_code,
            failure_detail,
            ..
        } = &mut ev
        {
            *failure_code = Some(robroz_contract::FailureCode::Denied);
            *failure_detail = Some("mail.send".into());
        }
        assert_eq!(preview(&ev), "没做成：你拒绝了 mail.send");

        let old: Event = serde_json::from_value(serde_json::json!({
            "type": "delivery", "id": "d1", "summary": "已写入结果",
            "path": "tasks/t1/result.txt", "briefing": false,
            "createdAt": "2026-10-07T19:00:00.000Z"
        }))
        .unwrap();
        let mut feed = Feed::default();
        feed.apply(&old);
        assert!(matches!(
            feed.rows().next(),
            Some(Row::Delivery(DeliveryCard { failure: None, .. }))
        ));
        assert_eq!(preview(&old), "已写入结果");

        let mut setup = ev.clone();
        if let Event::Delivery {
            failure,
            needs_setup,
            ..
        } = &mut setup
        {
            *failure = Some("还没设置联网搜索".into());
            *needs_setup = Some("search".into());
        }
        let mut feed = Feed::default();
        feed.apply(&setup);
        assert!(matches!(
            feed.rows().next(),
            Some(Row::Delivery(DeliveryCard {
                needs_search_setup: true,
                ..
            }))
        ));

        let mut setup_ok = ev.clone();
        if let Event::Delivery {
            failure,
            needs_setup,
            ..
        } = &mut setup_ok
        {
            *failure = None;
            *needs_setup = Some("search".into());
        }
        let mut feed = Feed::default();
        feed.apply(&setup_ok);
        assert!(
            matches!(
                feed.rows().next(),
                Some(Row::Delivery(DeliveryCard {
                    needs_search_setup: true,
                    failure: None,
                    ..
                }))
            ),
            "去设置 when needsSetup is set, even without failure"
        );

        let mut text_only = ev.clone();
        if let Event::Delivery {
            failure,
            needs_setup,
            ..
        } = &mut text_only
        {
            *failure = Some("还没设置联网搜索".into());
            *needs_setup = None;
        }
        let mut feed = Feed::default();
        feed.apply(&text_only);
        assert!(
            matches!(
                feed.rows().next(),
                Some(Row::Delivery(DeliveryCard {
                    needs_search_setup: false,
                    ..
                }))
            ),
            "no 去设置 without needsSetup"
        );

        let mut model_setup = ev.clone();
        if let Event::Delivery {
            failure,
            needs_setup,
            ..
        } = &mut model_setup
        {
            *failure = None;
            *needs_setup = Some("model".into());
        }
        let mut feed = Feed::default();
        feed.apply(&model_setup);
        assert!(
            matches!(
                feed.rows().next(),
                Some(Row::Delivery(DeliveryCard {
                    needs_model_setup: true,
                    needs_search_setup: false,
                    failure: None,
                    ..
                }))
            ),
            "去设置 when needsSetup is model"
        );

        let old_status: Event = serde_json::from_value(serde_json::json!({
            "type": "status", "id": "t1", "taskId": "t1", "botId": "b", "botName": "Ada",
            "state": "done", "label": "完成", "createdAt": "2026-10-07T19:00:00.000Z",
            "updatedAt": "2026-10-07T19:00:00.000Z", "containerUp": false
        }))
        .unwrap();
        let mut feed = Feed::default();
        feed.apply(&old_status);
        assert_eq!(
            actual(status_cards(&feed)[0]),
            expected(TaskState::Done, false)
        );
    }

    fn blocked_waiting(waiting: Option<WaitingFor>) -> Event {
        let mut ev = status("t5", "Ada", TaskState::Blocked, false);
        if let Event::Status { waiting_for, .. } = &mut ev {
            *waiting_for = waiting;
        }
        ev
    }

    #[test]
    fn waiting_for_approval_is_amber_without_take_over() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        let ev = blocked_waiting(Some(WaitingFor::Approval));
        let mut feed = Feed::default();
        feed.apply(&ev);
        let card = status_cards(&feed)[0];
        assert_eq!(card.status_line, "等你批准");
        assert_eq!(card.tone, Tone::Amber, "blocked yellow, not dead-run grey");
        assert_eq!(card.palette, expected(TaskState::Blocked, false).2);
        assert_eq!(
            card.action, None,
            "the approval card is the only way to act"
        );
        assert_eq!(preview_of(&ev, Some("u-me"), Some("u-me")), "Ada：等你批准");

        // Real takeovers (captcha) and older hosts without the field keep amber.
        for waiting in [Some(WaitingFor::Takeover), None] {
            let mut feed = Feed::default();
            feed.apply(&blocked_waiting(waiting));
            assert_eq!(
                actual(status_cards(&feed)[0]),
                expected(TaskState::Blocked, false),
                "{waiting:?}"
            );
            assert_eq!(status_cards(&feed)[0].action, Some("去私聊接管"));
        }
        // Once answered the next status has no waitingFor and updates in place.
        feed.apply(&status("t5", "Ada", TaskState::Queued, false));
        assert_eq!(feed.len(), 1);
        assert_eq!(
            actual(status_cards(&feed)[0]),
            expected(TaskState::Queued, false)
        );
    }

    #[test]
    fn awaiting_approval_keeps_the_host_progress_line() {
        let _zh = robroz_ui::i18n::scoped(robroz_ui::i18n::Locale::Zh);
        let ev = with_progress(
            blocked_waiting(Some(WaitingFor::Approval)),
            Some("在用 notes.write_note"),
        );
        let mut feed = Feed::default();
        feed.apply(&ev);
        let card = status_cards(&feed)[0];
        assert_eq!(card.status_line, "等你批准");
        assert_eq!(card.action, None);
        assert_eq!(zh(&card.progress).as_deref(), Some("在用 notes.write_note"));
        // The owner's list preview stays 等你批准; the progress line is
        // on the card. A non-owner sees the card sentence, tested below.
        assert_eq!(preview_of(&ev, Some("u-me"), Some("u-me")), "Ada：等你批准");
        // A stuck takeover still has no progress line.
        let stuck = with_progress(
            blocked_waiting(Some(WaitingFor::Takeover)),
            Some("在用 notes.write_note"),
        );
        assert_eq!(
            match to_row(&stuck, None).1 {
                Row::Status(c) => c.progress,
                _ => unreachable!(),
            },
            None
        );
    }

    #[test]
    fn awaiting_approval_preview_follows_the_viewer() {
        use robroz_ui::i18n::{scoped, Locale};
        let ev = blocked_waiting(Some(WaitingFor::Approval));
        let owner = "u-me";
        {
            let _zh = scoped(Locale::Zh);
            assert_eq!(preview_of(&ev, Some(owner), Some(owner)), "Ada：等你批准");
            assert_eq!(
                preview_of(&ev, Some("u-lin"), Some(owner)),
                "Ada：等这个 bro 的主人审批"
            );
            assert_eq!(
                preview_of(&ev, None, Some(owner)),
                "Ada：等这个 bro 的主人审批",
                "unknown me is not the owner, same as the card"
            );
            // A missing owner is local-admin, not a non-owner.
            assert_eq!(
                preview_of(&ev, Some(robroz_contract::BOOTSTRAP_ADMIN_ID), None),
                "Ada：等你批准"
            );
            assert_eq!(
                preview_of(&ev, Some("u-lin"), None),
                "Ada：等这个 bro 的主人审批"
            );
            assert_eq!(preview_of(&ev, None, None), "Ada：等这个 bro 的主人审批");
            assert_eq!(preview(&ev), "Ada：等这个 bro 的主人审批");
        }
        {
            let _en = scoped(Locale::En);
            assert_eq!(
                preview_of(&ev, Some(owner), Some(owner)),
                "Ada: Needs your approval"
            );
            assert_eq!(
                preview_of(&ev, Some("u-lin"), Some(owner)),
                "Ada: Waiting for this bro's owner"
            );
            assert_eq!(
                preview_of(&ev, Some(robroz_contract::BOOTSTRAP_ADMIN_ID), None),
                "Ada: Needs your approval"
            );
            assert_eq!(
                preview_of(&ev, Some("u-lin"), None),
                "Ada: Waiting for this bro's owner"
            );
            assert_eq!(
                preview_of(&ev, None, None),
                "Ada: Waiting for this bro's owner"
            );
        }
    }

    #[test]
    fn takeover_to_approval_replaces_the_same_card_in_place() {
        // First poll (list) says takeover, then SSE says approval for the
        // same task: one card, updated where it was.
        let mut feed = Feed::default();
        feed.apply(&text("m1", "帮我发周报"));
        feed.apply(&blocked_waiting(Some(WaitingFor::Takeover)));
        feed.apply(&text("m2", "之后"));
        feed.apply(&blocked_waiting(Some(WaitingFor::Approval)));
        feed.apply(&blocked_waiting(Some(WaitingFor::Approval)));
        assert_eq!(feed.len(), 3, "one status card, not two");
        let cards = status_cards(&feed);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].status_line, "等你批准");
        assert_eq!(cards[0].action, None);
        assert!(
            matches!(feed.rows().nth(1), Some(Row::Status(_))),
            "kept its slot"
        );
        // And back: approval -> takeover (card answered elsewhere, captcha next).
        feed.apply(&blocked_waiting(Some(WaitingFor::Takeover)));
        assert_eq!(status_cards(&feed).len(), 1);
        assert_eq!(status_cards(&feed)[0].action, Some("去私聊接管"));
    }

    #[test]
    fn awaiting_status_beats_its_approval_card_in_the_preview() {
        let at = "2026-11-20T10:29:00.000Z";
        let status = blocked_waiting(Some(WaitingFor::Approval));
        assert_eq!(preview_rank(&status), 2);
        let card = (at, 1); // unanswered approval created at the same instant
        assert!(!preview_wins(card, Some((at, preview_rank(&status)))));
        assert!(preview_wins((at, preview_rank(&status)), Some(card)));
        assert_eq!(
            preview_rank(&blocked_waiting(Some(WaitingFor::Takeover))),
            1
        );
    }

    #[test]
    fn waiting_for_from_host_json() {
        let json = |extra: serde_json::Value| {
            let mut v = serde_json::json!({
                "type": "status", "id": "t1", "taskId": "t1", "botId": "b", "botName": "Ada",
                "state": "blocked", "label": "卡住", "createdAt": "2026-11-20T02:00:01.000Z",
                "updatedAt": "2026-11-20T02:00:02.000Z", "containerUp": false
            });
            v.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            serde_json::from_value::<Event>(v).unwrap()
        };
        let line = |ev: Event| {
            let mut feed = Feed::default();
            feed.apply(&ev);
            status_cards(&feed)[0].status_line.clone()
        };
        assert_eq!(
            line(json(serde_json::json!({"waitingFor": "approval"}))),
            "等你批准"
        );
        assert_eq!(
            line(json(serde_json::json!({"waitingFor": "takeover"}))),
            "卡住"
        );
        assert_eq!(
            line(json(serde_json::json!({}))),
            "卡住",
            "older host: unchanged"
        );
    }

    #[test]
    fn list_preview_prefers_newest_then_outcome_over_answer() {
        let deny_at = "2026-11-20T10:10:07.502Z";
        let failed = ("2026-11-20T10:10:07.503Z", 2);
        let answered = (deny_at, 0);
        // Host writes the answer first, then the task outcome: outcome wins
        // whichever order the two events arrive in.
        assert!(preview_wins(failed, Some(answered)));
        assert!(!preview_wins(answered, Some(failed)));
        // Same millisecond: the outcome still wins the tie.
        assert!(!preview_wins((deny_at, 0), Some((deny_at, 2))));
        assert!(preview_wins((deny_at, 2), Some((deny_at, 0))));
        // A newer message always takes over; equal rank and time: last wins.
        assert!(preview_wins(
            ("2026-11-20T10:11:00Z", 1),
            Some((deny_at, 2))
        ));
        assert!(preview_wins((deny_at, 1), Some((deny_at, 1))));
        assert!(preview_wins((deny_at, 1), None));
    }

    #[test]
    fn preview_rank_orders_outcome_message_answer() {
        let mut failed = status("t6", "Ada", TaskState::Done, false);
        if let Event::Status { failure, .. } = &mut failed {
            *failure = Some("你拒绝了：发送周报".into());
        }
        assert_eq!(preview_rank(&failed), 2);
        assert_eq!(preview_rank(&text("m1", "hi")), 1);
        assert_eq!(
            preview_rank(&status("t6", "Ada", TaskState::Done, false)),
            1
        );
    }

    #[test]
    fn blocked_has_only_take_over_action() {
        let mut feed = Feed::default();
        feed.apply(&status("t3", "Bea", TaskState::Blocked, false));
        let card = status_cards(&feed)[0];
        assert_eq!(actual(card), expected(TaskState::Blocked, false));
        assert_eq!(card.action, Some("去私聊接管"));
    }

    #[test]
    fn briefing_delivery_carries_badge_and_raw_path() {
        let mut feed = Feed::default();
        feed.apply(&delivery("d1", "tasks/t1/result.txt", false));
        feed.apply(&delivery("d2", "tasks/t-brief/result.txt", true));
        let cards: Vec<_> = feed
            .rows()
            .filter_map(|r| match r {
                Row::Delivery(c) => Some(c),
                _ => None,
            })
            .collect();
        assert_eq!(cards[0].badge, None);
        assert_eq!(cards[0].path, "tasks/t1/result.txt");
        assert_eq!(cards[1].badge, Some(robroz_card::briefing_badge()));
        assert_eq!(cards[1].path, "tasks/t-brief/result.txt");
    }

    #[test]
    fn reapplying_a_snapshot_is_idempotent() {
        let events = vec![
            text("m1", "hi"),
            status("t1", "Ada", TaskState::Done, false),
            delivery("d1", "tasks/t1/result.txt", false),
        ];
        let mut feed = Feed::default();
        for e in &events {
            feed.apply(e);
        }
        let before: Vec<Row> = feed.rows().cloned().collect();
        for e in &events {
            feed.apply(e);
        }
        assert_eq!(feed.rows().cloned().collect::<Vec<_>>(), before);
    }

    #[test]
    fn replace_from_snapshot_swaps_atomically_and_keeps_local_timer() {
        let mut feed = Feed::default();
        feed.apply(&status("old", "Ada", TaskState::Done, false));
        feed.apply(&text("m1", "hello"));
        let mut live = status("keep", "Ada", TaskState::Queued, false);
        feed.apply(&live);
        if let Event::Status {
            state,
            container_up,
            ..
        } = &mut live
        {
            *state = TaskState::Running;
            *container_up = true;
        }
        feed.apply(&live);
        let before = status_cards(&feed)
            .iter()
            .find(|c| c.task_id == "keep")
            .unwrap()
            .local_started;
        assert!(before.is_some());

        let mut ev = status("keep", "Ada", TaskState::Running, true);
        if let Event::Status { started_at, .. } = &mut ev {
            *started_at = Some("2026-10-07T19:00:02.000Z".into());
        }
        let gone_is_dropped = text("m2", "new");
        feed.replace_from_snapshot(&[(ev, None), (gone_is_dropped, None)]);
        let ids: Vec<_> = feed
            .rows()
            .map(|r| match r {
                Row::Status(c) => format!("s:{}", c.task_id),
                Row::Text { body } => format!("t:{body}"),
                _ => "other".into(),
            })
            .collect();
        assert_eq!(
            ids,
            vec!["s:keep".to_string(), "t:new".to_string()],
            "{ids:?}"
        );
        let card = status_cards(&feed)[0];
        assert_eq!(card.task_id, "keep");
        assert_eq!(card.local_started, before, "session timer kept");
        assert_eq!(card.started_at.as_deref(), Some("2026-10-07T19:00:02.000Z"));
    }

    #[test]
    fn attachments_are_labelled_rows() {
        let mut feed = Feed::default();
        feed.apply(&Event::Message {
            id: "m9".into(),
            kind: MessageKind::Image,
            body: "hero.png".into(),
            created_at: "2026-10-07T19:00:00.000Z".into(),
            author_name: None,
            author_user_id: None,
        });
        assert_eq!(
            feed.rows().next(),
            Some(&Row::Attachment {
                kind: MessageKind::Image,
                name: "hero.png".into()
            })
        );
    }

    #[test]
    fn answered_choice_replaces_open_card_in_place() {
        let card = |choice: Option<&str>| Event::Choice {
            id: "c1".into(),
            bot_id: "bot-ada".into(),
            bot_name: "Ada".into(),
            prompt: "用哪个？".into(),
            options: vec![
                robroz_contract::ChoiceOption {
                    id: "a".into(),
                    label: "A".into(),
                    primary: true,
                },
                robroz_contract::ChoiceOption {
                    id: "b".into(),
                    label: "B".into(),
                    primary: false,
                },
            ],
            multi: false,
            choice: choice.map(|v| vec![v.to_string()]),
            created_at: "2026-10-03T19:00:00.000Z".into(),
            answered_at: choice.map(|_| "2026-10-03T19:01:00.000Z".into()),
            answered_by: None,
        };
        let mut feed = Feed::default();
        feed.apply(&card(None));
        feed.apply(&card(Some("b")));
        assert_eq!(feed.len(), 1);
        let row = feed.rows().next().unwrap().clone();
        match row {
            Row::Card(Card::Choice(c)) => {
                assert!(matches!(
                    c.state,
                    robroz_ui::cards::ChoiceState::Answered { ref selected, .. } if selected == &["b"]
                ));
                assert_eq!(c.options.len(), 2);
            }
            other => panic!("expected choice row, got {other:?}"),
        }
    }

    #[test]
    fn approval_and_draft_cards_update_in_place_by_id() {
        use robroz_ui::cards::{ActionKind, ApprovalCard, ApprovalState};
        let card = |state| {
            Card::Approval(ApprovalCard {
                id: "a1".into(),
                bot: "后端".into(),
                kind: ActionKind::Delete,
                summary: "清理 12 个旧日志文件".into(),
                path: Some("logs/2026-10/".into()),
                state,
                mcp_server: None,
                mcp_tool: None,
                reason: None,
                reason_code: None,
                owner_id: None,
                owner_name: None,
            })
        };
        let mut feed = Feed::default();
        feed.apply(&text("m1", "先问一下"));
        feed.apply_card(card(ApprovalState::Pending));
        feed.apply(&text("m2", "后面的话"));
        feed.apply_card(card(ApprovalState::Approved { by: None, at: 0 }));
        assert_eq!(feed.len(), 3, "one approval card, not two");
        assert!(matches!(
            feed.card("a1"),
            Some(Card::Approval(ApprovalCard {
                state: ApprovalState::Approved { .. },
                ..
            }))
        ));
        assert!(
            matches!(feed.rows().nth(1), Some(Row::Card(_))),
            "kept its slot"
        );
    }

    #[test]
    fn settings_approval_is_kept_out_of_the_feed() {
        let event = Event::Approval {
            reason_code: None,
            id: "apr-settings".into(),
            bot_id: robroz_contract::SETTINGS_APPROVAL_BOT_ID.into(),
            bot_name: robroz_contract::SETTINGS_APPROVAL_BOT_NAME.into(),
            action: robroz_contract::ApprovalAction::RuleAdd,
            summary: "允许发周报".into(),
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
        let mut feed = Feed::default();
        feed.apply(&text("m1", "打招呼"));
        feed.apply(&event);
        assert_eq!(feed.len(), 1);
        assert!(feed.card("apr-settings").is_none());
    }

    #[test]
    fn previews_redraw_in_the_new_language() {
        use robroz_ui::i18n::{scoped, Locale};
        let status: Event = serde_json::from_value(serde_json::json!({
            "type": "status", "id": "t1", "taskId": "t1", "botId": "b", "botName": "Ada",
            "state": "queued", "label": "排队", "createdAt": "2026-10-07T19:00:00.000Z",
            "updatedAt": "2026-10-07T19:00:00.000Z", "containerUp": false
        }))
        .unwrap();
        let mut previews = Previews::default();
        let shown = {
            let _en = scoped(Locale::En);
            previews.offer("g1", &status, None, None).unwrap()
        };
        let _zh = scoped(Locale::Zh);
        let redrawn: Vec<_> = previews.redraw(None).collect();
        assert_ne!(redrawn[0].1, shown);
        assert_eq!(redrawn, vec![("g1", preview(&status))]);
        // Same time and rank: the newer arrival still wins.
        assert!(
            previews.offer("g1", &status, None, None).is_some(),
            "tie, same rank"
        );
    }

    // ---- chat bubbles: who sent it, and grouping ----

    const T0: i64 = 1_790_000_000;

    fn own(at: Option<i64>) -> Meta {
        Meta {
            sender: Sender::Own,
            at,
        }
    }

    fn by(name: &str, at: Option<i64>) -> Meta {
        Meta {
            sender: bro(&format!("bot-{name}"), name),
            at,
        }
    }

    fn sizes(groups: &[Group<&str>]) -> Vec<usize> {
        groups.iter().map(|g| g.items.len()).collect()
    }

    #[test]
    fn same_sender_within_two_minutes_is_one_group() {
        let g = group_items(
            [
                ("a", own(Some(T0))),
                ("b", own(Some(T0 + 60))),
                ("c", own(Some(T0 + 60 + 120))),
            ],
            120,
        );
        assert_eq!(sizes(&g), vec![3], "exactly 2 minutes still groups");
        assert_eq!(g[0].last_at, Some(T0 + 180));
    }

    #[test]
    fn more_than_two_minutes_starts_a_new_group() {
        let g = group_items([("a", own(Some(T0))), ("b", own(Some(T0 + 121)))], 120);
        assert_eq!(sizes(&g), vec![1, 1]);
        // The gap is to the previous item, not to the group's first.
        let g = group_items(
            [
                ("a", own(Some(T0))),
                ("b", own(Some(T0 + 100))),
                ("c", own(Some(T0 + 200))),
            ],
            120,
        );
        assert_eq!(sizes(&g), vec![3]);
    }

    #[test]
    fn a_different_sender_starts_a_new_group() {
        let g = group_items(
            [
                ("a", own(Some(T0))),
                ("b", by("Ada", Some(T0 + 1))),
                ("c", by("Ada", Some(T0 + 2))),
                ("d", by("Bea", Some(T0 + 3))),
                ("e", own(Some(T0 + 4))),
            ],
            120,
        );
        assert_eq!(sizes(&g), vec![1, 2, 1, 1]);
        let sides: Vec<bool> = g.iter().map(|g| g.sender.is_own()).collect();
        assert_eq!(sides, vec![true, false, false, true]);
        assert_eq!(g[1].sender.name(), "Ada");
    }

    #[test]
    fn items_without_a_time_join_the_group_of_their_sender() {
        let g = group_items(
            [
                ("a", by("Ada", Some(T0))),
                ("card", by("Ada", None)),
                ("b", by("Ada", Some(T0 + 30))),
            ],
            120,
        );
        assert_eq!(sizes(&g), vec![3]);
        assert_eq!(g[0].last_at, Some(T0 + 30));
    }

    #[test]
    fn a_bro_is_matched_by_id_then_by_name() {
        assert!(bro("b1", "Ada").same(&bro("b1", "Ada 2")));
        assert!(!bro("b1", "Ada").same(&bro("b2", "Ada")));
        assert!(
            bro("", "Ada").same(&bro("b1", "Ada")),
            "client card: by name"
        );
        assert!(!Sender::Own.same(&bro("", "")));
    }

    #[test]
    fn messages_are_own_and_bro_events_name_their_bro() {
        let mut feed = Feed::default();
        feed.apply(&text("m1", "hello"));
        feed.apply(&status("t1", "Ada", TaskState::Running, true));
        feed.apply(&delivery("d1", "tasks/t1/result.txt", false));
        feed.apply(&status("t2", "Bea", TaskState::Queued, false));
        let items: Vec<_> = feed.items().collect();
        assert_eq!(items[0].1.sender, Sender::Own);
        assert_eq!(items[1].1.sender.name(), "Ada");
        assert!(
            items[2].1.sender.is_delivery(),
            "a delivery without bot_name stands alone"
        );
        assert_eq!(items[3].1.sender.name(), "Bea");
        assert_eq!(items[0].1.at, Some(1_791_399_600));
        let groups = group_items(items.iter().map(|(r, m)| (*r, (*m).clone())), 120);
        assert_eq!(
            groups.len(),
            4,
            "a delivery without bot_name starts its own group"
        );
        assert!(groups[2].sender.is_delivery());
        assert!(!Sender::Delivery.same(&bro("bot-ada", "Ada")));
        assert!(!Sender::Delivery.same(&Sender::Delivery));
    }

    fn reminder(state: robroz_contract::ReminderState, answered: Option<&str>) -> Event {
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
            created_at: "2026-10-07T19:00:00.000Z".into(),
            answered_at: answered.map(Into::into),
        }
    }

    #[test]
    fn a_reminder_is_one_row_that_updates_in_place_and_stands_alone() {
        use robroz_contract::ReminderState;
        let mut feed = Feed::default();
        feed.apply(&text("m1", "hi"));
        feed.apply(&reminder(ReminderState::Waiting, None));
        feed.apply(&text("m2", "again"));
        feed.apply(&reminder(
            ReminderState::Approved,
            Some("2026-10-07T19:02:00.000Z"),
        ));
        assert_eq!(feed.len(), 3, "the same reminder id, not a second row");
        let items: Vec<_> = feed.items().collect();
        match items[2].0 {
            Row::Reminder(r) => {
                assert_eq!(r.state, robroz_contract::ReminderState::Approved);
                assert_eq!(r.group_id, "g1");
                assert_eq!(r.approval_id, "a1");
            }
            other => panic!("expected a reminder row last, got {other:?}"),
        }
        assert_eq!(items[2].1.sender, Sender::Reminder);
        // The two own messages now sit together; the reminder stands alone last.
        let groups = group_items(items.iter().map(|(r, m)| (*r, (*m).clone())), 120);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].items.len(), 2);
        assert!(groups[1].sender.is_reminder());
        assert!(!Sender::Reminder.same(&Sender::Reminder));
    }

    #[test]
    fn reminders_sit_in_chronological_order_by_the_time_they_show() {
        use robroz_contract::ReminderState;
        fn at(id: &str, state: ReminderState, created: &str, answered: Option<&str>) -> Event {
            let mut ev = reminder(state, answered);
            if let Event::ApprovalReminder {
                id: ev_id,
                created_at,
                answered_at,
                approval_id,
                ..
            } = &mut ev
            {
                *ev_id = format!("reminder:{id}");
                *approval_id = id.into();
                *created_at = created.into();
                *answered_at = answered.map(Into::into);
            }
            ev
        }
        let mut feed = Feed::default();
        // Applied newest-first on purpose: the approved row must still
        // land after the waiting one, because 19:02 is later than 18:50.
        feed.apply(&at(
            "done",
            ReminderState::Approved,
            "2026-10-07T18:50:00.000Z",
            Some("2026-10-07T19:02:00.000Z"),
        ));
        feed.apply(&at(
            "wait",
            ReminderState::Waiting,
            "2026-10-07T18:50:00.000Z",
            None,
        ));
        let items: Vec<_> = feed.items().collect();
        assert_eq!(items.len(), 2);
        match items[0].0 {
            Row::Reminder(r) => assert_eq!(r.state, ReminderState::Waiting),
            other => panic!("waiting first, got {other:?}"),
        }
        match items[1].0 {
            Row::Reminder(r) => assert_eq!(r.state, ReminderState::Approved),
            other => panic!("approved second, got {other:?}"),
        }
        assert_eq!(items[0].1.at, Some(1_791_399_000)); // 18:50Z
        assert_eq!(items[1].1.at, Some(1_791_399_720)); // 19:02Z
                                                        // Updating the waiting row to approved at 19:05 moves it below.
        feed.apply(&at(
            "wait",
            ReminderState::Approved,
            "2026-10-07T18:50:00.000Z",
            Some("2026-10-07T19:05:00.000Z"),
        ));
        let items: Vec<_> = feed.items().collect();
        assert_eq!(items.len(), 2);
        match items[0].0 {
            Row::Reminder(r) => assert_eq!(r.state, ReminderState::Approved),
            other => panic!("19:02 stays first, got {other:?}"),
        }
        match items[1].0 {
            Row::Reminder(r) => assert_eq!(r.answered_at, Some(1_791_399_900)),
            other => panic!("19:05 moves last, got {other:?}"),
        }
    }

    #[test]
    fn a_failure_code_drives_the_preview_and_falls_back_without_one() {
        use robroz_contract::{CardStatusCode, FailureCode};
        use robroz_ui::i18n::{scoped, Locale};
        let mut ev = status("t1", "Ada", TaskState::Done, false);
        if let Event::Status {
            failure,
            failure_code,
            failure_detail,
            status_code,
            ..
        } = &mut ev
        {
            *failure = Some("模型没回：模型服务返回 401".into());
            *failure_code = Some(FailureCode::ModelNoReply);
            *failure_detail = Some("401".into());
            *status_code = Some(CardStatusCode::Done);
        }
        {
            let _g = scoped(Locale::Zh);
            assert_eq!(preview(&ev), "Ada：没做成：模型没回（HTTP 401）");
            assert!(!preview(&ev).contains("模型服务返回"));
        }
        {
            let _g = scoped(Locale::En);
            assert_eq!(
                preview(&ev),
                "Ada: Couldn't finish: the model didn't reply (HTTP 401)"
            );
        }
        // A non-numeric detail is not in the preview; it is the card's
        // detail line.
        if let Event::Status { failure_detail, .. } = &mut ev {
            *failure_detail = Some("response_not_json".into());
        }
        {
            let _g = scoped(Locale::Zh);
            assert_eq!(preview(&ev), "Ada：没做成：模型没回");
        }
        let mut feed = Feed::default();
        feed.apply(&ev);
        assert_eq!(
            status_cards(&feed)[0].detail_line().as_deref(),
            Some("response_not_json")
        );
        // No code: the unknown-code copy, the host text is never shown.
        let mut old = status("t2", "Ada", TaskState::Done, false);
        if let Event::Status { failure, .. } = &mut old {
            *failure = Some("模型服务返回 401".into());
        }
        let _g = scoped(Locale::Zh);
        assert_eq!(preview(&old), "Ada：出错了");
        let _g = scoped(Locale::En);
        assert_eq!(preview(&old), "Ada: Something went wrong");
    }

    #[test]
    fn a_reminder_previews_as_its_sentence() {
        use robroz_contract::ReminderState;
        use robroz_ui::i18n::{scoped, Locale};
        let ev = reminder(ReminderState::Waiting, None);
        let _en = scoped(Locale::En);
        assert_eq!(
            preview_of(&ev, Some("u-me"), None),
            "Ada is waiting for your approval in Site"
        );
        assert_eq!(
            preview_of(&ev, Some("u-lin"), None),
            "Waiting for this bro's owner"
        );
        assert_eq!(change_time(&ev), "2026-10-07T19:00:00.000Z");
        let done = reminder(ReminderState::Denied, Some("2026-10-07T19:05:00.000Z"));
        assert_eq!(change_time(&done), "2026-10-07T19:05:00.000Z");
    }

    #[test]
    fn a_delivery_with_bot_name_joins_that_bro() {
        let mut feed = Feed::default();
        feed.apply(&status("t1", "Ada", TaskState::Done, false));
        feed.apply(&delivery_from(
            "d1",
            "tasks/t1/result.txt",
            false,
            "2026-10-07T19:00:01.000Z",
            Some("bot-Ada"),
            Some("Ada"),
        ));
        let items: Vec<_> = feed.items().collect();
        assert_eq!(items[1].1.sender, bro("bot-Ada", "Ada"));
        let groups = group_items(items.iter().map(|(r, m)| (*r, (*m).clone())), 120);
        assert_eq!(
            groups[0].items.len(),
            2,
            "status and delivery are one group"
        );
        assert_eq!(groups[0].sender.name(), "Ada");
    }

    #[test]
    fn a_blank_bot_name_is_treated_as_missing() {
        let mut feed = Feed::default();
        feed.apply(&delivery_from(
            "d0",
            "tasks/t0/result.txt",
            false,
            "2026-10-07T19:00:00.000Z",
            Some("bot-ada"),
            Some("  "),
        ));
        let items: Vec<_> = feed.items().collect();
        assert!(items[0].1.sender.is_delivery());
    }

    #[test]
    fn two_bros_delivering_interleaved_stay_with_their_own_name() {
        // Ada, Bea, Ada, then an unattributed card, all within two minutes.
        let mut feed = Feed::default();
        feed.apply(&delivery_from(
            "d-ada-1",
            "tasks/t-ada/a.txt",
            false,
            "2026-10-07T19:00:00.000Z",
            Some("bot-ada"),
            Some("Ada"),
        ));
        feed.apply(&delivery_from(
            "d-bea",
            "tasks/t-bea/b.txt",
            false,
            "2026-10-07T19:00:00.000Z",
            Some("bot-bea"),
            Some("Bea"),
        ));
        feed.apply(&delivery_from(
            "d-ada-2",
            "tasks/t-ada/c.txt",
            false,
            "2026-10-07T19:00:01.000Z",
            Some("bot-ada"),
            Some("Ada"),
        ));
        feed.apply(&delivery_from(
            "d-orphan",
            "tasks/orphan/o.txt",
            false,
            "2026-10-07T19:00:01.000Z",
            None,
            None,
        ));
        let items: Vec<_> = feed.items().collect();
        let groups = group_items(items.iter().map(|(r, m)| (*r, (*m).clone())), 120);
        let names: Vec<&str> = groups
            .iter()
            .map(|g| {
                if g.sender.is_delivery() {
                    ""
                } else {
                    g.sender.name()
                }
            })
            .collect();
        assert_eq!(names, vec!["Ada", "Bea", "Ada", ""]);
        assert_eq!(
            groups.iter().map(|g| g.items.len()).collect::<Vec<_>>(),
            vec![1, 1, 1, 1]
        );
        assert!(groups[3].sender.is_delivery());
        // The unattributed card does not join the Ada group after it, either.
        feed.apply(&delivery_from(
            "d-ada-3",
            "tasks/t-ada/d.txt",
            false,
            "2026-10-07T19:00:02.000Z",
            Some("bot-ada"),
            Some("Ada"),
        ));
        let items: Vec<_> = feed.items().collect();
        let groups = group_items(items.iter().map(|(r, m)| (*r, (*m).clone())), 120);
        assert_eq!(
            groups.iter().map(|g| g.sender.name()).collect::<Vec<_>>(),
            vec!["Ada", "Bea", "Ada", "", "Ada"]
        );
    }

    #[test]
    fn an_updated_card_keeps_its_place_and_time() {
        let mut feed = Feed::default();
        feed.apply(&status("t1", "Ada", TaskState::Running, true));
        feed.apply(&text("m1", "later"));
        feed.apply(&status("t1", "Ada", TaskState::Done, false));
        let items: Vec<_> = feed.items().collect();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0].0, Row::Status(_)));
        assert_eq!(items[0].1.sender.name(), "Ada");
    }
}
