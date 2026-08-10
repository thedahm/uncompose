# Chained registration in `separate --project`

`uncompose separate --project <dir>` separates a recording and then registers the
finished job in that project's manifest, so provenance capture costs the user
nothing extra. This ADR records how the root CLI hands the job to
`uncompose-project` and what happens when that hand-off fails. Decided in the M5
integration contracts ([#67](https://github.com/thedahm/uncompose/issues/67),
resolutions 1, 4, 7, 9) and the M5 spec
([#89](https://github.com/thedahm/uncompose/issues/89), slice 3).

## Single writer, invoked as a subprocess

Only `uncompose-project` writes `uncompose.project.json`. The root CLI never
touches the manifest directly; it registers by invoking `uncompose-project` as a
subprocess. There is no shared manifest-writing library — canonical
serialization, slug minting, dedupe, and schema round-trip live in exactly one
codebase (#67 res. 1).

The invocation is the pinned cross-tool argv:

```text
uncompose-project import --project <abs-root> <abs-job.json>
```

Explicit `--project` flag, absolute paths, no reliance on the working directory,
so it works identically from any cwd (#67 res. 9). `import` dispatches on the
input file's schema URL; a `job.json` (which carries no `schema` field by design)
imports as a derivation.

## Pre-flight before any engine work

Every foreseeable failure is caught before provisioning or inference, so a
doomed run fails in milliseconds rather than after minutes of GPU work (#67
res. 4). Pre-flight, in order:

1. The manifest exists at exactly `<dir>/uncompose.project.json` — `--project`
   names the project root itself, with **no upward directory walk**, so a parent
   project can never silently capture the work.
2. The manifest parses as JSON whose top-level `schema` equals the project v0 URL
   (`https://uncompose.org/schemas/project/v0/uncompose.project.schema.json`).
   This exact-string match is the "is this a project manifest at all" gate; full
   strict validation stays `uncompose-project`'s job (#67 res. 9, no version
   handshake).
3. `uncompose-project` resolves on `PATH`, else fail fast with the family install
   hint (`uv tool install uncompose-project`) — the same hint dispatch prints on
   a `project` miss (#67 res. 1). The check reuses dispatch's own `PATH` walk so
   the two never disagree about what is runnable.
4. The input path resolves inside the project root.
5. The resolved job folder base — the default `<input>.stems` next to the input,
   or an `-o` override — resolves inside the project root. Refusing an out-of-root
   `-o` here, rather than letting `import` refuse after the separation, is new to
   this slice: the destination is known before any work, so a doomed override
   costs nothing.

Any pre-flight failure exits nonzero with a self-explanatory message before the
engine environment is touched.

## Failure semantics: the separation is never the casualty

After the existing success path — `job.json` written last as the completion
marker — the chained import runs. Exit 0 means separated **and** registered.

When import fails, the expensive separation must survive intact (#67 res. 7):

- The job folder and `job.json` are already written and are left **untouched** —
  no rollback, no pending state, no retries.
- The import's stderr is relayed (the subprocess inherits our stderr).
- The CLI exits nonzero, carrying the import's own exit code where it has one.
- The last line is the exact recovery command in its human form:
  `uncompose project import <job.json>`. Idempotent import makes it safe to
  rerun, so recovery is one copy-paste.

The pinned `--project` argv is what the CLI runs; the recovery line deliberately
prints the shorter human form (`--project` defaults to `.`), because the user is
expected to run it from the project root.

## Without `--project`

Behavior is byte-for-byte today's: no pre-flight, no registration, no manifest
read. Existing scripts and workflows are untouched (#89 story 8).
