# Contributing to Uncompose

Thanks for your interest in Uncompose. The project is pre-v0.1: there is no code to
contribute to yet, and this guide is intentionally minimal. It grows into a full
contributor guide once v0.1 exists.

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
