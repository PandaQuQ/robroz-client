//! Jump from the owner's DM reminder row to the approval card in its group:
//! find the card in the laid-out feed and scroll it into view.

use iced::advanced::widget::operation::{Operation, Outcome, Scrollable};
use iced::advanced::widget::{operate, Id};
use iced::widget::operation::AbsoluteOffset;
use iced::{Rectangle, Task, Vector};

/// Room left above the card when it is scrolled into view.
const MARGIN: f32 = 24.0;

/// The `AbsoluteOffset` y for a feed that is anchored to the bottom (it
/// counts from the bottom edge) so that `card_top`, measured from the top of
/// the content, lands `MARGIN` below the top of the viewport.
pub fn offset_from_bottom(card_top: f32, content_h: f32, viewport_h: f32) -> f32 {
    let max = (content_h - viewport_h).max(0.0);
    let top = (card_top - MARGIN).clamp(0.0, max);
    max - top
}

struct Locate {
    card: Id,
    feed: Id,
    card_top: Option<f32>,
    feed_box: Option<(Rectangle, Rectangle)>,
}

impl Operation<Option<AbsoluteOffset>> for Locate {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<AbsoluteOffset>>)) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.card) {
            self.card_top = Some(bounds.y);
        }
    }

    fn scrollable(
        &mut self,
        id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if id == Some(&self.feed) {
            self.feed_box = Some((bounds, content_bounds));
        }
    }

    fn finish(&self) -> Outcome<Option<AbsoluteOffset>> {
        let offset = match (self.card_top, self.feed_box) {
            (Some(top), Some((view, content))) => Some(AbsoluteOffset {
                x: 0.0,
                y: offset_from_bottom(top - content.y, content.height, view.height),
            }),
            _ => None,
        };
        Outcome::Some(offset)
    }
}

/// Scroll the feed `feed` so that the card `card` is in view. Does nothing
/// when the card is not on screen (not loaded, or in another chat).
pub fn to_card<T: Send + 'static>(feed: Id, card: Id) -> Task<T> {
    operate(Locate {
        card,
        feed: feed.clone(),
        card_top: None,
        feed_box: None,
    })
    .then(move |offset| match offset {
        Some(AbsoluteOffset { x, y }) => iced::widget::operation::scroll_to(
            feed.clone(),
            AbsoluteOffset {
                x: Some(x),
                y: Some(y),
            },
        ),
        None => Task::none(),
    })
}

#[cfg(test)]
mod tests {
    use super::offset_from_bottom;

    #[test]
    fn a_card_near_the_top_is_scrolled_to_just_below_the_top_edge() {
        // 2000 tall content, 600 viewport: max scroll 1400. A card 300 from
        // the top wants 24px of room above it: scroll-from-top 276, i.e.
        // 1124 from the bottom.
        assert_eq!(offset_from_bottom(300.0, 2000.0, 600.0), 1124.0);
    }

    #[test]
    fn a_card_in_the_last_screen_stays_at_the_bottom() {
        assert_eq!(offset_from_bottom(1900.0, 2000.0, 600.0), 0.0);
    }

    #[test]
    fn a_card_at_the_very_top_scrolls_all_the_way_up() {
        assert_eq!(offset_from_bottom(0.0, 2000.0, 600.0), 1400.0);
        assert_eq!(offset_from_bottom(10.0, 2000.0, 600.0), 1400.0);
        assert_eq!(offset_from_bottom(24.0, 2000.0, 600.0), 1400.0);
    }

    #[test]
    fn short_content_never_scrolls() {
        assert_eq!(offset_from_bottom(100.0, 400.0, 600.0), 0.0);
    }
}
