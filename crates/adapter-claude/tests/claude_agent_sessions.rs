//! `ClaudeAgentSessions` on anonymized transcript fixtures.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use adapter_claude::ClaudeAgentSessions;
use application::AgentStateSource;
use domain::{AgentSession, Provider, TurnSignal};
use tempfile::TempDir;

/// Copies `fixture` to `projects/<project>/<session>.jsonl`, written `age` ago.
fn add_transcript(
    root: &TempDir,
    project: &str,
    session: &str,
    fixture: &str,
    age: Duration,
) -> PathBuf {
    let transcript = root
        .path()
        .join("projects")
        .join(project)
        .join(format!("{session}.jsonl"));
    fs::create_dir_all(transcript.parent().expect("transcript has a folder"))
        .expect("folder should be writable");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture);
    fs::copy(&fixture, &transcript).expect("fixture should be copied");
    touch(&transcript, SystemTime::now() - age);
    transcript
}

fn touch(path: &Path, modified: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(modified))
        .expect("mtime should be settable");
}

fn source(root: &TempDir) -> ClaudeAgentSessions<impl Fn() -> Vec<PathBuf> + Send + Sync> {
    let roots = vec![root.path().to_path_buf()];
    ClaudeAgentSessions::new(move || roots.clone())
}

fn only_session(fixture: &str) -> AgentSession {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_transcript(
        &root,
        "-workspace-project-a",
        "session",
        fixture,
        Duration::ZERO,
    );

    let mut sessions = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(sessions.len(), 1);
    sessions.remove(0)
}

#[test]
fn an_end_turn_before_metadata_ends_the_turn() {
    let session = only_session("assistant-end-turn.jsonl");

    assert_eq!(session.last_signal, TurnSignal::TurnEnded);
    assert_eq!(session.provider, Provider::Claude);
    assert_eq!(session.session_id, "session");
    assert_eq!(session.project.as_deref(), Some("project-a"));
}

#[test]
fn a_tool_call_keeps_the_turn_going() {
    assert_eq!(
        only_session("assistant-tool-use.jsonl").last_signal,
        TurnSignal::TurnStarted
    );
}

#[test]
fn a_tool_result_from_the_user_keeps_the_turn_going() {
    assert_eq!(
        only_session("duplicated-messages.jsonl").last_signal,
        TurnSignal::TurnStarted
    );
}

#[test]
fn metadata_only_is_unknown_with_the_raw_project_folder() {
    let session = only_session("metadata-only.jsonl");

    assert_eq!(session.last_signal, TurnSignal::Unknown);
    assert_eq!(session.project.as_deref(), Some("-workspace-project-a"));
}

#[test]
fn lists_the_active_sessions_but_not_subagents() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let minute = Duration::from_mins(1);
    add_transcript(
        &root,
        "-workspace-project-a",
        "a",
        "assistant-end-turn.jsonl",
        minute,
    );
    add_transcript(
        &root,
        "-workspace-project-b",
        "b",
        "assistant-tool-use.jsonl",
        minute,
    );
    add_transcript(
        &root,
        "-workspace-project-b",
        "old",
        "assistant-tool-use.jsonl",
        Duration::from_mins(61),
    );
    add_transcript(
        &root,
        "-workspace-project-b/b/subagents",
        "agent",
        "assistant-tool-use.jsonl",
        minute,
    );

    let mut ids: Vec<String> = source(&root)
        .sessions()
        .expect("sessions should be readable")
        .into_iter()
        .map(|session| session.session_id)
        .collect();
    ids.sort();

    assert_eq!(ids, ["a", "b"]);
}

#[test]
fn reads_a_transcript_again_only_when_its_mtime_changes() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let transcript = add_transcript(
        &root,
        "-workspace-project-a",
        "a",
        "assistant-tool-use.jsonl",
        Duration::from_mins(1),
    );
    let source = source(&root);

    source.sessions().expect("readable");
    source.sessions().expect("readable");
    assert_eq!(source.reads(), 1);

    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assistant-end-turn.jsonl"),
        &transcript,
    )
    .expect("fixture should be copied");
    touch(&transcript, SystemTime::now());

    let sessions = source.sessions().expect("readable");
    assert_eq!(source.reads(), 2);
    assert_eq!(sessions[0].last_signal, TurnSignal::TurnEnded);
}

#[test]
fn a_running_subagent_keeps_its_session_active() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_transcript(
        &root,
        "-workspace-project-a",
        "a",
        "assistant-tool-use.jsonl",
        Duration::from_mins(10),
    );
    let subagent = add_transcript(
        &root,
        "-workspace-project-a/a/subagents",
        "agent",
        "assistant-end-turn.jsonl",
        Duration::ZERO,
    );

    let sessions = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(sessions.len(), 1);
    let written = fs::metadata(&subagent)
        .and_then(|metadata| metadata.modified())
        .expect("mtime should be readable");
    assert_eq!(sessions[0].last_write, adapter_fs::epoch_seconds(written));
    assert_eq!(sessions[0].last_signal, TurnSignal::TurnStarted);
}
