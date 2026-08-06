# Draft: uncompose.project.json manifest schema (core)

Asset of [Project manifest schema core (#62)](https://github.com/thedahm/uncompose/issues/62),
part of the ecosystem map ([#56](https://github.com/thedahm/uncompose/issues/56)). Draft, not
normative: the schema becomes real in the `uncompose-project` repo. Decisions and rationale
live in the ticket's resolution comment; prior-art grounding in
[`provenance-prior-art.md`](https://github.com/thedahm/uncompose/blob/research/provenance-prior-art/docs/research/provenance-prior-art.md).

## Decisions this draft encodes

1. Four top-level object kinds: `project`, `assets`, `derivations`, `evaluations` (reserved).
2. Integrity state (`verified` / `modified` / `missing`) is **derived** by the tool from
   sha256+size vs disk, never stored; at most a cached `last_verified` timestamp per asset.
3. Ids: ULID for the project; human-readable slugs (`^[a-z0-9][a-z0-9._-]*$`) for assets and
   derivations, auto-minted, user-overridable at creation, **immutable once referenced**.
   No per-asset UUIDs.
4. Identity is sha256+size over exact file bytes, computed **at registration** (no unhashed
   state, no audio canonicalization — a tag edit is a modification).
5. Paths are location hints: relative to the project root (the manifest's directory),
   forward slashes, must resolve inside the root (no `../`, no absolute paths) in v0.1.
6. Derivations are the minimal PROV triple: inputs + outputs (asset ids) + activity
   (tool, tool_version, opaque `params`, one RFC3339 UTC `created_at`). Optional hashed
   reference to `job.json` (path + sha256) — referenced, never absorbed. `tool: "manual"`
   is legal for out-of-band edits.
7. `role` is an open lowercase-slug string; recommended starter vocabulary `mix`, `stem`,
   `reference`. Not an enum.
8. One file, fixed name `uncompose.project.json` at project root. Collections are arrays of
   objects carrying their `id` (registration order = history); the tool enforces id
   uniqueness and writes canonically (fixed field order, 2-space indent, trailing newline,
   atomic temp+rename).
9. Deliberately left to sibling tickets: the schema-version URL and extension-field
   conventions ([#64](https://github.com/thedahm/uncompose/issues/64)); imported-job field
   mapping and the shape of `evaluations[]`
   ([#63](https://github.com/thedahm/uncompose/issues/63)).

## Draft JSON Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://uncompose.org/schemas/project/v0/uncompose.project.schema.json",
  "title": "uncompose.project.json (draft, pre-v0.1)",
  "type": "object",
  "required": ["schema", "project", "assets", "derivations", "evaluations"],
  "properties": {
    "schema": {
      "type": "string",
      "description": "Absolute schema URL, compared by exact string. Shape owned by #64."
    },
    "project": {
      "type": "object",
      "required": ["id", "name", "created_at"],
      "properties": {
        "id": { "type": "string", "description": "ULID, minted at project init" },
        "name": { "type": "string" },
        "created_at": { "type": "string", "format": "date-time" }
      }
    },
    "assets": {
      "type": "array",
      "items": { "$ref": "#/$defs/asset" }
    },
    "derivations": {
      "type": "array",
      "items": { "$ref": "#/$defs/derivation" }
    },
    "evaluations": {
      "type": "array",
      "description": "Reserved. Item shape owned by #63.",
      "items": { "type": "object" }
    }
  },
  "$defs": {
    "slug": { "type": "string", "pattern": "^[a-z0-9][a-z0-9._-]*$" },
    "sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "asset": {
      "type": "object",
      "required": ["id", "path", "sha256", "size", "role", "added_at"],
      "properties": {
        "id": { "$ref": "#/$defs/slug" },
        "path": {
          "type": "string",
          "description": "Relative to the project root, forward slashes, resolves inside the root. Location hint, not identity."
        },
        "sha256": { "$ref": "#/$defs/sha256" },
        "size": { "type": "integer", "minimum": 0 },
        "role": {
          "$ref": "#/$defs/slug",
          "description": "Open vocabulary; recommended: mix, stem, reference"
        },
        "added_at": { "type": "string", "format": "date-time" },
        "last_verified": {
          "type": "string",
          "format": "date-time",
          "description": "Cache of the last successful integrity check. Never a status claim."
        }
      }
    },
    "derivation": {
      "type": "object",
      "required": ["id", "inputs", "outputs", "tool", "created_at"],
      "properties": {
        "id": { "$ref": "#/$defs/slug" },
        "inputs": { "type": "array", "items": { "$ref": "#/$defs/slug" }, "minItems": 1 },
        "outputs": { "type": "array", "items": { "$ref": "#/$defs/slug" }, "minItems": 1 },
        "tool": { "type": "string", "description": "e.g. \"uncompose\"; \"manual\" for out-of-band edits" },
        "tool_version": { "type": "string" },
        "params": { "type": "object", "description": "Opaque to the schema; owned by the tool that wrote it" },
        "created_at": { "type": "string", "format": "date-time" },
        "job": {
          "type": "object",
          "required": ["path", "sha256"],
          "properties": {
            "path": { "type": "string" },
            "sha256": { "$ref": "#/$defs/sha256" }
          },
          "description": "Hashed reference to an imported job.json (C2PA hashed-ref pattern). Referenced, never absorbed."
        }
      }
    }
  }
}
```

## Worked example: the separation-shootout vertical slice

```json
{
  "schema": "https://uncompose.org/schemas/project/v0/uncompose.project.schema.json",
  "project": {
    "id": "01J4QDX0M9V7T2Y5B3N8K6W4RZ",
    "name": "separation-shootout",
    "created_at": "2026-08-06T14:00:00Z"
  },
  "assets": [
    { "id": "song", "path": "song.wav", "sha256": "aa…", "size": 52428800, "role": "mix", "added_at": "2026-08-06T14:01:00Z" },
    { "id": "vocals", "path": "song.stems-1/vocals.wav", "sha256": "bb…", "size": 26214400, "role": "stem", "added_at": "2026-08-06T14:10:00Z" },
    { "id": "vocals-2", "path": "song.stems-2/vocals.wav", "sha256": "cc…", "size": 26214400, "role": "stem", "added_at": "2026-08-06T14:20:00Z" }
  ],
  "derivations": [
    {
      "id": "separation-1",
      "inputs": ["song"],
      "outputs": ["vocals"],
      "tool": "uncompose",
      "tool_version": "0.1.0",
      "params": { "preset": "6-stem" },
      "created_at": "2026-08-06T14:10:00Z",
      "job": { "path": "song.stems-1/job.json", "sha256": "dd…" }
    },
    {
      "id": "separation-2",
      "inputs": ["song"],
      "outputs": ["vocals-2"],
      "tool": "uncompose",
      "tool_version": "0.1.0",
      "params": { "preset": "experimental" },
      "created_at": "2026-08-06T14:20:00Z",
      "job": { "path": "song.stems-2/job.json", "sha256": "ee…" }
    }
  ],
  "evaluations": []
}
```

`vocals@separation-1` addressing resolves as: derivation `separation-1`, output asset whose
role/name matches `vocals` — the exact resolution rule belongs to the integration-contracts
ticket ([#67](https://github.com/thedahm/uncompose/issues/67)).
