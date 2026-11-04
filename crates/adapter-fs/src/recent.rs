//! Lists the files written since a given time, to find the active sessions
//! without reading every session ever recorded.

use std::io;
use std::path::Path;
use std::time::SystemTime;

use crate::newest::{walk, DatedFile};

/// Files with `extension` under `dir` modified after `since`, most recent
/// first. The walk descends at most `max_depth` levels (1 = the files of
/// `dir` only).
///
/// Returns an empty list when `dir` is missing. Unreadable entries below it
/// are logged and skipped.
///
/// # Errors
///
/// Fails when `dir` exists but cannot be listed.
pub fn recent_files(
    dir: &Path,
    extension: &str,
    since: SystemTime,
    max_depth: usize,
) -> io::Result<Vec<DatedFile>> {
    let mut files = Vec::new();
    walk(dir, extension, max_depth, &mut |file| {
        if file.modified > since {
            files.push(file);
        }
    })?;
    files.sort_by_key(|file| std::cmp::Reverse(file.modified));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    use super::recent_files;

    fn write_at(path: &Path, modified: SystemTime) {
        fs::create_dir_all(path.parent().expect("file should have a parent"))
            .expect("folder should be writable");
        File::create(path)
            .and_then(|file| file.set_modified(modified))
            .expect("file should be writable");
    }

    #[test]
    fn keeps_the_files_written_since_most_recent_first() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        let now = SystemTime::now();
        let minutes = |count: u64| now - Duration::from_secs(count * 60);
        write_at(&root.path().join("a/old.jsonl"), minutes(90));
        write_at(&root.path().join("a/recent.jsonl"), minutes(1));
        write_at(&root.path().join("b/middle.jsonl"), minutes(30));
        write_at(&root.path().join("b/notes.txt"), minutes(0));

        let files =
            recent_files(root.path(), "jsonl", minutes(60), 2).expect("walk should succeed");

        assert_eq!(
            files
                .iter()
                .map(|file| file.path.clone())
                .collect::<Vec<_>>(),
            [
                root.path().join("a/recent.jsonl"),
                root.path().join("b/middle.jsonl")
            ]
        );
    }

    #[test]
    fn stops_at_the_depth_limit() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");
        let now = SystemTime::now();
        write_at(&root.path().join("project/session.jsonl"), now);
        write_at(
            &root.path().join("project/session/subagents/agent.jsonl"),
            now,
        );

        let shallow = recent_files(root.path(), "jsonl", now - Duration::from_secs(60), 2)
            .expect("walk should succeed");
        let deep = recent_files(root.path(), "jsonl", now - Duration::from_secs(60), 4)
            .expect("walk should succeed");

        assert_eq!(shallow.len(), 1);
        assert_eq!(deep.len(), 2);
    }

    #[test]
    fn a_missing_folder_has_no_file() {
        let root = tempfile::tempdir().expect("temporary folder should be writable");

        let files = recent_files(
            &root.path().join("absent"),
            "jsonl",
            SystemTime::UNIX_EPOCH,
            2,
        );

        assert!(files.is_ok_and(|files| files.is_empty()));
    }
}
