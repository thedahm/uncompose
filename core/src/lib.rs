//! uncompose-core: preset/model knowledge, job lifecycle, and the engine
//! client behind the Engine Contract (ADR-0001, ADR-0003). No CLI or
//! terminal code lives here; `run_job` streams typed events to its caller.

pub mod contract;
pub mod engine;
pub mod job;
pub mod state;

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use contract::{EngineEvent, EngineRequest};
use job::JobRecord;

/// What the caller wants run. The walking skeleton knows exactly one model.
pub struct JobConfig {
    pub input: PathBuf,
    /// Preset name recorded in the job record (e.g. "6-stem"). The core owns
    /// the preset-to-model mapping; here we only record what the caller chose.
    pub preset: String,
    pub model_id: String,
    /// Free-form separation parameters, recorded verbatim for reproducibility.
    pub parameters: serde_json::Value,
    pub device: String,
    pub model_dir: PathBuf,
    /// Where the last-job pointer is written after a successful run.
    pub state_dir: PathBuf,
    pub engine_python: PathBuf,
}

#[derive(Debug)]
pub struct JobOutcome {
    pub job_folder: PathBuf,
    pub stems: Vec<String>,
}

/// Progress events surfaced to the interface layer.
#[derive(Debug)]
pub enum JobEvent {
    Stage {
        stage: String,
        percent: Option<f64>,
        message: Option<String>,
    },
    Stem {
        name: String,
    },
}

/// Run one separation job in the foreground: create the Job Folder, spawn
/// the engine, stream events, and write `job.json` last as the completion
/// marker. On failure the folder is left as the diagnosable artifact
/// (engine.log, any partial output, no job.json).
pub fn run_job(config: &JobConfig, mut on_event: impl FnMut(JobEvent)) -> Result<JobOutcome> {
    let input = config
        .input
        .canonicalize()
        .with_context(|| format!("input not found: {}", config.input.display()))?;
    let job_folder = job::create_job_folder(&input)?;
    std::fs::create_dir_all(&config.model_dir).context("creating model dir")?;

    let request = EngineRequest {
        audio_path: input.to_string_lossy().into_owned(),
        model_id: config.model_id.clone(),
        output_dir: job_folder.to_string_lossy().into_owned(),
        model_dir: config.model_dir.to_string_lossy().into_owned(),
        device: config.device.clone(),
    };

    let mut stems: Vec<String> = Vec::new();
    let mut done: Option<(String, String, serde_json::Value)> = None;
    engine::run_engine(
        &config.engine_python,
        &request,
        &job_folder,
        |event| match event {
            EngineEvent::Stage {
                stage,
                percent,
                message,
            } => on_event(JobEvent::Stage {
                stage: stage.clone(),
                percent: *percent,
                message: message.clone(),
            }),
            EngineEvent::Stem { name, .. } => {
                stems.push(name.clone());
                on_event(JobEvent::Stem { name: name.clone() });
            }
            EngineEvent::Done {
                engine_version,
                device,
                timings,
                ..
            } => done = Some((engine_version.clone(), device.clone(), timings.clone())),
            EngineEvent::Error { .. } => {}
        },
    )?;

    let (engine_version, device, timings) = done.context("engine exited 0 without a done event")?;
    let record = JobRecord {
        input_path: input.to_string_lossy().into_owned(),
        input_sha256: job::sha256_file(&input)?,
        preset: config.preset.clone(),
        model_id: config.model_id.clone(),
        parameters: config.parameters.clone(),
        device,
        engine_version,
        stems: stems.clone(),
        timings,
        outcome: "success".into(),
        finished_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    // job.json written last: it is the completion marker. Only once it exists
    // do we point the last-job pointer at this now-complete folder.
    job::write_job_record(&job_folder, &record)?;
    state::write_last_job(
        &config.state_dir,
        &state::LastJob {
            job_folder: job_folder.clone(),
            input_path: input.clone(),
        },
    )?;

    Ok(JobOutcome { job_folder, stems })
}

/// Default model weights directory: `~/.cache/uncompose/models` (XDG).
pub fn default_model_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("uncompose/models")
}
