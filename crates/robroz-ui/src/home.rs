//! Pure rules for the v0.2 home list, group menus and profile pages.
//!
//! Widgets live in [`crate::view`]. The rename, delete and members entries
//! show only when [`can_manage`]: not a DM, and the caller is an admin or
//! the group's creator.

use robroz_contract::UserRole;

/// Whether this user may rename or delete the group, and see Members.
///
/// `!group.dm && (admin || createdBy == me)`, matching the contract
/// comment on [`robroz_contract::Group`]. `created_by` is missing on
/// groups created before that field existed: then only an admin may.
pub fn can_manage(
    dm: bool,
    created_by: Option<&str>,
    me_id: Option<&str>,
    me_role: Option<UserRole>,
) -> bool {
    if dm {
        return false;
    }
    let Some(me_id) = me_id else {
        return false;
    };
    me_role == Some(UserRole::Admin) || created_by == Some(me_id)
}

/// Unread count shown on the chat back button: every chat except the
/// open one.
pub fn unread_except(total: u64, open: u64) -> u64 {
    total.saturating_sub(open)
}

/// Case-insensitive name or preview match for the home search capsule.
pub fn row_matches(name: &str, preview: Option<&str>, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    let q = q.to_lowercase();
    name.to_lowercase().contains(&q) || preview.is_some_and(|p| p.to_lowercase().contains(&q))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manage_is_hidden_for_a_dm_even_when_the_caller_is_admin() {
        assert!(!can_manage(
            true,
            Some("u-me"),
            Some("u-me"),
            Some(UserRole::Admin)
        ));
    }

    #[test]
    fn manage_is_shown_for_the_creator_and_for_an_admin() {
        assert!(can_manage(
            false,
            Some("u-me"),
            Some("u-me"),
            Some(UserRole::Member)
        ));
        assert!(can_manage(
            false,
            Some("someone-else"),
            Some("u-me"),
            Some(UserRole::Admin)
        ));
        assert!(!can_manage(
            false,
            Some("someone-else"),
            Some("u-me"),
            Some(UserRole::Member)
        ));
    }

    #[test]
    fn an_old_group_without_created_by_is_admin_only() {
        assert!(!can_manage(
            false,
            None,
            Some("u-me"),
            Some(UserRole::Member)
        ));
        assert!(can_manage(false, None, Some("u-me"), Some(UserRole::Admin)));
        assert!(!can_manage(false, None, None, Some(UserRole::Admin)));
    }

    #[test]
    fn unread_except_drops_the_open_chat() {
        assert_eq!(unread_except(5, 2), 3);
        assert_eq!(unread_except(2, 5), 0);
        assert_eq!(unread_except(0, 0), 0);
    }

    #[test]
    fn search_matches_name_or_preview() {
        assert!(row_matches("官网", Some("hello"), ""));
        assert!(row_matches("官网", Some("hello"), " 网 "));
        assert!(row_matches("Ada", Some("Hello there"), "hello"));
        assert!(!row_matches("Ada", Some("Hello"), "bea"));
    }
}
