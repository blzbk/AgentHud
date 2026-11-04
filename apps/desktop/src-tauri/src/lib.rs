//! Composition root and Tauri shell hosting the overlay. It is the only place
//! allowed to assemble `application` with its adapters (see `usage_loop`).

mod overlay_window;
mod usage_loop;

use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Starts the desktop app and blocks until it exits.
///
/// # Panics
///
/// Panics if the Tauri runtime cannot start.
pub fn run() {
    let toggle_shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyH);

    tauri::Builder::default()
        // Must be the first plugin: a second launch exits before anything else
        // starts, so the global shortcut always belongs to the one overlay.
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
            log::info!("AgentHud is already running, second launch ignored");
        }))
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if *shortcut == toggle_shortcut && event.state == ShortcutState::Pressed {
                        if let Err(error) = overlay_window::toggle(app) {
                            log::error!("cannot toggle the overlay mode: {error}");
                        }
                    }
                })
                .build(),
        )
        .manage(overlay_window::OverlayState::default())
        .manage(usage_loop::UsageState::default())
        .invoke_handler(tauri::generate_handler![
            overlay_window::overlay_mode,
            usage_loop::usage
        ])
        .setup(move |app| {
            overlay_window::init(app.handle())?;
            usage_loop::start(app.handle());
            if let Err(error) = app.global_shortcut().register(toggle_shortcut) {
                log::error!("cannot register {toggle_shortcut}, staying click-through: {error}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run the AgentHud desktop app");
}
