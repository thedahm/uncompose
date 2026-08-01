//! A fake `uv` for provisioning tests, shared by the core and CLI suites
//! the way `fake_engine` is: substitution stays at the process boundary.
//! The script logs every invocation's argv to `<log_dir>/uv.log` and, on
//! `venv`, creates the new environment's `bin/python`.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub fn write_executable(path: &Path, contents: &str) {
    std::fs::write(path, contents).expect("writing executable");
    let mut perms = std::fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}

/// Write the fake `uv` into `bin`. `python_source` is what `venv` installs
/// as the environment's python: a real engine stand-in for end-to-end runs,
/// or `None` for an inert stub when only the argv log matters.
pub fn write_fake_uv(bin: &Path, log_dir: &Path, python_source: Option<&Path>) -> PathBuf {
    let install = match python_source {
        Some(src) => format!(r#"cp "{}" "$env_dir/bin/python""#, src.display()),
        None => r#"printf '#!/bin/sh\n' > "$env_dir/bin/python"
  chmod +x "$env_dir/bin/python""#
            .to_string(),
    };
    let script = format!(
        r#"#!/bin/sh
# The CLI may run us on a hermetic PATH; restore one for mkdir/cp below.
PATH=/usr/bin:/bin
echo "$@" >> "{log}"
if [ "$1" = venv ]; then
  for a; do env_dir=$a; done
  mkdir -p "$env_dir/bin"
  {install}
fi
"#,
        log = log_dir.join("uv.log").display(),
    );
    let path = bin.join("uv");
    write_executable(&path, &script);
    path
}

pub fn uv_log(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("uv.log")).unwrap_or_default()
}
