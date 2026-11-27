//! Process line, duration and the "working" row for a chat.
//!
//! One collapsed line per task round, expanded to the progress steps
//! recorded in this session. Duration is wall-clock, including blocked
//! time. The local timer starts when this session sees a transition
//! into a live run, and is exact. Otherwise a fallback is used and
//! always marked approximate: a live card uses `now − createdAt`
//! (`updatedAt` refreshes on every progress write, so it would be too
//! short); a finished card uses `updatedAt − createdAt`. After a
//! follow-up re-queue, `createdAt` is not reset, so from the second
//! round on the fallback is cumulative. When the host sends `startedAt`
//! (contract-v0.3.1), duration is exact (no 「约」 / "~"): a live card
//! uses `now − startedAt`, a finished card uses `updatedAt − startedAt`.
//! Without it (an older host, or a reconnect with no field) the
//! fallback stays approximate.
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

/// Show 「同步中…」 / "Syncing…" only if a resync snapshot is still
/// incomplete after this many milliseconds (empty groups resync on
/// every reconnect; a short snapshot must not flash the status).
pub const RESYNC_STATUS_MS: u128 = 300;

/// An empty snapshot has no id-carrying frame. After this many ms
/// with no data following `resync`, treat the snapshot as complete.
pub const RESYNC_EMPTY_COMPLETE_MS: u128 = 300;

/// Safety: data arrived after `resync` but no id-carrying frame came.
/// Complete after this many ms of silence so 「同步中…」 cannot stick.
pub const RESYNC_STALL_COMPLETE_MS: u128 = 2_000;

/// Default wait when a 503 `too_many_streams` has no `Retry-After`.
pub const TOO_MANY_STREAMS_RETRY_SECS: u64 = 5;

/// Whether a resync collection should finish.
///
/// The official signal is an id-carrying frame (`got_batch_end`). An
/// empty group never sends one, so no data for [`RESYNC_EMPTY_COMPLETE_MS`]
/// completes too. If some cards arrived but the id never did, complete
/// after [`RESYNC_STALL_COMPLETE_MS`] of silence.
pub fn resync_should_complete(
    got_batch_end: bool,
    seen_data: bool,
    elapsed_ms: u128,
    silence_ms: u128,
) -> bool {
    if got_batch_end {
        return true;
    }
    if !seen_data && elapsed_ms >= RESYNC_EMPTY_COMPLETE_MS {
        return true;
    }
    if seen_data && silence_ms >= RESYNC_STALL_COMPLETE_MS {
        return true;
    }
    false
}

/// Show the Syncing status only after [`RESYNC_STATUS_MS`], and never
/// once the snapshot is complete (so an empty group that finishes at
/// 300ms does not flash).
pub fn resync_show_status(complete: bool, elapsed_ms: u128) -> bool {
    !complete && elapsed_ms >= RESYNC_STATUS_MS
}

/// Locale-aware 「同步中…」 / "Syncing…".
pub fn syncing_banner() -> String {
    Key::Syncing.into()
}

/// Locale-aware 「连接太多了，稍后自动重试」 / "Too many connections. Retrying shortly."
pub fn too_many_streams_banner() -> String {
    Key::TooManyStreams.into()
}

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
    /// First `running` seen in this session, only if we saw the
    /// transition into running (not a snapshot that was already running).
    pub local_started: Option<i64>,
    /// Host `startedAt` (when the current round first entered `running`).
    /// Exact when present. Unix seconds.
    pub started_at: Option<i64>,
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
            // Under 1s never reads "0s" / 「约 0 秒」, exact or approximate.
            Self::Running { secs: 0, .. } => Some(Key::ProcessWorkingUnder1.into()),
            Self::Done { secs: 0, .. } => Some(Key::ProcessWorkedUnder1.into()),
            Self::Running { secs, approx } => {
                let d = format_duration(secs);
                if approx {
                    Some(Key::ProcessWorkingAbout.fill(&[&d]))
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

    // Host `startedAt` and the local timer are exact. Any fallback
    // (neither, this session) is approximate: an older host, a
    // reconnect without the field, or a follow-up round still keyed
    // off the first `createdAt`.
    let end = if input.live {
        input.now
    } else {
        input.updated_at.unwrap_or(input.now)
    };
    let (start, approx) = if let Some(start) = input.started_at {
        (start, false)
    } else if let Some(start) = input.local_started {
        (start, false)
    } else if let Some(created) = input.created_at {
        (created, true)
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
/// - 1: 「{name} 在处理…」 / "{name} is working…"
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

/// Split a process or status line into `(run, tabular)` spans.
///
/// ASCII digit runs use a tabular / monospace figure font; everything
/// else — including the space in 「38 秒」 — uses the UI font at the
/// same size. iced has no `tabular-nums` feature, so the view renders
/// each run as its own text widget.
pub fn line_spans(line: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut digits = false;
    for c in line.chars() {
        let is_digit = c.is_ascii_digit();
        if buf.is_empty() {
            digits = is_digit;
            buf.push(c);
            continue;
        }
        if is_digit == digits {
            buf.push(c);
            continue;
        }
        out.push((std::mem::take(&mut buf), digits));
        digits = is_digit;
        buf.push(c);
    }
    if !buf.is_empty() {
        out.push((buf, digits));
    }
    out
}

/// Unix seconds from an RFC 3339 timestamp.
pub fn parse_unix(iso: &str) -> Option<i64> {
    crate::runs::parse_unix(iso)
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
            local_started: None,
            started_at: None,
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
    fn under_one_second_never_says_zero() {
        let running = |approx| TimerPhase::Running { secs: 0, approx };
        let done = |approx| TimerPhase::Done { secs: 0, approx };
        let _en = scoped(Locale::En);
        for approx in [false, true] {
            assert_eq!(running(approx).line().as_deref(), Some("Working under 1s"));
            assert_eq!(done(approx).line().as_deref(), Some("Worked for under 1s"));
        }
        assert_eq!(
            line_spans("Worked for under 1s"),
            vec![
                ("Worked for under ".into(), false),
                ("1".into(), true),
                ("s".into(), false),
            ]
        );
        drop(_en);
        let _zh = scoped(Locale::Zh);
        for approx in [false, true] {
            assert_eq!(running(approx).line().as_deref(), Some("处理中不到 1 秒"));
            assert_eq!(done(approx).line().as_deref(), Some("用了不到 1 秒"));
        }
        assert_eq!(
            line_spans("用了不到 1 秒"),
            vec![
                ("用了不到 ".into(), false),
                ("1".into(), true),
                (" 秒".into(), false),
            ]
        );
    }

    #[test]
    fn local_timer_is_exact() {
        let _en = scoped(Locale::En);
        let phase = timer_phase(TimerInput {
            local_started: Some(1_000 - 12),
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
    fn blocked_and_queued_hide_the_timer() {
        for state in [TaskState::Blocked, TaskState::Queued] {
            let phase = timer_phase(TimerInput {
                state,
                live: false,
                created_at: Some(900),
                updated_at: Some(1_000),
                ..input()
            });
            assert_eq!(phase, TimerPhase::Hidden, "{state:?}");
            assert_eq!(phase.line(), None);
        }
    }

    #[test]
    fn done_local_timer_is_exact_and_includes_blocked_time() {
        let _en = scoped(Locale::En);
        let phase = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            local_started: Some(1_000 - 38),
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
    fn live_fallback_uses_now_minus_created_at() {
        let _en = scoped(Locale::En);
        // updatedAt is only 5s ago (a progress write); the card has been
        // running for 20s. The fallback must use now − createdAt.
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
        assert_eq!(running.line().as_deref(), Some("Working ~20s"));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(running.line().as_deref(), Some("处理中约 20 秒"));
    }

    #[test]
    fn done_fallback_uses_updated_at_minus_created_at() {
        let _en = scoped(Locale::En);
        let done = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            created_at: Some(1_000 - 38),
            updated_at: Some(1_000),
            now: 1_200,
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
        assert_eq!(done.line().as_deref(), Some("用了约 38 秒"));

        let _en = scoped(Locale::En);
        let mins = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            created_at: Some(1_000 - 72),
            updated_at: Some(1_000),
            now: 1_000,
            ..input()
        });
        assert_eq!(mins.line().as_deref(), Some("Worked for ~1m 12s"));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(mins.line().as_deref(), Some("用了约 1 分 12 秒"));
    }

    #[test]
    fn follow_up_round_without_a_local_timer_is_approximate() {
        // createdAt is not reset on a follow-up re-queue, so the fallback
        // from the second round on is the cumulative span and is marked
        // approximate. A local timer started on the re-queue transition
        // stays exact.
        let _en = scoped(Locale::En);
        let fallback = timer_phase(TimerInput {
            created_at: Some(1_000 - 200),
            updated_at: Some(1_000 - 2),
            now: 1_000,
            ..input()
        });
        assert_eq!(
            fallback,
            TimerPhase::Running {
                secs: 200,
                approx: true
            }
        );
        let exact = timer_phase(TimerInput {
            local_started: Some(1_000 - 8),
            created_at: Some(1_000 - 200),
            updated_at: Some(1_000 - 2),
            now: 1_000,
            ..input()
        });
        assert_eq!(
            exact,
            TimerPhase::Running {
                secs: 8,
                approx: false
            }
        );
    }

    #[test]
    fn a_failed_card_has_no_process_timer() {
        let phase = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            failed: true,
            created_at: Some(900),
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
        let line = reconnect_banner();
        assert_eq!(line, "Reconnecting…");
        assert_eq!(line.chars().last(), Some('\u{2026}'));
        assert!(!line.contains("..."));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(reconnect_banner(), "正在重新连接");
    }

    #[test]
    fn line_spans_keep_words_in_the_ui_font_and_digits_tabular() {
        assert_eq!(
            line_spans("Working 12s"),
            vec![
                ("Working ".into(), false),
                ("12".into(), true),
                ("s".into(), false),
            ]
        );
        assert_eq!(
            line_spans("用了 38 秒"),
            vec![
                ("用了 ".into(), false),
                ("38".into(), true),
                (" 秒".into(), false),
            ]
        );
        assert_eq!(
            line_spans("Worked for ~38s"),
            vec![
                ("Worked for ~".into(), false),
                ("38".into(), true),
                ("s".into(), false),
            ]
        );
        assert_eq!(
            line_spans("1m 12s"),
            vec![
                ("1".into(), true),
                ("m ".into(), false),
                ("12".into(), true),
                ("s".into(), false),
            ]
        );
        assert_eq!(
            line_spans("1 分 12 秒"),
            vec![
                ("1".into(), true),
                (" 分 ".into(), false),
                ("12".into(), true),
                (" 秒".into(), false),
            ]
        );
        assert_eq!(line_spans("卡住"), vec![("卡住".into(), false)]);
        assert_eq!(
            line_spans("Working ~12s"),
            vec![
                ("Working ~".into(), false),
                ("12".into(), true),
                ("s".into(), false),
            ]
        );
        assert_eq!(
            line_spans("用了约 38 秒"),
            vec![
                ("用了约 ".into(), false),
                ("38".into(), true),
                (" 秒".into(), false),
            ]
        );
        assert_eq!(
            line_spans("处理中约 12 秒"),
            vec![
                ("处理中约 ".into(), false),
                ("12".into(), true),
                (" 秒".into(), false),
            ]
        );
        assert_eq!(
            line_spans("用了约 1 分 12 秒"),
            vec![
                ("用了约 ".into(), false),
                ("1".into(), true),
                (" 分 ".into(), false),
                ("12".into(), true),
                (" 秒".into(), false),
            ]
        );
    }

    #[test]
    fn host_started_at_is_exact_and_beats_the_created_at_fallback() {
        let _en = scoped(Locale::En);
        let phase = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            started_at: Some(1_000 - 38),
            created_at: Some(1_000 - 120),
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
    fn without_started_at_the_fallback_stays_approximate() {
        let _en = scoped(Locale::En);
        let phase = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            started_at: None,
            created_at: Some(1_000 - 38),
            updated_at: Some(1_000),
            now: 1_000,
            ..input()
        });
        assert_eq!(
            phase,
            TimerPhase::Done {
                secs: 38,
                approx: true
            }
        );
        assert_eq!(phase.line().as_deref(), Some("Worked for ~38s"));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(phase.line().as_deref(), Some("用了约 38 秒"));
    }

    #[test]
    fn started_at_under_one_second_never_says_zero() {
        let phase = timer_phase(TimerInput {
            state: TaskState::Done,
            live: false,
            started_at: Some(1_000),
            updated_at: Some(1_000),
            now: 1_000,
            ..input()
        });
        let _en = scoped(Locale::En);
        assert_eq!(phase.line().as_deref(), Some("Worked for under 1s"));
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(phase.line().as_deref(), Some("用了不到 1 秒"));
    }

    #[test]
    fn resync_status_waits_300ms_and_empty_completes_without_flashing() {
        assert!(!resync_should_complete(false, false, 299, 299));
        assert!(resync_should_complete(false, false, 300, 300));
        assert!(resync_should_complete(true, true, 10, 0));
        assert!(!resync_should_complete(false, true, 1_999, 1_999));
        assert!(resync_should_complete(false, true, 2_500, 2_000));
        // Empty group finishes at 300ms: do not show Syncing.
        assert!(!resync_show_status(true, 300));
        assert!(!resync_show_status(false, 299));
        assert!(resync_show_status(false, 300));
    }

    #[test]
    fn header_notice_copy() {
        let _en = scoped(Locale::En);
        assert_eq!(syncing_banner(), "Syncing…");
        assert_eq!(
            too_many_streams_banner(),
            "Too many connections. Retrying shortly."
        );
        drop(_en);
        let _zh = scoped(Locale::Zh);
        assert_eq!(syncing_banner(), "同步中…");
        assert_eq!(too_many_streams_banner(), "连接太多了，稍后自动重试");
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
