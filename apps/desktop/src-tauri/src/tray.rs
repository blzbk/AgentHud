//! Tray icon of the app: switches the overlay mode, shows or hides the overlay,
//! toggles the remote quota sources and quits. The labels live in the `overlay` crate; this module is the
//! Tauri glue.

use std::sync::Arc;

use overlay::tray_menu::{self, CLAUDE_OAUTH_LABEL, CODEX_APP_SERVER_LABEL, QUIT_LABEL};
use overlay::Options;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

use crate::options_store::OptionsState;
use crate::overlay_window;
use crate::remote_poll::RemotePolls;

const MODE_ID: &str = "mode";
const VISIBILITY_ID: &str = "visibility";
const QUIT_ID: &str = "quit";
const CLAUDE_OAUTH_ID: &str = "claude-oauth";
const CODEX_APP_SERVER_ID: &str = "codex-app-server";

/// Tooltip shown when hovering the tray icon.
const TOOLTIP: &str = "AgentHud";

/// Menu entries whose label or check follows the app state, managed as
/// Tauri state.
struct TrayItems<R: Runtime> {
    mode: MenuItem<R>,
    visibility: MenuItem<R>,
    claude_oauth: CheckMenuItem<R>,
    codex_app_server: CheckMenuItem<R>,
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
    let options = app.state::<OptionsState>().current();
    let codex_app_server = CheckMenuItem::with_id(
        app,
        CODEX_APP_SERVER_ID,
        CODEX_APP_SERVER_LABEL,
        true,
        options.codex_app_server,
        None::<&str>,
    )?;
    let claude_oauth = CheckMenuItem::with_id(
        app,
        CLAUDE_OAUTH_ID,
        CLAUDE_OAUTH_LABEL,
        true,
        options.claude_oauth,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, QUIT_ID, QUIT_LABEL, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &mode,
            &visibility,
            &PredefinedMenuItem::separator(app)?,
            &codex_app_server,
            &claude_oauth,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut tray = TrayIconBuilder::new()
        .tooltip(TOOLTIP)
        .menu(&menu)
        .on_menu_event(on_menu_event);
    match app.default_window_icon() {
        Some(icon) => tray = tray.icon(icon.clone()),
        None => log::warn!("no app icon, the tray icon may be blank"),
    }
    tray.build(app)?;

    app.manage(TrayItems {
        mode,
        visibility,
        claude_oauth,
        codex_app_server,
    });
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
    let Some(options) = app.try_state::<OptionsState>().map(|state| state.current()) else {
        return;
    };
    for (item, checked) in [
        (&items.claude_oauth, options.claude_oauth),
        (&items.codex_app_server, options.codex_app_server),
    ] {
        if let Err(error) = item.set_checked(checked) {
            log::error!("cannot update a tray option entry: {error}");
        }
    }
}

/// Flips one option, applies it to its remote source and saves it.
fn toggle_option<R: Runtime>(app: &AppHandle<R>, flip: impl FnOnce(&mut Options)) {
    let options = app.state::<OptionsState>().update(app, flip);
    let remote = app.state::<Arc<RemotePolls>>();
    remote.claude_oauth.set_enabled(options.claude_oauth);
    remote
        .codex_app_server
        .set_enabled(options.codex_app_server);
    log::info!(
        "options: Claude OAuth {}, Codex app-server {}",
        options.claude_oauth,
        options.codex_app_server
    );
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
        CLAUDE_OAUTH_ID => toggle_option(app, |options| {
            options.claude_oauth = !options.claude_oauth;
        }),
        CODEX_APP_SERVER_ID => toggle_option(app, |options| {
            options.codex_app_server = !options.codex_app_server;
        }),
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
