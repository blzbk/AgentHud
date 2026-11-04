//! Composition root: the only place that assembles `application` with its
//! adapters, and the Tauri shell that hosts the overlay.

/// Starts the desktop app and blocks until it exits.
///
/// # Panics
///
/// Panics if the Tauri runtime cannot start.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("failed to run the AgentHud desktop app");
}
