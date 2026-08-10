"""The vertical slice, driven once through the installed commands.

Every step is typed the way the walkthrough types it. The two imports go
through different entry points on purpose — one through the root's dispatch,
one through the standalone command — because the family's promise is that both
reach the same tool and the same manifest.

The slice runs in two legs over one project: the CLI leg lands the source, the
two derivations and their stems, and the browser leg adds the verdict that
completes the story. They are separate so a machine without a browser still
gates everything up to the comparison.

Nothing here asserts; it records what each command did (argv, exit code,
stdout, stderr) and leaves the manifest on disk. The tests read those, the
manifest, and the record file, exactly as a user could.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Mapping

from .commands import CommandResult, run
from .fixtures import SliceFixtures
from .install import Installation
from .workbench import (
    BlindVerdict,
    WorkbenchLeg,
    candidate_refs,
    read_verdict,
    run_workbench_leg,
)

MANIFEST_NAME = "uncompose.project.json"

EXTENSIONS = ("project", "compare")


Cli = Callable[..., CommandResult]


def cli_runner(
    installation: Installation, project: Path, steps: list[CommandResult] | None = None
) -> Cli:
    """Run commands inside `installation` from `project`, filing each in `steps`."""

    def cli(*argv: str) -> CommandResult:
        result = run(installation, list(argv), cwd=project)
        if steps is not None:
            steps.append(result)
        return result

    return cli


def read_project_back(cli: Cli, project: Path) -> tuple[CommandResult, CommandResult]:
    """`verify` then `show --json`, in that order — the one reading of a project.

    `verify` stamps `last_verified` on every asset that passes, so it is a
    manifest write like any other. Running it first means `show` prints a
    settled manifest rather than one about to be stamped again. Both legs read
    the project this way, so the ordering is stated once, here.
    """
    verify = cli("uncompose-project", "verify", "--project", str(project))
    show_json = cli("uncompose", "project", "show", "--json", "--project", str(project))
    return verify, show_json


def dispatch_forms(extension: str) -> dict[str, tuple[str, ...]]:
    """The two ways an extension is reached (uncompose ADR-0005), dispatch first.

    Keyed by the command line a user would type — the same string the slice
    files each version result under.
    """
    return {
        f"uncompose {extension}": ("uncompose", extension),
        f"uncompose-{extension}": (f"uncompose-{extension}",),
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
    manifest_when_shown: dict
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
    cli = cli_runner(installation, project, steps)

    versions = {"uncompose": cli("uncompose", "--version")}
    for extension in EXTENSIONS:
        for name, form in dispatch_forms(extension).items():
            versions[name] = cli(*form, "--version")

    init = cli("uncompose", "project", "init", "--project", str(project))

    # One import per entry point, so a form that never reaches the manifest
    # cannot hide behind the other; the strict zip refuses a run without a form
    # rather than quietly leaving it unimported.
    imports = {
        fixture.slug: cli(*form, "import", "--project", str(project), str(fixture.job_json))
        for fixture, form in zip(
            fixtures.runs, dispatch_forms("project").values(), strict=True
        )
    }

    verify, show_json = read_project_back(cli, project)
    manifest_path = project / MANIFEST_NAME

    return CliSlice(
        project=project,
        fixtures=fixtures,
        manifest_path=manifest_path,
        versions=versions,
        init=init,
        imports=imports,
        show_json=show_json,
        # What was on disk when `show` ran: the browser leg registers an
        # evaluation into this same manifest afterwards, so comparing the two
        # has to be a comparison against that moment.
        manifest_when_shown=json.loads(manifest_path.read_text()),
        verify=verify,
        steps=tuple(steps),
        installation=installation,
    )


@dataclass(frozen=True)
class WholeStory:
    """The finished project: the CLI leg, the blind session, and what followed.

    `verify` and `show` are run again after the verdict lands, because the
    story the gate checks is the one the manifest tells at the end — with the
    evaluation in it.
    """

    cli: CliSlice
    leg: WorkbenchLeg
    refs: tuple[str, str]
    verify: CommandResult
    show_json: CommandResult

    @property
    def project(self) -> Path:
        return self.cli.project

    def manifest(self) -> dict:
        return self.cli.manifest()

    def record_path(self) -> Path:
        """The record file, as the closing screen reported it to the listener."""
        return self.leg.browser.record_path

    def record(self) -> dict:
        return json.loads(self.record_path().read_text())

    def verdict(self) -> BlindVerdict:
        return read_verdict(self.record())

    def evaluations(self) -> list[dict]:
        return self.manifest()["evaluations"]

    def describe(self) -> str:
        return self.leg.describe()


def tell_whole_story(cli_slice: CliSlice) -> WholeStory:
    """Compare the two runs blind, then read the project back the way `show` does."""
    installation, project = cli_slice.installation, cli_slice.project
    refs = candidate_refs(cli_slice.manifest())
    leg = run_workbench_leg(installation, project, refs)

    verify, show_json = read_project_back(cli_runner(installation, project), project)

    return WholeStory(cli=cli_slice, leg=leg, refs=refs, verify=verify, show_json=show_json)
