# Coding Standards

<!-- The reviewer agent loads this file during code review via
     @.sandcastle/CODING_STANDARDS.md so these standards are enforced during
     review without costing tokens during implementation. -->

## Style

- Rust: `cargo fmt` formatting is mandatory; `cargo clippy --workspace` must be clean.
- Use the project vocabulary from `CONTEXT.md` (stem, preset, separation engine, engine
  contract, job, job folder, job record) and avoid the listed banned synonyms.

## Testing

- Development is test-first, and substitution happens only at process boundaries: Rust
  tests run the real core and CLI against `fake-engine` (the Engine Contract seam), and
  system tools the core shells out to (ffmpeg, uv) are stubbed as fake executables on a
  hermetic PATH (ADR-0004). No in-process seams.
- Engine tests fake audio-separator at its Python interface. No test anywhere may need
  PyTorch, a GPU, or a model download.

## Documentation

- Documentation carries rationale, not narration (see CONTRIBUTING.md). Comments state
  constraints the code can't show; delete comments that restate what the code says.
- "We did X instead of Y because" belongs in `docs/adr/`, not in scattered comments.

## Architecture

- The core knows the engine contract, never an engine's internals.
- Jobs live entirely in their job folder (filesystem as database, ADR-0003); nothing
  about a job lives anywhere else.
