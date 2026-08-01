//! CLI-level contract for the `models` verbs: the printed surface of
//! `list`/`fetch`/`remove`, observed end to end. No network: the real models
//! have no download pin yet, so `fetch` reports that rather than hitting curl.

use std::path::Path;
use std::process::Command;

fn uncompose(cache: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_uncompose"));
    cmd.env("XDG_CACHE_HOME", cache);
    cmd
}

#[test]
fn models_list_shows_every_model_with_license_tier_and_cache_state() {
    let dir = tempfile::tempdir().unwrap();
    let out = uncompose(dir.path())
        .args(["models", "list"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("htdemucs_6s"), "got:\n{stdout}");
    assert!(stdout.contains("mel_band_roformer_kim"), "got:\n{stdout}");
    // Hardware tier and license status are surfaced here (relayed, not certified).
    assert!(stdout.contains("runs everywhere"), "tier, got:\n{stdout}");
    assert!(stdout.contains("GPU required"), "tier, got:\n{stdout}");
    assert!(stdout.contains("MIT"), "license, got:\n{stdout}");
    // Nothing cached in a fresh cache dir.
    assert!(stdout.contains("not cached"), "cache state, got:\n{stdout}");
}

#[test]
fn models_fetch_of_a_pinless_model_says_so_without_touching_the_network() {
    let dir = tempfile::tempdir().unwrap();
    let out = uncompose(dir.path())
        .args(["models", "fetch", "htdemucs_6s"])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(!out.status.success());
    assert!(stderr.contains("no download pin"), "got:\n{stderr}");
}

#[test]
fn models_fetch_of_an_unknown_target_fails_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let out = uncompose(dir.path())
        .args(["models", "fetch", "not-a-model"])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(!out.status.success());
    assert!(stderr.contains("unknown model or preset"), "got:\n{stderr}");
}

#[test]
fn models_remove_of_an_uncached_model_reports_nothing_to_remove() {
    let dir = tempfile::tempdir().unwrap();
    let out = uncompose(dir.path())
        .args(["models", "remove", "htdemucs_6s"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("not cached"), "got:\n{stdout}");
}

#[test]
fn models_remove_of_an_unknown_model_fails_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let out = uncompose(dir.path())
        .args(["models", "remove", "nope"])
        .output()
        .expect("running CLI");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown model"));
}
