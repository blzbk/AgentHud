//! Loads and saves the tray options in `<app_config_dir>/options.json`. The
//! options themselves live in the `overlay` crate; this module is the I/O.

use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use overlay::Options;
use tauri::{AppHandle, Manager, Runtime};

/// File holding the options, in the app config directory.
const OPTIONS_FILE: &str = "options.json";

#[derive(Debug, thiserror::Error)]
enum OptionsError {
    #[error("no app config directory: {0}")]
    ConfigDir(tauri::Error),
    #[error("cannot access the options file: {0}")]
    Io(#[from] std::io::Error),
    #[error("unreadable options file: {0}")]
    Format(#[from] serde_json::Error),
}

/// Current options, managed as Tauri state.
pub struct OptionsState(Mutex<Options>);

impl OptionsState {
    pub fn current(&self) -> Options {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Changes the options with `change`, saves them, and returns them.
    /// A failed save is logged: the change still applies until the app quits.
    pub fn update<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        change: impl FnOnce(&mut Options),
    ) -> Options {
        let mut options = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        change(&mut options);
        if let Err(error) = save(app, *options) {
            log::error!("cannot save the options: {error}");
        }
        *options
    }
}

/// Options saved by a previous launch, or the defaults when there are none
/// or they cannot be read.
pub fn load<R: Runtime>(app: &AppHandle<R>) -> OptionsState {
    let options = match read(app) {
        Ok(Some(options)) => options,
        Ok(None) => Options::default(),
        Err(error) => {
            log::error!("using the default options: {error}");
            Options::default()
        }
    };
    OptionsState(Mutex::new(options))
}

fn read<R: Runtime>(app: &AppHandle<R>) -> Result<Option<Options>, OptionsError> {
    match std::fs::read_to_string(options_file(app)?) {
        Ok(json) => Ok(Some(Options::from_json(&json)?)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn save<R: Runtime>(app: &AppHandle<R>, options: Options) -> Result<(), OptionsError> {
    let file = options_file(app)?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(file, options.to_json()?)?;
    Ok(())
}

fn options_file<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, OptionsError> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(OptionsError::ConfigDir)?;
    Ok(dir.join(OPTIONS_FILE))
}
