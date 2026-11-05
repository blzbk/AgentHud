//! The `AgentHud` statusline script and `ClaudeStatuslineSource`, end to end
//! on anonymized fixtures.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use adapter_claude::{ClaudeStatuslineSource, RATE_LIMITS_FILE};
use application::UsageSource;
use domain::{Origin, Provider, UsageSnapshot};
use tempfile::TempDir;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn source(root: &TempDir) -> ClaudeStatuslineSource<impl Fn() -> Vec<PathBuf> + Send + Sync> {
    let roots = vec![root.path().to_path_buf()];
    ClaudeStatuslineSource::new(move || roots.clone())
}

fn latest(source: &impl UsageSource) -> Option<UsageSnapshot> {
    source.latest().expect("statusline file should be readable")
}

/// Writes `fixture_name` as the statusline file of `root`, dated `age` ago.
fn add_rate_limits(root: &TempDir, fixture_name: &str, age: Duration) -> PathBuf {
    let file = root.path().join(RATE_LIMITS_FILE);
    fs::create_dir_all(file.parent().expect("file has a folder"))
        .expect("folder should be writable");
    fs::copy(fixture(fixture_name), &file).expect("fixture should be copied");
    touch(&file, SystemTime::now() - age);
    file
}

fn touch(path: &Path, modified: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .and_then(|file| file.set_modified(modified))
        .expect("mtime should be settable");
}

fn percents(snapshot: &UsageSnapshot) -> (Option<f64>, Option<f64>) {
    (
        snapshot.five_hours.and_then(|window| window.used_percent()),
        snapshot.weekly.and_then(|window| window.used_percent()),
    )
}

#[test]
fn reads_exact_windows_dated_by_the_file() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let file = add_rate_limits(&root, "rate-limits.json", Duration::from_secs(60));
    let written_at = adapter_fs::epoch_seconds(
        fs::metadata(&file)
            .and_then(|metadata| metadata.modified())
            .expect("mtime should be readable"),
    );

    let snapshot = latest(&source(&root)).expect("quotas should be known");

    assert_eq!(snapshot.provider, Provider::Claude);
    assert_eq!(snapshot.origin, Origin::Statusline);
    assert_eq!(snapshot.observed_at, written_at);
    assert_eq!(percents(&snapshot), (Some(100.0), Some(41.2)));
    assert_eq!(
        snapshot.five_hours.and_then(|window| window.resets_at()),
        Some(1_738_425_600)
    );
    assert!(!snapshot
        .five_hours
        .is_some_and(|window| window.is_estimated()));
}

#[test]
fn an_absent_window_stays_unknown() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    add_rate_limits(&root, "rate-limits-partial.json", Duration::from_secs(1));

    let snapshot = latest(&source(&root)).expect("quotas should be known");

    assert_eq!(percents(&snapshot), (None, Some(64.0)));
}

#[test]
fn a_missing_file_has_no_usage() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");

    assert_eq!(latest(&source(&root)), None);
}

#[test]
fn reads_the_file_again_only_when_its_mtime_changes() {
    let root = tempfile::tempdir().expect("temporary folder should be writable");
    let file = add_rate_limits(&root, "rate-limits.json", Duration::from_secs(60));
    let source = source(&root);

    latest(&source);
    latest(&source);
    assert_eq!(source.reads(), 1);

    fs::copy(fixture("rate-limits-partial.json"), &file).expect("fixture should be copied");
    touch(&file, SystemTime::now());

    assert_eq!(
        latest(&source).map(|snapshot| percents(&snapshot)),
        Some((None, Some(64.0)))
    );
    assert_eq!(source.reads(), 2);
}

#[test]
fn keeps_the_most_recent_file_across_roots() {
    let old = tempfile::tempdir().expect("temporary folder should be writable");
    let new = tempfile::tempdir().expect("temporary folder should be writable");
    add_rate_limits(&old, "rate-limits.json", Duration::from_secs(600));
    add_rate_limits(&new, "rate-limits-partial.json", Duration::from_secs(5));
    let roots = vec![old.path().to_path_buf(), new.path().to_path_buf()];
    let source = ClaudeStatuslineSource::new(move || roots.clone());

    assert_eq!(
        latest(&source).map(|snapshot| percents(&snapshot)),
        Some((None, Some(64.0)))
    );
}

#[cfg(unix)]
mod script {
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use super::{fixture, latest, percents, source};

    fn script() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/statusline/agenthud-statusline.sh")
    }

    /// Runs the real script with `input` on stdin and `CLAUDE_CONFIG_DIR`
    /// set to `config_dir`; returns what it prints.
    fn run(config_dir: &Path, input: &[u8], chain: Option<&str>) -> String {
        let mut command = Command::new("sh");
        command
            .arg(script())
            .args(chain)
            .env("CLAUDE_CONFIG_DIR", config_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        let mut child = command.spawn().expect("sh should start");
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(input)
            .expect("input should be written");
        let output = child.wait_with_output().expect("script should finish");
        assert!(output.status.success(), "script failed: {output:?}");
        String::from_utf8(output.stdout).expect("output should be text")
    }

    fn input() -> Vec<u8> {
        std::fs::read(fixture("statusline-input.json")).expect("fixture should be readable")
    }

    #[test]
    fn the_script_writes_the_quotas_the_adapter_reads() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");

        run(root.path(), &input(), None);

        let snapshot = latest(&source(&root)).expect("quotas should be known");
        assert_eq!(percents(&snapshot), (Some(23.5), Some(41.2)));
        assert_eq!(
            snapshot.weekly.and_then(|window| window.resets_at()),
            Some(1_738_857_600)
        );
    }

    #[test]
    fn an_input_without_quotas_keeps_the_previous_file() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        run(root.path(), &input(), None);

        run(root.path(), br#"{"model":{"display_name":"Opus"}}"#, None);

        let snapshot = latest(&source(&root)).expect("quotas should be kept");
        assert_eq!(percents(&snapshot), (Some(23.5), Some(41.2)));
        let leftovers = std::fs::read_dir(root.path().join("agenthud"))
            .expect("folder should exist")
            .count();
        assert_eq!(leftovers, 1, "no temporary file is left behind");
    }

    #[test]
    fn the_chained_statusline_gets_the_same_input_and_is_shown() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");

        let shown = run(root.path(), &input(), Some("wc -c | tr -d ' '"));

        // Command substitution drops the final newline; the JSON is unchanged.
        assert_eq!(shown.trim(), input().trim_ascii_end().len().to_string());
        assert!(latest(&source(&root)).is_some());
    }
}
