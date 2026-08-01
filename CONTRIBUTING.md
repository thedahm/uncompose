# Contributing to Uncompose

Thanks for your interest in Uncompose. The project is pre-v0.1 and this guide is
intentionally minimal. It grows into a full contributor guide once v0.1 exists.

## Working on the code

The repo is a Cargo workspace (`core`, `cli`) plus one Python package (`engine`), split
by the Engine Contract (ADR-0001, ADR-0003). Setup:

- Rust side: a stable toolchain; `cargo test` at the repo root runs everything.
- Engine side: [uv](https://docs.astral.sh/uv/), then `uv sync` in `engine/` and
  `uv run pytest` there. Tests fake audio-separator at its Python interface, so they
  need no GPU and load no models.

Development is test-first at the Engine Contract seam, and that seam is the only
substitution point: Rust tests run the real core and CLI against `fake-engine`, a small
workspace binary speaking the JSONL contract, so no test anywhere needs PyTorch or an
NVIDIA card. CI (fmt, clippy, Rust tests, shim tests) is CPU-only for the same reason.

## Governance

Uncompose is created and maintained by Dominic Hanzely ([@thedahm](https://github.com/thedahm)),
who acts as the project's maintainer and final decision-maker. Significant decisions are
recorded as numbered architecture decision records in [`docs/adr/`](docs/adr/), so the
reasoning behind the project's choices is public and reviewable. Issues and pull requests
are answered on a best-effort basis.

## Documentation carries rationale, not narration

Code is the source of truth for what the project does; committed documentation exists to
carry what code cannot: the reasoning, the constraints, and the roads not taken. ADRs in
[`docs/adr/`](docs/adr/) are the home for "we did X instead of Y because". Comments state
constraints the code can't show. Structural docs (layout, vocabulary, standards) are
welcome. What we avoid is documentation that restates what code already says: it competes
with the source of truth and loses the moment either changes.

## Before opening a large pull request

Open an issue first. Discussing the change before you build it keeps you from investing
effort in something that conflicts with a recorded decision or the current milestone.
Small fixes (typos, broken links, obvious corrections) are welcome directly.

## Conduct

Participation in the project is covered by the [Code of Conduct](CODE_OF_CONDUCT.md).
