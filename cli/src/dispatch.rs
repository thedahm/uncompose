//! Git-style external-command dispatch (ADR-0005, contract revision 1). Before
//! argv reaches clap, peek at the first token: if it is not a builtin and looks
//! like a command name, resolve `uncompose-<token>` on `PATH` and `exec()` it,
//! forwarding everything after the token verbatim. On Linux (v0.1's only
//! platform, per ADR-0004) `exec()` replaces this process, so the extension owns
//! stdin/stdout/stderr, TTY-ness, signals, and the exit code natively.
//!
//! When dispatch fails, the launcher convention applies: 127 for a missing
//! extension, 126 for one that is present but not executable (or whose `exec`
//! failed). The 127 message names the exact `uncompose-<token>` looked for;
//! known family names get an install hint, and other unknown names a
//! did-you-mean against builtins and the `uncompose-*` executables actually
//! installed (#79).

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

/// First-party sibling tools that exist as PyPI packages even when not
/// installed locally, so a miss on these names gets an install hint instead of
/// a did-you-mean (ADR-0005).
const FAMILY: &[&str] = &["project", "compare"];

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
            if FAMILY.contains(&token) {
                eprintln!("install it with: uv tool install {name}");
            } else {
                let close = suggestions(token, &installed_extensions());
                if !close.is_empty() {
                    let list: Vec<String> = close.iter().map(|s| format!("'{s}'")).collect();
                    eprintln!("did you mean {}?", list.join(" or "));
                }
            }
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

/// Scan `PATH` for executable `uncompose-*` files and return their command
/// names (prefix stripped), sorted and deduplicated. Names only, by directory
/// listing — nothing is executed. Also the future basis for the
/// "External commands (installed):" help section (ADR-0005).
fn installed_extensions() -> Vec<String> {
    let Some(path) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    for dir in std::env::split_paths(&path) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_name) = entry.file_name().into_string() else {
                continue;
            };
            let Some(name) = file_name.strip_prefix("uncompose-") else {
                continue;
            };
            if is_valid_token(name) && entry.path().is_file() && is_executable(&entry.path()) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// The closest candidates to `token` among builtins, family names, and
/// installed extensions: everything within edit distance 2, nearest first,
/// ties alphabetical (candidates are sorted before scoring and the distance
/// sort is stable). Family names are included so a typo of an uninstalled
/// family tool ('projct') routes the user to the name whose exact miss
/// carries the install hint.
fn suggestions(token: &str, installed: &[String]) -> Vec<String> {
    let mut candidates: Vec<String> = BUILTINS
        .iter()
        .chain(FAMILY)
        .map(|s| s.to_string())
        .chain(installed.iter().cloned())
        .collect();
    candidates.sort();
    candidates.dedup();
    let mut scored: Vec<(usize, String)> = candidates
        .into_iter()
        .map(|c| (levenshtein(token, &c), c))
        .filter(|(d, _)| *d <= 2)
        .collect();
    scored.sort_by_key(|(d, _)| *d);
    scored.into_iter().map(|(_, c)| c).collect()
}

/// Classic two-row Levenshtein; inputs are short command names, so O(n·m) is
/// nothing.
fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut curr = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != *cb);
            curr.push(sub.min(prev[j + 1] + 1).min(curr[j] + 1));
        }
        prev = curr;
    }
    prev[b.len()]
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

    /// Family names must never collide with builtins — a collision would make
    /// the install hint unreachable (the builtin always wins dispatch).
    #[test]
    fn family_names_are_not_builtins() {
        for family in super::FAMILY {
            assert!(!super::BUILTINS.contains(family));
        }
    }

    #[test]
    fn levenshtein_basics() {
        assert_eq!(super::levenshtein("separate", "separate"), 0);
        assert_eq!(super::levenshtein("seperate", "separate"), 1);
        assert_eq!(super::levenshtein("", "abc"), 3);
        assert_eq!(super::levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn suggestions_rank_nearest_first_and_cut_off_at_distance_two() {
        let installed = vec!["example".to_string(), "sep".to_string()];
        assert_eq!(super::suggestions("seperate", &installed), ["separate"]);
        assert_eq!(super::suggestions("exampel", &installed), ["example"]);
        assert!(super::suggestions("zzzqqq", &installed).is_empty());
    }

    #[test]
    fn suggestions_dedupe_a_name_that_is_both_builtin_and_installed() {
        let installed = vec!["play".to_string()];
        assert_eq!(super::suggestions("pla", &installed), ["play"]);
    }
}
