//! Starts `codex app-server` for one exchange, then stops it. Candidates: the
//! Codex of the host first, then the Codex of each WSL distribution already
//! running. A stopped distribution is never started (ADR 0002).

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use adapter_fs::DistroLister;
use application::{FetchError, UsageFetcher};
use domain::{EpochSeconds, Provider, UsageSnapshot};

use crate::app_server;

/// Longest exchange, start included; app-server requests time out at ~10 s.
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(15);

/// Time given to the server to exit once its stdin closes, before it is
/// killed.
const EXIT_GRACE: Duration = Duration::from_secs(2);

/// Executable names of Codex in the `PATH`, in order of preference.
#[cfg(windows)]
const CODEX_NAMES: [&str; 2] = ["codex.exe", "codex.cmd"];
#[cfg(not(windows))]
const CODEX_NAMES: [&str; 1] = ["codex"];

/// One way to start `codex app-server`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// The Codex executable of the host.
    Host(PathBuf),
    /// The Codex of a running WSL distribution, through `wsl.exe`.
    Wsl(String),
}

impl Launch {
    /// Command starting the server. WSL goes through a login shell, so the
    /// `PATH` of `~/.profile` (where Codex usually lives) applies.
    fn command(&self) -> Command {
        let command = match self {
            Self::Host(codex) => {
                let mut command = Command::new(codex);
                command.arg("app-server");
                command
            }
            Self::Wsl(distro) => {
                let mut command = Command::new("wsl.exe");
                command.args(["-d", distro, "--exec", "sh", "-lc", "exec codex app-server"]);
                command
            }
        };
        #[cfg(windows)]
        let command = {
            use std::os::windows::process::CommandExt;
            /// Keeps a console from flashing on screen.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            let mut command = command;
            command.creation_flags(CREATE_NO_WINDOW);
            command
        };
        command
    }
}

/// Candidates in order: the host Codex, then each running distribution.
/// WSL only exists on Windows.
#[must_use]
pub fn candidates(host_codex: Option<PathBuf>, running_distros: &[String]) -> Vec<Launch> {
    let wsl = running_distros
        .iter()
        .filter(|_| cfg!(windows))
        .map(|distro| Launch::Wsl(distro.clone()));
    host_codex
        .map(Launch::Host)
        .into_iter()
        .chain(wsl)
        .collect()
}

/// First Codex executable found in the folders of `path` (a `PATH` value).
#[must_use]
pub fn find_codex(path: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    let folders: Vec<PathBuf> = path.map(|path| std::env::split_paths(path).collect())?;
    CODEX_NAMES.iter().find_map(|name| {
        folders
            .iter()
            .map(|folder| folder.join(name))
            .find(|candidate| candidate.is_file())
    })
}

/// Codex quotas from `codex app-server`, started on demand.
pub struct CodexAppServer<L> {
    distros: L,
}

impl<L: DistroLister> CodexAppServer<L> {
    /// `distros` lists the running WSL distributions; share the cached one.
    pub fn new(distros: L) -> Self {
        Self { distros }
    }
}

impl<L: DistroLister> UsageFetcher for CodexAppServer<L> {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    /// The first candidate that answers. An expired login stops the search:
    /// every candidate shares the account.
    fn fetch(&self, now: EpochSeconds) -> Result<UsageSnapshot, FetchError> {
        let host = find_codex(std::env::var_os("PATH").as_deref());
        let mut last_error = FetchError::Unavailable("Codex not found".to_owned());
        for launch in candidates(host, &self.distros.running()) {
            match exchange(launch.command(), now) {
                Ok(snapshot) => return Ok(snapshot),
                Err(FetchError::AuthExpired) => return Err(FetchError::AuthExpired),
                Err(error) => {
                    log::debug!("codex app-server via {launch:?} failed: {error}");
                    last_error = error;
                }
            }
        }
        Err(last_error)
    }
}

/// Starts `command` as an app-server, reads the rate limits, and stops it.
///
/// # Errors
///
/// Fails when the server cannot start or does not answer in time.
pub fn exchange(mut command: Command, now: EpochSeconds) -> Result<UsageSnapshot, FetchError> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            FetchError::Unavailable(format!("cannot start codex app-server: {error}"))
        })?;
    let deadline = Instant::now() + EXCHANGE_TIMEOUT;
    let result = match (child.stdin.take(), child.stdout.take()) {
        (Some(mut stdin), Some(stdout)) => {
            let (sender, lines) = mpsc::channel();
            // Ends by itself once the server's stdout closes.
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if sender.send(line).is_err() {
                        break;
                    }
                }
            });
            let result = app_server::read_rate_limits(&mut stdin, &lines, deadline, now);
            drop(stdin);
            result
        }
        _ => Err(FetchError::Unavailable(
            "codex app-server has no stdio".to_owned(),
        )),
    };
    stop(&mut child);
    result
}

/// Waits briefly for the server to exit on its closed stdin, then kills it,
/// so a shim (`codex.cmd`, `wsl.exe`) leaves no server behind.
fn stop(child: &mut Child) {
    let until = Instant::now() + EXIT_GRACE;
    while Instant::now() < until {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                log::debug!("cannot wait for codex app-server: {error}");
                break;
            }
        }
    }
    if let Err(error) = child.kill() {
        log::debug!("cannot stop codex app-server: {error}");
    }
    if let Err(error) = child.wait() {
        log::debug!("cannot reap codex app-server: {error}");
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{candidates, find_codex, Launch, CODEX_NAMES};

    #[test]
    fn the_host_codex_comes_first() {
        let host = PathBuf::from("codex");

        let found = candidates(Some(host.clone()), &["Ubuntu".to_owned()]);

        assert_eq!(found.first(), Some(&Launch::Host(host)));
        let wsl = found
            .iter()
            .filter(|launch| matches!(launch, Launch::Wsl(_)))
            .count();
        assert_eq!(wsl, usize::from(cfg!(windows)));
    }

    #[test]
    fn only_running_distributions_are_candidates() {
        let found = candidates(None, &[]);

        assert_eq!(found, []);
    }

    #[test]
    fn finds_codex_in_the_path_in_folder_order() {
        let empty = tempfile::tempdir().expect("temporary folder should be writable");
        let first = tempfile::tempdir().expect("temporary folder should be writable");
        let second = tempfile::tempdir().expect("temporary folder should be writable");
        for folder in [&first, &second] {
            fs::write(folder.path().join(CODEX_NAMES[0]), "").expect("file should be writable");
        }
        let path = std::env::join_paths([empty.path(), first.path(), second.path()])
            .expect("paths should join");

        let found = find_codex(Some(&path)).expect("codex should be found");

        assert_eq!(found, first.path().join(CODEX_NAMES[0]));
        assert_eq!(find_codex(Some(empty.path().as_os_str())), None);
        assert_eq!(find_codex(None), None);
    }
}
