//! `CodexAgentSessions` on anonymized session fixtures.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use adapter_codex::CodexAgentSessions;
use application::AgentStateSource;
use domain::{AgentSession, Provider, TurnSignal};
use tempfile::TempDir;

/// Copies `fixture` to `sessions/<name>` under `root`, written `age` ago.
fn add_session(root: &TempDir, name: &str, fixture: &str, age: Duration) -> PathBuf {
    let session = root.path().join("sessions/2026/11/03").join(name);
    fs::create_dir_all(session.parent().expect("session has a folder"))
        .expect("folder should be writable");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    fs::copy(&fixture, &session).expect("fixture should be copied");
    touch(&session, SystemTime::now() - age);
    session
}

fn touch(path: &Path, modified: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(modified))
        .expect("mtime should be settable");
}

fn source(root: &TempDir) -> CodexAgentSessions<impl Fn() -> Vec<PathBuf> + Send + Sync> {
    let roots = vec![root.path().to_path_buf()];
    CodexAgentSessions::new(move || roots.clone())
}

fn signal_of(fixture: &str) -> TurnSignal {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_session(&root, "rollout.jsonl", fixture, Duration::ZERO);

    let sessions = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(sessions.len(), 1);
    sessions[0].last_signal
}

#[test]
fn a_running_task_is_a_started_turn() {
    assert_eq!(signal_of("turn-running.jsonl"), TurnSignal::TurnStarted);
}

#[test]
fn a_complete_task_is_an_ended_turn() {
    assert_eq!(signal_of("turn-complete.jsonl"), TurnSignal::TurnEnded);
}

#[test]
fn an_aborted_turn_is_an_ended_turn() {
    assert_eq!(signal_of("turn-aborted.jsonl"), TurnSignal::TurnEnded);
}

#[test]
fn a_session_without_turn_event_is_unknown() {
    assert_eq!(signal_of("no-token-count.jsonl"), TurnSignal::Unknown);
}

#[test]
fn lists_the_active_sessions_only() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_session(
        &root,
        "rollout-a.jsonl",
        "turn-running.jsonl",
        Duration::from_secs(10),
    );
    add_session(
        &root,
        "rollout-b.jsonl",
        "turn-complete.jsonl",
        Duration::from_secs(120),
    );
    add_session(
        &root,
        "rollout-old.jsonl",
        "turn-complete.jsonl",
        Duration::from_mins(61),
    );

    let sessions: Vec<AgentSession> = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(
        sessions
            .iter()
            .map(|session| (session.session_id.as_str(), session.last_signal))
            .collect::<Vec<_>>(),
        [
            ("rollout-a", TurnSignal::TurnStarted),
            ("rollout-b", TurnSignal::TurnEnded)
        ]
    );
    assert!(sessions
        .iter()
        .all(|session| session.provider == Provider::Codex
            && session.project.as_deref() == Some("project")));
}

#[test]
fn reads_a_session_again_only_when_its_mtime_changes() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let session = add_session(
        &root,
        "rollout.jsonl",
        "turn-running.jsonl",
        Duration::from_secs(60),
    );
    let source = source(&root);

    source.sessions().expect("sessions should be readable");
    source.sessions().expect("sessions should be readable");
    assert_eq!(source.reads(), 1);

    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/turn-complete.jsonl"),
        &session,
    )
    .expect("fixture should be copied");
    touch(&session, SystemTime::now());

    let sessions = source.sessions().expect("sessions should be readable");
    assert_eq!(source.reads(), 2);
    assert_eq!(sessions[0].last_signal, TurnSignal::TurnEnded);
}

#[test]
fn missing_roots_have_no_session() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let absent = root.path().join(".codex");

    let sessions = CodexAgentSessions::new(move || vec![absent.clone()]).sessions();

    assert!(sessions.is_ok_and(|sessions| sessions.is_empty()));
}
