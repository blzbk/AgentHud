//! Usage as sent to the UI: the wire shape of the `usage` command and event.
//! The TypeScript side mirrors it in `features/usage/useUsage.ts`.

use domain::{EpochSeconds, Provider, Usage, UsageWindow};
use serde::Serialize;

use crate::ProviderUsage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageStatus {
    Known,
    Unknown,
}

/// One quota window, known either in percent or in tokens. Times are seconds
/// since the Unix epoch. `estimated` is sent only when the window bounds are
/// guessed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<EpochSeconds>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub estimated: bool,
}

impl WindowReport {
    /// Report of `window` at `now`: a window already reset shows 0.
    #[must_use]
    pub fn new(window: &UsageWindow, now: EpochSeconds) -> Self {
        let (used_percent, used_tokens) = match window.used_at(now) {
            Usage::Percent(percent) => (Some(percent), None),
            Usage::Tokens(tokens) => (None, Some(tokens)),
        };
        Self {
            used_percent,
            used_tokens,
            resets_at: window.resets_at(),
            estimated: window.is_estimated(),
        }
    }
}

/// Usage of one provider. Windows are absent when unknown.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageReport {
    pub provider: &'static str,
    pub status: UsageStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub five_hours: Option<WindowReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weekly: Option<WindowReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<EpochSeconds>,
    /// Label of the `Origin` of the data, such as `statusline` or `estimate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<&'static str>,
    /// A polled source of the provider needs the user to log in again.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub auth_expired: bool,
}

impl UsageReport {
    /// Report of `usage` at `now`: a window already reset shows 0 %.
    #[must_use]
    pub fn new(usage: &ProviderUsage, now: EpochSeconds) -> Self {
        match usage {
            ProviderUsage::Known(snapshot) => {
                let window = |window: Option<UsageWindow>| {
                    window.map(|window| WindowReport::new(&window, now))
                };
                Self {
                    provider: snapshot.provider.label(),
                    status: UsageStatus::Known,
                    five_hours: window(snapshot.five_hours),
                    weekly: window(snapshot.weekly),
                    observed_at: Some(snapshot.observed_at),
                    source: Some(snapshot.origin.label()),
                    auth_expired: false,
                }
            }
            ProviderUsage::Unknown(provider) => Self::unknown(*provider),
        }
    }

    /// Report of a provider whose usage is not known (yet).
    #[must_use]
    pub const fn unknown(provider: Provider) -> Self {
        Self {
            provider: provider.label(),
            status: UsageStatus::Unknown,
            five_hours: None,
            weekly: None,
            observed_at: None,
            source: None,
            auth_expired: false,
        }
    }

    /// Same report, flagged when a polled source of the provider reports an
    /// expired login.
    #[must_use]
    pub const fn with_auth_expired(self, auth_expired: bool) -> Self {
        Self {
            auth_expired,
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use domain::{Origin, Provider, UsageSnapshot, UsageWindow, WindowKind};
    use serde_json::json;

    use super::UsageReport;
    use crate::ProviderUsage;

    const NOW: i64 = 1_793_840_000;

    fn known() -> ProviderUsage {
        ProviderUsage::Known(UsageSnapshot {
            provider: Provider::Codex,
            five_hours: UsageWindow::new(WindowKind::FiveHours, 12.0, Some(NOW - 1)),
            weekly: UsageWindow::new(WindowKind::Weekly, 51.0, Some(NOW + 3600)),
            observed_at: NOW - 180,
            origin: Origin::Sessions,
        })
    }

    #[test]
    fn serializes_a_known_usage_in_camel_case() {
        let report = serde_json::to_value(UsageReport::new(&known(), NOW + 3600 - 7200))
            .expect("report should serialize");

        assert_eq!(
            report,
            json!({
                "provider": "codex",
                "status": "known",
                "fiveHours": { "usedPercent": 12.0, "resetsAt": NOW - 1 },
                "weekly": { "usedPercent": 51.0, "resetsAt": NOW + 3600 },
                "observedAt": NOW - 180,
                "source": "sessions",
            })
        );
    }

    #[test]
    fn a_reset_window_reports_zero() {
        let report = UsageReport::new(&known(), NOW);

        assert_eq!(
            report.five_hours.and_then(|window| window.used_percent),
            Some(0.0)
        );
        assert_eq!(
            report.weekly.and_then(|window| window.used_percent),
            Some(51.0)
        );
    }

    #[test]
    fn serializes_an_unknown_usage_without_values() {
        let report = serde_json::to_value(UsageReport::new(
            &ProviderUsage::Unknown(Provider::Codex),
            NOW,
        ))
        .expect("report should serialize");

        assert_eq!(report, json!({ "provider": "codex", "status": "unknown" }));
    }

    #[test]
    fn a_window_without_reset_omits_it() {
        let usage = ProviderUsage::Known(UsageSnapshot {
            provider: Provider::Codex,
            five_hours: UsageWindow::new(WindowKind::FiveHours, 5.0, None),
            weekly: None,
            observed_at: NOW,
            origin: Origin::Sessions,
        });

        let report =
            serde_json::to_value(UsageReport::new(&usage, NOW)).expect("report should serialize");

        assert_eq!(
            report,
            json!({ "provider": "codex", "status": "known", "fiveHours": { "usedPercent": 5.0 }, "observedAt": NOW, "source": "sessions" })
        );
    }

    #[test]
    fn serializes_an_estimated_token_window_without_percent() {
        let usage = ProviderUsage::Known(UsageSnapshot {
            provider: Provider::Claude,
            five_hours: Some(
                UsageWindow::tokens(WindowKind::FiveHours, 1_200_000, Some(NOW + 600)).estimated(),
            ),
            weekly: None,
            observed_at: NOW - 10,
            origin: Origin::Transcripts,
        });

        let report =
            serde_json::to_value(UsageReport::new(&usage, NOW)).expect("report should serialize");

        assert_eq!(
            report,
            json!({
                "provider": "claude",
                "status": "known",
                "fiveHours": { "usedTokens": 1_200_000, "resetsAt": NOW + 600, "estimated": true },
                "observedAt": NOW - 10,
                "source": "estimate",
            })
        );
    }

    #[test]
    fn an_expired_token_window_reports_zero_tokens() {
        let usage = ProviderUsage::Known(UsageSnapshot {
            provider: Provider::Claude,
            five_hours: Some(
                UsageWindow::tokens(WindowKind::FiveHours, 1_200_000, Some(NOW)).estimated(),
            ),
            weekly: None,
            observed_at: NOW - 3600,
            origin: Origin::Sessions,
        });

        let report = UsageReport::new(&usage, NOW);

        assert_eq!(
            report.five_hours.and_then(|window| window.used_tokens),
            Some(0)
        );
    }

    #[test]
    fn flags_an_expired_login_only_when_set() {
        let flagged =
            serde_json::to_value(UsageReport::unknown(Provider::Claude).with_auth_expired(true))
                .expect("report should serialize");

        assert_eq!(
            flagged,
            json!({ "provider": "claude", "status": "unknown", "authExpired": true })
        );
    }
}
