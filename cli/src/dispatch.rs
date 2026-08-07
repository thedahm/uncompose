//! Git-style external-command dispatch (ADR-0005, contract revision 1). Before
//! argv reaches clap, peek at the first token: if it is not a builtin and looks
//! like a command name, resolve `uncompose-<token>` on `PATH` and `exec()` it,
//! forwarding everything after the token verbatim. On Linux (v0.1's only
//! platform, per ADR-0004) `exec()` replaces this process, so the extension owns
//! stdin/stdout/stderr, TTY-ness, signals, and the exit code natively.
//!
//! This module implements the M0.1 core dispatch: finding and exec'ing the
//! extension. The richer failure UX (install hints, did-you-mean) is layered on
//! top in later work; here a missing extension is a plain 127 and a
//! present-but-not-executable one a plain 126, per the launcher convention.

use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Builtins always win over a same-named `uncompose-*` executable on `PATH`, so
/// installing a rogue `uncompose-separate` changes nothing.
const BUILTINS: &[&str] = &["separate", "play", "open", "models"];

/// If argv designates an external command, `exec()` it (never returning) or exit
/// with a launcher error code. Otherwise return so clap parses argv normally.
///
/// Only the first token dispatches, and only when it is not a builtin and
/// matches `^[a-z0-9][a-z0-9-]*$`. A leading flag (or no argument at all) is the
/// root's own business and falls through to clap; no root flags are forwarded.
pub fn maybe_dispatch() {
    let args: Vec<String> = std::env::args().collect();
    let Some(token) = args.get(1) else {
        return;
    };
    if token.starts_with('-') {
        return;
    }
    if BUILTINS.contains(&token.as_str()) {
        return;
    }
    // Tokens that fail the naming rule (uppercase, dots, slashes) never trigger a
    // PATH lookup — they fall through to clap's ordinary unknown-command error,
    // which blocks `uncompose ../evil` path tricks.
    if !is_valid_token(token) {
        return;
    }
    dispatch(token, &args[2..]);
}

/// `^[a-z0-9][a-z0-9-]*$`, spelled out to avoid a regex dependency.
fn is_valid_token(token: &str) -> bool {
    let mut chars = token.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn dispatch(token: &str, forwarded: &[String]) -> ! {
    let name = format!("uncompose-{token}");
    match resolve(&name) {
        Resolution::Executable(path) => {
            // exec() only returns on failure; on success the extension has fully
            // replaced us.
            let err = Command::new(&path).args(forwarded).exec();
            eprintln!("uncompose: failed to run '{name}': {err}");
            std::process::exit(126);
        }
        Resolution::NotExecutable(path) => {
            eprintln!(
                "uncompose: '{name}' found at {} but is not executable",
                path.display()
            );
            std::process::exit(126);
        }
        Resolution::NotFound => {
            eprintln!(
                "uncompose: '{token}' is not an uncompose command \
                 (no '{name}' found on PATH)"
            );
            std::process::exit(127);
        }
    }
}

enum Resolution {
    Executable(PathBuf),
    NotExecutable(PathBuf),
    NotFound,
}

/// Walk `PATH` looking for `name`, returning the first match and whether it is
/// executable. Earlier `PATH` entries win, matching shell lookup order.
fn resolve(name: &str) -> Resolution {
    let Some(path) = std::env::var_os("PATH") else {
        return Resolution::NotFound;
    };
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return if is_executable(&candidate) {
                Resolution::Executable(candidate)
            } else {
                Resolution::NotExecutable(candidate)
            };
        }
    }
    Resolution::NotFound
}

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
