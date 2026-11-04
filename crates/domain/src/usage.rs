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

/// Share of a quota window already used, and when the window resets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsageWindow {
    kind: WindowKind,
    used_percent: f64,
    resets_at: Option<EpochSeconds>,
}

impl UsageWindow {
    /// Builds a window, clamping `used_percent` to 0..=100.
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
            used_percent: used_percent.clamp(0.0, 100.0),
            resets_at,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> WindowKind {
        self.kind
    }

    /// Usage as last observed, before any reset.
    #[must_use]
    pub const fn used_percent(&self) -> f64 {
        self.used_percent
    }

    #[must_use]
    pub const fn resets_at(&self) -> Option<EpochSeconds> {
        self.resets_at
    }

    /// Usage at `now`: 0 once the window has reset, the observed value before.
    #[must_use]
    pub fn used_at(&self, now: EpochSeconds) -> f64 {
        match self.resets_at {
            Some(resets_at) if resets_at <= now => 0.0,
            _ => self.used_percent,
        }
    }
}

/// Usage of one provider, as observed at a given time.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageSnapshot {
    pub provider: Provider,
    pub five_hours: Option<UsageWindow>,
    pub weekly: Option<UsageWindow>,
    pub observed_at: EpochSeconds,
}

#[cfg(test)]
mod tests {
    use super::{Provider, UsageWindow, WindowKind};

    const NOW: i64 = 1_793_840_000;

    fn window(used_percent: f64, resets_at: Option<i64>) -> UsageWindow {
        UsageWindow::new(WindowKind::FiveHours, used_percent, resets_at)
            .unwrap_or_else(|| panic!("{used_percent} should be a valid usage"))
    }

    #[test]
    fn keeps_the_observed_usage_before_the_reset() {
        assert!((window(40.0, Some(NOW + 60)).used_at(NOW) - 40.0).abs() < f64::EPSILON);
    }

    #[test]
    fn keeps_the_observed_usage_without_a_known_reset() {
        assert!((window(40.0, None).used_at(NOW) - 40.0).abs() < f64::EPSILON);
    }

    #[test]
    fn drops_to_zero_once_the_reset_has_passed() {
        assert!(window(40.0, Some(NOW - 1)).used_at(NOW).abs() < f64::EPSILON);
        assert!(window(40.0, Some(NOW)).used_at(NOW).abs() < f64::EPSILON);
    }

    #[test]
    fn clamps_the_usage_to_zero_to_one_hundred() {
        assert!((window(140.0, None).used_percent() - 100.0).abs() < f64::EPSILON);
        assert!(window(-5.0, None).used_percent().abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_a_usage_that_is_not_a_number() {
        assert!(UsageWindow::new(WindowKind::Weekly, f64::NAN, None).is_none());
        assert!(UsageWindow::new(WindowKind::Weekly, f64::INFINITY, None).is_none());
    }

    #[test]
    fn providers_have_distinct_wire_names() {
        assert_eq!(
            [Provider::Codex, Provider::Claude].map(Provider::label),
            ["codex", "claude"]
        );
    }
}
