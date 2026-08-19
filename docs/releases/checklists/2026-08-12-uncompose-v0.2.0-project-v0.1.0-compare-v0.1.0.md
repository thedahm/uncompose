# Release checklist record — 2026-08-12

**Release:** `uncompose` v0.2.0 + `uncompose-engine` v0.2.0 (coupled tag),
`uncompose-project` v0.1.0, `uncompose-compare` v0.1.0.
**Checklist version:** [`docs/release-checklist.md`](../../release-checklist.md) as merged in #114.
**Part 1 run by:** Dominic Hanzely (maintainer — the docs were largely agent-written this
milestone, so the maintainer is the closest available stranger), on a real project
(`~/Music/military-church`, a full song separated twice on CUDA), fresh install from PyPI,
public docs only.
**Parts 2–3 run by:** Claude (operator session), against the artifacts and site this
release actually shipped.

Context worth keeping: the release grew mid-checklist. The PyPI-mode acceptance run
caught that the published root `uncompose` 0.1.0 predated external-command dispatch, so
root v0.2.0 was cut (#118) before Part 1 could be walked at all. The gate doing that job
is why it exists.

## Part 1 — the vertical slice, docs-alone

| # | Check | Result | Notes |
| --- | --- | --- | --- |
| 1 | Install all three | **pass** | Root README + extension READMEs/PyPI pages. Finding: walkthrough says `uv`, extension READMEs say `pip` — inconsistent install story, filed as [#120](https://github.com/thedahm/uncompose/issues/120). |
| 2 | Create a project | **pass** | `docs/walkthrough.md` § 2. |
| 3 | Register an original recording | **pass** | Walkthrough § 3. |
| 4 | Two separation runs | **fail, then pass on retry** | First `separate --project` succeeded as a separation but failed chained registration: `job.json` records an absolute `input_path`, and the mix was not yet a registered asset, so import refused it. Repaired by hand-editing `job.json` to a root-relative path (undocumented). Second run registered cleanly (input resolved by hash). Filed as [#119](https://github.com/thedahm/uncompose/issues/119); per the checklist's own rule this step is recorded as a failure with a filed gap, accepted by the maintainer for this release because the error's stated workaround (`uncompose-project add` first) is functional and the failure only bites a project's first-ever separation. |
| 5 | Inspect the derivation graph | **pass** | Walkthrough § 4, `uncompose project show`. |
| 6 | Verify referenced files | **pass** | Walkthrough § 6, `uncompose project verify`. |
| 7 | Launch a blind comparison | **pass** | Walkthrough § 5 + compare docs. Two observations, neither a docs gap: (a) a stale pre-release `uncompose-compare` binary on `PATH` shadowed the published wheel and refused the manifest with a pre-#55 error — environment issue, cleared by a forced reinstall; (b) compare correctly refused to blind-compare `vocals` vs `vocals-2` as identical audio (both presets extract vocals with the same model, deterministically), so the comparison ran on stems that differ. |
| 8 | Select and repeat a region | **pass** | Compare workbench. |
| 9 | Timestamped observations (pins) | **pass** | Compare workbench. |
| 10 | Preference and confidence | **pass** | Compare workbench verdict step. |
| 11 | Save the comparison | **pass** | Conclude wrote the record into `evaluations/` and registered it. |
| 12 | Evaluation connected to assets | **pass** | `show` lists the evaluation with candidates/preference/record ref; `verify` green over assets and the record file. |

Bonus exercise beyond the twelve: a real Moises 6-stem export was imported as a
derivation via a hand-written evidence `job.json` and blind-compared against the local
run — the project's founding use case, performed. The ergonomics gap it exposed
(external runs masquerade as `tool: "uncompose"`, and the evidence contract shouldn't be
user-facing) is filed as
[uncompose-project#40](https://github.com/thedahm/uncompose-project/issues/40).

## Part 2 — website refresh

| # | Check | Result | Notes |
| --- | --- | --- | --- |
| 13 | Schema pins to release tags | **pass** | uncompose-website#11: `sources.json` pins moved to the `v0.1.0` tags (`ref_kind: tag`, pre-v0.1.0 notes dropped), README table and paragraph updated. Tagged schemas verified byte-identical to the committed copies before repinning. `check_schemas.py` green. |
| 14 | Landing-page release-status copy | **pass** | Same PR: the "reserved placeholders" sentence replaced with what shipped (all three install from PyPI; uncompose 0.2.0, extensions 0.1.0). `check_site.py` green. |

## Part 3 — deployed state

Run against production after uncompose-website#11 deployed, 2026-08-12.

| # | Check | Result | Notes |
| --- | --- | --- | --- |
| 15 | `.cc` → `.org` 301s | **pass** | `uncompose.cc/docs/x?q=1&r=2`, `www.uncompose.cc/a/b?z=9`, and `www.uncompose.org/p?q=1` each 301 (not 302) to the matching `uncompose.org` URL with path and query preserved. |
| 16 | Schema identifier URLs live | **pass** | Both URLs return `200` with `content-type: application/json`; bodies sha256-identical to the repo copies. |

## Verdict

15 of 16 pass; item 4 is a recorded failure with a filed gap (#119) accepted by the
maintainer. Supporting evidence: TestPyPI rehearsals for both extensions, PyPI-mode
acceptance run [31621802969](https://github.com/thedahm/uncompose/actions/runs/31621802969)
green against the published wheels (root 0.2.0, extensions 0.1.0), all four packages
published via trusted publishing with no long-lived tokens.
