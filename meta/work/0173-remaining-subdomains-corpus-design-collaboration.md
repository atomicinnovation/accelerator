---
type: work-item
id: "0173"
title: "Remaining Subdomains: corpus, design, collaboration"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: abandoned
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0195", "work-item:0196", "work-item:0197"]
tags: [rust, corpus, design, collaboration, subdomains]
last_updated: "2026-08-05T19:03:35+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-194"
---

# 0173: Remaining Subdomains: corpus, design, collaboration

**Kind**: Story
**Status**: Abandoned
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Migrate the remaining script clusters into their subdomain binaries:
`accelerator-corpus` (the grouped light corpus ops), `accelerator-design` (design
inventory/gap tooling), and `accelerator-collaboration` (the PR helpers, under the
github→collaboration domain rename).

## Context

These are the lower-complexity clusters left after the foundational layers and the
larger subdomains. Resolved Q5: the light corpus ops are grouped into one
`accelerator-corpus` binary over the shared `corpus`/`corpus-adapters` crates (no
dependency-bleed rationale applies; they are one bounded context over `meta/`);
the PR helpers stay separate as `accelerator-collaboration` per the open
github→collaboration rename. May be split into three separate work items if finer
granularity is wanted.

## Requirements

- `accelerator-corpus` — a thin inbound CLI over the shared `corpus`/`corpus-adapters`
  crates, invoked as `accelerator corpus <sub>`: ADR numbering/status
  (`adr-next-number`, `adr-read-status`), artifact-metadata derivation
  (`artifact-derive-metadata`), corpus-frontmatter validation
  (`validate-corpus-frontmatter`), and typed-linkage queries (`linkage-parser`).
- `accelerator-design` — design inventory/gap tooling
  (`inventory-design/scripts/*`, `analyse-design-gaps/scripts/*`); the Playwright
  executor (`run.sh`) stays a thin wrapper the binary execs, or is invoked as
  today, per the ADR-0048 thin-wrapper exception.
- `accelerator-collaboration` — the PR helpers (`pr-base-repo`, `pr-update-body`);
  shells to `gh`. Domain named `collaboration`, not `github`.
- Apply the Q7 interface-redesign principle and rewrite the relevant skills' call
  sites + `allowed-tools` following the contract established in 0167.

## Acceptance Criteria

- [ ] `accelerator corpus …` reproduces ADR numbering/status, artifact metadata,
      frontmatter validation, and linkage queries, verified against repointed
      suites (and characterization tests where none exist).
- [ ] `accelerator design …` reproduces the inventory/gap behaviours; the Playwright
      executor still launches correctly.
- [ ] `accelerator collaboration …` reproduces the PR-helper behaviours and shells
      to `gh` as before.
- [ ] The migrated `skills/decisions/scripts/*`, `skills/design/**`,
      `skills/github/**`, `artifact-derive-metadata.sh`,
      `validate-corpus-frontmatter.sh`, and `linkage-parser.sh` are removed, with
      the affected suite floors decremented in lockstep.

## Open Questions

- Whether the Playwright executor stays shell (thin-wrapper exception) or is folded
  into `accelerator-design` — decided during implementation.

## Dependencies

- Blocked by: none currently. Prior blockers are resolved: work-item:0166 (shared
  crates, done), work-item:0167 (invocation-contract pattern, done), work-item:0187
  (sub-binary registration surface, merged via PR #42).
- Registration for each of the three sub-binaries (corpus, design, collaboration)
  should follow the checklist 0187 added at
  `tasks/README.md#registering-a-dispatched-sub-binary` — applied three times over,
  once per subdomain.
- Parent: work-item:0136 (epic).

## Assumptions

- The light corpus ops share one bounded context and warrant a single binary
  (resolved Q5); design/collaboration warrant their own.

## Technical Notes

- Source bash: `skills/decisions/scripts/adr-*.sh`, `scripts/artifact-derive-metadata.sh`,
  `scripts/validate-corpus-frontmatter.sh`, `scripts/linkage-parser.sh`,
  `skills/design/**/scripts/*`, `skills/github/scripts/pr-base-repo.sh`,
  `skills/github/describe-pr/scripts/pr-update-body.sh`.
- `artifact-derive-metadata` is invoked by many skills; treat its output contract
  carefully under the Q7 redesign.

## Drafting Notes

- Treated as the Phase 10 story; grouped per the user's selection, carrying the
  resolved Q5 `accelerator-corpus` grouping and github→collaboration rename.
- 2026-08-05: dependencies refreshed during interactive enrichment — all three
  prior blockers (0166, 0167, 0187) are now done/merged, so `blocked_by` was
  cleared and the Dependencies section rewritten. The stale open question about
  splitting into three items was removed (already resolved in Context).
  Acceptance criteria, requirements, and kind were not reviewed this round —
  status intentionally left at `draft` pending that pass.
- 2026-08-05: review-1 (`meta/reviews/work/0173-remaining-subdomains-corpus-design-collaboration-review-1.md`,
  verdict REVISE) found the story bundled three functionally independent
  efforts and was undersized for a `story` kind. Per that finding, this item is
  **abandoned** in favour of a split into work-item:0195 (`accelerator-corpus`),
  work-item:0196 (`accelerator-design`), and work-item:0197
  (`accelerator-collaboration`), each parented directly under work-item:0136.
  The review's other findings (missing AC for the skill-rewrite requirement,
  Dependencies gaps, clarity ambiguities) were carried forward and addressed in
  the three successor items rather than fixed here.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Related: `meta/work/0150-rename-github-skill-group-to-collaboration.md`
  (github→collaboration rename precedent), `meta/work/0179-corpus-crates-parsing-conventions.md`
  (the `corpus`/`corpus-adapters` crates this sub-binary sits on top of)
- Superseded by: `meta/work/0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md`,
  `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`,
  `meta/work/0197-accelerator-collaboration-pr-helper-cli.md`
- ADRs: ADR-0045, ADR-0048, ADR-0053
