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
