//! Combines several sources of one provider into the best usage to show.

use domain::{Provider, UsageSnapshot};

use crate::ports::{SourceError, UsageSource};

/// The best usage among several sources of one provider: a published share
/// beats an estimate, then the most recent observation wins.
pub struct FreshestUsage {
    provider: Provider,
    sources: Vec<Box<dyn UsageSource>>,
}

impl FreshestUsage {
    #[must_use]
    pub fn new(provider: Provider, sources: Vec<Box<dyn UsageSource>>) -> Self {
        Self { provider, sources }
    }
}

impl UsageSource for FreshestUsage {
    fn provider(&self) -> Provider {
        self.provider
    }

    /// A failing source is logged and skipped; its error is only returned
    /// when no source gave a usage.
    fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError> {
        let mut best: Option<UsageSnapshot> = None;
        let mut first_error = None;
        for source in &self.sources {
            match source.latest() {
                Ok(Some(snapshot)) => {
                    let rank =
                        |snapshot: &UsageSnapshot| (snapshot.is_exact(), snapshot.observed_at);
                    if best
                        .as_ref()
                        .is_none_or(|best| rank(&snapshot) > rank(best))
                    {
                        best = Some(snapshot);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    log::warn!("{} usage source skipped: {error}", self.provider.label());
                    first_error.get_or_insert(error);
                }
            }
        }
        match (best, first_error) {
            (None, Some(error)) => Err(error),
            (best, _) => Ok(best),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use domain::{Origin, Provider, UsageSnapshot, UsageWindow, WindowKind};

    use super::FreshestUsage;
    use crate::ports::{SourceError, UsageSource};

    const NOW: i64 = 1_793_840_000;

    #[derive(Clone)]
    enum Answer {
        Usage(UsageSnapshot),
        Nothing,
        Failure,
    }

    struct Fixed(Answer);

    impl UsageSource for Fixed {
        fn provider(&self) -> Provider {
            Provider::Claude
        }

        fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError> {
            match &self.0 {
                Answer::Usage(snapshot) => Ok(Some(snapshot.clone())),
                Answer::Nothing => Ok(None),
                Answer::Failure => Err(SourceError::Io {
                    path: PathBuf::from("rate-limits.json"),
                    source: std::io::Error::other("unreadable"),
                }),
            }
        }
    }

    fn exact(origin: Origin, observed_at: i64) -> UsageSnapshot {
        UsageSnapshot {
            provider: Provider::Claude,
            five_hours: UsageWindow::new(WindowKind::FiveHours, 30.0, None),
            weekly: None,
            observed_at,
            origin,
        }
    }

    fn estimate(observed_at: i64) -> UsageSnapshot {
        UsageSnapshot {
            five_hours: Some(UsageWindow::tokens(WindowKind::FiveHours, 10, None).estimated()),
            origin: Origin::Transcripts,
            ..exact(Origin::Transcripts, observed_at)
        }
    }

    fn freshest(sources: Vec<Answer>) -> Result<Option<UsageSnapshot>, SourceError> {
        FreshestUsage::new(
            Provider::Claude,
            sources
                .into_iter()
                .map(|answer| Box::new(Fixed(answer)) as Box<dyn UsageSource>)
                .collect(),
        )
        .latest()
    }

    #[test]
    fn an_exact_usage_beats_a_newer_estimate() {
        let best = freshest(vec![
            Answer::Usage(estimate(NOW)),
            Answer::Usage(exact(Origin::Statusline, NOW - 3600)),
        ]);

        assert_eq!(
            best.expect("a usage is known"),
            Some(exact(Origin::Statusline, NOW - 3600))
        );
    }

    #[test]
    fn the_most_recent_exact_usage_wins() {
        let best = freshest(vec![
            Answer::Usage(exact(Origin::Statusline, NOW - 60)),
            Answer::Usage(exact(Origin::OAuth, NOW)),
        ]);

        assert_eq!(
            best.expect("a usage is known"),
            Some(exact(Origin::OAuth, NOW))
        );
    }

    #[test]
    fn a_failing_source_does_not_hide_the_others() {
        let best = freshest(vec![Answer::Failure, Answer::Usage(estimate(NOW))]);

        assert_eq!(best.expect("a usage is known"), Some(estimate(NOW)));
    }

    #[test]
    fn fails_only_when_no_source_has_a_usage() {
        assert!(freshest(vec![Answer::Failure, Answer::Nothing]).is_err());
        assert_eq!(freshest(vec![Answer::Nothing]).expect("no failure"), None);
    }
}
