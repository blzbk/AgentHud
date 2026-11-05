//! Polls the remote quota sources (`codex app-server`, the Claude OAuth
//! endpoint) on their own schedule, away from the file polling. The rules
//! live in `application` (`PollScheduler`); this module is the Tauri glue.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use application::{statusline_is_stale, Activity, PolledUsage, UsageSource};
use domain::Provider;
use tokio::time::MissedTickBehavior;

/// How often the schedules are checked. Far below the 1 min floor.
const CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// The remote sources and what their schedules depend on.
pub struct RemotePolls {
    pub codex_app_server: Arc<PolledUsage>,
    pub claude_oauth: Arc<PolledUsage>,
    /// Official Claude quotas: while fresh, OAuth is never polled.
    pub claude_statusline: Arc<dyn UsageSource>,
    activity: Mutex<Activity>,
    /// File keeping the time of the last OAuth poll across launches.
    oauth_marker: Mutex<Option<PathBuf>>,
}

impl RemotePolls {
    pub fn new(
        codex_app_server: Arc<PolledUsage>,
        claude_oauth: Arc<PolledUsage>,
        claude_statusline: Arc<dyn UsageSource>,
    ) -> Self {
        Self {
            codex_app_server,
            claude_oauth,
            claude_statusline,
            activity: Mutex::new(Activity::Quiet),
            oauth_marker: Mutex::new(None),
        }
    }

    /// Keeps the time of the last OAuth poll in `marker`, and resumes from
    /// the one a previous launch saved there: a restart never polls OAuth
    /// sooner than its floor (13 min).
    pub fn remember_oauth_polls(&self, marker: PathBuf) {
        match read_epoch(&marker) {
            Ok(Some(at)) => self.claude_oauth.resume(at),
            Ok(None) => {}
            Err(error) => log::warn!("last OAuth poll time unreadable: {error}"),
        }
        *self
            .oauth_marker
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(marker);
    }

    /// Records the activity of the agents, seen by the file polling.
    pub fn set_activity(&self, activity: Activity) {
        *self.activity.lock().unwrap_or_else(PoisonError::into_inner) = activity;
    }

    /// Whether a remote source of `provider` found an expired login.
    pub fn auth_expired(&self, provider: Provider) -> bool {
        match provider {
            Provider::Codex => self.codex_app_server.auth_expired(),
            Provider::Claude => self.claude_oauth.auth_expired(),
        }
    }

    /// Polls each source whose poll is due. OAuth only polls while the
    /// statusline quotas are absent or stale.
    fn poll_due(&self) {
        let now = adapter_fs::epoch_seconds(SystemTime::now());
        let activity = *self.activity.lock().unwrap_or_else(PoisonError::into_inner);
        self.codex_app_server.observe_activity(activity);
        self.claude_oauth.observe_activity(activity);
        self.codex_app_server.poll_if_due(now);
        if self.claude_oauth.is_enabled() {
            let statusline = self.claude_statusline.latest().unwrap_or_else(|error| {
                log::warn!("Claude statusline unreadable: {error}");
                None
            });
            if statusline_is_stale(statusline.as_ref(), now) && self.claude_oauth.poll_if_due(now) {
                self.save_oauth_poll(now);
            }
        }
    }
}

impl RemotePolls {
    fn save_oauth_poll(&self, at: domain::EpochSeconds) {
        let marker = self
            .oauth_marker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(marker) = marker {
            if let Err(error) = write_epoch(&marker, at) {
                log::warn!("cannot save the last OAuth poll time: {error}");
            }
        }
    }
}

/// Epoch seconds saved in `path`, `None` when the file does not exist.
fn read_epoch(path: &Path) -> std::io::Result<Option<domain::EpochSeconds>> {
    match std::fs::read_to_string(path) {
        Ok(text) => text
            .trim()
            .parse()
            .map(Some)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn write_epoch(path: &Path, at: domain::EpochSeconds) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, at.to_string())
}

/// Starts the remote polling task. It runs for the whole life of the app.
pub fn start(remote: Arc<RemotePolls>) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(CHECK_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let remote = Arc::clone(&remote);
            // Processes and HTTP block: keep them off the async workers.
            if let Err(error) =
                tauri::async_runtime::spawn_blocking(move || remote.poll_due()).await
            {
                log::error!("remote usage poll failed: {error}");
            }
        }
    });
}
