"""Fake audio-separator, substituted at its Python interface.

The shim under test imports `Separator` from here (the fakes dir shadows any
real install on sys.path). Behavior is steered by env vars so it works the
same in-process and across the subprocess tests:

- FAKE_SEPARATOR_CALL_LOG: path to write a JSON log of calls for assertions
- FAKE_SEPARATOR_SKIP_STEMS: comma-separated output names to *not* write,
  simulating audio-separator swallowing a decode failure
"""

import json
import os


class Separator:
    def __init__(self, **kwargs):
        self.kwargs = kwargs
        self.calls = []
        self.loaded = None

    def load_model(self, model_filename):
        self.loaded = model_filename
        self.calls.append({"method": "load_model", "model_filename": model_filename})

    def separate(self, audio_path, custom_output_names=None):
        # Anything the real library prints or bars through stdout must not
        # corrupt the JSONL stream; the purity test relies on this print.
        print("fake audio-separator chatter on stdout")
        self.calls.append(
            {
                "method": "separate",
                "audio_path": audio_path,
                "custom_output_names": custom_output_names,
            }
        )
        skip = set(filter(None, os.environ.get("FAKE_SEPARATOR_SKIP_STEMS", "").split(",")))
        for name in (custom_output_names or {}).values():
            if name in skip:
                continue
            path = os.path.join(self.kwargs["output_dir"], f"{name}.wav")
            with open(path, "wb") as f:
                f.write(b"RIFF")
        self._write_call_log()

    def _write_call_log(self):
        log_path = os.environ.get("FAKE_SEPARATOR_CALL_LOG")
        if log_path:
            with open(log_path, "w") as f:
                json.dump({"init": self.kwargs, "calls": self.calls}, f)
