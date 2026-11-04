//! Labels of the tray menu. Each entry names the action it performs, so it
//! always reflects the current state of the overlay.

use crate::OverlayMode;

/// Label of the entry that quits the app.
pub const QUIT_LABEL: &str = "Quitter";

/// Label of the entry that switches to the other mode.
#[must_use]
pub const fn mode_item_label(current: OverlayMode) -> &'static str {
    match current {
        OverlayMode::ClickThrough => "Mode interactif",
        OverlayMode::Interactive => "Mode click-through",
    }
}

/// Label of the entry that shows or hides the overlay.
#[must_use]
pub const fn visibility_item_label(visible: bool) -> &'static str {
    if visible {
        "Masquer"
    } else {
        "Afficher"
    }
}

#[cfg(test)]
mod tests {
    use super::{mode_item_label, visibility_item_label};
    use crate::OverlayMode;

    #[test]
    fn offers_interactive_mode_while_click_through() {
        assert_eq!(
            mode_item_label(OverlayMode::ClickThrough),
            "Mode interactif"
        );
    }

    #[test]
    fn offers_click_through_mode_while_interactive() {
        assert_eq!(
            mode_item_label(OverlayMode::Interactive),
            "Mode click-through"
        );
    }

    #[test]
    fn offers_to_hide_a_visible_overlay() {
        assert_eq!(visibility_item_label(true), "Masquer");
    }

    #[test]
    fn offers_to_show_a_hidden_overlay() {
        assert_eq!(visibility_item_label(false), "Afficher");
    }
}
