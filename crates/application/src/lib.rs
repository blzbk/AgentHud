//! Use cases and the ports they need (`UsageSource`, `AgentStateSource`).
//!
//! Depends on `domain` only; adapters implement the ports.

mod agent_report;
mod ports;
mod refresh_usage;
mod track_agents;
mod usage_report;

pub use agent_report::{AgentEntry, AgentReport, AgentsStatus};
pub use ports::{AgentStateSource, SourceError, UsageSource};
pub use refresh_usage::{ProviderUsage, RefreshUsage};
pub use track_agents::{ProviderAgents, TrackAgents};
pub use usage_report::{UsageReport, UsageStatus, WindowReport};
