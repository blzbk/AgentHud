//! `ClaudeOAuthUsage` against a fake transport and fake credentials: no
//! request ever leaves the test, and no real login is read.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use adapter_claude::{ClaudeOAuthUsage, HttpReply, HttpTransport, USAGE_URL};
use application::{FetchError, UsageFetcher};
use domain::{Origin, UsageSnapshot};
use tempfile::TempDir;

const NOW: i64 = 1_793_840_000;
/// `2026-11-05T18:00:00Z`.
const FIVE_HOUR_RESET: i64 = 1_793_901_600;

const FAKE_TOKEN: &str = "fake-token-for-tests";

/// A request seen by the fake transport: its URL and headers.
type Request = (String, Vec<(String, String)>);

/// Answers one canned reply and records the requests it gets.
struct FakeTransport {
    reply: Result<HttpReply, String>,
    requests: Mutex<Vec<Request>>,
}

impl FakeTransport {
    fn replying(status: u16, body: &str, retry_after: Option<&str>) -> Self {
        Self {
            reply: Ok(HttpReply {
                status,
                retry_after: retry_after.map(str::to_owned),
                body: body.to_owned(),
            }),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl HttpTransport for &FakeTransport {
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpReply, String> {
        self.requests
            .lock()
            .expect("requests should be writable")
            .push((
                url.to_owned(),
                headers
                    .iter()
                    .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                    .collect(),
            ));
        self.reply.clone()
    }
}

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(path).expect("fixture should be readable")
}

/// A `.claude` folder holding a fake Claude Code login.
fn logged_in() -> TempDir {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    fs::write(
        root.path().join(".credentials.json"),
        format!(r#"{{"claudeAiOauth":{{"accessToken":"{FAKE_TOKEN}","expiresAt":1}}}}"#),
    )
    .expect("credentials should be writable");
    root
}

fn fetch(root: &TempDir, transport: &FakeTransport) -> Result<UsageSnapshot, FetchError> {
    let roots: Vec<PathBuf> = vec![root.path().to_path_buf()];
    ClaudeOAuthUsage::new(move || roots.clone(), transport).fetch(NOW)
}

fn percents(snapshot: &UsageSnapshot) -> (Option<f64>, Option<f64>) {
    (
        snapshot.five_hours.and_then(|window| window.used_percent()),
        snapshot.weekly.and_then(|window| window.used_percent()),
    )
}

#[test]
fn sends_the_login_and_the_beta_header_to_the_usage_endpoint_only() {
    let root = logged_in();
    let transport = FakeTransport::replying(200, &fixture("oauth-usage.json"), None);

    fetch(&root, &transport).expect("usage should be read");

    let requests = transport
        .requests
        .lock()
        .expect("requests should be readable");
    assert_eq!(requests.len(), 1);
    let (url, headers) = &requests[0];
    assert_eq!(url, USAGE_URL);
    assert_eq!(url, "https://api.anthropic.com/api/oauth/usage");
    assert!(headers.contains(&("authorization".to_owned(), format!("Bearer {FAKE_TOKEN}"))));
    assert!(headers.contains(&("anthropic-beta".to_owned(), "oauth-2025-04-20".to_owned())));
}

#[test]
fn reads_the_flat_windows_with_their_iso_resets() {
    let root = logged_in();
    let transport = FakeTransport::replying(200, &fixture("oauth-usage.json"), None);

    let snapshot = fetch(&root, &transport).expect("usage should be read");

    assert_eq!(snapshot.origin, Origin::OAuth);
    assert_eq!(snapshot.observed_at, NOW);
    assert_eq!(percents(&snapshot), (Some(23.5), Some(41.2)));
    assert_eq!(
        snapshot.five_hours.and_then(|window| window.resets_at()),
        Some(FIVE_HOUR_RESET)
    );
}

#[test]
fn falls_back_to_the_limits_list() {
    let root = logged_in();
    let transport = FakeTransport::replying(200, &fixture("oauth-usage-limits.json"), None);

    let snapshot = fetch(&root, &transport).expect("usage should be read");

    assert_eq!(percents(&snapshot), (Some(100.0), Some(41.2)));
}

#[test]
fn fills_a_null_window_from_the_list_and_keeps_an_unknown_reset() {
    let root = logged_in();
    let transport = FakeTransport::replying(200, &fixture("oauth-usage-nulls.json"), None);

    let snapshot = fetch(&root, &transport).expect("usage should be read");

    assert_eq!(percents(&snapshot), (Some(12.0), Some(64.0)));
    assert_eq!(snapshot.weekly.and_then(|window| window.resets_at()), None);
}

#[test]
fn a_refused_login_is_an_expired_login() {
    let root = logged_in();
    for status in [401, 403] {
        let transport = FakeTransport::replying(status, "{}", None);

        assert!(matches!(
            fetch(&root, &transport),
            Err(FetchError::AuthExpired)
        ));
    }
}

#[test]
fn a_rate_limit_carries_its_retry_delay() {
    let root = logged_in();
    let with_delay = FakeTransport::replying(429, "", Some("1800"));
    let without = FakeTransport::replying(429, "", None);

    assert!(matches!(
        fetch(&root, &with_delay),
        Err(FetchError::RateLimited {
            retry_after: Some(1800)
        })
    ));
    assert!(matches!(
        fetch(&root, &without),
        Err(FetchError::RateLimited { retry_after: None })
    ));
}

#[test]
fn other_failures_are_unavailable_without_the_token() {
    let root = logged_in();
    let failures = [
        FakeTransport::replying(500, "oops", None),
        FakeTransport::replying(200, "<html>", None),
        FakeTransport {
            reply: Err("connection refused".to_owned()),
            requests: Mutex::new(Vec::new()),
        },
    ];

    for transport in &failures {
        match fetch(&root, transport) {
            Err(error @ FetchError::Unavailable(_)) => {
                assert!(!error.to_string().contains(FAKE_TOKEN), "{error}");
            }
            other => panic!("expected unavailable, got {other:?}"),
        }
    }
}

#[test]
fn sends_nothing_without_a_login() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let transport = FakeTransport::replying(200, &fixture("oauth-usage.json"), None);

    assert!(matches!(
        fetch(&root, &transport),
        Err(FetchError::Unavailable(_))
    ));
    assert!(transport
        .requests
        .lock()
        .expect("requests should be readable")
        .is_empty());
}
