//! When to ask a remote usage source again (`codex app-server`, the Claude
//! OAuth endpoint). Pure: callers pass `now` and what they observed.
//!
//! Rules, each tested below:
//! 1. never before the policy floor after the previous poll;
//! 2. quota used up: long period, and a poll right after the reset;
//! 3. quota used up and an agent becomes active: a poll at the floor;
//! 4. otherwise faster while agents are active, slower when all are quiet;
//! 5. (switching polling off is the caller's `enabled` flag);
//! 6. expired login: the long auth period.

use domain::{AgentState, EpochSeconds, Exhaustion};

use crate::ProviderAgents;

/// Delay after a reset before polling, so the provider has moved on.
const RESET_GRACE: EpochSeconds = 30;

/// Delays of one remote source, in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollPolicy {
    /// Shortest delay between two polls, whatever happens.
    pub floor: EpochSeconds,
    /// Period while an agent is active.
    pub busy: EpochSeconds,
    /// Period while every agent is idle or offline.
    pub quiet: EpochSeconds,
    /// Period while the quota is used up.
    pub exhausted: EpochSeconds,
    /// Delay after a failed poll, unless the source asks for longer.
    pub error_backoff: EpochSeconds,
    /// Delay after an expired login.
    pub auth_backoff: EpochSeconds,
}

impl PollPolicy {
    /// `codex app-server`: official client, at most once a minute.
    pub const CODEX_APP_SERVER: Self = Self {
        floor: 60,
        busy: 2 * 60,
        quiet: 10 * 60,
        exhausted: 10 * 60,
        error_backoff: 5 * 60,
        auth_backoff: 30 * 60,
    };

    /// Claude OAuth endpoint, the degraded mode: at most every 13 minutes
    /// (ADR 0002), longer on errors and expired logins.
    pub const CLAUDE_OAUTH: Self = Self {
        floor: 13 * 60,
        busy: 13 * 60,
        quiet: 30 * 60,
        exhausted: 30 * 60,
        error_backoff: 30 * 60,
        auth_backoff: 60 * 60,
    };
}

/// Whether the user's agents are at work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Activity {
    /// An agent is working or waiting for the user.
    Busy,
    /// Every agent is idle or offline.
    #[default]
    Quiet,
}

impl Activity {
    /// `Busy` when any session is `working` or `waiting` at `now`.
    #[must_use]
    pub fn of(agents: &[ProviderAgents], now: EpochSeconds) -> Self {
        let busy = agents.iter().any(|agents| match agents {
            ProviderAgents::Active(_, sessions) => sessions.iter().any(|session| {
                matches!(
                    session.state_at(now),
                    AgentState::Working | AgentState::Waiting
                )
            }),
            ProviderAgents::Offline(_) | ProviderAgents::Unknown(_) => false,
        });
        if busy {
            Self::Busy
        } else {
            Self::Quiet
        }
    }
}

/// What a poll gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollOutcome {
    /// New usage, and whether its quota is used up.
    Fresh(Exhaustion),
    /// No usage; `retry_after` is the delay the source asked for, if any.
    Failed { retry_after: Option<EpochSeconds> },
    /// The login of the source has expired.
    AuthExpired,
}

/// Decides when the next poll of one source is due.
#[derive(Debug, Clone)]
pub struct PollScheduler {
    policy: PollPolicy,
    last_poll: Option<(EpochSeconds, PollOutcome)>,
    activity: Activity,
    /// An agent became active since the last poll.
    woke: bool,
}

impl PollScheduler {
    #[must_use]
    pub const fn new(policy: PollPolicy) -> Self {
        Self {
            policy,
            last_poll: None,
            activity: Activity::Quiet,
            woke: false,
        }
    }

    /// Time of the next poll, `None` when no poll happened yet (due now).
    #[must_use]
    pub fn next_poll(&self) -> Option<EpochSeconds> {
        let (last, outcome) = self.last_poll?;
        let policy = &self.policy;
        let delay = match outcome {
            PollOutcome::Fresh(Exhaustion::Available) => match self.activity {
                Activity::Busy => policy.busy,
                Activity::Quiet => policy.quiet,
            },
            PollOutcome::Fresh(Exhaustion::Until(_)) if self.woke => policy.floor,
            PollOutcome::Fresh(Exhaustion::Until(resets_at)) => resets_at
                .map_or(policy.exhausted, |resets_at| {
                    policy.exhausted.min(resets_at + RESET_GRACE - last)
                }),
            PollOutcome::Failed { retry_after } => {
                policy.error_backoff.max(retry_after.unwrap_or(0))
            }
            PollOutcome::AuthExpired => policy.auth_backoff,
        };
        Some(last.saturating_add(delay.max(policy.floor)))
    }

    /// Whether a poll is due at `now`.
    #[must_use]
    pub fn is_due(&self, now: EpochSeconds) -> bool {
        self.next_poll().is_none_or(|next| now >= next)
    }

    /// Records a poll made at `now`.
    pub fn record(&mut self, now: EpochSeconds, outcome: PollOutcome) {
        self.last_poll = Some((now, outcome));
        self.woke = false;
    }

    /// Records the current activity of the agents.
    pub fn observe_activity(&mut self, activity: Activity) {
        if self.activity == Activity::Quiet && activity == Activity::Busy {
            self.woke = true;
        }
        self.activity = activity;
    }

    /// Whether the last poll found an expired login.
    #[must_use]
    pub fn auth_expired(&self) -> bool {
        matches!(self.last_poll, Some((_, PollOutcome::AuthExpired)))
    }
}

#[cfg(test)]
mod tests {
    use domain::{AgentSession, Exhaustion, Provider, TurnSignal};

    use super::{Activity, PollOutcome, PollPolicy, PollScheduler};
    use crate::ProviderAgents;

    const NOW: i64 = 1_793_840_000;
    const CODEX: PollPolicy = PollPolicy::CODEX_APP_SERVER;
    const OAUTH: PollPolicy = PollPolicy::CLAUDE_OAUTH;

    // Shape of the policies the rules rely on.
    const _: () = assert!(CODEX.busy < CODEX.quiet && OAUTH.busy < OAUTH.quiet);

    fn polled(policy: PollPolicy, activity: Activity, outcome: PollOutcome) -> PollScheduler {
        let mut scheduler = PollScheduler::new(policy);
        scheduler.observe_activity(activity);
        scheduler.record(NOW, outcome);
        scheduler
    }

    fn fresh() -> PollOutcome {
        PollOutcome::Fresh(Exhaustion::Available)
    }

    fn exhausted_until(resets_at: Option<i64>) -> PollOutcome {
        PollOutcome::Fresh(Exhaustion::Until(resets_at))
    }

    #[test]
    fn the_first_poll_is_due_at_once() {
        assert!(PollScheduler::new(CODEX).is_due(NOW));
    }

    #[test]
    fn rule1_never_polls_before_the_floor() {
        let short_retry = polled(
            CODEX,
            Activity::Busy,
            PollOutcome::Failed {
                retry_after: Some(1),
            },
        );
        let reset_soon = polled(CODEX, Activity::Busy, exhausted_until(Some(NOW + 1)));
        let mut woken = polled(CODEX, Activity::Quiet, exhausted_until(None));
        woken.observe_activity(Activity::Busy);

        for scheduler in [short_retry, reset_soon, woken] {
            assert!(!scheduler.is_due(NOW + CODEX.floor - 1));
            assert!(scheduler.next_poll() >= Some(NOW + CODEX.floor));
        }
    }

    #[test]
    fn rule1_the_oauth_floor_is_thirteen_minutes() {
        let scheduler = polled(OAUTH, Activity::Busy, fresh());

        assert_eq!(OAUTH.floor, 13 * 60);
        assert!(!scheduler.is_due(NOW + 13 * 60 - 1));
        assert!(scheduler.is_due(NOW + 13 * 60));
    }

    #[test]
    fn rule2_an_exhausted_quota_polls_right_after_its_reset() {
        let scheduler = polled(CODEX, Activity::Quiet, exhausted_until(Some(NOW + 180)));

        assert_eq!(scheduler.next_poll(), Some(NOW + 180 + 30));
    }

    #[test]
    fn rule2_an_exhausted_quota_far_from_its_reset_polls_on_the_long_period() {
        let far = polled(CODEX, Activity::Busy, exhausted_until(Some(NOW + 86_400)));
        let unknown = polled(CODEX, Activity::Busy, exhausted_until(None));

        assert_eq!(far.next_poll(), Some(NOW + CODEX.exhausted));
        assert_eq!(unknown.next_poll(), Some(NOW + CODEX.exhausted));
        assert!((5 * 60..=10 * 60).contains(&CODEX.exhausted));
    }

    #[test]
    fn rule3_an_agent_waking_up_on_an_exhausted_quota_polls_at_the_floor() {
        let mut scheduler = polled(CODEX, Activity::Quiet, exhausted_until(None));

        scheduler.observe_activity(Activity::Busy);

        assert_eq!(scheduler.next_poll(), Some(NOW + CODEX.floor));
    }

    #[test]
    fn rule3_an_agent_already_active_does_not_shorten_the_exhausted_period() {
        let mut scheduler = polled(CODEX, Activity::Busy, exhausted_until(None));

        scheduler.observe_activity(Activity::Busy);

        assert_eq!(scheduler.next_poll(), Some(NOW + CODEX.exhausted));
    }

    #[test]
    fn rule3_a_new_poll_forgets_the_wake_up() {
        let mut scheduler = polled(CODEX, Activity::Quiet, exhausted_until(None));
        scheduler.observe_activity(Activity::Busy);

        scheduler.record(NOW + 60, exhausted_until(None));

        assert_eq!(scheduler.next_poll(), Some(NOW + 60 + CODEX.exhausted));
    }

    #[test]
    fn rule4_polls_faster_while_agents_are_active() {
        let busy = polled(CODEX, Activity::Busy, fresh());
        let quiet = polled(CODEX, Activity::Quiet, fresh());

        assert_eq!(busy.next_poll(), Some(NOW + CODEX.busy));
        assert_eq!(quiet.next_poll(), Some(NOW + CODEX.quiet));
    }

    #[test]
    fn rule4_follows_the_activity_after_the_poll() {
        let mut scheduler = polled(CODEX, Activity::Busy, fresh());

        scheduler.observe_activity(Activity::Quiet);

        assert_eq!(scheduler.next_poll(), Some(NOW + CODEX.quiet));
    }

    #[test]
    fn rule6_an_expired_login_polls_less_often() {
        let codex = polled(CODEX, Activity::Busy, PollOutcome::AuthExpired);
        let oauth = polled(OAUTH, Activity::Busy, PollOutcome::AuthExpired);

        assert!(codex.auth_expired());
        assert_eq!(codex.next_poll(), Some(NOW + 30 * 60));
        assert_eq!(oauth.next_poll(), Some(NOW + 60 * 60));
    }

    #[test]
    fn a_fresh_poll_clears_the_expired_login() {
        let mut scheduler = polled(CODEX, Activity::Busy, PollOutcome::AuthExpired);

        scheduler.record(NOW + 1800, fresh());

        assert!(!scheduler.auth_expired());
    }

    #[test]
    fn a_failure_backs_off_and_honours_a_longer_retry_after() {
        let plain = polled(
            OAUTH,
            Activity::Busy,
            PollOutcome::Failed { retry_after: None },
        );
        let long = polled(
            OAUTH,
            Activity::Busy,
            PollOutcome::Failed {
                retry_after: Some(7200),
            },
        );

        assert_eq!(plain.next_poll(), Some(NOW + OAUTH.error_backoff));
        assert_eq!(long.next_poll(), Some(NOW + 7200));
    }

    fn session(signal: TurnSignal, age: i64) -> AgentSession {
        AgentSession {
            provider: Provider::Codex,
            session_id: "s".to_owned(),
            project: None,
            last_signal: signal,
            last_write: NOW - age,
        }
    }

    #[test]
    fn working_or_waiting_agents_are_busy() {
        let working = [ProviderAgents::Active(
            Provider::Codex,
            vec![session(TurnSignal::TurnStarted, 10)],
        )];
        let waiting = [
            ProviderAgents::Offline(Provider::Codex),
            ProviderAgents::Active(Provider::Claude, vec![session(TurnSignal::TurnEnded, 10)]),
        ];

        assert_eq!(Activity::of(&working, NOW), Activity::Busy);
        assert_eq!(Activity::of(&waiting, NOW), Activity::Busy);
    }

    #[test]
    fn idle_offline_or_unknown_agents_are_quiet() {
        let agents = [
            ProviderAgents::Active(Provider::Codex, vec![session(TurnSignal::TurnStarted, 600)]),
            ProviderAgents::Offline(Provider::Claude),
            ProviderAgents::Unknown(Provider::Claude),
        ];

        assert_eq!(Activity::of(&agents, NOW), Activity::Quiet);
        assert_eq!(Activity::of(&[], NOW), Activity::Quiet);
    }
}
