# AFK workflow: Sandcastle

[Sandcastle](https://github.com/mattpocock/sandcastle) drains `ready-for-agent` tickets
without a human at the keyboard, as it does across the Uncompose family (decided in
uncompose#22, adopted in uncompose#26). Wayfinder remains the planning flow; Sandcastle
only executes fully specified tickets.

## Shape

- **Provider**: Docker. The loop machinery (`main.mts`, prompts, `Dockerfile`) is not
  tracked here — it is synced in locally by `sandcastle-kit`'s `bin/sync`. Only
  `.sandcastle/CODING_STANDARDS.md` and `.sandcastle/sandcastle.config.json` are
  committed, so a fork can supply its own loop.
- **Toolchain**: the sandbox needs nothing but Python 3 for
  `tests/check_site.py`. There is no site toolchain to install (ADR-0001).
- **Tracker**: GitHub Issues, filtered to the `ready-for-agent` label
  (`docs/agents/triage-labels.md`).
- **Template**: sequential-reviewer — one issue per cycle, implement then review, landing
  on a named branch that the host pushes to origin. The agent does not open PRs, merge,
  or close issues; it hands the issue back as `ready-for-human`, and a human PRs the
  pushed branch and merges. Blocked issues are dequeued to `needs-info` rather than
  stalling the run.

## Running it

Follow `sandcastle-kit`'s setup: sync the machinery into `.sandcastle/`, provide
`CLAUDE_CODE_OAUTH_TOKEN` (from `claude setup-token`) and `GH_TOKEN` in
`.sandcastle/.env`, build the image, then run the loop. Logs land in `.sandcastle/logs/`.
