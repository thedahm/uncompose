"""Running installed commands: what the harness reports when one is not there.

Needs no wheels — a command that the installation's PATH cannot resolve is the
one failure the harness must describe in the shell's own terms rather than as a
Python traceback, because "the wheel did not install its entry point" is a real
result the gate can produce.
"""

from uncompose_acceptance.commands import run
from uncompose_acceptance.install import SYSTEM_PATH, Installation


def installation(tmp_path) -> Installation:
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


def test_a_command_the_installation_lacks_reports_the_launcher_exit_code(tmp_path):
    result = run(installation(tmp_path), ["uncompose-nothing", "--version"])

    assert result.returncode == 127
    assert "uncompose-nothing" in result.stderr
    assert "uncompose-nothing" in result.describe()


def test_a_command_the_installation_has_runs_under_its_own_path(tmp_path):
    install = installation(tmp_path)
    script = install.bin / "uncompose-echo"
    script.write_text('#!/bin/sh\necho "$PATH"\n')
    script.chmod(0o755)

    result = run(install, ["uncompose-echo"])

    assert result.ok, result.describe()
    assert result.stdout.strip() == install.path
