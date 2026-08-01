//! Runtime provisioning of the Engine Environment (issue #36, ADR-0003):
//! on a machine with no dev checkout, the core builds the Python engine
//! environment itself by shelling out to `uv`, installing the pinned
//! `uncompose-engine` release. uv is the only tool invoked; its output is
//! left on the caller's stderr so download progress stays visible.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

/// The engine release the core provisions: versions move in lockstep, so the
/// pin is this crate's own version (workspace-versioned with the product).
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default Engine Environment location: `~/.local/share/uncompose/engine/<version>`
/// (XDG data, not cache — losing it means a multi-GB reinstall). The version
/// segment makes an upgrade provision cleanly beside the old env instead of
/// mutating it midway.
pub fn default_engine_env_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("uncompose/engine").join(ENGINE_VERSION)
}

/// Progress surfaced while provisioning, for the interface layer to print.
#[derive(Debug)]
pub enum ProvisionEvent {
    /// Provisioning is about to start; the slow, one-time part begins.
    Started { env_dir: PathBuf },
    /// One uv step is starting (venv creation, engine install).
    Step { description: String },
}

/// Ensure the Engine Environment at `env_dir` exists and holds the pinned
/// engine, building it with `uv` if needed. Returns the environment's
/// interpreter path.
pub fn ensure_engine_env(
    uv: &Path,
    env_dir: &Path,
    engine_version: &str,
    mut on_event: impl FnMut(ProvisionEvent),
) -> Result<PathBuf> {
    let python = env_dir.join("bin/python");
    // The marker is written last, so its presence means a complete env —
    // the same completion-marker rule job.json follows for Job Folders. A
    // directory without it is a crashed provision and is rebuilt from
    // scratch.
    let marker = env_dir.join(".provisioned");
    if python.is_file() && marker_matches(&marker, engine_version) {
        return Ok(python);
    }
    if env_dir.exists() {
        std::fs::remove_dir_all(env_dir).context("removing partial engine environment")?;
    }
    on_event(ProvisionEvent::Started {
        env_dir: env_dir.to_path_buf(),
    });

    on_event(ProvisionEvent::Step {
        description: "creating the engine environment".into(),
    });
    // The interpreter pin matches the engine's `requires-python`
    // (engine/pyproject.toml); uv fetches a managed CPython when the host
    // has none, which is exactly the clean-machine case.
    run_uv(
        uv,
        Command::new(uv)
            .args(["venv", "--python", "3.12"])
            .arg(env_dir),
        "creating the engine environment",
    )?;

    let requirement = format!("uncompose-engine=={engine_version}");
    on_event(ProvisionEvent::Step {
        description: format!("installing {requirement}"),
    });
    run_uv(
        uv,
        Command::new(uv)
            .args(["pip", "install", "--python"])
            .arg(&python)
            .arg(&requirement),
        "installing the engine",
    )?;

    std::fs::write(&marker, engine_version).context("writing provision marker")?;
    Ok(python)
}

/// True when the marker records exactly the engine version we want; a
/// mismatch (older env after an upgrade) triggers a rebuild.
fn marker_matches(marker: &Path, engine_version: &str) -> bool {
    std::fs::read_to_string(marker)
        .map(|v| v.trim() == engine_version)
        .unwrap_or(false)
}

/// Run one uv invocation, turning a nonzero exit into a clear error. uv's
/// own stderr passes through: its progress output is the download UI.
fn run_uv(uv: &Path, command: &mut Command, what: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("running {} while {what}", uv.display()))?;
    if !status.success() {
        bail!("uv failed while {what} (exit: {status})");
    }
    Ok(())
}
