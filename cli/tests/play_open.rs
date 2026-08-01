//! Contract tests for the `play` and `open` verbs: both resolve the last-job
//! pointer written by `separate` and shell out to an external player/opener.
//! The players and opener are faked as tiny scripts on a controlled PATH so
//! CI never needs mpv, ffplay, or xdg-open installed.

// Shared fake-engine locator, included by path like the other CLI suite.
#[path = "../../core/tests/support/mod.rs"]
mod support;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run `separate` so a last-job pointer and a real stem folder exist. Returns
/// the job folder. State and cache stay inside `home`.
fn separate(home: &Path) -> PathBuf {
    let input = home.join("song.wav");
    std::fs::write(&input, b"not really audio").expect("writing input");
    let status = Command::new(env!("CARGO_BIN_EXE_uncompose"))
        .args(["separate", input.to_str().unwrap(), "--device", "cpu"])
        .env("UNCOMPOSE_ENGINE_PYTHON", support::fake_engine())
        .env("XDG_CACHE_HOME", home.join("cache"))
        .env("XDG_STATE_HOME", home.join("state"))
        .output()
        .expect("running separate")
        .status;
    assert!(status.success(), "separate should succeed");
    home.join("song.stems")
}

/// Create a `bin/` dir holding fake executables for each named program. Each
/// one appends its own path and arguments to `log`, so a test can assert both
/// which program ran and what target it was handed.
fn fake_bin(home: &Path, programs: &[&str], log: &Path) -> PathBuf {
    let bin = home.join("bin");
    std::fs::create_dir_all(&bin).expect("creating fake bin");
    for prog in programs {
        let script = bin.join(prog);
        std::fs::write(
            &script,
            format!("#!/bin/sh\necho \"$0 $@\" >> \"{}\"\n", log.display()),
        )
        .expect("writing fake program");
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).expect("chmod fake program");
    }
    bin
}

fn play(home: &Path, bin: &Path, arg: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_uncompose"))
        .args(["play", arg])
        .env("XDG_STATE_HOME", home.join("state"))
        .env("PATH", bin)
        .output()
        .expect("running play")
}

#[test]
fn play_auditions_the_last_jobs_stem_with_mpv() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = separate(dir.path());
    let log = dir.path().join("player.log");
    let bin = fake_bin(dir.path(), &["mpv", "ffplay"], &log);

    let output = play(dir.path(), &bin, "vocals");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let logged = std::fs::read_to_string(&log).expect("player invoked");
    assert!(logged.contains("mpv"), "mpv preferred, got: {logged}");
    assert!(
        logged.contains(folder.join("vocals.wav").to_str().unwrap()),
        "handed the stem path, got: {logged}"
    );
}

#[test]
fn play_falls_back_to_ffplay_when_mpv_is_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = separate(dir.path());
    let log = dir.path().join("player.log");
    let bin = fake_bin(dir.path(), &["ffplay"], &log);

    let output = play(dir.path(), &bin, "drums");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let logged = std::fs::read_to_string(&log).expect("player invoked");
    assert!(logged.contains("ffplay"), "ffplay fallback, got: {logged}");
    assert!(logged.contains(folder.join("drums.wav").to_str().unwrap()));
}

#[test]
fn play_is_path_addressable_without_reading_the_pointer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = separate(dir.path());
    let stem = folder.join("bass.wav");
    let log = dir.path().join("player.log");
    let bin = fake_bin(dir.path(), &["mpv"], &log);

    // Point XDG_STATE at an empty dir: a direct path must play regardless.
    let empty = dir.path().join("no-state");
    std::fs::create_dir_all(&empty).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_uncompose"))
        .args(["play", stem.to_str().unwrap()])
        .env("XDG_STATE_HOME", &empty)
        .env("PATH", &bin)
        .output()
        .expect("running play");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let logged = std::fs::read_to_string(&log).expect("player invoked");
    assert!(logged.contains(stem.to_str().unwrap()), "got: {logged}");
}

#[test]
fn play_reports_when_no_player_is_installed() {
    let dir = tempfile::tempdir().expect("tempdir");
    separate(dir.path());
    let log = dir.path().join("player.log");
    let bin = fake_bin(dir.path(), &[], &log); // empty PATH: nothing to spawn

    let output = play(dir.path(), &bin, "vocals");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("mpv"), "names mpv, got: {stderr}");
    assert!(stderr.contains("ffplay"), "names ffplay, got: {stderr}");
}

#[test]
fn play_without_a_prior_job_points_at_separate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("player.log");
    let bin = fake_bin(dir.path(), &["mpv"], &log);
    // No separate run: XDG_STATE points at an empty dir, so the pointer is
    // missing.
    let output = play(dir.path(), &bin, "vocals");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("separate"), "got: {stderr}");
}

#[test]
fn open_launches_xdg_open_on_the_last_job_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = separate(dir.path());
    let log = dir.path().join("open.log");
    let bin = fake_bin(dir.path(), &["xdg-open"], &log);

    let output = Command::new(env!("CARGO_BIN_EXE_uncompose"))
        .arg("open")
        .env("XDG_STATE_HOME", dir.path().join("state"))
        .env("PATH", &bin)
        .output()
        .expect("running open");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let logged = std::fs::read_to_string(&log).expect("xdg-open invoked");
    assert!(logged.contains("xdg-open"), "got: {logged}");
    assert!(logged.contains(folder.to_str().unwrap()), "got: {logged}");
}

#[test]
fn open_reports_when_xdg_open_is_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    separate(dir.path());
    let log = dir.path().join("open.log");
    let bin = fake_bin(dir.path(), &[], &log); // no xdg-open on PATH

    let output = Command::new(env!("CARGO_BIN_EXE_uncompose"))
        .arg("open")
        .env("XDG_STATE_HOME", dir.path().join("state"))
        .env("PATH", &bin)
        .output()
        .expect("running open");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("xdg-open"));
}
