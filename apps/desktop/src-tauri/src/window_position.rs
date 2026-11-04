//! Remembers where the user left the overlay and puts it back there on the
//! next launch. The placement rule lives in the `overlay` crate; this module
//! is the Tauri glue.

use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use overlay::{Point, Rect, Size};
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, Runtime, WebviewWindow, WindowEvent};

/// File holding the last position, in the app config directory.
const POSITION_FILE: &str = "window.json";

/// Quiet time after the last move before the position is written, so a drag
/// writes once instead of once per pixel.
const SAVE_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error)]
enum PositionError {
    #[error("no app config directory: {0}")]
    ConfigDir(tauri::Error),
    #[error("cannot access the position file: {0}")]
    Io(#[from] std::io::Error),
    #[error("unreadable position file: {0}")]
    Format(#[from] serde_json::Error),
    #[error("cannot query the window or its screens: {0}")]
    Window(#[from] tauri::Error),
}

/// Moves `window` back to its saved position, kept on a screen that still
/// exists. Any failure is logged and leaves the default position.
pub fn restore<R: Runtime>(window: &WebviewWindow<R>) {
    match saved_placement(window) {
        Ok(Some(position)) => {
            if let Err(error) = window.set_position(PhysicalPosition::new(position.x, position.y)) {
                log::error!("cannot restore the overlay position: {error}");
            }
        }
        Ok(None) => {}
        Err(error) => log::error!("keeping the default overlay position: {error}"),
    }
}

/// Saves the position of `window` each time the user stops moving it.
pub fn remember<R: Runtime>(window: &WebviewWindow<R>) {
    let last_move = Arc::new(AtomicU64::new(0));
    let watched = window.clone();
    window.on_window_event(move |event| {
        let WindowEvent::Moved(position) = event else {
            return;
        };
        let this_move = last_move.fetch_add(1, Ordering::SeqCst) + 1;
        let last_move = Arc::clone(&last_move);
        let window = watched.clone();
        let position = Point {
            x: position.x,
            y: position.y,
        };
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(SAVE_DELAY).await;
            // A newer move restarted the delay, or Windows parked the
            // minimized window off-screen: nothing worth saving.
            if last_move.load(Ordering::SeqCst) != this_move
                || window.is_minimized().unwrap_or(false)
            {
                return;
            }
            if let Err(error) = save(window.app_handle(), position) {
                log::error!("cannot save the overlay position: {error}");
            }
        });
    });
}

fn saved_placement<R: Runtime>(window: &WebviewWindow<R>) -> Result<Option<Point>, PositionError> {
    let json = match std::fs::read_to_string(position_file(window.app_handle())?) {
        Ok(json) => json,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            log::info!("no saved overlay position yet");
            return Ok(None);
        }
        Err(error) => return Err(error.into()),
    };
    let saved = Point::from_json(&json)?;
    let size = window.outer_size()?;
    let screens: Vec<Rect> = window
        .available_monitors()?
        .iter()
        .map(screen_area)
        .collect();
    let primary = window.primary_monitor()?.as_ref().map(screen_area);
    Ok(overlay::fit_on_screens(
        saved,
        Size {
            width: size.width,
            height: size.height,
        },
        &screens,
        primary,
    ))
}

fn save<R: Runtime>(app: &AppHandle<R>, position: Point) -> Result<(), PositionError> {
    let file = position_file(app)?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(file, position.to_json()?)?;
    Ok(())
}

fn position_file<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, PositionError> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(PositionError::ConfigDir)?;
    Ok(dir.join(POSITION_FILE))
}

fn screen_area(monitor: &Monitor) -> Rect {
    let origin = monitor.position();
    let size = monitor.size();
    Rect {
        origin: Point {
            x: origin.x,
            y: origin.y,
        },
        size: Size {
            width: size.width,
            height: size.height,
        },
    }
}
