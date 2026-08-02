//! End-to-end provisioning on a "stranger's machine" (issue #36): no dev
//! checkout, no `UNCOMPOSE_ENGINE_PYTHON` — the CLI must build the Engine
//! Environment by shelling out to uv, then run the job with the interpreter
//! it provisioned. A fake `uv` on PATH "installs" the fake engine as the
//! environment's python, so the whole flow runs with no network or torch.

#[path = "../../core/tests/support/mod.rs"]
mod support;

#[path = "../../core/tests/support/fake_uv.rs"]
mod fake_uv;

#[path = "../../core/tests/support/weights.rs"]
mod weights;

use std::path::{Path, PathBuf};
use std::process::Command;

use fake_uv::{uv_log, write_executable, write_fake_uv};

/// A hermetic PATH dir holding a stub ffmpeg and a fake uv whose `venv`
/// verb installs the fake engine as the new environment's python.
fn bin_dir(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("creating bin dir");
    write_executable(&bin.join("ffmpeg"), "#!/bin/sh\n");
    write_fake_uv(&bin, dir, Some(&support::fake_engine()));
    bin
}

fn uncompose(dir: &Path, input_name: &str) -> Command {
    let input = dir.join(input_name);
    std::fs::write(&input, b"not really audio").expect("writing input");
    // Warm weight cache: the auto-fetch trusts presence and stays offline.
    weights::seed_weights(&dir.join("cache"));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_uncompose"));
    cmd.args(["separate", input.to_str().expect("utf8 path")])
        .args(["--device", "cpu"])
        // No UNCOMPOSE_ENGINE_PYTHON, and a cwd outside any checkout: the
        // dev-venv walk-up must find nothing, leaving only provisioning.
        .env_remove("UNCOMPOSE_ENGINE_PYTHON")
        .current_dir(dir)
        .env("PATH", bin_dir(dir))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_DATA_HOME", dir.join("data"));
    cmd
}

#[test]
fn first_run_provisions_the_engine_environment_and_separates() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = uncompose(dir.path(), "song.wav")
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // uv built the env: a venv call, then an install of the pinned engine
    // release (the pin is the product's own version).
    let log = uv_log(dir.path());
    let pin = format!("uncompose-engine=={}", env!("CARGO_PKG_VERSION"));
    assert!(
        log.lines().any(|l| l.starts_with("venv")),
        "venv call: {log}"
    );
    assert!(log.contains(&pin), "pinned engine install: {log}");

    // The one-time step is announced before the run so the multi-GB
    // download never looks like a hang.
    assert!(
        stdout.contains("engine environment"),
        "provisioning announced: {stdout}"
    );

    // The provisioned interpreter (the fake engine) actually ran the job.
    assert!(dir.path().join("song.stems/job.json").is_file());
}

#[test]
fn later_runs_reuse_the_environment_without_uv() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = uncompose(dir.path(), "song.wav")
        .output()
        .expect("running CLI");
    assert!(first.status.success());
    std::fs::remove_file(dir.path().join("uv.log")).expect("clearing uv log");

    let second = uncompose(dir.path(), "song.wav")
        .output()
        .expect("running CLI again");
    let stdout = String::from_utf8_lossy(&second.stdout);

    assert!(second.status.success(), "stdout: {stdout}");
    assert_eq!(uv_log(dir.path()), "", "no uv calls on a later run");
    assert!(
        !stdout.contains("engine environment"),
        "no provisioning chatter once provisioned: {stdout}"
    );
}

#[test]
fn missing_uv_fails_with_an_install_message_before_any_work() {
    let dir = tempfile::tempdir().expect("tempdir");
    // ffmpeg present, uv absent: the preflight must name uv and how to get
    // it, not fail deep in a spawn error.
    let bin = dir.path().join("only-ffmpeg");
    std::fs::create_dir_all(&bin).expect("bin dir");
    write_executable(&bin.join("ffmpeg"), "#!/bin/sh\n");

    let output = uncompose(dir.path(), "song.wav")
        .env("PATH", &bin)
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("uv"), "names uv: {stderr}");
    assert!(
        stderr.contains("docs.astral.sh/uv"),
        "points at the uv install docs: {stderr}"
    );
    assert!(
        !dir.path().join("song.stems").exists(),
        "no job folder before provisioning succeeds"
    );
}
