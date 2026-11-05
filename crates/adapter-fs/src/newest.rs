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
    let mut newest: Option<DatedFile> = None;
    walk(&root.join(subdir), extension, MAX_DEPTH, &mut |file| {
        if newest
            .as_ref()
            .is_none_or(|best| file.modified > best.modified)
        {
            newest = Some(file);
        }
    })?;
    Ok(newest)
}

/// Calls `found` for each file with `extension` under `dir`, descending at
/// most `max_depth` levels (1 = the files of `dir` only).
///
/// A missing `dir` holds no file. Unreadable entries below it are logged and
/// skipped.
///
/// # Errors
///
/// Fails when `dir` exists but cannot be listed.
pub(crate) fn walk(
    dir: &Path,
    extension: &str,
    max_depth: usize,
    found: &mut impl FnMut(DatedFile),
) -> io::Result<()> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            visit(entries, extension, max_depth, found);
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn visit(entries: fs::ReadDir, extension: &str, depth: usize, found: &mut impl FnMut(DatedFile)) {
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
                    Ok(children) => visit(children, extension, depth - 1, found),
                    Err(error) => log::debug!("skipping {}: {error}", path.display()),
                }
            }
        } else if path.extension().is_some_and(|ext| ext == extension) {
            match metadata.modified() {
                Ok(modified) => found(DatedFile { path, modified }),
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
