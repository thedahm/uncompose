# Schema provenance

The two files served at `/schemas/project/v0/` and `/schemas/compare/v0/`
(`site/schemas/...`) are **committed copies pulled from the tool repos, not the
authoritative source**. The source of truth for each schema is the owning repo and the
conformance tests that exercise it:

| Served path | Owner | Pinned from |
| --- | --- | --- |
| `schemas/project/v0/uncompose.project.schema.json` | [`thedahm/uncompose-project`](https://github.com/thedahm/uncompose-project) (`schemas/project/v0/`) | `main` @ [`d178b5d`](https://github.com/thedahm/uncompose-project/commit/d178b5de5f4eb171e1ef1fcecb9bbd917be70bd4) |
| `schemas/compare/v0/uncompose.compare.schema.json` | [`thedahm/uncompose-compare`](https://github.com/thedahm/uncompose-compare) (`schemas/compare/v0/`) | `main` @ [`9e7fd4a`](https://github.com/thedahm/uncompose-compare/commit/9e7fd4aae87b8199d3a23142ef3d02ce66424f8e) |

[`sources.json`](sources.json) records the same pins in a form `tests/check_schemas.py`
reads. It lives here rather than under `site/` because it is repo documentation, not
something a schema consumer needs — `site/` is exactly what a visitor and a validator
receive (ADR-0001).

Neither tool has cut a v0.1.0 tag yet, so both pins are commit SHAs rather than release
tags. Once a tool tags a release, its pin here moves to that tag as part of the release
checklist (uncompose#92 story 25) — this is manual by design, the same way the vendored
copy in `uncompose-project`'s own `schemas/vendor/` is manually pinned.

CI (`tests/check_schemas.py`) fetches each schema from its pinned ref on GitHub and fails
if the committed copy has drifted, so staleness between what's served and what's pinned
is caught rather than silently accumulating. It also holds this file to `sources.json`:
each entry's `ref_kind` must match the shape of its `ref`, the table above must name the
same refs, a `tag` pin must no longer carry the pre-v0.1.0 `note`, and the paragraph above
must be gone once every pin is a tag — so a half-run refresh is red rather than green over
a record that has stopped being true. What it cannot catch is the pin itself going stale
against the tool repo's latest release; that is what the release checklist step is for.
