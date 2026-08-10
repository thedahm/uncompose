#!/usr/bin/env python3
"""Check the hosted JSON Schemas against what they promise to be.

Run with any Python 3 — no dependencies (matching check_site.py's no-toolchain rule,
ADR-0001), except a network fetch to each schema's pinned source ref: this check verifies
provenance, not just shape, and that seam is GitHub's API, not deployed Pages/DNS state
(the file/HTTP seam described in uncompose#92, distinct from the live-URL and redirect
checks ADR-0002 defers to the release checklist).

For each schema in schemas/sources.json this checks: the served file exists at the exact
identifier URL's path, is parseable JSON, declares that identifier as its own `$id`, is
byte-identical to the file at its pinned source ref, and that site/_headers serves it with
a JSON content type.
"""

from __future__ import annotations

import json
import sys
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
SOURCES_FILE = ROOT / "schemas" / "sources.json"
HEADERS_FILE = SITE / "_headers"


def load_sources() -> list[dict]:
    return json.loads(SOURCES_FILE.read_text(encoding="utf-8"))["schemas"]


def check_served_path(entry: dict) -> list[str]:
    """The file must live at exactly the path its identifier URL implies."""
    served_path = entry["served_path"]
    expected = "site" + urlparse(entry["identifier_url"]).path
    if served_path != expected:
        return [
            f"{served_path}: served_path does not match identifier_url "
            f"{entry['identifier_url']!r} (expected {expected!r})"
        ]
    return []


def check_present_and_parseable(entry: dict) -> tuple[list[str], bytes | None, dict | None]:
    path = ROOT / entry["served_path"]
    if not path.is_file():
        return [f"{entry['served_path']}: missing"], None, None

    raw = path.read_bytes()
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as exc:
        return [f"{entry['served_path']}: not parseable JSON ({exc})"], raw, None

    return [], raw, parsed


def check_id_matches(entry: dict, parsed: dict) -> list[str]:
    schema_id = parsed.get("$id")
    if schema_id != entry["identifier_url"]:
        return [
            f"{entry['served_path']}: $id {schema_id!r} does not match its identifier "
            f"URL {entry['identifier_url']!r}"
        ]
    return []


def check_matches_source_ref(entry: dict, raw: bytes) -> list[str]:
    url = (
        f"https://raw.githubusercontent.com/{entry['repo']}/{entry['ref']}/"
        f"{entry['source_path']}"
    )
    try:
        with urllib.request.urlopen(url, timeout=20) as response:  # noqa: S310 (fixed https host)
            upstream = response.read()
    except (urllib.error.URLError, TimeoutError) as exc:
        return [
            f"{entry['served_path']}: could not fetch pinned ref {entry['repo']}@"
            f"{entry['ref']} to verify byte-identity ({exc})"
        ]

    if upstream != raw:
        return [
            f"{entry['served_path']}: does not match {entry['repo']}@{entry['ref']} "
            f"({entry['source_path']}) — the committed copy has drifted from its pinned "
            "source ref"
        ]
    return []


def parse_headers_file(text: str) -> dict[str, list[str]]:
    """Path -> its indented header lines, per Cloudflare Pages' `_headers` format:
    a path on its own line, followed by one or more indented `Name: value` lines,
    blocks separated by blank lines. Comment lines (`#`) are ignored.
    """
    blocks: dict[str, list[str]] = {}
    current_path: str | None = None
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if line[0].isspace():
            if current_path is not None:
                blocks[current_path].append(line.strip())
        else:
            current_path = line.strip()
            blocks.setdefault(current_path, [])
    return blocks


def check_headers(entries: list[dict]) -> list[str]:
    if not HEADERS_FILE.is_file():
        return ["site/_headers: missing — schema URLs need a pinned JSON content type"]

    blocks = parse_headers_file(HEADERS_FILE.read_text(encoding="utf-8"))
    violations = []
    for entry in entries:
        url_path = urlparse(entry["identifier_url"]).path
        header_lines = blocks.get(url_path)
        if header_lines is None:
            violations.append(
                f"site/_headers: no stanza for {url_path} (needed for {entry['served_path']})"
            )
            continue

        stanza = "\n".join(header_lines).lower()
        if "content-type" not in stanza or "application/json" not in stanza:
            violations.append(
                f"site/_headers: {url_path} does not declare an application/json "
                "Content-Type"
            )
    return violations


def main() -> int:
    if not SOURCES_FILE.is_file():
        print(f"missing {SOURCES_FILE.relative_to(ROOT)}")
        return 1

    entries = load_sources()
    violations: list[str] = []

    for entry in entries:
        violations += check_served_path(entry)
        present_violations, raw, parsed = check_present_and_parseable(entry)
        violations += present_violations
        if raw is None:
            continue
        if parsed is not None:
            violations += check_id_matches(entry, parsed)
            violations += check_matches_source_ref(entry, raw)

    violations += check_headers(entries)

    for violation in violations:
        print(violation)

    if violations:
        print(f"\n{len(violations)} violation(s)")
        return 1

    print("schema checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
