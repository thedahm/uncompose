# Architecture Decision Records

This directory holds `uncompose-website`'s own numbered ADRs, starting at `0001`.

Each ADR records a decision this repo owns — what was chosen, the alternatives, and the
reasoning — in a file named `NNNN-short-slug.md`. Ecosystem-wide decisions live in the
[`thedahm/uncompose`](https://github.com/thedahm/uncompose) ADR series and are cited by
number where they apply here (e.g. the site's content and hosting were decided in
uncompose#71, the repository foundation in uncompose#68, schema identifier URLs in
uncompose#64).

## Records

- [0001 — Static HTML and CSS, no build step, `site/` is what ships](0001-static-html-no-build-step.md)
- [0002 — Cloudflare Pages on push to `main`, and uncompose.cc as a permanent redirect](0002-cloudflare-pages-and-cc-redirect.md)
- [0003 — Schema hosting as committed, pinned copies; a static-site CI lane replacing the family's Rust lanes](0003-schema-hosting-and-static-site-ci.md)
