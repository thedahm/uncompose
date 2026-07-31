# First separation models: htdemucs_6s and Mel-Band RoFormer via audio-separator, behind a process boundary

Uncompose v0.1 needs a separation model for its two presets, but every model that clears the
Moises-replacement bar ([#6](https://github.com/thedahm/uncompose/issues/6)) is a PyTorch
checkpoint, and we want the core free to be non-Python (Rust is the stated preference) and free
to swap models cheaply. We decided: **audio-separator** (PyPI, MIT, actively maintained) is the
v0.1 separation engine, wrapped behind a **process-boundary contract**, and the first models
wired through it are **`htdemucs_6s`** (the whole 6-stem preset in one pass) and
**Kim Mel-Band RoFormer** (the 2-stem preset).

## The engine contract

The core talks to a separation engine over a language-agnostic process boundary: audio path,
model id, and parameters in; stem files, timings, and a job record out (JSON over stdio or
similar). No Python or audio-separator types leak into the core. Presets are core-owned, fixed
model mappings; multi-model pipelines are composed by the core as multiple engine calls, so
engines stay dumb. Per-model **hardware tier** ("runs everywhere" vs "GPU required") is a
first-class contract field.

## Considered options

- **Direct `demucs` dependency, htdemucs 4-stem** — the pre-workflow frontrunner (only
  CPU-viable high-quality model, per the
  [model survey](https://github.com/thedahm/uncompose/issues/2)). Dropped when
  [#6](https://github.com/thedahm/uncompose/issues/6) fixed the default preset at 6 stems, made
  GPU the reference path, and set an A/B-vs-Moises bar that Demucs vocals (8.2 SDR vs
  RoFormer's ~11) would likely lose. Upstream is also frozen (repo archived Jan 2025).
- **Mel-Band RoFormer first** — best vocals, MIT weights, but 2-stem only; no drums/bass guts
  the practice workflow. It ships as the 2-stem preset instead.
- **audio-separator as a library dependency** — simpler plumbing, but locks the core into
  Python and makes the engine a foundation rather than a replaceable backend.

## Installation

Weights are never bundled (licensing:
[#3](https://github.com/thedahm/uncompose/issues/3) — Demucs weights are unlicensed
"research purposes only"; Kim Mel-Band RoFormer is MIT). First use of a preset downloads its
weights into the platformdirs cache with visible progress and the per-model license status
relayed (not certified). An explicit models verb (list/fetch/remove) is a convenience on top.
Provisioning the Python engine environment itself is a distribution question
([#11](https://github.com/thedahm/uncompose/issues/11)).

## Hardware

Claimed and tested: Linux + NVIDIA CUDA (reference rig: RTX 4060 Ti 16 GB) plus CPU fallback,
which realistically means the Demucs family at ~1.5x track duration. The 2-stem preset is
GPU-required. macOS/Windows are best-effort and unclaimed. Device auto-detects CUDA, falls
back to CPU, with an explicit `--device` override.

## Consequences

- A second model exists from day one, so the engine contract and license-surfacing UX are
  exercised for real, and later models (karaoke RoFormer, becruily guitar,
  [#13](https://github.com/thedahm/uncompose/issues/13)) are registry adds, not integrations.
- The hardware spike ([#14](https://github.com/thedahm/uncompose/issues/14)) decides whether
  the 6-stem preset gains a Mel-Band RoFormer vocal pre-pass to hit the Moises bar; the
  contract's pipeline composition already allows it.
- We own a process-spawning seam and its error handling from day one.
- Core language choice is explicitly deferred to the architecture decision
  ([#10](https://github.com/thedahm/uncompose/issues/10)).
