"""The browser leg's seams, exercised without wheels, a browser, or a network.

Everything here is either pure reading — which refs the manifest says to
compare, what the written record says the listener decided — or the plumbing
that launches a served session and catches what it printed. The Chromium drive
itself is the one part these cannot reach; it is stood in for by a fake driver,
the same way `test_commands.py` stands in for an installation.
"""

import json
from pathlib import Path

import pytest

from uncompose_acceptance.workbench import (
    LOOPBACK,
    WorkbenchError,
    candidate_refs,
    read_verdict,
    run_workbench_leg,
    spawn_served,
    workbench_argv,
)


def manifest(*derivations, assets=None):
    """A manifest holding `derivations`, each `(id, [(asset_id, path)])`."""
    if assets is None:
        assets = [
            {"id": asset_id, "path": path, "role": "stem"}
            for _, outputs in derivations
            for asset_id, path in outputs
        ]
    return {
        "project": {"id": "01PROJECT"},
        "assets": assets,
        "derivations": [
            {"id": deriv_id, "inputs": ["demo-song"], "outputs": [a for a, _ in outputs]}
            for deriv_id, outputs in derivations
        ],
        "evaluations": [],
    }


TWO_RUNS = manifest(
    ("htdemucs-6s", [("vocals", "runs/htdemucs-6s/vocals.wav"), ("drums", "runs/htdemucs-6s/drums.wav")]),
    ("roformer-2stem", [("vocals-2", "runs/roformer-2stem/vocals.wav")]),
)


def test_the_two_candidates_are_each_derivations_stem_named_by_its_derivation():
    # The grammar the walkthrough teaches: `<name>@<derivation>` picks that
    # stem among one derivation's outputs, so the same name names both runs.
    assert candidate_refs(TWO_RUNS, stem="vocals") == (
        "vocals@htdemucs-6s",
        "vocals@roformer-2stem",
    )


def test_a_derivation_without_the_stem_is_named_with_what_it_does_have():
    without = manifest(
        ("htdemucs-6s", [("vocals", "runs/htdemucs-6s/vocals.wav")]),
        ("roformer-2stem", [("instrumental", "runs/roformer-2stem/instrumental.wav")]),
    )

    with pytest.raises(WorkbenchError) as excinfo:
        candidate_refs(without, stem="vocals")

    message = str(excinfo.value)
    assert "roformer-2stem" in message and "instrumental.wav" in message


def test_a_project_without_two_derivations_cannot_furnish_two_candidates():
    one = manifest(("htdemucs-6s", [("vocals", "runs/htdemucs-6s/vocals.wav")]))

    with pytest.raises(WorkbenchError) as excinfo:
        candidate_refs(one, stem="vocals")

    assert "two derivations" in str(excinfo.value)


def test_the_session_is_launched_blind_through_the_root_dispatch():
    argv = workbench_argv("/tmp/project", ("vocals@a", "vocals@b"))

    # Root dispatch rather than the standalone command: the browser leg is also
    # where `uncompose compare` proves it reaches the installed extension.
    assert argv[:2] == ["uncompose", "compare"]
    assert argv[2:4] == ["--project", "/tmp/project"]
    assert argv[4:6] == ["vocals@a", "vocals@b"]
    assert "--blind" in argv


def shell(script):
    return ["/bin/sh", "-c", script]


def test_a_served_session_reports_the_loopback_url_it_printed(tmp_path):
    served = spawn_served(
        shell(f"echo '{LOOPBACK}5173/?token=abc'; sleep 30"),
        cwd=tmp_path,
        env={},
        stderr_path=tmp_path / "stderr",
    )
    try:
        assert served.url == f"{LOOPBACK}5173/?token=abc"
    finally:
        served.kill()


def test_a_session_that_prints_something_else_is_not_a_served_workbench(tmp_path):
    with pytest.raises(WorkbenchError) as excinfo:
        spawn_served(
            shell("echo http://example.invalid/; sleep 30"),
            cwd=tmp_path,
            env={},
            stderr_path=tmp_path / "stderr",
        )

    assert "http://example.invalid/" in str(excinfo.value)


def test_a_session_that_dies_first_reports_its_exit_code_and_stderr(tmp_path):
    with pytest.raises(WorkbenchError) as excinfo:
        spawn_served(
            shell("echo 'no asset with slug' >&2; exit 3"),
            cwd=tmp_path,
            env={},
            stderr_path=tmp_path / "stderr",
        )

    message = str(excinfo.value)
    assert "3" in message
    assert "no asset with slug" in message


def test_a_session_that_says_nothing_is_given_up_on(tmp_path):
    with pytest.raises(WorkbenchError) as excinfo:
        spawn_served(
            shell("sleep 30"),
            cwd=tmp_path,
            env={},
            stderr_path=tmp_path / "stderr",
            timeout=0.5,
        )

    assert "0.5" in str(excinfo.value)


def fake_session(installation, *, closing="exit 0", stderr=""):
    """A stub `uncompose` that serves until something concludes it, like the real one.

    It logs the argv it was handed, prints a loopback URL, and waits for the
    driver's conclude before exiting — so the leg's launch, its capture of the
    URL, and its wait for the exit are all exercised without a wheel.
    """
    stub = installation.bin / "uncompose"
    stub.write_text(
        "#!/bin/sh\n"
        'printf "%s\\n" "$@" > "$HOME/argv"\n'
        f'echo "{LOOPBACK}9000/?token=stub"\n'
        'while [ ! -f "$HOME/concluded" ]; do sleep 0.05; done\n'
        f'{f"echo {stderr!r} >&2" if stderr else ""}\n'
        f"{closing}\n"
    )
    stub.chmod(0o755)
    return stub


def fake_driver(installation, record_name="01HF8Z9K2M4P6R8T0V2X4Z6A8C.json"):
    """A driver that concludes the session and reports what the page showed."""
    from uncompose_acceptance.workbench import BrowserRun

    def drive(url):
        (installation.home / "concluded").write_text("")
        return BrowserRun(
            url=url,
            pin_text="cleaner top end here",
            preferred_label="A",
            confidence=4,
            record_path=Path("/p/evaluations") / record_name,
            reveal={"A": "A was /p/x.wav", "B": "B was /p/y.wav"},
            registered=True,
            before_conclude="A B",
            after_conclude="registered",
        )

    return drive


def test_the_leg_launches_the_session_a_listener_launches(empty_installation):
    fake_session(empty_installation)

    leg = run_workbench_leg(
        empty_installation,
        empty_installation.home,
        ("vocals@htdemucs-6s", "vocals@roformer-2stem"),
        driver=fake_driver(empty_installation),
    )

    argv = (empty_installation.home / "argv").read_text().split()
    assert argv[:2] == ["compare", "--project"]
    assert argv[3:] == ["vocals@htdemucs-6s", "vocals@roformer-2stem", "--blind"]
    assert leg.browser.url.startswith(LOOPBACK)


def test_a_concluded_session_closes_itself_and_the_leg_reports_that(empty_installation):
    fake_session(empty_installation)

    leg = run_workbench_leg(
        empty_installation,
        empty_installation.home,
        ("a", "b"),
        driver=fake_driver(empty_installation),
    )

    # Exit 0 is Compare's own statement that the record was saved *and*
    # registered, so the leg waits for the close rather than killing a server.
    assert leg.returncode == 0
    assert leg.browser.registered


def test_a_session_that_could_not_register_hands_back_its_exit_and_stderr(empty_installation):
    fake_session(empty_installation, closing="exit 1", stderr="could not run import")

    leg = run_workbench_leg(
        empty_installation,
        empty_installation.home,
        ("a", "b"),
        driver=fake_driver(empty_installation),
    )

    assert leg.returncode == 1
    assert "could not run import" in leg.stderr
    assert "could not run import" in leg.describe()


def record(preference="A", confidence=4):
    """A blind comparison record as Compare writes one in project mode."""
    return {
        "schema": "https://uncompose.org/schemas/compare/v0/uncompose.compare.schema.json",
        "id": "01HF8Z9K2M4P6R8T0V2X4Z6A8C",
        "mode": "ab-blind-randomized",
        "candidates": [
            {
                "label": "A",
                "path": "/p/runs/roformer-2stem/vocals.wav",
                "sha256": "b" * 64,
                "size": 8044,
                "asset": "vocals-2",
                "project": "01PROJECT",
            },
            {
                "label": "B",
                "path": "/p/runs/htdemucs-6s/vocals.wav",
                "sha256": "a" * 64,
                "size": 8044,
                "asset": "vocals",
                "project": "01PROJECT",
            },
        ],
        "observations": [{"text": "cleaner top end here", "candidate": "A"}],
        "result": {"preference": preference, "confidence": confidence},
    }


def test_the_record_reconnects_each_label_to_the_asset_behind_it():
    verdict = read_verdict(record())

    # The shuffle put the second run behind label A; the record is the only
    # place that says so, which is what makes the verdict blind.
    assert verdict.blind
    assert verdict.labels == ("A", "B")
    assert verdict.assets == {"A": "vocals-2", "B": "vocals"}
    assert verdict.paths["A"].endswith("roformer-2stem/vocals.wav")
    assert verdict.preferred_asset == "vocals-2"
    assert verdict.confidence == 4


def test_a_sighted_record_is_not_a_blind_one():
    sighted = record()
    sighted["mode"] = "ab"

    assert read_verdict(sighted).blind is False


def test_a_record_with_no_preference_prefers_no_asset():
    inconclusive = record(preference=None, confidence=None)
    del inconclusive["result"]["confidence"]

    verdict = read_verdict(inconclusive)

    assert verdict.preferred_label is None
    assert verdict.preferred_asset is None


def test_a_preference_naming_no_candidate_is_refused_rather_than_guessed():
    with pytest.raises(WorkbenchError) as excinfo:
        read_verdict(record(preference="C"))

    assert "C" in str(excinfo.value)


def test_a_record_can_be_read_from_the_file_the_session_wrote(tmp_path):
    path = tmp_path / "01HF8Z9K2M4P6R8T0V2X4Z6A8C.json"
    path.write_text(json.dumps(record()))

    assert read_verdict(json.loads(path.read_text())).preferred_asset == "vocals-2"
