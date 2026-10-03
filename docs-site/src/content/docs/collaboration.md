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

Both subcommands authenticate with a personal access token, resolved through
the same ladder as the `jira`/`linear` tokens. The first rung that yields a
usable value wins; a failed command or refused value is reported as a
`warning:` and the chain continues:

1. The `GH_TOKEN` environment variable.
2. The `GITHUB_TOKEN` environment variable.
3. `github.token` in `.accelerator/config.local.md`.
4. `github.token_cmd` in `.accelerator/config.local.md`, run by the
   command runner described in
   [`/accelerator:configure`](reference/skills/config/configure.md).
5. `github.token` in the shared `.accelerator/config.md`, only when
   `config.local.md` does not exist.

An ambient env var reliably escapes a stale or over-broad on-filesystem
config value rather than being shadowed by it. `github.token_cmd` is a
consent key: only you may supply a command that runs on your machine. These
refusals exit 2 when nothing else resolves, and are warnings otherwise:

| Code                             | Refuses                                                        |
|----------------------------------|----------------------------------------------------------------|
| `E_CONSENT_KEY_TEAM_LEVEL`       | `github.token_cmd` in the shared `config.md`                   |
| `E_CONSENT_KEY_TRACKED`          | `github.token_cmd` in a `config.local.md` tracked by version control |
| `E_CONSENT_KEY_TRACKING_UNKNOWN` | `github.token_cmd` in a `config.local.md` whose tracking status cannot be determined |
| `E_TOKEN_FROM_TRACKED_FILE`      | `github.token` in a tracked or undeterminable `config.local.md` |
| `E_TOKEN_MALFORMED`              | A token carrying a control character                           |

When the tracking status cannot be determined, the refusal names `GH_TOKEN`
as the route that still works. The
[consent keys](reference/skills/config/configure.md#consent-keys) reference
gives each code's remedy. Configure a token with:

```bash
accelerator config set github.token <token>          # personal, .accelerator/config.local.md
accelerator config set github.token_cmd '<command>'   # personal only — never in the shared config
```

The personal config file (`.accelerator/config.local.md`) must be mode
0600 or stricter and not a symlink. Otherwise it is not read, an
`E_LOCAL_PERMS_INSECURE` warning says so, a team `github.token` is not used
in its place, `GH_TOKEN` still works, and the command fails with that code (exit 2) unless `GH_TOKEN` or
`GITHUB_TOKEN` supplies a token; see
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
