"""Running the installed commands the way a user runs them.

Every invocation is a subprocess against the installed environment's `bin`,
under a home directory inside the test's own temporary tree, so a gate run
leaves nothing on the machine and reads nothing the machine already had.
"""

from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path

from .install import Installation


@dataclass(frozen=True)
class CommandResult:
    argv: tuple[str, ...]
    returncode: int
    stdout: str
    stderr: str

    @property
    def ok(self) -> bool:
        return self.returncode == 0

    def describe(self) -> str:
        return (
            f"$ {' '.join(self.argv)}\n"
            f"exit {self.returncode}\n"
            f"--- stdout ---\n{self.stdout}\n--- stderr ---\n{self.stderr}"
        )


def environment(installation: Installation) -> dict[str, str]:
    """The whole environment an installed command is given — nothing inherited.

    Shared with the browser leg, which launches a serving command of its own
    (`workbench.py`), so both legs run the installed artifacts under identical
    conditions.
    """
    return {
        "PATH": installation.path,
        "HOME": str(installation.home),
        # Keep every cache and state directory the family might use inside the
        # run's own tree: the gate must not read or write the machine's.
        "XDG_CACHE_HOME": str(installation.home / "cache"),
        "XDG_STATE_HOME": str(installation.home / "state"),
        "XDG_DATA_HOME": str(installation.home / "data"),
        "XDG_CONFIG_HOME": str(installation.home / "config"),
        # Comparison and manifest timestamps are rendered as UTC; pinning the
        # zone keeps a failure message the same on every machine.
        "TZ": "UTC",
    }


def run(
    installation: Installation,
    argv: list[str],
    *,
    cwd: Path | None = None,
    timeout: float = 120.0,
) -> CommandResult:
    """Run one command inside the installation, capturing what a user would see."""
    env = environment(installation)
    try:
        completed = subprocess.run(
            argv,
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except FileNotFoundError:
        # A package that installed without its entry point is a result the gate
        # can produce, so report it the way a shell does — 127, on stderr —
        # rather than as a traceback from the harness.
        return CommandResult(
            argv=tuple(argv),
            returncode=127,
            stdout="",
            stderr=f"{argv[0]}: not found in the installation",
        )
    return CommandResult(
        argv=tuple(argv),
        returncode=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
    )
