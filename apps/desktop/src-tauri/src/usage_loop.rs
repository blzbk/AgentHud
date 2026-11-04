//! Polls the providers' usage in the background and pushes it to the UI.
//! The reading itself lives in the adapters; this module is the Tauri glue.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use adapter_codex::{CodexUsageSource, CODEX_DIR};
use application::{RefreshUsage, UsageReport};
use domain::{EpochSeconds, Provider};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::time::MissedTickBehavior;

/// Event emitted to the UI with the new reports as payload.
const USAGE_EVENT: &str = "usage";

/// Time between two reads: mtime polling, never a file watcher.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

/// Last reports sent to the UI, managed as Tauri state.
pub struct UsageState(Mutex<Vec<UsageReport>>);

impl Default for UsageState {
    fn default() -> Self {
        Self(Mutex::new(vec![UsageReport::unknown(Provider::Codex)]))
    }
}

/// Current reports, read by the UI on load so a reloaded page shows the truth.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri injects `State` by value.
pub fn usage(state: State<'_, UsageState>) -> Vec<UsageReport> {
    state
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Starts the polling task. It runs for the whole life of the app.
pub fn start<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    let refresh = Arc::new(RefreshUsage::new(vec![Box::new(CodexUsageSource::new(
        || adapter_fs::home_roots(CODEX_DIR, &adapter_fs::WslCli),
    ))]));
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let refresh = Arc::clone(&refresh);
            // File reads block: keep them off the async workers.
            let reports = tauri::async_runtime::spawn_blocking(move || {
                let now = now();
                refresh
                    .execute()
                    .iter()
                    .map(|usage| UsageReport::new(usage, now))
                    .collect::<Vec<_>>()
            })
            .await;
            match reports {
                Ok(reports) => publish(&app, reports),
                Err(error) => log::error!("usage refresh failed: {error}"),
            }
        }
    });
}

/// Stores `reports` and notifies the UI, only when they changed.
fn publish<R: Runtime>(app: &AppHandle<R>, reports: Vec<UsageReport>) {
    let state = app.state::<UsageState>();
    let mut current = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    if *current == reports {
        return;
    }
    if let Err(error) = app.emit(USAGE_EVENT, &reports) {
        log::error!("cannot send the usage to the UI: {error}");
        return;
    }
    *current = reports;
}

fn now() -> EpochSeconds {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| EpochSeconds::try_from(elapsed.as_secs()).ok())
        .unwrap_or_default()
}
