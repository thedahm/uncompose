//! Engine client: spawn the separation engine, feed it the request, stream
//! its JSONL events. The engine is only ever spawned, never imported.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use crate::contract::{EngineEvent, EngineRequest};
use crate::Cancelled;

/// SIGINT signal number (Linux). A Ctrl+C reaches the whole foreground
/// process group, so an engine killed by it means the user cancelled.
const SIGINT: i32 = 2;

/// Shown when the Engine Environment must be provisioned but uv is missing.
/// Linux-only install pointer, matching the ffmpeg preflight's shape.
const UV_MISSING: &str = "uv not found on your PATH. Uncompose builds its engine environment \
with uv on first run.
Install it per https://docs.astral.sh/uv/getting-started/installation/, for example:
  curl -LsSf https://astral.sh/uv/install.sh | sh
then run uncompose again.";

/// Resolve the engine interpreter, in order: `$UNCOMPOSE_ENGINE_PYTHON`
/// (tests, overrides), a dev checkout's `engine/.venv` found by walking up
/// from the current directory, and finally the runtime-provisioned Engine
/// Environment — built with uv on first use, which is the only path a
/// PyPI-installed uncompose ever takes.
pub fn resolve_engine_python(
    on_event: impl FnMut(crate::provision::ProvisionEvent),
) -> Result<PathBuf> {
    if let Ok(p) = std::env::var("UNCOMPOSE_ENGINE_PYTHON") {
        return Ok(PathBuf::from(p));
    }
    if let Some(dev) = dev_engine_python() {
        return Ok(dev);
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    let Some(uv) = crate::deps::find_on_path(&path, "uv") else {
        bail!("{UV_MISSING}");
    };
    crate::provision::ensure_engine_env(
        &uv,
        &crate::provision::default_engine_env_dir(),
        crate::provision::ENGINE_VERSION,
        on_event,
    )
}

/// The dev-managed interpreter (`uv sync` venv), when running from inside a
/// checkout: walk up from the current directory for `engine/.venv/bin/python`.
fn dev_engine_python() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("engine/.venv/bin/python");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Spawn the engine, write the request, and hand each stdout event to
/// `on_event`. Engine stderr is appended to `log_path`, never the progress
/// UI; a multi-step pipeline shares one `engine.log` so every call's chatter
/// lands in one diagnosable place. Returns an error on nonzero exit or a
/// malformed stream.
pub fn run_engine(
    python: &Path,
    request: &EngineRequest,
    log_path: &Path,
    mut on_event: impl FnMut(&EngineEvent),
) -> Result<()> {
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .context("opening engine.log")?;
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
        if status.signal() == Some(SIGINT) {
            return Err(anyhow::Error::new(Cancelled));
        }
        let tail = stderr_tail(log_path, 15);
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
