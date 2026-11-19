---
type: "work-item-review"
id: "PP-886-work-item-writers-omit-the-required-tags-field-review-1"
title: "Work Item Review: Work Item Writers Omit the Required Tags Field"
date: "2026-11-18T14:58:56+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:PP-886"
work_item_id: "0333"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-11-18T15:08:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Work Item Writers Omit the Required Tags Field

**Verdict:** COMMENT

A tightly scoped, well-evidenced bug report: the reproduction is exact,
the Context settles which side is in error, and each of the three
affected write paths has its own acceptance criterion with a guard
against over-correcting the omit-when-empty fields. The remaining issues
are boundary and precision gaps — chiefly whether "no backfill needed"
covers consumer repositories, and a handful of requirements (template
order, linkage absence on batch/sync, tracker labels on import) that
lack a verifying criterion. Work item is acceptable but could be
improved — see the major finding below.

### Cross-Cutting Themes

- **"No backfill needed" is bounded only to this repository** (flagged
  by: clarity, scope) — the writers ship with the plugin, so consumer
  repositories may already hold untagged items (every sync-imported
  item, per Context); the item does not say whether repairing them is
  in scope, deferred, or deliberately left alone.
- **"In template order" is neither located nor verified** (flagged by:
  testability, clarity) — no criterion checks where `tags` sits, and
  the expected position is not stated.
- **Deferred label-to-tag mapping is unanchored** (flagged by:
  dependency, testability) — the Assumption that imports start with
  `tags: []` has no linked follow-up, and no criterion uses a labelled
  issue to prove labels are deliberately ignored.

### Findings

#### Major

- 🟡 **Clarity / Scope**: Unclear whether "meta/work/" in the
  no-backfill claim means this repository or every installation
  **Location**: Technical Notes
  The product writes into each consumer's own `meta/work/`, and Context
  says sync writes every pulled issue without the field, so untagged
  items likely exist elsewhere; the item does not state whether they are
  out of scope or how they would be handled.

#### Minor

- 🔵 **Testability**: "In template order" requirement has no verifying
  criterion
  **Location**: Acceptance Criteria
  A fix that appends `tags` at the end of the frontmatter would pass
  every criterion while breaking a stated requirement.
- 🔵 **Testability**: Linkage-absence regression check covers only
  `work create`
  **Location**: Acceptance Criteria
  Batch create and the sync importer have no criterion asserting that
  unpopulated linkage keys stay absent.
- 🔵 **Testability**: Sync criterion leaves the remote issue's labels
  unspecified
  **Location**: Acceptance Criteria
  A label-free fixture would pass without proving the Assumption that
  tracker labels are deliberately not mapped.
- 🔵 **Dependency**: Deferred label-to-tag mapping is not recorded as
  related work
  **Location**: Assumptions
  The "separate concern" has no work item or link, and would supersede
  this item's sync `tags: []` criterion when it lands.
- 🔵 **Clarity**: The fix is scoped by `render_frontmatter`, but the
  defect is located in `compose_frontmatter` and the link between them
  is never stated
  **Location**: Context
  An implementer cannot tell whether changing `compose_frontmatter`
  alone covers all three paths.
- 🔵 **Clarity**: Summary and Drafting Notes disagree on which skills
  run validation
  **Location**: Summary
  The Summary names only `create-work-item`; the Drafting Notes cite
  "three skills" without naming the sync skill or saying whether batch
  and sync paths validate.
- 🔵 **Clarity**: "Linkage" is used loosely and covers fields that are
  not lists
  **Location**: Requirements
  `parent`, `source`, and `external_id` are scalars; the omit-when-empty
  set is named four different ways and never defined once.
- 🔵 **Completeness**: Open Questions section absent
  **Location**: Open Questions
  Without the section (or an explicit "None."), a reader cannot tell
  whether questions were resolved or the section was forgotten.

#### Suggestions

- 🔵 **Testability**: Batch and sync criteria say "validates" without
  naming the check or exit code
  **Location**: Acceptance Criteria
  Only the first criterion names `accelerator corpus frontmatter
  validate` exits 0; batch also lacks a populated-tags case.
- 🔵 **Clarity**: The batch path has two names and no stated invocation
  **Location**: Summary
  "Its batch mode" versus "batch create", with no command line given.
- 🔵 **Clarity**: "In template order" does not say which template or
  where `tags` should go
  **Location**: Requirements
  State the expected position directly (after `external_id`, before
  `last_updated`).

### Strengths

- ✅ Exact reproduction: command, validation call, expected `tags: []`
  with exit 0, and actual `MISSING-BASE-FIELD` text with exit 1.
- ✅ Context states outright that the writer is the side in error,
  citing both `BASE_FIELDS` and the work-item template, closing off the
  alternative of a schema change.
- ✅ All three write paths are named with the skills that drive them,
  and each has its own acceptance criterion; one shared function
  justifies treating them as a single unit.
- ✅ Empty and populated `tags` cases are paired, and a dedicated
  criterion guards the omit-when-empty fields against an over-broad fix.
- ✅ Scope boundaries are explicit: linkage fields keep their behaviour,
  label mapping is deferred, and the SKILL.md correction follows
  directly from the behaviour change.
- ✅ Technical Notes name the existing test and the untested empty case,
  giving an obvious first red test.
- ✅ Dependencies are accurate: self-contained, with 0332 correctly
  recorded as related rather than blocking.

### Recommended Changes

1. **Bound the backfill claim** (addresses: no-backfill scope)
   State that "no backfill" covers this repository only, and say
   whether untagged items in consumer repositories are out of scope
   (users fix them, or a migration follows in a separate item).
2. **Make template order checkable** (addresses: template order
   unverified; which template)
   State the expected position (after `external_id`, before
   `last_updated`) in the Fix bullet and assert it in the first
   criterion.
3. **Tighten the batch and sync criteria** (addresses: linkage absence
   on batch/sync; sync labels; "validates" wording)
   Use a labelled remote issue as the sync precondition, assert
   `external_id` carries the key and other linkage keys are absent, and
   name `accelerator corpus frontmatter validate` exits 0 in both.
4. **Clarify the code referents** (addresses: `render_frontmatter`
   vs `compose_frontmatter`; batch naming; loose "linkage")
   Say that `render_frontmatter` delegates to `compose_frontmatter`,
   use one name for batch create, and define the omit-when-empty set
   once by name.
5. **Reconcile the validation claim** (addresses: Summary vs Drafting
   Notes)
   Name the sync skill and say which paths run validation, so the
   Summary and the priority rationale agree.
6. **Anchor the label-mapping follow-up** (addresses: deferred label
   mapping)
   Either link an existing item or note in Assumptions that none has
   been raised yet.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Clear overall — concrete reproduction, named defect
location and conflicting schema/template, consistent Given/When/Then.
Remaining ambiguities are referential: the `render_frontmatter` /
`compose_frontmatter` link, loose use of "linkage", the scope of the
no-backfill claim, and a Summary/Drafting Notes mismatch on which skills
validate.

**Strengths**:
- Exact reproduction with expected and actual results.
- Context states which side is in error and why.
- Omit-when-empty fields enumerated identically in Fix bullets and
  Acceptance Criteria.
- Assumptions pre-empt the label-mapping question.

**Findings**:
- 🟡 major (medium) — Technical Notes — Unclear whether "meta/work/" in
  the no-backfill claim means this repository or every installation.
  The plugin writes into each consumer's `meta/work/`, and Context says
  sync writes every pulled issue without the field. State which corpus
  the claim covers and how untagged items elsewhere are handled.
- 🔵 minor (medium) — Context — The fix is scoped by
  `render_frontmatter`, but the defect is located in
  `compose_frontmatter`; their relationship is never stated.
- 🔵 minor (medium) — Summary — Summary names only `create-work-item`
  as running validation; Drafting Notes cite three skills; the sync
  skill is never named.
- 🔵 minor (medium) — Requirements — "Linkage" used four ways and
  covers scalars (`parent`, `source`, `external_id`); define the
  omit-when-empty set once.
- 🔵 suggestion (low) — Summary — Batch path called both "batch mode"
  and "batch create", with no invocation shown.
- 🔵 suggestion (low) — Requirements — "In template order" names no
  template and no position.

### Completeness

**Summary**: A well-populated bug report with a four-part reproduction,
Context explaining why the writer is wrong, six criteria covering all
three paths, and complete frontmatter. The only gap is a missing Open
Questions section.

**Strengths**:
- Reproduction is fully specified.
- Context gives motivation and authority (`BASE_FIELDS`, template).
- All three paths named, each with a criterion.
- Requirements bound the fix (linkage unchanged, SKILL.md corrected).
- Assumptions and Technical Notes record the label-mapping and
  no-backfill decisions.
- Frontmatter complete and consistent with the body.

**Findings**:
- 🔵 minor (medium) — Open Questions — Section absent; 0332 includes it
  with "None.". Add it, stating "None." if nothing is open.

### Dependency

**Summary**: Self-contained writer fix with no blockers; Dependencies
are accurate and 0332 is correctly recorded as related. The one gap is
the deferred label-to-tag mapping, which is not anchored to any work
item and would supersede this item's sync criterion.

**Strengths**:
- Every affected path and its consuming skill named.
- 0332 recorded as related, not blocking.
- No-backfill note removes a hidden migration dependency.
- Fix targets existing, agreeing artefacts (schema and template).

**Findings**:
- 🔵 minor (medium) — Assumptions — Deferred label-to-tag mapping is
  not recorded as related work; link it or note it is unraised.

### Scope

**Summary**: A tightly bounded single-purpose bug fix; the three paths
are one unit via the shared function. The only boundary question is
whether "no backfill needed" holds beyond this repository.

**Strengths**:
- Every requirement serves the single purpose of aligning writer with
  schema.
- Three paths justified as one unit by the shared function.
- Explicit boundaries on linkage fields and label mapping.
- Drafting Notes close off the schema-change alternative.
- Summary, Requirements, and Acceptance Criteria consistent.
- Kind and size appropriate; standalone value.

**Findings**:
- 🔵 minor (medium) — Technical Notes — "No backfill needed" bounds
  scope only for this repository; say whether consumer repositories'
  untagged items are in scope, deferred, or left to users.

### Testability

**Summary**: Highly testable — exact reproduction and one concrete
pass/fail check per write path, plus a regression guard on linkage
fields. Gaps: template order unverified, linkage-absence check limited
to `work create`, and batch/sync criteria loosely specified.

**Strengths**:
- Complete reproduction with exact commands and outputs.
- "Every work item written through `render_frontmatter`" is bounded by
  per-path criteria.
- Empty and populated `tags` cases paired.
- Linkage-absence criterion guards against over-correction.
- Technical Notes identify the obvious first red test.

**Findings**:
- 🔵 minor (high) — Acceptance Criteria — "In template order" has no
  verifying criterion; assert `tags` position.
- 🔵 minor (medium) — Acceptance Criteria — Linkage-absence check
  covers only `work create`; extend to batch and sync.
- 🔵 minor (medium) — Acceptance Criteria — Sync criterion leaves the
  remote issue's labels unspecified; use a labelled issue.
- 🔵 suggestion (medium) — Acceptance Criteria — Batch and sync
  criteria say "validates" without the check or exit code; batch lacks
  a populated-tags case.

## Re-Review (Pass 2) — 2026-11-18T15:04:43+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity / Scope**: No-backfill claim bounded only to this
  repository — Resolved (backfill migration now in scope)
- 🔵 **Testability**: "In template order" has no verifying criterion —
  Partially resolved (position stated and checked for the no-tag
  `work create` case only)
- 🔵 **Testability**: Linkage-absence check covers only `work create` —
  Resolved
- 🔵 **Testability**: Sync criterion leaves labels unspecified — Resolved
- 🔵 **Dependency**: Deferred label-to-tag mapping not recorded —
  Partially resolved (noted as unraised in Assumptions; no "Informs:"
  entry in Dependencies)
- 🔵 **Clarity**: `render_frontmatter` / `compose_frontmatter` link
  unstated — Resolved
- 🔵 **Clarity**: Summary and Drafting Notes disagree on validating
  skills — Resolved
- 🔵 **Clarity**: "Linkage" used loosely — Resolved (omit-when-empty
  fields defined once)
- 🔵 **Completeness**: Open Questions section absent — Still present
  (deliberately omitted by the author)
- 🔵 **Testability**: Batch and sync criteria say "validates" —
  Resolved
- 🔵 **Clarity**: Batch path has two names — Resolved
- 🔵 **Clarity**: "In template order" names no template — Resolved

### New Issues Introduced

- 🟡 **Testability**: Migration criterion does not require the edited
  item to be otherwise unchanged — a migration that re-serialises the
  whole frontmatter would pass.
- 🔵 **Testability**: Migration behaviour unspecified for items lacking
  `last_updated`, block-style `tags` lists, or `tags:` lines in the body.
- 🔵 **Testability**: Batch and sync preconditions do not say the
  fixture has no linkage fields.
- 🔵 **Testability**: Position of `tags` not checked when
  omit-when-empty fields or populated tags are present.
- 🔵 **Dependency**: Migration's ordering after m0007 and its sequence
  number against other in-flight migrations not captured.
- 🔵 **Clarity**: "Untagged" and "non-empty-only" drift from the
  defined vocabulary.
- 🔵 **Clarity**: SKILL.md criterion requires a positive statement the
  Requirement does not, using internal vocabulary.
- 🔵 **Testability** (suggestion): Populated omit-when-empty fields not
  checked as still written.
- 🔵 **Clarity** (suggestion): `create.rs:77-78` could refer to either
  `create.rs`.
- 🔵 **Scope** (suggestion): Drafting Note justifies bundling the
  migration by release timing rather than shared goal.
- 🔵 **Completeness** (suggestion): How 0332 surfaced the defect is
  not explained.

### Assessment

Every first-pass major and most minors are resolved; widening the item
to include the backfill migration introduced one major gap (the
migration must preserve the rest of each edited file byte-for-byte)
and a cluster of minor migration edge cases. The item is acceptable
for planning, but tightening the migration criterion first is cheap
and protects users' repositories.

## Verdict Change — 2026-11-18T15:08:36+00:00

**Verdict:** APPROVE

The pass-2 findings were addressed in the work item and the author
marked the review approved without a further pass.
