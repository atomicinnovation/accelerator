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
last_updated: "2026-08-06T01:13:07+00:00"
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
separately in work-item:0150. This replaces bash that skills authors
currently shell out to with a typed, testable Rust implementation.

## Context

Split out of work-item:0173 (now abandoned) on 2026-08-05, per that item's
review-1 scope finding: bundling `accelerator-corpus`, `accelerator-design`, and
`accelerator-collaboration` into a single story risked partial-completion
ambiguity and an oversized PR. The PR helpers stay separate as
`accelerator-collaboration` per the github→collaboration rename (precedent:
work-item:0150) — the domain is named `collaboration`, not `github`. This
binary's own naming (`collaboration`) is fixed, not open (0173's review-1
flagged the "open" wording as ambiguous on this point); this replaces
untestable bash bound by the project's bash 3.2 floor, consistent with the
broader shell-to-Rust migration epic (work-item:0136). The wider
github→collaboration *directory* rename (renaming `skills/github/**` itself)
is a separate, still-in-progress initiative tracked by work-item:0150 —
this item does not depend on it and does not rename the skill directory
(see Dependencies).

## Requirements

- `accelerator-collaboration` — the PR helpers (`pr-base-repo`,
  `pr-update-body`); shells to `gh`. Domain named `collaboration`, not `github`.
- Rewrite the call sites and `allowed-tools` of every skill invoking
  `skills/github/scripts/pr-base-repo.sh` or
  `skills/github/describe-pr/scripts/pr-update-body.sh` to call the new
  `accelerator collaboration` subcommands, following the invocation contract
  established in 0167.
- Remove the migrated `skills/github/scripts/pr-base-repo.sh` and
  `skills/github/describe-pr/scripts/pr-update-body.sh` scripts, with the
  affected suite floors decremented in lockstep (see work-item:0174).
- `accelerator-collaboration` satisfies every item of the sub-binary
  registration checklist at
  `tasks/README.md#registering-a-dispatched-sub-binary`.

## Acceptance Criteria

- [ ] `accelerator collaboration …` reproduces the PR-helper behaviours,
      shelling to `gh` with the same invocations as the current bash (enumerate
      the specific `gh` sub-commands/flags per helper during implementation),
      verified via repointed suites (existing suites redirected to invoke
      `accelerator collaboration` instead of the bash script) supplemented by
      characterization tests for any current `gh` call-shape behaviour not
      already covered by those suites.
- [ ] All skills previously invoking `skills/github/scripts/pr-base-repo.sh` and
      `skills/github/describe-pr/scripts/pr-update-body.sh` now call the
      corresponding `accelerator collaboration` subcommand, with
      `allowed-tools` updated to match, per the 0167 contract.
- [ ] The migrated scripts (`skills/github/scripts/pr-base-repo.sh`,
      `skills/github/describe-pr/scripts/pr-update-body.sh`) are removed, with
      the affected suite floors decremented in lockstep (see work-item:0174).
- [ ] `accelerator-collaboration` passes every item of the sub-binary
      registration checklist at `tasks/README.md#registering-a-dispatched-sub-binary`.

## Assumptions

- The two named source scripts (`pr-base-repo.sh`, `pr-update-body.sh`)
  represent the complete current PR-helper behavioural surface for this
  migration; any undocumented edge-case behaviour they carry is expected to
  surface via the characterization tests above rather than being
  independently enumerated up front.

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
  merge contention.
- Blocks: work-item:0174 (shell/CI-guard retirement — floor decrements from
  this item's script removals feed its lockstep requirement).
- External: the `gh` CLI must be installed and authenticated for
  `accelerator collaboration`'s PR-helper subcommands at runtime
  (pre-existing coupling, carried forward from the bash scripts). Test-time
  verification (see Acceptance Criteria) should exercise `gh` invocation via
  a mockable/injectable interface rather than requiring live authenticated
  `gh` calls in CI.
- Parent: work-item:0136 (epic).

## Technical Notes

- Source bash: `skills/github/scripts/pr-base-repo.sh`,
  `skills/github/describe-pr/scripts/pr-update-body.sh`.

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
