"""Build the clean environment the gate tests against.

One virtual environment, the three packages installed into it, and a PATH that
contains that environment's `bin` and the system directories only — so every
command the tests run is the installed artifact, reached the way a user reaches
it, with none of the build toolchain visible to it.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Mapping, Sequence

from .sources import (
    MODE_BUILD,
    MODE_PYPI,
    AcceptanceConfig,
    GitSource,
    PathSource,
    PypiSource,
    Source,
)

# What each mode needs on PATH before it can produce an installation. Building
# the extensions from git means compiling them: Rust for both wheels, Node for
# Compare's embedded frontend.
TOOLCHAIN: Mapping[str, tuple[str, ...]] = {
    MODE_BUILD: ("uv", "git", "cargo", "npm"),
    MODE_PYPI: ("uv",),
}

# The system directories the installed commands may see, alongside the
# environment's own bin. Deliberately not the caller's PATH: an installed wheel
# that only works because cargo happened to be next to it is not what ships.
SYSTEM_PATH = ("/usr/bin", "/bin")


class ToolchainMissing(Exception):
    """A tool the configured mode needs is not on PATH."""


@dataclass(frozen=True)
class Installation:
    """A clean environment with all three packages installed.

    Carries where each package came from — including the commit a git ref
    resolved to — so a failing gate names the artifacts it was testing rather
    than leaving that to be reconstructed from the log.
    """

    env: Path
    bin: Path
    path: str
    home: Path
    sources: tuple[Source, ...]
    commits: Mapping[str, str]


def missing_tools(mode: str) -> tuple[str, ...]:
    return tuple(tool for tool in TOOLCHAIN[mode] if shutil.which(tool) is None)


def _log(message: str) -> None:
    print(f">> {message}", file=sys.stderr, flush=True)


def _run(argv: Sequence[str | Path], cwd: Path | None = None) -> None:
    subprocess.run([str(arg) for arg in argv], cwd=cwd, check=True)


def _checkout(source: GitSource, into: Path, log: Callable[[str], None]) -> tuple[Path, str]:
    """Fetch `source.ref` into a fresh checkout and run its pre-build steps.

    Fetch-then-checkout rather than `git clone --branch`, so a ref may be a
    branch, a tag, or a commit sha without the caller saying which.
    """
    into.mkdir(parents=True, exist_ok=True)
    log(f"fetching {source.describe()}")
    _run(["git", "init", "--quiet"], cwd=into)
    _run(["git", "remote", "add", "origin", source.url], cwd=into)
    _run(["git", "fetch", "--quiet", "--depth", "1", "origin", source.ref], cwd=into)
    _run(["git", "checkout", "--quiet", "FETCH_HEAD"], cwd=into)
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=into, check=True, capture_output=True, text=True
    ).stdout.strip()
    log(f"{source.package} at {commit}")
    for command in source.prepare:
        log(f"{source.package}: {' '.join(command)}")
        _run(command, cwd=into)
    return into, commit


def _install_target(
    source: Source, workdir: Path, log: Callable[[str], None]
) -> tuple[str, str | None]:
    """The argument `uv pip install` gets for this source, and its commit if any."""
    if isinstance(source, PypiSource):
        return source.requirement(), None
    if isinstance(source, PathSource):
        return str(source.path), None
    checkout, commit = _checkout(source, workdir / "src" / source.package, log)
    return str(checkout), commit


def install_environment(
    config: AcceptanceConfig,
    workdir: Path,
    log: Callable[[str], None] = _log,
) -> Installation:
    """Create the environment and install all three packages into it."""
    missing = missing_tools(config.mode)
    if missing:
        needed = ", ".join(TOOLCHAIN[config.mode])
        raise ToolchainMissing(
            f"{', '.join(missing)} not on PATH; "
            f"the '{config.mode}' wheel-sourcing mode needs {needed}"
        )

    env = workdir / "env"
    log(f"creating a clean environment at {env}")
    _run(["uv", "venv", "--quiet", env])

    sources = config.sources()
    targets: list[str] = []
    commits: dict[str, str] = {}
    for source in sources:
        target, commit = _install_target(source, workdir, log)
        targets.append(target)
        if commit is not None:
            commits[source.package] = commit

    log(f"installing {len(targets)} packages: {', '.join(targets)}")
    _run(["uv", "pip", "install", "--python", env / "bin" / "python", *targets])

    home = workdir / "home"
    home.mkdir(exist_ok=True)
    bin_dir = env / "bin"
    return Installation(
        env=env,
        bin=bin_dir,
        path=":".join([str(bin_dir), *SYSTEM_PATH]),
        home=home,
        sources=sources,
        commits=commits,
    )
