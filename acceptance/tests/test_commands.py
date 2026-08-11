"""Running installed commands: what the harness reports when one is not there.

Needs no wheels — a command that the installation's PATH cannot resolve is the
one failure the harness must describe in the shell's own terms rather than as a
Python traceback, because "the wheel did not install its entry point" is a real
result the gate can produce.
"""

from dataclasses import replace

from uncompose_acceptance.commands import run
from uncompose_acceptance.sources import GitSource


def test_a_command_the_installation_lacks_reports_the_launcher_exit_code(empty_installation):
    result = run(empty_installation, ["uncompose-nothing", "--version"])

    assert result.returncode == 127
    assert "uncompose-nothing" in result.stderr
    assert "uncompose-nothing" in result.describe()


def test_a_command_the_installation_has_runs_under_its_own_path(empty_installation):
    installation = empty_installation
    script = installation.bin / "uncompose-echo"
    script.write_text('#!/bin/sh\necho "$PATH"\n')
    script.chmod(0o755)

    result = run(installation, ["uncompose-echo"])

    assert result.ok, result.describe()
    assert result.stdout.strip() == installation.path


def test_every_result_names_the_artifacts_it_was_produced_against(empty_installation):
    installation = replace(
        empty_installation,
        sources=(GitSource("uncompose-project", "https://example.invalid/p.git", "main"),),
        commits={"uncompose-project": "0123abc"},
    )

    described = run(installation, ["uncompose-nothing"]).describe()

    # Why the installation carries its sources at all: a failing assertion
    # prints this, so the gate names the artifacts it was testing — including
    # what a moving ref resolved to — rather than leaving that to the log.
    assert "https://example.invalid/p.git@main" in described
    assert "0123abc" in described


def test_a_command_that_complains_on_stderr_is_not_a_quiet_one(empty_installation):
    script = empty_installation.bin / "uncompose-grumble"
    script.write_text('#!/bin/sh\necho "warning: two assets share a path" >&2\n')
    script.chmod(0o755)

    result = run(empty_installation, ["uncompose-grumble"])

    # A green exit code is not the whole of a green command: the gate reads
    # stderr for complaints too, so `ok` and `quiet` are separate readings.
    assert result.ok
    assert not result.quiet
