//! Discovery surface (ADR-0005, M0.3): `uncompose --help` ends with an
//! "External commands (installed):" section built from a PATH directory scan —
//! names only, never executing anything. Fixtures are shell scripts in a temp
//! `PATH` dir, like the dispatch tests.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

/// Run the compiled `uncompose` with `PATH` pointed only at `path_dir`.
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
fn help_lists_installed_extensions_after_builtins() {
    let dir = tempfile::tempdir().unwrap();
    write_script(dir.path(), "uncompose-example", "#!/bin/sh\nexit 0\n");
    write_script(dir.path(), "uncompose-project", "#!/bin/sh\nexit 0\n");

    let out = uncompose(dir.path())
        .arg("--help")
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(out.status.success());
    let section = stdout
        .find("External commands (installed):")
        .unwrap_or_else(|| panic!("missing external commands section, stdout:\n{stdout}"));
    assert!(stdout.contains("\n  example\n"), "stdout:\n{stdout}");
    assert!(stdout.contains("\n  project\n"), "stdout:\n{stdout}");
    // Builtins first, then the external section.
    let builtins = stdout.find("separate").expect("builtin listed");
    assert!(
        builtins < section,
        "builtins should precede the external section, stdout:\n{stdout}"
    );
}

#[test]
fn help_section_is_built_without_executing_extensions() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("EXECUTED");
    write_script(
        dir.path(),
        "uncompose-example",
        &format!("#!/bin/sh\ntouch {}\n", marker.display()),
    );

    let out = uncompose(dir.path())
        .arg("--help")
        .output()
        .expect("running CLI");

    assert!(out.status.success());
    assert!(
        !marker.exists(),
        "the help scan must never execute a scanned binary"
    );
}

#[test]
fn duplicate_names_across_path_entries_appear_once() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    write_script(first.path(), "uncompose-example", "#!/bin/sh\nexit 0\n");
    write_script(second.path(), "uncompose-example", "#!/bin/sh\nexit 1\n");

    let path = std::env::join_paths([first.path(), second.path()]).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_uncompose"))
        .env("PATH", path)
        .arg("--help")
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(out.status.success());
    assert_eq!(
        stdout.matches("example").count(),
        1,
        "a name shadowed by an earlier PATH entry must appear once, stdout:\n{stdout}"
    );
}

#[test]
fn help_omits_the_section_when_nothing_is_installed() {
    let dir = tempfile::tempdir().unwrap(); // empty PATH dir

    let out = uncompose(dir.path())
        .arg("--help")
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(out.status.success());
    assert!(
        !stdout.contains("External commands"),
        "an empty section is noise, stdout:\n{stdout}"
    );
}

#[test]
fn help_subcommand_shows_the_same_section() {
    let dir = tempfile::tempdir().unwrap();
    write_script(dir.path(), "uncompose-example", "#!/bin/sh\nexit 0\n");

    let out = uncompose(dir.path())
        .arg("help")
        .output()
        .expect("running CLI");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(out.status.success());
    assert!(
        stdout.contains("External commands (installed):"),
        "`uncompose help` should match `--help`, stdout:\n{stdout}"
    );
    assert!(stdout.contains("\n  example\n"), "stdout:\n{stdout}");
}
