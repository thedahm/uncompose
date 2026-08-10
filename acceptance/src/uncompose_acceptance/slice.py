"""The vertical slice, driven once through the installed commands.

Every step is typed the way the walkthrough types it. The two imports go
through different entry points on purpose — one through the root's dispatch,
one through the standalone command — because the family's promise is that both
reach the same tool and the same manifest.

Nothing here asserts; it records what each command did (argv, exit code,
stdout, stderr) and leaves the manifest on disk. The tests read those, and the
manifest, exactly as a user could.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

from .commands import CommandResult, run
from .fixtures import SliceFixtures
from .install import Installation

MANIFEST_NAME = "uncompose.project.json"

# The two ways an extension is reached (uncompose ADR-0005): through the root
# CLI's dispatch, and as the standalone command the wheel installs.
DISPATCH_FORMS: Mapping[str, tuple[str, ...]] = {
    "uncompose project": ("uncompose", "project"),
    "uncompose-project": ("uncompose-project",),
}


@dataclass(frozen=True)
class CliSlice:
    project: Path
    fixtures: SliceFixtures
    manifest_path: Path
    versions: Mapping[str, CommandResult]
    init: CommandResult
    imports: Mapping[str, CommandResult]
    show_json: CommandResult
    verify: CommandResult
    steps: tuple[CommandResult, ...]
    installation: Installation

    def manifest(self) -> dict:
        """The manifest as it sits on disk — the file a user would open."""
        return json.loads(self.manifest_path.read_text())

    def assets_by_id(self) -> dict[str, dict]:
        return {asset["id"]: asset for asset in self.manifest()["assets"]}


def run_cli_slice(
    installation: Installation, project: Path, fixtures: SliceFixtures
) -> CliSlice:
    steps: list[CommandResult] = []

    def cli(*argv: str) -> CommandResult:
        result = run(installation, list(argv), cwd=project)
        steps.append(result)
        return result

    versions = {
        "uncompose": cli("uncompose", "--version"),
        **{name: cli(*form, "--version") for name, form in DISPATCH_FORMS.items()},
        "uncompose compare": cli("uncompose", "compare", "--version"),
        "uncompose-compare": cli("uncompose-compare", "--version"),
    }

    init = cli("uncompose", "project", "init", "--project", str(project))

    # One import per entry point, so a form that never reaches the manifest
    # cannot hide behind the other.
    imports = {
        fixture.slug: cli(*form, "import", "--project", str(project), str(fixture.job_json))
        for fixture, form in zip(fixtures.runs, DISPATCH_FORMS.values())
    }

    # `verify` stamps `last_verified` on every asset that passes, so it is a
    # manifest write like any other. Running it before `show` leaves the file
    # settled: what `show --json` printed is still what is on disk when the
    # tests read it.
    verify = cli("uncompose-project", "verify", "--project", str(project))
    show_json = cli("uncompose", "project", "show", "--json", "--project", str(project))

    return CliSlice(
        project=project,
        fixtures=fixtures,
        manifest_path=project / MANIFEST_NAME,
        versions=versions,
        init=init,
        imports=imports,
        show_json=show_json,
        verify=verify,
        steps=tuple(steps),
        installation=installation,
    )
