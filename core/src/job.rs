//! Job Folder lifecycle (ADR-0003): the filesystem is the database. The
//! folder is created next to the input, never overwritten, and `job.json`
//! written last is the completion marker.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// The default Job Folder path for an input: `<input basename>.stems/` next
/// to the input. An explicit `-o` override replaces it wholesale.
pub fn job_folder_base(input: &Path, output: Option<&Path>) -> Result<PathBuf> {
    if let Some(output) = output {
        return Ok(output.to_path_buf());
    }
    let parent = input.parent().unwrap_or(Path::new("."));
    let stem = input
        .file_stem()
        .context("input path has no file name")?
        .to_string_lossy();
    Ok(parent.join(format!("{stem}.stems")))
}

/// Create the Job Folder at `base`, suffixing `-2`, `-3`, ... on collision
/// rather than ever reusing (and overwriting) an existing folder. Parent
/// directories are created as needed so an `-o` path can point anywhere.
pub fn create_job_folder(base: &Path) -> Result<PathBuf> {
    let parent = base
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent).context("creating output parent directory")?;
    let base_name = base
        .file_name()
        .context("output path has no final component")?
        .to_string_lossy()
        .into_owned();
    let mut n = 1u32;
    loop {
        let name = if n == 1 {
            base_name.clone()
        } else {
            format!("{base_name}-{n}")
        };
        let candidate = parent.join(name);
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => n += 1,
            Err(e) => return Err(e).context("creating job folder"),
        }
    }
}

/// Promote a completed stem from its `.partial` staging name to its final
/// `<stem>.wav`. Engines stream stems as `<stem>.wav.partial`; a stem only
/// takes its final name once the core sees the stem event announcing it. A
/// no-op if the engine already wrote the final name directly.
pub fn promote_stem(job_folder: &Path, name: &str) -> Result<()> {
    let partial = job_folder.join(format!("{name}.wav.partial"));
    if partial.exists() {
        let final_path = job_folder.join(format!("{name}.wav"));
        fs::rename(&partial, &final_path).with_context(|| format!("promoting stem {name}"))?;
    }
    Ok(())
}

/// Remove every `*.partial` file left in the job folder. Used on cancel so a
/// Ctrl+C leaves no half-written junk behind. Best-effort by caller.
pub fn remove_partials(job_folder: &Path) -> Result<()> {
    for entry in fs::read_dir(job_folder).context("scanning job folder for partials")? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "partial") {
            fs::remove_file(&path)
                .with_context(|| format!("removing partial {}", path.display()))?;
        }
    }
    Ok(())
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
    /// Free-form separation parameters, recorded verbatim for reproducibility.
    pub parameters: serde_json::Value,
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
