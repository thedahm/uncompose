# Uncompose Website

The Uncompose family's front door: one static page, no build step, no tracking.

This is the canonical agent-instructions file for `uncompose-website`, per Uncompose
family convention. `CLAUDE.md` imports this file rather than restating it.

## Agent skills

### Issue tracker

Issues live in GitHub Issues for `thedahm/uncompose-website` (via `gh`). See `docs/agents/issue-tracker.md`.

### Triage labels

Default label vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at repo root. See `docs/agents/domain.md`.

### AFK workflow

Sandcastle drains `ready-for-agent` tickets on named branches; humans merge. See `docs/agents/afk-workflow.md`.
