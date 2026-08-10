"""The CLI leg of the acceptance gate: the vertical slice through installed wheels.

These read only what a user can read — exit codes, stdout, stderr, and the
manifest file — of commands typed the way the walkthrough types them. Nothing
imports a tool's internals or looks inside a cache.
"""

import json
import re

from uncompose_acceptance.commands import cache_root
from uncompose_acceptance.reading import sha256

# What a run that never fetched a model leaves behind: config, state, and at
# most a little scratch. Separation weights are two orders of magnitude larger
# than this, so the budget separates "a tool kept a note" from "a tool
# downloaded a model" without pretending to know what either writes.
CACHE_BUDGET = 4 * 1024 * 1024


def stem_assets(manifest):
    return [asset for asset in manifest["assets"] if asset["role"] == "stem"]


def test_all_three_commands_are_installed_and_report_a_version(cli_slice):
    for name in ("uncompose", "uncompose-project", "uncompose-compare"):
        result = cli_slice.versions[name]
        assert result.ok, result.describe()
        assert re.fullmatch(rf"{name} \d+\.\d+\.\d+", result.stdout.strip()), result.describe()


def test_each_extension_answers_to_both_dispatch_forms(cli_slice):
    for extension in ("project", "compare"):
        dispatched = cli_slice.versions[f"uncompose {extension}"]
        standalone = cli_slice.versions[f"uncompose-{extension}"]
        assert dispatched.ok, dispatched.describe()
        assert dispatched.stdout == standalone.stdout, (
            f"`uncompose {extension}` and `uncompose-{extension}` disagree:\n"
            f"{dispatched.describe()}\n{standalone.describe()}"
        )


def test_init_creates_the_manifest_the_family_meets_at(cli_slice):
    assert cli_slice.init.ok, cli_slice.init.describe()
    manifest = cli_slice.manifest()
    assert manifest["schema"].endswith("/schemas/project/v0/uncompose.project.schema.json")
    assert manifest["project"]["id"]


def test_each_import_reports_the_stems_it_registered(cli_slice):
    for run in cli_slice.fixtures.runs:
        result = cli_slice.imports[run.slug]
        assert result.ok, result.describe()
        for stem in run.stems:
            assert stem in result.stdout, result.describe()


def test_the_manifest_holds_exactly_one_source_asset(cli_slice):
    manifest = cli_slice.manifest()
    source = cli_slice.fixtures.source

    sources = [asset for asset in manifest["assets"] if asset["role"] == "mix"]
    assert len(sources) == 1, f"expected one source asset, got {sources}"
    assert sources[0]["path"] == source.name
    assert sources[0]["sha256"] == sha256(source)
    # Both runs separated the same recording, so the second import must resolve
    # to the asset the first registered rather than adding a second copy.
    assert [asset["sha256"] for asset in manifest["assets"]].count(sha256(source)) == 1


def test_both_runs_land_as_derivations_pointing_back_at_their_job_records(cli_slice):
    manifest = cli_slice.manifest()
    derivations = manifest["derivations"]
    assert len(derivations) == 2, f"expected two derivations, got {derivations}"

    by_job_path = {derivation["job"]["path"]: derivation for derivation in derivations}
    for run in cli_slice.fixtures.runs:
        relative = run.job_json.relative_to(cli_slice.project).as_posix()
        assert relative in by_job_path, f"no derivation references {relative}: {by_job_path.keys()}"
        derivation = by_job_path[relative]
        assert derivation["tool"] == "uncompose"
        assert derivation["job"]["sha256"] == sha256(run.job_json)


def test_every_stem_is_registered_as_an_asset_of_its_derivation(cli_slice):
    manifest = cli_slice.manifest()
    assets = cli_slice.assets_by_id()
    derivations = {derivation["job"]["path"]: derivation for derivation in manifest["derivations"]}

    assert len(stem_assets(manifest)) == cli_slice.fixtures.stem_count

    for run in cli_slice.fixtures.runs:
        derivation = derivations[run.job_json.relative_to(cli_slice.project).as_posix()]
        outputs = [assets[asset_id] for asset_id in derivation["outputs"]]
        assert {asset["path"].rsplit("/", 1)[-1] for asset in outputs} == {
            f"{stem}.wav" for stem in run.stems
        }
        for asset in outputs:
            assert asset["role"] == "stem"
            assert asset["sha256"] == sha256(cli_slice.project / asset["path"])


def test_the_source_is_the_input_of_both_derivations(cli_slice):
    manifest = cli_slice.manifest()
    assets = cli_slice.assets_by_id()

    for derivation in manifest["derivations"]:
        inputs = [assets[asset_id] for asset_id in derivation["inputs"]]
        assert [asset["path"] for asset in inputs] == [cli_slice.fixtures.source.name]


def test_show_json_prints_the_manifest_on_disk(cli_slice):
    assert cli_slice.show_json.ok, cli_slice.show_json.describe()
    assert json.loads(cli_slice.show_json.stdout) == cli_slice.manifest_when_shown


def test_verify_is_green_over_the_whole_project(cli_slice):
    result = cli_slice.verify
    assert result.ok, result.describe()
    assert result.quiet, result.describe()
    # Green means every registered file was checked, source and stems alike.
    assert cli_slice.fixtures.source.name in result.stdout, result.describe()
    for asset in stem_assets(cli_slice.manifest()):
        assert asset["path"] in result.stdout, result.describe()


def test_no_step_downloaded_model_weights(cli_slice):
    # The whole leg runs on fake separations, so nothing should have fetched a
    # model. Weighed over the whole cache tree the run owns rather than looked
    # for under a name: which directory a tool caches into is its own business
    # (ADR-0007), and a guard that names one passes vacuously the day it moves.
    cache = cache_root(cli_slice.installation)
    files = [path for path in cache.rglob("*") if path.is_file()]
    total = sum(path.stat().st_size for path in files)

    largest = sorted(files, key=lambda path: path.stat().st_size, reverse=True)[:5]
    assert total <= CACHE_BUDGET, (
        f"{cache} holds {total} bytes, over the {CACHE_BUDGET}-byte budget, so a step "
        "fetched something the size of model weights. Largest:\n"
        + "\n".join(f"  {path.stat().st_size} {path.relative_to(cache)}" for path in largest)
    )
