//! Contract tests for the CLI binary end to end against the fake engine:
//! printed surface, exit codes, and kill behavior. Same seam as the core
//! tests, observed one level higher.

// Same helper as the core's contract tests; included by path so the two
// suites can't drift apart.
#[path = "../../core/tests/support/mod.rs"]
mod support;

use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

fn uncompose(dir: &Path, input_name: &str) -> Command {
    let input = dir.join(input_name);
    std::fs::write(&input, b"not really audio").expect("writing input");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_uncompose"));
    cmd.args(["separate", input.to_str().expect("utf8 path")])
        .args(["--device", "cpu"])
        .env("UNCOMPOSE_ENGINE_PYTHON", support::fake_engine())
        // Keep the model cache and last-job pointer inside the test's tempdir.
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("XDG_STATE_HOME", dir.join("state"));
    cmd
}

#[test]
fn separate_prints_header_progress_and_outcome() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = uncompose(dir.path(), "song.wav")
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("input:"), "header, got:\n{stdout}");
    assert!(stdout.contains("model:  htdemucs_6s"), "got:\n{stdout}");
    assert!(stdout.contains("device: cpu"), "got:\n{stdout}");
    assert!(
        stdout.contains("[model_load]"),
        "stage line, got:\n{stdout}"
    );
    assert!(stdout.contains("wrote vocals.wav"), "got:\n{stdout}");
    assert!(stdout.contains("wrote keys.wav"), "got:\n{stdout}");
    let folder = dir.path().join("song.stems");
    assert!(stdout.contains(&format!("done: 6 stems in {}", folder.display())));
    assert!(folder.join("job.json").is_file());

    // The last-job pointer names the completed folder so `play`/`open` work.
    let pointer = dir.path().join("state/uncompose/last-job");
    assert_eq!(
        std::fs::read_to_string(&pointer).expect("pointer written"),
        folder.to_string_lossy()
    );
}

#[test]
fn engine_failure_exits_nonzero_with_reason_and_no_job_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = uncompose(dir.path(), "error.wav")
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(
        stderr.contains("fake engine failed on purpose"),
        "got:\n{stderr}"
    );
    assert!(stderr.contains("engine.log tail"), "got:\n{stderr}");
    let folder = dir.path().join("error.stems");
    assert!(folder.join("engine.log").is_file(), "diagnosable folder");
    assert!(!folder.join("job.json").exists(), "must not look complete");
}

#[test]
fn missing_input_fails_with_a_clear_message() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_uncompose"));
    let output = cmd
        .args(["separate", dir.path().join("nope.wav").to_str().unwrap()])
        .env("UNCOMPOSE_ENGINE_PYTHON", support::fake_engine())
        .env("XDG_CACHE_HOME", dir.path().join("cache"))
        .output()
        .expect("running CLI");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("input not found"));
}

#[test]
fn sigint_mid_run_kills_the_job_without_a_job_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A real Ctrl+C is delivered to the foreground process group, CLI and
    // engine both; give the CLI its own group so the test can do the same
    // without the engine surviving as an orphan.
    let mut child = uncompose(dir.path(), "hang.wav")
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .expect("spawning CLI");

    // Wait until the engine is provably mid-run before interrupting.
    let stdout = child.stdout.take().expect("piped stdout");
    let mut saw_stage = false;
    for line in BufReader::new(stdout).lines() {
        if line.expect("reading CLI stdout").contains("[separate]") {
            saw_stage = true;
            break;
        }
    }
    assert!(saw_stage, "CLI never reached the hanging stage");

    let kill = Command::new("kill")
        .args(["-INT", "--", &format!("-{}", child.id())])
        .status()
        .expect("sending SIGINT");
    assert!(kill.success());

    let status = child.wait().expect("waiting for CLI");
    assert!(!status.success());
    let folder = dir.path().join("hang.stems");
    assert!(folder.is_dir(), "job folder left as the artifact");
    assert!(!folder.join("job.json").exists(), "must not look complete");
}
