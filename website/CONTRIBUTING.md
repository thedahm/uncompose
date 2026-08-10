# Contributing to the Uncompose Website

Thanks for your interest in the Uncompose website. The site is pre-v0.1 and this guide is
intentionally minimal. It grows into a full contributor guide once v0.1 exists.

## Working on the site

The repo has no toolchain: no framework, no static-site generator, no build step
([ADR-0001](docs/adr/0001-static-html-no-build-step.md)). Everything published lives in
`site/`, and what is in `site/` is exactly what a visitor receives.

- Preview: `python3 -m http.server -d site 8000`, then open `http://localhost:8000`.
- Check: `python3 tests/check_site.py` — the same command CI runs, using nothing but a
  stock Python 3. It fails if the page loses one of the brief's tool definitions, the
  command picture, the local-first copy, or a source/package link, and if anything on the
  page would make a request off this origin.

Changes to the page are checked at the seam a visitor meets: the delivered text and the
requests the browser would make, never the shape of the markup. Style the page however
reads best; just keep the promises testable.

Please don't add a dependency, a generator, a bundler, or an analytics snippet. Each is a
decision the site has already made in the other direction, and reversing one is an ADR,
not a pull request.

## Governance

Uncompose is created and maintained by Dominic Hanzely ([@thedahm](https://github.com/thedahm)),
who acts as the project's maintainer and final decision-maker. Significant decisions are
recorded as numbered architecture decision records in [`docs/adr/`](docs/adr/), so the
reasoning behind the project's choices is public and reviewable. Ecosystem-wide decisions
live in the [`thedahm/uncompose`](https://github.com/thedahm/uncompose) ADR series and are
cited explicitly where they apply. Issues and pull requests are answered on a best-effort
basis.

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
