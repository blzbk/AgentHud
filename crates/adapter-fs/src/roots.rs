//! Where a CLI keeps its data: the user's home, plus the homes of the WSL
//! distributions that are already running. A stopped distribution is never
//! touched, since reading `\\wsl.localhost\<distro>` would start it.

use std::fs;
use std::path::{Path, PathBuf};

/// Lists the WSL distributions currently running.
pub trait DistroLister: Send + Sync {
    fn running(&self) -> Vec<String>;
}

/// No WSL distribution: the host itself is the only root.
pub struct NoDistros;

impl DistroLister for NoDistros {
    fn running(&self) -> Vec<String> {
        Vec::new()
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

    use super::{decode_distro_list, home_roots, roots, NoDistros};

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
