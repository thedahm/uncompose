# Uncompose

Open-source music source separation.

Uncompose separates recorded music into vocals, drums, bass, and instruments — locally,
openly, and without a subscription. It is a local-first tool for musicians who want to
isolate or reduce parts of a recording for practice, transcription, remixing, or study,
while keeping control over their audio, models, and workflow.

## Status

Pre-v0.1: the first release is being built in the open on the
[issue tracker](https://github.com/thedahm/uncompose/issues), with decisions recorded
in [`docs/adr/`](docs/adr/). The install lines below go live with the `v0.1.0` tag.

## Install

Uncompose ships on PyPI with [uv](https://docs.astral.sh/uv/) as the recommended
installer — uv is also what Uncompose itself uses at runtime, so have it installed
either way:

```sh
uvx uncompose separate song.mp3        # run without installing
uv tool install uncompose              # or install the `uncompose` command
```

The package is small; the heavy part comes on first use. Uncompose then builds its
engine environment (PyTorch and the separation stack, several GB, CUDA build by
default) into `~/.local/share/uncompose/` — announced with progress, one time per
version. On a machine without an NVIDIA GPU, make that first run route torch to the
CPU build:

```sh
UV_TORCH_BACKEND=cpu uncompose separate song.mp3
```

Later runs reuse the environment; the variable is only needed while it is first built.
Model weights (≈900 MB for the default preset) download automatically on first use,
hash-verified, with each model's license status shown.

## Quickstart

```sh
uncompose separate song.mp3
```

That produces `song.stems/` next to the input:

```
song.stems/
├── vocals.wav  drums.wav  bass.wav  guitar.wav  keys.wav  other.wav
├── engine.log
└── job.json
```

Every run prints a header first — input, preset, models with their license status,
device, output folder — so you know what is about to happen before the slow part
starts. Repeated runs on the same input never overwrite: you get `song.stems-2/`,
`song.stems-3/`, and so on. `job.json` records everything needed to understand or
rerun the job (input hash, models, device, timings); it is written last, so its
presence means the folder is complete.

Then:

```sh
uncompose play vocals      # audition a stem of the last job (mpv or ffplay)
uncompose open             # open the last job's folder
uncompose models list      # cached models, license status, hardware needs
uncompose models fetch 6-stem   # pre-download weights before a session
```

### Presets and devices

- `6-stem` (default): vocals, drums, bass, guitar, keys, other. A Mel-Band RoFormer
  vocal pass followed by htdemucs_6s for the rest.
- `2-stem` (`--preset 2-stem`): vocals and instrumental. GPU-required.

The `keys.wav` caveat: the v0.1 model behind it is piano-trained. Acoustic piano
lands in `keys.wav`; synths and organs usually land in `other.wav`. The stem keeps
the name `keys` so nothing changes when a broader keys model arrives.

Device is auto-detected: CUDA when an NVIDIA GPU is present, CPU otherwise
(`--device cpu|cuda` overrides). On the machine of record a song takes 1 to 5
minutes on GPU; CPU produces identical stems but takes tens of minutes — correct,
just slow.

## Extending

Any executable named `uncompose-<command>` on `PATH` becomes a subcommand
(`uncompose compare` runs `uncompose-compare`). See
[`docs/extensions.md`](docs/extensions.md) for the extension-author guide and a
minimal example extension.

## Requirements

Uncompose is Linux-only for v0.1: NVIDIA CUDA is the primary target with CPU as the
slow-but-correct fallback, and the machine of record is Ubuntu Studio 26.04 with an
RTX 4060 Ti. macOS and Windows are untested; reports welcome. It needs **ffmpeg** on
your `PATH` to read and write audio. Install it with your system package manager, for example `sudo apt install ffmpeg`
on Debian/Ubuntu. If ffmpeg is missing, Uncompose stops before a run with a one-line
install message rather than a cryptic error.

## Responsible use

Uncompose processes audio you provide, entirely on your own machine — nothing is uploaded
anywhere. You are responsible for making sure you have the rights to the audio you
separate, and the rights to what you do with the resulting stems follow from the rights
you hold in the input. Separating a recording does not grant you any rights to it.

The separation models Uncompose can download carry their own licenses, some of which
restrict commercial use; Uncompose surfaces each model's license status but it is up to
you to comply with it.

## License

[MIT](LICENSE) © Dominic Hanzely
