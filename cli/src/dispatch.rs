//! Git-style external-command dispatch (ADR-0005, contract revision 1). Before
//! argv reaches clap, peek at the first token: if it is not a builtin and looks
//! like a command name, resolve `uncompose-<token>` on `PATH` and `exec()` it,
//! forwarding everything after the token verbatim. On Linux (v0.1's only
//! platform, per ADR-0004) `exec()` replaces this process, so the extension owns
//! stdin/stdout/stderr, TTY-ness, signals, and the exit code natively.
//!
//! This module implements the M0.1 core dispatch: finding and exec'ing the
//! extension. The richer failure UX (install hints, did-you-mean) is layered on
//! top in #79; here a missing extension is a plain 127 and a
//! present-but-not-executable one a plain 126, per the launcher convention.

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Builtins always win over a same-named `uncompose-*` executable on `PATH`, so
/// installing a rogue `uncompose-separate` changes nothing. `help` is clap's
/// auto-generated subcommand rather than a `Command` variant; the rest must
/// track the clap tree (enforced by a test below).
const BUILTINS: &[&str] = &["separate", "play", "open", "models", "help"];

/// If argv designates an external command, `exec()` it (never returning) or exit
/// with a launcher error code. Otherwise return so clap parses argv normally.
///
/// Only the first token dispatches, and only when it is not a builtin and
/// matches `^[a-z0-9][a-z0-9-]*$`. A leading flag (or no argument at all) is the
/// root's own business and falls through to clap; no root flags are forwarded.
pub fn maybe_dispatch() {
    let args: Vec<OsString> = std::env::args_os().collect();
    // A non-UTF-8 first token cannot match the ASCII naming rule, so it falls
    // through to clap like any other ineligible token. Later arguments stay
    // OsString the whole way — they are forwarded, never inspected.
    let Some(token) = args.get(1).and_then(|t| t.to_str()) else {
        return;
    };
    if token.starts_with('-') {
        return;
    }
    if BUILTINS.contains(&token) {
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

fn dispatch(token: &str, forwarded: &[OsString]) -> ! {
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

/// Walk `PATH` looking for an executable `name`. Like `execvp`, a
/// non-executable match does not stop the search — later directories can still
/// supply the real binary; the first such match is only reported (as 126) when
/// no executable exists anywhere on `PATH`.
fn resolve(name: &str) -> Resolution {
    let Some(path) = std::env::var_os("PATH") else {
        return Resolution::NotFound;
    };
    let mut not_executable: Option<PathBuf> = None;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            if is_executable(&candidate) {
                return Resolution::Executable(candidate);
            }
            not_executable.get_or_insert(candidate);
        }
    }
    match not_executable {
        Some(path) => Resolution::NotExecutable(path),
        None => Resolution::NotFound,
    }
}

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    /// Every clap subcommand must appear in `BUILTINS`, or an extension named
    /// after it could shadow the builtin. `help` is asserted separately because
    /// clap generates it outside the `Command` enum.
    #[test]
    fn builtins_cover_every_clap_subcommand() {
        for sub in crate::Cli::command().get_subcommands() {
            assert!(
                super::BUILTINS.contains(&sub.get_name()),
                "clap subcommand '{}' is missing from BUILTINS",
                sub.get_name()
            );
        }
        assert!(super::BUILTINS.contains(&"help"));
    }
}
