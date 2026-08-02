//! The provisioning pin only works if versions move in lockstep: the core
//! installs `uncompose-engine==<its own version>`, and the published wheel
//! must carry that same version. Guard the three places a release version
//! lives — the Cargo workspace, the CLI wheel's pyproject, and the engine's
//! pyproject — against drifting apart.

use std::path::Path;

/// Extract `version = "..."` from a pyproject's `[project]` table.
fn pyproject_version(path: &Path) -> String {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    text.lines()
        .map(str::trim)
        .find_map(|l| {
            l.strip_prefix("version = \"")?
                .strip_suffix('"')
                .map(str::to_string)
        })
        .unwrap_or_else(|| panic!("no version in {}", path.display()))
}

#[test]
fn wheel_and_engine_versions_match_the_workspace() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root");
    let workspace = env!("CARGO_PKG_VERSION");
    assert_eq!(
        pyproject_version(&repo.join("pyproject.toml")),
        workspace,
        "the uncompose wheel's pyproject.toml version must match the Cargo workspace"
    );
    assert_eq!(
        pyproject_version(&repo.join("engine/pyproject.toml")),
        workspace,
        "engine/pyproject.toml version must match the Cargo workspace: the core \
         provisions uncompose-engine=={workspace}"
    );
}
