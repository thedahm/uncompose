# 0002 — Cloudflare Pages on push to `main`, and uncompose.cc as a permanent redirect

Status: accepted (2026-08-10)

## Context

uncompose.org leads and uncompose.cc must not dead-end: older material printed the `.cc`
name, and every historical link should land on the real site (uncompose#71). Both domains'
DNS already sits on Cloudflare, and the site itself is a directory of static files with no
build step (ADR-0001).

Publishing has to be cheap enough that writing a paragraph is a git push, and it must not
introduce infrastructure — or credentials — that then need looking after.

## Decision

**Hosting.** Cloudflare Pages, connected to this repo through Cloudflare's Git
integration. A push to `main` triggers a deploy; the build command is empty and the output
directory is `site/`. Pull requests get preview deployments, which is what makes a content
change reviewable before it is public.

**Custom domains.** `uncompose.org` and `www.uncompose.org` are attached to the Pages
project; www redirects to the apex so one canonical URL is served.

**The `.cc` redirect.** A Cloudflare redirect rule on the `uncompose.cc` zone answers every
request — apex, `www`, any path, any query — with a 301 to the same path on
`uncompose.org`. It is a rule at the edge, not a page this repo serves, so nothing about
the redirect can rot with the site's content. `uncompose.cc` never becomes a second
deployment of anything.

The console steps for all three, including the proxied placeholder DNS records the `.cc`
zone needs for a redirect rule to fire at all, are written down in
[`docs/deploy.md`](../deploy.md).

## Alternatives considered

- **GitHub Pages.** Would put deploys in this repo's own CI, but the apex-domain and
  cross-domain redirect story is clunkier, and the `.cc` redirect would need a second
  hosted artifact rather than one edge rule. uncompose#71 chose Pages for exactly this.
- **A GitHub Actions deploy with `wrangler`.** Puts the deploy in version control, at the
  cost of storing a long-lived Cloudflare API token in repo secrets — a standing
  credential for a site whose entire threat model is "someone defaces the front door". The
  Git integration needs no secret at all.
- **A `.cc` Pages project serving redirects** (a `_redirects` file, or an HTML meta
  refresh). Two deployments to keep alive instead of one rule, and a meta refresh is not a
  301: it is invisible to anything that reads status codes.
- **DNS-level redirect (CNAME `.cc` to `.org`).** Not a redirect: it would serve the site
  on both names, splitting the canonical URL rather than consolidating it.

## Consequences

- Deploying is `git push`. There is no deploy workflow, no secret, and no local tooling.
- The deploy configuration lives in the Cloudflare console, not in this repo. That is a
  real cost — it is off-repo state that a reader cannot diff — so `docs/deploy.md` records
  it exactly, and re-creating the project from that document is the recovery path.
- The redirect and the live schema URLs can only be verified against deployed DNS and
  Pages state, so they are checked in the release checklist by hand rather than pretended
  at in CI (uncompose#92).
