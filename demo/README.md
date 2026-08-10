# Demonstration project

`build_demo.py` reproduces the finished state of the Uncompose vertical slice —
a project manifest carrying a source asset, two derivations with their stems,
and one evaluation — on any Linux machine, in seconds, with no GPU and no model
downloads.

```sh
python3 demo/build_demo.py            # builds ./uncompose-demo/
python3 demo/build_demo.py my-dir     # or into a directory you name
```

It needs the three tools installed and on `PATH`:

```sh
uv tool install uncompose uncompose-project uncompose-compare
```

Exit 0 means the whole end state landed — the source asset, both derivations,
*and* the evaluation. If the installed `uncompose-project` writes a manifest
this script cannot read, it says which expectation broke and exits nonzero
rather than reporting success over a partial project.

## The separations are synthetic

**No separation model runs here.** The "stems" are deterministic seeded noise
and the job folders that hold them are hand-built to the job-record shape. What
the script genuinely exercises is the *import contract* — `uncompose project
import` turning a job folder into a derivation and a comparison record into an
evaluation, then `uncompose project verify` confirming every referenced file.
That is the integration surface M5 delivers; the audio is fake so the end state
is reproducible anywhere.

For the real thing — `uncompose separate --project` on your own recording and
`uncompose compare --project` in the browser — follow
[`docs/walkthrough.md`](../docs/walkthrough.md). The demo is that walkthrough's
populated result, made reproducible without audio.

## Determinism

Fixture generation is a pure function of the constants at the top of the script:
no wall-clock, no unseeded randomness, no absolute paths. Running

```sh
python3 demo/build_demo.py --fixtures-only DIR
```

writes only the fixtures (source recording plus synthetic job folders) and
touches no external tool; rerunning yields byte-identical files. The repo's CI
checks exactly this (`cli/tests/demo.rs`), so the demonstration can't silently
drift.
