//! Tray icon of the app: switches the overlay mode, shows or hides the overlay
//! and quits. The labels live in the `overlay` crate; this module is the
//! Tauri glue.

use overlay::tray_menu::{self, QUIT_LABEL};
use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

use crate::overlay_window;

const MODE_ID: &str = "mode";
const VISIBILITY_ID: &str = "visibility";
const QUIT_ID: &str = "quit";

/// Tooltip shown when hovering the tray icon.
const TOOLTIP: &str = "AgentHud";

/// Menu entries whose label follows the overlay state, managed as Tauri state.
struct TrayItems<R: Runtime> {
    mode: MenuItem<R>,
    visibility: MenuItem<R>,
}

/// Adds the tray icon and its menu.
///
/// # Errors
///
/// Fails if the menu or the icon cannot be created.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let mode = MenuItem::with_id(app, MODE_ID, mode_label(app), true, None::<&str>)?;
    let visibility = MenuItem::with_id(
        app,
        VISIBILITY_ID,
        visibility_label(app),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_ID, QUIT_LABEL, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&mode, &visibility, &quit])?;

    let mut tray = TrayIconBuilder::new()
        .tooltip(TOOLTIP)
        .menu(&menu)
        .on_menu_event(on_menu_event);
    match app.default_window_icon() {
        Some(icon) => tray = tray.icon(icon.clone()),
        None => log::warn!("no app icon, the tray icon may be blank"),
    }
    tray.build(app)?;

    app.manage(TrayItems { mode, visibility });
    Ok(())
}

/// Updates the menu labels to the current mode and visibility, whatever
/// changed them.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let Some(items) = app.try_state::<TrayItems<R>>() else {
        return;
    };
    if let Err(error) = items.mode.set_text(mode_label(app)) {
        log::error!("cannot update the tray mode entry: {error}");
    }
    if let Err(error) = items.visibility.set_text(visibility_label(app)) {
        log::error!("cannot update the tray visibility entry: {error}");
    }
}

#[allow(clippy::needless_pass_by_value)] // Tauri passes the event by value.
fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        MODE_ID => {
            if let Err(error) = overlay_window::toggle(app) {
                log::error!("cannot toggle the overlay mode: {error}");
            }
        }
        VISIBILITY_ID => {
            if let Err(error) = overlay_window::set_visible(app, !is_visible(app)) {
                log::error!("cannot show or hide the overlay: {error}");
            }
        }
        QUIT_ID => {
            app.exit(0);
            return;
        }
        _ => return,
    }
    refresh(app);
}

fn mode_label<R: Runtime>(app: &AppHandle<R>) -> &'static str {
    tray_menu::mode_item_label(overlay_window::current_mode(app))
}

fn visibility_label<R: Runtime>(app: &AppHandle<R>) -> &'static str {
    tray_menu::visibility_item_label(is_visible(app))
}

/// Whether the overlay is shown. Assumes it is when the window cannot tell,
/// so the menu still offers to hide it.
fn is_visible<R: Runtime>(app: &AppHandle<R>) -> bool {
    overlay_window::window(app)
        .and_then(|window| window.is_visible())
        .unwrap_or_else(|error| {
            log::error!("cannot read the overlay visibility: {error}");
            true
        })
}
