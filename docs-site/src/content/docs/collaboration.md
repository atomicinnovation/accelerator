---
title: Collaboration CLI
---

`accelerator collaboration` is the sub-binary the GitHub PR skills
(`review-pr`, `respond-to-pr`, `describe-pr`) use to talk to the GitHub REST
API in-process — resolving a pull request's base (upstream) repository and
updating a pull request's body. It is plumbing rather than a feature you
reach for directly: skills invoke it through the `!`-preprocessor (see
[Anatomy of a skill invocation](internals.md#anatomy-of-a-skill-invocation)).
Running it by hand is mainly useful for reproducing what a skill did.

| Noun group | Verbs                      | What it does                                                       |
|------------|----------------------------|--------------------------------------------------------------------|
| `pr`       | `base-repo`, `update-body` | Resolving a PR's base (upstream) owner/repo, and updating its body |

See [Internals](internals.md#terminal-invocation) for how to reach
`accelerator` at all from a terminal; everything below assumes that's set up.

## `pr`

```bash
accelerator collaboration pr base-repo 42
accelerator collaboration pr update-body 42 --body-file body.md
```

`base-repo` parses the local repository's `origin` remote, looks up its
metadata, and — when it is a fork — follows GitHub's `parent` field to the
upstream repository, replicating `gh`'s own default fork-to-parent
resolution. Prints `<owner>/<repo>` to stdout on success. Exits 2 on a
usage/refusal failure (e.g. no `origin` remote configured), 1 on any other
failure (e.g. a GitHub API error), with stderr naming which stage failed.

`update-body` resolves the PR's base repository the same way `base-repo`
does, then PATCHes the PR's body via the GitHub REST API. Exits 2 on a
usage/refusal failure (e.g. a missing or unreadable `--body-file`, no
`origin` remote configured), 1 on any other failure (base-repo resolution
failure, or a GitHub API error).

## Authentication

Both subcommands authenticate with a personal access token, resolved in
this order:

1. The `GH_TOKEN` environment variable.
2. The `GITHUB_TOKEN` environment variable.
3. The `github.token` config value.
4. The `github.token_cmd` config value's output, executed via `bash -c` —
   personal config only; a `token_cmd` in the shared, committed
   `.accelerator/config.md` is refused rather than executed.

This precedence (environment first, config last) matches the `jira`/`linear`
integrations' own credential resolvers: an ambient env var reliably escapes
a stale or over-broad on-filesystem config value rather than being shadowed
by it. Configure a token with:

```bash
accelerator config set github.token <token>          # personal, .accelerator/config.local.md
accelerator config set github.token_cmd '<command>'   # personal only — never in the shared config
```

The personal config file (`.accelerator/config.local.md`) must be mode
0600 or stricter and not a symlink, or every read of it — not just
`github.token` — is refused; see
[Configuration](configuration.md#config-files).

## Local development

| Mechanism                       | Purpose                                                                                                                                                       |
|---------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `ACCELERATOR_COLLABORATION_BIN` | One-shot override pointing `accelerator collaboration …` at a locally-built `accelerator-collaboration` binary, bypassing the normal fetch-and-cache dispatch |

This mirrors `ACCELERATOR_CORPUS_BIN` and `ACCELERATOR_VCS_BIN` for the
plugin's other dispatched sub-binaries — set it when working on
`cli/collaboration/`, `cli/github/`, or `cli/collaboration-cli/` in this
repository, so dispatch resolves the binary you just built instead of
trying to fetch a release.
