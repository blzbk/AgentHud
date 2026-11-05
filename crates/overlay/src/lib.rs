//! Overlay mode of the main window, kept free of Tauri so it builds and is
//! tested on any host. `src-tauri` applies it to the real window.

mod options;
mod placement;
pub mod tray_menu;

pub use options::Options;
pub use placement::{fit_on_screens, Point, Rect, Size};

/// Whether mouse events go through the overlay or are handled by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayMode {
    #[default]
    ClickThrough,
    Interactive,
}

impl OverlayMode {
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::ClickThrough => Self::Interactive,
            Self::Interactive => Self::ClickThrough,
        }
    }

    /// Wire name shared with the UI.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClickThrough => "click-through",
            Self::Interactive => "interactive",
        }
    }

    /// Whether the window must let the cursor through in this mode.
    #[must_use]
    pub const fn ignores_cursor(self) -> bool {
        matches!(self, Self::ClickThrough)
    }
}

#[cfg(test)]
mod tests {
    use super::OverlayMode;

    #[test]
    fn starts_click_through() {
        assert_eq!(OverlayMode::default(), OverlayMode::ClickThrough);
    }

    #[test]
    fn toggle_alternates_between_the_two_modes() {
        let interactive = OverlayMode::ClickThrough.toggle();

        assert_eq!(interactive, OverlayMode::Interactive);
        assert_eq!(interactive.toggle(), OverlayMode::ClickThrough);
    }

    #[test]
    fn only_click_through_ignores_the_cursor() {
        assert!(OverlayMode::ClickThrough.ignores_cursor());
        assert!(!OverlayMode::Interactive.ignores_cursor());
    }

    #[test]
    fn wire_names_match_the_ui() {
        assert_eq!(OverlayMode::ClickThrough.as_str(), "click-through");
        assert_eq!(OverlayMode::Interactive.as_str(), "interactive");
    }
}
