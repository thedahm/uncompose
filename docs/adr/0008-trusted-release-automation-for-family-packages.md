# Trusted release automation for family packages

The family ships four PyPI packages from three repositories, and until now only this
one could publish. M6 gives `uncompose-project` and `uncompose-compare` the same
ability ([#92](https://github.com/thedahm/uncompose/issues/92) stories 21–24, slice
[#97](https://github.com/thedahm/uncompose/issues/97)). This ADR records the shape they
share, so the fifth package does not have to decide it again and a reviewer can check
any family release against one description. Each repo's own ADR records what it does
with that shape — `uncompose-project` ADR-0013, `uncompose-compare` ADR-0011.

## The shape

- **A `vX.Y.Z` tag is the only thing that publishes**, and one workflow run carries the
  release end to end: check the tag, run the gates, build, publish. The audit trail is
  one URL rather than a correlation exercise across four. A release is a fact in the
  repository's history, not an act somebody performed.

- **One version, declared in Cargo.toml; a tag is a claim about it.** `pyproject.toml`
  takes the version from Cargo (`dynamic = ["version"]`), so no two files can disagree,
  and the release refuses a tag that disagrees with that number — in the first job,
  before anything is built. The refusal is a script the repo tests as a process, not
  scaffolding around Actions internals.

- **Prerelease tags are refused, not translated.** Cargo's semver prereleases
  (`0.1.0-rc.1`) and PEP 440's (`0.1.0rc1`) do not spell the same version the same way,
  so a check that accepted them would have to invent a mapping and could no longer claim
  the tag and the package version agree. Rehearsals publish to TestPyPI from a manual
  run of the same workflow — one pipeline with a different index, not a second path to
  production.

- **The gate is the repo's own CI**, called with `workflow_call` rather than restated,
  so "the release ran the full suite" cannot drift from what the suite is.

- **Trusted Publishing (OIDC), in a named GitHub environment, with attestations.** No
  API token exists on any release path in the family, so there is none to leak, rotate,
  or scope wrong. The publish job holds `id-token: write` and every other job holds
  `contents: read`. The environment name is part of the identity the index trusts, which
  also makes it the place to add required reviewers. PEP 740 attestations make the
  published wheel name the repository, workflow, and commit that built it, so story 23's
  traceability is checkable by a stranger from the index alone.

- **Linux x86_64 wheels only, built in the manylinux container.** The v0.1 platform
  scope is Linux (ADR-0004), so no repo carries a wheel matrix. Release wheels are built
  through `PyO3/maturin-action` with `manylinux: auto` rather than on the runner, whose
  glibc would tag the wheel against itself and quietly exclude older machines.

- **The artifact that was proven is the artifact that is published.** The build job runs
  the repo's clean-install proof against the wheel it just built and uploads that wheel;
  the publish job downloads exactly it and never rebuilds.

## Where the packages differ, deliberately

- **This repo publishes an sdist; the extensions do not.** `uncompose-compare` cannot:
  its sdist would carry no frontend bundle, and its build refuses to embed an absent
  one. An extension sdist would also invite a source build on platforms v0.1 does not
  support, where "no matching distribution" is the more honest answer. The two
  extensions release together and install together, so they publish the same shape.

- **This repo's release predates the rest of the shape.** `Release` here publishes two
  packages from one tag (the CLI wheel and `uncompose-engine`), with `skip-existing` so
  a rerun after a partial publish uploads only what is missing — a two-package concern
  the extensions do not have. It does not yet check the tag against the source version,
  and it does not yet emit attestations. Bringing it in line is worth doing and is not
  part of M6, which changes this repo only for the acceptance test, checklist, and docs.

## Consequences

- Each index must be told which workflow it trusts — owner, repository, workflow file,
  environment — once per package per index. That is a human step with credentials no
  agent in this ecosystem holds; each repo's `docs/releasing.md` spells out the fields.
  Until it is done, the release builds and proves the wheel and then fails at publish.
- Renaming a release workflow file, or an environment, breaks publishing until the
  trusted publisher is updated. That is the mechanism working: the workflow's identity
  is the credential.
- The acceptance gate's `pypi` mode ([ADR-0007](0007-acceptance-test-installed-artifact-seam.md))
  is what checks that a release actually produced installable artifacts. Trusted
  publishing makes a wheel's provenance checkable; it does not make the wheel good.
