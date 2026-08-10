# Uncompose Website

The Uncompose family's front door: one static page at uncompose.org saying what the
ecosystem is, published straight from this repo with no build step and no tracking.

## Language

**Site**:
Everything published at uncompose.org. Its source is the `site/` directory and nothing
else in this repo — `site/` _is_ the deploy's output directory, so repo docs, ADRs, and
checks are never served.
_Avoid_: web app, docs site

**Landing page**:
`site/index.html`, the only page in v0.1. Its content is fixed by
[uncompose#71](https://github.com/thedahm/uncompose/issues/71): the three tools with the
brief's one-line definitions, the command picture, local-first copy, and links to the
repos and packages — exactly that, and nothing more.
_Avoid_: homepage, index, front page

**Deploy**:
A Cloudflare Pages build triggered by a push to `main`. It runs no build command; it
uploads `site/` as-is. A deploy is the only way content reaches visitors.
_Avoid_: publish, release, ship

**Redirect**:
The permanent 301 from uncompose.cc — apex, `www`, and every path — to the matching
uncompose.org URL. It is a Cloudflare redirect rule on the `.cc` zone, not anything this
repo serves.
_Avoid_: forward, alias

**Schema**:
A JSON Schema owned by `uncompose-project` or `uncompose-compare`, served here at the
exact identifier URL its files carry (e.g.
`/schemas/project/v0/uncompose.project.schema.json`). The file under `site/schemas/` is a
committed copy, not the source — it is pulled from the owning repo at a pinned ref
recorded in [`schemas/sources.json`](schemas/sources.json) and is never authoritative.
_Avoid_: spec, format definition

**Pinned ref**:
The repo, path, and commit (or, after a tool's first tag, release tag) a schema copy was
pulled from, recorded in `schemas/sources.json` and checked by `tests/check_schemas.py`
against the owning repo on every CI run. Moving a pin forward is a release-checklist step
([uncompose#92](https://github.com/thedahm/uncompose/issues/92) story 25), not a routine
edit.
_Avoid_: version, source of truth (the schema's actual source of truth is the owning
repo's conformance tests, not this pin)
