---
type: work-item
id: "0195"
title: "accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI"
date: "2026-08-05T19:03:35+00:00"
author: Toby Clemson
producer: review-work-item
status: done
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["work-item:0173"]
relates_to: ["work-item:0172"]
tags: [rust, corpus, cli, adr, frontmatter, linkage]
last_updated: "2026-08-09T18:52:21+00:00"
last_updated_by: Toby Clemson
last_updated_note: "Recorded the golden-capture-ordering edge work-item:0172's plan requires as a precondition of its Phase 0 bash-golden capture. Also recording, retroactively, the self-validation-obligation resolution work-item:0172's Phase 8 named as its only remaining dependency on this item: `accelerator corpus frontmatter validate` shipped here as an in-process corpus_adapters library (not a kept-alive shell script), which unblocked 0172's Rust port of migration 0007 — its self_validate_structural/referential gate calls this library directly via a new MigrationContext::validate_frontmatter method rather than shelling out to scripts/validate-corpus-frontmatter.sh."
schema_version: 1
---

# 0195: accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build `accelerator-corpus`, a thin inbound CLI over the shared
`corpus`/`corpus-adapters` crates, covering ADR numbering/status, artifact
metadata derivation, corpus-frontmatter validation, and typed-linkage
extraction.

## Context

Split out of work-item:0173 (now abandoned) on 2026-08-05, per that item's
review-1 scope finding: bundling `accelerator-corpus`, `accelerator-design`, and
`accelerator-collaboration` into a single story risked partial-completion
ambiguity and an oversized PR, since the three shared no functional relationship
beyond the registration pattern. This item carries forward only the corpus
subcommand cluster and its resolved grouping rationale: the light corpus ops
are grouped into one `accelerator-corpus` binary over the shared
`corpus`/`corpus-adapters` crates (no dependency-bleed rationale applies; they
are one bounded context over `meta/`). This migration also removes the bash
3.2 floor constraint for these scripts and collapses the cross-surface
duplication (bash library / visualiser) that ADR-0045 identifies. Skill
maintainers currently invoking these bash scripts, and the 0174
shell-retirement effort (which depends on this migration completing to
decrement its floors), both benefit from this migration.

## Requirements

- `accelerator-corpus` — a thin inbound CLI over the shared `corpus`/`corpus-adapters`
  crates, structured as `accelerator corpus <noun> <verb>` with four noun
  groups:
  - `adr next-number` (from `adr-next-number.sh`) and `adr read-status` (from
    `adr-read-status.sh`) — ADR numbering/status.
  - `metadata derive` (from `artifact-derive-metadata.sh`) — artifact-metadata
    derivation.
  - `frontmatter validate` (from `validate-corpus-frontmatter.sh`) —
    corpus-frontmatter validation.
  - `linkage extract` (from `linkage-parser.sh`) — typed-linkage extraction
    from an artifact's body sections (References, Dependencies, Historical
    Context, Related Research, Source References), per ADR-0034/0038; distinct
    from frontmatter validation since it parses body prose, not frontmatter.
- Rewrite the call sites and `allowed-tools` of every skill invoking
  `skills/decisions/scripts/adr-*.sh`, `scripts/artifact-derive-metadata.sh`,
  `scripts/validate-corpus-frontmatter.sh`, and `scripts/linkage-parser.sh` to
  call the new `accelerator corpus` subcommands, following the invocation
  contract established in 0167.

## Acceptance Criteria

- [x] For each of the five subcommands (`adr next-number`, `adr read-status`,
      `metadata derive`, `frontmatter validate`, `linkage extract`):
      characterization tests snapshot the source bash script's stdout, exit
      code, and stderr (where the script writes to it) for the same inputs as
      the golden baseline — the bash script's own current output for that
      input. Where an existing bash test suite covers the subcommand, it is
      repointed at the new subcommand and must cover at least one success and
      one failure path. Where none exists, a new characterization test is
      written meeting the same bar.
- [x] All callers of `skills/decisions/scripts/adr-*.sh`,
      `scripts/artifact-derive-metadata.sh`, `scripts/validate-corpus-frontmatter.sh`,
      and `scripts/linkage-parser.sh` — skills, CI workflows, `tasks/` build
      tasks, and other scripts alike — now call the corresponding `accelerator
      corpus <noun> <verb>` subcommand, with skills' `allowed-tools` updated to
      match per the 0167 contract, or are named as an explicit, justified
      exception. The closed set is defined by a repo-wide search for each
      script's *executable* invocations at implementation time (`Bash(...)`
      calls, `allowed-tools` entries, shell `source`/exec sites) — not bare
      string mentions in documentation, ADRs, or other work items, which are
      excluded from this criterion.
- [x] The migrated `skills/decisions/scripts/*`, `artifact-derive-metadata.sh`,
      `validate-corpus-frontmatter.sh`, and `linkage-parser.sh` are removed, with
      the affected suite floors decremented in lockstep (see work-item:0174).
- [x] `accelerator-corpus` passes every item of the sub-binary registration
      checklist at `tasks/README.md#registering-a-dispatched-sub-binary`.

## Dependencies

- Blocked by: none currently. Prior blockers are resolved: work-item:0166
  (shared crates, done), work-item:0179 (corpus/corpus-adapters crates, done —
  supplies the libraries this CLI consumes), work-item:0167 (invocation-contract
  pattern, done), work-item:0187 (sub-binary registration surface, merged via
  PR #42).
- Blocks: work-item:0174 (shell/CI-guard retirement — floor decrements from
  this item's script removals feed its lockstep requirement).
- **Golden-capture ordering constraint, recorded here reciprocally per
  work-item:0172's plan (2026-08-07):** `scripts/validate-corpus-frontmatter.sh`
  must not be deleted by this item until work-item:0172's Phase 0 bash-golden
  capture commit has landed — that capture is the only oracle for its own
  migration 0007 port, and the window closes irreversibly once this script is
  gone. Check `meta/plans/2026-08-07-0172-migration-engine-subdomain.md`
  (Phase 0) for whether that capture commit has landed before removing this
  script as part of the `frontmatter validate` noun group.
- Note: the compiled work-item-id scan regex needed by `linkage-parser`/
  `validate-corpus-frontmatter` has no runtime dependency on work-item:0170.
  `config` already models `work.id_pattern`/`work.default_project_code`
  (`cli/config/src/catalogue.rs`), and `corpus-adapters` (0179, done) already
  ships the compiler (`work_item_pattern::compile_scan_regex`) and the
  `RegexScanner` port adapter — this CLI wires the two together, it does not
  build either.
- Parent: work-item:0136 (epic).

## Assumptions

- The light corpus ops share one bounded context and warrant a single binary
  (carried forward from work-item:0173).

## Technical Notes

- Source bash: `skills/decisions/scripts/adr-*.sh`,
  `scripts/artifact-derive-metadata.sh`, `scripts/validate-corpus-frontmatter.sh`,
  `scripts/linkage-parser.sh`.
- If the skill call-site count for `metadata derive` (or any noun group) turns
  out to be large, delivery may be phased across multiple PRs per noun group
  (`adr`, `metadata`, `frontmatter`, `linkage`) while remaining one story —
  this keeps individual PRs reviewable without reopening the scope question
  that led to splitting 0173. Noun groups ship independently as each PR lands
  — a mixed bash/Rust state mid-migration is acceptable; the story is done
  only once all four noun groups and their call-site migrations are complete.
- `artifact-derive-metadata` is invoked by many skills; its output contract is
  preserved as-is (no data-shape changes) — the "interface-redesign" language
  in 0173's Requirements referred only to *how* skills invoke the script (the
  call-site/`allowed-tools` rewrite in Requirements bullet 2), not to
  `artifact-derive-metadata`'s own output. Treat the migration as a faithful
  behavioural port; any output-contract change is out of scope for this item.

## Drafting Notes

- Split out of work-item:0173 on 2026-08-05 following that item's review-1
  (verdict REVISE, scope lens): the three sub-binaries it bundled were
  functionally independent and separately deliverable. This item carries the
  resolved corpus-grouping rationale from 0173's Context section unchanged
  (review-1's "Q5" label was dropped as untraceable outside 0173 — the
  resolution text itself stands alone).
- The skill call-site/`allowed-tools` rewrite requirement now has an explicit
  matching Acceptance Criterion (0173's review-1 flagged this as missing — a
  completeness/testability finding).

## References

- Split from: `meta/work/0173-remaining-subdomains-corpus-design-collaboration.md`
  (abandoned)
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Related: `meta/work/0179-corpus-crates-parsing-conventions.md` (the
  `corpus`/`corpus-adapters` crates this CLI sits on top of),
  `meta/work/0174-retire-shell-tooling-and-ci-guards.md` (floor decrements this
  item's removals feed into)
- ADRs: ADR-0034 (typed-linkage), ADR-0038 (linkage resolution bands),
  ADR-0045, ADR-0053
