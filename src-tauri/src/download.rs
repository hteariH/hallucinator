use std::path::{Path, PathBuf};

/// Where an unfinished download of `dest` lives.
pub fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}
