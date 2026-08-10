# Coding Standards

<!-- The reviewer agent loads this file during code review via
     @.sandcastle/CODING_STANDARDS.md so these standards are enforced during
     review without costing tokens during implementation. -->

## Style

- HTML and CSS are hand-written and read like prose: semantic elements, no class soup, no
  inline styles. Two-space indentation.
- No JavaScript. No dependency, generator, bundler, or webfont — each is a decision made
  in the other direction (ADR-0001) and reversing one is an ADR, not a change.
- Use the project vocabulary from `CONTEXT.md` (site, landing page, deploy, redirect) and
  avoid the listed banned synonyms.

## Testing

- Checks run at the seam a visitor meets: the delivered text of the page and the requests
  a browser would make. Never assert on markup shape — a class rename must not fail a
  test, and a lost promise must.
- `tests/check_site.py` runs on a stock Python 3 with no dependencies. A checker that
  needed installing would be the toolchain arriving through the back door.

## Documentation

- Documentation carries rationale, not narration (see CONTRIBUTING.md). Comments state
  constraints the code can't show; delete comments that restate what the code says.
- "We did X instead of Y because" belongs in `docs/adr/`, not in scattered comments.
- Off-repo state (Cloudflare account configuration) is recorded in `docs/deploy.md`
  exactly, because nobody can diff it.

## Architecture

- `site/` is the published surface and the deploy's output directory; nothing outside it
  is ever served. Repo docs and checks stay outside it.
- Content claims on the page are promises: the local-first copy, the tool definitions, and
  the zero-tracking claim are checked, not asserted.
