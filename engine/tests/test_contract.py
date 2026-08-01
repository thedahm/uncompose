"""The shim's side of the Engine Contract: given a request, it drives
audio-separator correctly and emits valid JSONL. No model inference here or
in CI; audio-separator is faked at its Python interface (see fakes/)."""

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

FAKES = Path(__file__).parent / "fakes"


def make_request(tmp_path, model_id="htdemucs_6s"):
    audio = tmp_path / "song.wav"
    audio.write_bytes(b"not really audio")
    output_dir = tmp_path / "out"
    output_dir.mkdir()
    return {
        "audio_path": str(audio),
        "model_id": model_id,
        "output_dir": str(output_dir),
        "model_dir": str(tmp_path / "models"),
        "device": "cpu",
    }


# --- in-process: run() drives the library correctly ---


def run_shim(request, monkeypatch, tmp_path):
    from uncompose_engine.__main__ import run

    call_log = tmp_path / "calls.json"
    monkeypatch.setenv("FAKE_SEPARATOR_CALL_LOG", str(call_log))
    events = []
    code = run(request, events.append)
    calls = json.loads(call_log.read_text()) if call_log.exists() else None
    return code, events, calls


def test_success_emits_stages_stems_and_done(monkeypatch, tmp_path):
    request = make_request(tmp_path)
    code, events, calls = run_shim(request, monkeypatch, tmp_path)

    assert code == 0
    kinds = [e["event"] for e in events]
    assert kinds[:2] == ["stage", "stage"]
    assert events[0]["stage"] == "model_load"
    assert events[1]["stage"] == "separate"

    stems = [e for e in events if e["event"] == "stem"]
    # Preset-level stem names: keys.wav, never piano.wav.
    assert sorted(e["name"] for e in stems) == sorted(
        ["vocals", "drums", "bass", "guitar", "keys", "other"]
    )
    for e in stems:
        assert os.path.exists(e["path"])

    done = events[-1]
    assert done["event"] == "done"
    assert done["model_id"] == "htdemucs_6s"
    assert done["device"] == "cpu"  # fake torch reports no CUDA
    assert done["engine_version"]
    assert set(done["timings"]) == {"model_load_secs", "separate_secs"}


def test_success_calls_audio_separator_correctly(monkeypatch, tmp_path):
    request = make_request(tmp_path)
    _, _, calls = run_shim(request, monkeypatch, tmp_path)

    assert calls["init"]["output_dir"] == request["output_dir"]
    assert calls["init"]["model_file_dir"] == request["model_dir"]
    assert calls["init"]["output_format"] == "wav"
    load, separate = calls["calls"]
    assert load == {"method": "load_model", "model_filename": "htdemucs_6s.yaml"}
    assert separate["audio_path"] == request["audio_path"]
    assert separate["custom_output_names"]["Piano"] == "keys"


def test_unknown_model_id_raises(monkeypatch, tmp_path):
    request = make_request(tmp_path, model_id="not-a-model")
    with pytest.raises(ValueError, match="unknown model id"):
        run_shim(request, monkeypatch, tmp_path)


def test_missing_stem_output_is_a_failure_not_a_quiet_success(monkeypatch, tmp_path):
    monkeypatch.setenv("FAKE_SEPARATOR_SKIP_STEMS", "keys")
    request = make_request(tmp_path)
    with pytest.raises(RuntimeError, match="keys"):
        run_shim(request, monkeypatch, tmp_path)


# --- subprocess: the executable's stream stays parseable end to end ---


def run_engine_process(stdin_text):
    env = dict(os.environ)
    env["PYTHONPATH"] = os.pathsep.join(
        [str(FAKES)] + [p for p in [env.get("PYTHONPATH")] if p]
    )
    return subprocess.run(
        [sys.executable, "-m", "uncompose_engine"],
        input=stdin_text,
        capture_output=True,
        text=True,
        timeout=60,
        env=env,
    )


def parse_events(stdout):
    return [json.loads(line) for line in stdout.splitlines() if line.strip()]


def test_stdout_is_pure_jsonl_despite_library_chatter(tmp_path):
    result = run_engine_process(json.dumps(make_request(tmp_path)))
    assert result.returncode == 0, result.stderr
    # The fake Separator prints to stdout; the shim must have routed that to
    # stderr, leaving stdout parseable line by line.
    events = parse_events(result.stdout)
    assert all("event" in e for e in events)
    assert events[-1]["event"] == "done"
    assert "chatter" not in result.stdout
    assert "chatter" in result.stderr


def test_unknown_model_emits_error_event_and_exits_nonzero(tmp_path):
    result = run_engine_process(json.dumps(make_request(tmp_path, model_id="nope")))
    assert result.returncode == 1
    events = parse_events(result.stdout)
    assert events[-1]["event"] == "error"
    assert "unknown model id" in events[-1]["message"]


def test_garbage_stdin_emits_error_event_and_exits_nonzero(tmp_path):
    result = run_engine_process("this is not json")
    assert result.returncode == 1
    events = parse_events(result.stdout)
    assert events[-1]["event"] == "error"
