# Writing an uncompose extension

Uncompose grows by sibling tools rather than by expanding this repository. Any
executable named `uncompose-<command>` on `PATH` becomes a subcommand: installing
`uncompose-compare` makes `uncompose compare` work, with no registration and no
change to the root.

The contract of record is [ADR-0005](adr/0005-external-command-dispatch-contract.md)
(dispatch-contract **revision 1**). This page restates it for extension authors;
where the two disagree, the ADR wins.

## How dispatch works

- **Naming.** Your executable is `uncompose-<command>`, where `<command>` matches
  `^[a-z0-9][a-z0-9-]*$` — lowercase letters, digits, and dashes, no leading dash.
  Tokens outside that pattern are never looked up on `PATH`.
- **Only the first token dispatches.** `uncompose project init song.wav` runs
  `uncompose-project` with arguments `init song.wav`. You own your entire
  subcommand tree.
- **Verbatim forwarding.** Everything after the command token reaches you
  untouched. The root never parses, reorders, or consumes flags that follow the
  token: `uncompose compare --blind a.wav b.wav` delivers `--blind a.wav b.wav`
  exactly. The root also forwards none of its own flags; if extensions ever need
  context from the root, it will arrive as `UNCOMPOSE_*` environment variables in
  a later contract revision.
- **Exit codes.** The root `exec()`s your tool, so your process replaces it:
  stdin/stdout/stderr, TTY-ness, signals, and your exit code are all natively
  yours. When dispatch itself fails, the root exits with the launcher
  convention: **127** when no `uncompose-<command>` exists on `PATH`, **126**
  when one exists but is not executable.
- **Builtins win.** A builtin name (`separate`, `play`, ...) never dispatches; an
  extension cannot shadow it.

## What your extension must provide

- **`--version`** — one line to stdout, exit 0, shaped `uncompose-<command> X.Y.Z`
  (the same shape as the root's `uncompose X.Y.Z`).
- **`--help`** — human-readable usage to stdout, exit 0.

Both are reached through normal forwarding (`uncompose compare --version`); the
root adds no special handling.

## Reserved: `--uncompose-info`

The flag `--uncompose-info` is reserved by the contract. Nothing probes it in
revision 1, but do not claim it for other semantics. An extension *may*
implement it, printing a single JSON object to stdout:

```json
{"name": "example", "version": "0.1.0", "contract": 1, "description": "…"}
```

`contract` is the dispatch-contract revision the extension targets (currently 1).

## Try it: the example extension

A minimal conforming extension ships with the docs at
[`examples/uncompose-example`](examples/uncompose-example). To verify dispatch on
your machine, install it somewhere on `PATH`:

```sh
install -m 755 docs/examples/uncompose-example ~/.local/bin/uncompose-example
```

Then:

```sh
uncompose example hello --flag x   # echoes the forwarded arguments
uncompose example --version       # → uncompose-example 0.1.0
uncompose example --help          # usage text, exit 0
uncompose example exit 42         # exits 42, proving exit codes pass through
```

Copy the script as the starting point for your own tool: rename it to
`uncompose-<yourcommand>`, keep the `--version`/`--help` behavior, and replace
the argument echo with your real logic.
