//! Map v0.2 `statusCode` / `failureCode` / `badge` to the client's own copy.
//!
//! The host still sends the legacy `label` / `failure` / `briefing` text.
//! When a code is present the client never shows that text; it picks a
//! sentence from i18n. `failureDetail` always sits behind "Show details",
//! except [`FailureCode::Other`]: a detail in the UI language is shown as
//! the failed prefix plus that detail.

use robroz_contract::{CardStatusCode, DeliveryBadge, FailureCode};

use crate::i18n::{locale, Key, Locale};

/// Whether a host `failureDetail` can sit on the main line in `locale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailFit {
    /// Same language as the UI, or language-neutral (a status code, tool
    /// name, HTTP method, or a figure like `3/2`).
    Inline,
    /// CJK in an English UI, or English prose in a Chinese UI.
    Collapse,
}

/// A CJK ideograph, kana, hangul, or CJK punctuation / fullwidth mark.
pub fn is_cjk(c: char) -> bool {
    matches!(
        c,
        '\u{3400}'..='\u{9FFF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{3040}'..='\u{30FF}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{20000}'..='\u{2CEAF}'
            | '\u{3000}'..='\u{303F}'
            | '\u{FF00}'..='\u{FFEF}'
    )
}

/// A token that does not belong to one UI language: digits, an HTTP
/// method, a tool name, a path, a status code, or `used/limit`.
fn is_neutral(detail: &str) -> bool {
    if detail.chars().any(is_cjk) {
        return false;
    }
    if !detail.chars().any(|c| c.is_ascii_alphabetic()) {
        return true;
    }
    !detail.chars().any(char::is_whitespace)
}

/// How `detail` should be shown in `locale`.
pub fn detail_fit(locale: Locale, detail: &str) -> DetailFit {
    let detail = detail.trim();
    if detail.is_empty() || is_neutral(detail) {
        return DetailFit::Inline;
    }
    let cjk = detail.chars().any(is_cjk);
    match locale {
        Locale::En if cjk => DetailFit::Collapse,
        Locale::Zh if !cjk => DetailFit::Collapse,
        _ => DetailFit::Inline,
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

/// The corner badge for a delivery `badge`.
pub fn badge_text(badge: DeliveryBadge) -> String {
    match badge {
        DeliveryBadge::Briefing => Key::CardBriefing.into(),
    }
}

/// Standalone sentence for a `failureCode`, with no prefix and no detail.
pub fn failure_sentence(code: FailureCode) -> String {
    failure_key(code).into()
}

pub fn failure_key(code: FailureCode) -> Key {
    match code {
        FailureCode::NoResult => Key::HostNoResult,
        FailureCode::Denied => Key::FailDenied,
        FailureCode::ApprovalExpired => Key::FailApprovalExpired,
        FailureCode::ModelNoReply => Key::FailModelNoReply,
        FailureCode::ToolRoundLimit => Key::FailToolRoundLimit,
        FailureCode::OutboundOverQuota => Key::FailOutboundOverQuota,
        FailureCode::OutboundNotGet => Key::FailOutboundNotGet,
        FailureCode::SaveApprovalStateFailed => Key::FailSaveApprovalState,
        FailureCode::ResumeApprovedCallFailed => Key::FailResumeApprovedCall,
        FailureCode::WaitingApproval => Key::FailWaitingApproval,
        FailureCode::ToolCallFailed => Key::FailToolCall,
        FailureCode::ToolNeedsApproval => Key::FailToolNeedsApproval,
        FailureCode::Other => Key::FailOther,
    }
}

/// The line a failed or state card shows, plus an optional detail to
/// hide behind "Show details".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureLine {
    pub text: String,
    pub collapsed: Option<String>,
}

impl FailureLine {
    /// Build from a v0.2 code and optional raw `failureDetail`.
    ///
    /// The first line is the sentence for the code. Any detail goes in
    /// the disclosure, with one exception: [`FailureCode::Other`] shows
    /// the failed prefix plus the detail when the detail matches the UI
    /// language, and the bare failed sentence when it does not (detail
    /// still in the disclosure). `denied`, `approval_expired`,
    /// `waiting_approval` and `tool_needs_approval` are states and never
    /// take the failed prefix.
    pub fn from_code(code: FailureCode, detail: Option<&str>) -> Self {
        Self::from_code_in(locale(), code, detail)
    }

    pub fn from_code_in(locale: Locale, code: FailureCode, detail: Option<&str>) -> Self {
        let sentence = failure_key(code).to_string_in(locale);
        let detail = detail.map(str::trim).filter(|s| !s.is_empty());
        if code == FailureCode::Other {
            let Some(detail) = detail else {
                return Self {
                    text: sentence,
                    collapsed: None,
                };
            };
            if detail_fit(locale, detail) == DetailFit::Inline {
                return Self {
                    text: Key::HostFailedBecause.fill_in(locale, &[&detail]),
                    collapsed: None,
                };
            }
            return Self {
                text: sentence,
                collapsed: Some(detail.to_string()),
            };
        }
        Self {
            text: sentence,
            collapsed: detail.map(str::to_string),
        }
    }

    /// A host `failure` with no `failureCode`. Treated as `no_result` when
    /// the text is the dead-run line, otherwise as [`FailureCode::Other`].
    pub fn from_legacy(failure: &str) -> Self {
        Self::from_legacy_in(locale(), failure)
    }

    pub fn from_legacy_in(locale: Locale, failure: &str) -> Self {
        let failure = failure.trim();
        if failure == robroz_contract::legacy::NO_RESULT {
            return Self::from_code_in(locale, FailureCode::NoResult, None);
        }
        Self::from_code_in(locale, FailureCode::Other, Some(failure))
    }
}

trait ToStringIn {
    fn to_string_in(self, locale: Locale) -> String;
}

impl ToStringIn for Key {
    fn to_string_in(self, locale: Locale) -> String {
        crate::i18n::text_in(locale, self).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{scoped, text_in, Locale};

    fn line(locale: Locale, code: FailureCode, detail: Option<&str>) -> FailureLine {
        FailureLine::from_code_in(locale, code, detail)
    }

    #[test]
    fn every_failure_code_has_a_non_empty_sentence_in_both_locales() {
        for code in FailureCode::ALL {
            for locale in [Locale::En, Locale::Zh] {
                let s = text_in(locale, failure_key(code));
                assert!(!s.trim().is_empty(), "{code:?} {locale:?}");
                // The sentence itself must not carry the failed prefix: the
                // client adds that only for `other` when the detail fits.
                assert!(
                    !s.starts_with("Couldn't finish:") && !s.starts_with("没做成："),
                    "sentence for {code:?} starts with the failed prefix: {s}"
                );
            }
        }
        assert_eq!(FailureCode::ALL.len(), 13);
    }

    #[test]
    fn the_first_line_is_the_code_sentence_and_detail_goes_in_the_row() {
        let l = line(
            Locale::Zh,
            FailureCode::ModelNoReply,
            Some("模型服务返回 401"),
        );
        assert_eq!(l.text, "模型没有回复。");
        assert_eq!(l.collapsed.as_deref(), Some("模型服务返回 401"));
        assert!(!l.text.contains("没做成"));
        assert!(!l.text.contains("调用失败"));

        let l = line(Locale::En, FailureCode::OutboundOverQuota, Some("3/2"));
        assert_eq!(l.text, "The outbound request quota was reached.");
        assert_eq!(l.collapsed.as_deref(), Some("3/2"));
        assert!(!l.text.contains("3/2"));

        let l = line(
            Locale::En,
            FailureCode::ModelNoReply,
            Some("模型服务返回 401"),
        );
        assert_eq!(l.text, "The model didn't reply.");
        assert_eq!(l.collapsed.as_deref(), Some("模型服务返回 401"));
        assert!(!l.text.contains("Couldn't finish"));
    }

    #[test]
    fn other_inlines_a_matching_detail_and_collapses_a_mismatched_one() {
        let l = line(Locale::Zh, FailureCode::Other, Some("模型服务返回 500"));
        assert_eq!(l.text, "没做成：模型服务返回 500");
        assert_eq!(l.collapsed, None);

        let l = line(Locale::En, FailureCode::Other, Some("模型服务返回 500"));
        assert_eq!(l.text, "This didn't get done.");
        assert_eq!(l.collapsed.as_deref(), Some("模型服务返回 500"));
        assert!(!l.text.contains("模型"));

        let l = line(
            Locale::En,
            FailureCode::Other,
            Some("the model returned 500"),
        );
        assert_eq!(l.text, "Couldn't finish: the model returned 500");
        assert_eq!(l.collapsed, None);

        let l = line(Locale::Zh, FailureCode::Other, None);
        assert_eq!(l.text, "没做成。");
    }

    #[test]
    fn states_never_take_the_failed_prefix() {
        for code in [
            FailureCode::Denied,
            FailureCode::ApprovalExpired,
            FailureCode::WaitingApproval,
            FailureCode::ToolNeedsApproval,
        ] {
            let l = line(Locale::Zh, code, Some("发送周报"));
            assert!(!l.text.starts_with("没做成"), "{code:?} {t}", t = l.text);
            assert_eq!(l.collapsed.as_deref(), Some("发送周报"));
        }
        let l = line(Locale::En, FailureCode::Denied, Some("发送周报"));
        assert_eq!(l.text, "You denied this.");
        assert_eq!(l.collapsed.as_deref(), Some("发送周报"));
    }

    #[test]
    fn no_result_is_the_sentence_even_with_a_detail() {
        let l = line(Locale::Zh, FailureCode::NoResult, Some("模型服务返回 401"));
        assert_eq!(l.text, "没写出结果。");
        assert_eq!(l.collapsed.as_deref(), Some("模型服务返回 401"));
        let l = line(Locale::En, FailureCode::NoResult, None);
        assert_eq!(l.text, "Finished with no result.");
        assert_eq!(l.collapsed, None);
    }

    #[test]
    fn legacy_failures_follow_the_other_rule() {
        let l = FailureLine::from_legacy_in(Locale::En, "模型服务返回 500");
        assert_eq!(l.text, "This didn't get done.");
        assert_eq!(l.collapsed.as_deref(), Some("模型服务返回 500"));
        let l = FailureLine::from_legacy_in(Locale::Zh, "模型服务返回 500");
        assert_eq!(l.text, "没做成：模型服务返回 500");
        assert_eq!(l.collapsed, None);
        let l = FailureLine::from_legacy_in(Locale::Zh, robroz_contract::legacy::NO_RESULT);
        assert_eq!(l.text, "没写出结果。");
        assert_eq!(l.collapsed, None);
    }

    #[test]
    fn an_absent_detail_is_the_code_sentence() {
        let l = line(Locale::En, FailureCode::ToolRoundLimit, None);
        assert_eq!(l.text, "The tool was called too many times.");
        assert_eq!(l.collapsed, None);
        let l = line(Locale::Zh, FailureCode::ApprovalExpired, None);
        assert_eq!(l.text, "审批已过期。");
    }

    #[test]
    fn detail_fit_treats_tokens_as_neutral_and_prose_by_script() {
        assert_eq!(detail_fit(Locale::En, "POST"), DetailFit::Inline);
        assert_eq!(detail_fit(Locale::Zh, "3/2"), DetailFit::Inline);
        assert_eq!(detail_fit(Locale::En, "server.tool"), DetailFit::Inline);
        assert_eq!(
            detail_fit(Locale::En, "模型服务返回 401"),
            DetailFit::Collapse
        );
        assert_eq!(
            detail_fit(Locale::Zh, "model returned 401"),
            DetailFit::Collapse
        );
        assert_eq!(
            detail_fit(Locale::En, "model returned 401"),
            DetailFit::Inline
        );
        assert_eq!(
            detail_fit(Locale::Zh, "模型服务返回 401"),
            DetailFit::Inline
        );
        assert_eq!(detail_fit(Locale::En, ""), DetailFit::Inline);
    }

    #[test]
    fn status_and_badge_use_the_existing_card_words() {
        let _g = scoped(Locale::En);
        assert_eq!(status_text(CardStatusCode::Queued), "Queued");
        assert_eq!(status_text(CardStatusCode::Running), "Running");
        assert_eq!(status_text(CardStatusCode::Blocked), "Stuck");
        assert_eq!(status_text(CardStatusCode::Done), "Done");
        assert_eq!(
            status_text(CardStatusCode::NoResult),
            "Finished with no result."
        );
        assert_eq!(badge_text(DeliveryBadge::Briefing), "Briefing");
        let _g = scoped(Locale::Zh);
        assert_eq!(status_text(CardStatusCode::Running), "进行中");
        assert_eq!(badge_text(DeliveryBadge::Briefing), "简报");
        assert_eq!(status_text(CardStatusCode::NoResult), "没写出结果。");
    }
}
