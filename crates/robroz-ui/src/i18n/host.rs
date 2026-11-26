//! Map Chinese text the host sends to the active locale.
//!
//! Every host-written client string is a constant in
//! `robroz_contract::host_text` (listed in `HOST_UI_STRINGS`) or at the
//! contract root. They are compared against those constants, never against a
//! copy of their text. `robroz-card` lines are compared against its own
//! functions. Anything not listed here passes through unchanged.

use std::borrow::Cow;

use robroz_card::NO_RESULT;
use robroz_contract::host_text as ht;
use robroz_contract::{
    TaskState, APPROVAL_EXPIRED_REASON, APPROVAL_SENT_TO_OWNER_DM_NOTICE, MODEL_TEST_BAD_KEY,
    SETTINGS_APPROVAL_BOT_NAME,
};

use super::{locale, Key, Locale};

/// Whole `host_text` strings, compared with `==`.
pub(super) const WHOLE: &[(&str, Key)] = &[
    (ht::GROUP_NOT_FOUND, Key::HostGroupNotFound),
    (ht::BOT_NOT_IN_GROUP, Key::HostBotNotInGroup),
    (ht::NEED_TEXT, Key::HostNeedText),
    (ht::NEED_NAME, Key::HostNeedName),
    (ht::NEED_TITLE, Key::HostNeedTitle),
    (ht::NEED_INSTRUCTION, Key::HostNeedInstruction),
    (ht::NEED_URL, Key::HostNeedUrl),
    (ht::NEED_MODEL_NAME, Key::HostNeedModelName),
    (ht::NEED_COMMAND, Key::HostNeedCommand),
    (ht::NEED_SECRET_NAME, Key::HostNeedSecretName),
    (ht::NEED_SECRET_VALUE, Key::HostNeedSecretValue),
    (ht::NEED_RULE, Key::HostNeedRule),
    (ht::NEED_ACTION, Key::HostNeedAction),
    (ht::NEED_SEARXNG_URL, Key::HostNeedSearxngUrl),
    (ht::BAD_REQUEST_FORMAT, Key::HostBadRequestFormat),
    (ht::BODY_NOT_JSON, Key::HostBodyNotJson),
    (ht::BODY_INVALID, Key::HostBodyInvalid),
    (ht::NOTHING_TO_CHANGE, Key::HostNothingToChange),
    (ht::DATABASE_ERROR, Key::HostDatabaseError),
    (ht::ALREADY_ANSWERED, Key::HostAlreadyAnswered),
    (ht::SINCE_INVALID, Key::HostSinceInvalid),
    (ht::READ_AT_INVALID, Key::HostReadAtInvalid),
    (ht::URL_NEEDS_HTTP, Key::HostUrlNeedsHttp),
    (ht::KEY_EMPTY, Key::HostKeyEmpty),
    (ht::KEY_HAS_WHITESPACE, Key::HostKeyHasWhitespace),
    (ht::ARGS_INVALID, Key::HostArgsInvalid),
    (ht::BAD_SCOPE, Key::HostBadScope),
    (ht::BAD_STATUS, Key::HostBadStatus),
    (ht::BAD_MEMORY_KIND, Key::HostBadMemoryKind),
    (ht::BAD_MESSAGE_KIND, Key::HostBadMessageKind),
    (ht::BAD_SLUG, Key::HostBadSlug),
    (ht::BAD_CRON, Key::HostBadCron),
    (ht::BAD_TRANSPORT, Key::HostBadTransport),
    (ht::BAD_SEARCH_PROVIDER, Key::HostBadSearchProvider),
    (ht::TOKEN_REJECTED, Key::HostTokenRejected),
    (ht::NAME_TOO_LONG, Key::HostNameTooLong),
    (ht::NAME_MAX_80, Key::HostNameMax80),
    (ht::NAME_ONE_LINE, Key::HostNameOneLine),
    (ht::NAME_TAKEN, Key::HostNameTaken),
    (ht::TITLE_TOO_LONG, Key::HostTitleTooLong),
    (ht::INSTRUCTION_TOO_LONG, Key::HostInstructionTooLong),
    (ht::MEMORY_TOO_LONG, Key::HostMemoryTooLong),
    (ht::SUMMARY_ONE_LINE, Key::HostSummaryOneLine),
    (ht::RULE_ONE_LINE, Key::HostRuleOneLine),
    (ht::COMMAND_INVALID, Key::HostCommandInvalid),
    (ht::SECRET_NAME_MAX, Key::HostSecretNameMax),
    (ht::SECRET_NAME_WHITESPACE, Key::HostSecretNameWhitespace),
    (ht::SLUG_TAKEN, Key::HostSlugTaken),
    (ht::BRO_NOT_FOUND, Key::HostBroNotFound),
    (ht::UNKNOWN_BRO, Key::HostUnknownBro),
    (ht::TASK_NOT_FOUND, Key::HostTaskNotFound),
    (ht::APPROVAL_NOT_FOUND, Key::HostApprovalNotFound),
    (ht::APPROVAL_VANISHED, Key::HostApprovalVanished),
    (ht::CONFIRM_CARD_NOT_FOUND, Key::HostConfirmCardNotFound),
    (ht::CHOICE_CARD_NOT_FOUND, Key::HostChoiceCardNotFound),
    (ht::MEMORY_NOT_FOUND, Key::HostMemoryNotFound),
    (ht::SKILL_NOT_FOUND, Key::HostSkillNotFound),
    (ht::RULE_NOT_FOUND, Key::HostRuleNotFound),
    (ht::ROUTINE_NOT_FOUND, Key::HostRoutineNotFound),
    (ht::MCP_SERVER_NOT_FOUND, Key::HostMcpNotFound),
    (ht::MCP_NOT_ENABLED_IN_GROUP, Key::HostMcpNotEnabledInGroup),
    (ht::MCP_STDIO_ONLY_APPROVE, Key::HostStdioOnlyApprove),
    (ht::DM_ONE_BRO, Key::HostDmOneBro),
    (ht::ACTION_NOT_SUPPORTED, Key::HostUnsupportedAction),
    (ht::ACTION_HOST_ONLY, Key::HostActionHostOnly),
    (
        ht::APPROVE_CONTINUE_MARKER_FAILED,
        Key::HostApproveContinueMarkerFailed,
    ),
    (
        ht::APPROVE_TAKEOVER_MARKER_FAILED,
        Key::HostApproveTakeoverMarkerFailed,
    ),
    (ht::RESUME_NO_INTENT, Key::HostResumeNoIntent),
    (ht::RESUME_MARKER_FAILED, Key::HostResumeMarkerFailed),
    (ht::RESUME_NEEDS_APPROVAL, Key::HostResumeNeedsApproval),
    (ht::CANT_RESUME, Key::HostCantResume),
    (ht::CHOICE_OPTION_COUNT, Key::HostChoiceOptionCount),
    (ht::CHOICE_ONE_RECOMMENDED, Key::HostChoiceOneRecommended),
    (ht::CHOICE_OPTION_EMPTY, Key::HostChoiceOptionEmpty),
    (ht::CHOICE_DUPLICATE, Key::HostChoiceDuplicate),
    (ht::CHOICE_PICK_AT_LEAST_ONE, Key::HostChoicePickAtLeastOne),
    (ht::CHOICE_PICK_ONLY_ONE, Key::HostChoicePickOnlyOne),
    (ht::CHOICE_NO_SUCH_OPTION, Key::HostChoiceNoSuchOption),
    (ht::CHOICE_ALREADY_PICKED, Key::HostChoiceAlreadyPicked),
    (ht::MODEL_NOT_CONFIGURED, Key::HostModelNotConfigured),
    (ht::MODEL_KEY_HAS_WHITESPACE, Key::HostModelKeyHasWhitespace),
    (ht::MODEL_ENV_KEY_CONFLICT, Key::HostModelEnvKeyConflict),
    (ht::MODEL_ENV_KEY_FILE_EMPTY, Key::HostModelEnvKeyFileEmpty),
    (ht::SEARCH_NOT_CONFIGURED, Key::HostSearchNotConfigured),
    (ht::SEARCH_NOT_JSON, Key::HostSearchNotJson),
    (ht::SEARCH_NO_WEB_RESULTS, Key::HostSearchNoWebResults),
    (ht::SEARCH_NO_RESULTS, Key::HostSearchNoResults),
    (ht::PROGRESS_STARTED, Key::HostProgressStarted),
    (ht::PROGRESS_THINKING, Key::HostProgressThinking),
    (ht::PROGRESS_IN_SANDBOX, Key::HostProgressInSandbox),
    (ht::DELIVERY_WRITTEN, Key::HostDeliveryWritten),
    (ht::DELIVERY_NOT_RUN, Key::HostDeliveryNotRun),
    (ht::GATE_FALLBACK_LINE, Key::HostGateFallbackLine),
    (ht::DENIED_MCP_FALLBACK, Key::HostDeniedMcpFallback),
    (ht::TOOL_ROUND_LIMIT, Key::HostToolRoundLimit),
    (ht::OUTBOUND_OVER_QUOTA, Key::HostOutboundOverQuota),
    (ht::PENDING_NOT_APPROVED, Key::HostPendingNotApproved),
    (
        ht::PENDING_MISSING_MESSAGES,
        Key::HostPendingMissingMessages,
    ),
    (
        ht::PENDING_MISSING_REMAINING,
        Key::HostPendingMissingRemaining,
    ),
    (ht::PENDING_CALL_MISSING_ID, Key::HostPendingCallMissingId),
    (
        ht::PENDING_CALL_MISSING_NAME,
        Key::HostPendingCallMissingName,
    ),
    (
        ht::PENDING_MESSAGE_MISSING_ROLE,
        Key::HostPendingMessageMissingRole,
    ),
    (ht::REVIEW_NO_MODEL_REASON, Key::HostReviewNoModel),
    (ht::REVIEW_FAILED_REASON, Key::HostReviewFailed),
    (ht::REVIEW_NO_MATCH_REASON, Key::HostReviewNoMatch),
    (ht::REVIEW_ALLOW_FALLBACK, Key::HostReviewAllowFallback),
];

/// `host_text` `*_PREFIX` + runtime text; the text is mapped again and goes
/// into `{0}`.
pub(super) const PREFIXES: &[(&str, Key)] = &[
    (
        ht::MODEL_ENV_KEY_FILE_UNREADABLE_PREFIX,
        Key::HostModelEnvKeyFileUnreadable,
    ),
    (ht::CANT_CONNECT_PREFIX, Key::HostCantConnect),
    (ht::CONNECTION_LOST_PREFIX, Key::HostConnectionLost),
    (ht::URL_INVALID_PREFIX, Key::HostUrlInvalid),
    (
        ht::SEARCH_SERVICE_STATUS_PREFIX,
        Key::HostSearchServiceStatus,
    ),
    (
        ht::SEARCH_PROVIDER_UNSUPPORTED_PREFIX,
        Key::HostSearchProviderUnsupported,
    ),
    (ht::PROGRESS_USING_PREFIX, Key::HostProgressUsing),
    (ht::GATE_PREFIX, Key::HostExternalWrite),
    (ht::DENIED_PREFIX, Key::HostDeniedFailure),
    (ht::MODEL_NO_REPLY_PREFIX, Key::HostModelNoReply),
    (ht::WAITING_APPROVAL_PREFIX, Key::HostWaitingApproval),
    (
        ht::SAVE_APPROVAL_STATE_FAILED_PREFIX,
        Key::HostSaveApprovalStateFailed,
    ),
    (
        ht::RESUME_APPROVED_CALL_FAILED_PREFIX,
        Key::HostResumeApprovedCallFailed,
    ),
    (ht::PENDING_WRITE_FAILED_PREFIX, Key::HostPendingWriteFailed),
    (ht::PENDING_NOT_FOUND_PREFIX, Key::HostPendingNotFound),
    (ht::PENDING_CORRUPT_PREFIX, Key::HostPendingCorrupt),
    (ht::PENDING_UNKNOWN_ROLE_PREFIX, Key::HostPendingUnknownRole),
    (ht::TOOL_CALL_FAILED_PREFIX, Key::HostCallFailed),
];

/// Whole strings, compared with `==`.
fn exact(s: &str) -> Option<Key> {
    if let Some((_, key)) = WHOLE.iter().find(|(text, _)| *text == s) {
        return Some(*key);
    }
    // Fragments are also mappable on their own (`HOST_UI_STRINGS`). A
    // complete watchdog sentence is handled by `watchdog` below. The stall
    // suffix has no English of its own: a stall sentence only ever maps as
    // a whole, through the `stall_hint` form.
    for (text, key) in [
        (ht::WATCHDOG_STEP_TIMEOUT_PREFIX, Key::HostStepTimeoutPrefix),
        (ht::WATCHDOG_STEP_TIMEOUT_SUFFIX, Key::HostMinutesSuffix),
        (ht::MCP_CALL_SUMMARY_PREFIX, Key::ViewCallPrefix),
        (ht::MCP_CALL_ARGS_SEPARATOR, Key::ViewCallArgsSep),
    ] {
        if text == s {
            return Some(key);
        }
    }
    // Contract root and robroz-card constants.
    if s == APPROVAL_SENT_TO_OWNER_DM_NOTICE {
        return Some(Key::HostSentToOwnerDm);
    }
    if s == APPROVAL_EXPIRED_REASON {
        return Some(Key::HostApprovalExpired);
    }
    if s == SETTINGS_APPROVAL_BOT_NAME {
        return Some(Key::HostSettingsBotName);
    }
    if s == MODEL_TEST_BAD_KEY {
        return Some(Key::HostModelBadKey);
    }
    if s == NO_RESULT {
        return Some(Key::HostNoResult);
    }
    // robroz-card's own status copy (task status card and its action).
    for (state, key) in [
        (TaskState::Queued, Key::CardQueued),
        (TaskState::Done, Key::CardDone),
        (TaskState::Blocked, Key::CardBlocked),
    ] {
        if s == robroz_card::status_line(state, false) {
            return Some(key);
        }
    }
    if s == robroz_card::status_line(TaskState::Running, true) {
        return Some(Key::CardRunning);
    }
    if s == robroz_card::briefing_badge() {
        return Some(Key::CardBriefing);
    }
    if Some(s) == robroz_card::appearance(TaskState::Blocked, false).action {
        return Some(Key::CardTakeOver);
    }
    if s == ht::TOOL_STILL_NEEDS_APPROVAL {
        return Some(Key::HostToolNeedsApproval);
    }
    None
}

/// Watchdog hints: `step_timeout_hint(n)` / `stall_hint(n)`; `n` goes into
/// `{0}`. A stall sentence is recognised only when `stall_hint(n)` rebuilds
/// it exactly, so it always maps to the whole English sentence.
fn watchdog(s: &str) -> Option<(Key, &str)> {
    let is_number = |n: &str| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit());
    if let Some(n) = s
        .strip_prefix(ht::WATCHDOG_STEP_TIMEOUT_PREFIX)
        .and_then(|r| r.strip_suffix(ht::WATCHDOG_STEP_TIMEOUT_SUFFIX))
        .filter(|n| is_number(n))
    {
        return Some((Key::HostStepTimeoutHint, n));
    }
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    let n = &s[..digits];
    n.parse::<u64>()
        .ok()
        .filter(|&m| ht::stall_hint(m) == s)
        .map(|_| (Key::HostStallHint, n))
}

/// `s` in the active locale when it is a known host string; otherwise `s`
/// unchanged. Chinese is the host's own language, so it usually passes
/// through as is. Two display-only exceptions (the host contract is
/// unchanged): router-step progress is shown as 「第 n/m 步」, and a live
/// run's status word as 「进行中」 instead of the card crate's 「进行」.
pub fn host_text(s: &str) -> Cow<'_, str> {
    host_text_in(locale(), s)
}

/// Display-only Chinese rewrite of a host/card string, if any.
fn zh_display_rewrite(s: &str) -> Option<String> {
    if let Some((n, m)) = ht::parse_router_step_progress(s) {
        return Some(Key::HostRouterStep.fill_in(Locale::Zh, &[&n, &m]));
    }
    if exact(s) == Some(Key::CardRunning) {
        return Some(Key::CardRunning.fill_in(Locale::Zh, &[]));
    }
    None
}

/// [`host_text`] for a given locale.
pub fn host_text_in(locale: Locale, s: &str) -> Cow<'_, str> {
    if locale == Locale::Zh {
        return match zh_display_rewrite(s) {
            Some(rewritten) => Cow::Owned(rewritten),
            None => Cow::Borrowed(s),
        };
    }
    if let Some(key) = exact(s) {
        return Cow::Borrowed(super::text_in(locale, key));
    }
    if let Some((key, n)) = watchdog(s) {
        return Cow::Owned(key.fill_in(locale, &[&n]));
    }
    // Router step: `router_step_progress(n, m)`.
    if let Some((n, m)) = ht::parse_router_step_progress(s) {
        return Cow::Owned(Key::HostRouterStep.fill_in(locale, &[&n, &m]));
    }
    // An MCP summary contains user arguments. Translate only its fixed
    // prefix and separator, never an argument or tool name.
    if let Some(rest) = s.strip_prefix(ht::MCP_CALL_SUMMARY_PREFIX) {
        let prefix = super::text_in(locale, Key::ViewCallPrefix);
        let tail = match rest.split_once(ht::MCP_CALL_ARGS_SEPARATOR) {
            Some((tool, args)) => {
                let sep = super::text_in(locale, Key::ViewCallArgsSep);
                format!("{tool}{sep}{args}")
            }
            None => rest.to_string(),
        };
        return Cow::Owned(format!("{prefix}{tail}"));
    }
    // robroz-card `failure_line`: 「没做成：原因」 (its prefix, from the card
    // crate itself).
    let failed = robroz_card::failure_line("");
    let prefixes = PREFIXES
        .iter()
        .copied()
        .chain([(failed.as_str(), Key::HostFailedBecause)]);
    for (prefix, key) in prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            if rest.is_empty() {
                continue;
            }
            let inner = host_text_in(locale, rest);
            return Cow::Owned(key.fill_in(locale, &[&inner]));
        }
    }
    // A bare prefix is still a known fragment; don't leak its Chinese
    // when shown alone (the contract's coverage test exercises this too).
    for (prefix, key) in PREFIXES {
        if s == *prefix {
            return Cow::Owned(key.fill_in(locale, &[&""]));
        }
    }
    Cow::Borrowed(s)
}
