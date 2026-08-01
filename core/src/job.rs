//! Job Folder lifecycle (ADR-0003): the filesystem is the database. The
//! folder is created next to the input, never overwritten, and `job.json`
//! written last is the completion marker.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Create `<input basename>.stems/` next to the input, suffixing `-2`, `-3`,
/// ... on collision rather than ever reusing an existing folder.
pub fn create_job_folder(input: &Path) -> Result<PathBuf> {
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input
        .file_stem()
        .context("input path has no file name")?
        .to_string_lossy();
    let mut n = 1u32;
    loop {
        let name = if n == 1 {
            format!("{stem}.stems")
        } else {
            format!("{stem}.stems-{n}")
        };
        let candidate = parent.join(name);
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e).context("creating job folder"),
        }
    }
}

/// The Job Record: everything needed to understand and rerun the job from
/// the folder alone. Written last; its presence marks the job complete.
#[derive(Debug, Serialize)]
pub struct JobRecord {
    pub input_path: String,
    pub input_sha256: String,
    /// The preset run (e.g. `6-stem`).
    pub preset: String,
    /// The model ids in pipeline order (one per engine call).
    pub models: Vec<String>,
    pub device: String,
    pub engine_version: String,
    pub stems: Vec<String>,
    /// Per-step timings, keyed by model id.
    pub timings: serde_json::Value,
    pub outcome: String,
    pub finished_at_unix: u64,
}

pub fn write_job_record(job_folder: &Path, record: &JobRecord) -> Result<()> {
    let json = serde_json::to_string_pretty(record)?;
    fs::write(job_folder.join("job.json"), json).context("writing job.json")?;
    Ok(())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).context("opening input for hashing")?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
