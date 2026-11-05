//! Use cases and the ports they need (`UsageSource`, `UsageFetcher`,
//! `AgentStateSource`).
//!
//! Depends on `domain` only; adapters implement the ports.

mod agent_report;
mod freshest_usage;
mod poll_schedule;
mod polled_usage;
mod ports;
mod refresh_usage;
mod track_agents;
mod usage_report;

pub use agent_report::{AgentEntry, AgentReport, AgentsStatus};
pub use freshest_usage::FreshestUsage;
pub use poll_schedule::{Activity, PollOutcome, PollPolicy, PollScheduler};
pub use polled_usage::{statusline_is_stale, PolledUsage, STATUSLINE_FRESH_FOR};
pub use ports::{AgentStateSource, FetchError, SourceError, UsageFetcher, UsageSource};
pub use refresh_usage::{ProviderUsage, RefreshUsage};
pub use track_agents::{ProviderAgents, TrackAgents};
pub use usage_report::{UsageReport, UsageStatus, WindowReport};
