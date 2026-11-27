//! Process line, duration and the "working" row for a chat.
//!
//! One collapsed line per task round, expanded to the progress steps
//! recorded in this session. Duration is wall-clock, including blocked
//! time. `Status.startedAt` is used when the host sent it (contract
//! v0.3.1, read from JSON without bumping the crate tag); otherwise a
//! local timer from the first `running` seen this session, or
//! `updatedAt − createdAt` marked approximate after a restart.
//!
//! While the task is blocked or queued the line shows no timer. On
//! `done` it shows the total.

use robroz_contract::TaskState;

use crate::cards::codes::ProgressLine;
use crate::i18n::Key;

/// Seconds with no bytes on the SSE stream before the client reconnects.
pub const SSE_IDLE_SECS: u64 = 45;

/// After this many seconds of continuous failed reconnects, show the
/// reconnecting banner. Shorter failures stay silent.
pub const SSE_BANNER_SECS: u64 = 30;

/// Frozen `--fake` / gallery clock. Process timers are measured from this instant.
pub const FAKE_NOW_ISO: &str = "2026-10-07T19:10:00.000Z";

/// Frozen clock for `--fake` / gallery screenshots. `None` means wall clock.
static FAKE_NOW: std::sync::Mutex<Option<i64>> = std::sync::Mutex::new(None);

/// Current Unix seconds: wall clock, or the frozen fake clock.
pub fn now_unix() -> i64 {
    FAKE_NOW
        .lock()
        .expect("fake clock")
        .unwrap_or_else(|| chrono::Utc::now().timestamp())
}

/// Freeze [`now_unix`] at `unix` (fake and gallery runs).
pub fn freeze_clock(unix: i64) {
    *FAKE_NOW.lock().expect("fake clock") = Some(unix);
}

/// Inputs for one task's process timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerInput {
    pub state: TaskState,
    /// `true` when this is a live run (`Running` and `containerUp`).
    pub live: bool,
    pub failed: bool,
    /// Host `startedAt`, if the event carried it.
    pub started_at: Option<i64>,
    /// First `running` seen in this session, only if we saw the
    /// transition into running (not a snapshot that was already running).
    pub local_started: Option<i64>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    pub now: i64,
}

/// What the collapsed process line shows for duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerPhase {
    /// Queued, blocked, or no times to work from: no timer.
    Hidden,
    Running {
        secs: u64,
        approx: bool,
    },
    Done {
        secs: u64,
        approx: bool,
    },
}

impl TimerPhase {
    /// Duration line in the active locale, or `None` when hidden.
    pub fn line(self) -> Option<String> {
        match self {
            Self::Hidden => None,
            Self::Running { secs, approx } => {
                let d = format_duration(secs);
                if approx {
                    Some(Key::ProcessWorkedAbout.fill(&[&d]))
                } else {
                    Some(Key::ProcessWorking.fill(&[&d]))
                }
            }
            Self::Done { secs, approx } => {
                let d = format_duration(secs);
                if approx {
                    Some(Key::ProcessWorkedAbout.fill(&[&d]))
                } else {
                    Some(Key::ProcessWorked.fill(&[&d]))
                }
            }
        }
    }

    pub fn is_running(self) -> bool {
        matches!(self, Self::Running { .. })
    }
}

/// Wall-clock duration for this round.
pub fn timer_phase(input: TimerInput) -> TimerPhase {
    if input.failed {
        return TimerPhase::Hidden;
    }
    // Blocked and queued show no timer. A live run or a finished card does.
    if !input.live && input.state != TaskState::Done {
        return TimerPhase::Hidden;
    }

    let (start, end, approx) = if let Some(start) = input.started_at {
        let end = if input.live {
            input.now
        } else {
            input.updated_at.unwrap_or(input.now)
        };
        (start, end, false)
    } else if let Some(start) = input.local_started {
        let end = if input.live {
            input.now
        } else {
            input.updated_at.unwrap_or(input.now)
        };
        (start, end, false)
    } else if let Some(created) = input.created_at {
        // After a restart there is no local timer and the host did not
        // send `startedAt`. `createdAt` is enqueue time, so this is
        // approximate. A still-running card uses `now`; a finished one
        // uses `updatedAt − createdAt`.
        let end = if input.live {
            input.now
        } else {
            input.updated_at.unwrap_or(input.now)
        };
        (created, end, true)
    } else {
        return TimerPhase::Hidden;
    };

    let secs = end.saturating_sub(start).max(0) as u64;
    if input.live {
        TimerPhase::Running { secs, approx }
    } else {
        TimerPhase::Done { secs, approx }
    }
}

/// 「12 秒」 / "12s", or 「1 分 12 秒」 / "1m 12s".
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        Key::ProcessSecs.fill(&[&secs])
    } else {
        Key::ProcessMins.fill(&[&(secs / 60), &(secs % 60)])
    }
}

/// Record a newly seen progress line at the end of `steps`, skipping a
/// repeat of the last entry.
pub fn push_step(steps: &mut Vec<ProgressLine>, line: ProgressLine) {
    if steps.last() != Some(&line) {
        steps.push(line);
    }
}

/// Whether the collapsed line is expandable (has at least one recorded
/// step). After a restart a finished card typically has none, so it is
/// not clickable and has no ›.
pub fn expandable(steps: &[ProgressLine]) -> bool {
    !steps.is_empty()
}

/// The "working" row above the composer, from the live-running bros in
/// this group, in first-seen order.
///
/// - 1: 「{name} 在处理…」 / "{name} is working…" (pending UI review)
/// - 2: 「{a} 和 {b} 在处理…」 / "{a} and {b} are working…"
/// - >2: 「{a} 和另外 {n} 个」 / "{a} and {n} others"
pub fn working_row(names: &[&str]) -> Option<String> {
    match names {
        [] => None,
        [a] => Some(Key::WorkingRowOne.fill(&[a])),
        [a, b] => Some(Key::WorkingRowTwo.fill(&[a, b])),
        [a, rest @ ..] => Some(Key::WorkingRowMany.fill(&[a, &rest.len()])),
    }
}

/// Unix seconds from an RFC 3339 timestamp.
pub fn parse_unix(iso: &str) -> Option<i64> {
    crate::runs::parse_unix(iso)
}

/// Host `startedAt` on a status JSON object, if present and non-empty.
pub fn started_at_of(value: &serde_json::Value) -> Option<String> {
    value
        .get("startedAt")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Show the reconnecting banner after `failing_since` has been in a
/// continuous failed state for more than [`SSE_BANNER_SECS`].
pub fn show_reconnect_banner(failing_since: Option<i64>, now: i64) -> bool {
    failing_since
        .map(|start| now.saturating_sub(start) > SSE_BANNER_SECS as i64)
        .unwrap_or(false)
}

/// Locale-aware 「正在重新连接」 / "Reconnecting…".
pub fn reconnect_banner() -> String {
    Key::Reconnecting.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{scoped, Locale};
    use robroz_contract::{ProgressCode, ProgressParams};

    fn input() -> TimerInput {
        TimerInput {
            state: TaskState::Running,
            live: true,
            failed: false,
            started_at: None,
            local_started: None,
            created_at: None,
            updated_at: None,
            now: 1_000,
        }
    }

    #[test]
    fn format_duration_under_and_over_a_minute() {
        let _en = scoped(Locale::En);
        assert_eq!(format_duration(0), "0s");
        assert_eq!(format_duration(12), "12s");
        assert_eq!(format_duration(59), "59s");
        assert_eq!(format_duration(60), "1m 0s");
        assert_eq!(format_duration(72), "1m 12s");
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(format_duration(12), "12 秒");
        assert_eq!(format_duration(72), "1 分 12 秒");
    }

    #[test]
    fn running_line_uses_started_at_when_present() {
        let _en = scoped(Locale::En);
        let phase = timer_phase(TimerInput {
            started_at: Some(1_000 - 12),
            local_started: Some(1_000 - 3),
            created_at: Some(1_000 - 40),
            ..input()
        });
        assert_eq!(
            phase,
            TimerPhase::Running {
                secs: 12,
                approx: false
            }
        );
        assert_eq!(phase.line().as_deref(), Some("Working 12s"));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(phase.line().as_deref(), Some("处理中 12 秒"));
    }

    #[test]
    fn local_timer_is_the_fallback_before_started_at() {
        let _en = scoped(Locale::En);
        let phase = timer_phase(TimerInput {
            local_started: Some(1_000 - 12),
            created_at: Some(1_000 - 40),
            ..input()
        });
        assert_eq!(phase.line().as_deref(), Some("Working 12s"));
    }

    #[test]
    fn blocked_and_queued_hide_the_timer() {
        for state in [TaskState::Blocked, TaskState::Queued] {
            let phase = timer_phase(TimerInput {
                state,
                live: false,
                started_at: Some(900),
                updated_at: Some(1_000),
                ..input()
            });
            assert_eq!(phase, TimerPhase::Hidden, "{state:?}");
            assert_eq!(phase.line(), None);
        }
    }

    #[test]
    fn done_shows_the_total_including_blocked_time() {
        let _en = scoped(Locale::En);
        // started 38s before updatedAt; blocked time is included.
        let phase = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            started_at: Some(1_000 - 38),
            updated_at: Some(1_000),
            now: 1_000,
            ..input()
        });
        assert_eq!(
            phase,
            TimerPhase::Done {
                secs: 38,
                approx: false
            }
        );
        assert_eq!(phase.line().as_deref(), Some("Worked for 38s"));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(phase.line().as_deref(), Some("用了 38 秒"));
    }

    #[test]
    fn after_restart_without_started_at_the_span_is_approximate() {
        let _en = scoped(Locale::En);
        let done = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            created_at: Some(1_000 - 38),
            updated_at: Some(1_000),
            now: 1_000,
            ..input()
        });
        assert_eq!(
            done,
            TimerPhase::Done {
                secs: 38,
                approx: true
            }
        );
        assert_eq!(done.line().as_deref(), Some("Worked for ~38s"));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(done.line().as_deref(), Some("约 38 秒"));

        let _en = scoped(Locale::En);
        let running = timer_phase(TimerInput {
            created_at: Some(1_000 - 20),
            updated_at: Some(1_000 - 5),
            now: 1_000,
            ..input()
        });
        assert_eq!(
            running,
            TimerPhase::Running {
                secs: 20,
                approx: true
            }
        );
    }

    #[test]
    fn a_failed_card_has_no_process_timer() {
        let phase = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            failed: true,
            started_at: Some(900),
            updated_at: Some(1_000),
            ..input()
        });
        assert_eq!(phase, TimerPhase::Hidden);
    }

    #[test]
    fn working_row_copy_for_one_two_and_many() {
        assert_eq!(working_row(&[]), None);
        let _en = scoped(Locale::En);
        assert_eq!(working_row(&["Ada"]).as_deref(), Some("Ada is working…"));
        assert_eq!(
            working_row(&["Ada", "Bea"]).as_deref(),
            Some("Ada and Bea are working…")
        );
        assert_eq!(
            working_row(&["Ada", "Bea", "Cy"]).as_deref(),
            Some("Ada and 2 others")
        );
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(working_row(&["Ada"]).as_deref(), Some("Ada 在处理…"));
        assert_eq!(
            working_row(&["Ada", "Bea"]).as_deref(),
            Some("Ada 和 Bea 在处理…")
        );
        assert_eq!(
            working_row(&["Ada", "Bea", "Cy"]).as_deref(),
            Some("Ada 和另外 2 个")
        );
    }

    #[test]
    fn steps_are_recorded_once_and_make_the_line_expandable() {
        let mut steps = Vec::new();
        assert!(!expandable(&steps));
        let thinking = ProgressLine::Code(ProgressCode::Thinking, ProgressParams::default());
        let tool = ProgressLine::Code(
            ProgressCode::UsingTool,
            ProgressParams {
                tool: Some("search".into()),
                ..ProgressParams::default()
            },
        );
        push_step(&mut steps, thinking.clone());
        push_step(&mut steps, thinking);
        push_step(&mut steps, tool);
        assert_eq!(steps.len(), 2);
        assert!(expandable(&steps));
    }

    #[test]
    fn reconnect_banner_waits_more_than_thirty_seconds() {
        assert!(!show_reconnect_banner(None, 100));
        assert!(!show_reconnect_banner(Some(100), 130));
        assert!(show_reconnect_banner(Some(100), 131));
        let _en = scoped(Locale::En);
        assert_eq!(reconnect_banner(), "Reconnecting…");
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(reconnect_banner(), "正在重新连接");
    }

    #[test]
    fn frozen_clock_overrides_wall_time() {
        freeze_clock(42);
        assert_eq!(now_unix(), 42);
        // Leave the clock frozen only if a later test wants it; thaw.
        *FAKE_NOW.lock().unwrap() = None;
        assert!(now_unix() >= 42);
    }
}
