//! Where a CLI keeps its data: the user's home, plus the homes of the WSL
//! distributions that are already running. A stopped distribution is never
//! touched, since reading `\\wsl.localhost\<distro>` would start it.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Lists the WSL distributions currently running.
pub trait DistroLister: Send + Sync {
    fn running(&self) -> Vec<String>;
}

/// A shared lister, such as one [`CachedLister`] used by every source.
impl<T: DistroLister + ?Sized> DistroLister for Arc<T> {
    fn running(&self) -> Vec<String> {
        (**self).running()
    }
}

/// No WSL distribution: the host itself is the only root.
pub struct NoDistros;

impl DistroLister for NoDistros {
    fn running(&self) -> Vec<String> {
        Vec::new()
    }
}

/// How long [`CachedLister`] keeps the list of running distributions.
pub const DISTRO_LIST_TTL: Duration = Duration::from_secs(30);

/// Keeps the answer of another lister for [`DISTRO_LIST_TTL`], so `wsl.exe`
/// is not started on every refresh of every source. A distribution started
/// meanwhile is picked up at the next expiry.
pub struct CachedLister<L> {
    inner: L,
    clock: Box<dyn Fn() -> Instant + Send + Sync>,
    cached: Mutex<Option<(Instant, Vec<String>)>>,
}

impl<L: DistroLister> CachedLister<L> {
    pub fn new(inner: L) -> Self {
        Self::with_clock(inner, Instant::now)
    }

    /// Same as [`CachedLister::new`], reading the time from `clock`.
    pub fn with_clock(inner: L, clock: impl Fn() -> Instant + Send + Sync + 'static) -> Self {
        Self {
            inner,
            clock: Box::new(clock),
            cached: Mutex::new(None),
        }
    }
}

impl<L: DistroLister> DistroLister for CachedLister<L> {
    fn running(&self) -> Vec<String> {
        let now = (self.clock)();
        let mut cached = self.cached.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((listed_at, distros)) = cached.as_ref() {
            if now.saturating_duration_since(*listed_at) < DISTRO_LIST_TTL {
                return distros.clone();
            }
        }
        let distros = self.inner.running();
        *cached = Some((now, distros.clone()));
        distros
    }
}

/// Asks `wsl.exe --list --running --quiet`, which reports running
/// distributions without starting any.
#[cfg(windows)]
pub struct WslCli;

#[cfg(windows)]
impl DistroLister for WslCli {
    fn running(&self) -> Vec<String> {
        use std::os::windows::process::CommandExt;
        use std::process::Command;

        /// Keeps the console of `wsl.exe` from flashing on screen.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        match Command::new("wsl.exe")
            .args(["--list", "--running", "--quiet"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(output) if output.status.success() => decode_distro_list(&output.stdout),
            Ok(output) => {
                log::debug!("wsl.exe lists no running distribution ({})", output.status);
                Vec::new()
            }
            Err(error) => {
                log::debug!("wsl.exe unavailable, WSL homes skipped: {error}");
                Vec::new()
            }
        }
    }
}

/// Decodes the output of `wsl.exe --list`: UTF-16LE by default, UTF-8 when
/// `WSL_UTF8` is set.
#[cfg(any(windows, test))]
fn decode_distro_list(bytes: &[u8]) -> Vec<String> {
    let text = if bytes.contains(&0) {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes(*pair))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    text.lines()
        .map(|line| line.trim_matches(|c: char| c.is_whitespace() || c == '\0' || c == '\u{feff}'))
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Folders named `dir_name` in the user's home and in each home of the
/// running WSL distributions. Folders may not exist: callers skip them.
#[must_use]
pub fn home_roots(dir_name: &str, lister: &dyn DistroLister) -> Vec<PathBuf> {
    roots(user_home(), &lister.running(), dir_name, |distro| {
        PathBuf::from(format!(r"\\wsl.localhost\{distro}"))
    })
}

fn roots(
    home: Option<PathBuf>,
    distros: &[String],
    dir_name: &str,
    distro_root: impl Fn(&str) -> PathBuf,
) -> Vec<PathBuf> {
    let wsl_homes = distros
        .iter()
        .flat_map(|distro| user_dirs(&distro_root(distro).join("home")));
    home.into_iter()
        .chain(wsl_homes)
        .map(|home| home.join(dir_name))
        .collect()
}

fn user_dirs(homes: &Path) -> Vec<PathBuf> {
    match fs::read_dir(homes) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect(),
        Err(error) => {
            log::debug!("cannot list {}: {error}", homes.display());
            Vec::new()
        }
    }
}

/// Home of the current user: `USERPROFILE` on Windows, `HOME` elsewhere so
/// the code also runs on a Linux host.
fn user_home() -> Option<PathBuf> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(variable).map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::{
        decode_distro_list, home_roots, roots, CachedLister, DistroLister, NoDistros,
        DISTRO_LIST_TTL,
    };

    /// Counts its calls, standing for `wsl.exe`.
    struct CountingLister(Arc<AtomicUsize>);

    impl DistroLister for CountingLister {
        fn running(&self) -> Vec<String> {
            self.0.fetch_add(1, Ordering::Relaxed);
            vec!["Ubuntu".to_owned()]
        }
    }

    fn cached_with_clock() -> (
        CachedLister<CountingLister>,
        Arc<AtomicUsize>,
        Arc<Mutex<Instant>>,
    ) {
        let calls = Arc::new(AtomicUsize::new(0));
        let time = Arc::new(Mutex::new(Instant::now()));
        let clock = Arc::clone(&time);
        let lister = CachedLister::with_clock(CountingLister(Arc::clone(&calls)), move || {
            *clock.lock().expect("clock should be readable")
        });
        (lister, calls, time)
    }

    fn advance(time: &Mutex<Instant>, by: Duration) {
        let mut time = time.lock().expect("clock should be writable");
        *time += by;
    }

    #[test]
    fn lists_the_distributions_once_within_the_cache_delay() {
        let (lister, calls, time) = cached_with_clock();

        assert_eq!(lister.running(), ["Ubuntu"]);
        advance(&time, Duration::from_secs(29));
        assert_eq!(lister.running(), ["Ubuntu"]);

        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn lists_the_distributions_again_once_the_cache_expires() {
        let (lister, calls, time) = cached_with_clock();

        lister.running();
        advance(&time, DISTRO_LIST_TTL + Duration::from_secs(1));
        lister.running();

        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn adds_the_homes_of_running_distributions() {
        let wsl = tempfile::tempdir().expect("temporary folder should be writable");
        fs::create_dir_all(wsl.path().join("Ubuntu/home/thomas"))
            .expect("temporary folder should be writable");
        let home = PathBuf::from("C:/Users/Thomas");

        let found = roots(
            Some(home.clone()),
            &["Ubuntu".to_owned()],
            ".codex",
            |distro| wsl.path().join(distro),
        );

        assert_eq!(
            found,
            [
                home.join(".codex"),
                wsl.path().join("Ubuntu/home/thomas/.codex")
            ]
        );
    }

    #[test]
    fn only_the_user_home_without_running_distribution() {
        let home = PathBuf::from("C:/Users/Thomas");

        let found = roots(Some(home.clone()), &[], ".codex", |_| {
            unreachable!("no distribution to resolve")
        });

        assert_eq!(found, [home.join(".codex")]);
    }

    #[test]
    fn host_roots_are_the_current_home() {
        let found = home_roots(".codex", &NoDistros);

        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with(".codex"));
    }

    #[test]
    fn decodes_the_utf16_output_of_wsl() {
        let bytes: Vec<u8> = "Ubuntu\r\ndocker-desktop\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();

        assert_eq!(decode_distro_list(&bytes), ["Ubuntu", "docker-desktop"]);
    }

    #[test]
    fn decodes_the_utf8_output_of_wsl() {
        assert_eq!(decode_distro_list(b"Ubuntu\n\n"), ["Ubuntu"]);
    }

    #[test]
    fn decodes_an_empty_output_as_no_distribution() {
        assert_eq!(decode_distro_list(b""), Vec::<String>::new());
    }
}
