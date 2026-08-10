# Deploying the site

The site is served by Cloudflare Pages and publishes on every push to `main`
([ADR-0002](adr/0002-cloudflare-pages-and-cc-redirect.md)). Nothing here runs from this
repo: the deploy configuration is account state in the Cloudflare console, so this
document is its record. Re-creating the project from these steps is the recovery path if
it is ever lost.

The steps below are a **one-time setup** against the `thedahm` Cloudflare account, which
already holds the `uncompose.org` and `uncompose.cc` zones. Until they have been
performed, pushes to `main` deploy nowhere.

## 1. The Pages project

Workers & Pages → Create → Pages → Connect to Git → `thedahm/uncompose-website`.

| Setting            | Value                       |
| ------------------ | --------------------------- |
| Project name       | `uncompose-website`         |
| Production branch  | `main`                      |
| Framework preset   | None                        |
| Build command      | _(empty)_                   |
| Build output dir   | `site`                      |
| Root directory     | `/`                         |

There is no build step, so the deploy is an upload of `site/` (ADR-0001). Leave preview
deployments on: they are what makes a content change reviewable on a pull request before
it is public.

## 2. Custom domains on uncompose.org

In the Pages project → Custom domains, add:

- `uncompose.org`
- `www.uncompose.org`

Cloudflare writes the DNS records itself because the zone is in the same account. The
apex is canonical, so add a redirect rule on the `uncompose.org` zone sending `www` to it:

- Rules → Redirect Rules → Create rule, name `www → apex`
- When incoming requests match: `http.host eq "www.uncompose.org"`
- Then: Dynamic redirect, `concat("https://uncompose.org", http.request.uri.path)`
- Status 301, **Preserve query string** on

Confirm SSL/TLS → Edge Certificates → **Always Use HTTPS** is on for the zone.

## 3. uncompose.cc → uncompose.org

The `.cc` zone hosts nothing. It needs proxied DNS records purely so requests reach
Cloudflare's edge, where the redirect rule answers them — without a proxied record the
rule never fires, because nothing resolves.

DNS → Records, both **proxied** (orange cloud):

| Type | Name  | Content       |
| ---- | ----- | ------------- |
| A    | `@`   | `192.0.2.1`   |
| A    | `www` | `192.0.2.1`   |

`192.0.2.1` is the reserved documentation address from RFC 5737: it is never contacted,
because a proxied record is terminated at the edge.

Then Rules → Redirect Rules → Create rule on the `uncompose.cc` zone:

- Name: `uncompose.cc → uncompose.org (permanent)`
- When incoming requests match — custom filter expression:
  ```
  (http.host eq "uncompose.cc" or http.host eq "www.uncompose.cc")
  ```
- Then: Dynamic redirect, expression:
  ```
  concat("https://uncompose.org", http.request.uri.path)
  ```
- Status **301**, **Preserve query string** on

Confirm **Always Use HTTPS** on this zone too, so `http://uncompose.cc` is upgraded before
the redirect rather than dead-ending.

## 4. Verify

Both halves of a redirect matter — the status code and where it points — and `location:`
is not among the first response headers, so filter for it rather than taking the head:

```sh
check() { curl -sI "$1" | grep -iE '^(HTTP/|location:)'; }

check https://uncompose.org/             # 200
check https://www.uncompose.org/         # 301 → https://uncompose.org/
check https://uncompose.cc/              # 301 → https://uncompose.org/
check 'https://uncompose.cc/a/b?c=1'     # 301 → https://uncompose.org/a/b?c=1
check https://www.uncompose.cc/          # 301 → https://uncompose.org/
```

Every `.cc` response must be `301` (permanent), not `302`, and must carry the path and
query through. These are checks against deployed DNS and account state, which CI cannot
honestly reproduce, so they belong to the release checklist and are run by hand.

## Day to day

- **Publishing** is `git push` to `main`. There is no deploy workflow, no secret, and no
  local tooling.
- **Rolling back** is the Pages project's deployment list: open a previous deployment and
  promote it. Then fix forward in git — the console rollback is a stopgap, not the truth.
- **Checking what shipped**: the deployment list shows the commit behind each deploy.
