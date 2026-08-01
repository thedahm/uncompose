//! Last-job pointer: the one bit of state that lives outside a Job Folder
//! (ADR-0003). A platformdirs state file holding the absolute path of the
//! most recent job, so `play` and `open` have something to resolve against.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// `$XDG_STATE_HOME/uncompose` (default `~/.local/state/uncompose`).
pub fn state_dir() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("uncompose")
}

fn last_job_pointer() -> PathBuf {
    state_dir().join("last-job")
}

/// Record `job_folder` as the most recent job. Creates the state dir if
/// needed; the pointer is a single line holding the folder's path.
pub fn write_last_job(job_folder: &Path) -> Result<()> {
    let dir = state_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("creating state dir {}", dir.display()))?;
    std::fs::write(last_job_pointer(), job_folder.to_string_lossy().as_bytes())
        .context("writing last-job pointer")?;
    Ok(())
}

/// The most recent job folder, or a clear error pointing at `separate` when
/// no job has been run yet.
pub fn read_last_job() -> Result<PathBuf> {
    let pointer = last_job_pointer();
    let raw = std::fs::read_to_string(&pointer)
        .map_err(|_| no_previous_job())
        .and_then(|s| {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                Err(no_previous_job())
            } else {
                Ok(trimmed)
            }
        })?;
    Ok(PathBuf::from(raw))
}

fn no_previous_job() -> anyhow::Error {
    anyhow::anyhow!("no previous job found; run `uncompose separate <song>` first")
}
