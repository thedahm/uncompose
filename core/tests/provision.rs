//! Engine Environment provisioning behind the uv process seam (issue #36):
//! the core shells out to a `uv` executable to build the runtime engine
//! environment. Tests substitute a fake `uv` script, the same way the
//! contract suite substitutes the fake engine — no real uv, no torch, no
//! network.

#[path = "support/fake_uv.rs"]
mod fake_uv;

use std::path::{Path, PathBuf};

use fake_uv::{uv_log, write_executable};
use uncompose_core::provision::ensure_engine_env;

/// An argv-logging fake uv whose `venv` installs an inert stub python.
fn write_fake_uv(dir: &Path) -> PathBuf {
    fake_uv::write_fake_uv(dir, dir, None)
}

#[test]
fn fresh_provisioning_builds_the_env_and_returns_its_python() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let uv = write_fake_uv(tmp.path());
    let env_dir = tmp.path().join("engine-env");

    let mut events = Vec::new();
    let python = ensure_engine_env(&uv, &env_dir, "0.1.0", |e| events.push(format!("{e:?}")))
        .expect("provisioning succeeds");

    assert_eq!(python, env_dir.join("bin/python"));
    assert!(python.is_file(), "provisioned python exists");

    let log = uv_log(tmp.path());
    let calls: Vec<&str> = log.lines().collect();
    assert_eq!(calls.len(), 2, "one venv call, one install call: {log}");
    assert!(
        calls[0].starts_with("venv") && calls[0].ends_with(&env_dir.display().to_string()),
        "first call creates the venv: {}",
        calls[0]
    );
    assert!(
        calls[1].contains("pip install") && calls[1].contains("uncompose-engine==0.1.0"),
        "second call installs the pinned engine: {}",
        calls[1]
    );
    assert!(
        !events.is_empty(),
        "provisioning surfaces progress events to the caller"
    );
}

#[test]
fn a_provisioned_env_is_reused_without_invoking_uv() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let uv = write_fake_uv(tmp.path());
    let env_dir = tmp.path().join("engine-env");

    ensure_engine_env(&uv, &env_dir, "0.1.0", |_| {}).expect("first provisioning");
    std::fs::remove_file(tmp.path().join("uv.log")).expect("clearing uv log");

    let python = ensure_engine_env(&uv, &env_dir, "0.1.0", |_| {}).expect("second call");

    assert_eq!(python, env_dir.join("bin/python"));
    assert_eq!(uv_log(tmp.path()), "", "no uv calls for a complete env");
}

#[test]
fn a_partial_env_is_wiped_and_rebuilt() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let uv = write_fake_uv(tmp.path());
    let env_dir = tmp.path().join("engine-env");

    // A crash mid-provision leaves a directory without the completion
    // marker; a stale file inside must not survive the rebuild.
    std::fs::create_dir_all(env_dir.join("bin")).expect("partial env");
    std::fs::write(env_dir.join("bin/stale"), b"leftover").expect("stale file");

    let python = ensure_engine_env(&uv, &env_dir, "0.1.0", |_| {}).expect("rebuild");

    assert!(python.is_file(), "rebuilt python exists");
    assert!(
        !env_dir.join("bin/stale").exists(),
        "partial env contents are wiped before rebuilding"
    );
    assert_eq!(uv_log(tmp.path()).lines().count(), 2, "full rebuild ran");
}

#[test]
fn an_env_holding_an_older_engine_is_rebuilt_on_upgrade() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let uv = write_fake_uv(tmp.path());
    let env_dir = tmp.path().join("engine-env");

    ensure_engine_env(&uv, &env_dir, "0.1.0", |_| {}).expect("provision 0.1.0");
    std::fs::remove_file(tmp.path().join("uv.log")).expect("clearing uv log");

    ensure_engine_env(&uv, &env_dir, "0.2.0", |_| {}).expect("provision 0.2.0");

    let log = uv_log(tmp.path());
    assert!(
        log.contains("uncompose-engine==0.2.0"),
        "upgrade reinstalls the new pin: {log}"
    );
}

#[test]
fn a_failing_uv_surfaces_an_error_and_leaves_no_complete_env() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("uv");
    write_executable(&path, "#!/bin/sh\nexit 1\n");
    let env_dir = tmp.path().join("engine-env");

    let err = ensure_engine_env(&path, &env_dir, "0.1.0", |_| {})
        .expect_err("provisioning fails when uv fails");

    assert!(err.to_string().contains("uv"), "error names uv: {err}");
    // Whatever uv left behind, a later call must not mistake it for a
    // complete environment: a working fake uv provisions from scratch.
    let uv = write_fake_uv(tmp.path());
    ensure_engine_env(&uv, &env_dir, "0.1.0", |_| {}).expect("retry rebuilds");
    assert_eq!(
        uv_log(tmp.path()).lines().count(),
        2,
        "retry ran a full build"
    );
}
