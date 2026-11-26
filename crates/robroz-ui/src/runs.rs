//! Consecutive-message runs for the chat feed.
//!
//! A run is one sender's items until the sender changes, the gap to the
//! previous timed item exceeds `window` seconds, or the local calendar day
//! changes. The desktop and mobile clients both use this so grouping stays
//! identical. The default window is five minutes ([`GROUP_SECS`]).

use chrono::{Datelike, NaiveDate, TimeZone};

use crate::i18n::{self, Key};

/// Five minutes: the gap that starts a new run of the same sender.
pub const GROUP_SECS: i64 = robroz_tokens::base::chat::GROUP_SECS as i64;

/// Current Unix time, used to pick "Today" / "Yesterday" labels.
pub fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Two senders belong in the same run when this is true.
pub trait RunSender: Clone {
    fn same(&self, other: &Self) -> bool;
}

/// Client-side facts about a feed item, kept beside the item itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meta<S> {
    pub sender: S,
    pub at: Option<i64>,
}

/// Consecutive items of one sender, drawn as one block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run<T, S> {
    pub sender: S,
    pub items: Vec<T>,
    /// Time of the first item that has one.
    pub first_at: Option<i64>,
    /// Time of the last item that has one.
    pub last_at: Option<i64>,
}

/// A date separator or a run, in feed order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block<T, S> {
    /// Start of a local calendar day, using the first timestamp of that day.
    Date(i64),
    Run(Run<T, S>),
}

/// Split items into runs: a new run starts when the sender changes or the
/// gap to the previous timed item is over `window` seconds. Items without
/// a time never start a run on their own.
pub fn group_items<T, S: RunSender>(
    items: impl IntoIterator<Item = (T, Meta<S>)>,
    window: i64,
) -> Vec<Run<T, S>> {
    group_items_if(items, window, |_, _| true)
}

/// Like [`group_items`], and also starts a new run when `same_day` is false
/// for two timed neighbours.
pub fn group_items_if<T, S: RunSender, F>(
    items: impl IntoIterator<Item = (T, Meta<S>)>,
    window: i64,
    same_day: F,
) -> Vec<Run<T, S>>
where
    F: Fn(i64, i64) -> bool,
{
    let mut out: Vec<Run<T, S>> = Vec::new();
    for (item, meta) in items {
        let joins = out.last().is_some_and(|g| {
            g.sender.same(&meta.sender)
                && match (g.last_at, meta.at) {
                    (Some(prev), Some(at)) => (at - prev).abs() <= window && same_day(prev, at),
                    _ => true,
                }
        });
        if joins {
            let g = out.last_mut().expect("joins implies a run");
            g.items.push(item);
            if meta.at.is_some() {
                if g.first_at.is_none() {
                    g.first_at = meta.at;
                }
                g.last_at = meta.at;
            }
        } else {
            out.push(Run {
                sender: meta.sender,
                items: vec![item],
                first_at: meta.at,
                last_at: meta.at,
            });
        }
    }
    out
}

/// Local calendar day of a Unix timestamp in `tz`.
pub fn day_of<Tz: TimeZone>(at: i64, tz: &Tz) -> Option<NaiveDate> {
    tz.timestamp_opt(at, 0).single().map(|t| t.date_naive())
}

/// Runs that also break at a local day change, with a date separator at
/// the start of each day (including the first).
pub fn blocks<T, S: RunSender, Tz: TimeZone>(
    items: impl IntoIterator<Item = (T, Meta<S>)>,
    window: i64,
    tz: &Tz,
) -> Vec<Block<T, S>> {
    let runs = group_items_if(items, window, |a, b| day_of(a, tz) == day_of(b, tz));
    let mut out = Vec::new();
    let mut last_day = None;
    for run in runs {
        if let Some(at) = run.first_at.or(run.last_at) {
            let day = day_of(at, tz);
            if day != last_day {
                out.push(Block::Date(at));
                last_day = day;
            }
        }
        out.push(Block::Run(run));
    }
    out
}

/// Feed date label: "Today" / "昨天" / "Nov 26" / "Nov 26, 2026".
/// `now` and `at` are Unix seconds, shown in `tz`.
pub fn date_label<Tz: TimeZone>(at: i64, now: i64, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let Some(t) = tz.timestamp_opt(at, 0).single() else {
        return String::new();
    };
    let Some(n) = tz.timestamp_opt(now, 0).single() else {
        return String::new();
    };
    let (day, today) = (t.date_naive(), n.date_naive());
    if day == today {
        Key::ChatDateToday.into()
    } else if today.pred_opt() == Some(day) {
        Key::ChatDateYesterday.into()
    } else if t.format("%Y").to_string() == n.format("%Y").to_string() {
        i18n::month_day(t.month(), t.day())
    } else {
        Key::ChatDateYear.fill(&[
            &i18n::month_day(t.month(), t.day()),
            &t.format("%Y").to_string(),
        ])
    }
}

/// Unix seconds from an RFC 3339 timestamp.
pub fn parse_unix(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .ok()
        .map(|t| t.timestamp())
}

/// Runs that break at the computer's local day change.
pub fn blocks_local<T, S: RunSender>(
    items: impl IntoIterator<Item = (T, Meta<S>)>,
    window: i64,
) -> Vec<Block<T, S>> {
    blocks(items, window, &chrono::Local)
}

/// [`date_label`] in the computer's local zone.
pub fn date_label_local(at: i64, now: i64) -> String {
    date_label(at, now, &chrono::Local)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    const T0: i64 = 1_790_000_000;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Who {
        Own,
        Bro(&'static str),
    }

    impl RunSender for Who {
        fn same(&self, other: &Self) -> bool {
            self == other
        }
    }

    fn own(at: Option<i64>) -> Meta<Who> {
        Meta {
            sender: Who::Own,
            at,
        }
    }

    fn bro(name: &'static str, at: Option<i64>) -> Meta<Who> {
        Meta {
            sender: Who::Bro(name),
            at,
        }
    }

    fn sizes(runs: &[Run<&str, Who>]) -> Vec<usize> {
        runs.iter().map(|g| g.items.len()).collect()
    }

    #[test]
    fn same_sender_within_five_minutes_is_one_run() {
        let g = group_items(
            [
                ("a", own(Some(T0))),
                ("b", own(Some(T0 + 60))),
                ("c", own(Some(T0 + 300))),
            ],
            GROUP_SECS,
        );
        assert_eq!(sizes(&g), vec![3], "exactly five minutes still groups");
        assert_eq!(g[0].first_at, Some(T0));
        assert_eq!(g[0].last_at, Some(T0 + 300));
    }

    #[test]
    fn more_than_five_minutes_starts_a_new_run() {
        let g = group_items(
            [("a", own(Some(T0))), ("b", own(Some(T0 + 301)))],
            GROUP_SECS,
        );
        assert_eq!(sizes(&g), vec![1, 1]);
        let g = group_items(
            [
                ("a", own(Some(T0))),
                ("b", own(Some(T0 + 200))),
                ("c", own(Some(T0 + 400))),
            ],
            GROUP_SECS,
        );
        assert_eq!(sizes(&g), vec![3], "gap is to the previous item");
    }

    #[test]
    fn a_different_sender_starts_a_new_run() {
        let g = group_items(
            [
                ("a", own(Some(T0))),
                ("b", bro("Ada", Some(T0 + 1))),
                ("c", bro("Ada", Some(T0 + 2))),
                ("d", bro("Bea", Some(T0 + 3))),
                ("e", own(Some(T0 + 4))),
            ],
            GROUP_SECS,
        );
        assert_eq!(sizes(&g), vec![1, 2, 1, 1]);
        assert!(matches!(g[1].sender, Who::Bro("Ada")));
        assert_eq!(g[1].items, ["b", "c"]);
    }

    #[test]
    fn items_without_a_time_join_their_sender() {
        let g = group_items(
            [
                ("a", bro("Ada", Some(T0))),
                ("card", bro("Ada", None)),
                ("b", bro("Ada", Some(T0 + 30))),
            ],
            GROUP_SECS,
        );
        assert_eq!(sizes(&g), vec![3]);
        assert_eq!(g[0].last_at, Some(T0 + 30));
    }

    #[test]
    fn a_date_change_starts_a_new_run_even_inside_the_gap() {
        // 23:58 and 00:01 the next day, two minutes apart.
        let start = 1_767_225_480; // 2026-01-01 23:58:00 UTC
        let next = start + 180;
        let tz = FixedOffset::east_opt(0).unwrap();
        let g = group_items_if(
            [
                ("a", bro("Ada", Some(start))),
                ("b", bro("Ada", Some(next))),
            ],
            GROUP_SECS,
            |a, b| day_of(a, &tz) == day_of(b, &tz),
        );
        assert_eq!(sizes(&g), vec![1, 1]);
        let blocks = blocks(
            [
                ("a", bro("Ada", Some(start))),
                ("b", bro("Ada", Some(next))),
            ],
            GROUP_SECS,
            &tz,
        );
        assert!(matches!(
            &blocks[..],
            [Block::Date(_), Block::Run(_), Block::Date(_), Block::Run(_),]
        ));
    }

    #[test]
    fn date_label_today_yesterday_and_year() {
        let tz = FixedOffset::east_opt(0).unwrap();
        let now = 1_767_484_800; // 2026-01-04 00:00:00 UTC
        let today = now + 3600;
        let yesterday = now - 3600;
        let same_year = now - 3 * 86_400;
        let older = 1_704_067_200; // 2024-01-01
        let _en = i18n::scoped(i18n::Locale::En);
        assert_eq!(date_label(today, now, &tz), "Today");
        assert_eq!(date_label(yesterday, now, &tz), "Yesterday");
        assert_eq!(date_label(same_year, now, &tz), "Jan 1");
        assert_eq!(date_label(older, now, &tz), "Jan 1, 2024");
        drop(_en);
        let _zh = i18n::scoped(i18n::Locale::Zh);
        assert_eq!(date_label(today, now, &tz), "今天");
        assert_eq!(date_label(yesterday, now, &tz), "昨天");
        assert_eq!(date_label(same_year, now, &tz), "1月1日");
        assert_eq!(date_label(older, now, &tz), "2024年1月1日");
    }
}
