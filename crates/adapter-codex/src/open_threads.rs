//! Open Codex conversations: the CLI holds `thread-writer-locks/<id>.lock`
//! while a thread is open and deletes it when the thread closes. The folder
//! is internal and undocumented: it is only listed, never written.

use std::path::Path;
use std::time::SystemTime;

const LOCKS_DIR: &str = "thread-writer-locks";
const LOCK_EXTENSION: &str = "lock";

/// One open thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenThread {
    /// Thread id, also the end of its rollout file name.
    pub id: String,
    /// Last write of the lock, the thread opening.
    pub locked: SystemTime,
}

/// Open threads of the `.codex` folder `root`, `None` when the lock folder
/// is missing or unreadable: the caller then falls back on the rollouts.
/// Hidden locks such as `.coordination.lock` belong to no thread.
pub fn open_threads(root: &Path) -> Option<Vec<OpenThread>> {
    let dir = root.join(LOCKS_DIR);
    if !dir.is_dir() {
        return None;
    }
    let locks = match adapter_fs::recent_files(&dir, LOCK_EXTENSION, SystemTime::UNIX_EPOCH, 1) {
        Ok(locks) => locks,
        Err(error) => {
            log::debug!("Codex locks unreadable in {}: {error}", dir.display());
            return None;
        }
    };
    let threads = locks
        .into_iter()
        .filter_map(|lock| {
            let id = lock.path.file_stem()?.to_str()?;
            (!id.is_empty() && !id.starts_with('.')).then(|| OpenThread {
                id: id.to_owned(),
                locked: lock.modified,
            })
        })
        .collect();
    Some(threads)
}
