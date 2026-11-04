//! `CodexUsageSource` on anonymized session fixtures, one per Codex schema
//! met so far.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use adapter_codex::CodexUsageSource;
use application::UsageSource;
use domain::{Provider, UsageSnapshot, WindowKind};
use tempfile::TempDir;

/// `2026-11-03T12:00:04Z`, the last `token_count` of `current-resets-at`.
const CURRENT_OBSERVED_AT: i64 = 1_793_707_204;

/// `2026-10-01T08:00:02Z`, the `token_count` of `legacy-resets-in-seconds`.
const LEGACY_OBSERVED_AT: i64 = 1_790_841_602;

/// A `.codex` folder holding `fixture` as its only session.
fn codex_root(fixture: &str, age: Duration) -> TempDir {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let session = root.path().join("sessions/2026/11/03/rollout.jsonl");
    fs::create_dir_all(session.parent().expect("session has a folder"))
        .expect("folder should be writable");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    fs::copy(&fixture, &session).expect("fixture should be copied");
    File::options()
        .write(true)
        .open(&session)
        .and_then(|file| file.set_modified(SystemTime::now() - age))
        .expect("mtime should be settable");
    root
}

fn latest(roots: &[&TempDir]) -> Option<UsageSnapshot> {
    let roots: Vec<PathBuf> = roots.iter().map(|root| root.path().to_path_buf()).collect();
    CodexUsageSource::new(move || roots.clone())
        .latest()
        .expect("fixtures should be readable")
}

fn percent(window: Option<domain::UsageWindow>) -> f64 {
    window.expect("window should be known").used_percent()
}

#[test]
fn reads_the_current_schema_with_reset_times() {
    let root = codex_root("current-resets-at.jsonl", Duration::ZERO);

    let snapshot = latest(&[&root]).expect("usage should be known");

    assert_eq!(snapshot.provider, Provider::Codex);
    assert_eq!(snapshot.observed_at, CURRENT_OBSERVED_AT);
    let five_hours = snapshot.five_hours.expect("five hours window");
    assert_eq!(five_hours.kind(), WindowKind::FiveHours);
    assert!((five_hours.used_percent() - 1.0).abs() < f64::EPSILON);
    assert_eq!(five_hours.resets_at(), Some(1_793_844_772));
    let weekly = snapshot.weekly.expect("weekly window");
    assert_eq!(weekly.kind(), WindowKind::Weekly);
    assert!((weekly.used_percent() - 51.0).abs() < f64::EPSILON);
    assert_eq!(weekly.resets_at(), Some(1_794_184_565));
}

#[test]
fn dates_legacy_resets_from_the_event_time() {
    let root = codex_root("legacy-resets-in-seconds.jsonl", Duration::ZERO);

    let snapshot = latest(&[&root]).expect("usage should be known");

    assert_eq!(snapshot.observed_at, LEGACY_OBSERVED_AT);
    let five_hours = snapshot.five_hours.expect("five hours window");
    assert!((five_hours.used_percent() - 12.0).abs() < f64::EPSILON);
    assert_eq!(five_hours.resets_at(), Some(LEGACY_OBSERVED_AT + 3600));
    let weekly = snapshot.weekly.expect("weekly window");
    assert!((weekly.used_percent() - 30.0).abs() < f64::EPSILON);
    assert_eq!(weekly.resets_at(), Some(LEGACY_OBSERVED_AT + 86_400));
}

#[test]
fn falls_back_to_the_previous_usage_when_the_last_is_null() {
    let root = codex_root("null-rate-limits.jsonl", Duration::ZERO);

    let snapshot = latest(&[&root]).expect("usage should be known");

    assert!((percent(snapshot.five_hours) - 7.0).abs() < f64::EPSILON);
    assert!((percent(snapshot.weekly) - 40.0).abs() < f64::EPSILON);
}

#[test]
fn a_session_without_token_count_has_no_usage() {
    let root = codex_root("no-token-count.jsonl", Duration::ZERO);

    assert_eq!(latest(&[&root]), None);
}

#[test]
fn missing_roots_have_no_usage() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let absent = root.path().join(".codex");

    let usage = CodexUsageSource::new(move || vec![absent.clone()]).latest();

    assert!(matches!(usage, Ok(None)));
}

#[test]
fn keeps_the_most_recent_usage_across_roots() {
    // The Windows root holds the older session, the WSL root the newer one,
    // even though the Windows file was touched last.
    let windows = codex_root("legacy-resets-in-seconds.jsonl", Duration::ZERO);
    let wsl = codex_root("current-resets-at.jsonl", Duration::from_secs(3600));

    let snapshot = latest(&[&windows, &wsl]).expect("usage should be known");

    assert_eq!(snapshot.observed_at, CURRENT_OBSERVED_AT);
    assert!((percent(snapshot.weekly) - 51.0).abs() < f64::EPSILON);
}
