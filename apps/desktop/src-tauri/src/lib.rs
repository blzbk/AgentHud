//! Composition root and Tauri shell hosting the overlay. It is the only place
//! allowed to assemble `application` with its adapters (see `usage_loop`).

mod overlay_window;
mod tray;
mod usage_loop;
mod window_position;

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
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("AgentHud is already running, showing the overlay");
            if let Err(error) = overlay_window::set_visible(app, true) {
                log::error!("cannot show the overlay: {error}");
            }
            tray::refresh(app);
        }))
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if *shortcut == toggle_shortcut && event.state == ShortcutState::Pressed {
                        if let Err(error) = overlay_window::toggle(app) {
                            log::error!("cannot toggle the overlay mode: {error}");
                        }
                        tray::refresh(app);
                    }
                })
                .build(),
        )
        .manage(overlay_window::OverlayState::default())
        .manage(usage_loop::Published::<application::UsageReport>::default())
        .manage(usage_loop::Published::<application::AgentReport>::default())
        .invoke_handler(tauri::generate_handler![
            overlay_window::overlay_mode,
            usage_loop::usage,
            usage_loop::agents
        ])
        .setup(move |app| {
            overlay_window::init(app.handle())?;
            let window = overlay_window::window(app.handle())?;
            window_position::restore(&window);
            window_position::remember(&window);
            usage_loop::start(app.handle());
            // Without a tray the shortcut still drives the overlay: keep running.
            if let Err(error) = tray::init(app.handle()) {
                log::error!("cannot create the tray icon: {error}");
            }
            if let Err(error) = app.global_shortcut().register(toggle_shortcut) {
                log::error!("cannot register {toggle_shortcut}, staying click-through: {error}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run the AgentHud desktop app");
}
