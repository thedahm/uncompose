"""The whole story: the same project after a blind verdict was engraved in the browser.

Where the CLI leg stops at two derivations and a green `verify`, these read the
finished manifest — the outcome the milestone promises, not the steps that got
there: one source asset, two derivations, every stem registered, one evaluation
holding a hashed ref to the record file under `evaluations/`, `verify` still
green over all of it, and the blind labels the listener judged reconnected to
the assets behind them.

Everything asserted is what a user can read: the rendered page, the record
file, the manifest, and exit codes.
"""

import hashlib
import json

from uncompose_acceptance.workbench import CONFIDENCE, PIN_TEXT, PREFERRED_LABEL


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def test_the_session_served_the_workbench_and_closed_itself_on_the_verdict(whole_story):
    # Save-and-close: a project session that concludes writes its record, hands
    # it to `uncompose-project import`, and exits — 0 means saved *and*
    # registered, which is the whole handover in one exit code.
    assert whole_story.leg.returncode == 0, whole_story.describe()
    assert whole_story.leg.browser.registered, whole_story.describe()
    assert whole_story.leg.served.url.startswith("http://127.0.0.1:")


def test_the_record_landed_in_the_projects_evaluations_folder(whole_story):
    record_path = whole_story.record_path()

    assert record_path.parent == whole_story.project / "evaluations", whole_story.describe()
    assert record_path.is_file()
    assert record_path.suffix == ".json"


def test_the_page_named_neither_candidate_until_the_reveal(whole_story):
    verdict = whole_story.verdict()
    seen = whole_story.leg.browser.before_conclude

    # Blind means blind: nothing identifying either file — path, basename,
    # hash, or size — was on the page the verdict was engraved from.
    for label in verdict.labels:
        path = verdict.paths[label]
        assert path not in seen
        assert path.rsplit("/", 1)[-1] not in seen
    for candidate in whole_story.record()["candidates"]:
        assert candidate["sha256"] not in seen
        assert str(candidate["size"]) not in seen

    # The reveal, after the record was written, names each label's real file.
    for label in verdict.labels:
        assert verdict.paths[label] in whole_story.leg.browser.reveal[label]


def test_the_pin_and_the_verdict_the_listener_engraved_are_in_the_record(whole_story):
    record = whole_story.record()

    assert record["mode"] == "ab-blind-randomized"
    assert [observation["text"] for observation in record["observations"]] == [PIN_TEXT]
    assert record["result"]["preference"] == PREFERRED_LABEL
    assert record["result"]["confidence"] == CONFIDENCE
    # A project-launched record: every candidate names the asset it came from,
    # which is what lets the manifest register the verdict at all.
    assert all(candidate["asset"] for candidate in record["candidates"])


def test_the_manifest_holds_one_evaluation_referencing_the_record_by_hash(whole_story):
    evaluations = whole_story.evaluations()
    assert len(evaluations) == 1, f"expected one evaluation, got {evaluations}"

    reference = evaluations[0]["record"]
    record_path = whole_story.record_path()
    # A hashed ref: the record is referenced in place, never absorbed, so the
    # path is root-relative and the hash is of the file as written.
    assert reference["path"] == record_path.relative_to(whole_story.project).as_posix()
    assert reference["sha256"] == sha256(record_path)


def test_the_blind_labels_are_reconnected_to_the_assets_behind_them(whole_story):
    verdict = whole_story.verdict()
    evaluation = whole_story.evaluations()[0]
    assets = whole_story.cli.assets_by_id()

    # The listener judged A and B; the manifest records asset ids. The record is
    # the only place the two meet, and the registered evaluation must agree with
    # it — candidates in the record's own order, preference resolved through the
    # label that was preferred.
    assert list(evaluation["candidates"]) == list(verdict.candidate_assets)
    assert evaluation["preference"] == verdict.preferred_asset
    assert evaluation["confidence"] == verdict.confidence

    # And the file behind the preferred label is the file that asset names.
    preferred = assets[evaluation["preference"]]
    assert verdict.paths[verdict.preferred_label].endswith(preferred["path"])


def test_the_evaluation_compares_the_two_runs_takes_of_the_same_stem(whole_story):
    evaluation = whole_story.evaluations()[0]
    manifest = whole_story.manifest()
    assets = whole_story.cli.assets_by_id()

    compared = set(evaluation["candidates"])
    assert len(compared) == 2
    assert {assets[asset_id]["path"].rsplit("/", 1)[-1] for asset_id in compared} == {"vocals.wav"}
    # One take from each run: a verdict over two outputs of the same derivation
    # would compare a run against itself.
    producers = [
        derivation["id"]
        for derivation in manifest["derivations"]
        if compared & set(derivation["outputs"])
    ]
    assert sorted(producers) == sorted(ref.split("@", 1)[1] for ref in whole_story.refs)


def test_verify_is_still_green_with_the_evaluation_in_the_manifest(whole_story):
    result = whole_story.verify
    assert result.ok, result.describe()
    assert "error" not in result.stderr and "warning" not in result.stderr, result.describe()
    # `verify` walks the record it now references, not only the audio.
    assert whole_story.evaluations()[0]["record"]["path"] in result.stdout, result.describe()


def test_the_manifest_tells_the_whole_story(whole_story):
    manifest = whole_story.manifest()
    fixtures = whole_story.cli.fixtures

    assert whole_story.show_json.ok, whole_story.show_json.describe()
    assert json.loads(whole_story.show_json.stdout) == manifest

    roles = [asset["role"] for asset in manifest["assets"]]
    assert roles.count("mix") == 1
    assert roles.count("stem") == fixtures.stem_count
    assert len(manifest["derivations"]) == 2
    assert len(manifest["evaluations"]) == 1
