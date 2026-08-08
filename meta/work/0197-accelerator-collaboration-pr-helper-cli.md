---
type: work-item
id: "0197"
title: "accelerator-collaboration: PR Helper CLI"
date: "2026-08-05T19:03:35+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["work-item:0173"]
tags: [rust, collaboration, cli, github, gh]
last_updated: "2026-08-08T15:30:23+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0197: accelerator-collaboration: PR Helper CLI

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Migrate the PR helpers (`pr-base-repo`, `pr-update-body`) into an
`accelerator-collaboration` sub-binary, adopting the `collaboration` domain
name for this binary ahead of the full skill-directory rename tracked
separately in work-item:0150. Add the `origin`-remote-URL-to-owner/repo
parsing capability `vcs`/`vcs-adapters` needs to support the migration (see
Requirements — no such capability exists today). This replaces bash that
skills currently shell out to at runtime with a typed, testable Rust
implementation.

## Context

Split out of work-item:0173 (now abandoned) on 2026-08-05, per that item's
review-1 scope finding: bundling `accelerator-corpus`, `accelerator-design`, and
`accelerator-collaboration` into a single story risked partial-completion
ambiguity and an oversized PR. The PR helpers stay separate as
`accelerator-collaboration` per the github→collaboration rename (precedent:
work-item:0150) — the domain is named `collaboration`, not `github`. This
binary's own naming (`collaboration`) is fixed, not open (0173's review-1
flagged the "open" wording as ambiguous on this point); the migration
replaces untestable bash bound by the project's bash 3.2 floor, consistent
with the broader shell-to-Rust migration epic (work-item:0136), giving the
authors of `review-pr`, `respond-to-pr`, and `describe-pr` (the three
skills invoking the migrated scripts) a testable, `gh`-independent call
path. The wider github→collaboration *directory* rename (renaming
`skills/github/**` itself)
is a separate, still-in-progress initiative tracked by work-item:0150 —
this item does not depend on it and does not rename the skill directory
(see Dependencies).

## Requirements

- `accelerator-collaboration` — the PR helpers (`pr-base-repo`,
  `pr-update-body`); calls the GitHub REST API directly via `octocrab`, an
  in-process Rust HTTP client, not by shelling out to `gh` — removing the
  runtime requirement that `gh` be installed and authenticated locally. The
  body-update subcommand accepts the PR body via a `--body-file <path>`
  argument, mirroring the source script's file-based interface.
  Authenticates via the `github.token`/`github.token_cmd`
  config pairing (the same mechanism already used by `jira`/`linear`), with
  `GH_TOKEN` then `GITHUB_TOKEN` (in that order, matching `gh`'s own
  documented env-var precedence) as fallbacks below both config keys.
  Domain named `collaboration`, not `github`.
- Add `github.token`/`github.token_cmd` entries to the config catalogue — no
  such entries exist today; `cli/config/src/catalogue.rs`'s `EXTRA_KEYS` list
  (`catalogue.rs:121-131`) only has `jira.*`/`linear.*` pairs. This item adds
  the two `github.*` strings to `EXTRA_KEYS`, mirroring the `linear` shape (a
  bare `token`/`token_cmd` pair, no extra fields), plus the matching bash
  mirror in `config-defaults.sh`.
- Add `origin`-remote-URL-to-owner/repo parsing to `vcs`/`vcs-adapters` — no
  such capability exists in these crates today (`vcs`'s `RepoFacts.name`
  derives from the local directory name only, not any remote; see Technical
  Notes). Reads the git remote named `origin` specifically (mirroring `gh`'s
  default-repository behaviour); if no `origin` remote is configured, fails
  with a clear error — the Rust-level equivalent of `pr-base-repo.sh`'s
  "no default remote repository" remediation (Technical Notes branch (3) of
  the resolver subcommand's checklist). Parses the standard
  GitHub remote URL forms: `https://github.com/{owner}/{repo}.git`,
  `https://github.com/{owner}/{repo}`, `git@github.com:{owner}/{repo}.git`,
  and `ssh://git@github.com/{owner}/{repo}.git`. This item scopes the
  addition of a small remote-reading port plus this parsing logic, rather
  than treating it as already delivered.
- Rewrite the call sites and `allowed-tools` of every skill invoking
  `skills/github/scripts/pr-base-repo.sh` or
  `skills/github/describe-pr/scripts/pr-update-body.sh` to call the new
  `accelerator collaboration` subcommands, following the invocation contract
  established in 0167.
- Remove the migrated `skills/github/scripts/pr-base-repo.sh` and
  `skills/github/describe-pr/scripts/pr-update-body.sh` scripts, with the
  `_EXPECTED_GITHUB_SUITES` suite floor (the minimum-suite-count CI guard in
  `tasks/test/integration.py`) decremented from 3 to 0, coordinated in
  lockstep with work-item:0174.
- `accelerator-collaboration` satisfies every item of the sub-binary
  registration checklist at
  `tasks/README.md#registering-a-dispatched-sub-binary`.

## Acceptance Criteria

- [ ] **Base-repo and body-update REST behaviour**: `accelerator
      collaboration …` reproduces the PR-helper behaviours via direct
      in-process calls to the GitHub REST API using `octocrab` — not by
      shelling out to `gh` — specifically: resolving a PR's base repository
      via `GET /repos/{owner}/{repo}/pulls/{pull_number}`, where
      `{owner}/{repo}` is the *local* repository's own owner/repo, resolved
      by parsing the `origin` git remote's URL via new `vcs`/`vcs-adapters`
      parsing logic (see Requirements — this capability does not exist
      today; supported URL forms and the missing-remote error path are
      enumerated there) rather than `gh`'s implicit git-remote inference;
      the *base* owner/repo is then derived from the response's top-level
      `url` field (replacing `gh pr view <pr> --json url`); and updating a
      PR's body via `PATCH /repos/{owner}/{repo}/pulls/{pull_number}` with a
      JSON `{"body": ...}` payload (replacing `gh api --method PATCH ...
      --input <file>`).
- [ ] **Authentication**: Authenticates via the existing `token`/`token_cmd`
      config pairing (`github.token` / `github.token_cmd`), following the
      same resolution precedence and shared-config `token_cmd` ban (`token_cmd`
      may not be set in the checked-in/shared config file, only in local
      overrides) already implemented for `jira.token`/`jira.token_cmd` and
      `linear.token`/`linear.token_cmd` in `cli/config/src/catalogue.rs`.
      Precedence, highest first: `github.token` config value, then
      `github.token_cmd` output, then the `GH_TOKEN` env var, then the
      `GITHUB_TOKEN` env var (`GH_TOKEN` over `GITHUB_TOKEN` matches `gh`'s
      own documented env-var precedence) — no dependency on the `gh` CLI
      being installed or authenticated locally.
- [ ] **Verification strategy**: Verified via repointed suites (existing
      suites redirected to invoke `accelerator collaboration` instead of the
      bash scripts, with HTTP-level stubbing replacing the PATH-`gh`-stub
      harness), with one dedicated characterization test per branch
      enumerated in Technical Notes for `pr-base-repo.sh`'s resolver
      subcommand (7 branches) and `pr-update-body.sh`'s body-update
      subcommand (5 branches) — 12 characterization tests, restated as
      target Rust behaviour rather than the source bash/`jq` implementation
      detail (so branches with no Rust analog, e.g. `jq`-missing, are
      excluded from the count rather than tested vacuously), written
      regardless of whether a repointed suite case already exercises the
      same branch. Branch (2) in the resolver subcommand's checklist and
      branch (4) in the body-update subcommand's checklist also verify the
      fail-fast REST-error-surfacing behaviour stated in Dependencies (a
      non-zero exit code with the REST error's status and message on
      stderr). Additionally — since this is new behaviour with no bash-script
      branch to characterize — one test per supported remote-URL form (the
      four forms enumerated in Requirements), confirming each resolves to
      the correct `{owner}/{repo}` pair. 16 tests total (12 + 4).
- [ ] All skills previously invoking `skills/github/scripts/pr-base-repo.sh` and
      `skills/github/describe-pr/scripts/pr-update-body.sh` now call the
      corresponding `accelerator collaboration` subcommand, with
      `allowed-tools` updated to match, per the 0167 contract.
- [ ] The migrated scripts (`skills/github/scripts/pr-base-repo.sh`,
      `skills/github/describe-pr/scripts/pr-update-body.sh`) are removed, with
      the `_EXPECTED_GITHUB_SUITES` floor in `tasks/test/integration.py:74`
      decremented from 3 to 0 to match the removal of all three github shell
      suites (`test-pr-base-repo-scripts.sh`, `test-pr-base-repo-real-gh.sh`,
      `test-pr-update-body-scripts.sh`), coordinated in lockstep with
      work-item:0174.
- [ ] `accelerator-collaboration` passes every item of the sub-binary
      registration checklist at `tasks/README.md#registering-a-dispatched-sub-binary`.

## Assumptions

- The branch/error-path enumeration in Technical Notes (for
  `pr-base-repo.sh` and `pr-update-body.sh`) is treated as the complete
  current PR-helper behavioural surface for this migration; any
  undocumented edge-case behaviour beyond those branches is expected to
  surface via the characterization tests rather than being independently
  re-enumerated.

## Open Questions

None outstanding. The remote-URL-to-owner/repo resolution capability
identified as missing from `vcs`/`vcs-adapters` during review is scoped
into this item's own delivery (see Requirements) rather than left open.

## Dependencies

- Blocked by: none currently. Prior blockers are resolved: work-item:0166
  (shared crates, done), work-item:0167 (invocation-contract pattern, done),
  work-item:0187 (sub-binary registration surface, merged via PR #42).
- Not a blocker: work-item:0150 (github→collaboration rename, still
  `status: draft`) establishes the naming precedent this item follows, but
  this item does not depend on 0150 completing. 0150 renames the
  `skills/github/**` directory itself; this item leaves that rename to 0150
  and only migrates the two named scripts' behaviour and call sites within
  the existing directory structure.
- Coordination: siblings work-item:0195 (corpus) and work-item:0196 (design)
  register sub-binaries via the same checklist around the same time; if that
  checklist touches shared state (a central dispatch manifest or CI floor
  config) rather than being purely additive per-binary, coordinate to avoid
  merge contention. The same three siblings (0195, 0196, 0197) also each
  decrement their own shell-suite floor constant in the shared
  `tasks/test/integration.py` file around the same time — independent named
  constants, but concurrent edits to the same file still carry ordinary
  merge risk. Separately, work-item:0169 (VCS Subdomain and Hooks Migration)
  and work-item:0188 (Library-Backed VCS Adapter over gix and jj-lib) share
  this item's target crates (`vcs`/`vcs-adapters`) for their own,
  differently-scoped work; if either is still in flight when this item adds
  its remote-URL-parsing port, coordinate on the crates' port/adapter shape
  to avoid conflicting designs.
- Blocks: work-item:0174 (shell/CI-guard retirement — floor decrements from
  this item's script removals feed work-item:0174's own lockstep
  requirement).
- External: the GitHub REST API (`api.github.com`), called in-process via
  `octocrab` — this migration removes the `gh` CLI runtime dependency
  carried by the bash scripts, replacing it with `octocrab`-based GitHub REST
  API calls authenticated via the `github.token`/`github.token_cmd`
  config pairing (or `GH_TOKEN` then `GITHUB_TOKEN` env vars as a fallback,
  in that order). On
  API errors (rate-limiting, outage, or auth failure), `accelerator
  collaboration` surfaces the REST error to the caller rather than retrying
  or degrading silently: a non-zero exit code, with the REST error's status
  code and message written to stderr — the same fail-fast behaviour the
  replaced `gh` invocations exhibited. Test-time verification (see
  Acceptance Criteria)
  exercises the HTTP client via a mockable/injectable interface rather than
  live authenticated API calls in CI.
- Parent: work-item:0136 (epic).

## Technical Notes

- Source bash: `skills/github/scripts/pr-base-repo.sh`,
  `skills/github/describe-pr/scripts/pr-update-body.sh`.
- `gh` call-shape being replaced: `gh pr view <pr> --json url` →
  `GET /repos/{owner}/{repo}/pulls/{pull_number}` (base-repo resolution);
  `gh api --method PATCH repos/<repo>/pulls/<pr> --input <file>` →
  `PATCH /repos/{owner}/{repo}/pulls/{pull_number}` with a `{"body": ...}`
  JSON payload (body update). `pr-update-body.sh` also shells out to
  `pr-base-repo.sh` internally to resolve the base repo first.
- Local-repo owner/repo resolution moves from `gh`'s implicit git-remote
  inference to new `origin`-remote-URL parsing added to `vcs`/`vcs-adapters`
  as part of this item (supported URL forms and the missing-remote error
  path are enumerated in Requirements). No such capability exists in these
  crates today: `vcs`'s `RepoFacts.name` (`cli/vcs/src/lib.rs:110`) derives
  from the local directory name only, not any remote, and `vcs-adapters`'
  existing probes (`subprocess.rs`, `library.rs`) have no remote-reading
  port to extend.
- Characterization-test completion checklist (a *characterization test*
  pins down existing observable behaviour as a safety net during a
  rewrite, rather than testing new design intent), restated as the *target
  Rust behaviour* each source-script branch maps to (not the literal bash/`jq`
  implementation detail, since branches that are artefacts of the bash/`jq`
  toolchain have no analog in the `octocrab`-based rewrite and are dropped):
  `pr-base-repo.sh`'s resolver subcommand — (1) invalid CLI invocation
  (wrong/missing arguments), (2) `GET /repos/.../pulls/{pull_number}`
  request failure (network error or non-2xx response — replaces the
  script's generic `gh pr view` failure branch), (3) missing `origin` git
  remote when resolving the local repo (a `vcs`-level error, replacing the
  script's `gh`-specific "no default remote repository" remediation), (4)
  malformed/non-JSON REST response body, (5) empty/null `url` field in the
  PR response, (6) `url` field present but not matching the expected
  PR-URL shape, (7) success — 7 branches (the script's `jq`-missing branch
  is dropped; the Rust binary has no `jq` dependency). `pr-update-body.sh`'s
  body-update subcommand — (1) invalid CLI invocation (wrong/missing
  arguments), (2) missing/unreadable `--body-file` input, (3) base-repo
  resolution failure propagated in-process (replacing the script's
  subprocess exit-code propagation from calling `pr-base-repo.sh`), (4)
  `PATCH /repos/.../pulls/{pull_number}` request failure (network error or
  non-2xx response), (5) success — 5 branches (the script's two `jq`
  branches — missing `jq` and encode failure — are dropped for the same
  reason). 12 characterization tests total (7 + 5), plus 4 further tests
  (one per supported remote-URL form — see Requirements) for the new
  origin-remote parsing behaviour itself, which has no bash-script branch
  to characterize since `gh`'s remote inference was implicit rather than a
  manual parse. 16 tests total.
- New external dependency: `octocrab` (not yet in `cli/Cargo.toml`) for the
  GitHub REST client; likely needs a `cli/deny.toml` justification entry
  (registration checklist item 13).
- Auth: `github.token` / `github.token_cmd`, reusing the
  existing config pairing and precedence/shared-config `token_cmd` ban
  already implemented for `jira`/`linear` in
  `cli/config/src/catalogue.rs:124-127` and `cli/config-adapters/src/store.rs`;
  `GH_TOKEN` then `GITHUB_TOKEN` env vars as lower-precedence fallbacks (see
  Acceptance Criteria for the full four-way precedence order).
- Call sites to repoint: `skills/github/review-pr/SKILL.md`,
  `skills/github/respond-to-pr/SKILL.md` (both invoke `pr-base-repo.sh`), and
  `skills/github/describe-pr/SKILL.md` (invokes both scripts); each has an
  `allowed-tools` `Bash(...)` glob scoped to the script path that must become
  a `collaboration` subcommand glob.
- Test suites to repoint/replace: `skills/github/scripts/test-pr-base-repo-scripts.sh`
  (15 cases, PATH-stubbed `gh`), `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh`
  (23 cases, PATH-stubbed `gh`) — both need their stubbing swapped from a
  PATH `gh` stub to HTTP-level stubbing; `skills/github/scripts/test-pr-base-repo-real-gh.sh`
  (real-`gh` smoke test guarding the work-item:0071 regression) becomes moot
  once `gh --json` is no longer invoked and is a candidate for retirement
  rather than porting.
- Structural precedent: `vcs` → `vcs-cli` (package `accelerator-vcs`) and
  `work` → `work-cli` (package `accelerator-work`) are the actually-dispatched
  sub-binaries to mirror (domain crate + adapters crate + thin `-cli` crate
  with `cli.rs`/`main.rs`); `corpus`/`corpus-adapters` (0195/0179) exist as
  domain-only crates with no dispatched binary yet, so they are not yet a
  working precedent despite being the most recently touched sibling.
- Full registration checklist: `tasks/README.md:304-441`
  (`## Registering a dispatched sub-binary`); items 1, 2, 3, 4, 7, and 8 must
  land in the same change per its landing-together constraint.

## Drafting Notes

- Split out of work-item:0173 on 2026-08-05 following that item's review-1
  (verdict REVISE, scope lens): the three sub-binaries it bundled were
  functionally independent and separately deliverable.
- Domain-naming wording tightened per 0173's review-1 clarity finding: the
  github→collaboration rename is an in-progress, codebase-wide initiative, but
  this binary's own naming (`collaboration`) is settled, not open.

## References

- Split from: `meta/work/0173-remaining-subdomains-corpus-design-collaboration.md`
  (abandoned)
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Related: `meta/work/0150-rename-github-skill-group-to-collaboration.md`
  (github→collaboration rename precedent)
- ADRs: ADR-0053
