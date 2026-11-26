//! Status text, dot colour, labels and footnotes for the new cards, as pure
//! functions (SPEC §8: "客户端只画，不自己挑颜色"). The view never picks a
//! colour or a string for a state on its own.

use crate::i18n::Key;
use chrono::{DateTime, Datelike, NaiveDateTime, TimeZone};

use super::{ActionKind, ApprovalState, ChoiceCard, ChoiceState, DraftChannel, DraftState};

/// The 8px dot at the top right of a new card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dot {
    /// Waiting on the person: 等你选 / 等你批准 / 草稿待确认.
    Waiting,
    /// 已选 / 已批准 / 已发送.
    Done,
    /// 已失效 / 已拒绝 / 已放弃.
    Void,
    /// 卡住: needs the owner's own sign-in.
    Stuck,
}

pub fn choice_status(state: &ChoiceState) -> (&'static str, Dot) {
    match state {
        ChoiceState::Open => (Key::StatusChoiceOpen.as_str(), Dot::Waiting),
        ChoiceState::Answered { .. } => (Key::StatusChoiceAnswered.as_str(), Dot::Done),
        ChoiceState::Expired => (Key::StatusChoiceExpired.as_str(), Dot::Void),
    }
}

pub fn approval_status(state: &ApprovalState) -> (&'static str, Dot) {
    match state {
        ApprovalState::Pending => (Key::StatusApprovalPending.as_str(), Dot::Waiting),
        ApprovalState::Approved { .. } => (Key::StatusApprovalApproved.as_str(), Dot::Done),
        ApprovalState::Denied { .. } => (Key::StatusApprovalDenied.as_str(), Dot::Void),
        ApprovalState::NeedsOwner => (Key::StatusApprovalStuck.as_str(), Dot::Stuck),
        // Grey Void, same as other settled-void states; never the denied copy.
        ApprovalState::Expired { .. } => (crate::cards::review_rules::EXPIRED.as_str(), Dot::Void),
    }
}

pub fn draft_status(state: &DraftState) -> (&'static str, Dot) {
    match state {
        DraftState::Pending => (Key::StatusDraftPending.as_str(), Dot::Waiting),
        DraftState::Sent { .. } => (Key::StatusDraftSent.as_str(), Dot::Done),
        DraftState::Discarded { .. } => (Key::StatusDraftDiscarded.as_str(), Dot::Void),
    }
}

pub fn action_kind_label(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Delete => Key::StatusKindDelete.as_str(),
        ActionKind::Send => Key::StatusKindSend.as_str(),
        ActionKind::Spend => Key::StatusKindSpend.as_str(),
        ActionKind::Call => Key::StatusKindCall.as_str(),

        ActionKind::Unknown => Key::StatusKindUnknown.as_str(),
        ActionKind::RuleAdd | ActionKind::RuleDelete => "",
    }
}

pub fn channel_label(channel: DraftChannel) -> &'static str {
    match channel {
        DraftChannel::Email => Key::StatusChannelEmail.as_str(),
        DraftChannel::Message => Key::StatusChannelMessage.as_str(),
    }
}

/// The only action a stuck card offers (SPEC §0).
pub const TAKE_OVER: Key = Key::StatusTakeOver;
pub const NEEDS_OWNER_LINE: Key = Key::StatusNeedsOwnerLine;
pub const MULTI_HINT: Key = Key::StatusMultiHint;
pub const RETRY_NOTE: Key = Key::StatusRetryNote;
pub const PAUSE_FAILED_NOTE: Key = Key::StatusPauseFailedNote;

pub fn confirm_label(picked: usize) -> String {
    Key::StatusConfirmPicked.fill(&[&picked])
}

/// Local `HH:MM` for a Unix-seconds timestamp.
pub fn clock_in<Tz: TimeZone>(unix: i64, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    match tz.timestamp_opt(unix, 0).single() {
        Some(t) => t.format("%H:%M").to_string(),
        None => String::new(),
    }
}

/// [`clock_in`] in the computer's local zone.
pub fn clock_local(unix: i64) -> String {
    clock_in(unix, &chrono::Local)
}

/// Local `HH:MM` for an ISO 8601 contract timestamp; empty if unparsable.
pub fn clock_iso_in<Tz: TimeZone>(iso: &str, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    DateTime::parse_from_rfc3339(iso)
        .map(|t| t.with_timezone(tz).format("%H:%M").to_string())
        .unwrap_or_default()
}

fn with_time(text: String, time: &str) -> String {
    if time.is_empty() {
        text
    } else {
        format!("{text} · {time}")
    }
}

/// Footnote under a choice card; `tz` is the computer's local zone.
///
/// Answered cards read 「X 选了「…」 · HH:MM」 when the host knows who
/// answered (「我」 for this client), else 「已选「…」 · HH:MM」.
pub fn choice_footnote<Tz: TimeZone>(card: &ChoiceCard, tz: &Tz) -> Option<String>
where
    Tz::Offset: std::fmt::Display,
{
    match &card.state {
        ChoiceState::Open => None,
        ChoiceState::Expired => Some(Key::StatusChoiceExpiredFoot.into()),
        ChoiceState::Answered { selected, at, by } => {
            let labels = selected
                .iter()
                .map(|id| card.label_of(id).unwrap_or(id).to_string())
                .collect::<Vec<_>>()
                .join(Key::StatusListSeparator.as_str());
            let time = at
                .as_deref()
                .map(|a| clock_iso_in(a, tz))
                .unwrap_or_default();
            let text = match by {
                Some(by) => Key::StatusChoiceByFoot.fill(&[&super::who(by), &labels]),
                None => Key::StatusChoiceFoot.fill(&[&labels]),
            };
            Some(with_time(text, &time))
        }
    }
}

/// Decision text and optional local `HH:MM` under an answered approval
/// card. The view draws these as two separate spans; there is no `·`
/// between them. Approved copy is 「我已批准」 / 「{name}已批准」 / 「已批准」.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalFootnote {
    pub text: String,
    pub time: Option<String>,
}

impl ApprovalFootnote {
    fn new(text: String, time: String) -> Self {
        Self {
            text,
            time: if time.is_empty() { None } else { Some(time) },
        }
    }
}

pub fn approval_footnote<Tz: TimeZone>(state: &ApprovalState, tz: &Tz) -> Option<ApprovalFootnote>
where
    Tz::Offset: std::fmt::Display,
{
    match state {
        ApprovalState::Approved { by, at } => {
            let text = by.as_ref().map_or(Key::StatusApprovedFoot.into(), |b| {
                Key::StatusApprovedByFoot.fill(&[&super::who(b)])
            });
            Some(ApprovalFootnote::new(text, clock_in(*at, tz)))
        }
        ApprovalState::Denied { by, at } => {
            let text = by.as_ref().map_or(Key::StatusDeniedFoot.into(), |b| {
                Key::StatusDeniedByFoot.fill(&[&super::who(b)])
            });
            Some(ApprovalFootnote::new(text, clock_in(*at, tz)))
        }
        ApprovalState::Expired { at } => Some(ApprovalFootnote::new(
            crate::cards::review_rules::EXPIRED_FOOT.into(),
            clock_in(*at, tz),
        )),
        ApprovalState::Pending | ApprovalState::NeedsOwner => None,
    }
}

pub fn draft_footnote<Tz: TimeZone>(state: &DraftState, tz: &Tz) -> Option<String>
where
    Tz::Offset: std::fmt::Display,
{
    match state {
        DraftState::Sent { by, at } => Some(with_time(
            Key::StatusDraftSentFoot.fill(&[&super::who(by)]),
            &clock_in(*at, tz),
        )),
        DraftState::Discarded { by, at } => Some(with_time(
            Key::StatusDraftDiscardedFoot.fill(&[&super::who(by)]),
            &clock_in(*at, tz),
        )),
        DraftState::Pending => None,
    }
}

/// Preview lines for a draft: 3 while pending, 1 once settled (SPEC §6).
pub fn draft_preview_lines(state: &DraftState) -> usize {
    match state {
        DraftState::Pending => 3,
        _ => 1,
    }
}

/// Next-run text for the routines list (SPEC §7): 今天 23:00 / 明天 08:00 /
/// 周一 09:30 within a week, otherwise 11月30日 08:00. Both in local time.
pub fn format_next_run(next: NaiveDateTime, now: NaiveDateTime) -> String {
    let days = (next.date() - now.date()).num_days();
    let hm = next.format("%H:%M");
    match days {
        0 => Key::StatusToday.fill(&[&hm]),
        1 => Key::StatusTomorrow.fill(&[&hm]),
        2..=6 => {
            const WEEK: [Key; 7] = [
                Key::StatusMon,
                Key::StatusTue,
                Key::StatusWed,
                Key::StatusThu,
                Key::StatusFri,
                Key::StatusSat,
                Key::StatusSun,
            ];
            let w = WEEK[next.weekday().num_days_from_monday() as usize];
            format!("{w} {hm}")
        }
        _ => format!("{} {hm}", crate::i18n::month_day(next.month(), next.day())),
    }
}

/// Width units for a rough fit: CJK and full-width count 2, others 1.
fn units(c: char) -> usize {
    if c.is_ascii() {
        1
    } else {
        2
    }
}

/// Cut `text` to at most `max_lines` lines of `cols` width units each,
/// ending in "…" when anything was dropped. iced has no text ellipsis, so
/// the view estimates `cols` from the card width and font size.
pub fn clamp_lines(text: &str, cols: usize, max_lines: usize) -> String {
    let cols = cols.max(4);
    let budget = cols * max_lines.max(1);
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let total: usize = flat.chars().map(units).sum();
    if total <= budget {
        return flat;
    }
    let mut out = String::new();
    let mut used = 0;
    for c in flat.chars() {
        if used + units(c) > budget - 2 {
            break;
        }
        used += units(c);
        out.push(c);
    }
    out.push('…');
    out
}

/// WCAG relative luminance of an sRGB colour.
fn luminance(c: [u8; 3]) -> f32 {
    let ch = |v: u8| {
        let v = f32::from(v) / 255.0;
        if v <= 0.039_28 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
}

/// WCAG contrast ratio between two colours (1.0 to 21.0).
pub fn contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Colour of the progress line on a status card with this frozen palette.
/// It is the secondary text one step lighter than the page's grey small
/// text: on a dark fill (the running card) the dark skin's `text2`, on a
/// light fill the light skin's. When that misses the text contrast floor
/// on this fill, it falls back to the card's own text colour.
pub fn progress_rgb(fill: [u8; 3], text: [u8; 3]) -> [u8; 3] {
    use robroz_tokens::solid;
    let candidate = if luminance(fill) < 0.18 {
        solid::dark::palette::TEXT2
    } else {
        solid::light::palette::TEXT2
    };
    if contrast(candidate, fill) >= robroz_tokens::base::contrast::TEXT_MIN {
        candidate
    } else {
        text
    }
}

/// Narrow screens show at most this many characters of a bot name.
pub const NARROW_NAME_CHARS: usize = 8;

/// A bot name for a narrow screen: longer than [`NARROW_NAME_CHARS`]
/// characters (CJK counts one each) becomes the first ones plus "…".
/// Returns `None` when the name fits as is.
pub fn short_bot_name(name: &str) -> Option<String> {
    if name.chars().count() <= NARROW_NAME_CHARS {
        return None;
    }
    let mut s: String = name.chars().take(NARROW_NAME_CHARS).collect();
    s.push('…');
    Some(s)
}

/// Shorten a path from the middle to at most `max` width units (ASCII 1,
/// CJK 2), keeping its start and its last segment (the file name, or the
/// last directory with its trailing `/`): `logs/2026-10/app/x.log` →
/// `logs/…/x.log`. Falls back to a character-level middle cut when even
/// that is too wide. Paths that fit come back unchanged.
pub fn middle_ellipsis(path: &str, max: usize) -> String {
    let width = |s: &str| s.chars().map(units).sum::<usize>();
    if width(path) <= max {
        return path.to_string();
    }
    let trailing = path.ends_with('/');
    let parts: Vec<&str> = path.trim_end_matches('/').split('/').collect();
    if parts.len() >= 3 {
        let first = parts[0];
        let last = parts[parts.len() - 1];
        let candidate = format!("{first}/…/{last}{}", if trailing { "/" } else { "" });
        if width(&candidate) <= max {
            return candidate;
        }
    }
    // Keep as much of both ends as fits around one "…": the start gets
    // the larger half, the end the rest.
    let budget = max.saturating_sub(units('…')).max(2);
    let chars: Vec<char> = path.chars().collect();
    let mut head = String::new();
    let mut used = 0;
    let mut i = 0;
    while i < chars.len() && used + units(chars[i]) <= budget.div_ceil(2) {
        used += units(chars[i]);
        head.push(chars[i]);
        i += 1;
    }
    let mut tail = Vec::new();
    let mut j = chars.len();
    while j > i && used + units(chars[j - 1]) <= budget {
        used += units(chars[j - 1]);
        tail.push(chars[j - 1]);
        j -= 1;
    }
    tail.reverse();
    format!("{head}…{}", tail.into_iter().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_line_is_lighter_grey_and_legible_on_the_running_card() {
        use robroz_card::{appearance, palette, Tone};
        let ink = palette(appearance(robroz_contract::TaskState::Running, true).tone);
        let c = progress_rgb(ink.fill, ink.text);
        assert_eq!(c, robroz_tokens::solid::dark::palette::TEXT2);
        // Lighter than the grey small text the page uses on paper ...
        assert!(luminance(c) > luminance(robroz_tokens::solid::light::palette::TEXT2));
        assert!(luminance(c) > luminance(robroz_tokens::solid::dark::palette::TEXT3));
        // ... dimmer than the bro name, and readable on the dark fill.
        assert!(luminance(c) < luminance(ink.text));
        assert!(contrast(c, ink.fill) >= robroz_tokens::base::contrast::TEXT_MIN);
        for tone in [Tone::Gray, Tone::Ink, Tone::Green, Tone::Amber] {
            let p = palette(tone);
            let c = progress_rgb(p.fill, p.text);
            assert!(contrast(c, p.fill) >= 4.5, "{tone:?}");
        }
    }

    #[test]
    fn progress_fits_one_line_with_an_ellipsis() {
        let long = "在沙箱里跑：正在逐个检查日志文件并汇总出错的请求和时间点";
        let line = clamp_lines(long, 20, 1);
        assert!(line.ends_with('…'));
        assert!(line.chars().map(units).sum::<usize>() <= 20);
        assert_eq!(clamp_lines("开始干活", 20, 1), "开始干活");
    }
    use crate::cards::tests::opt;

    #[test]
    fn bot_names_cut_at_eight_chars() {
        assert_eq!(short_bot_name("文档哥"), None);
        assert_eq!(short_bot_name("最详尽开发文档哥"), None, "exactly 8");
        assert_eq!(
            short_bot_name("最详尽开发文档哥哥").as_deref(),
            Some("最详尽开发文档哥…")
        );
        assert_eq!(short_bot_name("GitHubBro!").as_deref(), Some("GitHubBr…"));
    }

    #[test]
    fn paths_keep_start_and_file_name() {
        assert_eq!(middle_ellipsis("logs/2026-10/", 40), "logs/2026-10/");
        assert_eq!(
            middle_ellipsis("logs/2026-10/app/server.log", 20),
            "logs/…/server.log"
        );
        assert_eq!(
            middle_ellipsis("workspace/cache/2026-10/", 22),
            "workspace/…/2026-10/"
        );
        let cut = middle_ellipsis("a_very_long_single_file_name.tar.gz", 16);
        assert!(cut.contains('…'));
        assert!(cut.starts_with("a_ve") && cut.ends_with(".gz"), "{cut}");
        assert!(cut.chars().map(units).sum::<usize>() <= 16);
    }
    use chrono::{NaiveDate, Utc};

    fn dt(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    #[test]
    fn status_text_and_dot_per_state() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        assert_eq!(choice_status(&ChoiceState::Open), ("等你选", Dot::Waiting));
        assert_eq!(
            choice_status(&ChoiceState::Answered {
                selected: vec![],
                at: None,
                by: None
            }),
            ("已选", Dot::Done)
        );
        assert_eq!(choice_status(&ChoiceState::Expired), ("已失效", Dot::Void));
        assert_eq!(
            approval_status(&ApprovalState::Pending),
            ("等你批准", Dot::Waiting)
        );
        assert_eq!(
            approval_status(&ApprovalState::Approved { by: None, at: 0 }),
            ("已批准", Dot::Done)
        );
        assert_eq!(
            approval_status(&ApprovalState::Denied { by: None, at: 0 }),
            ("已拒绝", Dot::Void)
        );
        assert_eq!(
            approval_status(&ApprovalState::NeedsOwner),
            ("卡住", Dot::Stuck)
        );
        assert_eq!(
            approval_status(&ApprovalState::Expired { at: 0 }),
            ("已过期", Dot::Void)
        );
        assert_eq!(
            draft_status(&DraftState::Pending),
            ("草稿待确认", Dot::Waiting)
        );
        assert_eq!(
            draft_status(&DraftState::Sent {
                by: "P".into(),
                at: 0
            }),
            ("已发送", Dot::Done)
        );
        assert_eq!(
            draft_status(&DraftState::Discarded {
                by: "P".into(),
                at: 0
            }),
            ("已放弃", Dot::Void)
        );
    }

    #[test]
    fn labels() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        assert_eq!(action_kind_label(ActionKind::Delete), "删除");
        assert_eq!(action_kind_label(ActionKind::Send), "发送");
        assert_eq!(action_kind_label(ActionKind::Spend), "付款");
        assert_eq!(action_kind_label(ActionKind::Call), "调用");
        assert_eq!(action_kind_label(ActionKind::Unknown), "操作");
        assert_eq!(channel_label(DraftChannel::Email), "邮件");
        assert_eq!(channel_label(DraftChannel::Message), "消息");
        assert_eq!(confirm_label(2), "确认（2）");
    }

    #[test]
    fn footnotes() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let at = dt(2026, 11, 20, 1, 52).and_utc().timestamp();
        let foot = |text: &str, time: &str| ApprovalFootnote {
            text: text.into(),
            time: Some(time.into()),
        };
        assert_eq!(
            approval_footnote(&ApprovalState::Approved { by: None, at }, &Utc),
            Some(foot("已批准", "01:52"))
        );
        assert_eq!(
            approval_footnote(&ApprovalState::Denied { by: None, at }, &Utc),
            Some(foot("已拒绝", "01:52"))
        );
        let by = |b: &str| Some(b.to_string());
        assert_eq!(
            approval_footnote(&ApprovalState::Approved { by: by("我"), at }, &Utc),
            Some(foot("我已批准", "01:52"))
        );
        // This client's own answer is stored as a mark and drawn in the
        // active language, so a switch changes it without a refresh.
        let mine = ApprovalState::Approved {
            by: by(super::super::ME_MARK),
            at,
        };
        assert_eq!(
            approval_footnote(&mine, &Utc),
            Some(foot("我已批准", "01:52"))
        );
        {
            let _en = crate::i18n::scoped(crate::i18n::Locale::En);
            assert_eq!(
                approval_footnote(&mine, &Utc).map(|f| f.text),
                Some("Approved by You".into())
            );
        }
        assert_eq!(
            approval_footnote(
                &ApprovalState::Denied {
                    by: by("Panda 的手机"),
                    at
                },
                &Utc
            ),
            Some(foot("Panda 的手机 拒绝", "01:52"))
        );
        assert_eq!(approval_footnote(&ApprovalState::NeedsOwner, &Utc), None);
        assert_eq!(
            approval_footnote(&ApprovalState::Expired { at: 1795164480 }, &Utc),
            Some(ApprovalFootnote {
                text: "没人批准，已过期".into(),
                time: Some("08:48".into()),
            })
        );
        assert_eq!(
            draft_footnote(
                &DraftState::Sent {
                    by: "Panda".into(),
                    at
                },
                &Utc
            ),
            Some("Panda 确认发送 · 01:52".into())
        );
        assert_eq!(
            draft_footnote(
                &DraftState::Discarded {
                    by: "Panda".into(),
                    at
                },
                &Utc
            ),
            Some("Panda 放弃 · 01:52".into())
        );
        let tz8 = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
        assert_eq!(clock_in(at, &tz8), "09:52", "local zone, not UTC");
    }

    #[test]
    fn choice_footnotes() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let mut card = ChoiceCard {
            id: "c".into(),
            bot: "文档哥".into(),
            prompt: "?".into(),
            options: vec![
                opt("a", "两种都写", false),
                opt("b", "cargo install", false),
            ],
            multi: false,
            state: ChoiceState::Open,
        };
        assert_eq!(choice_footnote(&card, &Utc), None);
        card.state = ChoiceState::Answered {
            selected: vec!["a".into()],
            at: Some("2026-11-20T01:52:00.000Z".into()),
            by: None,
        };
        assert_eq!(
            choice_footnote(&card, &Utc).unwrap(),
            "已选「两种都写」 · 01:52"
        );
        // Known answerer: that name (or 「我」) before the verb.
        card.state = ChoiceState::Answered {
            selected: vec!["a".into(), "b".into()],
            at: Some("2026-11-20T01:52:00.000Z".into()),
            by: Some("Panda 的手机".into()),
        };
        assert_eq!(
            choice_footnote(&card, &Utc).unwrap(),
            "Panda 的手机 选了「两种都写、cargo install」 · 01:52"
        );
        card.state = ChoiceState::Answered {
            selected: vec!["a".into()],
            at: Some("2026-11-20T01:52:00.000Z".into()),
            by: Some("我".into()),
        };
        assert_eq!(
            choice_footnote(&card, &Utc).unwrap(),
            "我 选了「两种都写」 · 01:52"
        );
        card.state = ChoiceState::Expired;
        assert_eq!(choice_footnote(&card, &Utc).unwrap(), "已过期，没有人作答");
    }

    #[test]
    fn next_run_format() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        // 2026-11-20 is a Friday.
        let now = dt(2026, 11, 20, 1, 0);
        assert_eq!(format_next_run(dt(2026, 11, 20, 23, 0), now), "今天 23:00");
        assert_eq!(format_next_run(dt(2026, 11, 21, 8, 0), now), "明天 08:00");
        assert_eq!(format_next_run(dt(2026, 11, 23, 9, 30), now), "周一 09:30");
        assert_eq!(format_next_run(dt(2026, 11, 26, 9, 30), now), "周四 09:30");
        assert_eq!(
            format_next_run(dt(2026, 11, 30, 8, 0), now),
            "11月30日 08:00"
        );
    }

    #[test]
    fn clamp_preview() {
        assert_eq!(clamp_lines("short  text\nhere", 40, 1), "short text here");
        let long = "大家好，第一期的 7 个节点里已经合并了 5 个：任务队列、Docker 沙箱、三级权限、定时任务和客户端外壳。";
        let one = clamp_lines(long, 40, 1);
        assert!(one.ends_with('…'));
        assert!(one.chars().map(units).sum::<usize>() <= 40);
        let three = clamp_lines(long, 40, 3);
        assert_eq!(three, long, "fits in three lines");
    }
}
