//! The status card for one task, from a contract `Event::Status`.
//!
//! This mirrors the desktop rules in `client/src/model.rs` without iced:
//! colours, status words and actions come from `robroz-card`; a task with a
//! `failure` is grey with the failure line, never the green "done"; a task
//! waiting on an approval is grey too; a live Router step (`n/m 步`) goes
//! left of the status word, other progress under the bro name. Lines are
//! kept as the host sent them and translated when drawn, so a language
//! switch redraws a card already on screen.

use robroz_card::{Palette, Tone};
use robroz_contract::host_text::parse_router_step_progress;
use robroz_contract::{Event, TaskState, WaitingFor};
use robroz_ui::i18n::{host_text, Key};

/// The status line while a task waits on an approval card. Same constant as
/// the desktop client; `display` translates it.
pub const AWAITING_APPROVAL: &str = "等你批准";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCard {
    pub bot_id: String,
    pub bot_name: String,
    /// `createdAt`, Unix seconds, when the host sent one.
    pub at: Option<i64>,
    /// Raw `robroz-card` copy, or 「没做成：…」 for a failure.
    pub status_line: String,
    pub tone: Tone,
    pub palette: Palette,
    /// What the bro is doing now, under the bro name.
    pub progress: Option<String>,
    /// Router `n/m 步`, left of the status word, live runs only.
    pub step_progress: Option<String>,
}

/// A status-card line in the active language; unknown text passes through.
pub fn display(s: &str) -> std::borrow::Cow<'_, str> {
    if s == AWAITING_APPROVAL {
        std::borrow::Cow::Borrowed(Key::ClientAwaitingApproval.as_str())
    } else {
        host_text(s)
    }
}

fn awaits_approval(state: TaskState, waiting_for: Option<WaitingFor>) -> bool {
    state == TaskState::Blocked && waiting_for == Some(WaitingFor::Approval)
}

/// Progress is shown only on a live run or while waiting on an approval,
/// never next to a finished or failed card. Whitespace collapses.
fn progress_line(
    state: TaskState,
    container_up: bool,
    failure: Option<&str>,
    waiting_for: Option<WaitingFor>,
    progress: &Option<String>,
) -> Option<String> {
    let live = state == TaskState::Running && container_up;
    if failure.is_some() || !(live || awaits_approval(state, waiting_for)) {
        return None;
    }
    let line = progress
        .as_deref()?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!line.is_empty()).then_some(line)
}

impl StatusCard {
    /// `None` for any event that is not a status card.
    pub fn from_event(event: &Event) -> Option<Self> {
        let Event::Status {
            bot_id,
            bot_name,
            created_at,
            state,
            container_up,
            failure,
            progress,
            waiting_for,
            ..
        } = event
        else {
            return None;
        };
        let (look, status_line) = match failure {
            // A failed task is grey, like a dead run.
            Some(f) => (
                robroz_card::appearance(TaskState::Running, false),
                robroz_card::failure_line(f),
            ),
            None if awaits_approval(*state, *waiting_for) => (
                robroz_card::appearance(TaskState::Running, false),
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
            progress,
        );
        let (progress, step_progress) = match shown {
            Some(line)
                if *state == TaskState::Running
                    && *container_up
                    && parse_router_step_progress(&line).is_some() =>
            {
                (None, Some(line))
            }
            other => (other, None),
        };
        Some(Self {
            bot_id: bot_id.clone(),
            bot_name: bot_name.clone(),
            at: robroz_ui::runs::parse_unix(created_at),
            status_line,
            tone: look.tone,
            palette: robroz_card::palette(look.tone),
            progress,
            step_progress,
        })
    }
}

/// The newest status card in a group's events: the one with the latest
/// `updatedAt` (RFC 3339 with millis, so text order is time order).
pub fn latest_status(events: &[Event]) -> Option<&Event> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Status { updated_at, .. } => Some((updated_at.as_str(), e)),
            _ => None,
        })
        .max_by(|a, b| a.0.cmp(b.0))
        .map(|(_, e)| e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use robroz_ui::i18n::{scoped, Locale};

    fn status(state: TaskState, container_up: bool) -> Event {
        Event::Status {
            progress_code: None,
            progress_params: None,
            id: "e1".into(),
            task_id: "t1".into(),
            bot_id: "b1".into(),
            bot_name: "Ada".into(),
            state,
            label: String::new(),
            created_at: "2026-11-20T10:00:00.000Z".into(),
            updated_at: "2026-11-20T10:00:05.000Z".into(),
            container_up,
            failure: None,
            waiting_for: None,
            progress: None,
            status_code: None,
            failure_code: None,
            failure_detail: None,
        }
    }

    fn set(ev: &mut Event, f: impl FnOnce(&mut Option<String>, &mut Option<String>)) {
        if let Event::Status {
            failure, progress, ..
        } = ev
        {
            f(failure, progress);
        }
    }

    #[test]
    fn failed_task_is_grey_with_the_failure_line_never_green_done() {
        let mut ev = status(TaskState::Done, false);
        set(&mut ev, |f, _| {
            *f = Some("模型没回：模型服务返回 500".into())
        }); // the variable part stays as the host sent it
        let card = StatusCard::from_event(&ev).unwrap();
        assert_eq!(card.tone, Tone::Gray);
        assert_ne!(card.palette, robroz_card::palette(Tone::Green));
        let _en = scoped(Locale::En);
        assert_eq!(
            display(&card.status_line),
            "Couldn't finish: No reply from the model: 模型服务返回 500"
        );
    }

    #[test]
    fn live_router_step_goes_left_of_the_status_word() {
        let mut ev = status(TaskState::Running, true);
        set(&mut ev, |_, p| {
            *p = Some(robroz_contract::host_text::router_step_progress(2, 3))
        });
        let card = StatusCard::from_event(&ev).unwrap();
        assert_eq!(card.tone, Tone::Ink);
        assert_eq!(card.progress, None);
        let _en = scoped(Locale::En);
        assert_eq!(
            display(card.step_progress.as_deref().unwrap()),
            "Step 2 of 3"
        );
        assert_eq!(display(&card.status_line), "Running");
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(display(card.step_progress.as_deref().unwrap()), "第 2/3 步");
        assert_eq!(display(&card.status_line), "进行中");
    }

    #[test]
    fn other_progress_stays_under_the_name_and_dead_runs_show_none() {
        let mut ev = status(TaskState::Running, true);
        set(&mut ev, |_, p| *p = Some("在查  资料".into()));
        let card = StatusCard::from_event(&ev).unwrap();
        assert_eq!(card.progress.as_deref(), Some("在查 资料"));
        assert_eq!(card.step_progress, None);

        let mut dead = status(TaskState::Running, false);
        set(&mut dead, |_, p| {
            *p = Some(robroz_contract::host_text::router_step_progress(1, 2))
        });
        let card = StatusCard::from_event(&dead).unwrap();
        assert_eq!((card.progress, card.step_progress), (None, None));
        assert_eq!(card.tone, Tone::Gray);
    }

    #[test]
    fn waiting_for_approval_is_grey_with_its_own_line() {
        let mut ev = status(TaskState::Blocked, true);
        if let Event::Status { waiting_for, .. } = &mut ev {
            *waiting_for = Some(WaitingFor::Approval);
        }
        let card = StatusCard::from_event(&ev).unwrap();
        assert_eq!(card.tone, Tone::Gray);
        let _en = scoped(Locale::En);
        assert_eq!(display(&card.status_line), "Needs your approval");
    }

    #[test]
    fn newest_status_wins_and_other_events_are_not_cards() {
        let mut old = status(TaskState::Running, true);
        let new = status(TaskState::Done, false);
        if let Event::Status { updated_at, .. } = &mut old {
            *updated_at = "2026-11-20T09:00:00.000Z".into();
        }
        let events = vec![new.clone(), old];
        assert_eq!(latest_status(&events), Some(&new));
        let msg = Event::Message {
            id: "m".into(),
            kind: robroz_contract::MessageKind::Text,
            body: "hi".into(),
            created_at: "2026-11-20T10:00:00.000Z".into(),
            author_user_id: None,
            author_name: None,
        };
        assert!(StatusCard::from_event(&msg).is_none());
        assert_eq!(latest_status(&[msg]), None);
    }
}
