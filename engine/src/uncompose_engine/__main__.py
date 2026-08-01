"""Engine Contract entry point: `python -m uncompose_engine`.

Reads one JSON request from stdin, runs the model via audio-separator, and
emits JSONL events on stdout. Everything else (library logging, tqdm bars,
stray prints) is forced onto stderr so the event stream stays parseable.
"""

import importlib.metadata
import json
import logging
import os
import sys
import time

# Models known to this engine: audio-separator filename plus the mapping
# from audio-separator's stem names to Uncompose's preset-level stem names
# (keys.wav, not piano.wav — the model is piano-trained, the name is stable).
MODELS = {
    "htdemucs_6s": {
        "filename": "htdemucs_6s.yaml",
        "output_names": {
            "Vocals": "vocals",
            "Drums": "drums",
            "Bass": "bass",
            "Guitar": "guitar",
            "Piano": "keys",
            "Other": "other",
        },
    },
}


def main() -> int:
    # Keep a private handle to the real stdout for events, then point fd 1
    # at stderr so nothing the ML stack prints can corrupt the JSONL stream.
    events = os.fdopen(os.dup(1), "w", buffering=1)
    os.dup2(2, 1)
    sys.stdout = sys.stderr

    def emit(event: dict) -> None:
        events.write(json.dumps(event) + "\n")

    try:
        request = json.load(sys.stdin)
        return run(request, emit)
    except Exception as exc:  # terminal: report on the contract, exit nonzero
        logging.exception("engine failed")
        emit({"event": "error", "message": str(exc)})
        return 1


def run(request: dict, emit) -> int:
    model = MODELS.get(request["model_id"])
    if model is None:
        raise ValueError(f"unknown model id: {request['model_id']}")

    if request.get("device") == "cpu":
        os.environ["CUDA_VISIBLE_DEVICES"] = ""

    logging.basicConfig(stream=sys.stderr, level=logging.INFO)

    emit({"event": "stage", "stage": "model_load", "message": request["model_id"]})
    t0 = time.monotonic()
    from audio_separator.separator import Separator

    separator = Separator(
        output_dir=request["output_dir"],
        model_file_dir=request["model_dir"],
        output_format="wav",
    )
    separator.load_model(model_filename=model["filename"])
    model_load_secs = time.monotonic() - t0

    emit({"event": "stage", "stage": "separate"})
    t1 = time.monotonic()
    separator.separate(request["audio_path"], custom_output_names=model["output_names"])
    separate_secs = time.monotonic() - t1

    # audio-separator can swallow decode failures and return with nothing
    # written; missing stems are a failed job, not a quiet success.
    missing = []
    for stem_name in model["output_names"].values():
        path = os.path.join(request["output_dir"], f"{stem_name}.wav")
        if os.path.exists(path):
            emit({"event": "stem", "name": stem_name, "path": path})
        else:
            missing.append(stem_name)
    if missing:
        raise RuntimeError(f"separation produced no output for: {', '.join(missing)}")

    import torch

    emit(
        {
            "event": "done",
            "model_id": request["model_id"],
            "engine_version": importlib.metadata.version("uncompose-engine"),
            "device": "cuda" if torch.cuda.is_available() else "cpu",
            "timings": {
                "model_load_secs": round(model_load_secs, 2),
                "separate_secs": round(separate_secs, 2),
            },
        }
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
