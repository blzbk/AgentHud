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

/// Writes the `running-session.json` fixture to `sessions/<pid>.json`, with
/// `status` and `cwd` replaced, beside a `.key` file that must stay unread.
fn add_marker(root: &TempDir, pid: u32, session: &str, status: &str, cwd: &str) {
    let dir = root.path().join("sessions");
    fs::create_dir_all(&dir).expect("folder should be writable");
    let fixture = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/running-session.json"),
    )
    .expect("fixture should be readable");
    let mut marker: serde_json::Value =
        serde_json::from_str(&fixture).expect("fixture should be JSON");
    marker["sessionId"] = session.into();
    marker["status"] = status.into();
    marker["cwd"] = cwd.into();
    fs::write(dir.join(format!("{pid}.json")), marker.to_string())
        .expect("marker should be writable");
    fs::write(
        dir.join(format!("{pid}.0123abcd.key")),
        "{\"sessionId\":\"key\"}",
    )
    .expect("key should be writable");
}

#[test]
fn only_the_running_sessions_are_agents() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let minute = Duration::from_mins(1);
    for session in ["running", "ended", "titled-only"] {
        add_transcript(
            &root,
            "-home-user-dev-project",
            session,
            "assistant-end-turn.jsonl",
            minute,
        );
    }
    add_marker(&root, 4242, "running", "busy", "/home/user/dev/project");

    let sessions = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "running");
    assert_eq!(sessions[0].project.as_deref(), Some("project"));
    assert_eq!(sessions[0].last_signal, TurnSignal::TurnStarted);
}

#[test]
fn an_idle_status_ends_the_turn_and_the_transcript_dates_it() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let transcript = add_transcript(
        &root,
        "-project",
        "running",
        "assistant-tool-use.jsonl",
        Duration::from_mins(6),
    );
    add_marker(&root, 4242, "running", "idle", r"C:\Users\user\project");

    let sessions = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].last_signal, TurnSignal::TurnEnded);
    assert_eq!(sessions[0].project.as_deref(), Some("project"));
    let written = fs::metadata(&transcript)
        .and_then(|metadata| metadata.modified())
        .expect("mtime should be readable");
    assert_eq!(sessions[0].last_write, adapter_fs::epoch_seconds(written));
}

#[test]
fn a_running_session_without_transcript_is_dated_by_its_marker() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_marker(&root, 4242, "new", "idle", "/home/user/dev/project");

    let sessions = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "new");
    assert_eq!(sessions[0].last_signal, TurnSignal::TurnEnded);
    let marked = fs::metadata(root.path().join("sessions/4242.json"))
        .and_then(|metadata| metadata.modified())
        .expect("mtime should be readable");
    assert_eq!(sessions[0].last_write, adapter_fs::epoch_seconds(marked));
}

#[test]
fn unreadable_markers_fall_back_on_the_transcripts() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    for session in ["a", "b"] {
        add_transcript(
            &root,
            "-workspace-project-a",
            session,
            "assistant-end-turn.jsonl",
            Duration::from_mins(1),
        );
    }
    let markers = root.path().join("sessions");
    fs::create_dir_all(&markers).expect("folder should be writable");
    fs::write(markers.join("1.json"), "{\"sessionId\":").expect("marker should be writable");
    fs::write(markers.join("2.json"), "{\"status\":\"busy\"}").expect("marker should be writable");

    let sessions = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(sessions.len(), 2);
}

#[test]
fn an_empty_marker_folder_means_no_running_session() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_transcript(
        &root,
        "-workspace-project-a",
        "ended",
        "assistant-end-turn.jsonl",
        Duration::from_mins(1),
    );
    fs::create_dir_all(root.path().join("sessions")).expect("folder should be writable");

    let sessions = source(&root)
        .sessions()
        .expect("sessions should be readable");

    assert_eq!(sessions.len(), 0);
}
