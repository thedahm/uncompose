"""Session-wide wiring for the acceptance gate.

The expensive parts — a clean environment with all three wheels installed, and
the one slice driven through it — are built once per session and shared; every
test then reads what a user could read. Nothing here imports any tool's
internals.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from uncompose_acceptance.fixtures import write_slice_fixtures
from uncompose_acceptance.install import (
    SYSTEM_PATH,
    Installation,
    ToolchainMissing,
    install_environment,
)
from uncompose_acceptance.slice import run_cli_slice, tell_whole_story
from uncompose_acceptance.sources import AcceptanceConfig

REPO_ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture(scope="session")
def repo_root() -> Path:
    return REPO_ROOT


@pytest.fixture(scope="session")
def config() -> AcceptanceConfig:
    return AcceptanceConfig.from_env(checkout=REPO_ROOT)


def gated(config: AcceptanceConfig, build):
    """Run one step of the gate, deciding what a missing toolchain means.

    A machine without what the step needs skips rather than fails — until
    `UNCOMPOSE_ACCEPTANCE_REQUIRE` is set, which is how CI states that a gate
    which cannot run is a failed gate, not a green one.
    """
    try:
        return build()
    except ToolchainMissing as missing:
        if config.required:
            pytest.fail(f"acceptance gate cannot run: {missing}")
        pytest.skip(str(missing))


@pytest.fixture(scope="session")
def installation(config, tmp_path_factory):
    """All three packages installed into one clean environment."""
    workdir = tmp_path_factory.mktemp("installation")
    return gated(config, lambda: install_environment(config, workdir))


@pytest.fixture(scope="session")
def cli_slice(installation, tmp_path_factory):
    """The vertical slice, driven once through the installed commands."""
    project = tmp_path_factory.mktemp("project")
    fixtures = write_slice_fixtures(project, repo_root=REPO_ROOT)
    return run_cli_slice(installation, project, fixtures)


@pytest.fixture(scope="session")
def whole_story(config, cli_slice):
    """The same project after the browser leg: a verdict, then verify and show.

    Its own gate: a machine with the wheels but no browser skips the browser
    leg alone, leaving the CLI leg's tests to run.
    """
    return gated(config, lambda: tell_whole_story(cli_slice))


@pytest.fixture
def empty_installation(tmp_path) -> Installation:
    """An installation with nothing installed into it — no wheels, no toolchain.

    Enough for the harness's own tests: commands resolve through its `bin`, so
    a stub dropped there stands in for an installed one.
    """
    bin_dir = tmp_path / "env" / "bin"
    bin_dir.mkdir(parents=True)
    home = tmp_path / "home"
    home.mkdir()
    return Installation(
        env=tmp_path / "env",
        bin=bin_dir,
        path=":".join([str(bin_dir), *SYSTEM_PATH]),
        home=home,
        sources=(),
        commits={},
    )
