# PROTOTYPE — throwaway, do not build on this

Mock of the Uncompose v0.1 CLI ergonomics for wayfinder ticket
[#17](https://github.com/thedahm/uncompose/issues/17). Nothing here separates
audio; every number, size, and timing is invented (plausible, not measured).

Two artifacts:

- **`TRANSCRIPTS.md`** — static mocked terminal transcripts of every verb,
  including error paths. Read this first; react to it async.
- **`uncompose_mock.py`** — a runnable fake CLI (stdlib Python, no deps) that
  plays the same transcripts with live progress bars, so the 1–5 minute run
  *feels* like something. Timing is sped up 12× by default.

## Run it

```
python3 prototype/uncompose_mock.py run "Take Me Home.mp3"            # warm everyday run
python3 prototype/uncompose_mock.py run "Take Me Home.mp3" --cold     # first-ever run (env + weights)
python3 prototype/uncompose_mock.py run song.wav --preset 2-stem
python3 prototype/uncompose_mock.py run song.wav --device cpu
python3 prototype/uncompose_mock.py play vocals
python3 prototype/uncompose_mock.py models list
python3 prototype/uncompose_mock.py models fetch 2-stem
python3 prototype/uncompose_mock.py models remove htdemucs_6s
python3 prototype/uncompose_mock.py fail                              # mid-run engine crash
```

Add `--realtime` to any `run` to feel the true pacing (about 3 minutes).
