//! Defensive parsing of the `event_msg` / `token_count` lines Codex writes in
//! its session files. Every field is optional: the schema changes between
//! Codex versions, and an unexpected line is skipped rather than failing.

use domain::{EpochSeconds, Origin, Provider, UsageSnapshot, UsageWindow, WindowKind};
use serde::Deserialize;

const FIVE_HOURS_MINUTES: i64 = 300;
const WEEK_MINUTES: i64 = 7 * 24 * 60;

#[derive(Deserialize)]
struct Line {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    payload: Option<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    rate_limits: Option<RateLimits>,
}

#[derive(Deserialize)]
struct RateLimits {
    #[serde(default)]
    primary: Option<Limit>,
    #[serde(default)]
    secondary: Option<Limit>,
}

#[derive(Deserialize)]
struct Limit {
    #[serde(default)]
    used_percent: Option<f64>,
    #[serde(default)]
    window_minutes: Option<i64>,
    #[serde(default)]
    resets_at: Option<i64>,
    #[serde(default)]
    resets_in_seconds: Option<i64>,
}

/// Usage carried by one session line, or `None` when the line is not a
/// `token_count` event with at least one usable window.
///
/// `fallback_time` dates the event when its own timestamp is missing.
#[must_use]
pub fn parse_line(line: &str, fallback_time: EpochSeconds) -> Option<UsageSnapshot> {
    let line: Line = serde_json::from_str(line).ok()?;
    if line.kind.as_deref() != Some("event_msg") {
        return None;
    }
    let payload = line.payload?;
    if payload.kind.as_deref() != Some("token_count") {
        return None;
    }
    let limits = payload.rate_limits?;
    let observed_at = line
        .timestamp
        .as_deref()
        .and_then(adapter_fs::parse_utc_timestamp)
        .unwrap_or(fallback_time);

    let mut five_hours = None;
    let mut weekly = None;
    let positioned = [
        (limits.primary, WindowKind::FiveHours),
        (limits.secondary, WindowKind::Weekly),
    ];
    for (window, position) in positioned {
        let Some(window) = window else { continue };
        let kind = window.window_minutes.and_then(kind_of).unwrap_or(position);
        let slot = match kind {
            WindowKind::FiveHours => &mut five_hours,
            WindowKind::Weekly => &mut weekly,
        };
        if slot.is_none() {
            *slot = to_usage_window(&window, kind, observed_at);
        }
    }
    if five_hours.is_none() && weekly.is_none() {
        return None;
    }
    Some(UsageSnapshot {
        provider: Provider::Codex,
        five_hours,
        weekly,
        observed_at,
        origin: Origin::Sessions,
    })
}

/// Window kind from its length, with a 10 % tolerance.
fn kind_of(minutes: i64) -> Option<WindowKind> {
    let near = |target: i64| (minutes - target).abs() * 10 <= target;
    if near(FIVE_HOURS_MINUTES) {
        Some(WindowKind::FiveHours)
    } else if near(WEEK_MINUTES) {
        Some(WindowKind::Weekly)
    } else {
        None
    }
}

fn to_usage_window(
    window: &Limit,
    kind: WindowKind,
    observed_at: EpochSeconds,
) -> Option<UsageWindow> {
    let resets_at = window.resets_at.or_else(|| {
        window
            .resets_in_seconds
            .map(|seconds| observed_at + seconds)
    });
    UsageWindow::new(kind, window.used_percent?, resets_at)
}

#[cfg(test)]
mod tests {
    use domain::WindowKind;

    use super::parse_line;

    const FALLBACK: i64 = 42;

    fn token_count(rate_limits: &str) -> String {
        format!(
            r#"{{"timestamp":"2026-11-03T12:00:00.000Z","type":"event_msg","payload":{{"type":"token_count","info":null,"rate_limits":{rate_limits}}}}}"#
        )
    }

    #[test]
    fn classifies_windows_by_length_before_position() {
        let line = token_count(
            r#"{"primary":{"used_percent":51.0,"window_minutes":10080},"secondary":{"used_percent":1.0,"window_minutes":300}}"#,
        );

        let snapshot = parse_line(&line, FALLBACK).expect("line should carry usage");

        let five_hours = snapshot.five_hours.expect("five hours window");
        assert_eq!(five_hours.kind(), WindowKind::FiveHours);
        assert_eq!(five_hours.used_percent(), Some(1.0));
        assert_eq!(
            snapshot.weekly.expect("weekly window").used_percent(),
            Some(51.0)
        );
    }

    #[test]
    fn classifies_windows_of_unknown_length_by_position() {
        let line = token_count(
            r#"{"primary":{"used_percent":3.0},"secondary":{"used_percent":4.0,"window_minutes":42}}"#,
        );

        let snapshot = parse_line(&line, FALLBACK).expect("line should carry usage");

        assert_eq!(
            snapshot.five_hours.expect("five hours").used_percent(),
            Some(3.0)
        );
        assert_eq!(snapshot.weekly.expect("weekly").used_percent(), Some(4.0));
    }

    #[test]
    fn dates_the_reset_from_the_event_when_only_a_delay_is_given() {
        let line = token_count(
            r#"{"primary":{"used_percent":3.0,"window_minutes":300,"resets_in_seconds":600}}"#,
        );

        let snapshot = parse_line(&line, FALLBACK).expect("line should carry usage");

        assert_eq!(snapshot.observed_at, 1_793_707_200);
        assert_eq!(
            snapshot.five_hours.and_then(|window| window.resets_at()),
            Some(1_793_707_800)
        );
        assert_eq!(snapshot.weekly, None);
    }

    #[test]
    fn uses_the_fallback_time_without_event_timestamp() {
        let line = r#"{"type":"event_msg","payload":{"type":"token_count","rate_limits":{"primary":{"used_percent":3.0}}}}"#;

        assert_eq!(
            parse_line(line, FALLBACK).map(|snapshot| snapshot.observed_at),
            Some(FALLBACK)
        );
    }

    #[test]
    fn skips_lines_without_usable_usage() {
        let lines = [
            "not json".to_owned(),
            "{".to_owned(),
            "[]".to_owned(),
            r#"{"type":"response_item","payload":{"type":"message"}}"#.to_owned(),
            r#"{"type":"event_msg","payload":{"type":"agent_message"}}"#.to_owned(),
            token_count("null"),
            token_count("{}"),
            token_count(r#"{"primary":null,"secondary":{"window_minutes":10080}}"#),
            token_count(r#"{"primary":{"used_percent":"high"}}"#),
        ];

        for line in &lines {
            assert_eq!(parse_line(line, FALLBACK), None, "{line}");
        }
    }
}
