# 0003 — Schema hosting as committed, pinned copies; a static-site CI lane replacing the family's Rust lanes

Status: accepted (2026-08-10)

## Context

Every manifest and comparison record in the family carries an absolute `schema` URL under
`uncompose.org` (uncompose#64) — `/schemas/project/v0/uncompose.project.schema.json` and
`/schemas/compare/v0/uncompose.compare.schema.json`. Until this repo serves those two
files, the format's own identifiers are dead links: a third party cannot fetch the schema
a file claims to conform to, and neither can `uncompose-project`'s own dispatch, which
matches the URL by exact string (ADR-0006 in `thedahm/uncompose`).

The schemas are not authored here. `uncompose-project` and `uncompose-compare` each own
and test their schema; this repo only needs to answer `GET` at the identifier URL with the
right bytes. uncompose#92 (M6) is explicit that the site's copy must never become a second
source of truth, and that the check for it belongs in this repo's own CI, not the manual,
deployed-state release checklist that verifies the redirect (ADR-0002).

`site/` also had one CI lane so far (`check_site.py`, ADR-0001's content and zero-tracking
promises). uncompose#68's family convention is Rust lanes (`cargo fmt`, `clippy`, `test`);
this repo has no Rust and no build step at all, so `check_site.py`'s introduction already
enumerated the divergence. This slice adds three more checks to the same lane rather than
opening a second one.

## Decision

**Hosting.** The two schemas are committed as byte copies under `site/schemas/`, so
`site/` — the deploy's output directory (ADR-0001) — serves them at exactly
`/schemas/project/v0/uncompose.project.schema.json` and
`/schemas/compare/v0/uncompose.compare.schema.json`. `site/_headers`
(Cloudflare Pages' header-override file) pins `Content-Type: application/json` on both
paths rather than relying on static-host MIME inference, since these URLs are the
format's own identifiers and off-the-shelf validators fetch them directly.

**Provenance, not authority.** `schemas/sources.json` and `schemas/README.md` (repo docs,
not served — outside `site/`) record, per schema, the owning repo, the file path there,
and the exact ref the committed copy was pulled from. Neither tool has tagged a v0.1.0
release yet, so both pins are commit SHAs rather than tags; the release checklist
(uncompose#92 story 25) moves each pin to its tool's release tag going forward and is the
only place these pins change. The source of truth stays the owning repo and its own
conformance tests — this repo's copy is documented, everywhere it's mentioned, as
non-authoritative.

**CI.** `tests/check_schemas.py` fetches each schema from its pinned ref on GitHub
(`raw.githubusercontent.com`) and fails if the committed copy has drifted, so staleness
between what's served and what's pinned is caught automatically rather than trusted. This
is the one part of this repo's checks that reaches the network; ADR-0002 already draws
the line this respects — deployed Pages/DNS state (the live `.cc` redirect, the live
schema URLs) is unreproducible in CI and stays in the manual release checklist, but the
*tool repos'* GitHub state is exactly what the pin claims to track, so checking it here is
the whole point of recording a ref at all.

Two more checks land alongside it, closing the rest of uncompose#92's static-site-CI ask:
`tests/check_html.py` (structural HTML validity — doctype, matching tags, `lang`, a
charset, unique ids — written against the stdlib rather than installing a validator, for
the same reason `check_site.py` did) and `tests/check_links.py` (every internal
`href`/`src`/`srcset` reference, including `#fragment`s, resolves to a real file and id
under `site/`).

## Alternatives considered

- **Fetch the schemas at request time (a Pages Function / edge redirect to the tool
  repos).** Would need Cloudflare Workers/Functions — infrastructure this repo has
  avoided everywhere else (ADR-0002) — and would make `uncompose.org`'s uptime depend on
  GitHub's raw-content host being reachable from every visitor, not just from CI.
- **Automated schema syncing (a bot or Action that re-pulls on every tool release).**
  Explicitly out of scope for M6 (uncompose#92): "manual copy in v0.1, automation is a
  parked backlog item." Automating the *check* that a manual pin has drifted is in scope;
  automating the pull itself is not.
- **`npx html-validate` for the HTML check**, as tried by hand in M6 slice 1. Reintroduces
  the Node dependency `check_site.py`'s own docstring already rejected ("a checker that
  needed installing would be the toolchain arriving through the back door").
- **Recording the pin as a tag placeholder** (e.g. `"pending-v0.1.0"`) instead of a real
  commit SHA. A placeholder can't be fetched, so the byte-identity check would have
  nothing to verify against until the first tag — the exact window (pre-release) this
  check is most useful in, since the schemas are still moving.

## Consequences

- A schema shape change in either tool repo is silent here until someone re-pulls the
  copy and re-pins the ref — `check_schemas.py` only catches drift *from the pinned ref*,
  not staleness of the pin itself against the tool's latest commit or release.
- CI now depends on outbound network access to GitHub for one check. If GitHub is
  unreachable, `check_schemas.py` fails closed (reports it as a violation) rather than
  skipping, per uncompose#92's "fails on violations."
- The three new checks are additive: `check_site.py` keeps its original, narrower scope
  (content and zero-tracking) rather than growing to cover structure, links, and schemas
  too.
