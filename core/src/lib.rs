//! uncompose-core: preset/model knowledge, job lifecycle, and the engine
//! client behind the Engine Contract (ADR-0001, ADR-0003). No CLI or
//! terminal code lives here; `run_job` streams typed events to its caller.

pub mod contract;
pub mod engine;
pub mod job;
pub mod preset;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};

use contract::{EngineEvent, EngineRequest};
use job::JobRecord;
use preset::{Preset, Route, StepInput};

/// What the caller wants run: an input, a preset, and where the engine and
/// its weights live.
pub struct JobConfig {
    pub input: PathBuf,
    /// The preset to run; owns the pipeline of engine calls.
    pub preset: &'static Preset,
    /// "auto" | "cpu" | "cuda". `auto` resolves against the local machine.
    pub device: String,
    pub model_dir: PathBuf,
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

/// Run one separation job in the foreground: create the Job Folder, run the
/// preset's pipeline (one engine call per step, composed by the core),
/// stream events, and write `job.json` last as the completion marker.
///
/// Each step runs in a hidden `.stage-N/` scratch dir inside the job folder;
/// intermediates (e.g. the 6-stem instrumental) live there while the
/// pipeline runs. On full success the final stems are moved into the job
/// folder and the scratch dirs — intermediates and discards with them — are
/// removed. On failure the folder is left as the diagnosable artifact
/// (engine.log, partials in the scratch dirs, no job.json).
pub fn run_job(config: &JobConfig, mut on_event: impl FnMut(JobEvent)) -> Result<JobOutcome> {
    let input = config
        .input
        .canonicalize()
        .with_context(|| format!("input not found: {}", config.input.display()))?;
    let device = resolve_device(&config.device)?;
    let preset = config.preset;

    let job_folder = job::create_job_folder(&input)?;
    std::fs::create_dir_all(&config.model_dir).context("creating model dir")?;
    let log_path = job_folder.join("engine.log");

    // Named intermediates a step produced, for a later step's input.
    let mut intermediates: HashMap<&'static str, PathBuf> = HashMap::new();
    // (final stem name, its current path in a scratch dir) to move on success.
    let mut to_finalize: Vec<(&'static str, PathBuf)> = Vec::new();
    let mut stage_dirs: Vec<PathBuf> = Vec::new();
    let mut engine_version: Option<String> = None;
    let mut timings = serde_json::Map::new();

    for (i, step) in preset.steps.iter().enumerate() {
        let stage_dir = job_folder.join(format!(".stage-{}", i + 1));
        std::fs::create_dir_all(&stage_dir).context("creating stage dir")?;
        stage_dirs.push(stage_dir.clone());

        let step_input = match step.input {
            StepInput::Song => input.clone(),
            StepInput::Intermediate(name) => {
                intermediates.get(name).cloned().with_context(|| {
                    format!("pipeline wants intermediate '{name}' no earlier step produced")
                })?
            }
        };

        let request = EngineRequest {
            audio_path: step_input.to_string_lossy().into_owned(),
            model_id: step.model.id.to_string(),
            output_dir: stage_dir.to_string_lossy().into_owned(),
            model_dir: config.model_dir.to_string_lossy().into_owned(),
            device: device.clone(),
        };

        let mut emitted: Vec<(String, PathBuf)> = Vec::new();
        let mut done: Option<(String, serde_json::Value)> = None;
        engine::run_engine(
            &config.engine_python,
            &request,
            &log_path,
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
                EngineEvent::Stem { name, path } => {
                    emitted.push((name.clone(), PathBuf::from(path)))
                }
                EngineEvent::Done {
                    engine_version,
                    timings,
                    ..
                } => done = Some((engine_version.clone(), timings.clone())),
                EngineEvent::Error { .. } => {}
            },
        )?;

        let (version, step_timings) = done.with_context(|| {
            format!(
                "engine step {} exited 0 without a done event",
                step.model.id
            )
        })?;
        engine_version = Some(version);
        timings.insert(step.model.id.to_string(), step_timings);

        // Route each emitted stem per the step's static plan.
        for (engine_name, path) in emitted {
            let output = step
                .outputs
                .iter()
                .find(|o| o.engine_name == engine_name.as_str())
                .with_context(|| {
                    format!(
                        "engine emitted unplanned stem '{engine_name}' for model {}",
                        step.model.id
                    )
                })?;
            match output.route {
                Route::Stem(final_name) => {
                    to_finalize.push((final_name, path));
                    on_event(JobEvent::Stem {
                        name: final_name.to_string(),
                    });
                }
                Route::Intermediate(name) => {
                    intermediates.insert(name, path);
                }
                Route::Discard => {}
            }
        }
    }

    // Every step succeeded: promote the final stems into the job folder,
    // confirm the preset delivered on its promise, then clear the scratch.
    for (final_name, path) in &to_finalize {
        let dest = job_folder.join(format!("{final_name}.wav"));
        std::fs::rename(path, &dest).with_context(|| format!("finalizing stem {final_name}"))?;
    }
    for stem in preset.stems {
        if !job_folder.join(format!("{stem}.wav")).is_file() {
            bail!("pipeline did not produce promised stem '{stem}'");
        }
    }
    for dir in &stage_dirs {
        // Intermediates and discards are deleted on success; ignore a
        // missing dir (a step may not have created one).
        let _ = std::fs::remove_dir_all(dir);
    }

    let stems: Vec<String> = preset.stems.iter().map(|s| s.to_string()).collect();
    let record = JobRecord {
        input_path: input.to_string_lossy().into_owned(),
        input_sha256: job::sha256_file(&input)?,
        preset: preset.name.to_string(),
        models: preset
            .steps
            .iter()
            .map(|s| s.model.id.to_string())
            .collect(),
        device,
        engine_version: engine_version.unwrap_or_default(),
        stems: stems.clone(),
        timings: serde_json::Value::Object(timings),
        outcome: "success".into(),
        finished_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    job::write_job_record(&job_folder, &record)?;

    Ok(JobOutcome { job_folder, stems })
}

/// Resolve the requested device to a concrete `cpu`/`cuda` the engine runs.
/// Device selection is core-owned; a preset never silently substitutes a
/// model, so `auto` only chooses where to run, never what.
pub fn resolve_device(requested: &str) -> Result<String> {
    match requested {
        "cpu" => Ok("cpu".into()),
        "cuda" => Ok("cuda".into()),
        "auto" => Ok(if cuda_available() { "cuda" } else { "cpu" }.into()),
        other => bail!("unknown device '{other}': expected auto, cpu, or cuda"),
    }
}

/// Best-effort CUDA detection that never loads torch: sniff for the NVIDIA
/// driver the way a non-Python core can. A false negative just means a CPU
/// fallback the user can override with `--device cuda`.
fn cuda_available() -> bool {
    Path::new("/proc/driver/nvidia/version").exists() || Path::new("/dev/nvidia0").exists()
}

/// Default model weights directory: `~/.cache/uncompose/models` (XDG).
pub fn default_model_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("uncompose/models")
}
