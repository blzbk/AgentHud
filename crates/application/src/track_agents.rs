//! Use case: list the active agents of every tracked provider.

use domain::{AgentSession, EpochSeconds, Provider};

use crate::ports::AgentStateSource;

/// Agents of one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderAgents {
    /// Active sessions, most recently written first. Never empty.
    Active(Provider, Vec<AgentSession>),
    /// No session written recently: no agent is running.
    Offline(Provider),
    /// The sessions could not be read.
    Unknown(Provider),
}

impl ProviderAgents {
    #[must_use]
    pub const fn provider(&self) -> Provider {
        match self {
            Self::Active(provider, _) | Self::Offline(provider) | Self::Unknown(provider) => {
                *provider
            }
        }
    }
}

/// Reads every source once and reports the agents of each provider.
pub struct TrackAgents {
    sources: Vec<Box<dyn AgentStateSource>>,
}

impl TrackAgents {
    #[must_use]
    pub fn new(sources: Vec<Box<dyn AgentStateSource>>) -> Self {
        Self { sources }
    }

    /// One entry per source, in source order. Sessions older than
    /// `ACTIVE_WITHIN` at `now` are dropped. A failing source is logged and
    /// reported `Unknown`, so one provider never hides the others.
    #[must_use]
    pub fn execute(&self, now: EpochSeconds) -> Vec<ProviderAgents> {
        self.sources
            .iter()
            .map(|source| {
                let provider = source.provider();
                match source.sessions() {
                    Ok(mut sessions) => {
                        sessions.retain(|session| session.is_active_at(now));
                        if sessions.is_empty() {
                            return ProviderAgents::Offline(provider);
                        }
                        sessions.sort_by_key(|session| std::cmp::Reverse(session.last_write));
                        ProviderAgents::Active(provider, sessions)
                    }
                    Err(error) => {
                        log::warn!("{} agents unknown: {error}", provider.label());
                        ProviderAgents::Unknown(provider)
                    }
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use domain::{AgentSession, Provider, TurnSignal, ACTIVE_WITHIN};

    use super::{ProviderAgents, TrackAgents};
    use crate::ports::{AgentStateSource, SourceError};

    const NOW: i64 = 1_793_840_000;

    struct FakeSource(Provider, Option<Vec<AgentSession>>);

    impl AgentStateSource for FakeSource {
        fn provider(&self) -> Provider {
            self.0
        }

        fn sessions(&self) -> Result<Vec<AgentSession>, SourceError> {
            self.1.clone().ok_or_else(|| SourceError::Io {
                path: PathBuf::from("projects"),
                source: std::io::Error::other("unreadable"),
            })
        }
    }

    fn session(id: &str, age: i64) -> AgentSession {
        AgentSession {
            provider: Provider::Claude,
            session_id: id.to_owned(),
            project: None,
            last_signal: TurnSignal::TurnEnded,
            last_write: NOW - age,
        }
    }

    fn track(source: FakeSource) -> Vec<ProviderAgents> {
        TrackAgents::new(vec![Box::new(source)]).execute(NOW)
    }

    #[test]
    fn sorts_active_sessions_by_last_write() {
        let agents = track(FakeSource(
            Provider::Claude,
            Some(vec![
                session("old", 600),
                session("new", 5),
                session("mid", 60),
            ]),
        ));

        assert_eq!(
            agents,
            [ProviderAgents::Active(
                Provider::Claude,
                vec![session("new", 5), session("mid", 60), session("old", 600)]
            )]
        );
    }

    #[test]
    fn drops_sessions_older_than_the_active_delay() {
        let agents = track(FakeSource(
            Provider::Claude,
            Some(vec![session("live", 10), session("gone", ACTIVE_WITHIN)]),
        ));

        assert_eq!(
            agents,
            [ProviderAgents::Active(
                Provider::Claude,
                vec![session("live", 10)]
            )]
        );
    }

    #[test]
    fn a_provider_without_active_session_is_offline() {
        assert_eq!(
            track(FakeSource(Provider::Codex, Some(vec![]))),
            [ProviderAgents::Offline(Provider::Codex)]
        );
        assert_eq!(
            track(FakeSource(
                Provider::Codex,
                Some(vec![session("gone", ACTIVE_WITHIN + 1)])
            )),
            [ProviderAgents::Offline(Provider::Codex)]
        );
    }

    #[test]
    fn a_failing_source_is_unknown_without_hiding_the_others() {
        let agents = TrackAgents::new(vec![
            Box::new(FakeSource(Provider::Codex, None)),
            Box::new(FakeSource(Provider::Claude, Some(vec![]))),
        ])
        .execute(NOW);

        assert_eq!(
            agents,
            [
                ProviderAgents::Unknown(Provider::Codex),
                ProviderAgents::Offline(Provider::Claude)
            ]
        );
    }
}
