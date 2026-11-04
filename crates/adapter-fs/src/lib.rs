//! File system access shared by provider adapters: mtime polling and
//! resolution of Windows and WSL (`\\wsl.localhost`, `\\wsl$`) paths.

mod newest;
mod recent;
mod roots;
mod tail;
mod timestamp;

pub use newest::{newest_file, DatedFile};
pub use recent::recent_files;
#[cfg(windows)]
pub use roots::WslCli;
pub use roots::{home_roots, CachedLister, DistroLister, NoDistros, DISTRO_LIST_TTL};
pub use tail::{find_last, read_tail_lines, DEFAULT_TAIL_BYTES, MAX_TAIL_BYTES};
pub use timestamp::{epoch_seconds, parse_utc_timestamp};
