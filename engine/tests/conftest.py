"""Shim contract tests run with audio-separator and torch faked at their
Python interface (Testing Decisions in #22): the fakes dir must shadow any
real install before the module under test is imported."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "fakes"))
