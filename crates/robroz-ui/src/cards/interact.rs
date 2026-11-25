//! Local interaction state for the new cards, and which look each button
//! gets. Pure and UI-free so the rules in SPEC §4–§6 are unit-tested.
//!
//! The server stays the source of truth: a card's state only changes when
//! an event (or the answer endpoint's reply) arrives. This module tracks
//! what is in flight and what to say if it failed.

use super::status::{self, Dot};
use super::{ApprovalCard, ApprovalState, ChoiceCard, ChoiceState, DraftCard, DraftState};
use crate::i18n::{host_text, Key};

/// How one button is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// Orange, at most one per card.
    Primary,
    /// Chat cards' primary: ink on light, inverted on dark (`Skin::chat`).
    Ink,
    /// Outlined.
    Secondary,
    /// Picked: sunken fill, strong outline, check mark, bold.
    Selected,
    /// Faded to the theme's disabled alpha and not clickable.
    Disabled,
    /// Red filled button: 「删除规则」 on a rule-delete confirm card.
    Danger,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionButton {
    pub id: String,
    pub label: String,
    pub look: Look,
    /// Multi-select open card: `Some(checked)` draws the 14×14 box.
    pub checkbox: Option<bool>,
    pub clickable: bool,
}

/// Why the last submit did not go through, shown as a `text3` footnote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Note {
    /// Network or server error: try again.
    Retry,
    /// 409: someone answered first; the stream will bring the answer.
    AlreadyAnswered,
    /// 409 on an approval card: it was approved or denied already.
    AlreadyDecided,
    /// 400: the host refused the pick (e.g. the option is gone).
    Rejected(String),
    /// Any other failure with the host's message.
    Failed(String),
    /// 403 `approval_forbidden` + `not_owner`: only the bro's owner may
    /// decide. Holds the owner's name when the host or the card gave one.
    /// Stays until the card changes; the buttons are gone.
    OnlyOwner(Option<String>),
    /// 403 `decided` / `expired`: not an error. No text; the buttons go
    /// while the real card is fetched, which then shows the result.
    Settled,
    /// Already-localised copy (a coded host error, or only the bro's owner
    /// may change it). Shown as-is; not sent through `host_text`.
    Copy(String),
}

impl Note {
    pub fn text(&self) -> String {
        match self {
            Note::Retry => status::RETRY_NOTE.into(),
            Note::AlreadyAnswered => Key::InteractAlreadyPicked.into(),
            Note::AlreadyDecided => Key::InteractAlreadyDecided.into(),
            Note::Rejected(msg) => Key::InteractRejected.fill(&[&host_text(msg)]),
            Note::Failed(msg) => Key::InteractFailed.fill(&[&host_text(msg)]),
            Note::OnlyOwner(owner) => Key::ApprovalOnlyOwner
                .fill(&[&crate::cards::group_approval::owner_label(owner.as_deref())]),
            Note::Settled => String::new(),
            Note::Copy(msg) => msg.clone(),
        }
    }
}

/// Per-card UI state, keyed by card id in the app.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardUi {
    /// Multi-select picks before 确认, in option order.
    pub picks: Vec<String>,
    /// What was sent and is waiting for the host.
    pub submitting: Option<Vec<String>>,
    /// Approval / draft action in flight.
    pub busy: bool,
    pub note: Option<Note>,
    /// Narrow screens: the user tapped the shortened bot name to see it all.
    pub name_expanded: bool,
    /// Skill confirm card: the full instruction is open.
    pub more_open: bool,
    /// The card's stroke is accent for a moment after a jump from the
    /// owner's DM reminder row (300 ms).
    pub flash: bool,
}

impl CardUi {
    /// Toggle a multi-select pick, keeping the card's option order.
    pub fn toggle_pick(&mut self, card: &ChoiceCard, option: &str) {
        if self.submitting.is_some() || !matches!(card.state, ChoiceState::Open) {
            return;
        }
        if let Some(i) = self.picks.iter().position(|p| p == option) {
            self.picks.remove(i);
        } else if card.options.iter().any(|o| o.id == option) {
            self.picks.push(option.to_string());
            self.picks
                .sort_by_key(|p| card.options.iter().position(|o| &o.id == p));
        }
        self.note = None;
    }

    /// Start sending a choice. Returns the body ids, or `None` if nothing
    /// should be sent (already in flight, not open, or empty multi pick).
    pub fn begin_choice(&mut self, card: &ChoiceCard, single: Option<&str>) -> Option<Vec<String>> {
        if self.submitting.is_some() || !matches!(card.state, ChoiceState::Open) {
            return None;
        }
        let ids = match (card.multi, single) {
            (false, Some(id)) if card.options.iter().any(|o| o.id == id) => vec![id.to_string()],
            (true, None) if !self.picks.is_empty() => self.picks.clone(),
            _ => return None,
        };
        self.submitting = Some(ids.clone());
        self.note = None;
        Some(ids)
    }

    /// The host accepted the choice.
    pub fn choice_sent(&mut self) {
        self.submitting = None;
        self.picks.clear();
        self.note = None;
    }

    /// The host did not accept it: back to Open with a footnote.
    pub fn choice_failed(&mut self, note: Note) {
        self.submitting = None;
        self.note = Some(note);
    }

    /// Start an approval/draft action; false if one is already in flight.
    pub fn begin_action(&mut self) -> bool {
        if self.busy {
            return false;
        }
        self.busy = true;
        self.note = None;
        true
    }

    pub fn action_done(&mut self, note: Option<Note>) {
        self.busy = false;
        self.note = note;
    }
}

/// Buttons for a choice card, in option order (SPEC §4).
pub fn choice_buttons(card: &ChoiceCard, ui: &CardUi) -> Vec<OptionButton> {
    let primary = card.primary_option();
    card.options
        .iter()
        .map(|o| {
            let (look, checkbox, clickable) = match &card.state {
                // 409: someone already picked; lock until the answer lands.
                ChoiceState::Open if ui.note == Some(Note::AlreadyAnswered) => {
                    (Look::Disabled, None, false)
                }
                ChoiceState::Open => match (&ui.submitting, card.multi) {
                    (Some(sent), false) => {
                        if sent.contains(&o.id) {
                            (Look::Selected, None, false)
                        } else {
                            (Look::Disabled, None, false)
                        }
                    }
                    (Some(sent), true) => {
                        let on = sent.contains(&o.id);
                        (
                            if on { Look::Secondary } else { Look::Disabled },
                            Some(on),
                            false,
                        )
                    }
                    (None, false) => {
                        let look = if primary == Some(o.id.as_str()) {
                            Look::Primary
                        } else {
                            Look::Secondary
                        };
                        (look, None, true)
                    }
                    (None, true) => (Look::Secondary, Some(ui.picks.contains(&o.id)), true),
                },
                ChoiceState::Answered { selected, .. } => {
                    if selected.contains(&o.id) {
                        (Look::Selected, None, false)
                    } else {
                        (Look::Disabled, None, false)
                    }
                }
                ChoiceState::Expired => (Look::Disabled, None, false),
            };
            OptionButton {
                id: o.id.clone(),
                label: o.label.clone(),
                look,
                checkbox,
                clickable,
            }
        })
        .collect()
}

/// The 确认（n） button on an open multi card: label and whether it can be
/// pressed. `None` when the card has no confirm button.
pub fn confirm_button(card: &ChoiceCard, ui: &CardUi) -> Option<(String, bool)> {
    if !card.multi || !matches!(card.state, ChoiceState::Open) {
        return None;
    }
    let n = ui.submitting.as_ref().map_or(ui.picks.len(), Vec::len);
    let locked = ui.submitting.is_some() || ui.note == Some(Note::AlreadyAnswered);
    Some((status::confirm_label(n), n > 0 && !locked))
}

/// What an approval card shows at the bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalActions {
    /// 批准 (primary) + 拒绝 (secondary); `enabled` false while in flight.
    Decide {
        enabled: bool,
    },
    /// Only 去私聊接管.
    TakeOver,
    None,
}

pub fn approval_actions(card: &ApprovalCard, ui: &CardUi) -> ApprovalActions {
    // 409: answered elsewhere. Nothing left to press while the answered
    // card is fetched; the note says why.
    if matches!(
        ui.note,
        Some(Note::AlreadyDecided | Note::Settled | Note::OnlyOwner(_))
    ) {
        return ApprovalActions::None;
    }
    match card.state {
        ApprovalState::Pending => ApprovalActions::Decide { enabled: !ui.busy },
        ApprovalState::NeedsOwner => ApprovalActions::TakeOver,
        ApprovalState::Approved { .. }
        | ApprovalState::Denied { .. }
        | ApprovalState::Expired { .. } => ApprovalActions::None,
    }
}

/// Draft: 发送 (primary) + 修改 (secondary) while pending, and only when
/// `live` (the fake source). Drafts are phase 2: against a real host the
/// buttons are not drawn at all (PM decision on #24).
pub fn draft_actions(card: &DraftCard, ui: &CardUi, live: bool) -> Option<bool> {
    (live && matches!(card.state, DraftState::Pending)).then_some(!ui.busy)
}

/// Every card draws at most one orange button. Used by tests and as a
/// debug check in the view.
pub fn primary_count(buttons: &[OptionButton], confirm: Option<&(String, bool)>) -> usize {
    buttons.iter().filter(|b| b.look == Look::Primary).count() + usize::from(confirm.is_some())
}

/// Dot shown on the card right now.
pub fn choice_dot(card: &ChoiceCard) -> Dot {
    status::choice_status(&card.state).1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::tests::opt;

    fn single() -> ChoiceCard {
        ChoiceCard {
            id: "c1".into(),
            bot: "文档哥".into(),
            prompt: "安装说明先写哪一种？".into(),
            options: vec![
                opt("docker", "Docker 一键", true),
                opt("cargo", "cargo install", false),
                opt("both", "两种都写", false),
            ],
            multi: false,
            state: ChoiceState::Open,
        }
    }

    fn multi() -> ChoiceCard {
        ChoiceCard {
            id: "c2".into(),
            bot: "前端".into(),
            prompt: "第一期客户端先做哪几页？".into(),
            options: vec![
                opt("chat", "群聊", true),
                opt("routines", "例行任务", false),
                opt("settings", "设置", false),
                opt("files", "文件", false),
            ],
            multi: true,
            state: ChoiceState::Open,
        }
    }

    fn looks(b: &[OptionButton]) -> Vec<Look> {
        b.iter().map(|b| b.look).collect()
    }

    #[test]
    fn open_single_has_one_primary_and_all_clickable() {
        let card = single();
        let b = choice_buttons(&card, &CardUi::default());
        assert_eq!(looks(&b), [Look::Primary, Look::Secondary, Look::Secondary]);
        assert!(b.iter().all(|b| b.clickable && b.checkbox.is_none()));
        assert_eq!(primary_count(&b, None), 1);
    }

    #[test]
    fn two_primaries_from_the_host_still_draw_one() {
        let mut card = single();
        card.options[2].primary = true;
        let b = choice_buttons(&card, &CardUi::default());
        assert_eq!(primary_count(&b, None), 1);
    }

    #[test]
    fn single_click_submits_once_and_locks() {
        let card = single();
        let mut ui = CardUi::default();
        assert_eq!(
            ui.begin_choice(&card, Some("both")),
            Some(vec!["both".to_string()])
        );
        assert_eq!(
            ui.begin_choice(&card, Some("cargo")),
            None,
            "no double submit"
        );
        let b = choice_buttons(&card, &ui);
        assert_eq!(looks(&b), [Look::Disabled, Look::Disabled, Look::Selected]);
        assert!(b.iter().all(|b| !b.clickable));
    }

    #[test]
    fn failure_restores_open_with_retry_note() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let card = single();
        let mut ui = CardUi::default();
        ui.begin_choice(&card, Some("cargo"));
        ui.choice_failed(Note::Retry);
        assert_eq!(ui.note.as_ref().unwrap().text(), "没发出去，再点一次");
        let b = choice_buttons(&card, &ui);
        assert_eq!(looks(&b), [Look::Primary, Look::Secondary, Look::Secondary]);
        assert!(b.iter().all(|b| b.clickable));
    }

    #[test]
    fn unknown_option_is_not_sent() {
        let card = single();
        let mut ui = CardUi::default();
        assert_eq!(ui.begin_choice(&card, Some("nope")), None);
    }

    #[test]
    fn multi_picks_keep_order_and_confirm_counts() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let card = multi();
        let mut ui = CardUi::default();
        assert_eq!(
            confirm_button(&card, &ui),
            Some(("确认（0）".into(), false))
        );
        ui.toggle_pick(&card, "routines");
        ui.toggle_pick(&card, "chat");
        assert_eq!(ui.picks, ["chat", "routines"]);
        let b = choice_buttons(&card, &ui);
        assert!(
            b.iter().all(|b| b.look == Look::Secondary),
            "no primary on multi"
        );
        assert_eq!(
            b.iter().map(|b| b.checkbox).collect::<Vec<_>>(),
            [Some(true), Some(true), Some(false), Some(false)]
        );
        let confirm = confirm_button(&card, &ui);
        assert_eq!(confirm, Some(("确认（2）".into(), true)));
        assert_eq!(primary_count(&b, confirm.as_ref()), 1);
        // A single click does nothing on a multi card; only 确认 sends.
        assert_eq!(ui.begin_choice(&card, Some("chat")), None);
        assert_eq!(
            ui.begin_choice(&card, None),
            Some(vec!["chat".to_string(), "routines".to_string()])
        );
        assert_eq!(
            confirm_button(&card, &ui),
            Some(("确认（2）".into(), false))
        );
        ui.toggle_pick(&card, "files");
        assert_eq!(ui.picks.len(), 2, "picks frozen while sending");
    }

    #[test]
    fn empty_multi_cannot_submit() {
        let card = multi();
        let mut ui = CardUi::default();
        assert_eq!(ui.begin_choice(&card, None), None);
    }

    #[test]
    fn answered_shows_selected_others_disabled() {
        let mut card = single();
        card.state = ChoiceState::Answered {
            selected: vec!["both".into()],
            at: None,
            by: None,
        };
        let b = choice_buttons(&card, &CardUi::default());
        assert_eq!(looks(&b), [Look::Disabled, Look::Disabled, Look::Selected]);
        assert_eq!(primary_count(&b, None), 0, "primary goes back to normal");
        let mut m = multi();
        m.state = ChoiceState::Answered {
            selected: vec!["chat".into(), "routines".into()],
            at: None,
            by: None,
        };
        assert_eq!(confirm_button(&m, &CardUi::default()), None);
    }

    #[test]
    fn expired_all_disabled_none_selected() {
        let mut card = single();
        card.state = ChoiceState::Expired;
        let b = choice_buttons(&card, &CardUi::default());
        assert!(b.iter().all(|b| b.look == Look::Disabled && !b.clickable));
    }

    #[test]
    fn conflict_locks_open_choice() {
        let mut ui = CardUi::default();
        ui.choice_failed(Note::AlreadyAnswered);
        let b = choice_buttons(&single(), &ui);
        assert!(b.iter().all(|b| b.look == Look::Disabled && !b.clickable));
        let card = multi();
        let mut ui = CardUi::default();
        ui.toggle_pick(&card, &card.options[0].id.clone());
        ui.choice_failed(Note::AlreadyAnswered);
        assert!(choice_buttons(&card, &ui).iter().all(|b| !b.clickable));
        assert!(!confirm_button(&card, &ui).unwrap().1);
    }

    #[test]
    fn approval_and_draft_actions() {
        let mut a = ApprovalCard {
            id: "a1".into(),
            bot: "后端".into(),
            kind: crate::cards::ActionKind::Delete,
            summary: "清理 12 个旧日志文件".into(),
            path: Some("logs/2026-10/".into()),
            state: ApprovalState::Pending,
            mcp_server: None,
            mcp_tool: None,
            reason: None,
            owner_id: None,
            owner_name: None,
        };
        let mut ui = CardUi::default();
        assert_eq!(
            approval_actions(&a, &ui),
            ApprovalActions::Decide { enabled: true }
        );
        assert!(ui.begin_action());
        assert!(!ui.begin_action(), "one action at a time");
        assert_eq!(
            approval_actions(&a, &ui),
            ApprovalActions::Decide { enabled: false }
        );
        a.state = ApprovalState::NeedsOwner;
        assert_eq!(approval_actions(&a, &ui), ApprovalActions::TakeOver);
        a.state = ApprovalState::Expired { at: 0 };
        assert_eq!(approval_actions(&a, &ui), ApprovalActions::None);
        // 409 on a pending card: no buttons while the answer is fetched.
        a.state = ApprovalState::Pending;
        ui.action_done(Some(Note::AlreadyDecided));
        assert_eq!(approval_actions(&a, &ui), ApprovalActions::None);
        a.state = ApprovalState::Approved { by: None, at: 0 };
        assert_eq!(approval_actions(&a, &ui), ApprovalActions::None);

        let mut d = DraftCard {
            id: "d1".into(),
            bot: "文档哥".into(),
            channel: crate::cards::DraftChannel::Email,
            to: "team@example.com".into(),
            subject: Some("robroz 第一期进度".into()),
            body: "大家好".into(),
            state: DraftState::Pending,
        };
        let ui = CardUi::default();
        assert_eq!(draft_actions(&d, &ui, true), Some(true));
        assert_eq!(draft_actions(&d, &ui, false), None, "real host: no buttons");
        d.state = DraftState::Sent {
            by: "Panda".into(),
            at: 0,
        };
        assert_eq!(draft_actions(&d, &ui, true), None);
    }
}
