//! System-dependency preflight checks. ffmpeg is a checked dependency
//! (issue #35): verbs that need it detect its absence up front and stop with
//! one clear install message rather than letting the engine fail deep inside
//! audio-separator with a cryptic stack trace.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

/// Shown when ffmpeg is missing. Linux-only install lines (v0.1 is Linux-only).
const FFMPEG_MISSING: &str =
    "ffmpeg not found on your PATH. Uncompose needs ffmpeg to read and write audio.
Install it with your system package manager, for example:
  Debian/Ubuntu:  sudo apt install ffmpeg
  Fedora:         sudo dnf install ffmpeg
  Arch:           sudo pacman -S ffmpeg
then run uncompose again.";

/// Ensure ffmpeg is available before starting a verb that needs it. Returns a
/// clear, actionable error when it is missing.
pub fn ensure_ffmpeg() -> Result<()> {
    match std::env::var_os("PATH") {
        Some(path) if find_on_path(&path, "ffmpeg").is_some() => Ok(()),
        _ => bail!("{FFMPEG_MISSING}"),
    }
}

/// Find an executable `name` in a PATH-style variable, if present.
fn find_on_path(path: &OsStr, name: &str) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|dir| dir.join(name))
        .find(|candidate| is_executable_file(candidate))
}

fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;

    fn write_executable(path: &Path) {
        std::fs::write(path, b"#!/bin/sh\n").expect("writing fake binary");
        let mut perms = std::fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).expect("chmod");
    }

    #[test]
    fn finds_an_executable_ffmpeg_on_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_executable(&dir.path().join("ffmpeg"));
        let path = OsString::from(dir.path());
        assert_eq!(
            find_on_path(&path, "ffmpeg"),
            Some(dir.path().join("ffmpeg"))
        );
    }

    #[test]
    fn ignores_a_non_executable_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("ffmpeg"), b"not executable").expect("write");
        let path = OsString::from(dir.path());
        assert_eq!(find_on_path(&path, "ffmpeg"), None);
    }

    #[test]
    fn reports_absence_when_no_directory_has_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = OsString::from(dir.path());
        assert_eq!(find_on_path(&path, "ffmpeg"), None);
    }
}
