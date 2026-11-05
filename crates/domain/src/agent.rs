//! Activity of a coding agent, deduced from the last turn event of its
//! session and from how recently the session was written.

use crate::usage::{EpochSeconds, Provider};

/// A session not written for this long is shown `idle`.
pub const IDLE_AFTER: EpochSeconds = 5 * 60;

/// A session written within this delay is active; older ones are dropped.
pub const ACTIVE_WITHIN: EpochSeconds = 60 * 60;

/// Activity state of a coding agent, as shown by its badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Waiting,
    Working,
    Idle,
    Offline,
    Unknown,
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
            Self::Unknown => "unknown",
        }
    }
}

/// Last turn event found in a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnSignal {
    /// The agent is in a turn: thinking, calling tools or answering.
    TurnStarted,
    /// The turn is over: the agent waits for the user.
    TurnEnded,
    /// No turn event could be read.
    Unknown,
}

/// One session of a coding agent, as last observed on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    pub provider: Provider,
    pub session_id: String,
    pub project: Option<String>,
    pub last_signal: TurnSignal,
    /// Last modification of the session file.
    pub last_write: EpochSeconds,
}

impl AgentSession {
    /// Whether the session was written recently enough to be shown.
    #[must_use]
    pub const fn is_active_at(&self, now: EpochSeconds) -> bool {
        now - self.last_write < ACTIVE_WITHIN
    }

    /// State at `now`: `idle` once the session is quiet for [`IDLE_AFTER`],
    /// otherwise deduced from the last turn event.
    #[must_use]
    pub const fn state_at(&self, now: EpochSeconds) -> AgentState {
        if now - self.last_write >= IDLE_AFTER {
            return AgentState::Idle;
        }
        match self.last_signal {
            TurnSignal::TurnStarted => AgentState::Working,
            TurnSignal::TurnEnded => AgentState::Waiting,
            TurnSignal::Unknown => AgentState::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentSession, AgentState, TurnSignal, ACTIVE_WITHIN, IDLE_AFTER};
    use crate::Provider;

    const NOW: i64 = 1_793_840_000;

    fn session(last_signal: TurnSignal, age: i64) -> AgentSession {
        AgentSession {
            provider: Provider::Claude,
            session_id: "session".to_owned(),
            project: None,
            last_signal,
            last_write: NOW - age,
        }
    }

    #[test]
    fn each_state_has_a_distinct_label() {
        let labels = [
            AgentState::Waiting,
            AgentState::Working,
            AgentState::Idle,
            AgentState::Offline,
            AgentState::Unknown,
        ]
        .map(AgentState::label);

        assert_eq!(labels, ["waiting", "working", "idle", "offline", "unknown"]);
    }

    #[test]
    fn a_started_turn_is_working() {
        assert_eq!(
            session(TurnSignal::TurnStarted, 10).state_at(NOW),
            AgentState::Working
        );
    }

    #[test]
    fn an_ended_turn_is_waiting() {
        assert_eq!(
            session(TurnSignal::TurnEnded, 30).state_at(NOW),
            AgentState::Waiting
        );
    }

    #[test]
    fn an_unreadable_turn_is_unknown() {
        assert_eq!(
            session(TurnSignal::Unknown, 30).state_at(NOW),
            AgentState::Unknown
        );
    }

    #[test]
    fn a_quiet_session_is_idle_whatever_its_signal() {
        for signal in [
            TurnSignal::TurnStarted,
            TurnSignal::TurnEnded,
            TurnSignal::Unknown,
        ] {
            assert_eq!(session(signal, 6 * 60).state_at(NOW), AgentState::Idle);
        }
    }

    #[test]
    fn idle_starts_exactly_after_five_minutes() {
        let started = |age| session(TurnSignal::TurnStarted, age).state_at(NOW);

        assert_eq!(started(IDLE_AFTER - 1), AgentState::Working);
        assert_eq!(started(IDLE_AFTER), AgentState::Idle);
    }

    #[test]
    fn a_session_stays_active_for_sixty_minutes() {
        let active = |age| session(TurnSignal::TurnEnded, age).is_active_at(NOW);

        assert!(active(ACTIVE_WITHIN - 1));
        assert!(!active(ACTIVE_WITHIN));
    }
}
