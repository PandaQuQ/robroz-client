use robroz_contract::TaskState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Gray,
    Ink,
    Green,
    Amber,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appearance {
    pub tone: Tone,
    pub action: Option<&'static str>,
}

/// The dead-run line. The host ends a dead run as `done` with exactly this
/// `failure`; older data may still have it as `running` with no container.
///
/// The text lives in `robroz_contract::legacy` so the host does not need this
/// crate; this is the same constant.
pub const NO_RESULT: &str = robroz_contract::legacy::NO_RESULT;

/// The grey line for a `done` task that carries a `failure`. A dead run shows
/// [`NO_RESULT`] as it is; every other reason reads 「没做成：原因」.
pub fn failure_line(failure: &str) -> String {
    if failure == NO_RESULT {
        NO_RESULT.to_string()
    } else {
        format!("没做成：{failure}")
    }
}

/// `container_up` is ignored except for a live `running` card.
/// A dead run in old data is `Running` and uses the same gray as queued.
pub fn appearance(state: TaskState, container_up: bool) -> Appearance {
    match state {
        TaskState::Queued => Appearance {
            tone: Tone::Gray,
            action: None,
        },
        TaskState::Running if container_up => Appearance {
            tone: Tone::Ink,
            action: None,
        },
        TaskState::Running => Appearance {
            tone: Tone::Gray,
            action: None,
        },
        TaskState::Done => Appearance {
            tone: Tone::Green,
            action: None,
        },
        TaskState::Blocked => Appearance {
            tone: Tone::Amber,
            action: Some("去私聊接管"),
        },
    }
}

/// A task blocked on an unanswered approval card (`waitingFor: "approval"`).
/// Same amber as other blocked states, so the stuck dot uses `status.blocked`
/// yellow; no takeover button, because the approval card is the only way to act.
pub fn awaiting_approval() -> Appearance {
    Appearance {
        tone: Tone::Amber,
        action: None,
    }
}

pub fn status_line(state: TaskState, container_up: bool) -> &'static str {
    match state {
        TaskState::Queued => "排队",
        TaskState::Running if container_up => "进行",
        TaskState::Running => NO_RESULT,
        TaskState::Done => "完成",
        TaskState::Blocked => "卡住",
    }
}

pub fn briefing_badge() -> &'static str {
    "简报"
}

/// sRGB fill and text colour for a card tone. Clients paint exactly these
/// values and never pick their own colour for a task state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub fill: [u8; 3],
    pub text: [u8; 3],
}

pub fn palette(tone: Tone) -> Palette {
    match tone {
        Tone::Gray => Palette {
            fill: [0xE3, 0xE3, 0xE3],
            text: [0x4A, 0x4A, 0x4A],
        },
        Tone::Ink => Palette {
            fill: [0x1F, 0x23, 0x28],
            text: [0xFF, 0xFF, 0xFF],
        },
        Tone::Green => Palette {
            fill: [0x2E, 0x7D, 0x32],
            text: [0xFF, 0xFF, 0xFF],
        },
        Tone::Amber => Palette {
            fill: [0xF5, 0xB7, 0x00],
            text: [0x1F, 0x23, 0x28],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_for_each_state() {
        assert_eq!(
            appearance(TaskState::Queued, false),
            Appearance {
                tone: Tone::Gray,
                action: None
            }
        );
        assert_eq!(
            appearance(TaskState::Queued, true),
            Appearance {
                tone: Tone::Gray,
                action: None
            },
            "queued ignores container_up"
        );
        assert_eq!(
            appearance(TaskState::Running, true),
            Appearance {
                tone: Tone::Ink,
                action: None
            }
        );
        assert_eq!(
            appearance(TaskState::Running, false),
            Appearance {
                tone: Tone::Gray,
                action: None
            }
        );
        assert_eq!(
            appearance(TaskState::Done, false),
            Appearance {
                tone: Tone::Green,
                action: None
            }
        );
        assert_eq!(
            appearance(TaskState::Blocked, false),
            Appearance {
                tone: Tone::Amber,
                action: Some("去私聊接管")
            }
        );
        assert_eq!(
            awaiting_approval(),
            Appearance {
                tone: Tone::Amber,
                action: None
            }
        );
        assert_eq!(
            awaiting_approval().tone,
            appearance(TaskState::Blocked, false).tone
        );
    }

    #[test]
    fn failure_line_keeps_the_dead_run_line() {
        assert_eq!(failure_line("没写出结果"), "没写出结果");
        assert_eq!(
            failure_line(NO_RESULT),
            status_line(TaskState::Running, false)
        );
        assert_eq!(failure_line("模型没回：超时"), "没做成：模型没回：超时");
    }

    #[test]
    fn status_line_for_each_state() {
        assert_eq!(status_line(TaskState::Queued, false), "排队");
        assert_eq!(status_line(TaskState::Queued, true), "排队");
        assert_eq!(status_line(TaskState::Running, true), "进行");
        assert_eq!(status_line(TaskState::Running, false), "没写出结果");
        assert_eq!(status_line(TaskState::Done, false), "完成");
        assert_eq!(status_line(TaskState::Blocked, false), "卡住");
    }

    #[test]
    fn card_copy_is_the_contract_legacy_copy() {
        for state in [
            TaskState::Queued,
            TaskState::Running,
            TaskState::Blocked,
            TaskState::Done,
        ] {
            for up in [false, true] {
                assert_eq!(
                    status_line(state, up),
                    robroz_contract::legacy::status_line(state, up)
                );
            }
        }
        assert_eq!(NO_RESULT, robroz_contract::legacy::NO_RESULT);
        assert_eq!(briefing_badge(), robroz_contract::legacy::briefing_badge());
    }

    #[test]
    fn briefing_badge_text() {
        assert_eq!(briefing_badge(), "简报");
    }

    #[test]
    fn palette_per_tone_is_distinct() {
        let tones = [Tone::Gray, Tone::Ink, Tone::Green, Tone::Amber];
        for (i, a) in tones.iter().enumerate() {
            for b in &tones[i + 1..] {
                assert_ne!(palette(*a).fill, palette(*b).fill);
            }
            assert_ne!(palette(*a).fill, palette(*a).text);
        }
        // A dead run paints exactly like queued.
        assert_eq!(
            palette(appearance(TaskState::Running, false).tone),
            palette(appearance(TaskState::Queued, false).tone)
        );
    }
}
