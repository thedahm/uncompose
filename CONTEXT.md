# Uncompose

Local-first, open-source music source separation: take a finished recording, split it into its
constituent parts, and hand them to the musician's own tools.

## Language

**Stem**:
A single separated part of a recording (vocals, drums, bass, guitar, keys, other) produced by a
separation run.
_Avoid_: track, part, layer

**Preset**:
A named, fixed recipe the user picks (`6-stem`, `2-stem`) that maps to one or more model runs.
Presets are owned by the core; they never silently substitute models based on hardware.
_Avoid_: mode, profile

**Separation Engine**:
A replaceable backend that executes model inference behind a process boundary. The core knows
the engine contract, never the engine's internals.
_Avoid_: wrapper, runner, backend library

**Engine Contract**:
The language-agnostic protocol between the core and a separation engine: audio, model id, and
parameters in; stem files, progress events, and timings out.
_Avoid_: plugin API, engine interface

**Engine Environment**:
The Python environment a separation engine runs in. On a user machine the core builds it at
runtime with uv, pinned to the product's own version; in a dev checkout it is the `uv sync`
venv in `engine/`.
_Avoid_: venv (in user-facing text), runtime, sandbox

**Model**:
A specific separation checkpoint an engine can run (e.g. `htdemucs_6s`, Kim Mel-Band RoFormer),
carrying its own license status and hardware tier.
_Avoid_: algorithm, network

**Hardware Tier**:
A model's declared hardware requirement — "runs everywhere" or "GPU required" — surfaced before
a run, not discovered during one.

**Job**:
One separation run: an input recording, a preset, and the resulting job folder.

**Job Folder**:
The self-contained per-job directory holding the stems and the job record — the canonical
representation of a completed job; nothing about a job lives anywhere else.
_Avoid_: output directory, results folder

**Job Record**:
The reproducibility file written with every job: model, version, and parameters sufficient to
rerun it.
_Avoid_: metadata file; **manifest** on its own, which names the project file below

## Language: the extensions

The vocabulary `uncompose-project` and `uncompose-compare` own. It appears in this repo's
docs — the walkthrough, the release checklist, the acceptance gate — because the family's
own commands do.

**Project Manifest**:
The single `uncompose.project.json` a project is described by: its assets, the derivations
between them, and the evaluations passed over them. Always qualified as the *project*
manifest when a Job Record is anywhere nearby, since a job's own file is never called that.
_Avoid_: index, database, project file

**Asset**:
One file a project registers, referenced in place by path and SHA-256 and never copied into
the manifest — a source recording (`mix`) or a separated part (`stem`).
_Avoid_: file entry, artifact

**Derivation**:
The recorded relationship between the assets a process consumed and the ones it produced,
carrying a hashed reference to the Job Record that produced them.
_Avoid_: edge, transformation, lineage entry

**Evaluation**:
A comparison registered in the manifest: which assets were judged, which was preferred at
what confidence, and a hashed reference to the comparison record on disk.
_Avoid_: verdict (that is the listener's decision inside one record), rating, score
