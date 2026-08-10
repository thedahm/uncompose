"""Session-wide wiring for the acceptance gate.

The expensive part — a clean environment with all three wheels installed — is
built once per session and shared; every test then reads what a user could
read. Nothing here imports any tool's internals.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from uncompose_acceptance.fixtures import write_slice_fixtures
from uncompose_acceptance.install import ToolchainMissing, install_environment
from uncompose_acceptance.slice import run_cli_slice
from uncompose_acceptance.sources import AcceptanceConfig

REPO_ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture(scope="session")
def repo_root() -> Path:
    return REPO_ROOT


@pytest.fixture(scope="session")
def config() -> AcceptanceConfig:
    return AcceptanceConfig.from_env(checkout=REPO_ROOT)


@pytest.fixture(scope="session")
def installation(config, tmp_path_factory):
    """All three packages installed into one clean environment.

    A machine without the toolchain the configured mode needs skips rather than
    fails — until `UNCOMPOSE_ACCEPTANCE_REQUIRE` is set, which is how CI states
    that a gate which cannot run is a failed gate, not a green one.
    """
    workdir = tmp_path_factory.mktemp("installation")
    try:
        return install_environment(config, workdir)
    except ToolchainMissing as missing:
        if config.required:
            pytest.fail(f"acceptance gate cannot run: {missing}")
        pytest.skip(str(missing))


@pytest.fixture(scope="session")
def cli_slice(installation, tmp_path_factory):
    """The vertical slice, driven once through the installed commands."""
    project = tmp_path_factory.mktemp("project")
    fixtures = write_slice_fixtures(project, repo_root=REPO_ROOT)
    return run_cli_slice(installation, project, fixtures)
