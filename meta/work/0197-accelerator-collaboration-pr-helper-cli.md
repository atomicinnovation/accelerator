---
type: work-item
id: "0197"
title: "accelerator-collaboration: PR Helper CLI"
date: 2026-08-05T19:03:35+00:00
author: Toby Clemson
producer: review-work-item
status: done
kind: story
priority: medium
parent: work-item:0136
derived_from: [work-item:0173]
tags: [rust, collaboration, cli, github, gh]
last_updated: 2026-08-08T16:30:32+00:00
last_updated_by: Toby Clemson
schema_version: 1
---

# 0197: accelerator-collaboration: PR Helper CLI

**Kind**: Story
**Status**: Done
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
  Base-repo resolution replicates `gh`'s own cross-fork-safe default: the
  `origin` remote's owner/repo is looked up via `GET /repos/{owner}/{repo}`,
  and if that repository has a `parent` (i.e. `origin` is a fork), the
  parent's owner/repo is used as the base instead — never the response's
  echoed-back `url`/self-link fields, which cannot carry base-repo
  information distinct from what was already queried (see Technical Notes).
  Authenticates via `GH_TOKEN` then `GITHUB_TOKEN` (in that order, matching
  `gh`'s own documented env-var precedence), ahead of the `github.token`/
  `github.token_cmd` config pairing (the same mechanism already used by
  `jira`/`linear`) as a fallback — env-first, matching `jira`/`linear`'s own
  credential-resolution precedent, so an ambient env var reliably escapes a
  stale or over-broad on-filesystem config value.
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
      by (1) parsing the `origin` git remote's URL via new `vcs`/
      `vcs-adapters` parsing logic (see Requirements — this capability does
      not exist today; supported URL forms and the missing-remote error
      path are enumerated there) to get a candidate owner/repo, rather than
      `gh`'s implicit git-remote inference; (2) calling
      `GET /repos/{owner}/{repo}` against that candidate and checking its
      `parent` field — if present (the candidate is a fork), the parent's
      owner/repo becomes the base, replicating `gh`'s own default
      fork-to-parent resolution (replacing `gh pr view <pr> --json url`,
      whose cross-fork-safety comes from this same `gh`-internal
      resolution, not from the field it happens to read back); and (3)
      confirming the PR exists at the resolved base via
      `GET /repos/{base_owner}/{base_repo}/pulls/{pull_number}` (a 404 here
      means the PR is not at the resolved base — e.g. it was opened
      directly against the fork rather than upstream — and is surfaced as a
      failure rather than a silently wrong repo). Updating a PR's body via
      `PATCH /repos/{owner}/{repo}/pulls/{pull_number}` with a JSON
      `{"body": ...}` payload (replacing `gh api --method PATCH ...
      --input <file>`) is unchanged.
- [ ] **Authentication**: Authenticates via the existing `token`/`token_cmd`
      config pairing (`github.token` / `github.token_cmd`), following the
      same resolution precedence and shared-config `token_cmd` ban (`token_cmd`
      may not be set in the checked-in/shared config file, only in local
      overrides) already implemented for `jira.token`/`jira.token_cmd` and
      `linear.token`/`linear.token_cmd` in `cli/config/src/catalogue.rs`.
      Precedence, highest first: the `GH_TOKEN` env var, then the
      `GITHUB_TOKEN` env var (`GH_TOKEN` over `GITHUB_TOKEN` matches `gh`'s
      own documented env-var precedence), then the `github.token` config
      value, then `github.token_cmd` output — env-first, matching
      `jira-auth.sh`/`linear-auth.sh`'s own precedence order, so an ambient
      env var reliably escapes a stale or over-broad on-filesystem config
      value; no dependency on the `gh` CLI being installed or authenticated
      locally. This item also adds the
      Rust-native equivalent of `jira-auth.sh`/`linear-auth.sh`'s personal
      config-file permission check (`config.local.md` must be mode 0600 or
      stricter, never a symlink, to be read at all) — the first Rust
      credential-resolution path in this codebase, so this protection does
      not yet exist in Rust anywhere. Encapsulated inside `config-adapters`
      (`FileConfigStore`) rather than `collaboration`-specific, so every
      `Level::Personal` config read benefits, not just `github.token`; no
      bypass gate is needed since `config-adapters` already clamps every
      personal-level write to 0600, so the check only ever trips on
      external tampering.
- [ ] **Verification strategy**: Verified by the new Rust characterization
      tests plus `mise run test:integration:skill-invocation`, which
      together supersede the two bash suites' PATH-`gh`-stub-based
      coverage rather than repointing those suites to HTTP-level stubbing
      from bash (see Technical Notes for why deletion, not repointing, is
      the deliberate choice here). One dedicated characterization test per
      branch
      enumerated in Technical Notes for `pr-base-repo.sh`'s resolver
      subcommand (8 branches — see Technical Notes for why this is 8, not
      7: the fork-aware resolution design adds a repository-metadata lookup
      step with its own failure/malformed-response branches, and splits
      success into a non-fork and a fork-resolved case) and
      `pr-update-body.sh`'s body-update subcommand (5 branches) — 13
      characterization tests, restated as target Rust behaviour rather than
      the source bash/`jq` implementation detail (so branches with no Rust
      analog, e.g. `jq`-missing, are excluded from the count rather than
      tested vacuously), written regardless of whether a deleted bash
      suite case already exercised the same branch. The resolver subcommand's
      repository-metadata-lookup-failure and PR-existence-check-failure
      branches, and the body-update subcommand's branch (4), also verify
      the fail-fast REST-error-surfacing behaviour stated in Dependencies (a
      non-zero exit code with the REST error's status and message on
      stderr). Additionally — since this is new behaviour with no bash-script
      branch to characterize — one test per supported remote-URL form (the
      four forms enumerated in Requirements), confirming each resolves to
      the correct `{owner}/{repo}` pair. 17 tests total (13 + 4).
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
  API calls authenticated via `GH_TOKEN` then `GITHUB_TOKEN` env vars, ahead
  of the `github.token`/`github.token_cmd` config pairing as a fallback. On
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
  a two-step resolution, `GET /repos/{owner}/{repo}` (repository metadata,
  checked for a `parent`) followed by
  `GET /repos/{base_owner}/{base_repo}/pulls/{pull_number}` (PR-existence
  confirmation at the resolved base) — see below for why this replaces a
  single call; `gh api --method PATCH repos/<repo>/pulls/<pr> --input
  <file>` → `PATCH /repos/{owner}/{repo}/pulls/{pull_number}` with a
  `{"body": ...}` JSON payload (body update). `pr-update-body.sh` also
  shells out to `pr-base-repo.sh` internally to resolve the base repo
  first.
- Local-repo owner/repo resolution moves from `gh`'s implicit git-remote
  inference to two new capabilities added as part of this item: (1)
  `origin`-remote-URL parsing added to `vcs`/`vcs-adapters` (supported URL
  forms and the missing-remote error path are enumerated in Requirements)
  — no such capability exists in these crates today: `vcs`'s
  `RepoFacts.name` (`cli/vcs/src/lib.rs:110`) derives from the local
  directory name only, not any remote, and `vcs-adapters`' existing probes
  (`subprocess.rs`, `library.rs`) have no remote-reading port to extend;
  and (2) a repository-metadata lookup in `collaboration`/
  `collaboration-adapters` that checks the parsed candidate for a `parent`
  repository, replicating `gh`'s own default fork-to-parent resolution.
  Both are needed together: `origin`-URL parsing alone only recovers the
  *local checkout's own* repo, which is the fork itself for a contributor
  working from a fork — exactly the case `gh repo view` gets wrong and
  `gh pr view`'s implicit resolution gets right (see below).
- **Why a single `GET .../pulls/{pull_number}` call cannot resolve the base
  repo on its own.** An earlier draft of this item's design derived the
  base owner/repo from that endpoint's response `url` field. This is
  incorrect: `GET /repos/{owner}/{repo}/pulls/{pull_number}` only ever
  returns PRs scoped to the exact `{owner}/{repo}` supplied in the request
  — the response cannot reveal a *different* repo than what was queried,
  regardless of which field is read (`url`, or `base.repo.full_name`), so
  parsing owner/repo back out of it is circular. `gh pr view <pr> --json
  url`'s actual cross-fork-safety comes from `gh`'s own pre-call
  resolution (it targets the parent repo by default when the local
  checkout is a fork), not from anything in the response. This item's
  design must therefore resolve the fork-parent relationship itself,
  before making any PR-number-scoped call — hence the two-step
  `GET /repos/{owner}/{repo}` (parent check) then
  `GET /repos/{base_owner}/{base_repo}/pulls/{pull_number}` (existence
  confirmation) shape above.
- Characterization-test completion checklist (a *characterization test*
  pins down existing observable behaviour as a safety net during a
  rewrite, rather than testing new design intent), restated as the *target
  Rust behaviour* each source-script branch maps to (not the literal bash/`jq`
  implementation detail, since branches that are artefacts of the bash/`jq`
  toolchain have no analog in the `octocrab`-based rewrite and are dropped).
  `pr-base-repo.sh`'s resolver subcommand's branches are restated against
  the fork-aware two-call design (see above) rather than the single-call
  design an earlier draft of this item assumed — (1) invalid CLI invocation
  (wrong/missing arguments), (2) missing `origin` git remote when resolving
  the local repo (a `vcs`-level error, resolved before any network call,
  replacing the script's `gh`-specific "no default remote repository"
  remediation), (3) `GET /repos/{owner}/{repo}` repository-metadata lookup
  request failure (network error or non-2xx response), (4) malformed/
  non-JSON repository-metadata response body, (5) repository-metadata
  response indicates a fork (`parent` present) but the parent's owner/name
  fields are missing or malformed, (6) `GET /repos/{base_owner}/
  {base_repo}/pulls/{pull_number}` PR-existence-check request failure
  (network error or non-2xx response, including a 404 when the resolved
  base does not actually hold this PR — replaces the script's generic
  `gh pr view` failure branch), (7) success with no `parent` (the `origin`
  repo is its own base), (8) success with a `parent` (the `origin` repo is
  a fork; its parent is the base) — 8 branches (the script's `jq`-missing
  branch is dropped; the Rust binary has no `jq` dependency). This
  supersedes an earlier version of this checklist that assumed a single
  `GET .../pulls/{pull_number}` call and derived owner/repo from its `url`
  field — see the "why a single call cannot resolve the base repo" note
  above for why that design was replaced.
  `pr-update-body.sh`'s body-update subcommand — (1) invalid CLI invocation
  (wrong/missing arguments), (2) missing/unreadable `--body-file` input, (3)
  base-repo resolution failure propagated in-process (replacing the
  script's subprocess exit-code propagation from calling `pr-base-repo.sh`;
  covers any of resolver branches (2)-(6) above), (4)
  `PATCH /repos/.../pulls/{pull_number}` request failure (network error or
  non-2xx response), (5) success — 5 branches (the script's two `jq`
  branches — missing `jq` and encode failure — are dropped for the same
  reason). 13 characterization tests total (8 + 5), plus 4 further tests
  (one per supported remote-URL form — see Requirements) for the new
  origin-remote parsing behaviour itself, which has no bash-script branch
  to characterize since `gh`'s remote inference was implicit rather than a
  manual parse. 17 tests total.
- New external dependency: `octocrab` (not yet in `cli/Cargo.toml`) for the
  GitHub REST client; likely needs a `cli/deny.toml` justification entry
  (registration checklist item 13).
- Auth: `github.token` / `github.token_cmd`, reusing the
  existing config pairing and shared-config `token_cmd` ban already
  implemented for `jira`/`linear` in
  `cli/config/src/catalogue.rs:124-127` and `cli/config-adapters/src/store.rs`;
  `GH_TOKEN` then `GITHUB_TOKEN` env vars take precedence over both config
  keys (see Acceptance Criteria for the full four-way precedence order).
- Call sites to repoint: `skills/github/review-pr/SKILL.md`,
  `skills/github/respond-to-pr/SKILL.md` (both invoke `pr-base-repo.sh`), and
  `skills/github/describe-pr/SKILL.md` (invokes both scripts); each has an
  `allowed-tools` `Bash(...)` glob scoped to the script path that must become
  a `collaboration` subcommand glob.
- Test suites superseded, not repointed:
  `skills/github/scripts/test-pr-base-repo-scripts.sh` (15 cases,
  PATH-stubbed `gh`) and
  `skills/github/describe-pr/scripts/test-pr-update-body-scripts.sh` (23
  cases, PATH-stubbed `gh`) are deleted rather than repointed to
  HTTP-level stubbing from bash. Reworking a bash suite to stand up and
  assert against a mock HTTP server is disproportionate effort compared
  to the Rust-native mock-server tests `collaboration-adapters` already
  builds (Phase 4), and largely redundant with them: the 38 old cases
  exercise the same underlying branches the 17 new Rust characterization
  tests cover, and a meaningful share of the 38 are bash/regex artifacts
  (e.g. exhaustively probing malformed-URL shapes against a hand-rolled
  regex) that collapse into a single Rust branch once the parser is a
  typed function rather than a regex — porting them case-for-case would
  test the old implementation's incidental structure, not new behaviour.
  `skills/github/scripts/test-pr-base-repo-real-gh.sh` (real-`gh` smoke
  test guarding the work-item:0071 regression) becomes moot once `gh
  --json` is no longer invoked and is a candidate for retirement rather
  than porting.
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
