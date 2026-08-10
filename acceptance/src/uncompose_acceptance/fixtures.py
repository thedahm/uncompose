"""The audio and the finished job folders the gate feeds to the installed tools.

Both come from the repo's own demonstration script (`demo/build_demo.py`),
loaded by path rather than reimplemented: it already writes deterministic
seeded noise and job folders shaped exactly like a completed `uncompose`
separation, and one generator means the gate and the demo cannot drift into
disagreeing about what a job folder is. Loading a script from this repo is not
the same as importing a tool's internals — the tools under test are only ever
reached as installed commands.
"""

from __future__ import annotations

import importlib.util
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType


@dataclass(frozen=True)
class FixtureRun:
    """One synthetic separation: the job folder an import will register."""

    slug: str
    job_folder: Path
    job_json: Path
    stems: tuple[str, ...]


@dataclass(frozen=True)
class SliceFixtures:
    root: Path
    source: Path
    runs: tuple[FixtureRun, ...]

    @property
    def stem_count(self) -> int:
        return sum(len(run.stems) for run in self.runs)


def load_demo_module(repo_root: Path) -> ModuleType:
    script = repo_root / "demo" / "build_demo.py"
    spec = importlib.util.spec_from_file_location("uncompose_demo_build", script)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load the demonstration script at {script}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_slice_fixtures(root: Path, *, repo_root: Path) -> SliceFixtures:
    """Write the source recording and both synthetic runs inside `root`."""
    demo = load_demo_module(repo_root)
    job_folders = demo.generate_fixtures(root)
    runs = tuple(
        FixtureRun(
            slug=run["slug"],
            job_folder=folder,
            job_json=folder / "job.json",
            stems=tuple(run["stems"]),
        )
        for run, folder in zip(demo.RUNS, job_folders, strict=True)
    )
    return SliceFixtures(root=root, source=root / demo.SOURCE_NAME, runs=runs)
