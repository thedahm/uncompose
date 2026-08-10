"""Running installed commands: what the harness reports when one is not there.

Needs no wheels — a command that the installation's PATH cannot resolve is the
one failure the harness must describe in the shell's own terms rather than as a
Python traceback, because "the wheel did not install its entry point" is a real
result the gate can produce.
"""

from uncompose_acceptance.commands import run


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
