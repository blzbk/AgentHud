//! Finds the most recently written file under a root, by polling mtimes
//! instead of watching the disk.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Depth limit of the walk, deep enough for `sessions/<year>/<month>/<day>`.
const MAX_DEPTH: usize = 6;

/// A file and its last modification time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatedFile {
    pub path: PathBuf,
    pub modified: SystemTime,
}

/// Most recently modified file with `extension` under `root/subdir`.
///
/// Returns `Ok(None)` when the folder is missing or holds no such file.
/// Unreadable entries below the folder are logged and skipped.
///
/// # Errors
///
/// Fails when `root/subdir` exists but cannot be listed.
pub fn newest_file(root: &Path, subdir: &str, extension: &str) -> io::Result<Option<DatedFile>> {
    let dir = root.join(subdir);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut newest = None;
    visit(entries, extension, MAX_DEPTH, &mut newest);
    Ok(newest)
}

fn visit(entries: fs::ReadDir, extension: &str, depth: usize, newest: &mut Option<DatedFile>) {
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                log::debug!("skipping an unreadable entry: {error}");
                continue;
            }
        };
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                log::debug!("skipping {}: {error}", path.display());
                continue;
            }
        };
        if metadata.is_dir() {
            if depth > 1 {
                match fs::read_dir(&path) {
                    Ok(children) => visit(children, extension, depth - 1, newest),
                    Err(error) => log::debug!("skipping {}: {error}", path.display()),
                }
            }
        } else if path.extension().is_some_and(|ext| ext == extension) {
            match metadata.modified() {
                Ok(modified) if newest.as_ref().is_none_or(|best| modified > best.modified) => {
                    *newest = Some(DatedFile { path, modified });
                }
                Ok(_) => {}
                Err(error) => log::debug!("skipping {}: {error}", path.display()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    use super::newest_file;

    fn write_at(path: &Path, age_seconds: u64) {
        fs::create_dir_all(path.parent().expect("file should have a parent"))
            .expect("folder should be writable");
        let file = File::create(path).expect("file should be writable");
        file.set_modified(SystemTime::now() - Duration::from_secs(age_seconds))
            .expect("mtime should be settable");
    }

    #[test]
    fn returns_the_most_recently_modified_file() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        let sessions = root.path().join("sessions");
        write_at(&sessions.join("2026/11/02/old.jsonl"), 3600);
        write_at(&sessions.join("2026/11/03/recent.jsonl"), 10);
        write_at(&sessions.join("2026/11/03/middle.jsonl"), 600);
        write_at(&sessions.join("2026/11/03/newer.txt"), 0);

        let newest = newest_file(root.path(), "sessions", "jsonl")
            .expect("walk should succeed")
            .expect("a file should be found");

        assert_eq!(newest.path, sessions.join("2026/11/03/recent.jsonl"));
    }

    #[test]
    fn a_missing_root_has_no_file() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");

        let newest = newest_file(&root.path().join("absent"), "sessions", "jsonl");

        assert!(matches!(newest, Ok(None)));
    }

    #[test]
    fn an_empty_sessions_folder_has_no_file() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        fs::create_dir(root.path().join("sessions")).expect("folder should be writable");

        let newest = newest_file(root.path(), "sessions", "jsonl");

        assert!(matches!(newest, Ok(None)));
    }
}
