//! Polls the remote quota sources (`codex app-server`, the Claude OAuth
//! endpoint) on their own schedule, away from the file polling. The rules
//! live in `application` (`PollScheduler`); this module is the Tauri glue.

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
        }
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
            if statusline_is_stale(statusline.as_ref(), now) {
                self.claude_oauth.poll_if_due(now);
            }
        }
    }
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
