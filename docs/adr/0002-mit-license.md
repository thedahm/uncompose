# License: MIT

Uncompose needs a license before any public work lands, and the choice is effectively
irreversible: once outside contributions exist, relicensing would require every
contributor's consent, and no CLA is planned that could grant it. We decided: **MIT**,
copyright Dominic Hanzely.

The weights research ([#3](https://github.com/thedahm/uncompose/issues/3)) established
that model weights are runtime-downloaded data, never bundled, so no weight license
constrains the choice of code license. That freed the decision to be made on merits:

- The surrounding ecosystem — demucs, spleeter, audio-separator, Open-Unmix — is MIT.
  Matching it removes every friction for embedding, wrapping, and packaging Uncompose.
- MIT matches the project identity: "Moises is a service musicians use. Uncompose is a
  tool they own." A maximally permissive license is the licensing expression of that.

## Considered options

- **AGPL-3.0** — would prevent a third party from offering Uncompose as a closed hosted
  service without sharing their changes. That scenario was considered and consciously
  accepted: the project's value proposition is the local-first workflow, a hosted wrapper
  doesn't threaten it, and copyleft would cut against the ecosystem norm and add adoption
  friction for the developers and packagers Uncompose wants as contributors.
- **Apache-2.0** — the explicit patent grant is the only material difference from MIT.
  Not worth diverging from the ecosystem's MIT norm for a project with no patent exposure.

## Consequences

- Anyone may build closed or commercial products on Uncompose, including hosted services.
  Accepted, per the above.
- The license is locked in practice from the first outside contribution (no CLA, so no
  relicensing path). This ADR records that this was understood at decision time.
