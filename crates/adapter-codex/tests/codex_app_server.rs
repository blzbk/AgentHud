//! The `codex app-server` exchange against fake servers written in `sh`:
//! the real process plumbing runs, no Codex nor account is ever involved.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use adapter_codex::exchange;
use application::FetchError;
use domain::Origin;

const NOW: i64 = 1_793_840_000;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// A fake server: logs every line it gets to `log`, answers `initialize`,
/// sends a notification and a junk line, then answers with `answer`.
fn fake_server(log: &Path, answer: &Path) -> Command {
    let script = r#"
        read -r initialize; printf '%s\n' "$initialize" >> "$1"
        printf '%s\n' '{"id":1,"result":{"userAgent":"fake","codexHome":"/tmp","platformFamily":"unix","platformOs":"linux"}}'
        read -r initialized; printf '%s\n' "$initialized" >> "$1"
        read -r request; printf '%s\n' "$request" >> "$1"
        printf '%s\n' '{"method":"account/rateLimits/updated","params":{}}' 'not json' '{"id":7,"result":{}}'
        cat "$2"
        cat > /dev/null
    "#;
    let mut command = Command::new("sh");
    command
        .args(["-c", script, "fake-server"])
        .arg(log)
        .arg(answer);
    command
}

fn sent_lines(log: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(log)
        .expect("log should be readable")
        .lines()
        .map(|line| serde_json::from_str(line).expect("each request should be JSON"))
        .collect()
}

#[test]
fn reads_the_rate_limits_after_the_handshake() {
    let dir = tempfile::tempdir().expect("temporary folder should be writable");
    let log = dir.path().join("requests.jsonl");

    let snapshot = exchange(
        fake_server(&log, &fixture("app-server-rate-limits.json")),
        NOW,
    )
    .expect("rate limits should be read");

    assert_eq!(snapshot.origin, Origin::AppServer);
    assert_eq!(snapshot.observed_at, NOW);
    let five_hours = snapshot.five_hours.expect("five hours window");
    assert_eq!(five_hours.used_percent(), Some(12.0));
    assert_eq!(five_hours.resets_at(), Some(1_793_844_772));
    assert_eq!(
        snapshot.weekly.and_then(|window| window.used_percent()),
        Some(100.0)
    );

    let sent = sent_lines(&log);
    assert_eq!(sent.len(), 3);
    assert_eq!(sent[0]["method"], "initialize");
    assert_eq!(sent[0]["id"], 1);
    assert_eq!(sent[0]["params"]["clientInfo"]["name"], "agenthud");
    assert_eq!(sent[1]["method"], "initialized");
    assert!(sent[1].get("id").is_none());
    assert_eq!(sent[2]["method"], "account/rateLimits/read");
    assert_eq!(sent[2]["id"], 2);
    assert!(sent.iter().all(|message| message.get("jsonrpc").is_none()));
}

#[test]
fn a_missing_login_is_an_expired_login() {
    let dir = tempfile::tempdir().expect("temporary folder should be writable");
    let log = dir.path().join("requests.jsonl");

    let result = exchange(
        fake_server(&log, &fixture("app-server-auth-required.json")),
        NOW,
    );

    assert!(matches!(result, Err(FetchError::AuthExpired)));
}

#[test]
fn a_server_that_stops_early_is_unavailable() {
    let mut command = Command::new("sh");
    command.args(["-c", "read -r line"]);

    assert!(matches!(
        exchange(command, NOW),
        Err(FetchError::Unavailable(_))
    ));
}

#[test]
fn a_missing_codex_is_unavailable() {
    let command = Command::new("/nonexistent/codex");

    assert!(matches!(
        exchange(command, NOW),
        Err(FetchError::Unavailable(_))
    ));
}
