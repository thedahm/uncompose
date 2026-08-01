# Uncompose

Open-source music source separation.

Uncompose separates recorded music into vocals, drums, bass, and instruments — locally,
openly, and without a subscription. It is a local-first tool for musicians who want to
isolate or reduce parts of a recording for practice, transcription, remixing, or study,
while keeping control over their audio, models, and workflow.

## Status

Pre-v0.1: the project is in its planning phase and there is nothing to install yet.
The plan for the first release is being worked out in the open on the
[issue tracker](https://github.com/thedahm/uncompose/issues), with decisions recorded
in [`docs/adr/`](docs/adr/).

## Requirements

Uncompose is Linux-only for v0.1 and needs **ffmpeg** on your `PATH` to read and write
audio. Install it with your system package manager, for example `sudo apt install ffmpeg`
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
