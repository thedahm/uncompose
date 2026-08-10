#!/usr/bin/env python3
"""Build the Uncompose demonstration project from deterministic fixtures.

This script reproduces the *populated end state* of the M5 vertical slice —
a project manifest carrying a source asset, two derivations with their stems,
and one evaluation — on any Linux machine, without a GPU, without downloading
any model weights, and in seconds rather than minutes.

    THE SEPARATIONS PRODUCED HERE ARE SYNTHETIC.

Nothing here runs a real separation model. The "stems" are deterministic
seeded noise (the #73 approach); the job folders they live in are hand-built
to the M2 job-record shape. What the demo genuinely exercises is the *import
contract* — `uncompose project import` turning a job folder into a derivation,
and a comparison record into an evaluation — which is the integration surface
M5 delivers. The walkthrough in `docs/walkthrough.md` shows the same flow with
the real `uncompose separate --project` and `uncompose compare --project`
commands a user actually runs; this script exists only so the finished
manifest is reproducible anywhere, including CI-less machines.

Usage:

    python3 demo/build_demo.py [WORKDIR]
        Build the whole demo project in WORKDIR (default: ./uncompose-demo),
        driving the three installed tools (uncompose, uncompose-project,
        uncompose-compare). Requires the wheels installed and on PATH.

    python3 demo/build_demo.py --fixtures-only DIR
        Only write the deterministic fixtures (source recording + synthetic
        job folders) into DIR and stop, touching no external tool. Rerunning
        with the same DIR contents yields byte-identical files — this is the
        determinism the repo's CI checks.
"""

from __future__ import annotations

import hashlib
import json
import os
import random
import subprocess
import sys
import wave
from pathlib import Path

# --- Fixture determinism knobs -------------------------------------------
#
# Everything the fixtures contain is a pure function of these constants, so
# `--fixtures-only` writes the same bytes on every machine and every rerun.
# No wall-clock, no RNG without a fixed seed, no absolute paths.

SEED = 0x5EED0011  # one seed; each fixture derives its own from a stable label
SAMPLE_RATE = 8000
FRAME_COUNT = 4000  # ~0.5 s mono; small and fast, still a real WAV
# A fixed instant stands in for the job's finish time; a live clock would make
# job.json bytes differ between runs and defeat the determinism check.
FINISHED_AT_UNIX = 1_700_000_000

# The two synthetic separation runs. Each becomes one derivation on import;
# together with the shared source they give the manifest a source asset, two
# derivations, and their stems.
RUNS = [
    {
        "slug": "htdemucs-6s",
        "preset": "6-stem",
        "models": ["htdemucs_6s"],
        "stems": ["vocals", "drums", "bass", "guitar", "keys", "other"],
    },
    {
        "slug": "roformer-2stem",
        "preset": "2-stem",
        "models": ["mel_band_roformer"],
        "stems": ["vocals", "instrumental"],
    },
]

SOURCE_NAME = "demo-song.wav"

# The comparison-record schema URL uncompose-project dispatches on to import a
# file as an evaluation rather than a derivation (#65, M5 spec).
COMPARE_SCHEMA_URL = (
    "https://uncompose.org/schemas/compare/v0/uncompose.compare.schema.json"
)
EVALUATED_AT = "2023-11-14T22:13:20Z"  # fixed instant, matching FINISHED_AT_UNIX


def seed_for(label: str) -> int:
    """A stable per-fixture seed so each file is distinct yet reproducible."""
    digest = hashlib.sha256(f"{SEED:x}:{label}".encode()).digest()
    return int.from_bytes(digest[:8], "big")


def write_wav(path: Path, label: str) -> None:
    """Write a deterministic mono 16-bit PCM WAV of seeded noise."""
    rng = random.Random(seed_for(label))
    frames = rng.randbytes(2 * FRAME_COUNT)
    with wave.open(str(path), "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)
        wav.setframerate(SAMPLE_RATE)
        wav.writeframes(frames)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_job_json(folder: Path, input_path: str, input_sha256: str, run: dict) -> None:
    """Write a job.json to the M2 job-record shape (core::job::JobRecord).

    A job record carries no `schema` field by design; uncompose-project's
    schema-URL dispatch reads that absence as 'this is a job record' and
    imports it as a derivation.
    """
    record = {
        "input_path": input_path,
        "input_sha256": input_sha256,
        "preset": run["preset"],
        "models": run["models"],
        "parameters": {},
        "device": "cpu",
        "engine_version": "demo-synthetic",
        "stems": run["stems"],
        "timings": {},
        "outcome": "success",
        "finished_at_unix": FINISHED_AT_UNIX,
    }
    (folder / "job.json").write_text(json.dumps(record, indent=2) + "\n")


def generate_fixtures(dest: Path, input_path: str | None = None) -> list[Path]:
    """Write the source recording and the synthetic job folders under `dest`.

    Returns the job-folder paths in run order. `input_path` is what each
    job.json records as the recording it separated; left unset it defaults to
    the bare source name, which keeps the bytes machine-independent for the
    determinism check.
    """
    dest.mkdir(parents=True, exist_ok=True)
    source = dest / SOURCE_NAME
    write_wav(source, "source")
    source_sha256 = sha256_file(source)
    recorded_input = input_path if input_path is not None else SOURCE_NAME

    job_folders: list[Path] = []
    runs_dir = dest / "runs"
    for run in RUNS:
        folder = runs_dir / run["slug"]
        folder.mkdir(parents=True, exist_ok=True)
        for stem in run["stems"]:
            write_wav(folder / f"{stem}.wav", f"{run['slug']}/{stem}")
        write_job_json(folder, recorded_input, source_sha256, run)
        job_folders.append(folder)
    return job_folders


# --- Full demo run (drives the three installed tools) --------------------


class ToolMissing(Exception):
    """Raised when a required `uncompose*` command is not on PATH."""


def run_uncompose(args: list[str], **kwargs) -> subprocess.CompletedProcess:
    """Run `uncompose <args>`, echoing the command first.

    A missing `uncompose` binary raises ToolMissing so the caller can print
    the install hint rather than a raw traceback.
    """
    printable = " ".join(["uncompose", *args])
    print(f"$ {printable}")
    try:
        return subprocess.run(["uncompose", *args], check=True, **kwargs)
    except FileNotFoundError as exc:
        raise ToolMissing(printable) from exc


def find_stem_asset(manifest: dict, run_slug: str, stem: str) -> dict | None:
    """Best-effort lookup of a stem's manifest asset (id + project ULID).

    The manifest schema is uncompose-project's; we read it defensively rather
    than binding to a private layout. An asset is any dict carrying an `id`
    and a `path` whose basename is `<stem>.wav` under the run's folder.
    """
    wanted = f"{stem}.wav"
    project_ulid = manifest.get("project") or manifest.get("id")

    def walk(node):
        if isinstance(node, dict):
            path = node.get("path")
            if (
                isinstance(path, str)
                and os.path.basename(path) == wanted
                and run_slug in path
                and ("id" in node or "slug" in node)
            ):
                return node
            for value in node.values():
                hit = walk(value)
                if hit:
                    return hit
        elif isinstance(node, list):
            for value in node:
                hit = walk(value)
                if hit:
                    return hit
        return None

    asset = walk(manifest)
    if asset is None:
        return None
    return {
        "id": asset.get("id") or asset.get("slug"),
        "project": project_ulid,
        "path": asset["path"],
    }


def build_comparison_record(project: Path, candidates: list[dict]) -> Path:
    """Fabricate a project-launched comparison record and return its path.

    Candidates carry the manifest `asset` and `project` refs the evaluation
    import requires (#63/#67): a record whose candidates lack `asset` refs is
    refused, because only project-launched records can be registered in v0.1.
    """
    record = {
        "schema": COMPARE_SCHEMA_URL,
        "completed_at": EVALUATED_AT,
        "preference": candidates[0]["label"],
        "confidence": 0.7,
        "candidates": candidates,
    }
    dest = project / "evaluations"
    dest.mkdir(parents=True, exist_ok=True)
    record_path = dest / "demo-comparison.json"
    record_path.write_text(json.dumps(record, indent=2) + "\n")
    return record_path


def run_demo(workdir: Path) -> None:
    workdir = workdir.resolve()
    print(__doc__.splitlines()[0])
    print("Separations below are SYNTHETIC seeded noise — no models run.\n")

    workdir.mkdir(parents=True, exist_ok=True)
    manifest_path = workdir / "uncompose.project.json"

    # 1. Initialise the project.
    run_uncompose(["project", "init", "--project", str(workdir)])

    # 2. Lay down the fixtures inside the project and import each synthetic run
    #    as a derivation — this is the real import contract at work.
    source = workdir / SOURCE_NAME
    job_folders = generate_fixtures(workdir, input_path=str(source))
    for folder in job_folders:
        job_json = (folder / "job.json").resolve()
        result = run_uncompose(
            ["project", "import", "--project", str(workdir), str(job_json)],
            check=False,
        )
        if result.returncode != 0:
            print(
                "\nimport failed; the job folder is intact. re-register with:\n"
                f"    uncompose project import {job_json}",
                file=sys.stderr,
            )
            raise SystemExit(result.returncode)

    # 3. Read the manifest back to resolve the two vocals stems to asset refs,
    #    then register a comparison of them as an evaluation.
    manifest = json.loads(manifest_path.read_text())
    candidates = []
    for run in RUNS:
        asset = find_stem_asset(manifest, run["slug"], "vocals")
        if asset is None:
            print(
                "could not resolve the vocals stems to manifest assets; "
                "skipping the evaluation step (the derivations still landed).",
                file=sys.stderr,
            )
            break
        rel = asset["path"]
        stem_path = Path(rel) if os.path.isabs(rel) else workdir / rel
        candidates.append(
            {
                "label": run["slug"],
                "path": str(stem_path),
                "sha256": sha256_file(stem_path),
                "size": stem_path.stat().st_size,
                "asset": asset["id"],
                "project": asset["project"],
            }
        )

    if len(candidates) == len(RUNS):
        record_path = build_comparison_record(workdir, candidates)
        record_abs = str(record_path.resolve())
        result = run_uncompose(
            ["project", "import", "--project", str(workdir), record_abs],
            check=False,
        )
        if result.returncode != 0:
            print(
                "\nevaluation import failed; the record is intact. re-register with:\n"
                f"    uncompose project import {record_path.resolve()}",
                file=sys.stderr,
            )
            raise SystemExit(result.returncode)

    # 4. Show the populated project and verify it end to end.
    run_uncompose(["project", "show", "--project", str(workdir)])
    run_uncompose(["project", "verify", "--project", str(workdir)])

    print(f"\nDemo project built at {workdir}")
    print(f"Manifest: {manifest_path}")


def main(argv: list[str]) -> int:
    args = argv[1:]
    if args and args[0] == "--fixtures-only":
        if len(args) != 2:
            print("usage: build_demo.py --fixtures-only DIR", file=sys.stderr)
            return 2
        generate_fixtures(Path(args[1]))
        return 0

    workdir = Path(args[0]) if args else Path("uncompose-demo")
    try:
        run_demo(workdir)
    except ToolMissing as exc:
        tool = exc.args[0].split()[0]
        print(
            f"\n'{tool}' is not installed or not on PATH.\n"
            "The demo needs all three tools:\n"
            "    uv tool install uncompose\n"
            "    uv tool install uncompose-project\n"
            "    uv tool install uncompose-compare",
            file=sys.stderr,
        )
        return 127
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
