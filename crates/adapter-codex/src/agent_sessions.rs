//! Codex agents. When the CLI locks its open threads, the agents are
//! exactly those threads; otherwise one per session file written recently.
//! The state comes from the last turn of the session file, `waiting` for a
//! thread that has none yet.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use adapter_fs::DatedFile;
use application::{AgentStateSource, SourceError};
use domain::{AgentSession, Provider, TurnSignal, ACTIVE_WITHIN};
use serde::Deserialize;

use crate::open_threads::{open_threads, OpenThread};
use crate::{turn_signal, SESSIONS_DIR, SESSION_EXTENSION};

/// Depth of `sessions/<year>/<month>/<day>/<file>`, with some slack.
const SESSIONS_DEPTH: usize = 6;

/// Largest first line read for the `session_meta` of a session.
const META_MAX_BYTES: u64 = 256 * 1024;

/// Codex sessions written within `ACTIVE_WITHIN`, across all roots. A file
/// is only read again once its mtime changes.
pub struct CodexAgentSessions<F> {
    roots: F,
    cache: Mutex<HashMap<PathBuf, SessionState>>,
    /// Session files read so far, to check the mtime cache.
    reads: AtomicUsize,
}

/// What was last read from one session file.
struct SessionState {
    modified: SystemTime,
    signal: TurnSignal,
    project: Option<String>,
}

impl<F> CodexAgentSessions<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    /// `roots` lists the `.codex` folders to scan, on each call.
    pub fn new(roots: F) -> Self {
        Self {
            roots,
            cache: Mutex::new(HashMap::new()),
            reads: AtomicUsize::new(0),
        }
    }

    /// Session files read from disk so far.
    pub fn reads(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    fn session(
        &self,
        file: &DatedFile,
        cache: &mut HashMap<PathBuf, SessionState>,
    ) -> Result<AgentSession, SourceError> {
        let cached = cache.get(&file.path);
        let (signal, project) = match cached {
            Some(state) if state.modified == file.modified => (state.signal, state.project.clone()),
            _ => {
                self.reads.fetch_add(1, Ordering::Relaxed);
                let signal = adapter_fs::find_last(&file.path, turn_signal::parse_line)
                    .map_err(|source| SourceError::Io {
                        path: file.path.clone(),
                        source,
                    })?
                    .unwrap_or(TurnSignal::Unknown);
                // The first line never changes: keep the project once read.
                let project = match cached {
                    Some(state) => state.project.clone(),
                    None => read_project(&file.path),
                };
                cache.insert(
                    file.path.clone(),
                    SessionState {
                        modified: file.modified,
                        signal,
                        project: project.clone(),
                    },
                );
                (signal, project)
            }
        };
        Ok(AgentSession {
            provider: Provider::Codex,
            session_id: session_id(&file.path),
            project,
            last_signal: signal,
            last_write: adapter_fs::epoch_seconds(file.modified),
        })
    }
}

impl<F> AgentStateSource for CodexAgentSessions<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    /// Sessions of every root. An unreadable root or file is logged and
    /// skipped; an error is only returned when nothing could be read.
    fn sessions(&self) -> Result<Vec<AgentSession>, SourceError> {
        let since = active_since(SystemTime::now());
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        let mut sessions = Vec::new();
        let mut seen = Vec::new();
        let mut first_error = None;
        for root in (self.roots)() {
            let dir = root.join(SESSIONS_DIR);
            let files =
                match adapter_fs::recent_files(&dir, SESSION_EXTENSION, since, SESSIONS_DEPTH) {
                    Ok(files) => files,
                    Err(source) => {
                        let error = SourceError::Io { path: dir, source };
                        log::warn!("Codex root skipped: {error}");
                        first_error.get_or_insert(error);
                        continue;
                    }
                };
            let threads = open_threads(&root);
            let files = match &threads {
                Some(threads) => locked_files(files, threads),
                None => files,
            };
            for file in &files {
                match self.session(file, &mut cache) {
                    Ok(session) => sessions.push(session),
                    Err(error) => {
                        log::warn!("Codex session skipped: {error}");
                        first_error.get_or_insert(error);
                    }
                }
                seen.push(file.path.clone());
            }
            if let Some(threads) = threads {
                sessions.extend(new_threads(threads, &files));
            }
        }
        cache.retain(|path, _| seen.contains(path));
        match (sessions.is_empty(), first_error) {
            (true, Some(error)) => Err(error),
            _ => Ok(sessions),
        }
    }
}

/// Session files of the open threads; the others belong to closed threads.
fn locked_files(files: Vec<DatedFile>, threads: &[OpenThread]) -> Vec<DatedFile> {
    files
        .into_iter()
        .filter(|file| threads.iter().any(|thread| holds(file, thread)))
        .collect()
}

/// Open threads without session file yet: Codex writes it on the first
/// message, so they wait for the user since their lock.
fn new_threads(threads: Vec<OpenThread>, files: &[DatedFile]) -> Vec<AgentSession> {
    threads
        .into_iter()
        .filter(|thread| !files.iter().any(|file| holds(file, thread)))
        .map(|thread| AgentSession {
            provider: Provider::Codex,
            session_id: thread.id,
            project: None,
            last_signal: TurnSignal::TurnEnded,
            last_write: adapter_fs::epoch_seconds(thread.locked),
        })
        .collect()
}

/// Whether `file` is the session file of `thread`: its name has the id.
fn holds(file: &DatedFile, thread: &OpenThread) -> bool {
    file.path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.contains(thread.id.as_str()))
}

/// Oldest mtime of an active session.
fn active_since(now: SystemTime) -> SystemTime {
    let window = Duration::from_secs(ACTIVE_WITHIN.unsigned_abs());
    now.checked_sub(window).unwrap_or(SystemTime::UNIX_EPOCH)
}

fn session_id(path: &Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

/// Project of a session: the last folder of the `cwd` in its first line,
/// `None` when that line cannot be read cheaply.
fn read_project(path: &Path) -> Option<String> {
    let first_line = || -> io::Result<Vec<u8>> {
        let mut line = Vec::new();
        BufReader::new(File::open(path)?.take(META_MAX_BYTES)).read_until(b'\n', &mut line)?;
        Ok(line)
    };
    match first_line() {
        Ok(line) => parse_project(&String::from_utf8_lossy(&line)),
        Err(error) => {
            log::debug!("no project for {}: {error}", path.display());
            None
        }
    }
}

#[derive(Deserialize)]
struct MetaLine {
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    payload: Option<MetaPayload>,
}

#[derive(Deserialize)]
struct MetaPayload {
    #[serde(default)]
    cwd: Option<String>,
}

/// Last folder of the `cwd` of a `session_meta` line, Windows or Unix path.
fn parse_project(line: &str) -> Option<String> {
    let line: MetaLine = serde_json::from_str(line.trim_end()).ok()?;
    if line.kind.as_deref() != Some("session_meta") {
        return None;
    }
    line.payload?
        .cwd?
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::parse_project;

    #[test]
    fn the_project_is_the_last_folder_of_the_cwd() {
        let meta = |cwd: &str| {
            format!(
                r#"{{"type":"session_meta","payload":{{"cwd":{}}}}}"#,
                serde_json::json!(cwd)
            )
        };

        assert_eq!(
            parse_project(&meta("/workspace/project")).as_deref(),
            Some("project")
        );
        assert_eq!(
            parse_project(&meta(r"C:\work\project\")).as_deref(),
            Some("project")
        );
    }

    #[test]
    fn no_project_without_a_session_meta() {
        assert_eq!(
            parse_project(r#"{"type":"event_msg","payload":{"cwd":"/a"}}"#),
            None
        );
        assert_eq!(
            parse_project(r#"{"type":"session_meta","payload":{}}"#),
            None
        );
        assert_eq!(parse_project("{\"type\":\"session_meta\""), None);
    }
}
