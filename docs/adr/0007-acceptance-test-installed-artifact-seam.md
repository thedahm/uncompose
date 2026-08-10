# The acceptance test runs at the installed-artifact seam

The ecosystem's release gate is an end-to-end acceptance test that installs all
three packages into a clean environment and drives the vertical slice through
them. This ADR records where that test lives, what it may touch, and how it
gets the wheels it tests. Decided in the M6 spec
([#92](https://github.com/thedahm/uncompose/issues/92), stories 33–38 and its
testing decisions) and built in slices 5 and 6
([#99](https://github.com/thedahm/uncompose/issues/99),
[#103](https://github.com/thedahm/uncompose/issues/103)).

## The seam: installed commands on `PATH`, nothing below

The test observes exactly one boundary: the artifacts a user installs. Every
step is a subprocess invocation of `uncompose`, `uncompose project`,
`uncompose compare`, or the standalone entry points, resolved through a `PATH`
that holds the environment's `bin` and the system directories only — so a wheel
that works because the build toolchain happened to be next to it fails here.
Assertions read what a user reads: exit codes, stdout, stderr, and the project
manifest. No test imports a tool's library code, inspects a wheel's contents,
or reaches into a cache or server state.

This is why it is the *acceptance* test rather than another integration suite.
Each repo already tests its own seams: this repo's contract tests run the real
core and CLI against `fake-engine`, `uncompose-project` has CLI integration
tests, `uncompose-compare` has its Playwright specs. What none of them can
observe is the composition of three separately released wheels, which is the
only thing this gate exists to observe.

The run is hermetic in the other direction too: `HOME` and every XDG directory
point inside the run's temporary tree, so the gate neither reads nor writes the
machine's caches, state, or projects.

That tree is the one thing the gate may weigh rather than only read through a
command: the no-weights guard sizes the whole of the run's `XDG_CACHE_HOME`,
which the harness chose the location of. Sizing a directory the harness owns is
not the forbidden move — that is *naming* a path inside it, which would encode
a layout the tools are free to change and would pass vacuously the day they do.

## Python and `uv`, not `cargo test`

The harness is a pytest suite under `acceptance/`, run with `uv run pytest`,
outside the Cargo workspace. The subject is a Python virtual environment with
three wheels in it; building that environment is `uv`'s job, and the browser
leg the gate grows next drives Playwright, whose home is the same place. A Rust
integration test would have to shell out to `uv` for all of it while paying the
workspace's cost on every `cargo test`.

It is therefore *not* part of `cargo test`, which stays fast, hermetic, and
offline. The gate runs on demand and, per the M6 spec, in its own CI lane on
changes to itself.

## Wheel sourcing is a parameter; the test body is not

The gate must be green *before* anything is tagged, and must also be able to
verify what PyPI actually served afterwards. Both are the same test body over a
different source per package:

- **`build` (default)** — the extensions from configurable git refs (branch,
  tag, or commit sha), the root package from this checkout. Needs `uv`, `git`,
  `cargo`, and `npm`; `npm` only because `uncompose-compare` embeds its
  frontend in the binary at compile time, so a checkout is not installable
  until the bundle is built.
- **`pypi`** — all three from the index, optionally pinned to the versions
  under verification.

A machine lacking the toolchain skips rather than fails, because a contributor
without a Rust compiler should not see a red suite; CI sets
`UNCOMPOSE_ACCEPTANCE_REQUIRE=1`, which turns that skip into a failure, because
a gate that could not run has not passed.

## The browser leg is the same seam, reached through loopback

Half the slice cannot be typed. `uncompose compare --project` does not finish —
it serves a workbench and exits when the listener concludes — so the gate
launches that session as a subprocess like every other command, catches the
tokened loopback URL it prints, and drives the page in Chromium: place a pin,
engrave a blind verdict, conclude. The record it writes, the manifest entry
`uncompose-project import` makes from it, and the closing screen the listener
sees are all read afterwards; nothing talks to the server directly.

Three choices behind that, each of which could have gone the other way:

- **Playwright from Python, in this same pytest session, rather than a Node
  harness beside it.** Compare's specs are the prior art for *what* to drive
  (the flow and its test ids); copying their runner as well would mean two
  processes, two dependency trees, and a project handed between them. The gate
  is one command either way, and the browser leg needs the installation the CLI
  leg already built.
- **One project, driven in two legs, rather than a project per leg.** The
  milestone's assertion is that the manifest tells the *whole* story; a
  comparison in a project of its own would only prove Compare can write a
  record. The verdict has to land in the manifest the imports built, over
  stems those imports registered.
- **Chromium only.** The engine-dependent half of this workbench is its audio
  contract, and Compare already runs that across three engines. What this gate
  adds is the flow through the *installed* wheel, which is engine-independent.

The browser itself is not under test and is not part of the installation, so it
runs with the machine's environment. The session it drives is the installed
command, hermetic as ever. A machine without Chromium skips the browser leg
alone, leaving the CLI leg to gate everything up to the comparison — the same
bargain the missing-toolchain skip strikes, and `UNCOMPOSE_ACCEPTANCE_REQUIRE`
closes both.

## Fake separations, and why the gate imports rather than separates

The audio is deterministic seeded noise and the two separation runs are
hand-built job folders, generated by this repo's `demo/build_demo.py` so the
gate and the demonstration project cannot drift apart. No model weights, no
GPU, no engine: the gate stays fast enough to run on every change to it, which
is what keeps it honest (#92 story 38).

The gate does not run a real `uncompose separate`. It could only do so by
either downloading ~900 MB of weights on every run — which the milestone
explicitly rules out — or by pre-seeding the model cache with stub files, which
would mean reaching into a cache with knowledge of a tool's internal layout,
the one thing this seam forbids. The separation pipeline itself is already
covered where it belongs, in the contract tests against `fake-engine`; what M5
added and M6 must gate is the cross-tool handoff, and that handoff is `import`
consuming a job folder. Driving `import` on a job folder tests exactly the
contract `separate --project` chains to (ADR-0006), through the same installed
binary, without pretending the fake stems came from a model.
