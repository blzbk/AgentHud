//! Overlay mode of the main window: click-through by default, interactive on
//! demand. The native side owns the mode and notifies the UI of each change.

use std::sync::{Mutex, PoisonError};

use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};

/// Label of the overlay window, as declared in `tauri.conf.json`.
const WINDOW_LABEL: &str = "main";

/// Event emitted to the UI with the new mode as payload.
const MODE_EVENT: &str = "overlay-mode";

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

    /// Payload of [`MODE_EVENT`], matched by the UI.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClickThrough => "click-through",
            Self::Interactive => "interactive",
        }
    }

    const fn ignores_cursor(self) -> bool {
        matches!(self, Self::ClickThrough)
    }
}

/// Current overlay mode, managed as Tauri state.
#[derive(Default)]
pub struct OverlayState(Mutex<OverlayMode>);

/// Applies the initial mode to the overlay window.
///
/// # Errors
///
/// Fails if the window is missing or rejects the cursor setting.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let mode = *app
        .state::<OverlayState>()
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    window(app)?.set_ignore_cursor_events(mode.ignores_cursor())
}

/// Switches to the other mode and notifies the UI.
///
/// # Errors
///
/// Fails if the window is missing, rejects the cursor setting, or the event
/// cannot be emitted. The stored mode only changes once the window accepts it.
pub fn toggle<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let state = app.state::<OverlayState>();
    let mut mode = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    let next = mode.toggle();
    window(app)?.set_ignore_cursor_events(next.ignores_cursor())?;
    *mode = next;
    app.emit(MODE_EVENT, next.as_str())
}

fn window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    app.get_webview_window(WINDOW_LABEL)
        .ok_or(tauri::Error::WebviewNotFound)
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
}
