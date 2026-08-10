"""The fixtures the CLI leg drives: a source recording and two fake separations.

Seeded noise standing in for audio and hand-built job folders standing in for
separation runs (the #73 approach) is what keeps this gate fast enough to run
on every change: no model weights, no GPU, no engine. The generator is the
repo's own demonstration script, so the gate and the demo can never disagree
about what a finished job folder looks like.
"""

import hashlib
import json

from uncompose_acceptance.fixtures import write_slice_fixtures


def test_fixtures_land_a_source_and_two_synthetic_runs_inside_the_project(tmp_path, repo_root):
    fixtures = write_slice_fixtures(tmp_path, repo_root=repo_root)

    assert fixtures.source.parent == tmp_path
    assert fixtures.source.read_bytes()[:4] == b"RIFF"
    assert len(fixtures.runs) == 2
    assert len({run.slug for run in fixtures.runs}) == 2
    for run in fixtures.runs:
        assert run.job_json == run.job_folder / "job.json"
        assert {path.name for path in run.job_folder.glob("*.wav")} == {
            f"{stem}.wav" for stem in run.stems
        }
        assert run.job_folder.is_relative_to(tmp_path)


def test_every_job_record_names_its_source_by_a_root_relative_path(tmp_path, repo_root):
    fixtures = write_slice_fixtures(tmp_path, repo_root=repo_root)
    source_sha256 = hashlib.sha256(fixtures.source.read_bytes()).hexdigest()

    for run in fixtures.runs:
        record = json.loads(run.job_json.read_text())
        # import resolves a job's input against the project root and refuses
        # absolute paths, so a fixture recording one would fail the same way a
        # real run does.
        assert record["input_path"] == fixtures.source.name
        assert record["input_sha256"] == source_sha256
        assert record["outcome"] == "success"
        assert record["stems"] == list(run.stems)


def test_the_two_runs_produce_the_stems_the_manifest_should_end_up_with(tmp_path, repo_root):
    fixtures = write_slice_fixtures(tmp_path, repo_root=repo_root)

    written = [path for run in fixtures.runs for path in run.job_folder.glob("*.wav")]
    assert fixtures.stem_count == len(written)
    assert fixtures.stem_count > 2, "a one-stem-each pair would not exercise much"
