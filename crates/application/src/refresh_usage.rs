//! Use case: get the current usage of every tracked provider.

use domain::{Provider, UsageSnapshot};

use crate::ports::UsageSource;

/// Usage of one provider, `Unknown` when it cannot be told.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderUsage {
    Known(UsageSnapshot),
    Unknown(Provider),
}

impl ProviderUsage {
    #[must_use]
    pub const fn provider(&self) -> Provider {
        match self {
            Self::Known(snapshot) => snapshot.provider,
            Self::Unknown(provider) => *provider,
        }
    }
}

/// Reads every source once and reports one usage per source.
pub struct RefreshUsage {
    sources: Vec<Box<dyn UsageSource>>,
}

impl RefreshUsage {
    #[must_use]
    pub fn new(sources: Vec<Box<dyn UsageSource>>) -> Self {
        Self { sources }
    }

    /// One usage per source, in source order. A failing source is logged and
    /// reported `Unknown`, so one provider never hides the others.
    #[must_use]
    pub fn execute(&self) -> Vec<ProviderUsage> {
        self.sources
            .iter()
            .map(|source| match source.latest() {
                Ok(Some(snapshot)) => ProviderUsage::Known(snapshot),
                Ok(None) => ProviderUsage::Unknown(source.provider()),
                Err(error) => {
                    log::warn!("{} usage unknown: {error}", source.provider().label());
                    ProviderUsage::Unknown(source.provider())
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use domain::{Provider, UsageSnapshot, UsageWindow, WindowKind};

    use super::{ProviderUsage, RefreshUsage};
    use crate::ports::{SourceError, UsageSource};

    enum Outcome {
        Snapshot(UsageSnapshot),
        Nothing,
        Failure,
    }

    struct FakeSource(Provider, Outcome);

    impl UsageSource for FakeSource {
        fn provider(&self) -> Provider {
            self.0
        }

        fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError> {
            match &self.1 {
                Outcome::Snapshot(snapshot) => Ok(Some(snapshot.clone())),
                Outcome::Nothing => Ok(None),
                Outcome::Failure => Err(SourceError::Io {
                    path: PathBuf::from("sessions"),
                    source: std::io::Error::other("unreadable"),
                }),
            }
        }
    }

    fn snapshot() -> UsageSnapshot {
        UsageSnapshot {
            provider: Provider::Codex,
            five_hours: UsageWindow::new(WindowKind::FiveHours, 12.0, Some(1_793_844_772)),
            weekly: UsageWindow::new(WindowKind::Weekly, 51.0, Some(1_794_184_565)),
            observed_at: 1_793_840_000,
        }
    }

    fn refresh(source: FakeSource) -> Vec<ProviderUsage> {
        RefreshUsage::new(vec![Box::new(source)]).execute()
    }

    #[test]
    fn returns_the_snapshot_of_a_source() {
        let usage = refresh(FakeSource(Provider::Codex, Outcome::Snapshot(snapshot())));

        assert_eq!(usage, [ProviderUsage::Known(snapshot())]);
    }

    #[test]
    fn reports_unknown_when_the_source_has_no_data() {
        let usage = refresh(FakeSource(Provider::Codex, Outcome::Nothing));

        assert_eq!(usage, [ProviderUsage::Unknown(Provider::Codex)]);
    }

    #[test]
    fn reports_unknown_when_the_source_fails() {
        let usage = refresh(FakeSource(Provider::Claude, Outcome::Failure));

        assert_eq!(usage, [ProviderUsage::Unknown(Provider::Claude)]);
    }

    #[test]
    fn keeps_one_usage_per_source_in_order() {
        let usage = RefreshUsage::new(vec![
            Box::new(FakeSource(Provider::Claude, Outcome::Failure)),
            Box::new(FakeSource(Provider::Codex, Outcome::Snapshot(snapshot()))),
        ])
        .execute();

        assert_eq!(
            usage
                .iter()
                .map(ProviderUsage::provider)
                .collect::<Vec<_>>(),
            [Provider::Claude, Provider::Codex]
        );
    }
}
