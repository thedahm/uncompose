# The release checklist is a document a human fills in, not a test

M6 gates a coordinated family release three ways: the automated acceptance test
([ADR-0007](0007-acceptance-test-installed-artifact-seam.md)) checks that the installed
artifacts work; trusted release automation
([ADR-0008](0008-trusted-release-automation-for-family-packages.md)) checks that a
publish is traceable to a tag and a green suite; neither can check that a stranger could
actually follow our documentation, or that what we believe is deployed is what's really
live. That's the third gate ([#92](https://github.com/thedahm/uncompose/issues/92) story
37, slice [#100](https://github.com/thedahm/uncompose/issues/100)):
[`docs/release-checklist.md`](../release-checklist.md), the kickoff
[brief](https://github.com/thedahm/uncompose/issues/55)'s twelve-step definition of done
for the first public phase, plus the website's schema refresh (story 25) and the two
checks that only exist as deployed state.

## Decisions

- **One document holds all three concerns** — the docs-alone DoD walk, the schema
  refresh, the deployed-state checks — rather than three. They share what matters here:
  none of them can be honestly automated, and the issue that created this document (#100)
  scoped them together. Splitting them would let one get run without the others.

- **"Docs-alone" excludes this repo's own ADRs**, even though they're public. ADRs
  record why a decision was made, not how to use the shipped thing; a checker who
  needed one to get through a step has actually found an undocumented step; letting
  ADRs count would hide that.

- **A checklist failure is a finding, not a workaround.** The document says so
  explicitly: skipping a gap because the runner already knows the answer defeats the
  entire point, which is verifying what a newcomer can do, not what a maintainer can do.

- **Filled copies are committed files**, `docs/releases/checklists/<date>-<tags>.md`,
  not a GitHub issue comment, a gist, or an external tracker. The family already treats
  git as the record for everything else it cares about keeping (ADRs, release notes);
  a checklist run is no different, and a committed file is greppable and diffable the
  same way. The filename carries the date and the tags being gated rather than a single
  version number, because the two extension repos return to independent semver right
  after the coupled cut, and a future run may gate one tag, not two.

- **This never becomes CI**, unlike everything else in the family's release path. A
  script that "passes the checklist" would only prove the script agrees with itself
  about what the docs say; the claim being verified is specifically that a human,
  reading only what's published, can get through the slice. Automating the check
  changes what's being checked.

## Consequences

- The checklist has to be re-run, by hand, on every coordinated release — it doesn't
  get cheaper or faster over time the way the automated gates do. That cost is the
  point: it's the same cost a real new user pays.
- When a step turns up undocumented, the fix lands in the repo that owns the gap
  (usually not this one), and the checklist reruns after — it's a gate on the release,
  not a one-time audit.
- `docs/releases/checklists/` doesn't exist until the first release checklist is run;
  nothing in this change creates it early.
