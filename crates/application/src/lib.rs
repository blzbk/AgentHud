//! Use cases and the ports they need (`UsageSource`, `AgentStateSource`).
//!
//! Depends on `domain` only; adapters implement the ports.

mod ports;
mod refresh_usage;

pub use ports::{SourceError, UsageSource};
pub use refresh_usage::{ProviderUsage, RefreshUsage};
