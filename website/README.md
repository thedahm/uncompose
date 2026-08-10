# Uncompose Website

The front door for the Uncompose family: [uncompose.org](https://uncompose.org).

One landing page that says what Uncompose is — the three tools with their one-line
definitions, the shared command namespace, honest local-first copy, and links to source
and packages. It is plain HTML and CSS with no framework, no static-site generator, and
no build step (see [ADR-0001](docs/adr/0001-static-html-no-build-step.md)), and it
carries no analytics, no tracking, and no third-party requests — the site keeps the same
promise the tools make.

## Layout

```
site/     what is published — the deploy's output directory, nothing else is served
tests/    check_site.py, the content and zero-tracking checks CI runs
docs/     ADRs and the deployment runbook (repo docs, never published)
```

## Working on it

There is nothing to install and nothing to build. Edit `site/index.html` and
`site/styles.css`, then look at it:

```sh
python3 -m http.server -d site 8000    # http://localhost:8000
python3 tests/check_site.py            # what CI runs
```

## Deploying

Pushing to `main` deploys to [uncompose.org](https://uncompose.org) via Cloudflare Pages,
which builds nothing and serves `site/` as-is. `uncompose.cc` is a permanent 301 to
`uncompose.org` for the apex, `www`, and every path. Both are recorded in
[ADR-0002](docs/adr/0002-cloudflare-pages-and-cc-redirect.md), and the one-time Cloudflare
setup they depend on is written down step by step in [`docs/deploy.md`](docs/deploy.md).

## Status

Pre-v0.1, like the rest of the family. The site's first job is the landing page; serving
the family's JSON Schemas at their identifier URLs lands next
([uncompose#101](https://github.com/thedahm/uncompose/issues/101)).

## Family

This repo holds content only. It is part of the
[Uncompose](https://github.com/thedahm/uncompose) family alongside
[`uncompose-project`](https://github.com/thedahm/uncompose-project) and
[`uncompose-compare`](https://github.com/thedahm/uncompose-compare). Ecosystem-wide
decisions live in `thedahm/uncompose`'s ADR series and are cited by number here; this
repo's own ADRs cover the site.

## License

[MIT](LICENSE) © 2026 Dominic Hanzely
