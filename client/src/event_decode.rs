//! Decode host events.
//!
//! `reason` and `expiresAt` live on `Event::Approval`. Unknown `action`
//! values parse as [`robroz_contract::ApprovalAction::Unknown`]. `rule_add`
//! / `rule_delete` are on the contract and parse as themselves.

use robroz_contract::Event;
use robroz_ui::cards::ApprovalExtras;
use serde_json::Value;

/// Decode one event. Unknown event *types* still fail (the caller skips
/// them).
///
/// A `failureCode` / `progressCode` / `reasonCode` this client does not
/// know is logged and dropped first, so the card still shows (with the
/// unknown-code copy) instead of the whole event failing to decode.
pub fn parse_event(mut value: Value) -> Result<(Event, Option<ApprovalExtras>), serde_json::Error> {
    robroz_ui::cards::codes::strip_unknown_codes(&mut value);
    let event = serde_json::from_value(value)?;
    Ok((event, None))
}

pub fn parse_event_str(body: &str) -> Result<(Event, Option<ApprovalExtras>), serde_json::Error> {
    parse_event(serde_json::from_str(body)?)
}

/// Settings-page rule cards must not appear in any chat, stream or unread.
pub fn is_settings_approval(event: &Event) -> bool {
    matches!(
        event,
        Event::Approval { bot_id, .. } if bot_id == robroz_contract::SETTINGS_APPROVAL_BOT_ID
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use robroz_contract::{ApprovalAction, ApprovalDecision, Event, SETTINGS_APPROVAL_BOT_ID};
    use robroz_ui::cards::{review_rules, ActionKind, ApprovalCard, ApprovalState};

    fn ev(json: &str) -> (Event, Option<ApprovalExtras>) {
        parse_event_str(json).expect("parse")
    }

    fn approval_json(action: &str, extra: &str) -> String {
        format!(
            r#"{{"type":"approval","id":"a1","botId":"bot","botName":"文档哥","action":"{action}","summary":"允许发周报","path":null,"decision":null,"createdAt":"2026-11-20T08:00:00Z","answeredAt":null{extra}}}"#
        )
    }

    #[test]
    fn known_approval_is_unchanged() {
        let (event, extras) = ev(&approval_json("call", ""));
        let Event::Approval {
            action,
            reason,
            expires_at,
            ..
        } = &event
        else {
            panic!("not approval");
        };
        assert_eq!(*action, ApprovalAction::Call);
        assert_eq!(reason, &None);
        assert_eq!(expires_at, &None);
        assert!(extras.is_none());
        let card = ApprovalCard::from_event(&event).unwrap();
        assert_eq!(card.kind, ActionKind::Call);
    }

    #[test]
    fn rule_add_parses_as_a_rule_card() {
        let (event, extras) = ev(&approval_json("rule_add", r#","reason":null"#));
        assert!(extras.is_none());
        let Event::Approval { action, .. } = &event else {
            panic!("not approval");
        };
        assert_eq!(*action, ApprovalAction::RuleAdd);
        let card = ApprovalCard::from_event(&event).unwrap();
        assert_eq!(card.kind, ActionKind::RuleAdd);
        assert_eq!(card.summary, "允许发周报");
        assert_eq!(card.state, ApprovalState::Pending);
    }

    #[test]
    fn rule_delete_parses() {
        let (event, extras) = ev(&approval_json("rule_delete", ""));
        assert!(extras.is_none());
        let card = ApprovalCard::from_event(&event).unwrap();
        assert_eq!(card.kind, ActionKind::RuleDelete);
    }

    #[test]
    fn settings_bot_id_is_a_settings_approval() {
        let json = format!(
            r#"{{"type":"approval","id":"a1","botId":"{SETTINGS_APPROVAL_BOT_ID}","botName":"设置","action":"rule_add","summary":"允许发周报","path":null,"decision":null,"createdAt":"2026-11-20T08:00:00Z","answeredAt":null}}"#
        );
        let (event, _) = ev(&json);
        assert!(is_settings_approval(&event));
        assert!(!is_settings_approval(&ev(&approval_json("call", "")).0));
    }

    #[test]
    fn unknown_action_is_a_generic_card() {
        let (event, extras) = ev(&approval_json("shell", r#","reason":"要改生产""#));
        assert!(extras.is_none());
        let Event::Approval { action, reason, .. } = &event else {
            panic!("not approval");
        };
        assert_eq!(*action, ApprovalAction::Unknown);
        assert_eq!(reason.as_deref(), Some("要改生产"));
        let card = ApprovalCard::from_event(&event).unwrap();
        assert_eq!(card.kind, ActionKind::Unknown);
        assert_eq!(card.reason.as_deref(), Some("要改生产"));
        let raw = approval_json("shell", "");
        let raw_event: Event = serde_json::from_str(&raw).unwrap();
        assert!(matches!(
            raw_event,
            Event::Approval {
                action: ApprovalAction::Unknown,
                ..
            }
        ));
    }

    #[test]
    fn expired_reason_becomes_expired_state() {
        let json = format!(
            r#"{{"type":"approval","id":"a1","botId":"bot","botName":"文档哥","action":"call","summary":"发周报","path":null,"decision":"denied","createdAt":"2026-11-20T08:00:00Z","answeredAt":"2026-11-21T08:00:00Z","reason":"{}","expiresAt":"2026-11-21T08:00:00Z"}}"#,
            review_rules::APPROVAL_EXPIRED_REASON
        );
        let (event, extras) = ev(&json);
        assert!(extras.is_none());
        let Event::Approval {
            decision: Some(ApprovalDecision::Denied),
            reason,
            expires_at,
            ..
        } = &event
        else {
            panic!("expected denied event");
        };
        assert_eq!(
            reason.as_deref(),
            Some(review_rules::APPROVAL_EXPIRED_REASON)
        );
        assert!(expires_at.is_some());
        let card = ApprovalCard::from_event(&event).unwrap();
        assert!(matches!(card.state, ApprovalState::Expired { .. }));
    }

    #[test]
    fn approval_reminder_decodes_and_the_owner_fields_reach_the_card() {
        let (event, _) = ev(
            r#"{"type":"approval_reminder","id":"reminder:a1","approvalId":"a1",
            "groupId":"g1","groupName":"Site","botId":"b1","botName":"Ada","ownerUserId":"u1",
            "ownerName":"Panda","state":"waiting","createdAt":"2026-10-07T19:00:00.000Z"}"#,
        );
        assert!(matches!(event, Event::ApprovalReminder { .. }));
        let (event, _) = ev(
            r#"{"type":"approval","id":"a1","botId":"b1","botName":"Ada",
            "action":"delete","summary":"x","path":null,"decision":null,
            "createdAt":"2026-10-07T19:00:00.000Z","answeredAt":null,"answeredBy":null,
            "ownerUserId":"u1","ownerName":"Panda"}"#,
        );
        let card = ApprovalCard::from_event(&event).unwrap();
        assert_eq!(card.owner_id.as_deref(), Some("u1"));
        assert_eq!(card.owner_name.as_deref(), Some("Panda"));
    }

    #[test]
    fn unknown_codes_still_decode_and_fall_back() {
        use robroz_ui::cards::codes::{reason_text_in, FailureLine, ProgressLine};
        use robroz_ui::i18n::Locale;
        let (event, _) = ev(
            r#"{"type":"status","id":"t","taskId":"t","botId":"b","botName":"Ada",
            "state":"done","label":"完成","createdAt":"2026-11-20T08:00:00Z",
            "updatedAt":"2026-11-20T08:00:01Z","containerUp":false,
            "failure":"新的失败","failureCode":"quantum_flux","failureDetail":"raw",
            "progress":"x","progressCode":"teleporting"}"#,
        );
        let Event::Status {
            failure_code,
            failure_detail,
            failure,
            progress_code,
            ..
        } = &event
        else {
            panic!("not status");
        };
        assert_eq!((*failure_code, failure_detail.as_deref()), (None, None));
        let line = FailureLine::from_wire_in(Locale::Zh, None, None, failure.as_deref()).unwrap();
        assert_eq!(line.text, "出错了");
        assert_eq!(*progress_code, None);
        let p = ProgressLine::from_wire(None, None, Some("x")).unwrap();
        assert_eq!(p.text_in(Locale::En), "In progress");

        let (event, _) = ev(&approval_json(
            "call",
            r#","reason":"r","reasonCode":"cosmic""#,
        ));
        let Event::Approval {
            reason,
            reason_code,
            ..
        } = &event
        else {
            panic!("not approval");
        };
        assert_eq!(*reason_code, None);
        assert_eq!(
            reason_text_in(Locale::En, *reason_code, reason.as_deref()).as_deref(),
            Some("Something went wrong")
        );
        let (event, _) = ev(&approval_json(
            "call",
            r#","reason":"r","reasonCode":"review_no_match""#,
        ));
        let card = ApprovalCard::from_event(&event).unwrap();
        assert_eq!(
            reason_text_in(Locale::Zh, card.reason_code, card.reason.as_deref()).as_deref(),
            Some("没有放行规则允许这个操作")
        );
    }

    #[test]
    fn a_started_at_field_on_status_json_is_not_read() {
        // contract-v0.3.0 has no startedAt; a future host may send one.
        // This PR does not parse it (that moves to the v0.3.1 follow-up).
        let json = r#"{"type":"status","id":"t","taskId":"t","botId":"b","botName":"Ada",
            "state":"running","label":"进行中","createdAt":"2026-11-20T08:00:00Z",
            "updatedAt":"2026-11-20T08:00:12Z","containerUp":true,
            "startedAt":"2026-11-20T08:00:01Z"}"#;
        let value: serde_json::Value = serde_json::from_str(json).expect("json");
        assert_eq!(
            value.get("startedAt").and_then(|v| v.as_str()),
            Some("2026-11-20T08:00:01Z")
        );
        let (event, extras) = ev(json);
        assert!(matches!(event, Event::Status { .. }));
        assert!(extras.is_none());
        // The decode result is only the contract event; no startedAt is returned.
        let (event, extras) = parse_event_str(json).expect("parse");
        assert!(matches!(event, Event::Status { .. }));
        assert!(extras.is_none());
    }
}
