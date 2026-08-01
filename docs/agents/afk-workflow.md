# AFK workflow: Sandcastle

[Sandcastle](https://github.com/mattpocock/sandcastle) drains `ready-for-agent` tickets
without a human at the keyboard (decided in the grilling session on #22, adopted in #26).
Wayfinder remains the planning flow; Sandcastle only executes fully specified tickets.

## Shape

- **Provider**: Docker. `.sandcastle/Dockerfile` documents the contributor toolchain
  (stable Rust + uv). Deliberately no torch: tests are engine-faked (see CONTRIBUTING.md),
  so the sandbox never needs a GPU stack.
- **Tracker**: GitHub Issues, filtered to the `ready-for-agent` label
  (`docs/agents/triage-labels.md`).
- **Template**: sequential-reviewer — one issue per cycle, implement then review, landing
  on a named `sandcastle/sequential-reviewer/*` branch. The agent does not close issues
  or merge; it hands the issue back as `ready-for-human` and a human merges the branch.
  This keeps PRs human-merged.

The root `package.json` exists only to host this workflow's dependencies; the shipped
product is the Cargo workspace plus `engine/`.

## Running it

One-time setup:

1. `npm install`
2. `cp .sandcastle/.env.example .sandcastle/.env` and fill in `CLAUDE_CODE_OAUTH_TOKEN`
   (from `claude setup-token`) and `GH_TOKEN`.
3. `npx @ai-hero/sandcastle docker build-image`

Then: `npm run sandcastle`. Logs land in `.sandcastle/logs/`.
