#!/usr/bin/env python3
"""Check that every internal link and resource reference in site/ resolves.

Run with any Python 3 — no dependencies (ADR-0001). A same-origin `href`/`src` that
points nowhere is invisible until a visitor clicks it or a browser fails to load it;
this walks every page under `site/`, resolves each relative reference against the files
actually there, and checks any `#fragment` against a real `id` on the target page.
References that name somewhere else (anything with a scheme, `data:` included, or a
protocol-relative `//`) are out of scope here — `check_site.py` is what enforces that the
page makes no off-origin request at all.
"""

from __future__ import annotations

import re
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urldefrag

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"

LINK_ATTRS = {"href", "src", "srcset", "poster", "action", "formaction"}

# Anything carrying a scheme (`https:`, `mailto:`, `data:`) or protocol-relative `//`
# names something outside this tree, so there is no file here to resolve it to.
NON_LOCAL = re.compile(r"^(?:[a-zA-Z][a-zA-Z0-9+.-]*:|//)")


def is_local(value: str) -> bool:
    return not NON_LOCAL.match(value.strip())


def reference_urls(attr: str, value: str) -> list[str]:
    """The URLs one attribute points at.

    Most attributes hold a single URL; `srcset` holds several, each a URL followed by a
    descriptor ("small.png 1x, large.png 2x"), so each has to be resolved on its own.
    """
    if attr == "srcset":
        return [candidate.split()[0] for candidate in value.split(",") if candidate.strip()]
    return [value]


class Page(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: set[str] = set()
        self.refs: list[tuple[str, str]] = []  # (attr, value)

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self._handle(tag, attrs)

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self._handle(tag, attrs)

    def _handle(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        element_id = attributes.get("id")
        if element_id:
            self.ids.add(element_id)
        for name, value in attributes.items():
            if name in LINK_ATTRS and value and value.strip():
                self.refs.append((name, value.strip()))


def load_pages() -> dict[Path, Page]:
    pages = {}
    for html_file in sorted(SITE.glob("**/*.html")):
        page = Page()
        page.feed(html_file.read_text(encoding="utf-8"))
        pages[html_file] = page
    return pages


def check_reference(source: Path, attr: str, value: str, pages: dict[Path, Page]) -> str | None:
    name = str(source.relative_to(SITE))
    path_part, fragment = urldefrag(value)

    if not path_part:
        # Fragment-only reference: resolves against the current page's own ids.
        if fragment and fragment not in pages[source].ids:
            return f"{name}: <{attr}='{value}'> — no element with id {fragment!r} on this page"
        return None

    # A leading slash is the site root a visitor gets, which is `site/` itself — not the
    # filesystem root Path's `/` join would hand back.
    if path_part.startswith("/"):
        target = (SITE / path_part.lstrip("/")).resolve()
    else:
        target = (source.parent / path_part).resolve()

    try:
        target.relative_to(SITE.resolve())
    except ValueError:
        return f"{name}: <{attr}='{value}'> resolves outside site/"

    if not target.is_file():
        return f"{name}: <{attr}='{value}'> — {path_part} does not exist"

    if fragment:
        target_page = pages.get(target)
        if target_page is None:
            # Not an HTML page (e.g. a stylesheet with a #fragment) — nothing to check.
            return None
        if fragment not in target_page.ids:
            return (
                f"{name}: <{attr}='{value}'> — no element with id {fragment!r} in "
                f"{target.relative_to(SITE)}"
            )
    return None


def main() -> int:
    if not SITE.is_dir():
        print(f"missing {SITE.relative_to(ROOT)}")
        return 1

    pages = load_pages()
    if not pages:
        print(f"no HTML pages found under {SITE.relative_to(ROOT)}")
        return 1

    violations: list[str] = []
    for source, page in pages.items():
        for attr, value in page.refs:
            for url in reference_urls(attr, value):
                if not is_local(url):
                    continue
                violation = check_reference(source, attr, url, pages)
                if violation:
                    violations.append(violation)

    for violation in violations:
        print(violation)

    if violations:
        print(f"\n{len(violations)} violation(s)")
        return 1

    print("link checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
