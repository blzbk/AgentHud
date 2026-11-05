//! JSON-RPC exchange with `codex app-server`, the official Codex client
//! (<https://learn.chatgpt.com/docs/app-server>): newline-delimited JSON on
//! stdio, JSON-RPC 2.0 without the `jsonrpc` field. One connection makes
//! one `initialize`, then the `initialized` notification, then
//! `account/rateLimits/read`. No other method is ever called.

use std::io::Write;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Instant;

use application::FetchError;
use domain::{EpochSeconds, Origin, Provider, UsageSnapshot, UsageWindow, WindowKind};
use serde_json::{json, Value};

use crate::token_count::kind_of;

const INITIALIZE_ID: i64 = 1;
const RATE_LIMITS_ID: i64 = 2;

/// Error code of a request the server rejects, such as one without login.
const INVALID_REQUEST: i64 = -32600;

/// Asks the server behind `writer` and `lines` for the rate limits, giving
/// up at `deadline`. `lines` carries the server's stdout, line by line.
pub(crate) fn read_rate_limits(
    writer: &mut impl Write,
    lines: &Receiver<String>,
    deadline: Instant,
    now: EpochSeconds,
) -> Result<UsageSnapshot, FetchError> {
    send(
        writer,
        &json!({
            "id": INITIALIZE_ID,
            "method": "initialize",
            "params": {
                "clientInfo": {
                    "name": "agenthud",
                    "title": "AgentHud",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            },
        }),
    )?;
    answer(lines, INITIALIZE_ID, deadline)?;
    send(writer, &json!({ "method": "initialized", "params": {} }))?;
    send(
        writer,
        &json!({
            "id": RATE_LIMITS_ID,
            "method": "account/rateLimits/read",
            "params": { "excludeResetCreditDetails": true },
        }),
    )?;
    let result = answer(lines, RATE_LIMITS_ID, deadline)?;
    parse(&result, now).ok_or_else(|| FetchError::Unavailable("no Codex rate limits".to_owned()))
}

fn send(writer: &mut impl Write, message: &Value) -> Result<(), FetchError> {
    writeln!(writer, "{message}")
        .and_then(|()| writer.flush())
        .map_err(|error| {
            FetchError::Unavailable(format!("cannot write to codex app-server: {error}"))
        })
}

/// Result of the response to request `id`. Notifications, server requests
/// and unreadable lines are skipped.
fn answer(lines: &Receiver<String>, id: i64, deadline: Instant) -> Result<Value, FetchError> {
    loop {
        let wait = deadline.saturating_duration_since(Instant::now());
        let line = match lines.recv_timeout(wait) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => {
                return Err(FetchError::Unavailable(
                    "codex app-server timed out".to_owned(),
                ))
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(FetchError::Unavailable(
                    "codex app-server stopped".to_owned(),
                ))
            }
        };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if message.get("method").is_some() || message.get("id").and_then(Value::as_i64) != Some(id)
        {
            continue;
        }
        if let Some(error) = message.get("error") {
            return Err(classify(error));
        }
        return Ok(message.get("result").cloned().unwrap_or(Value::Null));
    }
}

/// An error answer: a missing or refused login is an expired login.
fn classify(error: &Value) -> FetchError {
    let code = error.get("code").and_then(Value::as_i64);
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let lowered = message.to_lowercase();
    let auth = (code == Some(INVALID_REQUEST) && lowered.contains("authentication required"))
        || lowered.contains("401")
        || lowered.contains("unauthorized");
    if auth {
        FetchError::AuthExpired
    } else {
        let message: String = message.chars().take(120).collect();
        FetchError::Unavailable(format!(
            "codex app-server error {}: {message}",
            code.unwrap_or_default()
        ))
    }
}

/// Windows of `rateLimits` (`primary`, `secondary`), classified by length,
/// then by position, like the session events.
fn parse(result: &Value, now: EpochSeconds) -> Option<UsageSnapshot> {
    let limits = result.get("rateLimits")?;
    let mut five_hours = None;
    let mut weekly = None;
    for (field, position) in [
        ("primary", WindowKind::FiveHours),
        ("secondary", WindowKind::Weekly),
    ] {
        let Some(window) = limits.get(field).filter(|window| !window.is_null()) else {
            continue;
        };
        let kind = window
            .get("windowDurationMins")
            .and_then(Value::as_i64)
            .and_then(kind_of)
            .unwrap_or(position);
        let slot = match kind {
            WindowKind::FiveHours => &mut five_hours,
            WindowKind::Weekly => &mut weekly,
        };
        if slot.is_none() {
            *slot = window
                .get("usedPercent")
                .and_then(Value::as_f64)
                .and_then(|used| {
                    UsageWindow::new(kind, used, window.get("resetsAt").and_then(Value::as_i64))
                });
        }
    }
    if five_hours.is_none() && weekly.is_none() {
        return None;
    }
    Some(UsageSnapshot {
        provider: Provider::Codex,
        five_hours,
        weekly,
        observed_at: now,
        origin: Origin::AppServer,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{classify, parse};
    use application::FetchError;

    #[test]
    fn classifies_login_errors_as_expired() {
        let missing = json!({ "code": -32600, "message": "codex account authentication required to read rate limits" });
        let refused = json!({ "code": -32603, "message": "failed to fetch codex rate limits: 401 Unauthorized" });
        let other =
            json!({ "code": -32603, "message": "failed to fetch codex rate limits: timeout" });

        assert!(matches!(classify(&missing), FetchError::AuthExpired));
        assert!(matches!(classify(&refused), FetchError::AuthExpired));
        assert!(matches!(classify(&other), FetchError::Unavailable(_)));
    }

    #[test]
    fn a_result_without_window_has_no_usage() {
        for result in [
            json!(null),
            json!({}),
            json!({ "rateLimits": { "primary": null, "secondary": null } }),
            json!({ "rateLimits": { "primary": { "usedPercent": "high" } } }),
        ] {
            assert_eq!(parse(&result, 0), None, "{result}");
        }
    }
}
