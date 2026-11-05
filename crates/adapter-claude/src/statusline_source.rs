//! Claude quotas copied by the `AgentHud` statusline script
//! (`scripts/statusline/`) from the official `rate_limits` Claude Code gives
//! its statusline. Read passively, like any other file.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::SystemTime;

use application::{SourceError, UsageSource};
use domain::{EpochSeconds, Origin, Provider, UsageSnapshot, UsageWindow, WindowKind};
use serde::Deserialize;
use serde_json::Value;

/// File written by the statusline script, under each `.claude` folder.
pub const RATE_LIMITS_FILE: &str = "agenthud/rate-limits.json";

/// Claude quotas from the statusline file of each root. A file is only read
/// again once its mtime changes.
pub struct ClaudeStatuslineSource<F> {
    roots: F,
    cache: Mutex<HashMap<PathBuf, Cached>>,
    /// Files read so far, to check the mtime cache.
    reads: AtomicUsize,
}

struct Cached {
    modified: SystemTime,
    snapshot: Option<UsageSnapshot>,
}

impl<F> ClaudeStatuslineSource<F>
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

    /// Statusline files read so far.
    pub fn reads(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }

    fn root_snapshot(
        &self,
        file: &Path,
        cache: &mut HashMap<PathBuf, Cached>,
    ) -> io::Result<Option<UsageSnapshot>> {
        let modified = match fs::metadata(file).and_then(|metadata| metadata.modified()) {
            Ok(modified) => modified,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                cache.remove(file);
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if let Some(cached) = cache.get(file) {
            if cached.modified == modified {
                return Ok(cached.snapshot.clone());
            }
        }
        self.reads.fetch_add(1, Ordering::Relaxed);
        let text = fs::read_to_string(file)?;
        let snapshot = parse(&text, adapter_fs::epoch_seconds(modified));
        cache.insert(
            file.to_path_buf(),
            Cached {
                modified,
                snapshot: snapshot.clone(),
            },
        );
        Ok(snapshot)
    }
}

impl<F> UsageSource for ClaudeStatuslineSource<F>
where
    F: Fn() -> Vec<PathBuf> + Send + Sync,
{
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    /// Most recent statusline quotas across all roots. An unreadable file is
    /// logged and skipped; its error is only returned when no root gave one.
    fn latest(&self) -> Result<Option<UsageSnapshot>, SourceError> {
        let files: Vec<PathBuf> = (self.roots)()
            .into_iter()
            .map(|root| root.join(RATE_LIMITS_FILE))
            .collect();
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.retain(|file, _| files.contains(file));

        let mut latest: Option<UsageSnapshot> = None;
        let mut first_error = None;
        for file in &files {
            match self.root_snapshot(file, &mut cache) {
                Ok(Some(snapshot)) => {
                    if latest
                        .as_ref()
                        .is_none_or(|best| snapshot.observed_at > best.observed_at)
                    {
                        latest = Some(snapshot);
                    }
                }
                Ok(None) => {}
                Err(source) => {
                    let error = SourceError::Io {
                        path: file.clone(),
                        source,
                    };
                    log::warn!("Claude statusline file skipped: {error}");
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

#[derive(Deserialize)]
struct StatuslineFile {
    #[serde(default)]
    rate_limits: Option<RateLimits>,
}

#[derive(Deserialize)]
struct RateLimits {
    #[serde(default)]
    five_hour: Option<Value>,
    #[serde(default)]
    seven_day: Option<Value>,
}

/// Quotas of a statusline file written at `written_at`, or `None` when it
/// holds no usable window. Every field is optional: an unknown window is
/// left out, never shown as 0 %.
fn parse(text: &str, written_at: EpochSeconds) -> Option<UsageSnapshot> {
    let text = text.trim_start_matches('\u{feff}');
    let file: StatuslineFile = serde_json::from_str(text).ok()?;
    let limits = file.rate_limits?;
    let five_hours = limits
        .five_hour
        .and_then(|window| to_window(&window, WindowKind::FiveHours));
    let weekly = limits
        .seven_day
        .and_then(|window| to_window(&window, WindowKind::Weekly));
    if five_hours.is_none() && weekly.is_none() {
        return None;
    }
    Some(UsageSnapshot {
        provider: Provider::Claude,
        five_hours,
        weekly,
        observed_at: written_at,
        origin: Origin::Statusline,
    })
}

fn to_window(window: &Value, kind: WindowKind) -> Option<UsageWindow> {
    let used = window.get("used_percentage")?.as_f64()?;
    let resets_at = window.get("resets_at").and_then(epoch);
    UsageWindow::new(kind, used, resets_at)
}

/// Seconds since the epoch, given as a number or as an RFC 3339 text.
pub(crate) fn epoch(value: &Value) -> Option<EpochSeconds> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text
            .parse()
            .ok()
            .or_else(|| adapter_fs::parse_utc_timestamp(text)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{epoch, parse};

    #[test]
    fn reads_epoch_seconds_as_numbers_or_texts() {
        assert_eq!(epoch(&json!(1_738_425_600)), Some(1_738_425_600));
        assert_eq!(epoch(&json!("1738425600")), Some(1_738_425_600));
        assert_eq!(epoch(&json!("1970-01-01T00:01:00Z")), Some(60));
        assert_eq!(epoch(&json!(null)), None);
        assert_eq!(epoch(&json!("soon")), None);
    }

    #[test]
    fn a_file_without_window_has_no_usage() {
        for text in [
            "",
            "not json",
            "{}",
            r#"{"rate_limits":null}"#,
            r#"{"rate_limits":{}}"#,
            r#"{"rate_limits":{"five_hour":{"used_percentage":"high"}}}"#,
            r#"{"rate_limits":{"five_hour":null,"seven_day":{"resets_at":1}}}"#,
        ] {
            assert_eq!(parse(text, 0), None, "{text}");
        }
    }
}
