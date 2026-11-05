//! Running Claude Code sessions: the CLI keeps one `sessions/<pid>.json`
//! marker per running session and deletes it when the session ends. The
//! format is internal and undocumented, so parsing is defensive: a marker
//! that cannot be read is skipped. Only `*.json` files are read, never the
//! `*.key` files beside them.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::time::SystemTime;

use adapter_fs::DatedFile;
use domain::TurnSignal;
use serde_json::Value;

use crate::agent_sessions::last_folder;

const SESSIONS_DIR: &str = "sessions";
const MARKER_EXTENSION: &str = "json";

/// Largest marker read; real ones are under 1 KiB.
const MARKER_MAX_BYTES: u64 = 64 * 1024;

/// One running session, as its marker describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningSession {
    /// Name of the session transcript, without extension.
    pub session_id: String,
    /// Last folder of the session `cwd`.
    pub project: Option<String>,
    /// `busy` starts a turn, `idle` ends it; `None` for any other status.
    pub signal: Option<TurnSignal>,
    /// Last write of the marker.
    pub modified: SystemTime,
}

/// Running sessions of the `.claude` folder `root`. Empty when the folder
/// holds no readable marker: the caller then falls back on the transcripts.
pub fn running_sessions(root: &Path) -> Vec<RunningSession> {
    let dir = root.join(SESSIONS_DIR);
    let markers = match adapter_fs::recent_files(&dir, MARKER_EXTENSION, SystemTime::UNIX_EPOCH, 1)
    {
        Ok(markers) => markers,
        Err(error) => {
            log::debug!("no Claude session marker in {}: {error}", dir.display());
            return Vec::new();
        }
    };
    markers.iter().filter_map(read_marker).collect()
}

fn read_marker(marker: &DatedFile) -> Option<RunningSession> {
    let read = || -> io::Result<String> {
        let mut text = String::new();
        File::open(&marker.path)?
            .take(MARKER_MAX_BYTES)
            .read_to_string(&mut text)?;
        Ok(text)
    };
    let session = match read() {
        Ok(text) => parse_marker(&text, marker.modified),
        Err(error) => {
            log::debug!(
                "Claude marker {} unreadable: {error}",
                marker.path.display()
            );
            return None;
        }
    };
    if session.is_none() {
        log::debug!("Claude marker {} skipped", marker.path.display());
    }
    session
}

/// Session of a marker. `None` when the marker is not a JSON object, has
/// no `sessionId`, or is a spare process that holds no conversation yet.
fn parse_marker(text: &str, modified: SystemTime) -> Option<RunningSession> {
    let marker: Value = serde_json::from_str(text).ok()?;
    let marker = marker.as_object()?;
    if marker.get("spare").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let session_id = marker
        .get("sessionId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())?;
    let project = marker
        .get("cwd")
        .and_then(Value::as_str)
        .and_then(last_folder);
    let signal = match marker.get("status").and_then(Value::as_str) {
        Some("busy") => Some(TurnSignal::TurnStarted),
        Some("idle") => Some(TurnSignal::TurnEnded),
        _ => None,
    };
    Some(RunningSession {
        session_id: session_id.to_owned(),
        project,
        signal,
        modified,
    })
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use domain::TurnSignal;

    use super::parse_marker;

    fn parse(text: &str) -> Option<super::RunningSession> {
        parse_marker(text, SystemTime::UNIX_EPOCH)
    }

    #[test]
    fn the_status_gives_the_turn_signal() {
        let signal = |status: &str| {
            parse(&format!(r#"{{"sessionId":"s","status":"{status}"}}"#))
                .and_then(|session| session.signal)
        };

        assert_eq!(signal("busy"), Some(TurnSignal::TurnStarted));
        assert_eq!(signal("idle"), Some(TurnSignal::TurnEnded));
        assert_eq!(signal("paused"), None);
    }

    #[test]
    fn the_project_is_the_last_folder_of_a_windows_or_unix_cwd() {
        let project = |cwd: &str| {
            parse(&format!(
                r#"{{"sessionId":"s","cwd":{}}}"#,
                serde_json::json!(cwd)
            ))
            .and_then(|session| session.project)
        };

        assert_eq!(
            project("/home/user/dev/project").as_deref(),
            Some("project")
        );
        assert_eq!(
            project(r"C:\Users\user\project").as_deref(),
            Some("project")
        );
    }

    #[test]
    fn a_marker_without_session_id_or_spare_is_skipped() {
        for text in [
            "not json",
            "[]",
            r#"{"status":"busy"}"#,
            r#"{"sessionId":"","status":"busy"}"#,
            r#"{"sessionId":"s","status":"idle","spare":true}"#,
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
    }
}
