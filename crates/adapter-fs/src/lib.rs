//! File system access shared by provider adapters: mtime polling and
//! resolution of Windows and WSL (`\\wsl.localhost`, `\\wsl$`) paths.

mod newest;
mod roots;
mod tail;

pub use newest::{newest_file, DatedFile};
#[cfg(windows)]
pub use roots::WslCli;
pub use roots::{home_roots, DistroLister, NoDistros};
pub use tail::{find_last, read_tail_lines, DEFAULT_TAIL_BYTES, MAX_TAIL_BYTES};
