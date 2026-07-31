#!/usr/bin/env python3
"""PROTOTYPE — throwaway mock of the Uncompose v0.1 CLI (wayfinder #17).

Separates nothing. Prints the transcripts from TRANSCRIPTS.md with live
progress so the run *feels* like a 1-5 minute GPU job. Stateless: use
--cold to see the first-ever run (engine env + weight download).

Timing is 12x faster than "real"; pass --realtime for true pacing.
"""

import argparse
import os
import sys
import time

SPEED = 12.0  # divide all durations; --realtime sets to 1

BOLD, DIM, GREEN, RED, RESET = "\033[1m", "\033[2m", "\033[32m", "\033[31m", "\033[0m"
if not sys.stdout.isatty() or os.environ.get("NO_COLOR"):
    BOLD = DIM = GREEN = RED = RESET = ""

PRESETS = {
    "6-stem": {
        "stems": ["vocals", "drums", "bass", "guitar", "piano", "other"],
        "model": "htdemucs_6s v4",
        "license": "research-only license",
        "size_mb": 319,
        "gpu_required": False,
        "sep_secs": 136,
    },
    "2-stem": {
        "stems": ["vocals", "instrumental"],
        "model": "melband-roformer-kim",
        "license": "MIT",
        "size_mb": 214,
        "gpu_required": True,
        "sep_secs": 101,
    },
}


def nap(secs):
    time.sleep(secs / SPEED)


def fmt_mmss(secs):
    return f"{int(secs) // 60}:{int(secs) % 60:02d}"


def bar(frac, width=20):
    filled = int(frac * width)
    return "[" + "█" * filled + "·" * (width - filled) + "]"


def stage_progress(label, total_secs, render, final):
    """Animate one stage line in place, then collapse it to its final form."""
    t = 0.0
    step = total_secs / 60
    while t < total_secs:
        sys.stdout.write("\r\033[K  " + label.ljust(9) + render(t / total_secs, t))
        sys.stdout.flush()
        nap(step)
        t += step
    sys.stdout.write("\r\033[K  " + label.ljust(9) + final + "\n")
    sys.stdout.flush()


def download(model, license_, size_mb, secs, indent="  "):
    def render(frac, t):
        rate = size_mb / secs
        return f"{model.split()[0]}  {bar(frac)}  {int(frac * size_mb)} / {size_mb} MB   {rate:.1f} MB/s"

    stage_progress(
        "weights",
        secs,
        render,
        f"{model.split()[0]}  {bar(1.0)}  {size_mb} MB   done  {fmt_mmss(secs)}",
    )


def header(inp, preset_name, p, device_line, out):
    print()
    print(f"  {DIM}input{RESET}    {inp}  (3:41, 44.1 kHz stereo MP3)")
    print(f"  {DIM}preset{RESET}   {preset_name}  ({', '.join(p['stems'])})")
    print(f"  {DIM}model{RESET}    {p['model']}  — weights: {p['license']}")
    print(f"  {DIM}device{RESET}   {device_line}")
    print(f"  {DIM}output{RESET}   {out}")
    print()


def cmd_run(args):
    p = PRESETS[args.preset]
    stem_base = os.path.splitext(os.path.basename(args.input))[0]
    out = f"{stem_base}.stems/"

    if args.device == "cpu":
        if p["gpu_required"]:
            print(f"{RED}error:{RESET} preset {args.preset} ({p['model'].split()[0]}) requires a CUDA GPU, none found")
            print(f"\n  the 6-stem preset can run on CPU:  uncompose run {args.input!r}")
            sys.exit(1)
        device_line = "cpu   [no CUDA device found — expect roughly 15–30 min, not 1–5]"
    else:
        device_line = "cuda  (NVIDIA GeForce RTX 4060 Ti, 16 GB)"
        if p["gpu_required"]:
            device_line += "  [GPU required for this preset]"

    header(args.input, args.preset, p, device_line, out)

    if args.cold:
        stage_progress(
            "engine",
            112,
            lambda frac, t: "setting up Python engine (first run only) " + "." * int(frac * 10),
            f"setting up Python engine (first run only) .......... done  {fmt_mmss(112)}",
        )
        download(p["model"], p["license"], p["size_mb"], 26)

    sep = p["sep_secs"]
    stage_progress(
        "separate",
        sep,
        lambda frac, t: f"{bar(frac)}  {int(frac * 100):2d}%   {fmt_mmss(t)} elapsed  ~{fmt_mmss(sep - t)} left",
        f"{bar(1.0)}  100%  {fmt_mmss(sep)}",
    )
    stage_progress("write", 4, lambda frac, t: "  ".join(p["stems"][: max(1, int(frac * len(p["stems"])))]), "  ".join(p["stems"]))

    if args.fail_at is not None:
        pass  # handled by cmd_fail instead
    print(f"\n{GREEN}✓{RESET} {BOLD}{out}{RESET}  ({len(p['stems'])} stems, 32-bit float WAV, {fmt_mmss(sep)} separation)")
    print(f"\n  {DIM}play a stem:    uncompose play {p['stems'][0]}{RESET}")
    print(f"  {DIM}open folder:    uncompose open{RESET}\n")


def cmd_fail(args):
    p = PRESETS["6-stem"]
    header("song.mp3", "6-stem", p, "cuda  (NVIDIA GeForce RTX 4060 Ti, 16 GB)", "song.stems/")
    t, target = 0.0, 0.36 * p["sep_secs"]
    step = p["sep_secs"] / 60
    while t < target:
        frac = t / p["sep_secs"]
        sys.stdout.write(f"\r\033[K  separate {bar(frac)}  {int(frac * 100):2d}%   {fmt_mmss(t)} elapsed")
        sys.stdout.flush()
        nap(step)
        t += step
    print(f"\r\033[K  separate {bar(0.36)}  36%")
    print(f"{RED}✗{RESET} engine failed during separate  (exit 137 — likely out of GPU memory)")
    print("\n  partial output kept for diagnosis:  song.stems.partial/")
    print("  engine log:                         song.stems.partial/engine.log")
    print("  nothing named song.stems/ was created — a visible .stems folder")
    print("  always means a complete job.\n")
    sys.exit(1)


def cmd_play(args):
    if args.stem is None:
        job, stem = "Take Me Home.stems-2", args.job_or_stem
    else:
        job, stem = args.job_or_stem.rstrip("/"), args.stem
    print(f"▶ {job}/{stem}.wav  (mpv, q to quit)")


def cmd_open(args):
    print("opened Take Me Home.stems-2/  (xdg-open)")


def cmd_models(args):
    if args.action == "list":
        print()
        print(f"  {DIM}MODEL                  PRESET   SIZE     WEIGHTS LICENSE   CACHED{RESET}")
        print("  htdemucs_6s v4         6-stem   319 MB   research-only     ✓")
        print("  melband-roformer-kim   2-stem   214 MB   MIT               —")
        print(f"\n  {DIM}cache: ~/.cache/uncompose/models  (319 MB used){RESET}\n")
    elif args.action == "fetch":
        p = PRESETS.get(args.target, PRESETS["2-stem"])
        print(f"  {p['model']}  — weights: {p['license']}")
        download(p["model"], p["license"], p["size_mb"], 18)
        print(f"  verified sha256 {GREEN}✓{RESET} — cached in ~/.cache/uncompose/models")
    elif args.action == "remove":
        print(f"removed {args.target}  (freed 319 MB; it will re-download on next 6-stem run)")


def main():
    global SPEED
    ap = argparse.ArgumentParser(prog="uncompose-mock", description=__doc__)
    ap.add_argument("--realtime", action="store_true", help="true pacing (minutes)")
    sub = ap.add_subparsers(dest="verb", required=True)

    run = sub.add_parser("run")
    run.add_argument("--realtime", action="store_true", help="true pacing (minutes)")
    run.add_argument("input")
    run.add_argument("--preset", choices=PRESETS, default="6-stem")
    run.add_argument("--device", choices=["cuda", "cpu"], default="cuda")
    run.add_argument("--cold", action="store_true", help="simulate first-ever run")
    run.set_defaults(func=cmd_run, fail_at=None)

    sub.add_parser("fail").set_defaults(func=cmd_fail)

    play = sub.add_parser("play")
    play.add_argument("job_or_stem")
    play.add_argument("stem", nargs="?")
    play.set_defaults(func=cmd_play)

    sub.add_parser("open").set_defaults(func=cmd_open)

    models = sub.add_parser("models")
    models.add_argument("action", choices=["list", "fetch", "remove"])
    models.add_argument("target", nargs="?")
    models.set_defaults(func=cmd_models)

    args = ap.parse_args()
    if args.realtime:
        SPEED = 1.0
    try:
        args.func(args)
    except KeyboardInterrupt:
        print(f"\n{RED}✗{RESET} cancelled — removed staging folder, no output written")
        sys.exit(130)


if __name__ == "__main__":
    main()
