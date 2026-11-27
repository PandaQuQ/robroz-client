//! Pure layout of the Settings page (PR5): which rows sit in which group,
//! in order. Widgets live in [`crate::view::settings_page`].
//!
//! Groups: General (Language, Appearance), Bros (the existing bro pages),
//! Connection (Host). Sign out sits in its own red card under them, and
//! the version line is centred under that.

use crate::i18n::Key;

/// Where a settings row leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dest {
    Language,
    Appearance,
    Model,
    Search,
    Mcp,
    Skills,
    Memories,
    Routines,
    ReviewRules,
    Connection,
}

/// One row: a label (filled in by the client), an optional `text2` value
/// before the `›`, and whether it leaves the app (`↗` instead of `›`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub dest: Dest,
    pub value: Option<String>,
    pub external: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub title: Key,
    pub rows: Vec<Row>,
}

fn row(dest: Dest, value: Option<String>) -> Row {
    Row {
        dest,
        value,
        external: false,
    }
}

/// The landing page groups. `language` and `appearance` are the active
/// choices as shown (e.g. 「中文」, 「跟随系统」); `host` is the host
/// address without the scheme, or `None` when there is none yet.
pub fn landing(language: String, appearance: String, host: Option<String>) -> Vec<Group> {
    vec![
        Group {
            title: Key::SettingsGroupGeneral,
            rows: vec![
                row(Dest::Language, Some(language)),
                row(Dest::Appearance, Some(appearance)),
            ],
        },
        Group {
            title: Key::SettingsGroupBros,
            rows: [
                Dest::Model,
                Dest::Search,
                Dest::Mcp,
                Dest::Skills,
                Dest::Memories,
                Dest::Routines,
                Dest::ReviewRules,
            ]
            .into_iter()
            .map(|d| row(d, None))
            .collect(),
        },
        Group {
            title: Key::ConnSectionTitle,
            rows: vec![row(Dest::Connection, host)],
        },
    ]
}

/// The host address as a row value: no scheme, no trailing slash.
pub fn host_value(origin: &str) -> Option<String> {
    let s = origin.trim();
    let s = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(s)
        .trim_end_matches('/');
    (!s.is_empty()).then(|| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landing_groups_are_general_bros_connection_in_order() {
        let g = landing(
            "中文".into(),
            "跟随系统".into(),
            Some("127.0.0.1:8790".into()),
        );
        let titles: Vec<Key> = g.iter().map(|g| g.title).collect();
        assert_eq!(
            titles,
            [
                Key::SettingsGroupGeneral,
                Key::SettingsGroupBros,
                Key::ConnSectionTitle
            ]
        );
        assert_eq!(g[0].rows[0].dest, Dest::Language);
        assert_eq!(g[0].rows[0].value.as_deref(), Some("中文"));
        assert_eq!(g[0].rows[1].value.as_deref(), Some("跟随系统"));
        assert_eq!(g[1].rows.len(), 7);
        assert!(g[1].rows.iter().all(|r| r.value.is_none()));
        assert_eq!(g[2].rows[0].dest, Dest::Connection);
        assert_eq!(g[2].rows[0].value.as_deref(), Some("127.0.0.1:8790"));
        // No row leaves the app yet, so no `↗` is drawn.
        assert!(g.iter().flat_map(|g| &g.rows).all(|r| !r.external));
    }

    #[test]
    fn host_value_drops_scheme_and_slash() {
        assert_eq!(
            host_value("http://127.0.0.1:8790/").as_deref(),
            Some("127.0.0.1:8790")
        );
        assert_eq!(
            host_value("https://robroz.example").as_deref(),
            Some("robroz.example")
        );
        assert_eq!(host_value("  "), None);
    }
}
