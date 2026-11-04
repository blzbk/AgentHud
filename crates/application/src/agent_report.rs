//! Agents as sent to the UI: the wire shape of the `agents` command and
//! event. The TypeScript side mirrors it in `features/agents/useAgents.ts`.

use domain::{AgentSession, EpochSeconds, Provider};
use serde::Serialize;

use crate::ProviderAgents;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentsStatus {
    Active,
    Offline,
    Unknown,
}

/// One agent session. Times are seconds since the Unix epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEntry {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// Label of the `AgentState`: `waiting`, `working`, `idle` or `unknown`.
    pub state: &'static str,
    pub last_write: EpochSeconds,
}

impl AgentEntry {
    fn new(session: &AgentSession, now: EpochSeconds) -> Self {
        Self {
            session_id: session.session_id.clone(),
            project: session.project.clone(),
            state: session.state_at(now).label(),
            last_write: session.last_write,
        }
    }
}

/// Agents of one provider. `agents` is empty unless `status` is `active`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReport {
    pub provider: &'static str,
    pub status: AgentsStatus,
    pub agents: Vec<AgentEntry>,
}

impl AgentReport {
    /// Report of `agents`, with each state deduced at `now`.
    #[must_use]
    pub fn new(agents: &ProviderAgents, now: EpochSeconds) -> Self {
        let (status, entries) = match agents {
            ProviderAgents::Active(_, sessions) => (
                AgentsStatus::Active,
                sessions
                    .iter()
                    .map(|session| AgentEntry::new(session, now))
                    .collect(),
            ),
            ProviderAgents::Offline(_) => (AgentsStatus::Offline, Vec::new()),
            ProviderAgents::Unknown(_) => (AgentsStatus::Unknown, Vec::new()),
        };
        Self {
            provider: agents.provider().label(),
            status,
            agents: entries,
        }
    }

    /// Report of a provider whose agents are not known (yet).
    #[must_use]
    pub const fn unknown(provider: Provider) -> Self {
        Self {
            provider: provider.label(),
            status: AgentsStatus::Unknown,
            agents: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use domain::{AgentSession, Provider, TurnSignal};
    use serde_json::json;

    use super::AgentReport;
    use crate::ProviderAgents;

    const NOW: i64 = 1_793_840_000;

    fn session(id: &str, project: Option<&str>, signal: TurnSignal, age: i64) -> AgentSession {
        AgentSession {
            provider: Provider::Claude,
            session_id: id.to_owned(),
            project: project.map(str::to_owned),
            last_signal: signal,
            last_write: NOW - age,
        }
    }

    #[test]
    fn serializes_active_agents_in_camel_case() {
        let agents = ProviderAgents::Active(
            Provider::Claude,
            vec![
                session("a1", Some("AgentHud"), TurnSignal::TurnStarted, 10),
                session("b2", None, TurnSignal::TurnEnded, 30),
                session("c3", None, TurnSignal::TurnEnded, 600),
            ],
        );

        let report =
            serde_json::to_value(AgentReport::new(&agents, NOW)).expect("report should serialize");

        assert_eq!(
            report,
            json!({
                "provider": "claude",
                "status": "active",
                "agents": [
                    { "sessionId": "a1", "project": "AgentHud", "state": "working", "lastWrite": NOW - 10 },
                    { "sessionId": "b2", "state": "waiting", "lastWrite": NOW - 30 },
                    { "sessionId": "c3", "state": "idle", "lastWrite": NOW - 600 },
                ],
            })
        );
    }

    #[test]
    fn serializes_an_offline_provider_without_agents() {
        let report = serde_json::to_value(AgentReport::new(
            &ProviderAgents::Offline(Provider::Codex),
            NOW,
        ))
        .expect("report should serialize");

        assert_eq!(
            report,
            json!({ "provider": "codex", "status": "offline", "agents": [] })
        );
    }

    #[test]
    fn serializes_an_unknown_provider_without_agents() {
        assert_eq!(
            AgentReport::new(&ProviderAgents::Unknown(Provider::Codex), NOW),
            AgentReport::unknown(Provider::Codex)
        );
        assert_eq!(
            serde_json::to_value(AgentReport::unknown(Provider::Codex))
                .expect("report should serialize"),
            json!({ "provider": "codex", "status": "unknown", "agents": [] })
        );
    }
}
