# Rust core, direct execution, and the filesystem as the job database

Uncompose v0.1 needs its structural decisions made: core language, execution model, job
persistence, and where the boundaries sit. Constraints inherited from earlier decisions: the
engine is Python behind a process boundary ([ADR-0001](0001-first-separation-models-and-engine-boundary.md)),
the CLI is the sole v0.1 surface with an interface-agnostic core
([#8](https://github.com/thedahm/uncompose/issues/8)), and the workflow is one song at a time
([#6](https://github.com/thedahm/uncompose/issues/6)). We decided: a **Rust core** with a thin
CLI, **direct foreground execution**, and **the filesystem as the only database**.

## Core language: Rust

The core and CLI are Rust; all Python lives behind the engine contract. The process boundary
from ADR-0001 was designed to make this possible, and a non-Python core keeps the "engines are
replaceable backends" claim honest. Distribution stays `uvx uncompose` via maturin-built wheels
(the ruff/uv precedent, per [#8](https://github.com/thedahm/uncompose/issues/8)). The cost is
real: the core must provision a Python engine environment at runtime (likely by shelling out to
`uv`), which is [#11](https://github.com/thedahm/uncompose/issues/11)'s problem. A Python core
would make provisioning trivial but was rejected because core and engine would blur together
and the contract would go untested.

## Execution model: direct

`uncompose separate` runs the job in the foreground: the CLI process asks the core to run the
job, the core spawns the engine subprocess and streams progress, and the process exits when the
job does. No daemon, no queue — one GPU, one song, a user waiting to audition the result.
Interface-agnosticism is satisfied at the code level, not the process level: the core exposes a
`run_job` API that streams typed progress events, and the CLI is merely its first caller. The
post-v0.1 web UI's HTTP API wraps that same API in a resident process; the daemon is that era's
problem.

## Jobs: the filesystem is the database

The job folder is the job. It is self-contained — stems plus `job.json` (input file and content
hash, preset, model ids/versions, parameters, device, engine version, timings, outcome) — and
nothing about a job is stored anywhere else. No SQLite, no index, no history verb in v0.1. A
job is addressed by its folder path; the single convenience state is a last-job pointer
(platformdirs state file) so `uncompose play vocals` works right after a run. A future index
for the web UI era can be rebuilt entirely by scanning job folders — that self-describing
property is the invariant worth protecting.

- **Location and naming**: default is next to the input file, `<input basename>.stems/`
  (`-o` overrides). Collisions never overwrite: `.stems-2/`, `.stems-3/`, ….
- **Stage-then-rename**: jobs run in a hidden staging folder (`.<name>.stems.partial/`) renamed
  into place on success, so a visible `.stems` folder is always a complete, good job.
- **Failure**: staging folder is kept (job record with the error, engine log) as the
  diagnosable artifact; the CLI prints the stderr tail and exits nonzero.
- **Cancel**: Ctrl+C kills the engine process group and deletes the staging folder.
- **Retries: none.** Local deterministic work; retry is the user rerunning.

## Progress

The engine emits JSONL events on stdout: stage (download, model load, separation pass, write),
best-effort percent, optional message. A stage with no percent is legal — the adapter emits
what audio-separator actually exposes. Engine stderr goes to a log file in the job folder,
never into the progress UI. Weights downloads report bytes, since first-run downloads are the
slow surprise.

## Boundaries

Monorepo: a Cargo workspace plus one Python package.

- **`core/` (`uncompose-core`)** — preset/model registry (in-code, static), job lifecycle
  (staging, record, rename), engine contract types, engine client (spawn, stream, kill),
  progress events. No CLI or terminal code.
- **`cli/` (`uncompose`)** — clap verbs (`separate`, `play`, `models`), progress rendering,
  exit codes, shell-out `play`. Thin by design.
- **`engine/` (`uncompose-engine`, Python)** — the shim: JSONL contract on one side,
  audio-separator on the other. Versioned with the product, provisioned per
  [#11](https://github.com/thedahm/uncompose/issues/11).

Boundary rules: the core never imports interface code; the engine is only ever spawned, never
imported; the contract is the only thing both sides know. There is no persistence layer —
job-folder I/O is core domain logic.

## Consequences

- The architecture supports a second model, a second engine, and a second interface without
  redesign — the kickoff's measures 4 and 5 — while v0.1 itself has zero resident moving parts.
- Engine-environment provisioning becomes the hardest distribution problem
  ([#11](https://github.com/thedahm/uncompose/issues/11)) and is on the critical path.
- Contributors need Rust for the core and Python for the engine; audio/ML contributions touch
  only the engine side of the contract.
