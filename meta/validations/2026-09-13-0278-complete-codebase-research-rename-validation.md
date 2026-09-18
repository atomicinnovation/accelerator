---
type: "plan-validation"
id: "2026-09-13-0278-complete-codebase-research-rename-validation"
title: "Validation Report: Complete the Codebase-Research Rename Implementation Plan"
date: "2026-09-15T13:54:36+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "partial"
target: "plan:2026-09-13-0278-complete-codebase-research-rename"
tags: ["rename", "codebase-research", "doc-type", "migration"]
last_updated: "2026-09-15T13:54:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Complete the Codebase-Research Rename

All five phases are implemented and verified on disk. `m0010` is registered and
recorded applied; the corpus rewrite is complete and correct — every
frontmatter title and body H1 is stripped, with only the two deliberate in-body
template examples surviving. The Rust, server, and frontend read-only checks
exit 0. This is the most complete of the three plans: the only outstanding
items are three low-risk manual visual confirmations and the two orthogonal
aggregate reds. It stops just short of `pass` because `mise run check` is
currently red — solely on a newly-disclosed dependency advisory, not on
anything this plan touched.

### Implementation Status

- ✅ Phase 1: template-name consistency — `configure/SKILL.md` key
  (`codebase-research`) and filename example corrected; `template-tier.ts` glyph
  key/value; server seeds + fixture rename; frontend template-index assertions.
- ✅ Phase 2: frontend display copy — `types.ts` `label: "Codebase research"`,
  `placeholder: "no codebase research yet"`; `empty-descriptions.ts` plural
  `"codebase research notes"`.
- ✅ Phase 3: stop emitting the prefix — `templates/codebase-research.md`
  `title:`/H1 are placeholder-only (no `Research:`); skill title line updated;
  regression guard in place.
- ✅ Phase 4: `m0010` migration — `m0010.rs` present, registered in `mod.rs` +
  `registry.rs`, ledger records `0010-strip-research-title-prefix` applied,
  public-API snapshot regenerated, cross-cutting ledger tests updated.
- ✅ Phase 5: typed labels + enumerations — `**Codebase research directory**`
  labels, template cross-references, template-type and doc-type enumerations.

### Automated Verification Results

- ✅ `mise run cli:check` — exit 0 (includes the full migrate suite).
- ✅ `mise run server:check` — exit 0 (renamed fixture, seed updates).
- ✅ `mise run frontend:check` — exit 0 (display-copy + template-index tests).
- ✅ Migration applied on the live repo; ledger shows `0010` (a re-run would be
  a no-op). `grep 'title: "Research: ' meta/research/codebase/` matches only two
  in-body template examples (`…0067-create-note-skill.md:162`, this plan's
  driving doc); `grep '^# Research: '` returns nothing.
- ❌ `mise run check` — exit 1, on `deny:check` `RUSTSEC-2026-0285` only. The
  aggregate `check` is red for a reason wholly outside this plan.
- ❌ `mise run test` — exit 1, on the pre-existing `work-item:0286` DUPLICATE-ID
  and the 0277 conformance count. No rename-owned test fails.

### Code Review Findings

#### Matches Plan

- Every edit site the plan enumerates is present and correct. The migration
  behaved exactly as its Migration Notes predicted: frontmatter titles and
  pre-first-`## ` H1s stripped, in-body template examples left intact,
  idempotent (ledger records it applied). The two surviving `Research:` strings
  are the documented exceptions, confirmed to be body lines — each file's own
  frontmatter title (line 4) is prefix-free.

#### Deviations from Plan

- None observed.

#### Potential Issues

- None owned by this plan. The corpus rewrite touched ~177 files as the plan
  anticipated; the working tree is clean now (the rewrite is committed).

### Manual Testing Required

Phases 4 and 5 manual criteria were already completed and are corroborated on
disk (migration applied, diff reviewed, greps clean, labels/docs updated). The
three remaining items are visual confirmations in the running visualiser:

1. The library templates view still renders the codebase-research glyph
   (Phase 1).
2. The lifecycle cluster view shows "Codebase research" as the step caption and
   "no codebase research yet" when empty; the library empty state reads
   "codebase research notes" (Phase 2).
3. Invoking `research-codebase` produces a document whose `title:` and H1 carry
   no `Research:` prefix (Phase 3).

### Recommendations

- Confirm the three visual items above, then this plan is fully done — its
  automated and data-side verification is complete and green in its own scope.
- ⚠️ The two aggregate reds are shared, orthogonal blockers, not rename defects:
  `RUSTSEC-2026-0285` (`rustls`, `cargo update -p rustls` to ≥ 0.23.45) and the
  `work-item:0286` DUPLICATE-ID (renumber one of the two `meta/work/0286-*.md`
  files). Clearing both restores a green `mise run`.
