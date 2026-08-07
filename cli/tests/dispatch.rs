//! CLI-level contract for external-command dispatch (ADR-0005, M0.1): the root
//! `uncompose` execs `uncompose-<token>` from `PATH`, forwarding argv verbatim,
//! with the extension owning the exit code. Fixtures are tiny shell scripts the
//! tests write into a temp `PATH` dir; the tests observe only external behavior
//! (stdout/stderr text and exit codes) by running the compiled binary.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

/// Run the compiled `uncompose` with `PATH` pointed only at `path_dir`, so the
/// only `uncompose-*` executables visible are the fixtures the test wrote.
fn uncompose(path_dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_uncompose"));
    cmd.env("PATH", path_dir);
    cmd
}

/// Write an executable shell-script fixture named `name` into `dir`.
fn write_script(dir: &Path, name: &str, body: &str) {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
}

#[test]
fn dispatches_to_extension_forwarding_argv_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    write_script(
        dir.path(),
        "uncompose-example",
        "#!/bin/sh\nfor a in \"$@\"; do echo \"$a\"; done\n",
    );

    let out = uncompose(dir.path())
        .args(["example", "init", "--flag", "x"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout, "init\n--flag\nx\n",
        "argv forwarded verbatim, got:\n{stdout}"
    );
}

#[test]
fn forwards_flag_like_tokens_untouched() {
    let dir = tempfile::tempdir().unwrap();
    write_script(
        dir.path(),
        "uncompose-example",
        "#!/bin/sh\nfor a in \"$@\"; do echo \"$a\"; done\n",
    );

    let out = uncompose(dir.path())
        .args(["example", "--blind", "a.wav", "b.wav"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        stdout, "--blind\na.wav\nb.wav\n",
        "flag-like tokens must reach the extension, not the root parser, got:\n{stdout}"
    );
}

#[test]
fn extension_exit_code_passes_through() {
    let dir = tempfile::tempdir().unwrap();
    write_script(dir.path(), "uncompose-example", "#!/bin/sh\nexit 42\n");

    let out = uncompose(dir.path())
        .args(["example"])
        .output()
        .expect("running CLI");

    assert_eq!(
        out.status.code(),
        Some(42),
        "extension exit code must pass through"
    );
}

#[test]
fn rogue_extension_never_shadows_a_builtin() {
    let dir = tempfile::tempdir().unwrap();
    write_script(
        dir.path(),
        "uncompose-separate",
        "#!/bin/sh\necho ROGUE-SEPARATE-RAN\n",
    );

    // `separate` is a builtin; the rogue on PATH must never run. The builtin
    // errors on the missing song argument, which is the proof clap took over.
    let out = uncompose(dir.path())
        .args(["separate"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !stdout.contains("ROGUE"),
        "builtin was shadowed, stdout:\n{stdout}"
    );
    assert!(
        !stderr.contains("ROGUE"),
        "builtin was shadowed, stderr:\n{stderr}"
    );
    assert!(
        !out.status.success(),
        "missing required arg should fail in clap"
    );
}

#[test]
fn token_failing_the_naming_rule_is_not_looked_up() {
    let dir = tempfile::tempdir().unwrap();
    // A fixture that *would* run if an uppercase token were ever looked up.
    write_script(dir.path(), "uncompose-Foo", "#!/bin/sh\necho RAN\n");

    let out = uncompose(dir.path())
        .args(["Foo"])
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !stdout.contains("RAN"),
        "invalid token must not dispatch, stdout:\n{stdout}"
    );
    assert!(
        !out.status.success(),
        "invalid token should be clap's unknown-command error"
    );
    // clap's ordinary unknown-subcommand error, no launcher/PATH message.
    assert!(
        !stderr.contains("found on PATH"),
        "invalid token must not reach PATH resolution, stderr:\n{stderr}"
    );
}

#[test]
fn path_traversal_token_is_not_looked_up() {
    let dir = tempfile::tempdir().unwrap();

    let out = uncompose(dir.path())
        .args(["../evil"])
        .output()
        .expect("running CLI");
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(!out.status.success());
    assert!(
        !stderr.contains("found on PATH"),
        "a slash token must not reach PATH resolution, stderr:\n{stderr}"
    );
}
