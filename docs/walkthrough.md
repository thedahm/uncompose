# End-to-end walkthrough: separate, register, compare, verify

This is the whole Uncompose vertical slice performed from start to finish:
install the three tools, create a project, separate a recording so the job
registers itself, inspect what landed, compare two candidate stems and record a
verdict, then confirm the manifest tells the complete story. It assumes nothing
checked out — only the three published wheels — and it is written so you can
follow it without reading any source.

The three tools are separate programs that meet at one file, the project
manifest (`uncompose.project.json`):

- **`uncompose`** separates a recording into stems.
- **`uncompose-project`** owns the manifest — the single writer that records
  where every file came from and how it was judged.
- **`uncompose-compare`** auditions candidate stems side by side and writes a
  verdict.

You invoke the last two through `uncompose` itself: any executable named
`uncompose-<name>` on your `PATH` becomes `uncompose <name>` (see
[`extensions.md`](extensions.md)). So `uncompose project …` runs
`uncompose-project`, and `uncompose compare …` runs `uncompose-compare`.

> **Want the finished result without supplying audio?** The
> [`demo/`](../demo/) directory builds this exact end state from synthetic
> fixtures in seconds — see [Reproducing this without your own
> recording](#reproducing-this-without-your-own-recording).

## 1. Install the three tools

Each ships on PyPI; install them with [uv](https://docs.astral.sh/uv/):

```sh
uv tool install uncompose
uv tool install uncompose-project
uv tool install uncompose-compare
```

Uncompose is Linux-only for v0.1 and needs **ffmpeg** on your `PATH`
(`sudo apt install ffmpeg` on Debian/Ubuntu). The first real separation builds
the engine environment and downloads model weights; on a machine without an
NVIDIA GPU, prefix that first run with `UV_TORCH_BACKEND=cpu` (see the
[README](../README.md#install)).

## 2. Create a project

Pick a directory to hold the work and initialise it:

```sh
mkdir my-project && cd my-project
uncompose project init
```

`--project` defaults to `.`, so every command below run from inside
`my-project/` acts on this project. `init` writes an empty
`uncompose.project.json` here — the manifest. `--project <dir>` always names the
project root *itself*: the manifest must be exactly
`<dir>/uncompose.project.json`, and no parent directories are ever searched, so
a project can never silently capture work from a directory above it.

## 3. Separate a recording — registration comes free

Put the recording somewhere inside the project and separate it with
`--project`:

```sh
cp ~/Music/song.wav .
uncompose separate --project . song.wav
```

This is an ordinary separation — the same stems in `song.stems/` as without
`--project` — followed by one extra step: the finished job registers itself in
the manifest. **Exit 0 means separated *and* registered.**

Before any GPU work, `separate --project` pre-flights every foreseeable
failure, so a doomed run costs milliseconds instead of minutes:

- the manifest exists at `./uncompose.project.json` and is a v0 project
  manifest;
- `uncompose-project` is installed and on `PATH`;
- the input recording is inside the project root;
- the destination stems folder (default, or an `-o` override) is inside the
  project root.

Run it a second time with a different model to get a second derivation of the
same song — the comparison in step 5 needs two candidates:

```sh
uncompose separate --project . --preset 2-stem song.wav
```

### If registration fails

The expensive separation is never the casualty. If the manifest write fails,
the job folder and its `job.json` are left completely intact, the underlying
error is shown, and the last line is the exact command to finish the job:

```text
re-register with: uncompose project import /abs/path/song.stems/job.json
```

`import` is idempotent, so that command is always safe to copy-paste and rerun —
recovery is one line, and nothing is ever lost.

## 4. Inspect what landed

```sh
uncompose project show
```

The overview now lists the **source asset** (the recording you separated), each
**derivation** (a separation run), and the **stems** each produced. Every file
carries the hash the manifest recorded, so `show` is the map of where each file
came from. `uncompose project show --json` prints the same as machine-readable
JSON.

## 5. Compare two candidates and record a verdict

With two derivations of the same song, compare their vocals. In project mode
you name candidates by manifest reference, not by file path. `uncompose project
show` (step 4) prints each derivation's slug; use those slugs here — for two
runs they will look like `htdemucs-1` and `roformer-1`:

```sh
uncompose compare --project . vocals@htdemucs-1 vocals@roformer-1 --blind
```

The reference grammar:

- A bare token (`vocals`) is an asset slug.
- `<name>@<derivation>` picks, among that derivation's outputs, the stem whose
  filename stem equals `<name>` — so `vocals@htdemucs-1` is "the vocals stem
  from the first htdemucs run". If a reference matches nothing or is ambiguous,
  the error lists that derivation's outputs with their slugs and filenames, so
  the right reference is in front of you.

In project mode Compare also:

- **auto-loads the source** into a third lane (SRC), so you can always A/B each
  candidate against the original without hunting for the file;
- with `--blind`, conceals the A/B identities (as in ordinary blind listening)
  while keeping the SRC lane identified — you chose the source knowingly, so
  hiding it would be pointless;
- refuses raw file paths (project sessions always produce records the manifest
  can connect) and refuses to start if any referenced file is missing or fails
  its recorded hash.

Listen, place a pin at a moment worth marking, engrave your verdict (which
candidate you prefer, and how confident you are), then **save and close**. That
single act writes the comparison record into `./evaluations/<id>.json` and
registers it in the manifest as an evaluation. Exit 0 means saved *and*
registered; if the manifest write fails, the record file is kept and the same
one-line recovery command is printed, exactly as in step 3.

## 6. Confirm the manifest tells the whole story

```sh
uncompose project show
uncompose project verify
```

`show` now renders the evaluation alongside the assets and derivations: which
candidates were compared, which you preferred, your confidence, and a
`record: {path, sha256}` reference to the evidence file under `evaluations/`.
The verdict lives in the manifest; the full listening detail (pins,
observations) stays in the referenced record file.

`verify` walks every referenced file — source, stems, and evaluation records —
and confirms each is present and matches its recorded hash. A green `verify` is
the whole point of the slice: the recording, every derived stem, and the
judgment you made are all accounted for, in one manifest, from three tools that
handed the work to each other without you acting as the courier.

## Reproducing this without your own recording

The commands above use real models on real audio. To see the *finished*
manifest — a source asset, two derivations with stems, and one evaluation —
without supplying audio, downloading any model, or owning a GPU, run the
demonstration script:

```sh
python3 demo/build_demo.py
```

It builds the project in `./uncompose-demo/` from **synthetic** fixtures:
deterministic seeded noise standing in for the stems, and hand-built job folders
imported through the real `uncompose project import` contract — so what it
exercises (registration, evaluation import, `verify`) is genuine, while the
"separations" are fake and run in seconds. The script says so up front and needs
all three tools installed. See [`demo/README.md`](../demo/README.md) for details.
