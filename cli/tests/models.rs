//! CLI-level contract for the `models` verbs: the printed surface of
//! `list`/`fetch`/`remove`, observed end to end. No network: with real pinned
//! digests in the manifest, a successful `fetch` can only be exercised against
//! the real artifacts, so its happy path is manual acceptance (#33); the
//! download/verify machinery is covered in core against a fake fetcher.

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

#[test]
fn models_remove_all_clears_every_cached_model() {
    let dir = tempfile::tempdir().unwrap();
    let cache_dir = dir.path().join("uncompose").join("models");
    std::fs::create_dir_all(&cache_dir).unwrap();
    // Stand-ins for every manifest entry's weight files, across two models.
    for file_name in [
        "5c90dfd2-34c22ccb.th",
        "htdemucs_6s.yaml",
        "vocals_mel_band_roformer.ckpt",
        "vocals_mel_band_roformer.yaml",
    ] {
        std::fs::write(cache_dir.join(file_name), b"stand-in weights").unwrap();
    }

    let out = uncompose(dir.path())
        .args(["models", "remove", "--all"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("htdemucs_6s: removed"), "got:\n{stdout}");
    assert!(
        stdout.contains("mel_band_roformer_kim: removed"),
        "got:\n{stdout}"
    );
    assert!(
        std::fs::read_dir(&cache_dir).unwrap().next().is_none(),
        "cache dir should be empty after --all"
    );
}

#[test]
fn models_remove_without_id_or_all_fails_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let out = uncompose(dir.path())
        .args(["models", "remove"])
        .output()
        .expect("running CLI");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("required") || stderr.contains("--all"),
        "got:\n{stderr}"
    );
}

#[test]
fn models_remove_rejects_id_and_all_together() {
    let dir = tempfile::tempdir().unwrap();
    let out = uncompose(dir.path())
        .args(["models", "remove", "htdemucs_6s", "--all"])
        .output()
        .expect("running CLI");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflict"),
        "got:\n{stderr}"
    );
}
