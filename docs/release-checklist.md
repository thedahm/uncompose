# Release checklist: the docs-alone pass

This is the manual gate for a coordinated Uncompose family release (the coupled v0.1.0
cut, and any coordinated release after it). It exists to answer one question by hand,
every time it's claimed: *can a stranger with no inside knowledge actually do this,
using only what we published?* [ADR-0007](adr/0007-acceptance-test-installed-artifact-seam.md)'s
acceptance test answers "does the installed artifact work" by running code; this answers
"does the documentation carry a new user through the whole slice" and "is the deployed
state actually what we think it is" — neither of which a subprocess can honestly check.
See [ADR-0009](adr/0009-docs-alone-release-checklist.md) for why this stays a document a
human fills in rather than something automated, and the
[M6 spec](https://github.com/thedahm/uncompose/issues/92) (story 37) for where it comes
from.

## The docs-alone rule

Run this as a stranger would, not as the person who wrote the feature:

- **Use only what is public at release time**: this repo's `README.md` and `docs/`
  (excluding `docs/adr/` — ADRs record *why*, not *how to use*), each extension's own
  `README.md` and `docs/`, each package's PyPI project page, and the live
  `https://uncompose.org` site.
- **Do not use**: source code, tests, code comments, CI configuration, issue or PR
  history, or anything recalled from having built the thing. If a step only works
  because you remember an internal detail, it fails, even if your hands know what to do.
- **A step that needs something undocumented is a checklist failure**, not a shortcut to
  take silently. Write down what was missing, file it against the repo that owns the
  gap, and don't record this checklist as passed until it's fixed and re-run.

Ideally run by someone other than whoever wrote the docs being checked. If that's not
available, run it on a machine with nothing checked out, days removed from having
written the change, reading only what's linked above.

## Part 1 — the vertical slice, from the brief's definition of done

The [kickoff brief](https://github.com/thedahm/uncompose/issues/55)'s "definition of done
for the first public phase," unchanged, as twelve performable checks. For each, note which
published document actually got you through it.

1. **Install Uncompose and its two extensions.**
   Docs: root `README.md` § Install, each extension's `README.md` § Install, the three
   PyPI project pages.
2. **Create an audio project.**
   Docs: [`docs/walkthrough.md`](walkthrough.md) § 2, or `uncompose-project`'s own docs.
3. **Register an original recording.**
   Docs: [`docs/walkthrough.md`](walkthrough.md) § 3 (`uncompose separate --project`).
4. **Perform or import two separation runs.**
   Docs: [`docs/walkthrough.md`](walkthrough.md) § 3 (a second `separate --project` with
   a different preset), or `uncompose-project`'s `import` docs.
5. **Inspect the derivation graph.**
   Docs: [`docs/walkthrough.md`](walkthrough.md) § 4 (`uncompose project show`).
6. **Verify that the referenced files remain unchanged.**
   Docs: [`docs/walkthrough.md`](walkthrough.md) § 6 / `uncompose-project`'s docs
   (`uncompose project verify`).
7. **Launch a blind comparison between two project assets.**
   Docs: [`docs/walkthrough.md`](walkthrough.md) § 5, `uncompose-compare`'s own docs for
   `--project` and blind mode.
8. **Select and repeat a meaningful musical region.**
   Docs: `uncompose-compare`'s own docs and its workbench UI (region select and loop).
9. **Record timestamped observations.**
   Docs: `uncompose-compare`'s own docs and its workbench UI (pins).
10. **Select a preference and confidence level.**
    Docs: `uncompose-compare`'s own docs and its workbench UI (the verdict step).
11. **Save the comparison.**
    Docs: `uncompose-compare`'s own docs (save and close) / `docs/walkthrough.md` § 5.
12. **Inspect the project and see the evaluation connected to the assets it judged.**
    Docs: [`docs/walkthrough.md`](walkthrough.md) § 6 (`uncompose project show`,
    `uncompose project verify`).

## Part 2 — website: what a release makes stale (manual in v0.1, per #71 / story 25)

13. **Refresh the website's committed schema copies from each tool's tagged release.**
    In `uncompose-website`: for each of `uncompose-project` and `uncompose-compare`,
    copy `schemas/*/v0/*.schema.json` from the release tag into the matching
    `site/schemas/.../` path, and move that pin in `schemas/sources.json` from the old
    commit SHA (or tag) to the new release tag — see that repo's `schemas/README.md` for
    the exact convention. The rest of the pin record moves with it: set `ref_kind` to
    `tag`, drop the entry's pre-v0.1.0 `note`, and update the "Pinned from" table and the
    "neither tool has cut a v0.1.0 tag yet" paragraph in `schemas/README.md`.
    `check_schemas.py` cross-checks all four of those against `sources.json`, so a
    half-done refresh is a red check — but it cannot tell you the pin has gone stale
    against a release you never came here to record, which is why this step exists.
    Confirm `tests/check_schemas.py` passes against the new pins before pushing.

14. **Refresh the landing page's release-status copy.**
    `site/index.html` carries a paragraph about which packages are real yet — before the
    v0.1.0 cut it reads *"Project and Compare are pre-v0.1: their packages are reserved
    placeholders until the first release."* The moment this release publishes working
    wheels that sentence is false, and it discourages exactly the `pip install` the page
    exists to enable. Rewrite it for what shipped (or delete it, once nothing on the page
    is a placeholder), and re-run `tests/check_site.py`. Nothing checks this paragraph
    automatically: it is prose about the world outside the repo, which is what makes it a
    checklist item rather than a check.

## Part 3 — deployed state (CI can't honestly reproduce this)

Both of these depend on live DNS, Cloudflare account state, and the current Pages
deployment — exactly what `uncompose-website`'s CI is scoped to not touch. Run the
verify block in that repo's `docs/deploy.md` ("## 4. Verify") against production:

15. **`uncompose.cc` (apex and `www`, any path and query) 301s to the matching
    `uncompose.org` URL**, not a 302, with the path and query string preserved.
16. **Both schema identifier URLs resolve on the live site** (`200`, `content-type`
    starting `application/json`):
    `https://uncompose.org/schemas/project/v0/uncompose.project.schema.json` and
    `https://uncompose.org/schemas/compare/v0/uncompose.compare.schema.json`.

## Recording a filled copy

Each run gets committed as its own file at
`docs/releases/checklists/<date>-<tags>.md` — for example
`docs/releases/checklists/2026-08-10-project-v0.1.0-compare-v0.1.0.md` — a copy of this
checklist with every item marked pass/fail, the document that got you through it (or the
gap that didn't), and who ran it. The file is the record; nothing elsewhere needs to
point at it, and git's own history says when it ran. A release isn't done until its
checklist file is committed, all sixteen items pass, and Parts 2 and 3 were run against
the artifacts and site that release actually shipped, not a rehearsal.
