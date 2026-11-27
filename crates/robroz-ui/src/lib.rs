//! robroz shared UI layer.
//!
//! - [`avatar`]: bro avatar shapes, palette and the id-based default,
//!   shared by the desktop and mobile clients.
//! - [`cards`]: view models and pure rules for the phase-1 cards (`choice`,
//!   `approval`, `draft`) and the routines list (UI SPEC §4–§8).
//! - [`conversations`]: the IM conversation list (groups and bro DMs,
//!   unread counts, previews), as pure data.
//! - [`theme`]: the skin struct every component reads; the default solid
//!   skin is filled from `robroz-tokens`.
//! - [`view`]: iced widgets for the above. They emit [`view::Action`] and
//!   never do I/O, so the same layer can later run in a web build.

pub mod avatar;
#[cfg(feature = "gui")]
pub mod avatar_view;
pub mod cards;
pub mod conversations;
pub mod home;
pub mod i18n;
#[cfg(feature = "gui")]
pub mod icons;
pub mod process;
pub mod runs;
pub mod theme;
#[cfg(feature = "gui")]
pub mod view;
#[cfg(feature = "gui")]
mod widgets;

pub use theme::{Mode, Skin, CHAT_COMPACT_WIDTH};
#[cfg(feature = "gui")]
pub use view::Action;
