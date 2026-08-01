# PyPI packaging and runtime Engine Environment provisioning

ADR-0003 chose a Rust core and accepted its hardest distribution consequence: the multi-GB
Python engine environment cannot ride in the package, so the core must build it on the user's
machine. This ADR records how v0.1 ships ([#11](https://github.com/thedahm/uncompose/issues/11),
[#36](https://github.com/thedahm/uncompose/issues/36)): **two PyPI packages** — a maturin-built
binary wheel for the CLI and a pure-Python engine package — with the core **provisioning the
Engine Environment at runtime by shelling out to uv**.

## Two packages, one version

- **`uncompose`** — the Rust CLI as a `bindings = "bin"` maturin wheel (root `pyproject.toml`,
  pointing at the `cli/` crate). No Python code in the wheel; `uvx uncompose` and
  `uv tool install uncompose` work because the wheel is just a binary with PyPI plumbing.
  Linux x86_64 manylinux only, matching the v0.1 platform claim.
- **`uncompose-engine`** — the existing hatchling package in `engine/`, published as-is. It is
  never a dependency of `uncompose`: declaring it one would drag CUDA torch into `uvx` and give
  the resolver, not the core, control over the engine env.

The engine pin is the product version itself: the core installs
`uncompose-engine==<CARGO_PKG_VERSION>`. Workspace Cargo version, root pyproject, and engine
pyproject must agree; a core test (`core/tests/version_sync.rs`) fails the build when they
drift. A tag-driven release workflow builds and publishes both packages together via trusted
publishing.

## Provisioning: uv builds the Engine Environment on first run

On first `separate`, the core finds `uv` on PATH (it is missing only when the user installed
with plain pip; the error says how to get it), then builds
`~/.local/share/uncompose/engine/<version>/`:

- `uv venv --python 3.12 <dir>` — uv fetches a managed CPython when the host has none, which
  is exactly the clean-machine case; then `uv pip install uncompose-engine==<version>` into it.
- A `.provisioned` marker is written last — the same completion-marker rule `job.json`
  follows. A directory without it is a crashed provision and is wiped and rebuilt; so is an
  env whose marker records another version.
- The env lives under XDG *data*, not cache: losing it costs a multi-GB reinstall.
- uv's own stderr passes through as the download UI; the CLI announces the one-time build (and
  the CPU-only escape hatch) before it starts, so it never looks like a hang.

Interpreter resolution order: `$UNCOMPOSE_ENGINE_PYTHON` (tests, overrides) → a dev checkout's
`engine/.venv` found by walking up from the cwd → the provisioned env. Dev machines never
provision; user machines always do.

## Torch flavor: CUDA by default, uv routing for CPU

Plain provisioning pulls audio-separator's default CUDA torch — correct for the claimed
platform (Linux + NVIDIA, ADR-0001). The one documented CPU-only line is uv's own knob passed
through the shell-out: `UV_TORCH_BACKEND=cpu uncompose separate …` on the first run. No extras
games, no second package flavor, no core-owned index logic.

## Testing

uv is a process boundary, so it is tested like the other two (fake engine, stub ffmpeg): a
fake `uv` script on a hermetic PATH records its argv and "installs" the fake engine as the
env's python. CI builds the real wheel per PR and smoke-runs it with `uvx --from` — packaging
breakage surfaces before a release, still with no torch anywhere in CI.

## Consequences

- A stranger's first run downloads several GB before any separation; the cost is visible, not
  removable. GHCR images (fast-follow) are the answer for prebaked environments.
- uv becomes a hard runtime dependency for PyPI users. Accepted: it is the headline install
  path anyway, and bootstrapping envs is uv's whole job.
- Publishing is two-package: a release that ships `uncompose` without `uncompose-engine`
  strands provisioning on a 404. The single release workflow publishing both from one tag is
  the guard.
