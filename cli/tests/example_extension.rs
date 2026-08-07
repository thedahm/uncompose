//! The committed example extension (docs/examples/uncompose-example) must work
//! exactly as the extension-author docs promise (M0.4, ADR-0005): installed on
//! PATH, `uncompose example` delegates to it with arguments and exit behavior
//! preserved, and it honors the `--version`/`--help` contract. Unlike the
//! fixtures in dispatch.rs, these tests copy the real committed script so the
//! shipped example can never drift from the contract.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Path of the committed example script in the repo.
fn committed_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/examples/uncompose-example")
}

/// Install the committed example into `dir` as an executable, the way an
/// author's `install -m 755` would.
fn install_example(dir: &Path) {
    let dest = dir.join("uncompose-example");
    fs::copy(committed_script(), &dest).expect("committed example script must exist");
    let mut perms = fs::metadata(&dest).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&dest, perms).unwrap();
}

/// Run the compiled `uncompose` with `PATH` pointed only at `path_dir`.
fn uncompose(path_dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_uncompose"));
    cmd.env("PATH", path_dir);
    cmd
}

#[test]
fn uncompose_example_delegates_with_arguments() {
    let dir = tempfile::tempdir().unwrap();
    install_example(dir.path());

    let out = uncompose(dir.path())
        .args(["example", "hello", "--flag", "x"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The example echoes each forwarded argument on its own line, proving the
    // root forwarded argv verbatim.
    assert!(
        stdout.contains("hello") && stdout.contains("--flag") && stdout.contains("x"),
        "forwarded arguments should appear in the example's output, got:\n{stdout}"
    );
}

#[test]
fn version_flag_matches_contract_shape() {
    let dir = tempfile::tempdir().unwrap();
    install_example(dir.path());

    let out = uncompose(dir.path())
        .args(["example", "--version"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(out.status.success(), "--version must exit 0");
    // Contract shape: `uncompose-example X.Y.Z`, one line to stdout.
    let line = stdout.trim_end();
    assert!(
        !line.contains('\n'),
        "--version must be one line: {stdout:?}"
    );
    let version = line
        .strip_prefix("uncompose-example ")
        .unwrap_or_else(|| panic!("version line must start with the tool name: {line:?}"));
    let parts: Vec<&str> = version.split('.').collect();
    assert!(
        parts.len() == 3 && parts.iter().all(|p| p.parse::<u32>().is_ok()),
        "version must be X.Y.Z, got: {version:?}"
    );
}

#[test]
fn help_flag_prints_usage_and_exits_zero() {
    let dir = tempfile::tempdir().unwrap();
    install_example(dir.path());

    let out = uncompose(dir.path())
        .args(["example", "--help"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(out.status.success(), "--help must exit 0");
    assert!(
        stdout.to_lowercase().contains("usage"),
        "--help must print usage to stdout, got:\n{stdout}"
    );
}

#[test]
fn exit_codes_pass_through_from_the_example() {
    let dir = tempfile::tempdir().unwrap();
    install_example(dir.path());

    let out = uncompose(dir.path())
        .args(["example", "exit", "42"])
        .output()
        .expect("running CLI");

    assert_eq!(
        out.status.code(),
        Some(42),
        "the example's exit code must be the user-visible exit code"
    );
}
