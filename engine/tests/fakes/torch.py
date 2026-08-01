"""Fake torch: just enough surface for the shim's device report."""

from types import SimpleNamespace

cuda = SimpleNamespace(is_available=lambda: False)
