//! Pure business model of `AgentHud`: entities and value objects, without I/O.

mod agent;
mod usage;

pub use agent::{AgentSession, AgentState, TurnSignal, ACTIVE_WITHIN, IDLE_AFTER};
pub use usage::{EpochSeconds, Provider, Usage, UsageSnapshot, UsageWindow, WindowKind};
