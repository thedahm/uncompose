//! The root's version line is the reference shape ADR-0005 holds extensions
//! to (`uncompose-<command> X.Y.Z` mirrors the root's `uncompose X.Y.Z`), so
//! its exact stdout and exit code are pinned here the same way the extension
//! side pins its own.

use std::process::Command;

fn assert_version_output(flag: &str) {
    let out = Command::new(env!("CARGO_BIN_EXE_uncompose"))
        .arg(flag)
        .output()
        .expect("running CLI");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("uncompose {}\n", env!("CARGO_PKG_VERSION")),
    );
}

#[test]
fn version_flag_prints_contract_shape() {
    assert_version_output("--version");
}

#[test]
fn short_version_flag_matches_long() {
    assert_version_output("-V");
}
