//! Map host codes to the client's own copy (contract-v0.3.0).
//!
//! The host still sends the legacy `label` / `failure` / `progress` /
//! `reason` text. The client never shows it: every line comes from a code
//! (`statusCode`, `failureCode` + `failureDetail`, `progressCode` +
//! `progressParams`, `reasonCode`) and i18n. Copy is 最优雅UI姐's
//! `contract-v030-codes.md` (rev 2).
//!
//! An unknown code (a newer host) or a legacy text with no code is never
//! blank: failures and approval reasons show 「出错了」 / "Something went
//! wrong", progress shows 「进行中」 / "In progress", and the raw value is
//! logged once when the event is decoded ([`strip_unknown_codes`]).

use robroz_contract::{
    ApprovalReasonCode, CardStatusCode, DeliveryBadge, FailureCode, ProgressCode, ProgressParams,
    TaskState,
};
use serde_json::Value;

use crate::i18n::{locale, text_in, Key, Locale};

/// Log a value the client could not map. Never shown to the user.
fn log_raw(field: &str, raw: &str) {
    eprintln!("robroz: unknown {field} {raw:?}");
}

/// Remove `failureCode`, `progressCode` (with `progressParams`) and
/// `reasonCode` values this client does not know from one event JSON (or
/// an array / object holding events), logging each raw value. The typed
/// event then decodes, and the card falls back to the unknown-code copy
/// because the legacy text is there without a code.
pub fn strip_unknown_codes(value: &mut Value) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(strip_unknown_codes),
        Value::Object(map) => {
            let known = |field: &str, raw: &str| match field {
                "failureCode" => FailureCode::ALL.iter().any(|c| c.as_str() == raw),
                "progressCode" => ProgressCode::ALL.iter().any(|c| c.as_str() == raw),
                _ => ApprovalReasonCode::parse(raw).is_some(),
            };
            for field in ["failureCode", "progressCode", "reasonCode"] {
                let unknown = match map.get(field) {
                    Some(Value::String(raw)) if !known(field, raw) => Some(raw.clone()),
                    Some(Value::Null) | Some(Value::String(_)) | None => None,
                    Some(other) => Some(other.to_string()),
                };
                if let Some(raw) = unknown {
                    log_raw(field, &raw);
                    map.remove(field);
                    match field {
                        // The detail belongs to the code; never show it alone.
                        "failureCode" => {
                            map.remove("failureDetail");
                        }
                        "progressCode" => {
                            map.remove("progressParams");
                        }
                        _ => {}
                    }
                }
            }
            // Legacy text with no code draws the unknown-code copy: log it
            // here, once per decode, not on every redraw.
            for (text, code) in [
                ("failure", "failureCode"),
                ("progress", "progressCode"),
                ("reason", "reasonCode"),
            ] {
                if let (Some(Value::String(raw)), None) = (map.get(text), map.get(code)) {
                    let typed = map.get("type").and_then(Value::as_str);
                    let card = matches!(typed, Some("status" | "delivery" | "approval"));
                    let skip = raw.trim().is_empty()
                        || raw == robroz_contract::legacy::NO_RESULT
                        || raw == robroz_contract::APPROVAL_EXPIRED_REASON;
                    if card && !skip {
                        log_raw(&format!("{text} without {code}"), raw);
                    }
                }
            }
            map.values_mut().for_each(strip_unknown_codes);
        }
        _ => {}
    }
}

/// The status word for a `statusCode`.
pub fn status_text(code: CardStatusCode) -> String {
    match code {
        CardStatusCode::Queued => Key::CardQueued.into(),
        CardStatusCode::Running => Key::CardRunning.into(),
        CardStatusCode::Blocked => Key::CardBlocked.into(),
        CardStatusCode::Done => Key::CardDone.into(),
        CardStatusCode::NoResult => Key::HostNoResult.into(),
    }
}

/// The status word for a task `state` when the host sent no `statusCode`.
/// `running` without a live container is [`CardStatusCode::NoResult`].
pub fn status_text_from_state(state: TaskState, container_up: bool) -> String {
    status_text(match state {
        TaskState::Queued => CardStatusCode::Queued,
        TaskState::Running if container_up => CardStatusCode::Running,
        TaskState::Running => CardStatusCode::NoResult,
        TaskState::Done => CardStatusCode::Done,
        TaskState::Blocked => CardStatusCode::Blocked,
    })
}

/// The corner badge for a delivery `badge`.
pub fn badge_text(badge: DeliveryBadge) -> String {
    match badge {
        DeliveryBadge::Briefing => Key::CardBriefing.into(),
    }
}

/// A failed card's copy: one line, plus an optional raw detail line drawn
/// under it (12px monospace, muted, selectable, never translated).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureLine {
    pub text: String,
    pub detail: Option<String>,
}

impl FailureLine {
    /// From the wire: the code when there is one, else the legacy text.
    /// `None` when the event did not fail.
    pub fn from_wire(
        code: Option<FailureCode>,
        detail: Option<&str>,
        legacy: Option<&str>,
    ) -> Option<Self> {
        Self::from_wire_in(locale(), code, detail, legacy)
    }

    pub fn from_wire_in(
        locale: Locale,
        code: Option<FailureCode>,
        detail: Option<&str>,
        legacy: Option<&str>,
    ) -> Option<Self> {
        match (code, legacy) {
            (Some(code), _) => Some(Self::from_code_in(locale, code, detail)),
            (None, Some(legacy)) => Some(Self::from_legacy_in(locale, legacy)),
            (None, None) => None,
        }
    }

    pub fn from_code(code: FailureCode, detail: Option<&str>) -> Self {
        Self::from_code_in(locale(), code, detail)
    }

    /// The copy for `code`, with `failureDetail` (`d`) placed per code:
    /// inline, on the detail line, or not at all. A missing `d` drops its
    /// brackets or clause.
    pub fn from_code_in(locale: Locale, code: FailureCode, detail: Option<&str>) -> Self {
        let d = detail.map(str::trim).filter(|s| !s.is_empty());
        let tr = |key: Key| text_in(locale, key).to_string();
        let fill = |key: Key, d: &str| key.fill_in(locale, &[&d]);
        let with = |key: Key, alone: Key| d.map_or_else(|| tr(alone), |d| fill(key, d));
        let mut below = None;
        let body = match code {
            FailureCode::NoResult => {
                return Self {
                    text: tr(Key::FailNoResult),
                    detail: None,
                }
            }
            FailureCode::Denied => with(Key::FailDenied, Key::FailDeniedStep),
            FailureCode::ApprovalExpired => tr(Key::FailApprovalExpired),
            FailureCode::ModelNoReply => match d {
                Some(d) if d.chars().all(|c| c.is_ascii_digit()) => {
                    fill(Key::FailModelNoReplyHttp, d)
                }
                other => {
                    below = other;
                    tr(Key::FailModelNoReply)
                }
            },
            FailureCode::ToolRoundLimit => tr(Key::FailToolRoundLimit),
            FailureCode::OutboundOverQuota => {
                with(Key::FailOutboundOverQuotaCount, Key::FailOutboundOverQuota)
            }
            FailureCode::OutboundNotGet => {
                with(Key::FailOutboundNotGetUsed, Key::FailOutboundNotGet)
            }
            FailureCode::SaveApprovalStateFailed => {
                below = d;
                tr(Key::FailSaveApprovalState)
            }
            FailureCode::ResumeApprovedCallFailed => {
                below = d;
                tr(Key::FailResumeApprovedCall)
            }
            FailureCode::WaitingApproval => {
                with(Key::FailWaitingApprovalTool, Key::FailWaitingApproval)
            }
            FailureCode::ToolCallFailed => {
                below = d;
                tr(Key::FailToolCall)
            }
            FailureCode::ToolNeedsApproval => tr(Key::FailToolNeedsApproval),
            FailureCode::Other => match d {
                Some(d) => d.to_string(),
                None => {
                    return Self::unknown_in(locale);
                }
            },
        };
        Self {
            text: Key::HostFailedBecause.fill_in(locale, &[&body]),
            detail: below.map(str::to_string),
        }
    }

    /// 「出错了」 / "Something went wrong": no prefix, no detail.
    pub fn unknown_in(locale: Locale) -> Self {
        Self {
            text: text_in(locale, Key::CodeUnknown).to_string(),
            detail: None,
        }
    }

    /// A host `failure` with no `failureCode` (an older host, or a code this
    /// client stripped as unknown). The dead-run line is `no_result`;
    /// anything else is the unknown copy, and the raw text is logged.
    pub fn from_legacy(failure: &str) -> Self {
        Self::from_legacy_in(locale(), failure)
    }

    pub fn from_legacy_in(locale: Locale, failure: &str) -> Self {
        let failure = failure.trim();
        if failure == robroz_contract::legacy::NO_RESULT {
            return Self::from_code_in(locale, FailureCode::NoResult, None);
        }
        Self::unknown_in(locale)
    }
}

/// What a status card says the bro is doing, kept as a code so a language
/// switch redraws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressLine {
    Code(ProgressCode, ProgressParams),
    /// Unknown code, a required param missing, or legacy text with no code.
    Unknown,
}

impl ProgressLine {
    /// `None` when the event has no progress at all.
    pub fn from_wire(
        code: Option<ProgressCode>,
        params: Option<&ProgressParams>,
        legacy: Option<&str>,
    ) -> Option<Self> {
        let params = params.cloned().unwrap_or_default();
        match code {
            Some(code) => {
                let line = Self::Code(code, params);
                if line.render(Locale::Zh).is_none() {
                    return Some(Self::Unknown);
                }
                Some(line)
            }
            None => {
                legacy.map(str::trim).filter(|s| !s.is_empty())?;
                Some(Self::Unknown)
            }
        }
    }

    /// The Router step slot (left of the status word) takes this line.
    pub fn is_step(&self) -> bool {
        matches!(self, Self::Code(ProgressCode::RouterStep, _))
    }

    pub fn text(&self) -> String {
        self.text_in(locale())
    }

    pub fn text_in(&self, locale: Locale) -> String {
        self.render(locale)
            .unwrap_or_else(|| text_in(locale, Key::ProgressUnknown).to_string())
    }

    fn render(&self, locale: Locale) -> Option<String> {
        let Self::Code(code, p) = self else {
            return None;
        };
        let tr = |key: Key| text_in(locale, key).to_string();
        Some(match code {
            ProgressCode::Started => tr(Key::ProgressStarted),
            ProgressCode::Thinking => tr(Key::ProgressThinking),
            ProgressCode::InSandbox => tr(Key::ProgressInSandbox),
            ProgressCode::UsingTool => {
                let tool = p.tool.as_deref().map(str::trim).filter(|s| !s.is_empty())?;
                Key::ProgressUsingTool.fill_in(locale, &[&tool])
            }
            ProgressCode::RouterStep => {
                Key::ProgressRouterStep.fill_in(locale, &[&p.step?, &p.steps?])
            }
            ProgressCode::StepTimeout => Key::ProgressStepTimeout.fill_in(locale, &[&p.minutes?]),
            ProgressCode::Stalled => Key::ProgressStalled.fill_in(locale, &[&p.minutes?]),
            ProgressCode::Custom => {
                let text = p
                    .text
                    .as_deref()?
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                (!text.is_empty()).then_some(text)?
            }
        })
    }
}

/// The approval card's reason line. `None` when there is no reason, and
/// for `expired` (the card shows its expired state instead).
pub fn reason_text(code: Option<ApprovalReasonCode>, reason: Option<&str>) -> Option<String> {
    reason_text_in(locale(), code, reason)
}

pub fn reason_text_in(
    locale: Locale,
    code: Option<ApprovalReasonCode>,
    reason: Option<&str>,
) -> Option<String> {
    let reason = reason.map(str::trim).filter(|s| !s.is_empty());
    let tr = |key: Key| Some(text_in(locale, key).to_string());
    let unknown = || tr(Key::CodeUnknown);
    match code {
        Some(ApprovalReasonCode::ReviewModel) => reason.map(str::to_string).or_else(unknown),
        Some(ApprovalReasonCode::ReviewAllowed) => tr(Key::ReasonAllowed),
        Some(ApprovalReasonCode::ReviewNoMatch) => tr(Key::ReasonNoMatch),
        Some(ApprovalReasonCode::ReviewNoModel) => tr(Key::ReasonNoModel),
        Some(ApprovalReasonCode::ReviewFailed) => tr(Key::ReasonFailed),
        Some(ApprovalReasonCode::ReviewAllowFailed) => tr(Key::ReasonAllowFailed),
        Some(ApprovalReasonCode::Expired) => None,
        None => {
            let reason = reason?;
            if reason == robroz_contract::APPROVAL_EXPIRED_REASON {
                return None;
            }
            unknown()
        }
    }
}

/// `expired` by code, or by the legacy text from an older host.
pub fn reason_is_expired(code: Option<ApprovalReasonCode>, reason: Option<&str>) -> bool {
    code == Some(ApprovalReasonCode::Expired)
        || (code.is_none() && reason == Some(robroz_contract::APPROVAL_EXPIRED_REASON))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::scoped;
    use serde_json::json;
    use FailureCode as F;

    fn fl(locale: Locale, code: FailureCode, d: Option<&str>) -> (String, Option<String>) {
        let l = FailureLine::from_code_in(locale, code, d);
        (l.text, l.detail)
    }

    fn s(x: &str) -> String {
        x.to_string()
    }

    #[test]
    fn every_failure_code_with_and_without_detail_in_both_locales() {
        use Locale::{En, Zh};
        type Case<'a> = (
            FailureCode,
            Option<&'a str>,
            Locale,
            &'a str,
            Option<&'a str>,
        );
        let cases: [Case; 44] = [
            (F::NoResult, None, Zh, "没写出结果", None),
            (F::NoResult, Some("x"), En, "Finished with no result", None),
            (
                F::Denied,
                Some("server.tool"),
                Zh,
                "没做成：你拒绝了 server.tool",
                None,
            ),
            (
                F::Denied,
                Some("server.tool"),
                En,
                "Couldn't finish: you declined server.tool",
                None,
            ),
            (F::Denied, None, Zh, "没做成：你拒绝了这一步", None),
            (
                F::Denied,
                None,
                En,
                "Couldn't finish: you declined this step",
                None,
            ),
            (F::ApprovalExpired, None, Zh, "没做成：审批过期了", None),
            (
                F::ApprovalExpired,
                None,
                En,
                "Couldn't finish: the approval expired",
                None,
            ),
            (
                F::ModelNoReply,
                Some("500"),
                Zh,
                "没做成：模型没回（HTTP 500）",
                None,
            ),
            (
                F::ModelNoReply,
                Some("500"),
                En,
                "Couldn't finish: the model didn't reply (HTTP 500)",
                None,
            ),
            (
                F::ModelNoReply,
                Some("response_not_json"),
                Zh,
                "没做成：模型没回",
                Some("response_not_json"),
            ),
            (
                F::ModelNoReply,
                Some("timed out"),
                En,
                "Couldn't finish: the model didn't reply",
                Some("timed out"),
            ),
            (F::ModelNoReply, None, Zh, "没做成：模型没回", None),
            (
                F::ModelNoReply,
                None,
                En,
                "Couldn't finish: the model didn't reply",
                None,
            ),
            (
                F::ToolRoundLimit,
                None,
                Zh,
                "没做成：工具调用轮数到上限了",
                None,
            ),
            (
                F::ToolRoundLimit,
                None,
                En,
                "Couldn't finish: it hit the tool call limit",
                None,
            ),
            (
                F::OutboundOverQuota,
                Some("3/2"),
                Zh,
                "没做成：出网请求超额（3/2）",
                None,
            ),
            (
                F::OutboundOverQuota,
                Some("3/2"),
                En,
                "Couldn't finish: too many outbound requests (3/2)",
                None,
            ),
            (F::OutboundOverQuota, None, Zh, "没做成：出网请求超额", None),
            (
                F::OutboundOverQuota,
                Some("  "),
                En,
                "Couldn't finish: too many outbound requests",
                None,
            ),
            (
                F::OutboundNotGet,
                Some("POST"),
                Zh,
                "没做成：只允许 GET，这次用了 POST",
                None,
            ),
            (
                F::OutboundNotGet,
                Some("POST"),
                En,
                "Couldn't finish: only GET is allowed, this used POST",
                None,
            ),
            (F::OutboundNotGet, None, Zh, "没做成：只允许 GET", None),
            (
                F::OutboundNotGet,
                None,
                En,
                "Couldn't finish: only GET is allowed",
                None,
            ),
            (
                F::SaveApprovalStateFailed,
                Some("pending_corrupt"),
                Zh,
                "没做成：审批状态没存上",
                Some("pending_corrupt"),
            ),
            (
                F::SaveApprovalStateFailed,
                None,
                En,
                "Couldn't finish: couldn't save the approval state",
                None,
            ),
            (
                F::ResumeApprovedCallFailed,
                Some("pending_not_found"),
                En,
                "Couldn't finish: couldn't continue the approved call",
                Some("pending_not_found"),
            ),
            (
                F::ResumeApprovedCallFailed,
                None,
                Zh,
                "没做成：批准后没能接着调用",
                None,
            ),
            (
                F::WaitingApproval,
                Some("send_email"),
                Zh,
                "没做成：还在等批准：send_email",
                None,
            ),
            (
                F::WaitingApproval,
                Some("send_email"),
                En,
                "Couldn't finish: still waiting for approval: send_email",
                None,
            ),
            (F::WaitingApproval, None, Zh, "没做成：还在等批准", None),
            (
                F::WaitingApproval,
                None,
                En,
                "Couldn't finish: still waiting for approval",
                None,
            ),
            (
                F::ToolCallFailed,
                Some("connection refused"),
                Zh,
                "没做成：工具调用失败",
                Some("connection refused"),
            ),
            (
                F::ToolCallFailed,
                Some("connection refused"),
                En,
                "Couldn't finish: a tool call failed",
                Some("connection refused"),
            ),
            (
                F::ToolCallFailed,
                None,
                En,
                "Couldn't finish: a tool call failed",
                None,
            ),
            (
                F::ToolNeedsApproval,
                None,
                Zh,
                "没做成：批准过的调用又要审批",
                None,
            ),
            (
                F::ToolNeedsApproval,
                Some("x"),
                En,
                "Couldn't finish: the approved call needed approval again",
                None,
            ),
            (F::Other, Some("磁盘满了"), Zh, "没做成：磁盘满了", None),
            (
                F::Other,
                Some("disk full"),
                En,
                "Couldn't finish: disk full",
                None,
            ),
            (F::Other, None, Zh, "出错了", None),
            (F::Other, None, En, "Something went wrong", None),
            (
                F::ApprovalExpired,
                Some("x"),
                En,
                "Couldn't finish: the approval expired",
                None,
            ),
            (
                F::ToolRoundLimit,
                Some("8"),
                Zh,
                "没做成：工具调用轮数到上限了",
                None,
            ),
            (
                F::SaveApprovalStateFailed,
                Some("disk"),
                En,
                "Couldn't finish: couldn't save the approval state",
                Some("disk"),
            ),
        ];
        for (code, d, locale, text, detail) in cases {
            assert_eq!(
                fl(locale, code, d),
                (s(text), detail.map(s)),
                "{code:?} {d:?} {locale:?}"
            );
        }
        // Every code is covered, and no code leaves an unfilled placeholder.
        for code in FailureCode::ALL {
            assert!(cases.iter().any(|c| c.0 == code), "{code:?} untested");
            for locale in [En, Zh] {
                for d in [None, Some("1"), Some("a b")] {
                    let (t, _) = fl(locale, code, d);
                    assert!(!t.trim().is_empty() && !t.contains('{'), "{code:?} {t}");
                    assert!(!t.contains("()") && !t.contains("（）"), "{code:?} {t}");
                }
            }
        }
        assert_eq!(FailureCode::ALL.len(), 13);
    }

    #[test]
    fn unknown_and_legacy_failures_use_the_fallback() {
        assert_eq!(
            FailureLine::from_legacy_in(Locale::Zh, "模型服务返回 500"),
            FailureLine {
                text: s("出错了"),
                detail: None
            }
        );
        assert_eq!(
            FailureLine::from_legacy_in(Locale::En, "模型服务返回 500").text,
            "Something went wrong"
        );
        assert_eq!(
            FailureLine::from_legacy_in(Locale::Zh, robroz_contract::legacy::NO_RESULT).text,
            "没写出结果"
        );
        assert_eq!(
            FailureLine::from_wire_in(Locale::En, None, None, None),
            None
        );
    }

    #[test]
    fn unknown_codes_are_stripped_with_their_payload_and_known_ones_kept() {
        let mut v = json!([{
            "type": "status", "failure": "x", "failureCode": "brand_new",
            "failureDetail": "secret", "progress": "p", "progressCode": "teleporting",
            "progressParams": {"tool": "t"}
        }, {"type": "approval", "reason": "r", "reasonCode": "galaxy_brain"},
           {"type": "status", "failureCode": "denied", "failureDetail": "a.b",
            "progressCode": "started"}]);
        strip_unknown_codes(&mut v);
        assert_eq!(
            v,
            json!([{"type": "status", "failure": "x", "progress": "p"},
                   {"type": "approval", "reason": "r"},
                   {"type": "status", "failureCode": "denied", "failureDetail": "a.b",
                    "progressCode": "started"}])
        );
    }

    fn pl(code: ProgressCode, p: ProgressParams, locale: Locale) -> String {
        ProgressLine::from_wire(Some(code), Some(&p), None)
            .unwrap()
            .text_in(locale)
    }

    #[test]
    fn every_progress_code_in_both_locales() {
        use Locale::{En, Zh};
        use ProgressCode as P;
        let none = ProgressParams::default;
        let tool = || ProgressParams {
            tool: Some("web.search".into()),
            ..none()
        };
        let step = || ProgressParams {
            step: Some(2),
            steps: Some(3),
            ..none()
        };
        let mins = || ProgressParams {
            minutes: Some(4),
            ..none()
        };
        let text = || ProgressParams {
            text: Some("  查  资料 ".into()),
            ..none()
        };
        let cases = [
            (P::Started, none(), "开始干活", "Starting"),
            (P::Thinking, none(), "在想怎么回", "Thinking"),
            (P::InSandbox, none(), "在沙箱里跑", "Running in the sandbox"),
            (P::UsingTool, tool(), "在用 web.search", "Using web.search"),
            (P::RouterStep, step(), "第 2/3 步", "Step 2 of 3"),
            (
                P::StepTimeout,
                mins(),
                "模型和工具还没回，已等 4 分钟",
                "No reply from the model or tools for 4 min",
            ),
            (
                P::Stalled,
                mins(),
                "4 分钟没有新进度",
                "No progress for 4 min",
            ),
            (P::Custom, text(), "查 资料", "查 资料"),
        ];
        for (code, p, zh, en) in cases.clone() {
            assert_eq!(pl(code, p.clone(), Zh), zh, "{code:?}");
            assert_eq!(pl(code, p, En), en, "{code:?}");
        }
        for code in ProgressCode::ALL {
            assert!(cases.iter().any(|c| c.0 == code), "{code:?} untested");
        }
        assert!(
            ProgressLine::from_wire(Some(P::RouterStep), Some(&step()), None)
                .unwrap()
                .is_step()
        );
    }

    #[test]
    fn progress_missing_params_unknown_and_legacy_fall_back() {
        use ProgressCode as P;
        for code in [
            P::UsingTool,
            P::RouterStep,
            P::StepTimeout,
            P::Stalled,
            P::Custom,
        ] {
            let line = ProgressLine::from_wire(Some(code), None, None).unwrap();
            assert_eq!(line, ProgressLine::Unknown, "{code:?}");
            assert_eq!(line.text_in(Locale::Zh), "进行中");
            assert_eq!(line.text_in(Locale::En), "In progress");
        }
        let legacy = ProgressLine::from_wire(None, None, Some("在搞别的")).unwrap();
        assert_eq!(legacy.text_in(Locale::En), "In progress");
        assert_eq!(ProgressLine::from_wire(None, None, Some("  ")), None);
        assert_eq!(ProgressLine::from_wire(None, None, None), None);
    }

    #[test]
    fn every_reason_code_in_both_locales() {
        use ApprovalReasonCode as R;
        use Locale::{En, Zh};
        let cases = [
            (
                R::ReviewModel,
                Some("会改生产数据"),
                Some("会改生产数据"),
                Some("会改生产数据"),
            ),
            (
                R::ReviewAllowed,
                None,
                Some("符合放行规则"),
                Some("Allowed by your rules"),
            ),
            (
                R::ReviewNoMatch,
                Some("x"),
                Some("没有放行规则允许这个操作"),
                Some("No rule allows this"),
            ),
            (
                R::ReviewNoModel,
                None,
                Some("还没设置模型，没法自动审核"),
                Some("No model set, so auto-review can't run"),
            ),
            (
                R::ReviewFailed,
                None,
                Some("自动审核没成功，需要你批准"),
                Some("Auto-review didn't work. It needs your approval"),
            ),
            (
                R::ReviewAllowFailed,
                None,
                Some("审核放行了，但任务没能接着跑，你来定"),
                Some("Auto-review allowed it, but the task couldn't continue. Your call"),
            ),
            (R::Expired, Some("审批已过期"), None, None),
        ];
        for (code, reason, zh, en) in cases {
            assert_eq!(
                reason_text_in(Zh, Some(code), reason).as_deref(),
                zh,
                "{code:?}"
            );
            assert_eq!(
                reason_text_in(En, Some(code), reason).as_deref(),
                en,
                "{code:?}"
            );
        }
        for code in ApprovalReasonCode::ALL {
            assert!(cases.iter().any(|c| c.0 == code), "{code:?} untested");
        }
        assert_eq!(text_in(Zh, Key::ReasonExpired), "审批已过期");
        assert_eq!(text_in(En, Key::ReasonExpired), "Approval expired");
        assert!(reason_is_expired(Some(R::Expired), None));
        assert!(reason_is_expired(
            None,
            Some(robroz_contract::APPROVAL_EXPIRED_REASON)
        ));
        assert!(!reason_is_expired(Some(R::ReviewModel), Some("x")));
    }

    #[test]
    fn unknown_or_missing_reason_falls_back() {
        use ApprovalReasonCode as R;
        assert_eq!(
            reason_text_in(Locale::Zh, None, Some("别的原因")).as_deref(),
            Some("出错了")
        );
        assert_eq!(
            reason_text_in(Locale::En, None, Some("别的原因")).as_deref(),
            Some("Something went wrong")
        );
        assert_eq!(
            reason_text_in(Locale::En, Some(R::ReviewModel), Some(" ")).as_deref(),
            Some("Something went wrong")
        );
        assert_eq!(reason_text_in(Locale::En, None, None), None);
        assert_eq!(
            reason_text_in(
                Locale::En,
                None,
                Some(robroz_contract::APPROVAL_EXPIRED_REASON)
            ),
            None
        );
    }

    #[test]
    fn status_and_badge_use_the_existing_card_words() {
        let _g = scoped(Locale::En);
        assert_eq!(status_text(CardStatusCode::Queued), "Queued");
        assert_eq!(status_text(CardStatusCode::Running), "Running");
        assert_eq!(status_text(CardStatusCode::Blocked), "Stuck");
        assert_eq!(status_text(CardStatusCode::Done), "Done");
        assert_eq!(badge_text(DeliveryBadge::Briefing), "Briefing");
        let _g = scoped(Locale::Zh);
        assert_eq!(status_text(CardStatusCode::Running), "进行中");
        assert_eq!(badge_text(DeliveryBadge::Briefing), "简报");
    }

    #[test]
    fn status_text_from_state_matches_status_code_and_follows_locale() {
        use crate::i18n::{scoped, Locale};
        use robroz_contract::TaskState;
        assert_eq!(
            status_text_from_state(TaskState::Queued, false),
            status_text(CardStatusCode::Queued)
        );
        assert_eq!(
            status_text_from_state(TaskState::Running, true),
            status_text(CardStatusCode::Running)
        );
        assert_eq!(
            status_text_from_state(TaskState::Running, false),
            status_text(CardStatusCode::NoResult)
        );
        assert_eq!(
            status_text_from_state(TaskState::Done, false),
            status_text(CardStatusCode::Done)
        );
        assert_eq!(
            status_text_from_state(TaskState::Blocked, false),
            status_text(CardStatusCode::Blocked)
        );
        {
            let _en = scoped(Locale::En);
            assert_eq!(status_text_from_state(TaskState::Done, false), "Done");
            assert_eq!(status_text_from_state(TaskState::Running, true), "Running");
        }
        let _zh = scoped(Locale::Zh);
        assert_eq!(status_text_from_state(TaskState::Done, false), "完成");
        assert_eq!(status_text_from_state(TaskState::Running, true), "进行中");
    }
}
