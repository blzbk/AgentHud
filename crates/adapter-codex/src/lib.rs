//! Codex adapter: reads the files under `~/.codex` and implements the
//! `application` ports for the Codex provider.

mod token_count;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use adapter_fs::DatedFile;
use application::{SourceError, UsageSource};
use domain::{Provider, UsageSnapshot};

/// Folder of the Codex data, under each home.
pub const CODEX_DIR: &str = ".codex";

const SESSIONS_DIR: &str = "sessions";
const SESSION_EXTENSION: &str = "jsonl";

/// Codex usage, read from the newest session file of each root. A file is
/// only read again once its path or mtime changes.
pub struct CodexUsageSource<F> {
    roots: F,
    cache: Mutex<HashMap<PathBuf, RootState>>,
    /// Session files read so far, to check the mtime cache.
    reads: AtomicUsize,
}

/// What was last read under one root.
struct RootState {
    file: DatedFile,
    snapshot: Option<UsageSnapshot>,
}

impl<F> CodexUsageSource<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    /// `roots` lists the `.codex` folders to scan. It is called on each
    /// refresh, so a WSL distribution started later is picked up.
    pub fn new(roots: F) -> Self {
        Self {
            roots,
            cache: Mutex::new(HashMap::new()),
            reads: AtomicUsize::new(0),
        }
    }

    /// Latest snapshot under `root`, read from disk only when its newest
    /// session changed since the previous call.
    fn root_snapshot(
        &self,
        root: &Path,
        cache: &mut HashMap<PathBuf, RootState>,
    ) -> Result<Option<UsageSnapshot>, SourceError> {
        let io_error = |path: &Path| {
            let path = path.to_path_buf();
            move |source| SourceError::Io { path, source }
        };
        let Some(newest) = adapter_fs::newest_file(root, SESSIONS_DIR, SESSION_EXTENSION)
            .map_err(io_error(&root.join(SESSIONS_DIR)))?
        else {
            return Ok(cache.remove(root).and_then(|state| state.snapshot));
        };
        if let Some(state) = cache.get(root) {
            if state.file == newest {
                return Ok(state.snapshot.clone());
            }
        }

        self.reads.fetch_add(1, Ordering::Relaxed);
        let fallback_time = adapter_fs::epoch_seconds(newest.modified);
        let read = adapter_fs::find_last(&newest.path, |line| {
            token_count::parse_line(line, fallback_time)
        })
        .map_err(io_error(&newest.path))?;
        // A session without usage yet (just started) keeps the previous one.
        let snapshot = read.or_else(|| cache.get(root).and_then(|state| state.snapshot.clone()));
        cache.insert(
            root.to_path_buf(),
            RootState {
                file: newest,
                snapshot: snapshot.clone(),
            },
        );
        Ok(snapshot)
    }
}

impl<F> UsageSource for CodexUsageSource<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    /// Most recent snapshot across all roots. An unreadable root is logged
    /// and skipped; its error is only returned when no root gave a usage.
    fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError> {
        let roots = (self.roots)();
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.retain(|root, _| roots.contains(root));

        let mut latest: Option<UsageSnapshot> = None;
        let mut first_error = None;
        for root in &roots {
            match self.root_snapshot(root, &mut cache) {
                Ok(Some(snapshot)) => {
                    if latest
                        .as_ref()
                        .is_none_or(|best| snapshot.observed_at > best.observed_at)
                    {
                        latest = Some(snapshot);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    log::warn!("Codex root skipped: {error}");
                    first_error.get_or_insert(error);
                }
            }
        }
        match (latest, first_error) {
            (None, Some(error)) => Err(error),
            (latest, _) => Ok(latest),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::Ordering;
    use std::time::{Duration, SystemTime};

    use application::UsageSource;

    use super::CodexUsageSource;

    fn token_count(timestamp: &str, used_percent: f64) -> String {
        format!(
            "{{\"timestamp\":\"{timestamp}\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"rate_limits\":{{\"primary\":{{\"used_percent\":{used_percent},\"window_minutes\":300}}}}}}}}\n"
        )
    }

    fn write_session(path: &Path, content: &str, modified: SystemTime) {
        fs::create_dir_all(path.parent().expect("session has a folder"))
            .expect("folder should be writable");
        fs::write(path, content).expect("session should be writable");
        File::options()
            .write(true)
            .open(path)
            .and_then(|file| file.set_modified(modified))
            .expect("mtime should be settable");
    }

    fn five_hours(source: &impl UsageSource) -> Option<f64> {
        source
            .latest()
            .expect("usage should be readable")
            .and_then(|snapshot| snapshot.five_hours)
            .and_then(|window| window.used_percent())
    }

    #[test]
    fn reads_a_session_again_only_when_its_mtime_changes() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        let session = root.path().join("sessions/2026/11/03/rollout.jsonl");
        let then = SystemTime::now() - Duration::from_secs(60);
        write_session(&session, &token_count("2026-11-03T12:00:00Z", 10.0), then);
        let roots = vec![PathBuf::from(root.path())];
        let source = CodexUsageSource::new(move || roots.clone());

        assert_eq!(five_hours(&source), Some(10.0));
        assert_eq!(five_hours(&source), Some(10.0));
        assert_eq!(source.reads.load(Ordering::Relaxed), 1);

        write_session(
            &session,
            &token_count("2026-11-03T12:01:00Z", 20.0),
            SystemTime::now(),
        );

        assert_eq!(five_hours(&source), Some(20.0));
        assert_eq!(source.reads.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn a_new_session_without_usage_keeps_the_previous_one() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        let then = SystemTime::now() - Duration::from_secs(60);
        write_session(
            &root.path().join("sessions/2026/11/03/old.jsonl"),
            &token_count("2026-11-03T12:00:00Z", 10.0),
            then,
        );
        let roots = vec![PathBuf::from(root.path())];
        let source = CodexUsageSource::new(move || roots.clone());
        assert_eq!(five_hours(&source), Some(10.0));

        write_session(
            &root.path().join("sessions/2026/11/03/new.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{}}\n",
            SystemTime::now(),
        );

        assert_eq!(five_hours(&source), Some(10.0));
    }
}
