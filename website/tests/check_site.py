#!/usr/bin/env python3
"""Check the landing page against the promises the site makes about itself.

Run with any Python 3 — no dependencies, matching the site's own no-toolchain rule
(ADR-0001). Prints one line per violation and exits nonzero if there are any.

What is checked here is what the site would otherwise only claim: that the three
tool definitions are the brief's own words, that the command picture is present,
that the local-first copy is on the page itself, that both link sets are complete,
and that a visitor's browser makes no request off this origin. Copy is checked
against the rendered text of <body>, not the source, so a claim buried in a meta
tag does not count as saying it.
"""

from __future__ import annotations

import re
import sys
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
PAGE = "index.html"

# The brief's one-line definitions (uncompose#55), verbatim. The landing page
# quotes these; drift here is drift from the ecosystem's own definition.
TOOL_DEFINITIONS = [
    ("Uncompose", "Transform recorded music into stems."),
    (
        "Uncompose Project",
        "Record the assets, relationships, and processes in an audio project.",
    ),
    (
        "Uncompose Compare",
        "Evaluate audio assets through controlled listening and structured observations.",
    ),
]

# The command picture, also from the brief.
COMMAND_PICTURE = [
    "uncompose separate song.wav",
    "uncompose project init",
    "uncompose project import song.stems/job.json",
    "uncompose compare vocals-a.wav vocals-b.wav",
]

LOCAL_FIRST_CLAIMS = [
    "no accounts",
    "no telemetry",
    "no network I/O",
    "never leaves your machine",
]

REQUIRED_LINKS = [
    "https://github.com/thedahm/uncompose",
    "https://github.com/thedahm/uncompose-project",
    "https://github.com/thedahm/uncompose-compare",
    "https://pypi.org/project/uncompose/",
    "https://pypi.org/project/uncompose-project/",
    "https://pypi.org/project/uncompose-compare/",
]

# Elements that fetch a subresource or execute code. <a href> is a link the
# visitor chooses to follow, not a request the page makes, so it is not here.
FETCHING_ATTRS = {"src", "srcset", "poster", "data", "action", "formaction"}
FORBIDDEN_TAGS = {"script", "iframe", "embed", "object", "frame"}

# Anything that is not a same-origin relative reference.
OFF_ORIGIN = re.compile(r"^(?:[a-zA-Z][a-zA-Z0-9+.-]*:|//)")


def is_off_origin(value: str) -> bool:
    value = value.strip()
    return bool(OFF_ORIGIN.match(value)) and not value.startswith("data:")


def off_origin_urls(attr: str, value: str) -> list[str]:
    """The off-origin URLs one attribute would fetch.

    Most attributes hold a single URL; `srcset` holds several, each a URL followed
    by a descriptor ("small.png 1x, https://cdn/large.png 2x"), so one off-origin
    candidate among same-origin ones has to be caught on its own.
    """
    if attr == "srcset":
        urls = [candidate.split()[0] for candidate in value.split(",") if candidate.strip()]
    else:
        urls = [value]
    return [url for url in urls if is_off_origin(url)]


class Page(HTMLParser):
    """Collects what a visitor gets: the text, the links, and the requests made."""

    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.violations: list[str] = []
        self.links: list[str] = []
        self._text: list[str] = []
        self._in_body = False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        if tag == "body":
            self._in_body = True
        if tag in FORBIDDEN_TAGS:
            self.violations.append(f"{PAGE}: <{tag}> — the page runs and embeds nothing")

        attributes = dict(attrs)
        for name, value in attributes.items():
            if not value:
                continue
            if name in FETCHING_ATTRS:
                for url in off_origin_urls(name, value):
                    self.violations.append(
                        f"{PAGE}: <{tag} {name}> fetches off-origin: {url}"
                    )
            if name == "style" and "url(" in value:
                self.violations.append(f"{PAGE}: inline style fetches a resource: {value}")

        href = (attributes.get("href") or "").strip()
        if tag == "link" and is_off_origin(href):
            self.violations.append(f"{PAGE}: <link href> is off-origin: {href}")
        if tag == "a" and href:
            self.links.append(href)

    def handle_endtag(self, tag: str) -> None:
        if tag == "body":
            self._in_body = False

    def handle_data(self, data: str) -> None:
        if self._in_body:
            self._text.append(data)

    @property
    def text(self) -> str:
        return "".join(self._text)


def check_page(html: str) -> list[str]:
    page = Page()
    page.feed(html)
    violations = list(page.violations)

    # Wrapping the markup must never change what the page is found to say, so
    # prose is matched on flattened text; the command picture keeps its lines,
    # which are its content.
    prose = " ".join(page.text.split())

    for name, definition in TOOL_DEFINITIONS:
        if definition not in prose:
            violations.append(
                f"{PAGE}: missing the brief's definition of {name}: {definition!r}"
            )

    for command in COMMAND_PICTURE:
        if command not in page.text:
            violations.append(f"{PAGE}: missing command-picture line: {command!r}")

    for claim in LOCAL_FIRST_CLAIMS:
        if claim.lower() not in prose.lower():
            violations.append(f"{PAGE}: missing local-first copy: {claim!r}")

    for link in REQUIRED_LINKS:
        if link not in page.links:
            violations.append(f"{PAGE}: missing link: {link}")

    return violations


def check_stylesheets() -> list[str]:
    violations = []
    for css in sorted(SITE.glob("**/*.css")):
        text = css.read_text(encoding="utf-8")
        for ref in re.findall(r"url\(\s*['\"]?([^'\")]+)", text):
            if is_off_origin(ref):
                violations.append(f"{css.name}: fetches off-origin: {ref}")
        if "@import" in text:
            violations.append(f"{css.name}: @import pulls in another stylesheet")
    return violations


def check_no_toolchain() -> list[str]:
    # ADR-0001: no framework, no SSG, no build step. These files are how one arrives.
    forbidden = [
        "package.json",
        "yarn.lock",
        "Gemfile",
        "config.toml",
        "_config.yml",
        "astro.config.mjs",
        "next.config.js",
        "vite.config.js",
    ]
    return [
        f"{name}: a build toolchain has no place in this repo"
        for name in forbidden
        if (ROOT / name).exists()
    ]


def main() -> int:
    index = SITE / PAGE
    if not index.is_file():
        print(f"missing {index.relative_to(ROOT)}")
        return 1

    violations = check_page(index.read_text(encoding="utf-8"))
    violations += check_stylesheets()
    violations += check_no_toolchain()

    for violation in violations:
        print(violation)

    if violations:
        print(f"\n{len(violations)} violation(s)")
        return 1

    print("site checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
