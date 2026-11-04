//! Pure business model of `AgentHud`: entities and value objects, without I/O.

mod usage;

pub use usage::{EpochSeconds, Provider, UsageSnapshot, UsageWindow, WindowKind};

/// Activity state of a coding agent, as shown by its badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Waiting,
    Working,
    Idle,
    Offline,
}

impl AgentState {
    /// Label displayed next to the state color.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Working => "working",
            Self::Idle => "idle",
            Self::Offline => "offline",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentState;

    #[test]
    fn each_state_has_a_distinct_label() {
        let labels = [
            AgentState::Waiting,
            AgentState::Working,
            AgentState::Idle,
            AgentState::Offline,
        ]
        .map(AgentState::label);

        assert_eq!(labels, ["waiting", "working", "idle", "offline"]);
    }
}
