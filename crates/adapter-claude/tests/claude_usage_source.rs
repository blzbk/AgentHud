//! `ClaudeUsageSource` on anonymized transcript fixtures.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use adapter_claude::ClaudeUsageSource;
use application::UsageSource;
use domain::{Provider, Usage, UsageSnapshot, WindowKind};
use tempfile::TempDir;

/// `2026-11-03T12:00:00Z`: the hour of the first fixture message.
const NOON: i64 = 1_793_707_200;

/// Tokens of `assistant-end-turn`: 1 115 + 2 270.
const END_TURN_TOKENS: u64 = 3_385;
/// Tokens of `assistant-tool-use`.
const TOOL_USE_TOKENS: u64 = 112;
/// Tokens of `duplicated-messages`, whose single message is written 3 times.
const DUPLICATED_TOKENS: u64 = 336;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Copies `fixture` to `projects/<project>/<session>.jsonl` under `root`.
fn add_transcript(root: &TempDir, project: &str, session: &str, fixture_name: &str) -> PathBuf {
    let transcript = root
        .path()
        .join("projects")
        .join(project)
        .join(format!("{session}.jsonl"));
    fs::create_dir_all(transcript.parent().expect("transcript has a folder"))
        .expect("folder should be writable");
    fs::copy(fixture(fixture_name), &transcript).expect("fixture should be copied");
    touch(&transcript, SystemTime::now() - Duration::from_secs(5));
    transcript
}

fn touch(path: &Path, modified: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(modified))
        .expect("mtime should be settable");
}

fn source(root: &TempDir) -> ClaudeUsageSource<impl Fn() -> Vec<PathBuf> + Send + Sync> {
    let roots = vec![root.path().to_path_buf()];
    ClaudeUsageSource::new(move || roots.clone())
}

fn tokens(snapshot: &UsageSnapshot) -> Option<u64> {
    match snapshot.five_hours?.used() {
        Usage::Tokens(tokens) => Some(tokens),
        Usage::Percent(_) => None,
    }
}

#[test]
fn sums_the_tokens_of_every_project_in_an_estimated_block() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_transcript(
        &root,
        "-workspace-project-a",
        "a",
        "assistant-end-turn.jsonl",
    );
    add_transcript(
        &root,
        "-workspace-project-b",
        "b",
        "assistant-tool-use.jsonl",
    );
    add_transcript(
        &root,
        "-workspace-project-b",
        "c",
        "duplicated-messages.jsonl",
    );

    let snapshot = source(&root)
        .latest()
        .expect("transcripts should be readable")
        .expect("usage should be known");

    assert_eq!(snapshot.provider, Provider::Claude);
    assert_eq!(
        tokens(&snapshot),
        Some(END_TURN_TOKENS + TOOL_USE_TOKENS + DUPLICATED_TOKENS)
    );
    let window = snapshot.five_hours.expect("five hours window");
    assert_eq!(window.kind(), WindowKind::FiveHours);
    assert!(window.is_estimated());
    assert_eq!(window.resets_at(), Some(NOON + 5 * 3600));
    assert_eq!(snapshot.weekly, None);
    // `2026-11-03T13:45:05Z`, the first copy of the last message.
    assert_eq!(snapshot.observed_at, NOON + 6305);
}

#[test]
fn counts_a_message_copied_into_another_session_once() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_transcript(
        &root,
        "-workspace-project-b",
        "original",
        "duplicated-messages.jsonl",
    );
    add_transcript(
        &root,
        "-workspace-project-b",
        "resumed",
        "duplicated-messages.jsonl",
    );

    let snapshot = source(&root)
        .latest()
        .expect("transcripts should be readable")
        .expect("usage should be known");

    assert_eq!(tokens(&snapshot), Some(DUPLICATED_TOKENS));
}

#[test]
fn transcripts_without_messages_have_no_usage() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_transcript(&root, "-workspace-project-a", "a", "metadata-only.jsonl");

    assert_eq!(source(&root).latest().expect("readable"), None);
}

#[test]
fn missing_roots_have_no_usage() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let absent = root.path().join(".claude");

    let usage = ClaudeUsageSource::new(move || vec![absent.clone()]).latest();

    assert!(matches!(usage, Ok(None)));
}

#[test]
fn reads_a_transcript_again_only_when_its_mtime_changes() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let transcript = add_transcript(
        &root,
        "-workspace-project-a",
        "a",
        "assistant-end-turn.jsonl",
    );
    let source = source(&root);

    source.latest().expect("readable");
    source.latest().expect("readable");
    assert_eq!(source.reads(), 1);

    let appended = fs::read_to_string(fixture("assistant-tool-use.jsonl"))
        .expect("fixture should be readable");
    File::options()
        .append(true)
        .open(&transcript)
        .and_then(|mut file| file.write_all(appended.as_bytes()))
        .expect("transcript should be writable");
    touch(&transcript, SystemTime::now());

    let snapshot = source
        .latest()
        .expect("readable")
        .expect("usage should be known");
    assert_eq!(source.reads(), 2);
    assert_eq!(tokens(&snapshot), Some(END_TURN_TOKENS + TOOL_USE_TOKENS));
}
