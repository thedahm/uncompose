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
_Avoid_: manifest, metadata file
