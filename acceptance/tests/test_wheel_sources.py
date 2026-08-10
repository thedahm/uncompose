"""The wheel-sourcing seam: where each of the three packages comes from.

Pure configuration, so these run anywhere — no network, no toolchain, no
installed wheels. The gate has to be runnable before a tag exists (build the
extensions from git refs, the root from the checkout) and again afterwards
against what PyPI actually serves; both modes name the same three packages and
feed the same test body.
"""

from pathlib import Path

import pytest

from uncompose_acceptance.sources import (
    COMPARE_URL,
    PROJECT_URL,
    AcceptanceConfig,
    GitSource,
    PathSource,
    PypiSource,
)


def config(env: dict[str, str], root: Path) -> AcceptanceConfig:
    return AcceptanceConfig.from_env(env, checkout=root)


def test_default_mode_builds_extensions_from_git_and_the_root_from_the_checkout(tmp_path):
    root, project, compare = config({}, tmp_path).sources()

    assert root == PathSource("uncompose", tmp_path)
    assert isinstance(project, GitSource) and project.url == PROJECT_URL
    assert isinstance(compare, GitSource) and compare.url == COMPARE_URL
    assert (project.ref, compare.ref) == ("main", "main")


def test_git_refs_and_urls_are_configurable_per_extension(tmp_path):
    _, project, compare = config(
        {
            "UNCOMPOSE_ACCEPTANCE_PROJECT_REF": "v0.1.0",
            "UNCOMPOSE_ACCEPTANCE_COMPARE_REF": "0f1e2d3",
            "UNCOMPOSE_ACCEPTANCE_COMPARE_URL": "https://example.invalid/fork.git",
        },
        tmp_path,
    ).sources()

    assert project.ref == "v0.1.0"
    assert (compare.ref, compare.url) == ("0f1e2d3", "https://example.invalid/fork.git")


def test_only_compare_needs_a_frontend_build_before_its_wheel(tmp_path):
    _, project, compare = config({}, tmp_path).sources()

    # uncompose-compare embeds its Vite bundle at compile time and its build.rs
    # refuses to compile without it, so a git checkout is not installable until
    # the frontend is built. uncompose-project has no such step.
    assert project.prepare == ()
    assert compare.prepare, "compare must build its frontend before the wheel"
    assert all(command[0] == "npm" for command in compare.prepare)
    assert any("build" in command for command in compare.prepare)


def test_pypi_mode_installs_all_three_from_the_index(tmp_path):
    sources = config({"UNCOMPOSE_ACCEPTANCE_MODE": "pypi"}, tmp_path).sources()

    assert [source.package for source in sources] == [
        "uncompose",
        "uncompose-project",
        "uncompose-compare",
    ]
    assert all(isinstance(source, PypiSource) for source in sources)
    assert [source.requirement() for source in sources] == [
        "uncompose",
        "uncompose-project",
        "uncompose-compare",
    ]


def test_pypi_mode_pins_versions_when_given(tmp_path):
    sources = config(
        {
            "UNCOMPOSE_ACCEPTANCE_MODE": "pypi",
            "UNCOMPOSE_ACCEPTANCE_ROOT_VERSION": "0.1.0",
            "UNCOMPOSE_ACCEPTANCE_PROJECT_VERSION": "0.1.0",
        },
        tmp_path,
    ).sources()

    assert [source.requirement() for source in sources] == [
        "uncompose==0.1.0",
        "uncompose-project==0.1.0",
        "uncompose-compare",
    ]


def test_an_unknown_mode_is_refused_by_name(tmp_path):
    with pytest.raises(ValueError) as excinfo:
        config({"UNCOMPOSE_ACCEPTANCE_MODE": "wheelhouse"}, tmp_path)

    message = str(excinfo.value)
    assert "wheelhouse" in message and "build" in message and "pypi" in message


def test_the_gate_can_be_told_never_to_skip(tmp_path):
    assert config({}, tmp_path).required is False
    assert config({"UNCOMPOSE_ACCEPTANCE_REQUIRE": "1"}, tmp_path).required is True
