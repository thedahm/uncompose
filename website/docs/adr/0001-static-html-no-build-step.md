# 0001 — Static HTML and CSS, no build step, `site/` is what ships

Status: accepted (2026-08-10)

## Context

The Uncompose website exists to say what the ecosystem is. Its v0.1 content is fixed and
small — three tool definitions, a command picture, local-first copy, links (uncompose#71)
— and the milestone that creates it (uncompose#92) also asks the site to practise what
the tools preach: zero tracking, no third-party requests.

Websites accrete toolchains. A generator arrives to template a shared header, a bundler
arrives to minify, a framework arrives because the generator wants one, and a year later
publishing a paragraph needs a working Node install and a lockfile audit. For a repo whose
entire content is one page, that cost buys nothing.

## Decision

The site is plain HTML and CSS, authored directly. No framework, no static-site generator,
no bundler, no build step, and no JavaScript required for any content on the page.

Everything published lives in `site/`, which is the deploy's output directory. Repo
documentation (`README.md`, `docs/`, ADRs) and the checks in `tests/` therefore never
reach visitors — the served surface is exactly the directory named, not "the repo minus
whatever we remembered to exclude".

Styling is one same-origin stylesheet using system fonts. Nothing on the page — no font,
no script, no image, no analytics beacon — is fetched from another origin, so a visitor's
browser talks only to uncompose.org.

`tests/check_site.py` enforces the parts of this that are silent when broken: that the
page still carries the brief's definitions, the command picture, the local-first copy, and
both link sets, and that nothing on it would make an off-origin request. It runs on a
stock Python 3 with no dependencies, because a checker that needed installing would be the
toolchain arriving through the back door.

## Alternatives considered

- **A static-site generator (Astro, Eleventy, Hugo).** Buys templating and asset pipelines
  the site has no use for at one page, and costs a runtime, a lockfile, and a build that
  can break independently of the content.
- **Tailwind or any CSS framework.** A build step for a stylesheet shorter than its own
  config.
- **A webfont.** One line of CSS, one third-party origin, and the end of the zero-request
  claim. System font stacks look native and cost nothing.
- **Serving the repo root.** Simpler to configure and immediately wrong: `CONTEXT.md`,
  ADRs, and CI files would all be public URLs on the product's front door.

## Consequences

- Publishing a change is editing a file. Nothing is installed to work on the site.
- Shared markup across pages would be copy-paste. At one page that is free; if the site
  ever grows past a handful of pages, this ADR is the one to revisit — with the note that
  a second page is not itself a reason to adopt a generator.
- The zero-tracking claim is mechanically checked rather than asserted, so a future
  contributor cannot quietly add a beacon without CI failing.
