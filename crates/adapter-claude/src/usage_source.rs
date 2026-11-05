//! Claude usage over the estimated 5-hour window, summed from the tokens of
//! the local transcripts. No request is ever sent to Anthropic.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use adapter_fs::DatedFile;
use application::{SourceError, UsageSource};
use domain::{Origin, Provider, UsageSnapshot, UsageWindow, WindowKind};

use crate::transcript::{self, UsageEntry};
use crate::usage_block;
use crate::{PROJECTS_DIR, TRANSCRIPT_EXTENSION};

/// Transcripts written within this delay are read. Longer than one block,
/// so the start of the current block is found even after a busy day.
const HISTORY: Duration = Duration::from_hours(24);

/// Depth of `projects/<project>/<session>/subagents/<agent>.jsonl`: the
/// tokens of subagents count too.
const TRANSCRIPTS_DEPTH: usize = 4;

/// Claude usage across all roots. A transcript is only read again once its
/// mtime changes, and then only from where the previous read stopped.
pub struct ClaudeUsageSource<F> {
    roots: F,
    cache: Mutex<HashMap<PathBuf, TranscriptState>>,
    /// Transcript reads so far, to check the mtime cache.
    reads: AtomicUsize,
}

/// What was last read from one transcript.
struct TranscriptState {
    modified: SystemTime,
    /// Bytes read so far: the end of the last complete line.
    offset: u64,
    entries: Vec<UsageEntry>,
}

impl<F> ClaudeUsageSource<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    /// `roots` lists the `.claude` folders to scan, on each refresh.
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

    /// Brings the cache of `file` up to date.
    fn refresh(
        &self,
        file: &DatedFile,
        cache: &mut HashMap<PathBuf, TranscriptState>,
    ) -> io::Result<()> {
        let state = cache.entry(file.path.clone()).or_insert(TranscriptState {
            modified: SystemTime::UNIX_EPOCH,
            offset: 0,
            entries: Vec::new(),
        });
        if state.modified == file.modified {
            return Ok(());
        }
        self.reads.fetch_add(1, Ordering::Relaxed);
        let (text, offset) = if let Some(read) = read_complete_lines(&file.path, state.offset)? {
            read
        } else {
            // Shorter than before: rewritten, read it all again.
            state.entries.clear();
            read_complete_lines(&file.path, 0)?.unwrap_or_default()
        };
        state
            .entries
            .extend(text.lines().filter_map(transcript::usage_entry));
        state.offset = offset;
        state.modified = file.modified;
        Ok(())
    }
}

impl<F> UsageSource for ClaudeUsageSource<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    /// Current block across all roots, as an estimated window in tokens.
    /// An unreadable root or transcript is logged and skipped; an error is
    /// only returned when no usage could be read.
    fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError> {
        let since = SystemTime::now()
            .checked_sub(HISTORY)
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        let mut seen = Vec::new();
        let mut first_error = None;
        for root in (self.roots)() {
            let dir = root.join(PROJECTS_DIR);
            let files = match adapter_fs::recent_files(
                &dir,
                TRANSCRIPT_EXTENSION,
                since,
                TRANSCRIPTS_DEPTH,
            ) {
                Ok(files) => files,
                Err(source) => {
                    let error = SourceError::Io { path: dir, source };
                    log::warn!("Claude root skipped: {error}");
                    first_error.get_or_insert(error);
                    continue;
                }
            };
            for file in files {
                if let Err(source) = self.refresh(&file, &mut cache) {
                    let error = SourceError::Io {
                        path: file.path.clone(),
                        source,
                    };
                    log::warn!("Claude transcript skipped: {error}");
                    first_error.get_or_insert(error);
                    cache.remove(&file.path);
                    continue;
                }
                seen.push(file.path);
            }
        }
        cache.retain(|path, _| seen.contains(path));

        let block = usage_block::latest_block(cache.values().flat_map(|state| &state.entries));
        match (block, first_error) {
            (None, Some(error)) => Err(error),
            (None, None) => Ok(None),
            (Some(block), _) => Ok(Some(UsageSnapshot {
                provider: Provider::Claude,
                five_hours: Some(
                    UsageWindow::tokens(WindowKind::FiveHours, block.tokens, Some(block.end))
                        .estimated(),
                ),
                weekly: None,
                observed_at: block.last_message,
                origin: Origin::Transcripts,
            })),
        }
    }
}

/// Text of the complete lines of `path` after `offset`, and the offset of
/// the end of the last one. A line still being written is left for the next
/// read. `None` when the file is now shorter than `offset`.
fn read_complete_lines(path: &Path, offset: u64) -> io::Result<Option<(String, u64)>> {
    let mut file = File::open(path)?;
    if file.metadata()?.len() < offset {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let complete = bytes
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |last| last + 1);
    bytes.truncate(complete);
    let read = u64::try_from(complete).unwrap_or(u64::MAX);
    Ok(Some((
        String::from_utf8_lossy(&bytes).into_owned(),
        offset.saturating_add(read),
    )))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::read_complete_lines;

    #[test]
    fn reads_only_complete_lines_from_the_offset() {
        let dir = tempfile::tempdir().expect("temporary folder should be writable");
        let path = dir.path().join("session.jsonl");
        fs::write(&path, "one\ntwo\nthr").expect("file should be writable");

        assert_eq!(
            read_complete_lines(&path, 0).expect("file should be readable"),
            Some(("one\ntwo\n".to_owned(), 8))
        );
        assert_eq!(
            read_complete_lines(&path, 4).expect("file should be readable"),
            Some(("two\n".to_owned(), 8))
        );
        assert_eq!(
            read_complete_lines(&path, 20).expect("file should be readable"),
            None
        );
    }
}
