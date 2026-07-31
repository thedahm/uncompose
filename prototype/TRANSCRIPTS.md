# Uncompose v0.1 CLI — mocked transcripts (PROTOTYPE)

Throwaway artifact for [#17](https://github.com/thedahm/uncompose/issues/17).
Every size and timing is invented. The command surface mocks the decisions in
#6/#7/#8/#10/#11. Open questions for reaction are at the bottom.

---

## 1. First-ever run (cold: engine env + weights)

```
$ uncompose run "Take Me Home.mp3"

  input    Take Me Home.mp3  (3:41, 44.1 kHz stereo MP3)
  preset   6-stem  (vocals, drums, bass, guitar, piano, other)
  model    htdemucs_6s v4  — weights: research-only license
  device   cuda  (NVIDIA GeForce RTX 4060 Ti, 16 GB)
  output   Take Me Home.stems/

  engine    setting up Python engine (first run only) .......... done  1:52
  weights   htdemucs_6s  [██████████████······]  241 / 319 MB   12.4 MB/s
  separate  [████████████████····]  81%   1:48 elapsed  ~0:25 left
  write     vocals  drums  bass  guitar  piano  other

✓ Take Me Home.stems/  (6 stems, 32-bit float WAV, 2:19 separation)

  play a stem:    uncompose play vocals
  open folder:    uncompose open
```

Notes: the header block prints immediately, before any work, so a typo'd
preset or wrong file dies fast. Each stage line updates in place; finished
stages collapse to one line with their elapsed time. The weights line carries
the license relay (`research-only` / `MIT`) — that's where #7 said it must
surface.

## 2. Everyday warm run (the loop this CLI is optimized for)

```
$ uncompose run "Take Me Home.mp3"

  input    Take Me Home.mp3  (3:41, 44.1 kHz stereo MP3)
  preset   6-stem  (vocals, drums, bass, guitar, piano, other)
  model    htdemucs_6s v4  — weights: research-only license
  device   cuda  (NVIDIA GeForce RTX 4060 Ti, 16 GB)
  output   Take Me Home.stems-2/   (Take Me Home.stems/ exists, never overwritten)

  separate  [████████████████████]  100%  2:16
  write     vocals  drums  bass  guitar  piano  other

✓ Take Me Home.stems-2/  (6 stems, 32-bit float WAV, 2:16 separation)
```

Note the collision handling on the output line: the existing job folder is
left alone, the new job gets `-2`, and the header says so before the run
starts (cheap moment to Ctrl+C if a rerun wasn't intended).

## 3. 2-stem run

```
$ uncompose run song.wav --preset 2-stem

  input    song.wav  (4:07, 48 kHz stereo WAV)
  preset   2-stem  (vocals, instrumental)
  model    melband-roformer-kim  — weights: MIT
  device   cuda  (NVIDIA GeForce RTX 4060 Ti, 16 GB)  [GPU required for this preset]
  output   song.stems/

  weights   melband-roformer-kim  [████████████████████]  214 MB   done  0:19
  separate  [████████████████████]  100%  1:41
  write     vocals  instrumental

✓ song.stems/  (2 stems, 32-bit float WAV, 1:41 separation)
```

First use of the preset auto-fetches its weights inline (per #7), same as the
cold run — `models fetch` exists only as a convenience to front-load this.

## 4. play — the lightweight per-stem check

```
$ uncompose play vocals
▶ Take Me Home.stems-2/vocals.wav  (mpv, q to quit)
```

Bare form uses the last-job pointer (#10). Explicit-job form:

```
$ uncompose play "Take Me Home.stems" drums
▶ Take Me Home.stems/drums.wav  (mpv, q to quit)
```

No player installed — fall back to the folder, per #8:

```
$ uncompose play vocals
error: no audio player found (looked for: mpv, ffplay)

  the stems are plain WAV files in:  /home/dom/music/Take Me Home.stems-2/
  open the folder:                   uncompose open
  or install a player:               sudo apt install mpv
```

```
$ uncompose open
opened Take Me Home.stems-2/  (xdg-open)
```

## 5. models — list / fetch / remove

```
$ uncompose models list

  MODEL                  PRESET   SIZE     WEIGHTS LICENSE   CACHED
  htdemucs_6s v4         6-stem   319 MB   research-only     ✓
  melband-roformer-kim   2-stem   214 MB   MIT               —

  cache: ~/.cache/uncompose/models  (319 MB used)
```

```
$ uncompose models fetch 2-stem
  melband-roformer-kim  — weights: MIT
  [████████████████████]  214 / 214 MB   11.8 MB/s   done 0:18
  verified sha256 ✓ — cached in ~/.cache/uncompose/models
```

```
$ uncompose models remove htdemucs_6s
removed htdemucs_6s  (freed 319 MB; it will re-download on next 6-stem run)
```

`fetch` accepts a preset name or a model id; both are listed in `models list`.

## 6. Failure paths

ffmpeg missing (preflight, before any work):

```
$ uncompose run song.mp3
error: ffmpeg not found — Uncompose needs it to decode/encode audio

  install it:  sudo apt install ffmpeg
```

No CUDA, 6-stem (CPU fallback exists but is slow — warn, proceed):

```
$ uncompose run song.mp3

  input    song.mp3  (3:41, 44.1 kHz stereo MP3)
  preset   6-stem  (vocals, drums, bass, guitar, piano, other)
  model    htdemucs_6s v4  — weights: research-only license
  device   cpu   [no CUDA device found — expect roughly 15–30 min, not 1–5]
  ...
```

No CUDA, 2-stem (GPU required — refuse):

```
$ uncompose run song.mp3 --preset 2-stem
error: preset 2-stem (melband-roformer-kim) requires a CUDA GPU, none found

  the 6-stem preset can run on CPU:  uncompose run song.mp3
```

Engine crash mid-run (staging folder kept as the diagnosable artifact, #10):

```
$ uncompose run song.mp3
  ...
  separate  [███████·············]  36%
✗ engine failed during separate  (exit 137 — likely out of GPU memory)

  partial output kept for diagnosis:  song.stems.partial/
  engine log:                         song.stems.partial/engine.log
  nothing named song.stems/ was created — a visible .stems folder always
  means a complete job.
```

Ctrl+C (staging deleted, nothing left behind):

```
$ uncompose run song.mp3
  separate  [█████···············]  22%  ^C
✗ cancelled — removed staging folder, no output written
```

---

## Open questions (react to any or all)

1. **Run verb name** — `uncompose run song.mp3` as mocked, `uncompose
   separate song.mp3`, or bare `uncompose song.mp3`? Bare reads best for the
   one-song loop but fights with subcommands (`play`, `models`) in clap and
   in docs.
2. **Header block** — the five-line input/preset/model/device/output echo
   before every run: right amount, or chatty for the everyday warm run?
3. **Progress style** — one updating line per stage (mocked) vs a single
   overall percent bar vs near-silent with `--verbose` for stages. The
   runnable mock plays the per-stage style; react after feeling it.
4. **Stem name** — `piano.wav` (model-native for htdemucs_6s) vs `keys.wav`
   (#6 called the stem "keys"). Whatever lands here also lands in `play
   piano` vs `play keys`.
5. **License relay placement** — mocked on the model line of every run
   header plus `models list`. Enough, too loud, or should it be
   download-time only?
6. **`open` verb** — kept as mocked (`uncompose open` = xdg-open the last
   job folder), or is printing the path enough?
7. **Post-run hint lines** (`play a stem: …`, `open folder: …`) — keep
   always, first-runs only, or drop?
8. **Staging folder name** — `song.stems.partial/` as mocked, or hidden
   (`.song.stems.tmp/`)? Visible-but-clearly-partial vs invisible-until-done.
