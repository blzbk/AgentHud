//! Degraded mode (ADR 0002): the Claude quotas from the OAuth usage endpoint
//! Claude Code itself uses. Anthropic does not allow this for third-party
//! tools: it only runs when the user enables it, on the schedule of
//! `PollPolicy::CLAUDE_OAUTH`, and only while the statusline is stale.
//!
//! The token is read again from the Claude Code login for each request,
//! sent only to `api.anthropic.com`, never stored nor logged.

use std::path::PathBuf;
use std::time::Duration;

use application::{FetchError, UsageFetcher};
use domain::{EpochSeconds, Origin, Provider, UsageSnapshot, UsageWindow, WindowKind};
use serde_json::Value;

use crate::credentials;
use crate::statusline_source::epoch;

/// The only network address the app ever contacts.
pub const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

/// Beta header the endpoint requires.
const BETA_HEADER: (&str, &str) = ("anthropic-beta", "oauth-2025-04-20");

/// Largest answer read; a usage answer is a few hundred bytes.
const MAX_BODY_BYTES: u64 = 64 * 1024;

/// One HTTP answer, as much as the fetcher needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpReply {
    pub status: u16,
    /// Raw `Retry-After` header, if any.
    pub retry_after: Option<String>,
    pub body: String,
}

/// Sends one GET request. Errors are plain messages without the headers.
pub trait HttpTransport: Send + Sync {
    /// # Errors
    ///
    /// Fails when no answer arrives: network, TLS, timeout, oversized body.
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpReply, String>;
}

/// Blocking HTTPS client: rustls, 3 s to connect, 8 s in all, no redirect,
/// no retry, at most [`MAX_BODY_BYTES`] read.
pub struct UreqTransport {
    agent: ureq::Agent,
}

impl Default for UreqTransport {
    fn default() -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(3)))
            .timeout_global(Some(Duration::from_secs(8)))
            .max_redirects(0)
            .https_only(true)
            .http_status_as_error(false)
            .build()
            .new_agent();
        Self { agent }
    }
}

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpReply, String> {
        let mut request = self.agent.get(url);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let mut response = request.call().map_err(|error| error.to_string())?;
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_BODY_BYTES)
            .read_to_string()
            .map_err(|error| error.to_string())?;
        Ok(HttpReply {
            status: response.status().as_u16(),
            retry_after,
            body,
        })
    }
}

/// Claude quotas from the OAuth usage endpoint, with the login of the first
/// `.claude` root that has one.
pub struct ClaudeOAuthUsage<F, T> {
    roots: F,
    transport: T,
}

impl<F, T> ClaudeOAuthUsage<F, T>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
    T: HttpTransport,
{
    /// `roots` lists the `.claude` folders to look for a login in.
    pub fn new(roots: F, transport: T) -> Self {
        Self { roots, transport }
    }
}

impl<F, T> UsageFetcher for ClaudeOAuthUsage<F, T>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
    T: HttpTransport,
{
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn fetch(&self, now: EpochSeconds) -> Result<UsageSnapshot, FetchError> {
        let token = credentials::access_token(&(self.roots)())
            .ok_or_else(|| FetchError::Unavailable("no Claude Code login found".to_owned()))?;
        let authorization = format!("Bearer {}", token.secret());
        let reply = self
            .transport
            .get(USAGE_URL, &[("authorization", &authorization), BETA_HEADER])
            .map_err(FetchError::Unavailable)?;
        match reply.status {
            200 => parse(&reply.body, now)
                .ok_or_else(|| FetchError::Unavailable("unreadable usage answer".to_owned())),
            401 | 403 => Err(FetchError::AuthExpired),
            429 => Err(FetchError::RateLimited {
                retry_after: reply
                    .retry_after
                    .as_deref()
                    .and_then(|value| retry_after_seconds(value, now)),
            }),
            status => Err(FetchError::Unavailable(format!("HTTP {status}"))),
        }
    }
}

/// Quotas of a usage answer observed at `now`, or `None` without any usable
/// window. Flat `five_hour` / `seven_day` fields win, window by window, over
/// the `limits[]` list some versions send.
fn parse(body: &str, now: EpochSeconds) -> Option<UsageSnapshot> {
    let answer: Value = serde_json::from_str(body).ok()?;
    let listed = |kind: &str| {
        answer
            .get("limits")?
            .as_array()?
            .iter()
            .find(|limit| limit.get("kind").and_then(Value::as_str) == Some(kind))
    };
    let window = |flat: &str, listed_kind: &str, field: &str, kind: WindowKind| {
        answer
            .get(flat)
            .and_then(|window| to_window(window, "utilization", kind))
            .or_else(|| listed(listed_kind).and_then(|window| to_window(window, field, kind)))
    };
    let five_hours = window("five_hour", "session", "percent", WindowKind::FiveHours);
    let weekly = window("seven_day", "weekly_all", "percent", WindowKind::Weekly);
    if five_hours.is_none() && weekly.is_none() {
        return None;
    }
    Some(UsageSnapshot {
        provider: Provider::Claude,
        five_hours,
        weekly,
        observed_at: now,
        origin: Origin::OAuth,
    })
}

fn to_window(window: &Value, used_field: &str, kind: WindowKind) -> Option<UsageWindow> {
    let used = window.get(used_field)?.as_f64()?;
    let resets_at = window.get("resets_at").and_then(epoch);
    UsageWindow::new(kind, used, resets_at)
}

/// Delay of a `Retry-After` header: seconds, or an HTTP date such as
/// `Sun, 06 Nov 1994 08:49:37 GMT`, counted from `now`.
fn retry_after_seconds(value: &str, now: EpochSeconds) -> Option<EpochSeconds> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let value = value.trim();
    if let Ok(seconds) = value.parse::<EpochSeconds>() {
        return Some(seconds.max(0));
    }
    let [_, day, month, year, time, "GMT"] = value.split_whitespace().collect::<Vec<_>>()[..]
    else {
        return None;
    };
    let month = MONTHS.iter().position(|name| *name == month)? + 1;
    let at = adapter_fs::parse_utc_timestamp(&format!("{year}-{month:02}-{day:0>2}T{time}Z"))?;
    Some((at - now).max(0))
}

#[cfg(test)]
mod tests {
    use super::{parse, retry_after_seconds};

    #[test]
    fn reads_both_forms_of_retry_after() {
        assert_eq!(retry_after_seconds("120", 0), Some(120));
        assert_eq!(
            retry_after_seconds("Thu, 01 Jan 1970 00:10:00 GMT", 60),
            Some(540)
        );
        assert_eq!(
            retry_after_seconds("Thu, 01 Jan 1970 00:00:00 GMT", 60),
            Some(0)
        );
        assert_eq!(retry_after_seconds("later", 0), None);
    }

    #[test]
    fn an_answer_without_window_has_no_usage() {
        for body in [
            "",
            "<html>",
            "{}",
            r#"{"five_hour":null,"seven_day":null}"#,
            r#"{"five_hour":{"utilization":null},"limits":[]}"#,
        ] {
            assert_eq!(parse(body, 0), None, "{body}");
        }
    }
}
