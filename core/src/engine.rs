//! Engine client: spawn the separation engine, feed it the request, stream
//! its JSONL events. The engine is only ever spawned, never imported.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::contract::{EngineEvent, EngineRequest};

/// Locate the dev-managed engine interpreter (M1: `uv sync` venv).
/// `$UNCOMPOSE_ENGINE_PYTHON` wins; otherwise walk up from the current
/// directory looking for `engine/.venv/bin/python`.
pub fn discover_engine_python() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("UNCOMPOSE_ENGINE_PYTHON") {
        return Ok(PathBuf::from(p));
    }
    let mut dir = std::env::current_dir().context("getting current dir")?;
    loop {
        let candidate = dir.join("engine/.venv/bin/python");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !dir.pop() {
            bail!(
                "engine interpreter not found: set UNCOMPOSE_ENGINE_PYTHON or run \
                 `uv sync` in engine/ and invoke from inside the repo"
            );
        }
    }
}

/// Spawn the engine, write the request, and hand each stdout event to
/// `on_event`. Engine stderr goes to `engine.log` in the job folder, never
/// the progress UI. Returns an error on nonzero exit or a malformed stream.
pub fn run_engine(
    python: &Path,
    request: &EngineRequest,
    job_folder: &Path,
    mut on_event: impl FnMut(&EngineEvent),
) -> Result<()> {
    let log = File::create(job_folder.join("engine.log")).context("creating engine.log")?;
    let mut command = Command::new(python);
    command.args(["-m", "uncompose_engine"]);
    // Device selection is core-owned; hiding the GPU before Python starts
    // is the one way to force CPU that no library can re-decide later.
    if request.device == "cpu" {
        command.env("CUDA_VISIBLE_DEVICES", "");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log))
        .spawn()
        .with_context(|| format!("spawning engine via {}", python.display()))?;

    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(serde_json::to_string(request)?.as_bytes())?;
    drop(stdin);

    let stdout = child.stdout.take().expect("piped stdout");
    let mut engine_error: Option<String> = None;
    for line in BufReader::new(stdout).lines() {
        let line = line.context("reading engine stdout")?;
        if line.trim().is_empty() {
            continue;
        }
        let event: EngineEvent = serde_json::from_str(&line)
            .with_context(|| format!("malformed engine event: {line}"))?;
        if let EngineEvent::Error { message } = &event {
            engine_error = Some(message.clone());
        }
        on_event(&event);
    }

    let status = child.wait().context("waiting for engine")?;
    if !status.success() {
        let tail = stderr_tail(&job_folder.join("engine.log"), 15);
        let msg = engine_error.unwrap_or_else(|| format!("engine exited with {status}"));
        bail!("{msg}\n--- engine.log tail ---\n{tail}");
    }
    Ok(())
}

fn stderr_tail(log_path: &Path, lines: usize) -> String {
    match std::fs::read_to_string(log_path) {
        Ok(s) => {
            let all: Vec<&str> = s.lines().collect();
            let start = all.len().saturating_sub(lines);
            all[start..].join("\n")
        }
        Err(_) => String::from("(no engine.log)"),
    }
}
