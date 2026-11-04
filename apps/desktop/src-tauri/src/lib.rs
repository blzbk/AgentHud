//! Composition root: the only place that assembles `application` with its
//! adapters, and the Tauri shell that hosts the overlay.

mod overlay;

use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Starts the desktop app and blocks until it exits.
///
/// # Panics
///
/// Panics if the Tauri runtime cannot start.
pub fn run() {
    let toggle_shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyH);

    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if *shortcut == toggle_shortcut && event.state == ShortcutState::Pressed {
                        if let Err(error) = overlay::toggle(app) {
                            log::error!("cannot toggle the overlay mode: {error}");
                        }
                    }
                })
                .build(),
        )
        .manage(overlay::OverlayState::default())
        .setup(move |app| {
            overlay::init(app.handle())?;
            if let Err(error) = app.global_shortcut().register(toggle_shortcut) {
                log::error!("cannot register {toggle_shortcut}, staying click-through: {error}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run the AgentHud desktop app");
}
