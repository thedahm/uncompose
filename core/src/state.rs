//! The last-job pointer: the only state that lives outside a Job Folder
//! (ADR-0003). A small file in the platform state dir records the most
//! recent *completed* job so `play`/`open` work right after a run without
//! the user restating the path.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Where the last-job pointer lives, keyed to a completed job on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastJob {
    /// The completed Job Folder (holds the stems and `job.json`).
    pub job_folder: PathBuf,
    /// The original input the job separated, for display.
    pub input_path: PathBuf,
}

/// Platform state dir: `$XDG_STATE_HOME/uncompose`, falling back to
/// `~/.local/state/uncompose`, then the current dir as a last resort so the
/// pointer always has a home.
pub fn default_state_dir() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("uncompose")
}

fn pointer_path(state_dir: &Path) -> PathBuf {
    state_dir.join("last-job.json")
}

/// Record `last` as the most recent job. Called only after `job.json` is
/// written, so the pointer never names an incomplete job.
pub fn write_last_job(state_dir: &Path, last: &LastJob) -> Result<()> {
    fs::create_dir_all(state_dir).context("creating state dir")?;
    let json = serde_json::to_string_pretty(last)?;
    fs::write(pointer_path(state_dir), json).context("writing last-job pointer")?;
    Ok(())
}

/// Read the last-job pointer, or `None` when no job has run yet.
pub fn read_last_job(state_dir: &Path) -> Result<Option<LastJob>> {
    match fs::read_to_string(pointer_path(state_dir)) {
        Ok(s) => Ok(Some(
            serde_json::from_str(&s).context("parsing last-job pointer")?,
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).context("reading last-job pointer"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_the_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let last = LastJob {
            job_folder: dir.path().join("song.stems"),
            input_path: dir.path().join("song.wav"),
        };
        write_last_job(dir.path(), &last).expect("write");
        assert_eq!(read_last_job(dir.path()).expect("read"), Some(last));
    }

    #[test]
    fn missing_pointer_reads_as_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(read_last_job(dir.path()).expect("read"), None);
    }

    #[test]
    fn write_creates_the_state_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state = dir.path().join("nested/state");
        let last = LastJob {
            job_folder: dir.path().join("a.stems"),
            input_path: dir.path().join("a.wav"),
        };
        write_last_job(&state, &last).expect("write");
        assert!(state.join("last-job.json").is_file());
    }
}
