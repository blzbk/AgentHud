//! Reads the end of a file, so the last event of a large session is found
//! without loading the whole session.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Size of the first window read from the end of the file.
pub const DEFAULT_TAIL_BYTES: u64 = 64 * 1024;

/// Largest window read from the end of the file.
pub const MAX_TAIL_BYTES: u64 = 1024 * 1024;

/// Complete lines among the last `max_bytes` of the file, last line first.
///
/// A line cut by the start of the window, or still being written (no final
/// newline), is left out.
///
/// # Errors
///
/// Fails when the file cannot be opened or read.
pub fn read_tail_lines(path: &Path, max_bytes: u64) -> io::Result<Vec<String>> {
    Ok(read_tail(path, max_bytes)?.lines)
}

/// Walks the lines from the end of the file and returns the first one `find`
/// accepts. The window starts at [`DEFAULT_TAIL_BYTES`] and doubles, up to
/// [`MAX_TAIL_BYTES`], while no line is accepted.
///
/// # Errors
///
/// Fails when the file cannot be opened or read.
pub fn find_last<T>(path: &Path, mut find: impl FnMut(&str) -> Option<T>) -> io::Result<Option<T>> {
    let mut window = DEFAULT_TAIL_BYTES;
    loop {
        let tail = read_tail(path, window)?;
        if let Some(found) = tail.lines.iter().find_map(|line| find(line)) {
            return Ok(Some(found));
        }
        if tail.whole_file || window >= MAX_TAIL_BYTES {
            return Ok(None);
        }
        window = (window * 2).min(MAX_TAIL_BYTES);
    }
}

struct Tail {
    lines: Vec<String>,
    whole_file: bool,
}

fn read_tail(path: &Path, max_bytes: u64) -> io::Result<Tail> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let start = size.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(max_bytes).read_to_end(&mut bytes)?;

    let mut segments: Vec<&[u8]> = bytes.split(|&byte| byte == b'\n').collect();
    // The last segment follows the final newline: empty, or a line still
    // being written.
    segments.pop();
    if start > 0 && !segments.is_empty() {
        segments.remove(0);
    }
    let lines = segments
        .into_iter()
        .rev()
        .map(|line| String::from_utf8_lossy(line.strip_suffix(b"\r").unwrap_or(line)).into_owned())
        .collect();
    Ok(Tail {
        lines,
        whole_file: start == 0,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::{find_last, read_tail_lines, DEFAULT_TAIL_BYTES};

    fn file_with(content: &str) -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("temporary folder should be writable");
        let path = dir.path().join("session.jsonl");
        fs::write(&path, content).expect("file should be writable");
        (dir, path)
    }

    fn numbered_lines(count: usize) -> String {
        (0..count)
            .map(|n| format!("line {n:06}\n"))
            .collect::<Vec<_>>()
            .concat()
    }

    #[test]
    fn returns_only_complete_lines_of_the_end_last_first() {
        let content = numbered_lines(10_000);
        assert!(content.len() as u64 > DEFAULT_TAIL_BYTES);
        let (_dir, path) = file_with(&content);

        let lines = read_tail_lines(&path, DEFAULT_TAIL_BYTES).expect("tail should be readable");

        assert_eq!(lines[0], "line 009999");
        assert_eq!(lines[1], "line 009998");
        let expected: Vec<String> = content
            .lines()
            .rev()
            .take(lines.len())
            .map(str::to_owned)
            .collect();
        assert_eq!(lines, expected);
        assert!(
            lines.len() * "line 000000\n".len()
                <= usize::try_from(DEFAULT_TAIL_BYTES).expect("fits")
        );
    }

    #[test]
    fn reads_every_line_of_a_small_file() {
        let (_dir, path) = file_with("first\nsecond\n");

        let lines = read_tail_lines(&path, DEFAULT_TAIL_BYTES).expect("tail should be readable");

        assert_eq!(lines, ["second", "first"]);
    }

    #[test]
    fn leaves_out_a_line_still_being_written() {
        let (_dir, path) = file_with("first\nsecond\n{\"partial\":");

        let lines = read_tail_lines(&path, DEFAULT_TAIL_BYTES).expect("tail should be readable");

        assert_eq!(lines, ["second", "first"]);
    }

    #[test]
    fn widens_the_window_until_a_line_is_found() {
        let content = format!("wanted\n{}", numbered_lines(20_000));
        let (_dir, path) = file_with(&content);

        let found = find_last(&path, |line| (line == "wanted").then_some(line.to_owned()));

        assert_eq!(
            found.expect("tail should be readable").as_deref(),
            Some("wanted")
        );
    }

    #[test]
    fn finds_nothing_when_no_line_matches() {
        let (_dir, path) = file_with(&numbered_lines(10));

        let found = find_last(&path, |line| (line == "wanted").then_some(()));

        assert_eq!(found.expect("tail should be readable"), None);
    }
}
