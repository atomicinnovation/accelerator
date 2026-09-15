---
type: "pr-description"
id: "123"
title: "[0278] Complete the codebase-research rename"
date: "2026-09-15T13:36:10+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0278"
parent: "work-item:0278"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/123"
pr_number: 123
tags: ["rename", "codebase-research", "doc-type", "migration"]
revision: "ca4815aa3adb2f260d21e7b2ef1df939b0f3236f"
repository: "accelerator"
last_updated: "2026-09-15T13:36:10+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0278] Complete the codebase-research rename

## Summary

Finishes the bare `research` → `codebase-research` doc-type rename by clearing
the non-enforced residue the earlier work left behind. Every compiler- or
test-enforced surface already reads `codebase-research`; what remained was
display copy, template-name consistency, typed doc-type labels, and — the
substantive piece — stripping the `Research: ` title prefix from both what the
`research-codebase` skill emits and the ~177 documents already on disk.

Stacked on `0278-topic-research-visualiser-doc-type` (PR #118).

## Changes

- **`m0010` migration** (`cli/migrate/src/migrations/m0010.rs`, registered in
  `registry.rs` + `mod.rs`): strips the `Research: ` prefix from the frontmatter
  `title:` and the pre-first-`## ` body H1 of every file under
  `paths.research_codebase`. Content-gated, quote-aware, idempotent (byte-stable
  re-apply), refuses a dirty tree and dangerous config paths.
- **Corpus rewrite**: 177 codebase-research documents under
  `meta/research/codebase/` retitled by the `m0010` run, including this work's
  own driving research document.
- **Templates stop emitting the prefix**: `templates/codebase-research.md` and
  the `research-codebase` skill drop `Research: ` from the minted `title:` and
  H1; the topic-research templates drop their own `Research Topic: ` / doc-type
  title prefixes.
- **Visualiser display copy**: frontend lifecycle label and empty-state copy
  move from "Research" to "Codebase research" (`types.ts`,
  `empty-descriptions.ts`); `template-tier.ts` stem map keys on
  `codebase-research`; server template seeds and the renamed fixture
  (`codebase-research.md`) drive the assertions.
- **Docs + typed labels**: docs-site references and typed doc-type labels across
  skills (`configure`, `init`, `extract-adrs`, `research-codebase`,
  `visualise`, `extract-work-items`) and templates name `codebase-research`.

## Context

- Work item: `work-item:0278`
- Plan: `meta/plans/2026-09-13-0278-complete-codebase-research-rename.md`
- Review: `meta/reviews/plans/2026-09-13-0278-complete-codebase-research-rename-review-1.md`
- Migration precedent: `cli/migrate/src/migrations/m0006.rs`

The `m0006` quote helpers and positional body anchoring are duplicated inline in
`m0010` rather than shared: an applied migration is immutable history, and
sharing helpers that later evolve would risk silently changing byte-stable
behaviour or shifting the public-API snapshot.

## Testing

- [x] Migration unit + integration tests (`m0010.rs`, `migration_0010.rs`):
      quote variants, `refuses` guard, block-scalar no-op, near-miss prefixes,
      positional H1 anchoring, missing-corpus `Applied`, dangerous-path refusal,
      ledger-drop idempotency.
- [x] Full registry chain (`full_registry_e2e.rs`) now applies ten migrations
      (`applied: 9; pending (no-op): 1`); ledger/preflight suites include
      `0010-…`.
- [x] Frontend + server expectations updated as the spec
      (`LibraryTemplatesIndex.test.tsx`, `LifecycleClusterView.test.tsx`,
      `templates.rs`, `api_related.rs`, `api_work_item_pattern.rs`).
- [x] Plan Phase 4/5 manual verification recorded complete (minted doc carries
      no prefix; `mise run migrate` applies then no-ops; visualiser shows the
      new labels).
- [ ] Full `mise run` suite not executed in this session — CI is the gate.

## Notes for Reviewers

`m0010` is destructive on the live repo: it rewrites ~177 uniform,
double-quoted `title: "Research: …"` corpus files, this plan's driving research
document among them, so the original titles then survive only in VCS history.
Recovery from a mid-walk failure is `jj restore meta/research/codebase/` then
re-run on a clean tree (per-file writes are atomic; the ledger advances only on
success); `ACCELERATOR_MIGRATE_FORCE` is not the recovery path here.

The three on-disk exceptions to the `Research: ` title pattern are embedded
template examples inside document bodies, which the migration never touches — it
rewrites only a document's own frontmatter and pre-`## ` H1.
