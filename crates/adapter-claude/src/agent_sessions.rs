//! Claude agents. When the CLI marks its running sessions, the agents are
//! exactly those sessions; otherwise one per transcript written recently.
//! The state comes from the marker status, else from the last turn of the
//! transcript.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use adapter_fs::DatedFile;
use application::{AgentStateSource, SourceError};
use domain::{AgentSession, Provider, TurnSignal, ACTIVE_WITHIN};

use crate::running_sessions::{running_sessions, RunningSession};
use crate::transcript;
use crate::{PROJECTS_DIR, TRANSCRIPT_EXTENSION};

/// Depth of `projects/<project>/<session>/subagents/<agent>.jsonl`.
/// Subagents are not sessions, but their writes keep their session active.
const SESSIONS_DEPTH: usize = 4;

const SUBAGENTS_DIR: &str = "subagents";

/// Claude sessions written within `ACTIVE_WITHIN`, across all roots. A
/// transcript is only read again once its mtime changes.
pub struct ClaudeAgentSessions<F> {
    roots: F,
    cache: Mutex<HashMap<PathBuf, SessionState>>,
    /// Transcript reads so far, to check the mtime cache.
    reads: AtomicUsize,
}

struct SessionState {
    modified: SystemTime,
    signal: TurnSignal,
    project: Option<String>,
}

impl<F> ClaudeAgentSessions<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    /// `roots` lists the `.claude` folders to scan, on each call.
    pub fn new(roots: F) -> Self {
        Self {
            roots,
            cache: Mutex::new(HashMap::new()),
            reads: AtomicUsize::new(0),
        }
    }

    /// Transcript reads so far.
    pub fn reads(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    /// Session of the transcript `file`, last written at `last_write`.
    fn session(
        &self,
        file: &DatedFile,
        last_write: SystemTime,
        cache: &mut HashMap<PathBuf, SessionState>,
    ) -> Result<AgentSession, SourceError> {
        let (signal, project) = match cache.get(&file.path) {
            Some(state) if state.modified == file.modified => (state.signal, state.project.clone()),
            _ => {
                self.reads.fetch_add(1, Ordering::Relaxed);
                let turn =
                    adapter_fs::find_last(&file.path, transcript::turn_line).map_err(|source| {
                        SourceError::Io {
                            path: file.path.clone(),
                            source,
                        }
                    })?;
                let signal = turn
                    .as_ref()
                    .map_or(TurnSignal::Unknown, |turn| turn.signal);
                let project = turn
                    .and_then(|turn| turn.cwd)
                    .and_then(|cwd| last_folder(&cwd))
                    .or_else(|| project_folder(&file.path));
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
            provider: Provider::Claude,
            session_id: file
                .path
                .file_stem()
                .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned()),
            project,
            last_signal: signal,
            last_write: adapter_fs::epoch_seconds(last_write),
        })
    }
}

impl<F> AgentStateSource for ClaudeAgentSessions<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    /// Sessions of every root. An unreadable root or transcript is logged
    /// and skipped; an error is only returned when nothing could be read.
    fn sessions(&self) -> Result<Vec<AgentSession>, SourceError> {
        let window = Duration::from_secs(ACTIVE_WITHIN.unsigned_abs());
        let since = SystemTime::now()
            .checked_sub(window)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        let mut sessions = Vec::new();
        let mut seen = Vec::new();
        let mut first_error = None;
        for root in (self.roots)() {
            let dir = root.join(PROJECTS_DIR);
            let transcripts =
                match adapter_fs::recent_files(&dir, TRANSCRIPT_EXTENSION, since, SESSIONS_DEPTH) {
                    Ok(files) => files,
                    Err(source) => {
                        let error = SourceError::Io { path: dir, source };
                        log::warn!("Claude root skipped: {error}");
                        first_error.get_or_insert(error);
                        continue;
                    }
                };
            let (transcripts, subagent_writes) = split_subagents(&dir, transcripts);
            let mut root_sessions = Vec::new();
            for file in &transcripts {
                let last_write = subagent_writes
                    .get(&file.path)
                    .map_or(file.modified, |&write| write.max(file.modified));
                match self.session(file, last_write, &mut cache) {
                    Ok(session) => root_sessions.push(session),
                    Err(error) => {
                        log::warn!("Claude session skipped: {error}");
                        first_error.get_or_insert(error);
                    }
                }
                seen.push(file.path.clone());
            }
            match running_sessions(&root) {
                Some(running) => sessions.extend(running_agents(running, root_sessions)),
                None => sessions.extend(root_sessions),
            }
        }
        cache.retain(|path, _| seen.contains(path));
        match (sessions.is_empty(), first_error) {
            (true, Some(error)) => Err(error),
            _ => Ok(sessions),
        }
    }
}

/// Splits the files under `projects` into the session transcripts and, for
/// each transcript, the latest write of its subagents.
fn split_subagents(
    projects: &Path,
    files: Vec<DatedFile>,
) -> (Vec<DatedFile>, HashMap<PathBuf, SystemTime>) {
    let mut transcripts = Vec::new();
    let mut subagent_writes: HashMap<PathBuf, SystemTime> = HashMap::new();
    for file in files {
        let parent = file.path.parent();
        if parent.and_then(Path::parent) == Some(projects) {
            transcripts.push(file);
        } else if let Some(session_dir) = parent
            .filter(|dir| dir.file_name().is_some_and(|name| name == SUBAGENTS_DIR))
            .and_then(Path::parent)
        {
            let transcript = session_dir.with_extension(TRANSCRIPT_EXTENSION);
            let write = subagent_writes.entry(transcript).or_insert(file.modified);
            *write = (*write).max(file.modified);
        }
    }
    (transcripts, subagent_writes)
}

/// The running sessions, completed by their transcript when it was written
/// recently: its last write, its project when the marker has no `cwd`, and
/// its last turn when the marker status is unknown. Transcripts of ended
/// sessions are dropped.
fn running_agents(
    running: Vec<RunningSession>,
    transcripts: Vec<AgentSession>,
) -> impl Iterator<Item = AgentSession> {
    let mut transcripts: HashMap<String, AgentSession> = transcripts
        .into_iter()
        .map(|session| (session.session_id.clone(), session))
        .collect();
    running.into_iter().map(move |marker| {
        let transcript = transcripts.remove(&marker.session_id);
        AgentSession {
            provider: Provider::Claude,
            project: marker.project.or_else(|| {
                transcript
                    .as_ref()
                    .and_then(|session| session.project.clone())
            }),
            last_signal: marker
                .signal
                .or_else(|| transcript.as_ref().map(|session| session.last_signal))
                .unwrap_or(TurnSignal::Unknown),
            last_write: transcript.map_or_else(
                || adapter_fs::epoch_seconds(marker.modified),
                |session| session.last_write,
            ),
            session_id: marker.session_id,
        }
    })
}

/// Last folder of a Windows or Unix path.
pub(crate) fn last_folder(path: &str) -> Option<String> {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .map(str::to_owned)
}

/// Encoded folder of the project holding `transcript`, kept raw: decoding
/// it is ambiguous, since `-` also stands for `/` and `.`.
fn project_folder(transcript: &Path) -> Option<String> {
    transcript
        .parent()?
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{last_folder, project_folder};

    #[test]
    fn the_project_is_the_last_folder_of_the_cwd() {
        assert_eq!(
            last_folder("/workspace/project").as_deref(),
            Some("project")
        );
        assert_eq!(last_folder(r"C:\work\project\").as_deref(), Some("project"));
        assert_eq!(last_folder("/"), None);
    }

    #[test]
    fn without_cwd_the_project_is_the_raw_folder_name() {
        assert_eq!(
            project_folder(Path::new("projects/-workspace-project/session.jsonl")).as_deref(),
            Some("-workspace-project")
        );
    }
}
