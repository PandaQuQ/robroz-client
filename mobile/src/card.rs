//! The status card for one task, from a contract `Event::Status`.
//!
//! This mirrors the desktop rules in `client/src/model.rs` without iced:
//! colours, status words and actions come from `robroz-card`; a task with a
//! `failure` is grey with the failure line, never the green "done"; a task
//! waiting on an approval is grey too; a live Router step (`n/m 步`) goes
//! left of the status word, other progress under the bro name. Failure and
//! progress lines come from `failureCode` + `failureDetail` and
//! `progressCode` + `progressParams` (never the host text); they are kept as
//! codes and rendered when drawn, so a language switch redraws a card
//! already on screen.

use robroz_card::{Palette, Tone};
use robroz_contract::{Event, FailureCode, TaskState, WaitingFor};
use robroz_ui::cards::codes::{FailureLine, ProgressLine};
use robroz_ui::i18n::{host_text, Key};

/// The status line while a task waits on an approval card. Same constant as
/// the desktop client; `display` translates it.
pub const AWAITING_APPROVAL: &str = "等你批准";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCard {
    pub task_id: String,
    pub bot_id: String,
    pub bot_name: String,
    /// `createdAt`, Unix seconds, when the host sent one.
    pub at: Option<i64>,
    /// Raw `robroz-card` copy. A failed card draws [`Self::failure`] instead.
    pub status_line: String,
    pub tone: Tone,
    pub palette: Palette,
    /// What the bro is doing now, under the bro name.
    pub progress: Option<ProgressLine>,
    /// `progressCode: router_step`, left of the status word, live runs only.
    pub step_progress: Option<ProgressLine>,
    /// `(failureCode, failureDetail, failure)` when the task did not get done.
    pub failure: Option<(Option<FailureCode>, Option<String>, String)>,
    pub state: TaskState,
    pub container_up: bool,
    pub waiting_for: Option<WaitingFor>,
    pub created_at: String,
    pub updated_at: String,
    pub local_started: Option<i64>,
    pub started_at: Option<String>,
    pub steps: Vec<ProgressLine>,
    pub expanded: bool,
}

impl StatusCard {
    fn failure_line(&self) -> Option<FailureLine> {
        let (code, detail, legacy) = self.failure.as_ref()?;
        FailureLine::from_wire(*code, detail.as_deref(), Some(legacy))
    }

    /// The status (or failure) line in the active language.
    pub fn shown_status(&self) -> String {
        match self.failure_line() {
            Some(line) => line.text,
            None => display(&self.status_line).into_owned(),
        }
    }

    /// The raw failure detail line under the status (monospace, muted,
    /// selectable, never translated).
    pub fn detail_line(&self) -> Option<String> {
        self.failure_line().and_then(|l| l.detail)
    }

    pub fn is_live(&self) -> bool {
        self.failure.is_none() && self.state == TaskState::Running && self.container_up
    }

    /// The task is sitting on an approval card.
    pub fn awaits_approval(&self) -> bool {
        awaits_approval(self.state, self.waiting_for)
    }

    pub fn timer(&self, now: i64) -> robroz_ui::process::TimerPhase {
        robroz_ui::process::timer_phase(robroz_ui::process::TimerInput {
            state: self.state,
            live: self.is_live(),
            failed: self.failure.is_some(),
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

    pub fn process_line(&self, now: i64) -> Option<String> {
        if self.failure.is_some() {
            return None;
        }
        if let Some(line) = self.timer(now).line() {
            return Some(line);
        }
        Some(self.shown_status())
    }

    pub fn expandable(&self) -> bool {
        robroz_ui::process::expandable(&self.steps)
    }
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
/// never next to a finished or failed card.
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

impl StatusCard {
    /// `None` for any event that is not a status card.
    pub fn from_event(event: &Event) -> Option<Self> {
        let Event::Status {
            task_id,
            bot_id,
            bot_name,
            created_at,
            updated_at,
            state,
            container_up,
            failure,
            progress,
            progress_code,
            progress_params,
            waiting_for,
            failure_code,
            failure_detail,
            started_at,
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
        let (progress, step_progress) = match shown {
            Some(line) if *state == TaskState::Running && *container_up && line.is_step() => {
                (None, Some(line))
            }
            other => (other, None),
        };
        let mut card = Self {
            task_id: task_id.clone(),
            bot_id: bot_id.clone(),
            bot_name: bot_name.clone(),
            at: robroz_ui::runs::parse_unix(created_at),
            status_line,
            tone: look.tone,
            palette: robroz_card::palette(look.tone),
            progress,
            step_progress,
            failure: failure
                .clone()
                .map(|f| (*failure_code, failure_detail.clone(), f)),
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
        if let Some(line) = card.progress.clone().or(card.step_progress.clone()) {
            robroz_ui::process::push_step(&mut card.steps, line);
        }
        Some(card)
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
    use robroz_contract::{ProgressCode, ProgressParams};
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
            started_at: None,
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

    fn set_progress(ev: &mut Event, code: ProgressCode, params: ProgressParams) {
        if let Event::Status {
            progress,
            progress_code,
            progress_params,
            ..
        } = ev
        {
            *progress = Some("host text".into());
            *progress_code = Some(code);
            *progress_params = Some(params);
        }
    }

    #[test]
    fn failed_task_is_grey_with_the_code_line_never_green_done() {
        let mut ev = status(TaskState::Done, false);
        set(&mut ev, |f, _| {
            *f = Some("模型没回：模型服务返回 500".into())
        });
        if let Event::Status {
            failure_code,
            failure_detail,
            ..
        } = &mut ev
        {
            *failure_code = Some(FailureCode::ModelNoReply);
            *failure_detail = Some("500".into());
        }
        let card = StatusCard::from_event(&ev).unwrap();
        assert_eq!(card.tone, Tone::Gray);
        assert_ne!(card.palette, robroz_card::palette(Tone::Green));
        {
            let _en = scoped(Locale::En);
            assert_eq!(
                card.shown_status(),
                "Couldn't finish: the model didn't reply (HTTP 500)"
            );
        }
        let _zh = scoped(Locale::Zh);
        assert_eq!(card.shown_status(), "没做成：模型没回（HTTP 500）");
        assert_eq!(card.detail_line(), None);
    }

    #[test]
    fn a_raw_detail_goes_on_its_own_line_and_unknown_failures_fall_back() {
        let mut ev = status(TaskState::Done, false);
        set(&mut ev, |f, _| {
            *f = Some("调用失败：connection refused".into())
        });
        if let Event::Status {
            failure_code,
            failure_detail,
            ..
        } = &mut ev
        {
            *failure_code = Some(FailureCode::ToolCallFailed);
            *failure_detail = Some("connection refused".into());
        }
        let card = StatusCard::from_event(&ev).unwrap();
        let _en = scoped(Locale::En);
        assert_eq!(card.shown_status(), "Couldn't finish: a tool call failed");
        assert_eq!(card.detail_line().as_deref(), Some("connection refused"));

        // No code (older host, or a code this client stripped as unknown).
        let mut old = status(TaskState::Done, false);
        set(&mut old, |f, _| *f = Some("模型服务返回 500".into()));
        let card = StatusCard::from_event(&old).unwrap();
        assert_eq!(card.shown_status(), "Something went wrong");
        assert_eq!(card.detail_line(), None);
    }

    #[test]
    fn live_router_step_goes_left_of_the_status_word() {
        let mut ev = status(TaskState::Running, true);
        let step = ProgressParams {
            step: Some(2),
            steps: Some(3),
            ..Default::default()
        };
        set_progress(&mut ev, ProgressCode::RouterStep, step);
        let card = StatusCard::from_event(&ev).unwrap();
        assert_eq!(card.tone, Tone::Ink);
        assert_eq!(card.progress, None);
        let step = card.step_progress.clone().unwrap();
        assert_eq!(step.text_in(Locale::En), "Step 2 of 3");
        assert_eq!(step.text_in(Locale::Zh), "第 2/3 步");
        let _en = scoped(Locale::En);
        assert_eq!(card.shown_status(), "Running");
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(card.shown_status(), "进行中");
    }

    #[test]
    fn other_progress_stays_under_the_name_and_dead_runs_show_none() {
        let mut ev = status(TaskState::Running, true);
        let tool = ProgressParams {
            tool: Some("web.search".into()),
            ..Default::default()
        };
        set_progress(&mut ev, ProgressCode::UsingTool, tool);
        let card = StatusCard::from_event(&ev).unwrap();
        let p = card.progress.clone().unwrap();
        assert_eq!(p.text_in(Locale::Zh), "在用 web.search");
        assert_eq!(p.text_in(Locale::En), "Using web.search");
        assert_eq!(card.step_progress, None);

        // Legacy text with no code: 「进行中」, never the host text.
        let mut legacy = status(TaskState::Running, true);
        set(&mut legacy, |_, p| *p = Some("在查  资料".into()));
        let card = StatusCard::from_event(&legacy).unwrap();
        let p = card.progress.unwrap();
        assert_eq!(p.text_in(Locale::Zh), "进行中");
        assert_eq!(p.text_in(Locale::En), "In progress");

        let mut dead = status(TaskState::Running, false);
        set_progress(&mut dead, ProgressCode::Started, ProgressParams::default());
        let card = StatusCard::from_event(&dead).unwrap();
        assert_eq!((card.progress, card.step_progress), (None, None));
        assert_eq!(card.tone, Tone::Gray);
    }

    #[test]
    fn waiting_for_approval_is_amber_with_its_own_line() {
        let mut ev = status(TaskState::Blocked, true);
        if let Event::Status { waiting_for, .. } = &mut ev {
            *waiting_for = Some(WaitingFor::Approval);
        }
        let card = StatusCard::from_event(&ev).unwrap();
        assert_eq!(card.tone, Tone::Amber, "blocked yellow, not dead-run grey");
        let _en = scoped(Locale::En);
        assert_eq!(card.shown_status(), "Needs your approval");
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
