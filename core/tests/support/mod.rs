//! Locate (building if needed) the fake-engine binary the contract tests
//! spawn in place of the Python shim.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Once;

pub fn fake_engine() -> PathBuf {
    // current_exe is <target>/debug/deps/<test>-<hash>; the workspace bins
    // live two levels up. This tracks CARGO_TARGET_DIR moves for free.
    let mut path = std::env::current_exe().expect("test binary path");
    path.pop();
    path.pop();
    let path = path.join("fake-engine");

    // A workspace-wide `cargo test` builds it; a `-p` run may not have.
    static BUILD: Once = Once::new();
    if !path.is_file() {
        BUILD.call_once(|| {
            let status = Command::new(env!("CARGO"))
                .args(["build", "-p", "fake-engine"])
                .status()
                .expect("running cargo build -p fake-engine");
            assert!(status.success(), "building fake-engine failed");
        });
    }
    assert!(path.is_file(), "fake-engine binary missing at {path:?}");
    path
}
