//! Quota usage of a provider over its rolling windows.
//!
//! Times are seconds since the Unix epoch, so the model needs no clock or
//! date crate: callers pass `now`.

/// Seconds since the Unix epoch.
pub type EpochSeconds = i64;

/// A coding agent vendor whose quota usage is tracked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Provider {
    Codex,
    Claude,
}

impl Provider {
    /// Wire name shared with the UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

/// Length of a rolling quota window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    FiveHours,
    Weekly,
}

/// How much of a window is used: a share of the quota when the provider
/// publishes it, or a raw token count when only the tokens are known.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Usage {
    Percent(f64),
    Tokens(u64),
}

/// Usage of a quota window, and when the window resets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsageWindow {
    kind: WindowKind,
    used: Usage,
    resets_at: Option<EpochSeconds>,
    estimated: bool,
}

impl UsageWindow {
    /// Builds a window known in percent, clamping `used_percent` to 0..=100.
    ///
    /// Returns `None` when `used_percent` is not a finite number: an unknown
    /// usage must never be shown as 0 %.
    #[must_use]
    pub fn new(
        kind: WindowKind,
        used_percent: f64,
        resets_at: Option<EpochSeconds>,
    ) -> Option<Self> {
        used_percent.is_finite().then(|| Self {
            kind,
            used: Usage::Percent(used_percent.clamp(0.0, 100.0)),
            resets_at,
            estimated: false,
        })
    }

    /// Builds a window known only by the tokens used in it.
    #[must_use]
    pub const fn tokens(
        kind: WindowKind,
        used_tokens: u64,
        resets_at: Option<EpochSeconds>,
    ) -> Self {
        Self {
            kind,
            used: Usage::Tokens(used_tokens),
            resets_at,
            estimated: false,
        }
    }

    /// Marks the window as estimated: its bounds are guessed, not published
    /// by the provider.
    #[must_use]
    pub const fn estimated(self) -> Self {
        Self {
            estimated: true,
            ..self
        }
    }

    #[must_use]
    pub const fn kind(&self) -> WindowKind {
        self.kind
    }

    /// Usage as last observed, before any reset.
    #[must_use]
    pub const fn used(&self) -> Usage {
        self.used
    }

    /// Share used as last observed, `None` for a window known in tokens.
    #[must_use]
    pub const fn used_percent(&self) -> Option<f64> {
        match self.used {
            Usage::Percent(percent) => Some(percent),
            Usage::Tokens(_) => None,
        }
    }

    #[must_use]
    pub const fn resets_at(&self) -> Option<EpochSeconds> {
        self.resets_at
    }

    #[must_use]
    pub const fn is_estimated(&self) -> bool {
        self.estimated
    }

    /// Usage at `now`: 0 once the window has reset, the observed value before.
    #[must_use]
    pub fn used_at(&self, now: EpochSeconds) -> Usage {
        match (self.resets_at, self.used) {
            (Some(resets_at), Usage::Percent(_)) if resets_at <= now => Usage::Percent(0.0),
            (Some(resets_at), Usage::Tokens(_)) if resets_at <= now => Usage::Tokens(0),
            (_, used) => used,
        }
    }
}

/// Where a usage snapshot comes from, shown next to the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Codex session files (`token_count` events).
    Sessions,
    /// `codex app-server`, the official Codex client.
    AppServer,
    /// The Claude Code statusline script, the official Claude source.
    Statusline,
    /// The Claude OAuth usage endpoint, the degraded mode.
    OAuth,
    /// Tokens summed from the Claude transcripts, an estimate.
    Transcripts,
}

impl Origin {
    /// Wire name shared with the UI.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sessions => "sessions",
            Self::AppServer => "app-server",
            Self::Statusline => "statusline",
            Self::OAuth => "oauth",
            Self::Transcripts => "estimate",
        }
    }
}

/// Whether a quota is used up, and until when.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exhaustion {
    /// Every known window still has room.
    Available,
    /// A window is at 100 %: it resets at the given time, when known.
    Until(Option<EpochSeconds>),
}

/// Usage of one provider, as observed at a given time.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageSnapshot {
    pub provider: Provider,
    pub five_hours: Option<UsageWindow>,
    pub weekly: Option<UsageWindow>,
    pub observed_at: EpochSeconds,
    pub origin: Origin,
}

impl UsageSnapshot {
    /// Whether a window known in percent is used up at `now`. With several
    /// used-up windows, the quota is back only once the last one resets.
    #[must_use]
    pub fn exhaustion(&self, now: EpochSeconds) -> Exhaustion {
        let mut exhausted = None;
        for window in [self.five_hours, self.weekly].into_iter().flatten() {
            let used_up =
                matches!(window.used_at(now), Usage::Percent(percent) if percent >= 100.0);
            if !used_up {
                continue;
            }
            exhausted = Some(match (exhausted, window.resets_at()) {
                (None, resets_at) => resets_at,
                (Some(Some(previous)), Some(resets_at)) => Some(resets_at.max(previous)),
                (Some(_), _) => None,
            });
        }
        exhausted.map_or(Exhaustion::Available, Exhaustion::Until)
    }

    /// Whether the usage is a published share, not an estimate in tokens.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        [self.five_hours, self.weekly]
            .into_iter()
            .flatten()
            .any(|window| !window.is_estimated() && window.used_percent().is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::{Exhaustion, Origin, Provider, Usage, UsageSnapshot, UsageWindow, WindowKind};

    const NOW: i64 = 1_793_840_000;

    fn window(used_percent: f64, resets_at: Option<i64>) -> UsageWindow {
        UsageWindow::new(WindowKind::FiveHours, used_percent, resets_at)
            .expect("usage should be a finite number")
    }

    #[test]
    fn keeps_the_observed_usage_before_the_reset() {
        assert_eq!(
            window(40.0, Some(NOW + 60)).used_at(NOW),
            Usage::Percent(40.0)
        );
    }

    #[test]
    fn keeps_the_observed_usage_without_a_known_reset() {
        assert_eq!(window(40.0, None).used_at(NOW), Usage::Percent(40.0));
    }

    #[test]
    fn drops_to_zero_once_the_reset_has_passed() {
        assert_eq!(
            window(40.0, Some(NOW - 1)).used_at(NOW),
            Usage::Percent(0.0)
        );
        assert_eq!(window(40.0, Some(NOW)).used_at(NOW), Usage::Percent(0.0));
    }

    #[test]
    fn clamps_the_usage_to_zero_to_one_hundred() {
        assert_eq!(window(140.0, None).used_percent(), Some(100.0));
        assert_eq!(window(-5.0, None).used_percent(), Some(0.0));
    }

    #[test]
    fn rejects_a_usage_that_is_not_a_number() {
        assert!(UsageWindow::new(WindowKind::Weekly, f64::NAN, None).is_none());
        assert!(UsageWindow::new(WindowKind::Weekly, f64::INFINITY, None).is_none());
    }

    #[test]
    fn a_percent_window_is_published_not_estimated() {
        assert!(!window(40.0, None).is_estimated());
    }

    #[test]
    fn a_token_window_keeps_its_tokens_until_the_reset() {
        let window = UsageWindow::tokens(WindowKind::FiveHours, 1_200_000, Some(NOW + 60));

        assert_eq!(window.used(), Usage::Tokens(1_200_000));
        assert_eq!(window.used_percent(), None);
        assert_eq!(window.used_at(NOW), Usage::Tokens(1_200_000));
        assert_eq!(window.used_at(NOW + 60), Usage::Tokens(0));
    }

    #[test]
    fn an_estimated_window_keeps_its_usage() {
        let window = UsageWindow::tokens(WindowKind::FiveHours, 10, None).estimated();

        assert!(window.is_estimated());
        assert_eq!(window.used(), Usage::Tokens(10));
    }

    #[test]
    fn providers_have_distinct_wire_names() {
        assert_eq!(
            [Provider::Codex, Provider::Claude].map(Provider::label),
            ["codex", "claude"]
        );
    }

    fn snapshot(five_hours: Option<UsageWindow>, weekly: Option<UsageWindow>) -> UsageSnapshot {
        UsageSnapshot {
            provider: Provider::Codex,
            five_hours,
            weekly,
            observed_at: NOW,
            origin: Origin::AppServer,
        }
    }

    #[test]
    fn origins_have_distinct_wire_names() {
        assert_eq!(
            [
                Origin::Sessions,
                Origin::AppServer,
                Origin::Statusline,
                Origin::OAuth,
                Origin::Transcripts
            ]
            .map(Origin::label),
            ["sessions", "app-server", "statusline", "oauth", "estimate"]
        );
    }

    #[test]
    fn a_quota_with_room_is_available() {
        let usage = snapshot(Some(window(99.5, Some(NOW + 60))), None);

        assert_eq!(usage.exhaustion(NOW), Exhaustion::Available);
    }

    #[test]
    fn a_full_window_is_exhausted_until_its_reset() {
        let usage = snapshot(Some(window(100.0, Some(NOW + 60))), None);

        assert_eq!(usage.exhaustion(NOW), Exhaustion::Until(Some(NOW + 60)));
        assert_eq!(usage.exhaustion(NOW + 60), Exhaustion::Available);
    }

    #[test]
    fn the_latest_reset_of_full_windows_ends_the_exhaustion() {
        let weekly = UsageWindow::new(WindowKind::Weekly, 100.0, Some(NOW + 3600));
        let usage = snapshot(Some(window(100.0, Some(NOW + 60))), weekly);

        assert_eq!(usage.exhaustion(NOW), Exhaustion::Until(Some(NOW + 3600)));
    }

    #[test]
    fn a_full_window_without_reset_is_exhausted_for_an_unknown_time() {
        let usage = snapshot(Some(window(100.0, None)), None);

        assert_eq!(usage.exhaustion(NOW), Exhaustion::Until(None));
    }

    #[test]
    fn a_token_window_is_never_exhausted_nor_exact() {
        let tokens = UsageWindow::tokens(WindowKind::FiveHours, 10, None).estimated();
        let usage = snapshot(Some(tokens), None);

        assert_eq!(usage.exhaustion(NOW), Exhaustion::Available);
        assert!(!usage.is_exact());
        assert!(snapshot(Some(window(1.0, None)), None).is_exact());
    }
}
