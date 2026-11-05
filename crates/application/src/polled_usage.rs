//! Use case: poll a remote usage source on its schedule and keep its last
//! answer, so the file polling never waits on the network or a process.

use std::sync::{Mutex, PoisonError};

use domain::{EpochSeconds, Provider, UsageSnapshot};

use crate::poll_schedule::{Activity, PollOutcome, PollPolicy, PollScheduler};
use crate::ports::{FetchError, SourceError, UsageFetcher, UsageSource};

/// A statusline usage written within this delay is fresh: the degraded
/// OAuth source is not needed.
pub const STATUSLINE_FRESH_FOR: EpochSeconds = 15 * 60;

/// Whether the statusline usage is absent, or older than
/// [`STATUSLINE_FRESH_FOR`] at `now`: only then may the degraded source poll.
#[must_use]
pub fn statusline_is_stale(statusline: Option<&UsageSnapshot>, now: EpochSeconds) -> bool {
    statusline.is_none_or(|snapshot| now - snapshot.observed_at >= STATUSLINE_FRESH_FOR)
}

/// A remote source polled on its own schedule. Switched off, it neither
/// polls nor provides any usage.
pub struct PolledUsage {
    fetcher: Box<dyn UsageFetcher>,
    state: Mutex<State>,
}

struct State {
    enabled: bool,
    scheduler: PollScheduler,
    last: Option<UsageSnapshot>,
}

impl PolledUsage {
    #[must_use]
    pub fn new(fetcher: Box<dyn UsageFetcher>, policy: PollPolicy, enabled: bool) -> Self {
        Self {
            fetcher,
            state: Mutex::new(State {
                enabled,
                scheduler: PollScheduler::new(policy),
                last: None,
            }),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Switches the source on or off. Switched off, its last usage is
    /// dropped so the passive sources show again.
    pub fn set_enabled(&self, enabled: bool) {
        let mut state = self.state();
        state.enabled = enabled;
        if !enabled {
            state.last = None;
        }
    }

    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.state().enabled
    }

    /// Resumes after a poll made at `at` by a previous launch.
    pub fn resume(&self, at: EpochSeconds) {
        self.state().scheduler.resume(at);
    }

    /// Time of the last poll, `None` before the first one.
    #[must_use]
    pub fn last_poll_at(&self) -> Option<EpochSeconds> {
        self.state().scheduler.last_poll_at()
    }

    /// Records the current activity of the agents.
    pub fn observe_activity(&self, activity: Activity) {
        self.state().scheduler.observe_activity(activity);
    }

    /// Whether the source is on and its last poll found an expired login.
    #[must_use]
    pub fn auth_expired(&self) -> bool {
        let state = self.state();
        state.enabled && state.scheduler.auth_expired()
    }

    /// Polls the source when it is on and a poll is due at `now`. Returns
    /// whether a poll was made. The fetch runs without holding the state,
    /// so readers of the last usage never wait on it.
    pub fn poll_if_due(&self, now: EpochSeconds) -> bool {
        {
            let state = self.state();
            if !state.enabled || !state.scheduler.is_due(now) {
                return false;
            }
        }
        let fetched = self.fetcher.fetch(now);
        let mut state = self.state();
        let outcome = match fetched {
            Ok(snapshot) => {
                let outcome = PollOutcome::Fresh(snapshot.exhaustion(now));
                if state.enabled {
                    state.last = Some(snapshot);
                }
                outcome
            }
            Err(error) => {
                log::warn!(
                    "{} usage poll failed: {error}",
                    self.fetcher.provider().label()
                );
                match error {
                    FetchError::AuthExpired => PollOutcome::AuthExpired,
                    FetchError::RateLimited { retry_after } => PollOutcome::Failed { retry_after },
                    FetchError::Unavailable(_) => PollOutcome::Failed { retry_after: None },
                }
            }
        };
        state.scheduler.record(now, outcome);
        true
    }
}

impl UsageSource for PolledUsage {
    fn provider(&self) -> Provider {
        self.fetcher.provider()
    }

    /// Last usage polled, kept on failures, `None` while switched off.
    fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError> {
        let state = self.state();
        Ok(state.last.clone().filter(|_| state.enabled))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use domain::{Origin, Provider, UsageSnapshot, UsageWindow, WindowKind};

    use super::{statusline_is_stale, PolledUsage, STATUSLINE_FRESH_FOR};
    use crate::poll_schedule::{Activity, PollPolicy};
    use crate::ports::{FetchError, UsageFetcher, UsageSource};

    const NOW: i64 = 1_793_840_000;
    const POLICY: PollPolicy = PollPolicy::CODEX_APP_SERVER;

    fn snapshot(used: f64) -> UsageSnapshot {
        UsageSnapshot {
            provider: Provider::Codex,
            five_hours: UsageWindow::new(WindowKind::FiveHours, used, Some(NOW + 3600)),
            weekly: None,
            observed_at: NOW,
            origin: Origin::AppServer,
        }
    }

    /// Answers the queued results in order, counting its calls.
    struct FakeFetcher {
        answers: Mutex<Vec<Result<UsageSnapshot, FetchError>>>,
        calls: Arc<AtomicUsize>,
    }

    impl UsageFetcher for FakeFetcher {
        fn provider(&self) -> Provider {
            Provider::Codex
        }

        fn fetch(&self, _now: i64) -> Result<UsageSnapshot, FetchError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let mut answers = self.answers.lock().expect("answers should be readable");
            if answers.is_empty() {
                Err(FetchError::Unavailable("no answer".to_owned()))
            } else {
                answers.remove(0)
            }
        }
    }

    fn polled(
        answers: Vec<Result<UsageSnapshot, FetchError>>,
        enabled: bool,
    ) -> (PolledUsage, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let fetcher = FakeFetcher {
            answers: Mutex::new(answers),
            calls: Arc::clone(&calls),
        };
        (PolledUsage::new(Box::new(fetcher), POLICY, enabled), calls)
    }

    fn latest(source: &PolledUsage) -> Option<UsageSnapshot> {
        source.latest().expect("a polled source never fails")
    }

    #[test]
    fn polls_once_then_waits_for_the_schedule() {
        let (source, calls) = polled(vec![Ok(snapshot(10.0)), Ok(snapshot(20.0))], true);
        source.observe_activity(Activity::Busy);

        assert!(source.poll_if_due(NOW));
        assert!(!source.poll_if_due(NOW + POLICY.busy - 1));
        assert_eq!(latest(&source), Some(snapshot(10.0)));
        assert!(source.poll_if_due(NOW + POLICY.busy));
        assert_eq!(latest(&source), Some(snapshot(20.0)));
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn rule5_a_switched_off_source_neither_polls_nor_provides_usage() {
        let (source, calls) = polled(vec![Ok(snapshot(10.0))], false);

        assert!(!source.poll_if_due(NOW));
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        source.set_enabled(true);
        assert!(source.poll_if_due(NOW));
        source.set_enabled(false);

        assert_eq!(latest(&source), None);
        assert!(!source.is_enabled());
    }

    #[test]
    fn keeps_the_last_usage_when_a_poll_fails() {
        let (source, _) = polled(
            vec![
                Ok(snapshot(10.0)),
                Err(FetchError::Unavailable("down".to_owned())),
            ],
            true,
        );

        source.poll_if_due(NOW);
        source.poll_if_due(NOW + POLICY.quiet);

        assert_eq!(latest(&source), Some(snapshot(10.0)));
    }

    #[test]
    fn reports_an_expired_login_while_switched_on() {
        let (source, _) = polled(vec![Err(FetchError::AuthExpired)], true);

        source.poll_if_due(NOW);

        assert!(source.auth_expired());
        assert!(!source.poll_if_due(NOW + 29 * 60));
        source.set_enabled(false);
        assert!(!source.auth_expired());
    }

    #[test]
    fn honours_the_retry_delay_of_a_rate_limit() {
        let (source, calls) = polled(
            vec![Err(FetchError::RateLimited {
                retry_after: Some(3600),
            })],
            true,
        );

        source.poll_if_due(NOW);

        assert!(!source.poll_if_due(NOW + 3599));
        assert!(source.poll_if_due(NOW + 3600));
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn an_exhausted_quota_polls_again_after_its_reset() {
        let (source, _) = polled(vec![Ok(snapshot(100.0))], true);

        source.poll_if_due(NOW);

        assert!(!source.poll_if_due(NOW + POLICY.exhausted - 1));
        assert!(source.poll_if_due(NOW + POLICY.exhausted));
    }

    #[test]
    fn the_statusline_is_stale_when_absent_or_old() {
        let mut official = snapshot(10.0);

        assert!(statusline_is_stale(None, NOW));
        assert!(!statusline_is_stale(
            Some(&official),
            NOW + STATUSLINE_FRESH_FOR - 1
        ));
        official.observed_at = NOW - STATUSLINE_FRESH_FOR;
        assert!(statusline_is_stale(Some(&official), NOW));
    }
}
