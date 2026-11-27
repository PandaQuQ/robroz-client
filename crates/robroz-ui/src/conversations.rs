//! The IM conversation list: groups and bro DMs in one list, most recently
//! active first, with a preview, a time and an unread count.
//!
//! Pure data and rules; the widgets are in [`crate::view::conversation_list`].
//! The host sends unread counts and activity times (`GET /api/unread`); the
//! client fills names, DM flags and previews from what it has seen.

use crate::i18n::Key;
use chrono::{DateTime, Datelike, TimeZone};

/// Empty-state line when there is nothing to talk to yet.
pub const EMPTY_HINT: Key = Key::ConversationsEmptyHint;
/// Button that creates a bro and opens its DM (PM wording).
pub const NEW_BRO: Key = Key::ConversationsNewBro;
/// Tooltip-ish hint after a bro DM's name: click the name to rename.
pub const RENAME_HINT: Key = Key::ConversationsRenameHint;
/// Rename input placeholder.
pub const RENAME_PLACEHOLDER: Key = Key::ConversationsRenamePlaceholder;

/// Composer placeholder in a group (a DM says 「发给 {name}…」).
pub const GROUP_HINT: Key = Key::ConversationsGroupHint;

/// Composer placeholder for this chat.
pub fn composer_hint(name: &str, dm: bool) -> String {
    if dm {
        Key::ConversationsDmHint.fill(&[&name])
    } else {
        GROUP_HINT.to_string()
    }
}
/// Composer button.
pub const SEND: Key = Key::ConversationsSend;
/// List heading.
pub const TITLE: Key = Key::ConversationsTitle;
/// Stored preview cut, in characters; the view fits it to the row width.
pub const PREVIEW_CHARS: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Conversation {
    pub group_id: String,
    /// Group name, or the bro's name for a DM.
    pub name: String,
    /// A DM: a group whose only member is one bro.
    pub dm: bool,
    pub preview: Option<String>,
    /// Newest activity, RFC 3339. `None` for a chat with nothing in it.
    pub last_at: Option<String>,
    pub unread: u64,
    /// A DM's bro avatar (stored or id default); `None` for groups and for
    /// a DM whose bro is not known yet.
    pub bro_avatar: Option<crate::avatar::BroAvatar>,
    /// Bros in a group, in add order, for the 2×2 tile (at most four).
    pub group_avatars: Vec<crate::avatar::BroAvatar>,
    /// Extra bros past the four shown in the tile.
    pub group_avatar_more: usize,
    /// The user who created this group; `None` for a DM or an older group.
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Conversations {
    rows: Vec<Conversation>,
}

impl Conversations {
    /// Rows in display order.
    pub fn rows(&self) -> &[Conversation] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn get(&self, group_id: &str) -> Option<&Conversation> {
        self.rows.iter().find(|c| c.group_id == group_id)
    }

    fn slot(&mut self, group_id: &str) -> &mut Conversation {
        if let Some(i) = self.rows.iter().position(|c| c.group_id == group_id) {
            return &mut self.rows[i];
        }
        self.rows.push(Conversation {
            group_id: group_id.to_string(),
            name: group_id.to_string(),
            ..Conversation::default()
        });
        self.rows.last_mut().expect("just pushed")
    }

    /// A group or DM the client knows about. Keeps counts and previews.
    pub fn upsert(&mut self, group_id: &str, name: &str, dm: bool) {
        let c = self.slot(group_id);
        c.name = name.to_string();
        c.dm = c.dm || dm;
        self.sort();
    }

    /// Host unread state: `(group, unread, lastActivityAt)`. Activity only
    /// moves forward, so a live event seen before this poll is kept.
    pub fn apply_unread(&mut self, group_id: &str, unread: u64, last_at: Option<&str>) {
        let c = self.slot(group_id);
        c.unread = unread;
        if let Some(at) = last_at {
            bump(&mut c.last_at, at);
        }
        self.sort();
    }

    /// Something new arrived in this chat (stream or poll).
    pub fn touch(&mut self, group_id: &str, preview: Option<String>, at: Option<&str>) {
        let c = self.slot(group_id);
        if let Some(p) = preview {
            c.preview = Some(clip(&p, PREVIEW_CHARS));
        }
        if let Some(at) = at {
            bump(&mut c.last_at, at);
        }
        self.sort();
    }

    /// Replace a chat's preview without moving it (language switch).
    pub fn set_preview(&mut self, group_id: &str, preview: String) {
        if let Some(c) = self.rows.iter_mut().find(|c| c.group_id == group_id) {
            c.preview = Some(clip(&preview, PREVIEW_CHARS));
        }
    }

    /// The avatar to draw for a DM row.
    pub fn set_avatar(&mut self, group_id: &str, avatar: crate::avatar::BroAvatar) {
        if let Some(c) = self.rows.iter_mut().find(|c| c.group_id == group_id) {
            c.bro_avatar = Some(avatar);
        }
    }

    /// Opened, or the host confirmed a read: the badge goes away.
    pub fn mark_read(&mut self, group_id: &str) {
        if let Some(c) = self.rows.iter_mut().find(|c| c.group_id == group_id) {
            c.unread = 0;
        }
    }

    /// Creator and DM flag from `GET /api/groups`.
    pub fn set_flags(&mut self, group_id: &str, dm: bool, created_by: Option<String>) {
        let c = self.slot(group_id);
        c.dm = c.dm || dm;
        if created_by.is_some() {
            c.created_by = created_by;
        }
    }

    /// Bros drawn in a group's 2×2 tile.
    pub fn set_group_avatars(
        &mut self,
        group_id: &str,
        avatars: Vec<crate::avatar::BroAvatar>,
        more: usize,
    ) {
        if let Some(c) = self.rows.iter_mut().find(|c| c.group_id == group_id) {
            c.group_avatars = avatars;
            c.group_avatar_more = more;
        }
    }

    /// Drop a chat after the host deleted it.
    pub fn remove(&mut self, group_id: &str) {
        self.rows.retain(|c| c.group_id != group_id);
    }

    /// Unread across every chat.
    pub fn total_unread(&self) -> u64 {
        self.rows.iter().map(|c| c.unread).sum()
    }

    /// Unread across every chat except `group_id` (the open one).
    pub fn unread_except(&self, group_id: Option<&str>) -> u64 {
        self.rows
            .iter()
            .filter(|c| Some(c.group_id.as_str()) != group_id)
            .map(|c| c.unread)
            .sum()
    }

    /// Rows whose name or preview matches the home search capsule.
    pub fn matching(&self, query: &str) -> Vec<&Conversation> {
        self.rows
            .iter()
            .filter(|c| crate::home::row_matches(&c.name, c.preview.as_deref(), query))
            .collect()
    }

    /// Newest first; chats with no activity last, in the order they came.
    fn sort(&mut self) {
        self.rows.sort_by(|a, b| match (&a.last_at, &b.last_at) {
            (Some(x), Some(y)) => instant(y).cmp(&instant(x)),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }
}

fn instant(iso: &str) -> i64 {
    DateTime::parse_from_rfc3339(iso).map_or(0, |t| t.timestamp_millis())
}

fn bump(slot: &mut Option<String>, at: &str) {
    if slot.as_deref().is_none_or(|old| instant(at) > instant(old)) {
        *slot = Some(at.to_string());
    }
}

/// Single-line preview: newlines become spaces, long text gets `…`.
pub fn clip(s: &str, max: usize) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        return flat;
    }
    let mut out: String = flat.chars().take(max).collect();
    out.push('…');
    out
}

fn units(c: char) -> usize {
    if c.is_ascii() {
        1
    } else {
        2
    }
}

/// Fit one line into `max` width units (ASCII 1, CJK 2), ending in `…`
/// when cut. iced has no text ellipsis, so the view estimates `max` from
/// the space left beside the time or the unread badge.
pub fn fit(s: &str, max: usize) -> String {
    let total: usize = s.chars().map(units).sum();
    if total <= max {
        return s.to_string();
    }
    let budget = max.saturating_sub(units('…'));
    let mut used = 0;
    let mut out = String::new();
    for c in s.chars() {
        if used + units(c) > budget {
            break;
        }
        used += units(c);
        out.push(c);
    }
    out.push('…');
    out
}

/// Badge text: `1`…`99`, then `99+`; `None` for zero.
pub fn badge(unread: u64) -> Option<String> {
    match unread {
        0 => None,
        1..=99 => Some(unread.to_string()),
        _ => Some("99+".into()),
    }
}

/// Avatar letter: the first character of the name, upper-cased.
pub fn initial(name: &str) -> String {
    name.trim()
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_else(|| "?".into())
}

/// List time like WeChat: `HH:MM` today, `昨天` yesterday, else `M月D日`
/// (with the year when it is not this year). `now` and `iso` are shown in
/// `tz`, the computer's zone.
pub fn list_time<Tz: TimeZone>(iso: &str, now: i64, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let Ok(t) = DateTime::parse_from_rfc3339(iso) else {
        return String::new();
    };
    let t = t.with_timezone(tz);
    let Some(n) = tz.timestamp_opt(now, 0).single() else {
        return String::new();
    };
    let (day, today) = (t.date_naive(), n.date_naive());
    if day == today {
        t.format("%H:%M").to_string()
    } else if today.pred_opt() == Some(day) {
        Key::ConversationsYesterday.into()
    } else if t.format("%Y").to_string() == n.format("%Y").to_string() {
        crate::i18n::month_day(t.month(), t.day())
    } else {
        t.format("%Y/%-m/%-d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{FixedOffset, Utc};

    #[test]
    fn sorted_by_activity_and_badges_follow_host() {
        let mut c = Conversations::default();
        c.upsert("g1", "官网", false);
        c.upsert("dm1", "bro", true);
        c.upsert("g2", "空群", false);
        c.apply_unread("g1", 2, Some("2026-11-20T01:00:00.000Z"));
        c.apply_unread("dm1", 0, Some("2026-11-20T02:00:00.000Z"));
        c.apply_unread("g2", 0, None);
        let order: Vec<_> = c.rows().iter().map(|r| r.group_id.as_str()).collect();
        assert_eq!(order, ["dm1", "g1", "g2"], "newest first, empty last");
        assert!(c.get("dm1").unwrap().dm);

        // A live event in g1 moves it to the top with its preview.
        c.touch(
            "g1",
            Some("首页\n改好了".into()),
            Some("2026-11-20T03:00:00+00:00"),
        );
        assert_eq!(c.rows()[0].group_id, "g1");
        assert_eq!(c.rows()[0].preview.as_deref(), Some("首页 改好了"));
        // An older poll result does not move activity back.
        c.apply_unread("g1", 3, Some("2026-11-20T01:00:00.000Z"));
        assert_eq!(c.rows()[0].group_id, "g1");
        assert_eq!(c.rows()[0].unread, 3);

        c.mark_read("g1");
        assert_eq!(c.get("g1").unwrap().unread, 0);
        // Renaming keeps the DM flag and the counts.
        c.upsert("dm1", "小明", false);
        assert!(c.get("dm1").unwrap().dm);
        assert_eq!(c.get("dm1").unwrap().name, "小明");
    }

    #[test]
    fn unknown_group_from_host_gets_a_row() {
        let mut c = Conversations::default();
        c.apply_unread("new", 1, Some("2026-11-20T01:00:00Z"));
        assert_eq!(c.get("new").unwrap().name, "new");
    }

    #[test]
    fn badge_preview_initial() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        assert_eq!(badge(0), None);
        assert_eq!(badge(7).as_deref(), Some("7"));
        assert_eq!(badge(100).as_deref(), Some("99+"));
        assert_eq!(clip("abc", 5), "abc");
        assert_eq!(clip("一二三四五六", 4), "一二三四…");
        assert_eq!(initial("bro 2"), "B");
        assert_eq!(initial("小明"), "小");
        assert_eq!(initial("  "), "?");
        assert_eq!(composer_hint("Ada", true), "发给 Ada…");
        assert_eq!(composer_hint("官网", false), GROUP_HINT);
        assert_eq!(fit("abc", 3), "abc");
        assert_eq!(fit("abcdef", 5), "abc…");
        assert_eq!(
            fit("你好，我是 Ada。有事直接在这里找我。", 16),
            "你好，我是 Ada…"
        );
        let cut = fit("一二三四五六七八", 9);
        assert_eq!(cut, "一二三…");
        assert!(cut.chars().map(units).sum::<usize>() <= 9);
    }

    #[test]
    fn list_times() {
        let _zh = crate::i18n::scoped(crate::i18n::Locale::Zh);
        let now = DateTime::parse_from_rfc3339("2026-11-20T10:00:00Z")
            .unwrap()
            .timestamp();
        assert_eq!(list_time("2026-11-20T01:52:00Z", now, &Utc), "01:52");
        assert_eq!(list_time("2026-11-19T23:00:00Z", now, &Utc), "昨天");
        assert_eq!(list_time("2026-10-03T08:00:00Z", now, &Utc), "10月3日");
        assert_eq!(list_time("2025-10-03T08:00:00Z", now, &Utc), "2025/10/3");
        assert_eq!(list_time("bad", now, &Utc), "");
        // Local zone decides the day: 23:00 UTC is tomorrow 07:00 at +8.
        let tz8 = FixedOffset::east_opt(8 * 3600).unwrap();
        assert_eq!(list_time("2026-11-19T23:00:00Z", now, &tz8), "07:00");
    }

    #[test]
    fn unread_totals_and_search_and_remove() {
        let mut c = Conversations::default();
        c.upsert("g1", "官网", false);
        c.upsert("dm1", "Ada", true);
        c.apply_unread("g1", 2, None);
        c.apply_unread("dm1", 3, None);
        assert_eq!(c.total_unread(), 5);
        assert_eq!(c.unread_except(Some("g1")), 3);
        assert_eq!(c.matching("ada").len(), 1);
        assert_eq!(c.matching("网").len(), 1);
        c.remove("g1");
        assert_eq!(c.total_unread(), 3);
        assert!(c.get("g1").is_none());
    }
}
