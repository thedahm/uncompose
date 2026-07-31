# Split Feasibility: Guitar Lead/Rhythm and Vocal Lead/Backing

Research for issue #13. Question: what exists in the open ecosystem for the two fast-follow splits set by the workflow decision (#6) — guitar lead vs rhythm (top priority) and vocals lead vs backing — and how close can a multi-pass pipeline get? Compiled 2026-07-31 from primary sources (model catalogs, repos, Hugging Face metadata).

## Part 1 — Guitar lead vs rhythm

### What exists: nothing open

- **No open checkpoint splits lead from rhythm guitar.** Sweeping the places community checkpoints actually live turns up zero candidates:
  - **python-audio-separator model registry** (https://raw.githubusercontent.com/nomadkaraoke/python-audio-separator/main/audio_separator/models.json): 60+ RoFormer-class models across vocals/instrumental/karaoke/de-reverb/denoise categories; **no guitar models at all**.
  - **MSST pretrained model list** (https://github.com/ZFTurbo/Music-Source-Separation-Training/blob/main/docs/pretrained_models.md): no dedicated guitar model; guitar appears only as one stem of HTDemucs4 6-stem.
  - **MVSEP catalog** (https://mvsep.com/en/algorithms): "MVSep Guitar" (algorithm #17) and "BS Roformer SW" 6-stem (algorithm #77, 9.05 guitar SDR) both do **guitar-vs-everything-else**, not lead-vs-rhythm. These are also service-side models; SW weights are not published for local use.
  - The question is a known open problem on the Demucs tracker — "Separation of rhythm guitar and lead guitar" (https://github.com/facebookresearch/demucs/issues/588) — with "train your own" as the only answer.
- Moises' lead/rhythm guitar option is proprietary (Music AI models, cloud-only), and per the prior-art research (#4) nothing in the open ecosystem reimplements it.

### The nearest useful step: guitar-vs-rest

- **becruily's Mel-Band RoFormer Guitar** (https://huggingface.co/becruily/mel-band-roformer-guitar): `becruily_guitar.ckpt` + MSST-style `config_guitar_becruily.yaml`, published May 2025, **no license declared** (download-from-original-host only, per the #3 weights policy). Runs via MSST or via audio-separator's custom-model path (ckpt+yaml); not in the audio-separator registry.
- Pipeline sketch for "two guitars": 6-stem separation → guitar stem → nothing further automated; lead vs rhythm recovery stays manual (DAW, ear, EQ/pan). A second-pass on the guitar stem with today's models cannot distinguish the two parts because no training data/checkpoint targets that distinction.

### Verdict: effectively unavailable

Not a fast-follow; a train-your-own-model project, which the kickoff brief rules out. Revisit if the field moves (watch MVSEP quality-checker and MSST issue #1 where new community checkpoints land). v0.1+ should ship guitar-vs-rest quality improvements instead (becruily guitar as an optional second pass over the 6-stem guitar stem).

## Part 2 — Vocals lead vs backing (karaoke models)

### What exists: mature, multiple checkpoints, integrated

- **audio-separator registry carries four karaoke RoFormer checkpoints** (https://raw.githubusercontent.com/nomadkaraoke/python-audio-separator/main/audio_separator/models.json):
  - `mel_band_roformer_karaoke_aufr33_viperx_sdr_10.1956.ckpt` — the community reference (SDR 10.20), by aufr33 + viperx of the UVR team; originally an X-Minus/UVR Patreon early-access release, later public (https://www.patreon.com/posts/mel-roformer-104352762).
  - `mel_band_roformer_karaoke_gabox.ckpt` and `..._v2.ckpt` (Gabox).
  - `mel_band_roformer_karaoke_becruily.ckpt` (becruily).
- **MVSEP offers the same capability as a service** ("MVSep Karaoke", algorithm #76: best model 10.41 lead / 6.61 back SDR, https://mvsep.com/algorithms/76), which is useful as the quality benchmark to compare local output against.
- Two-stage is the established pattern: extract vocals first with a top vocal model, then run the karaoke model on the vocal stem for lead vs backing ("Extract vocals first" on MVSEP; same flow locally).

### Licensing and cost

- Like nearly all community checkpoints (#3), the karaoke ckpts carry **no explicit license** — auto-download from their original hosts at runtime, never bundle.
- Runtime on the reference RTX 4060 Ti 16GB: Mel-RoFormer inference is the same architecture class as the main vocal models — roughly 1–2 min for a typical song per pass, and the karaoke second pass runs on an already-isolated vocal stem. Comfortably inside even the 1–5 min standard tier as an optional pass, and trivially inside the 10–20 min max-quality tier.

### Verdict: viable fast-follow now

Integration path is the one Uncompose already plans to use (python-audio-separator; the aufr33/viperx checkpoint works out of the registry with zero custom config). The only work is pipeline plumbing (vocals pass → karaoke pass → lead/backing stems in the job folder) and preset naming.

## Part 3 — Implications for the map

1. **The two fast-follows are asymmetric.** Vocal lead/backing is plumbing; guitar lead/rhythm is research-frontier. The 10–20 min "max quality / max split" tier from #6 can realistically ship 7 stems (lead vocals, backing vocals, drums, bass, guitar, keys, other) — not the full Moises-parity 8.
2. **Moises parity on guitar split is not reachable locally in 2026.** Worth stating honestly in positioning/roadmap docs: the one Moises capability Uncompose cannot replace yet.
3. **Watchlist for the field moving:** MSST community-models issue (https://github.com/ZFTurbo/Music-Source-Separation-Training/issues/1), MVSEP quality checker (https://mvsep.com/quality_checker/), audio-separator release notes.
