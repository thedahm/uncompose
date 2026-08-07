# Draft: uncompose comparison record schema (core)

Asset of [Comparison record schema (#65)](https://github.com/thedahm/uncompose/issues/65),
part of the ecosystem map ([#56](https://github.com/thedahm/uncompose/issues/56)). Draft, not
normative: the schema becomes real in the `uncompose-compare` repo. Decisions and rationale
live in the ticket's resolution comment. Companion draft:
[`draft-manifest-schema.md`](https://github.com/thedahm/uncompose/blob/wayfinder/62-manifest-schema/docs/research/draft-manifest-schema.md).
Family conventions (version URL, strict unknowns, `ext`) per
[#64](https://github.com/thedahm/uncompose/issues/64).

## Decisions this draft encodes

1. The record is **standalone-first**: candidates always carry their own identity
   (`label`, `path`, `sha256`, `size`), with optional `asset` + `project` fields when
   launched from a project. The file self-describes even without a manifest.
2. Record `id` is a ULID (machine-minted artifact). The human-facing slug lives on the
   manifest side (`evaluations[].id`, per [#63](https://github.com/thedahm/uncompose/issues/63)).
3. **Concealment is a session concern, not a storage concern.** The record is written at
   completion and always contains full identities; each candidate's `label` preserves what
   the listener saw (blind/randomized included). All internal references are **by label**.
   No sealed/scrubbed record variant; no incomplete-record modeling — an abandoned session
   writes nothing.
4. Observations are one shape with optional attachment fields (`position_ms`, `loop`,
   `candidate`), covering "timestamp / loop / candidate / comparison generally". Positions
   are integer milliseconds. Free `text` only in v0.1; structured vocabularies incubate
   under `ext`.
5. Loops are `{start_ms, end_ms, label?}`, an array from day one; v0.1 writes at most one.
6. Result: `preference` (label or null), `confidence` (integer 1–5, required iff preference
   non-null), optional free-text `criterion` and `summary`. **No status field**: existence =
   completed; `preference: null` = no preference.
7. `mode` is one of `ab`, `ab-blind`, `ab-blind-randomized`.
8. `playback` is reserved as an object whose contents (including any loudness-adjustment
   record) are owned by [#66](https://github.com/thedahm/uncompose/issues/66); whatever
   lands there must record adjustments actually applied, never alter source files.

## Draft JSON Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://uncompose.org/schemas/compare/v0/uncompose.compare.schema.json",
  "title": "uncompose comparison record (draft, pre-v0.1)",
  "type": "object",
  "required": ["schema", "id", "created_at", "completed_at", "candidates", "mode", "observations", "result"],
  "properties": {
    "schema": { "type": "string" },
    "id": { "type": "string", "description": "ULID" },
    "created_at": { "type": "string", "format": "date-time" },
    "completed_at": { "type": "string", "format": "date-time" },
    "candidates": {
      "type": "array",
      "minItems": 2,
      "items": { "$ref": "#/$defs/candidate" }
    },
    "mode": { "enum": ["ab", "ab-blind", "ab-blind-randomized"] },
    "playback": {
      "type": "object",
      "description": "Playback configuration incl. any loudness adjustment applied. Shape owned by #66."
    },
    "loops": {
      "type": "array",
      "items": { "$ref": "#/$defs/loop" },
      "description": "v0.1 writes at most one"
    },
    "observations": {
      "type": "array",
      "items": { "$ref": "#/$defs/observation" },
      "description": "Append-only; array order is the order observations were made"
    },
    "result": { "$ref": "#/$defs/result" },
    "context": { "type": "string", "description": "Listener-provided intent, free text" },
    "ext": { "$ref": "#/$defs/ext" }
  },
  "$defs": {
    "sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
    "ext": {
      "type": "object",
      "propertyNames": { "pattern": "^[a-z0-9][a-z0-9._-]*$" },
      "additionalProperties": { "type": "object" }
    },
    "candidate": {
      "type": "object",
      "required": ["label", "path", "sha256", "size"],
      "properties": {
        "label": { "type": "string", "description": "What the listener saw (e.g. \"A\"); the reference key everywhere in this record" },
        "path": { "type": "string" },
        "sha256": { "$ref": "#/$defs/sha256" },
        "size": { "type": "integer", "minimum": 0 },
        "asset": { "type": "string", "description": "Manifest asset id, when project-launched" },
        "project": { "type": "string", "description": "Project ULID, when project-launched" },
        "ext": { "$ref": "#/$defs/ext" }
      }
    },
    "loop": {
      "type": "object",
      "required": ["start_ms", "end_ms"],
      "properties": {
        "start_ms": { "type": "integer", "minimum": 0 },
        "end_ms": { "type": "integer", "minimum": 0 },
        "label": { "type": "string" }
      }
    },
    "observation": {
      "type": "object",
      "required": ["at", "text"],
      "properties": {
        "at": { "type": "string", "format": "date-time" },
        "position_ms": { "type": "integer", "minimum": 0 },
        "loop": { "type": "integer", "minimum": 0, "description": "Index into loops[]" },
        "candidate": { "type": "string", "description": "Candidate label; absent = the comparison generally" },
        "text": { "type": "string" },
        "ext": { "$ref": "#/$defs/ext" }
      }
    },
    "result": {
      "type": "object",
      "required": ["preference"],
      "properties": {
        "preference": {
          "type": ["string", "null"],
          "description": "Candidate label, or null = no preference"
        },
        "confidence": {
          "type": "integer", "minimum": 1, "maximum": 5,
          "description": "Required when preference is non-null, absent otherwise"
        },
        "criterion": { "type": "string" },
        "summary": { "type": "string" }
      }
    }
  }
}
```

## Worked example: blind shootout, project-launched

```json
{
  "schema": "https://uncompose.org/schemas/compare/v0/uncompose.compare.schema.json",
  "id": "01J4QF8ZK3M2X7W9C5V1B6N4TQ",
  "created_at": "2026-08-07T10:00:00Z",
  "completed_at": "2026-08-07T10:22:00Z",
  "candidates": [
    { "label": "A", "path": "song.stems-1/vocals.wav", "sha256": "bb…", "size": 26214400,
      "asset": "vocals", "project": "01J4QDX0M9V7T2Y5B3N8K6W4RZ" },
    { "label": "B", "path": "song.stems-2/vocals.wav", "sha256": "cc…", "size": 26214400,
      "asset": "vocals-2", "project": "01J4QDX0M9V7T2Y5B3N8K6W4RZ" }
  ],
  "mode": "ab-blind",
  "playback": {},
  "loops": [ { "start_ms": 62000, "end_ms": 78500, "label": "chorus" } ],
  "observations": [
    { "at": "2026-08-07T10:08:12Z", "position_ms": 73250, "loop": 0, "candidate": "A",
      "text": "vocal consonants carry more cymbal bleed" },
    { "at": "2026-08-07T10:15:40Z", "candidate": "B",
      "text": "clearer for transcription overall" },
    { "at": "2026-08-07T10:20:05Z",
      "text": "difference only really audible in the chorus" }
  ],
  "result": {
    "preference": "B",
    "confidence": 4,
    "criterion": "clarity for transcription",
    "summary": "B cleaner in the chorus; A slightly warmer but bleeds"
  },
  "context": "picking the vocal stem for practice transcription"
}
```

Standalone use differs only in `candidates[]` lacking `asset`/`project`. On project
registration the manifest's `evaluations[]` summary derives its asset-id `preference` by
mapping the label through `candidates[]` (per #63); the handover choreography is
[#67](https://github.com/thedahm/uncompose/issues/67)'s.
