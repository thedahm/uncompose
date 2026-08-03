# External-command dispatch contract

The Uncompose ecosystem grows by sibling tools (`uncompose-project`, `uncompose-compare`)
rather than by expanding this repository, and the kickoff brief
([#55](https://github.com/thedahm/uncompose/issues/55)) fixes the `uncompose-<command>`
dispatch convention as a Given. This ADR decides its contract: how the root `uncompose`
finds and runs extensions, and what an extension must promise in return. This is contract
**revision 1**. Decided in [#57](https://github.com/thedahm/uncompose/issues/57).

## Dispatch: git-style open namespace

When the first non-flag argument is not a builtin, the root looks up `uncompose-<command>`
on `PATH` and runs it. The namespace is open — any `uncompose-foo` on `PATH` is
dispatchable, with no allowlist to maintain in this repo when a new tool appears, first- or
third-party. An allowlist was rejected: it would make every new ecosystem tool a root
release.

Only the first token dispatches. `uncompose project init song.wav` runs `uncompose-project`
with arguments `init song.wav`; the extension owns its entire subcommand tree.

## Naming: `^[a-z0-9][a-z0-9-]*$`

A token is eligible for dispatch only if it matches `^[a-z0-9][a-z0-9-]*$`. Anything else —
uppercase, dots, slashes, a leading dash — is an ordinary unknown-command error with no
`PATH` lookup, which blocks `uncompose ../evil` path tricks and keeps command names
URL- and package-name-friendly. This is stricter than git, which will attempt nearly any
token as `git-<token>`; strictness here is deliberate.

## Forwarding: verbatim argv, then `exec()`

Everything after the command token is forwarded untouched. The root never parses, reorders,
or consumes flags that follow the token — `uncompose compare --blind a.wav b.wav` delivers
`--blind a.wav b.wav` exactly. On Linux (v0.1's only platform, per ADR-0004) the root
`exec()`s the extension, replacing its own process, so stdin/stdout/stderr, TTY-ness,
signals, and the exit code all belong to the extension natively, with no relay code to get
wrong.

The root defines no global flags that are forwarded to extensions. `uncompose --verbose
project …` is the root's business only. If extensions ever need context from the root, it
will arrive as `UNCOMPOSE_*` environment variables in a later contract revision, never as
injected flags.

## Exit codes: the launcher convention

The extension's exit code is the user-visible exit code (free with `exec()`). When dispatch
itself fails, the root uses the POSIX wrapper-utility convention (`env`, `nice`, `nohup`,
`xargs`) rather than git's flat 1, because at that moment the root *is* a launcher:

- **127** — no `uncompose-<command>` on `PATH`.
- **126** — found but not executable (or `exec` otherwise failed).

The 127 message names the mechanism so the fix is self-evident:

```text
uncompose: 'foo' is not an uncompose command (no 'uncompose-foo' found on PATH)
```

For known family names (`project`, `compare`) it appends an install hint
(`install it with: uv tool install uncompose-project`). Other names get a did-you-mean
against builtins and whatever `uncompose-*` executables are actually present.

## What an extension must provide

- **`--version`**: one line to stdout, exit 0, shaped `uncompose-<command> X.Y.Z` — the
  same shape as the root's `uncompose X.Y.Z`.
- **`--help`**: human-readable usage to stdout, exit 0.

Both are reached through normal forwarding (`uncompose compare --version`); the root adds
no special handling.

## Help discovery

`uncompose --help` lists builtins, then an "External commands (installed):" section built
by scanning `PATH` for `uncompose-*` executables — names only, found by directory listing,
never executed. Per-command one-line descriptions are not attempted in revision 1; that
would mean executing binaries or a metadata sidecar, and the reserved capability probe
below is the future hook for it.

## Reserved: `--uncompose-info`

An extension **may** support `--uncompose-info`, printing a single JSON object to stdout:

```json
{"name": "compare", "version": "0.1.0", "contract": 1, "description": "…"}
```

`contract` is the dispatch-contract revision the extension targets. Nothing consumes this
in revision 1 — the flag is reserved now so no extension can ever claim it for other
semantics, and so richer help, aggregate version output, and compatibility warnings have a
defined probe to build on. Extensions that don't implement it fail with their normal
unknown-flag error, which a caller can detect.

## Compatibility

The contract is the versioned thing: an integer revision, changed additively where
possible, bumped only by breaking changes. Root and extensions version and release
independently, and the root dispatches whatever `uncompose-<command>` it finds
unconditionally — no version handshake, no refusing "too old" extensions in revision 1.
The `contract` field of `--uncompose-info` is the hook for smarter warnings later.

Data-format compatibility (job.json, the project manifest, comparison records) is out of
this ADR's scope; those schemas carry their own versioning decisions.
