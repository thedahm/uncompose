"""Reading files off disk the way the manifest says to read them.

Both legs' tests check registered hashes against the bytes on disk, and both
would otherwise carry their own copy of the same three lines. The manifest and
the comparison record agree on SHA-256, so there is one way to do this.
"""

from __future__ import annotations

import hashlib
from pathlib import Path


def sha256(path: Path) -> str:
    """The hash the manifest and the record both name a file by."""
    return hashlib.sha256(path.read_bytes()).hexdigest()
