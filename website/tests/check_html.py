#!/usr/bin/env python3
"""Validate the structural correctness of every page in site/.

Run with any Python 3 — no dependencies, matching check_site.py's no-toolchain rule
(ADR-0001): reaching for `npx html-validate` would be exactly the toolchain-through-the-
back-door this repo has already decided against. This checks what a real validator would
catch and a lenient parser silently accepts: a doctype, matching/nested tags, a `lang` on
`<html>`, a non-empty `<title>` and a charset declaration, unique `id`s, and that every
`<a>` and `<img>` carries the attributes a reader or a screen reader needs.
"""

from __future__ import annotations

import sys
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"

# Elements with no closing tag; absence from the open-tag stack is correct, not an error.
VOID_ELEMENTS = {
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "param", "source", "track", "wbr",
}


class Page(HTMLParser):
    def __init__(self, name: str) -> None:
        super().__init__(convert_charrefs=True)
        self.name = name
        self.violations: list[str] = []
        self._stack: list[str] = []
        self.has_doctype = False
        self.html_lang: str | None = None
        self.has_charset = False
        self.title_text = ""
        self._in_title = False
        self.id_counts: dict[str, int] = {}

    def handle_decl(self, decl: str) -> None:
        if decl.strip().lower().startswith("doctype html"):
            self.has_doctype = True

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self._check_attributes(tag, dict(attrs))
        if tag == "title":
            self._in_title = True
        if tag not in VOID_ELEMENTS:
            self._stack.append(tag)

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        # An XHTML-style `<img/>` closes itself: its attributes are checked the same way,
        # but it never joins the open-tag stack and encloses no content.
        self._check_attributes(tag, dict(attrs))

    def _check_attributes(self, tag: str, attributes: dict[str, str | None]) -> None:
        element_id = attributes.get("id")
        if element_id:
            self.id_counts[element_id] = self.id_counts.get(element_id, 0) + 1

        if tag == "html":
            self.html_lang = attributes.get("lang")
        if tag == "meta" and attributes.get("charset") is not None:
            self.has_charset = True
        if tag == "a" and not (attributes.get("href") or "").strip():
            self.violations.append(f"{self.name}: <a> with no (or empty) href")
        if tag == "img" and "alt" not in attributes:
            self.violations.append(f"{self.name}: <img> with no alt attribute")

    def handle_endtag(self, tag: str) -> None:
        if tag == "title":
            self._in_title = False
        if tag in VOID_ELEMENTS:
            return
        if not self._stack or self._stack[-1] != tag:
            self.violations.append(
                f"{self.name}: </{tag}> does not match the currently open tag "
                f"({self._stack[-1] if self._stack else 'none'})"
            )
            return
        self._stack.pop()

    def handle_data(self, data: str) -> None:
        if self._in_title:
            self.title_text += data

    def finish(self) -> list[str]:
        violations = list(self.violations)
        if self._stack:
            violations.append(f"{self.name}: unclosed tag(s): {', '.join(self._stack)}")
        if not self.has_doctype:
            violations.append(f"{self.name}: missing <!DOCTYPE html>")
        if not self.html_lang:
            violations.append(f"{self.name}: <html> missing a lang attribute")
        if not self.has_charset:
            violations.append(f"{self.name}: missing <meta charset>")
        if not self.title_text.strip():
            violations.append(f"{self.name}: missing or empty <title>")
        for element_id, count in self.id_counts.items():
            if count > 1:
                violations.append(f"{self.name}: id {element_id!r} used {count} times")
        return violations


def check_page(path: Path) -> list[str]:
    name = str(path.relative_to(SITE))
    page = Page(name)
    page.feed(path.read_text(encoding="utf-8"))
    return page.finish()


def main() -> int:
    paths = sorted(SITE.glob("**/*.html"))
    if not paths:
        print(f"no HTML pages found under {SITE.relative_to(ROOT)}")
        return 1

    violations: list[str] = []
    for path in paths:
        violations += check_page(path)

    for violation in violations:
        print(violation)

    if violations:
        print(f"\n{len(violations)} violation(s)")
        return 1

    print("html checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
