//! Usage as sent to the UI: the wire shape of the `usage` command and event.
//! The TypeScript side mirrors it in `features/usage/useUsage.ts`.

use domain::{EpochSeconds, Provider, UsageWindow};
use serde::Serialize;

use crate::ProviderUsage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UsageStatus {
    Known,
    Unknown,
}

/// One quota window. Times are seconds since the Unix epoch.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowReport {
    pub used_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<EpochSeconds>,
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
}

impl UsageReport {
    /// Report of `usage` at `now`: a window already reset shows 0 %.
    #[must_use]
    pub fn new(usage: &ProviderUsage, now: EpochSeconds) -> Self {
        match usage {
            ProviderUsage::Known(snapshot) => {
                let window = |window: Option<UsageWindow>| {
                    window.map(|window| WindowReport {
                        used_percent: window.used_at(now),
                        resets_at: window.resets_at(),
                    })
                };
                Self {
                    provider: snapshot.provider.label(),
                    status: UsageStatus::Known,
                    five_hours: window(snapshot.five_hours),
                    weekly: window(snapshot.weekly),
                    observed_at: Some(snapshot.observed_at),
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
        }
    }
}

#[cfg(test)]
mod tests {
    use domain::{Provider, UsageSnapshot, UsageWindow, WindowKind};
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
            })
        );
    }

    #[test]
    fn a_reset_window_reports_zero() {
        let report = UsageReport::new(&known(), NOW);

        assert_eq!(
            report.five_hours.map(|window| window.used_percent),
            Some(0.0)
        );
        assert_eq!(report.weekly.map(|window| window.used_percent), Some(51.0));
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
        });

        let report =
            serde_json::to_value(UsageReport::new(&usage, NOW)).expect("report should serialize");

        assert_eq!(
            report,
            json!({ "provider": "codex", "status": "known", "fiveHours": { "usedPercent": 5.0 }, "observedAt": NOW })
        );
    }
}
