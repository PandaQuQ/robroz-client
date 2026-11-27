//! Bro avatars on mobile: who may edit, what the title capsule shows, and
//! the optimistic save with its revert. Shapes and colours come from
//! [`robroz_ui::avatar`], the same module the desktop client draws from;
//! [`glyph`] turns one into a Slint image.

use robroz_contract::{Bro, BroAvatar, Me};
use robroz_ui::avatar;
use robroz_ui::theme::Mode;

use crate::chat::{ChatItem, Side};

/// One avatar as a Slint image of `size` logical px, drawn at `scale`
/// (the window's device-pixel ratio) so the bitmap matches the screen
/// pixels 1:1 and is not scaled again by the Image element.
pub fn glyph(a: BroAvatar, mode: Mode, size: f32, scale: f32) -> slint::Image {
    let scale = scale.max(1.0);
    let px = (size * scale).round().max(1.0) as u32;
    let rgba = avatar::rasterize(a, mode == Mode::Dark, None, px);
    let buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&rgba, px, px);
    slint::Image::from_rgba8(buf)
}

/// The title capsule's avatars as one image (first on top, `ring` bands)
/// and its width in logical px; an empty image for no bros.
pub fn stack(list: &[BroAvatar], mode: Mode, ring: [u8; 3], scale: f32) -> (slint::Image, f32) {
    use robroz_tokens::base::avatar as tok;
    stack_at(list, mode, ring, scale, tok::TITLE_SIZE, tok::TITLE_OVERLAP)
}

/// Stacked avatars at a given logical size (16px for the working row).
pub fn stack_at(
    list: &[BroAvatar],
    mode: Mode,
    ring: [u8; 3],
    scale: f32,
    size: f32,
    overlap: f32,
) -> (slint::Image, f32) {
    use robroz_tokens::base::avatar as tok;
    if list.is_empty() {
        return (slint::Image::default(), 0.0);
    }
    let scale = scale.max(1.0);
    let px = (size * scale).round() as u32;
    let step = ((size - overlap) * scale).round() as u32;
    let ring_w = tok::TITLE_RING * scale * (size / tok::TITLE_SIZE);
    let (w, rgba) = avatar::rasterize_stack(list, mode == Mode::Dark, (ring, ring_w), px, step);
    let buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&rgba, w, px);
    (
        slint::Image::from_rgba8(buf),
        avatar::stack_width(list.len(), size, overlap),
    )
}

/// You own this bro: its `ownerUserId` is your `GET /api/me` id. A bro with
/// no owner (null or empty) belongs to `local-admin`, the same as the host's
/// `COALESCE(owner_user_id, 'local-admin')` and the desktop approval rule
/// ([`robroz_ui::cards::is_bro_owner`]). Without `GET /api/me` the answer is
/// no, so the edit entry stays hidden.
pub fn owns(me: Option<&Me>, bro: &Bro) -> bool {
    robroz_ui::cards::is_bro_owner(me.map(|m| m.id.as_str()), bro.owner_user_id.as_deref())
}

/// The avatar to draw for bro `id`: the stored one, else the id default.
pub fn avatar_of(bros: &[Bro], id: &str) -> BroAvatar {
    let stored = bros.iter().find(|b| b.id == id).and_then(|b| b.avatar);
    avatar::resolve(stored, id)
}

/// The bro whose DM is `group`.
pub fn dm_bro<'a>(bros: &'a [Bro], group: &str) -> Option<&'a Bro> {
    bros.iter()
        .find(|b| b.dm_group_id.as_deref() == Some(group))
}

/// Bros for the title capsule: a DM shows its bro; a group shows the bros
/// seen in its feed, newest first (same rule as the desktop client).
pub fn title_ids(dm: Option<&Bro>, items: &[ChatItem]) -> Vec<String> {
    if let Some(b) = dm {
        return vec![b.id.clone()];
    }
    let mut ids: Vec<String> = Vec::new();
    for item in items.iter().rev() {
        let id = match item {
            ChatItem::Text {
                side: Side::Bro { id, .. },
                ..
            } => id,
            ChatItem::Status(c) => &c.bot_id,
            ChatItem::Approval(a) => &a.bot_id,
            _ => continue,
        };
        if !id.is_empty() && !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    ids
}

/// The avatar page's save state for one bro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor {
    pub bro: String,
    /// What the host last confirmed; a failed save puts this back.
    pub confirmed: Option<BroAvatar>,
    /// Latest save sent; only its reply may revert or clear the note.
    pub seq: u64,
    /// The inline line under the preview.
    pub note: Option<String>,
}

impl Editor {
    pub fn open(bro: &Bro) -> Self {
        Self {
            bro: bro.id.clone(),
            confirmed: bro.avatar,
            seq: 0,
            note: None,
        }
    }

    /// Show `next` at once (optimistic). `None` when nothing changes.
    pub fn begin(&mut self, bros: &mut [Bro], next: Option<BroAvatar>) -> Option<u64> {
        let b = bros.iter_mut().find(|b| b.id == self.bro)?;
        if b.avatar == next {
            return None;
        }
        b.avatar = next;
        self.note = None;
        self.seq += 1;
        Some(self.seq)
    }

    /// Apply the host's answer to save `seq`. A failure of the latest save
    /// puts the confirmed avatar back and sets the note.
    pub fn finish(
        &mut self,
        bros: &mut [Bro],
        seq: u64,
        result: Result<Bro, crate::api::AvatarError>,
    ) {
        let latest = seq == self.seq;
        match result {
            Ok(saved) => {
                self.confirmed = saved.avatar;
                if latest {
                    if let Some(b) = bros.iter_mut().find(|b| b.id == saved.id) {
                        *b = saved;
                    }
                    self.note = None;
                }
            }
            Err(_) if !latest => {}
            Err(e) => {
                if let Some(b) = bros.iter_mut().find(|b| b.id == self.bro) {
                    b.avatar = self.confirmed;
                }
                self.note = Some(e.copy());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::AvatarError;
    use robroz_contract::UserRole;
    use robroz_ui::i18n::Key;

    fn bro(id: &str, owner: Option<&str>, avatar: Option<BroAvatar>) -> Bro {
        Bro {
            id: id.into(),
            name: id.into(),
            dm_group_id: Some(format!("dm-{id}")),
            owner_user_id: owner.map(str::to_string),
            avatar,
        }
    }

    fn me(id: &str) -> Me {
        Me {
            id: id.into(),
            name: id.into(),
            role: UserRole::Member,
        }
    }

    const A: BroAvatar = BroAvatar { shape: 1, color: 2 };
    const B: BroAvatar = BroAvatar { shape: 5, color: 6 };

    #[test]
    fn only_the_owner_may_edit() {
        let b = bro("ada", Some("u1"), None);
        assert!(owns(Some(&me("u1")), &b));
        assert!(!owns(Some(&me("u2")), &b));
        assert!(!owns(None, &b));
        assert!(!owns(Some(&me("u1")), &bro("ada", None, None)));
    }

    #[test]
    fn a_bro_with_no_owner_belongs_to_local_admin() {
        for owner in [None, Some(""), Some("  ")] {
            let b = bro("ada", owner, None);
            assert!(owns(Some(&me("local-admin")), &b), "owner {owner:?}");
            assert!(!owns(Some(&me("u2")), &b), "owner {owner:?}");
            assert!(!owns(None, &b), "owner {owner:?}");
        }
        // A real owner is still only that user, local-admin included.
        assert!(!owns(
            Some(&me("local-admin")),
            &bro("ada", Some("u1"), None)
        ));
    }

    #[test]
    fn clearing_shows_the_id_default_at_once() {
        let mut bros = vec![bro("ada", Some("u1"), Some(A))];
        let mut ed = Editor::open(&bros[0]);
        assert_eq!(ed.begin(&mut bros, None), Some(1));
        assert_eq!(avatar_of(&bros, "ada"), avatar::default_for("ada"));
    }

    #[test]
    fn a_failed_save_reverts_and_sets_the_line() {
        let mut bros = vec![bro("ada", Some("u1"), Some(A))];
        let mut ed = Editor::open(&bros[0]);
        let seq = ed.begin(&mut bros, Some(B)).unwrap();
        assert_eq!(bros[0].avatar, Some(B));
        ed.finish(&mut bros, seq, Err(AvatarError::Failed));
        assert_eq!(bros[0].avatar, Some(A));
        assert_eq!(ed.note.as_deref(), Some(Key::AvatarSaveFailed.as_str()));
        // The next pick clears the line; a 403 has its own copy.
        let seq = ed.begin(&mut bros, None).unwrap();
        assert_eq!(ed.note, None);
        ed.finish(&mut bros, seq, Err(AvatarError::NotOwner));
        assert_eq!(bros[0].avatar, Some(A));
        assert_eq!(ed.note.as_deref(), Some(Key::AvatarNotOwner.as_str()));
    }

    #[test]
    fn only_the_latest_reply_decides() {
        let mut bros = vec![bro("ada", Some("u1"), None)];
        let mut ed = Editor::open(&bros[0]);
        let first = ed.begin(&mut bros, Some(A)).unwrap();
        let second = ed.begin(&mut bros, Some(B)).unwrap();
        // The first save lands, then fails late: nothing moves.
        ed.finish(&mut bros, first, Ok(bro("ada", Some("u1"), Some(A))));
        assert_eq!(bros[0].avatar, Some(B));
        ed.finish(&mut bros, first, Err(AvatarError::Failed));
        assert_eq!((bros[0].avatar, ed.note.clone()), (Some(B), None));
        // The latest fails: back to what the host confirmed (A).
        ed.finish(&mut bros, second, Err(AvatarError::Failed));
        assert_eq!(bros[0].avatar, Some(A));
    }

    #[test]
    fn title_shows_the_dm_bro_or_feed_bros_newest_first() {
        let ada = bro("ada", None, None);
        assert_eq!(title_ids(Some(&ada), &[]), vec!["ada".to_string()]);
        let text = |id: &str| ChatItem::Text {
            body: "x".into(),
            side: Side::Bro {
                id: id.into(),
                name: id.into(),
            },
            at: None,
        };
        let own = ChatItem::Text {
            body: "x".into(),
            side: Side::Own,
            at: None,
        };
        let items = vec![text("ada"), own, text("bea"), text("ada"), text("cy")];
        assert_eq!(title_ids(None, &items), vec!["cy", "ada", "bea"]);
    }
}

/// Up to three group avatars for a home-list row, stacked and overlapped
/// like the chat title capsule (24px, 8px overlap).
pub fn group_stack(list: &[BroAvatar], mode: Mode, ring: [u8; 3], scale: f32) -> slint::Image {
    use robroz_tokens::home as h;
    let n = list.len().min(3);
    stack_at(
        &list[..n],
        mode,
        ring,
        scale,
        h::GROUP_STACK,
        h::GROUP_STACK_OVERLAP,
    )
    .0
}
