# Uncompose — Domain Context

Glossary and domain model. Terms land here as decisions resolve them (see `docs/agents/domain.md`); if a concept isn't here yet, it isn't pinned yet.

## Glossary

- **Stem** — one separated part of a recording (e.g. vocals, drums, bass, guitar, keys, other). Written as a 32-bit float WAV at the source sample rate, plainly named (`vocals.wav`, `drums.wav`, …).
- **Preset** — a named stem configuration a user picks when starting a job. v0.1 ships two: `6-stem` (the default: vocals, drums, bass, guitar, keys, other) and `2-stem` (vocals, instrumental). Not "model": a preset may be realized by one model or a multi-model pipeline.
- **Job** — one separation run: an input audio file plus a preset, executed locally, producing a job folder. The unit of history and reproducibility.
- **Job folder** — the per-job output directory holding the stems and the job record. One folder per job, named after the song.
- **Job record** — the file written alongside the stems capturing what produced them: model(s), version(s), parameters. A job is reproducible and diagnosable from its record alone.
- **Quick check** — the lightweight per-stem audition (play/solo) used to answer "good, or rerun?" after a job. Deliberately not a mixer; the user's DAW is the real audition surface.
- **Reference set** — the 3–5 songs from the primary user's covering repertoire, with Moises exports kept for comparison, used to judge quality.
- **Replacement bar** — the v0.1 quality criterion: for the reference set, the primary user would rather import the Uncompose `6-stem` output into their DAW than the Moises stems. Defined empirically, not by SDR.
- **Fast-follow splits** — separations deliberately outside the v0.1 bar, pursued as research allows: guitar lead/rhythm (top priority) and vocal lead/backing.

## Decisions

Recorded as ADRs in `docs/adr/` once the model-choice and architecture decisions land. Workflow-level decisions so far live on the wayfinder map ([#1](https://github.com/thedahm/uncompose/issues/1)), notably the v0.1 workflow and replacement bar ([#6](https://github.com/thedahm/uncompose/issues/6)).
