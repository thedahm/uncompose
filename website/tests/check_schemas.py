#!/usr/bin/env python3
"""Check the hosted JSON Schemas against what they promise to be.

Run with any Python 3 — no dependencies (matching check_site.py's no-toolchain rule,
ADR-0001), except a network fetch to each schema's pinned source ref: this check verifies
provenance, not just shape, and that seam is GitHub's raw-content host, not deployed
Pages/DNS state (the file/HTTP seam described in uncompose#92, distinct from the live-URL
and redirect checks ADR-0002 defers to the release checklist).

For each schema in schemas/sources.json this checks: the served file exists at the exact
identifier URL's path, is parseable JSON, declares that identifier as its own `$id`, is
byte-identical to the file at its pinned source ref, and that site/_headers serves it with
a JSON content type.

It also checks the pin record itself against the prose that restates it, because the
release checklist's schema-refresh step (uncompose#92 story 25) is hand-run and half of it
lands in `schemas/README.md`: `ref_kind` must match the shape of `ref`, the README's
"Pinned from" table must name the same ref, a `tag` pin must not still carry a pre-v0.1.0
`note`, and the README must not still say no tool has tagged once every pin is a tag. A
half-done refresh is then a red check rather than a green one over a false record.
"""

from __future__ import annotations

import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
SOURCES_FILE = ROOT / "schemas" / "sources.json"
SCHEMAS_README = ROOT / "schemas" / "README.md"
HEADERS_FILE = SITE / "_headers"

REF_KINDS = ("commit", "tag", "branch")
COMMIT_REF = re.compile(r"^[0-9a-f]{40}$")

# The sentence in schemas/README.md that stops being true the moment both pins are tags.
PRE_TAG_CLAIM = "Neither tool has cut a v0.1.0 tag yet"


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


def check_ref_kind(entry: dict) -> list[str]:
    """`ref_kind` must describe the ref actually pinned, and a tag pin must be finished."""
    ref, kind = entry["ref"], entry.get("ref_kind")
    if kind not in REF_KINDS:
        return [
            f"{entry['served_path']}: ref_kind {kind!r} is none of {', '.join(REF_KINDS)}"
        ]

    looks_like_commit = bool(COMMIT_REF.match(ref))
    if looks_like_commit and kind != "commit":
        return [
            f"{entry['served_path']}: ref {ref!r} is a commit sha but ref_kind says {kind!r}"
        ]
    if not looks_like_commit and kind == "commit":
        return [
            f"{entry['served_path']}: ref_kind says 'commit' but ref {ref!r} is not a sha"
        ]

    # The refresh step moves a pin to a tag and drops the note explaining why it
    # was not one; a tag still carrying that note is a half-done refresh.
    if kind == "tag" and entry.get("note"):
        return [
            f"{entry['served_path']}: pinned to tag {ref!r} but still carries the "
            "pre-v0.1.0 note — the release checklist's step 13 drops it"
        ]
    return []


def check_readme_records_pin(entry: dict, readme: str) -> list[str]:
    """The README's "Pinned from" table must name the ref sources.json pins."""
    served = entry["served_path"].removeprefix("site/")
    rows = [line for line in readme.splitlines() if line.startswith("|") and served in line]
    if not rows:
        return [
            f"schemas/README.md: no 'Pinned from' row for {served} "
            "(the table and sources.json must name the same schemas)"
        ]
    if not any(entry["ref"] in row for row in rows):
        return [
            f"schemas/README.md: the row for {served} does not name the pinned ref "
            f"{entry['ref']!r} that sources.json records"
        ]
    return []


def check_readme_prose(entries: list[dict], readme: str) -> list[str]:
    """Once every pin is a tag, the README must stop saying neither tool has tagged."""
    if entries and all(entry.get("ref_kind") == "tag" for entry in entries):
        if PRE_TAG_CLAIM.lower() in readme.lower():
            return [
                f"schemas/README.md: still says {PRE_TAG_CLAIM!r} while every pin in "
                "sources.json is a release tag"
            ]
    return []


def check_readme(entries: list[dict]) -> list[str]:
    if not SCHEMAS_README.is_file():
        return ["schemas/README.md: missing — the pins have no prose record"]

    readme = SCHEMAS_README.read_text(encoding="utf-8")
    violations = check_readme_prose(entries, readme)
    for entry in entries:
        violations += check_readme_records_pin(entry, readme)
    return violations


def check_entry(entry: dict) -> list[str]:
    """Every check for one schema file, each step earning the next."""
    served_path = entry["served_path"]
    violations = check_served_path(entry) + check_ref_kind(entry)

    path = ROOT / served_path
    if not path.is_file():
        return violations + [f"{served_path}: missing"]

    raw = path.read_bytes()
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as exc:
        # Unparseable bytes have no `$id` to compare, and reporting drift from the pinned
        # ref on top of that would only restate the same broken file.
        return violations + [f"{served_path}: not parseable JSON ({exc})"]

    violations += check_id_matches(entry, parsed)
    violations += check_matches_source_ref(entry, raw)
    return violations


def parse_headers_file(text: str) -> dict[str, dict[str, str]]:
    """Path -> its headers, keyed by lowercased name, per Cloudflare Pages' `_headers`
    format: a path on its own line, followed by one or more indented `Name: value` lines,
    blocks separated by blank lines. Comment lines (`#`) are ignored.
    """
    blocks: dict[str, dict[str, str]] = {}
    headers: dict[str, str] | None = None
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line[0].isspace():
            headers = blocks.setdefault(line.strip(), {})
        elif headers is not None and ":" in line:
            name, _, value = line.partition(":")
            headers[name.strip().lower()] = value.strip()
    return blocks


def check_headers(entries: list[dict]) -> list[str]:
    if not HEADERS_FILE.is_file():
        return ["site/_headers: missing — schema URLs need a pinned JSON content type"]

    blocks = parse_headers_file(HEADERS_FILE.read_text(encoding="utf-8"))
    violations = []
    for entry in entries:
        url_path = urlparse(entry["identifier_url"]).path
        headers = blocks.get(url_path)
        if headers is None:
            violations.append(
                f"site/_headers: no stanza for {url_path} (needed for {entry['served_path']})"
            )
        elif not headers.get("content-type", "").lower().startswith("application/json"):
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
        violations += check_entry(entry)

    violations += check_headers(entries)
    violations += check_readme(entries)

    for violation in violations:
        print(violation)

    if violations:
        print(f"\n{len(violations)} violation(s)")
        return 1

    print("schema checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
