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

**Model**:
A specific separation checkpoint an engine can run (e.g. `htdemucs_6s`, Kim Mel-Band RoFormer),
carrying its own license status and hardware tier.
_Avoid_: algorithm, network

**Hardware Tier**:
A model's declared hardware requirement — "runs everywhere" or "GPU required" — surfaced before
a run, not discovered during one.

**Job**:
One separation run: an input recording, a preset, and the resulting stem folder.

**Job Record**:
The reproducibility file written with every job: model, version, and parameters sufficient to
rerun it.
_Avoid_: manifest, metadata file
