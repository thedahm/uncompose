//! Guards the demonstration script (`demo/build_demo.py`) at the two
//! properties the M5 slice-6 issue names as its CI floor: the script parses
//! cleanly (a lint floor via `py_compile`) and its fixture generation is
//! deterministic — the same bytes on every rerun. The full demo drives the
//! sibling wheels (uncompose-project, uncompose-compare) and so is exercised
//! by the manual docs-alone pass and M6, not here.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The repo root: this crate is `<root>/cli`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cli crate has a parent")
        .to_path_buf()
}

fn demo_script() -> PathBuf {
    repo_root().join("demo/build_demo.py")
}

/// Locate a Python 3 interpreter, or `None` if none is on PATH. The Rust CI
/// job runs on an image that ships python3; a machine without it simply skips
/// these checks rather than failing a Rust-only environment.
fn python3() -> Option<String> {
    for candidate in ["python3", "python"] {
        let ok = Command::new(candidate)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return Some(candidate.to_string());
        }
    }
    None
}

#[test]
fn demo_script_compiles_cleanly() {
    let Some(python) = python3() else {
        eprintln!("skipping: no python3 on PATH");
        return;
    };
    let status = Command::new(python)
        .args(["-m", "py_compile"])
        .arg(demo_script())
        .status()
        .expect("running py_compile");
    assert!(status.success(), "demo script failed py_compile");
}

#[test]
fn demo_fixtures_are_byte_identical_across_runs() {
    let Some(python) = python3() else {
        eprintln!("skipping: no python3 on PATH");
        return;
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let a = dir.path().join("a");
    let b = dir.path().join("b");

    for out in [&a, &b] {
        let status = Command::new(&python)
            .arg(demo_script())
            .arg("--fixtures-only")
            .arg(out)
            .status()
            .expect("running build_demo.py --fixtures-only");
        assert!(status.success(), "fixture generation failed");
    }

    let first = read_tree(&a);
    let second = read_tree(&b);
    assert!(!first.is_empty(), "fixtures produced no files");
    assert_eq!(
        first.keys().collect::<Vec<_>>(),
        second.keys().collect::<Vec<_>>(),
        "two runs produced a different set of files"
    );
    for (rel, bytes) in &first {
        assert_eq!(
            second.get(rel),
            Some(bytes),
            "file {} differs between runs",
            rel.display()
        );
    }
}

/// Every file under `root`, keyed by path relative to `root`, with its bytes.
fn read_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("reading fixture dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(root).expect("under root").to_path_buf();
                files.insert(rel, std::fs::read(&path).expect("reading fixture file"));
            }
        }
    }
    files
}
