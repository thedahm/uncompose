# Bootstrapping `thedahm/uncompose-website`

The [`website/`](../website/) directory in this repo holds the complete initial tree of
`thedahm/uncompose-website` — the family foundation, the landing page, its checks, and the
deployment runbook — built for [uncompose#96](https://github.com/thedahm/uncompose/issues/96).

It is staged here because creating a GitHub repository needs credentials the AFK agent
does not have. Everything below is what a maintainer runs once, with their own
credentials, to give the tree its real home. **After the transplant, delete `website/` and
this document from this repo**: the website's source of truth is its own repo, and a
second copy here would drift.

The transplant and that deletion are tracked as
[uncompose#115](https://github.com/thedahm/uncompose/issues/115), which also lists the
[#92](https://github.com/thedahm/uncompose/issues/92) stories that stay open until it runs
(the `.cc` redirect, the live schema URLs, the mirrored repo settings, and the Pages
deploy).

## 1. Create the repo and push the tree

```sh
gh repo create thedahm/uncompose-website --public \
  --description "The Uncompose family's website: uncompose.org. Static HTML/CSS, no build step."

git clone https://github.com/thedahm/uncompose-website.git /tmp/uncompose-website
cp -a website/. /tmp/uncompose-website/
cd /tmp/uncompose-website
git add -A
git commit -m "Website foundation: family docs, landing page, and the Cloudflare deploy record"
# The clone of an empty repo takes its branch name from your own git config, so
# name it explicitly rather than assuming init.defaultBranch=main.
git branch -M main
git push -u origin main
```

## 2. Mirror the family repo settings

Per uncompose#68 res. 8: default `main`, issues on, delete branch on merge, secret
scanning and push protection, private vulnerability reporting, and deliberately **no**
branch protection (the PR flow stays a convention).

```sh
REPO=thedahm/uncompose-website

gh api --method PATCH repos/$REPO \
  -F has_issues=true -F has_projects=true -F has_wiki=true \
  -F allow_squash_merge=true -F allow_merge_commit=true -F allow_rebase_merge=true \
  -F allow_auto_merge=false -F delete_branch_on_merge=true \
  -f squash_merge_commit_title=COMMIT_OR_PR_TITLE \
  -f squash_merge_commit_message=COMMIT_MESSAGES

gh api --method PUT repos/$REPO/private-vulnerability-reporting
gh api --method PATCH repos/$REPO \
  -f 'security_and_analysis[secret_scanning][status]=enabled' \
  -f 'security_and_analysis[secret_scanning_push_protection][status]=enabled'
```

## 3. Create the triage labels

The same set `uncompose-project` carries, on top of GitHub's defaults.

```sh
REPO=thedahm/uncompose-website

gh label create needs-triage    -R $REPO -c d73a4a -d "Maintainer needs to evaluate this issue"
gh label create needs-info      -R $REPO -c fbca04 -d "Waiting on reporter for more information"
gh label create ready-for-agent -R $REPO -c 0e8a16 -d "Fully specified, ready for an AFK agent"
gh label create ready-for-human -R $REPO -c 1d76db -d "Requires human implementation"
gh label create Sandcastle      -R $REPO -c F9A825 -d "Issues for Sandcastle to work on"
gh label create spec            -R $REPO -c 8d65c8
```

`wontfix` ships with GitHub's default set, so the five canonical triage roles in
[`docs/agents/triage-labels.md`](../website/docs/agents/triage-labels.md) are all covered.

## 4. Deploy

The Cloudflare Pages project and the `uncompose.cc` redirect rule are console state; the
exact settings are in the tree at `docs/deploy.md`. Follow it there, then run its
verification block. Until that setup is done, pushes to `main` deploy nowhere.
