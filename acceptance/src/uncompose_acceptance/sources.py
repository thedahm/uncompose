"""Where the three wheels under test come from.

The acceptance gate has to run twice in a release's life: before the tag, when
the only artifacts that exist are whatever the extension repos' git refs build,
and after it, against the wheels PyPI actually serves. Both runs drive the same
test body, so the difference lives here — a source per package, resolved from
the environment, and nothing else in the harness knows which mode it is in.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping, Union

MODE_BUILD = "build"
MODE_PYPI = "pypi"
MODES = (MODE_BUILD, MODE_PYPI)

PROJECT_URL = "https://github.com/thedahm/uncompose-project.git"
COMPARE_URL = "https://github.com/thedahm/uncompose-compare.git"

ROOT_PACKAGE = "uncompose"
PROJECT_PACKAGE = "uncompose-project"
COMPARE_PACKAGE = "uncompose-compare"

# uncompose-compare embeds its Vite bundle in the binary at compile time and
# its build.rs refuses to compile without `frontend/dist`, so a fresh checkout
# is not installable until the frontend is built. Node is a build-time
# requirement of the wheel, never of the installed tool.
COMPARE_PREPARE = (
    ("npm", "--prefix", "frontend", "ci", "--no-audit", "--no-fund"),
    ("npm", "--prefix", "frontend", "run", "build"),
)


@dataclass(frozen=True)
class PathSource:
    """A package built from a local checkout — the root wheel, from this repo."""

    package: str
    path: Path

    def describe(self) -> str:
        return f"{self.package} from {self.path}"


@dataclass(frozen=True)
class GitSource:
    """A package built from a git ref, with any pre-build steps it needs."""

    package: str
    url: str
    ref: str
    prepare: tuple[tuple[str, ...], ...] = ()

    def describe(self) -> str:
        return f"{self.package} from {self.url}@{self.ref}"


@dataclass(frozen=True)
class PypiSource:
    """A package installed from the index, optionally pinned to a version."""

    package: str
    version: str | None = None

    def requirement(self) -> str:
        return self.package if self.version is None else f"{self.package}=={self.version}"

    def describe(self) -> str:
        return f"{self.requirement()} from PyPI"


Source = Union[PathSource, GitSource, PypiSource]


@dataclass(frozen=True)
class AcceptanceConfig:
    """Everything the harness reads from the environment, resolved once."""

    mode: str
    checkout: Path
    project_url: str
    project_ref: str
    compare_url: str
    compare_ref: str
    root_version: str | None
    project_version: str | None
    compare_version: str | None
    required: bool

    @classmethod
    def from_env(
        cls, env: Mapping[str, str] | None = None, *, checkout: Path
    ) -> AcceptanceConfig:
        env = os.environ if env is None else env
        mode = env.get("UNCOMPOSE_ACCEPTANCE_MODE", MODE_BUILD)
        if mode not in MODES:
            raise ValueError(
                f"unknown UNCOMPOSE_ACCEPTANCE_MODE '{mode}'; expected one of {', '.join(MODES)}"
            )
        return cls(
            mode=mode,
            checkout=Path(env.get("UNCOMPOSE_ACCEPTANCE_CHECKOUT", checkout)),
            project_url=env.get("UNCOMPOSE_ACCEPTANCE_PROJECT_URL", PROJECT_URL),
            project_ref=env.get("UNCOMPOSE_ACCEPTANCE_PROJECT_REF", "main"),
            compare_url=env.get("UNCOMPOSE_ACCEPTANCE_COMPARE_URL", COMPARE_URL),
            compare_ref=env.get("UNCOMPOSE_ACCEPTANCE_COMPARE_REF", "main"),
            root_version=env.get("UNCOMPOSE_ACCEPTANCE_ROOT_VERSION") or None,
            project_version=env.get("UNCOMPOSE_ACCEPTANCE_PROJECT_VERSION") or None,
            compare_version=env.get("UNCOMPOSE_ACCEPTANCE_COMPARE_VERSION") or None,
            required=env.get("UNCOMPOSE_ACCEPTANCE_REQUIRE", "") not in ("", "0"),
        )

    def sources(self) -> tuple[Source, Source, Source]:
        """The three packages to install, root first."""
        if self.mode == MODE_PYPI:
            return (
                PypiSource(ROOT_PACKAGE, self.root_version),
                PypiSource(PROJECT_PACKAGE, self.project_version),
                PypiSource(COMPARE_PACKAGE, self.compare_version),
            )
        return (
            PathSource(ROOT_PACKAGE, self.checkout),
            GitSource(PROJECT_PACKAGE, self.project_url, self.project_ref),
            GitSource(COMPARE_PACKAGE, self.compare_url, self.compare_ref, COMPARE_PREPARE),
        )
