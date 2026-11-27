//! MCP page (screen 9): the same `robroz_ui::cards::mcp::Book` rules as the
//! desktop page, flattened into Slint rows. Adding or editing a server stays
//! on the desktop client; the phone lists servers, filters them and runs
//! Connect (switch on, test, approve a stdio command when the host asks).

use robroz_ui::cards::mcp::{self, Book, Filter, Transport};

/// Slint's `mcp-filter`: 0 all, 1 stdio, 2 HTTP. The connected count is
/// not a filter.
pub fn filter_index(f: Filter) -> i32 {
    match f {
        Filter::All => 0,
        Filter::Transport(Transport::Stdio) => 1,
        Filter::Transport(Transport::Http) => 2,
    }
}

pub fn filter_from_index(i: i32) -> Filter {
    match i {
        1 => Filter::Transport(Transport::Stdio),
        2 => Filter::Transport(Transport::Http),
        _ => Filter::All,
    }
}

/// One row as plain data (id, name, tile letter, second line, connected).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub id: String,
    pub name: String,
    pub letter: String,
    pub sub: String,
    pub connected: bool,
}

pub fn lines(book: &Book) -> Vec<Line> {
    book.visible()
        .map(|s| Line {
            id: s.id.clone(),
            name: s.name.clone(),
            letter: mcp::icon_letter(&s.name),
            sub: book.row_line(s),
            connected: mcp::is_connected(s),
        })
        .collect()
}

/// The "N connected" count text, empty (hidden) when nothing is connected.
pub fn connected_text(book: &Book) -> String {
    match book.connected_count() {
        0 => String::new(),
        n => mcp::connected_count_text(n),
    }
}

/// Category capsules only show when both transports are in use, so the
/// fixed stdio / HTTP capsules always match at least one server.
pub fn show_categories(book: &Book) -> bool {
    book.show_categories()
}

#[cfg(test)]
mod tests {
    use super::*;
    use robroz_contract::McpServer;

    fn server(id: &str, transport: &str, enabled: bool, approved: Option<bool>) -> McpServer {
        McpServer {
            id: id.into(),
            name: format!("srv {id}"),
            transport: transport.into(),
            command: (transport == "stdio").then(|| "npx".into()),
            args: (transport == "stdio").then(Vec::new),
            url: (transport == "http").then(|| "https://x.invalid/".into()),
            secrets: vec![],
            enabled,
            tool_count: Some(2),
            created_at: "2026-11-27T00:00:00Z".into(),
            updated_at: format!("2026-11-27T00:00:0{}Z", id.len()),
            command_approved: approved,
            command_approved_at: None,
        }
    }

    #[test]
    fn filter_index_round_trips() {
        for i in 0..3 {
            assert_eq!(filter_index(filter_from_index(i)), i);
        }
        assert_eq!(filter_from_index(3), Filter::All);
        assert_eq!(filter_from_index(9), Filter::All);
    }

    #[test]
    fn lines_follow_the_book() {
        let mut b = Book::default();
        b.load(vec![
            server("a", "stdio", true, Some(true)),
            server("bb", "http", false, None),
        ]);
        assert!(show_categories(&b));
        let l = lines(&b);
        assert_eq!(l.len(), 2);
        assert!(l
            .iter()
            .any(|x| x.id == "a" && x.connected && x.letter == "S"));
        assert!(l.iter().any(|x| x.id == "bb" && !x.connected));
        assert!(!connected_text(&b).is_empty());
        b.set_filter(filter_from_index(2));
        assert_eq!(lines(&b).len(), 1);
        assert_eq!(lines(&b)[0].id, "bb");
        b.set_filter(filter_from_index(0));
        b.set_enabled("a", false);
        assert_eq!(connected_text(&b), "");
        assert_eq!(filter_index(b.active_filter()), 0);
    }
}
