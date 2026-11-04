//! Applies the overlay mode to the main window and notifies the UI. The mode
//! itself lives in the `overlay` crate; this module is the Tauri glue.

use std::sync::{Mutex, PoisonError};

use overlay::OverlayMode;
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewWindow};

/// Label of the overlay window, as declared in `tauri.conf.json`.
const WINDOW_LABEL: &str = "main";

/// Event emitted to the UI with the new mode as payload.
const MODE_EVENT: &str = "overlay-mode";

/// Current overlay mode, managed as Tauri state.
#[derive(Default)]
pub struct OverlayState(Mutex<OverlayMode>);

impl OverlayState {
    fn current(&self) -> OverlayMode {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Applies the initial mode to the overlay window.
///
/// # Errors
///
/// Fails if the window is missing or rejects the cursor setting.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let mode = app.state::<OverlayState>().current();
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

/// Current mode of the overlay.
pub fn current_mode<R: Runtime>(app: &AppHandle<R>) -> OverlayMode {
    app.state::<OverlayState>().current()
}

/// Shows or hides the overlay window.
///
/// # Errors
///
/// Fails if the window is missing or rejects the change.
pub fn set_visible<R: Runtime>(app: &AppHandle<R>, visible: bool) -> tauri::Result<()> {
    let window = window(app)?;
    if visible {
        window.show()
    } else {
        window.hide()
    }
}

/// Current mode, read by the UI on load so a reloaded page shows the truth.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri injects `State` by value.
pub fn overlay_mode(state: State<'_, OverlayState>) -> &'static str {
    state.current().as_str()
}

/// The overlay window.
///
/// # Errors
///
/// Fails if the window is missing.
pub fn window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    app.get_webview_window(WINDOW_LABEL)
        .ok_or(tauri::Error::WebviewNotFound)
}
