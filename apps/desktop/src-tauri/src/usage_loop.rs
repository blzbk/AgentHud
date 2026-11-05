//! Polls the providers' usage and agents in the background and pushes them
//! to the UI. The reading itself lives in the adapters; this module is the
//! Tauri glue and the composition of the sources.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use adapter_claude::{
    ClaudeAgentSessions, ClaudeOAuthUsage, ClaudeStatuslineSource, ClaudeUsageSource,
    UreqTransport, CLAUDE_DIR,
};
use adapter_codex::{CodexAgentSessions, CodexAppServer, CodexUsageSource, CODEX_DIR};
use adapter_fs::{CachedLister, WslCli};
use application::{
    Activity, AgentReport, FreshestUsage, PollPolicy, PolledUsage, RefreshUsage, TrackAgents,
    UsageReport, UsageSource,
};
use domain::Provider;
use overlay::Options;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tokio::time::MissedTickBehavior;

use crate::remote_poll::RemotePolls;

/// Event emitted to the UI with the new usage reports as payload.
const USAGE_EVENT: &str = "usage";

/// Event emitted to the UI with the new agent reports as payload.
const AGENTS_EVENT: &str = "agents";

/// Time between two reads: mtime polling, never a file watcher.
const POLL_INTERVAL: Duration = Duration::from_secs(3);

/// Providers in display order.
const PROVIDERS: [Provider; 2] = [Provider::Codex, Provider::Claude];

/// Last reports of one kind sent to the UI, managed as Tauri state.
pub struct Published<T>(Mutex<Vec<T>>);

impl Default for Published<UsageReport> {
    fn default() -> Self {
        Self(Mutex::new(PROVIDERS.map(UsageReport::unknown).to_vec()))
    }
}

impl Default for Published<AgentReport> {
    fn default() -> Self {
        Self(Mutex::new(PROVIDERS.map(AgentReport::unknown).to_vec()))
    }
}

impl<T: Clone> Published<T> {
    fn current(&self) -> Vec<T> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Current usage, read by the UI on load so a reloaded page shows the truth.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri injects `State` by value.
pub fn usage(state: State<'_, Published<UsageReport>>) -> Vec<UsageReport> {
    state.current()
}

/// Current agents, read by the UI on load.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri injects `State` by value.
pub fn agents(state: State<'_, Published<AgentReport>>) -> Vec<AgentReport> {
    state.current()
}

/// Use cases of the file polling, and the remote sources they share.
pub struct Composition {
    usage: RefreshUsage,
    agents: TrackAgents,
    pub remote: Arc<RemotePolls>,
}

/// Usage and agent use cases wired to the Codex and Claude adapters, in
/// display order. All sources share one cached list of WSL distributions.
///
/// - Codex: the freshest of the session files and `codex app-server`.
/// - Claude: the statusline or OAuth quotas when known, else the estimate
///   from the transcripts.
pub fn compose(options: Options) -> Composition {
    let lister = Arc::new(CachedLister::new(WslCli));
    let roots = |dir: &'static str| {
        let lister = Arc::clone(&lister);
        move || adapter_fs::home_roots(dir, &*lister)
    };
    let codex_app_server = Arc::new(PolledUsage::new(
        Box::new(CodexAppServer::new(Arc::clone(&lister))),
        PollPolicy::CODEX_APP_SERVER,
        options.codex_app_server,
    ));
    let claude_oauth = Arc::new(PolledUsage::new(
        Box::new(ClaudeOAuthUsage::new(
            roots(CLAUDE_DIR),
            UreqTransport::default(),
        )),
        PollPolicy::CLAUDE_OAUTH,
        options.claude_oauth,
    ));
    let claude_statusline: Arc<dyn UsageSource> =
        Arc::new(ClaudeStatuslineSource::new(roots(CLAUDE_DIR)));
    let usage = RefreshUsage::new(vec![
        Box::new(FreshestUsage::new(
            Provider::Codex,
            vec![
                Box::new(CodexUsageSource::new(roots(CODEX_DIR))),
                Box::new(Arc::clone(&codex_app_server)),
            ],
        )),
        Box::new(FreshestUsage::new(
            Provider::Claude,
            vec![
                Box::new(Arc::clone(&claude_statusline)),
                Box::new(Arc::clone(&claude_oauth)),
                Box::new(ClaudeUsageSource::new(roots(CLAUDE_DIR))),
            ],
        )),
    ]);
    let agents = TrackAgents::new(vec![
        Box::new(CodexAgentSessions::new(roots(CODEX_DIR))),
        Box::new(ClaudeAgentSessions::new(roots(CLAUDE_DIR))),
    ]);
    let remote = Arc::new(RemotePolls::new(
        codex_app_server,
        claude_oauth,
        claude_statusline,
    ));
    Composition {
        usage,
        agents,
        remote,
    }
}

/// Starts the file polling task. It runs for the whole life of the app.
pub fn start<R: Runtime>(app: &AppHandle<R>, composition: Composition) {
    let app = app.clone();
    let composition = Arc::new(composition);
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            let composition = Arc::clone(&composition);
            // File reads block: keep them off the async workers.
            let reports = tauri::async_runtime::spawn_blocking(move || {
                let Composition {
                    usage,
                    agents,
                    remote,
                } = &*composition;
                let now = adapter_fs::epoch_seconds(SystemTime::now());
                let usage: Vec<UsageReport> = usage
                    .execute()
                    .iter()
                    .map(|usage| {
                        UsageReport::new(usage, now)
                            .with_auth_expired(remote.auth_expired(usage.provider()))
                    })
                    .collect();
                let agents = agents.execute(now);
                remote.set_activity(Activity::of(&agents, now));
                let agents: Vec<AgentReport> = agents
                    .iter()
                    .map(|agents| AgentReport::new(agents, now))
                    .collect();
                (usage, agents)
            })
            .await;
            match reports {
                Ok((usage, agents)) => {
                    publish(&app, USAGE_EVENT, usage);
                    publish(&app, AGENTS_EVENT, agents);
                }
                Err(error) => log::error!("usage refresh failed: {error}"),
            }
        }
    });
}

/// Stores `reports` and notifies the UI on `event`, only when they changed.
fn publish<R, T>(app: &AppHandle<R>, event: &str, reports: Vec<T>)
where
    R: Runtime,
    T: PartialEq + Serialize + Clone + Send + Sync + 'static,
{
    let state = app.state::<Published<T>>();
    let mut current = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    if *current == reports {
        return;
    }
    if let Err(error) = app.emit(event, &reports) {
        log::error!("cannot send the {event} to the UI: {error}");
        return;
    }
    *current = reports;
}
