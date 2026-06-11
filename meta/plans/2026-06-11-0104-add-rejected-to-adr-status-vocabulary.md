---
type: plan
id: "2026-06-11-0104-add-rejected-to-adr-status-vocabulary"
title: "Add rejected to the ADR Status Vocabulary Implementation Plan"
date: "2026-06-11T13:15:24+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0104"
parent: "work-item:0104"
derived_from: ["codebase-research:2026-06-11-0104-add-rejected-to-adr-status-vocabulary"]
relates_to: ["work-item:0103"]
tags: [frontmatter, schema, adr, status, validator]
revision: "f7d348e72248cf66d70955c7a90e5d51297ce9fd"
repository: "build-system"
last_updated: "2026-06-11T13:42:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Add rejected to the ADR Status Vocabulary Implementation Plan

## Overview

Add `rejected` to the `adr` `status_vocab` in `scripts/templates-schema.tsv`
so the unified-schema corpus validator accepts an ADR carrying
`status: rejected`. The producer (`review-adr`) already persists
`status: rejected` + `rejected_reason` on the `proposed → rejected` transition,
exactly as accepted ADR-0031 sanctions — but the schema source omits `rejected`,
so a rejected ADR persisted by `review-adr` would fail
`validate-corpus-frontmatter.sh` with `BAD-STATUS`. This plan closes that
schema-source divergence and discharges the `skip_test` that work item 0103
deliberately left keyed to this work item's id.

## Current State Analysis

The `adr` status vocabulary has a single authoritative definition and a small,
well-mapped fan-out of read-sites and re-encodings:

- **Schema source** — `scripts/templates-schema.tsv:6`, column 5
  (`status_vocab`), currently reads `proposed | accepted | superseded | deprecated`.
  `rejected` is absent. This is the one cell to edit.
- **Data-driven read-sites (auto-propagate, no edit)**:
  - `scripts/validate-corpus-frontmatter.sh` loads the cell into `SCHEMA_STATUS`
    (`:51`) and the `BAD-STATUS` path (`:315-330`) splits it on `|` and trims —
    no hardcoded adr list.
  - `scripts/test-validate-corpus-frontmatter.sh` per-type valid-fixture loop
    (`:37-44`) walks every TSV row via `emit_valid`. **Caveat**: `emit_valid`
    pins `status` to the *first* vocab token (`frontmatter-fixtures.sh:40`,
    `cut -d'|' -f1`), so this loop emits `status: proposed` for `adr` and never
    exercises `rejected`.
  - `scripts/test-skill-frontmatter-conformance.sh:341-342` reads `ADR_VOCAB`
    from `SCHEMA_STATUS[$ADR_IDX]`.
- **Coupled re-encodings (must be hand-edited in lockstep)**:
  - `templates/adr.md:8` — the `status:` line's trailing comment re-encodes the
    vocab; `test-template-frontmatter.sh:237` does `grep -qF -- "$status_vocab"`
    against that line (fixed-string substring of the TSV cell, ` | ` separators
    included). The templates suite reds until this comment is synced.
  - `scripts/test-skill-frontmatter-conformance.sh:346-351` — the `rejected`
    `skip_test` branch, keyed to work item `0104` (comment + skip reason name it),
    runs **no** assertion (`test-helpers.sh:304-309`). The producer-prose oracle
    `extract_review_adr_targets` (`:141-144`) already discovers `rejected` from
    `review-adr`'s prose, so the divergence is "prose says it, vocab doesn't".
- **Independent re-encodings that already carry `rejected` (verify only)**:
  - `skills/visualisation/visualise/frontend/src/api/status-variant.ts:7` maps
    `rejected → red`; tests at `StatusBadge.test.tsx:50` and
    `status-variant.test.ts:29`.
  - `skills/decisions/scripts/adr-read-status.sh:51` lists `rejected` in a
    docstring/error enum only; it reads/echoes status, never validates.

### Key Discoveries:

- The change is sanctioned, not novel: ADR-0031 (accepted) adopts
  `proposed, accepted, rejected, superseded, deprecated`; ADR-0042 (the 0070
  vocab-reconciliation ADR) never mentions `rejected` and never touches the
  `adr` type — its absence from the TSV is an oversight, not a deliberate
  exclusion.
- `emit_valid`'s first-token rule (`frontmatter-fixtures.sh:40`) means widening a
  vocab never auto-tests the new value via the generic per-type loop — a
  dedicated accept-fixture with an explicit `status: rejected` is required.
- The conformance suite's `skip_test` is unconditional on `tgt = rejected`
  (`test-skill-frontmatter-conformance.sh:346-350`): it skips regardless of the
  TSV contents, so adding `rejected` to the TSV does **not** by itself convert it
  to a live assertion — the branch must be removed by hand.
- Test reachability is guarded: `test:integration:config` glob-discovers all
  three relevant scripts with a 16-suite count floor and a named-presence guard
  for `test-skill-frontmatter-conformance.sh` (`tasks/test/integration.py:14,21,49-64`);
  `test:unit:templates` runs `test-template-frontmatter.sh` from a fixed list
  (`tasks/test/unit.py:34-50`).

## Desired End State

`templates-schema.tsv`'s `adr` row reads
`proposed | accepted | rejected | superseded | deprecated`; the `templates/adr.md`
comment matches it verbatim; the corpus validator accepts a `status: rejected`
ADR and a fixture proves it; and the `review-adr` conformance axis runs a live
`assert_check` + `assert_accepts` for `rejected` instead of a `skip_test`.
Verified by `mise run test:integration:config` and `mise run test:unit:templates`
both green **with the new rejected coverage present**.

## What We're NOT Doing

- **Not** changing how `rejected` ADRs surface to readers — filtering rejected
  ADRs out of active-decision listings, or giving them a distinct visual
  treatment, is an open question explicitly out of scope. The visualiser already
  renders `rejected → red`; this plan only makes a rejected ADR schema-valid.
- **Not** editing the independent re-encodings that already carry `rejected`
  (`status-variant.ts`, `adr-read-status.sh`) — verify only. **Accepted latent
  coupling**: of the vocab's four re-encodings, only the `templates/adr.md`
  comment is mechanically guarded against the TSV (via
  `test-template-frontmatter.sh`'s `grep -qF`). `status-variant.ts` and
  `adr-read-status.sh` are maintained independently in separate skill trees with
  no cross-check, so a *future* vocab edit could silently leave them stale with
  no suite going red. This is out of scope here (both already carry `rejected`),
  but it is a standing drift hazard worth a follow-up — e.g. a conformance
  assertion that parses the adr `status_vocab` from the TSV and checks every
  token is present in both re-encodings.
- **Not** editing ADR-0033 or ADR-0042 (accepted/immutable). The editable schema
  source is the TSV.
- **Not** touching any other TSV row, the validator logic, or the conformance
  guard's structure beyond removing the single deferred `skip_test` branch.

## Implementation Approach

Two phases, each green-to-green and independently mergeable, split along the
natural seam between the **source-of-truth fix** (Phase 1) and the **coverage
lock-in** that proves it (Phase 2):

- **Phase 1** edits the schema source (TSV cell) and its one coupled,
  verbatim-checked re-encoding (the template comment). After Phase 1 the corpus
  validator already accepts a rejected ADR, all suites are green, and the
  `skip_test` continues to skip harmlessly. This phase is independently valuable
  and mergeable on its own — it delivers the actual fix.
- **Phase 2** adds the two test artifacts that *prove* the new value is exercised
  — a corpus-validator accept-fixture and the flip of the conformance `skip_test`
  to a live assertion — discharging the 0103 deferral. Phase 2 has a **hard
  ordering precondition**: it must not merge unless Phase 1's TSV change is
  already on the integration branch. Applied without Phase 1 present, the live
  `assert_check status_in_vocab rejected` and the corpus accept-fixture would
  correctly red on non-member / `BAD-STATUS` — desired load-bearing behaviour
  (the Phase 2 reversal check confirms exactly this), but a CI-blocking surprise
  if the merge order is reversed. Given Phase 2 alone has no value, prefer
  landing the two phases together unless there is a specific reason to split.

**TDD applied per phase**: because the production change is a data edit (not
logic), the red-then-green cycle is run *within* each phase locally — confirm the
relevant suite/assertion fails before the edit and passes after — rather than as
a separate red commit, which would leave a phase boundary red and break
independent mergeability. The conformance suite's producer-prose oracle is
already wired to expect `rejected`; Phase 2 simply removes the deferral that
suppresses the assertion.

## Phase 1: Add `rejected` to the schema source and sync the template comment

### Overview

Make `rejected` a member of the `adr` `status_vocab` and bring the coupled
`templates/adr.md` comment into verbatim sync, leaving every suite green.

### Changes Required:

#### 1. Schema source — the `adr` `status_vocab` cell

**File**: `scripts/templates-schema.tsv`
**Changes**: On the `adr` row (line 6), column 5, insert `rejected` after
`accepted` so the cell reads `proposed | accepted | rejected | superseded | deprecated`.
Preserve the ` | ` (space-pipe-space) separators and the tab-delimited column
structure exactly — the validator trims tokens but the template test matches the
cell as a fixed-string substring.

```
adr.md	adr	no	decision_makers	proposed | accepted | rejected | superseded | deprecated	adr_id	parent supersedes relates_to
```

#### 2. Coupled template comment (verbatim-checked re-encoding)

**File**: `templates/adr.md` (line 8)
**Changes**: Update the trailing comment on the `status:` line so the portion
after `# ` contains the TSV cell byte-identically. Only the comment text matters
to the test (`grep -qF`); the leading padding spaces and `# ` prefix are not part
of the match, but keep the existing column alignment for readability.

```
status: proposed                             # proposed | accepted | rejected | superseded | deprecated
```

### Success Criteria:

#### Automated Verification:

- [x] Templates suite passes (the verbatim-comment check is green):
      `mise run test:unit:templates`
- [x] Config integration suites pass (validator + conformance + template tests):
      `mise run test:integration:config`
- [x] The TSV `adr` row col 5 contains `rejected`:
      `awk -F'\t' '$2=="adr"{print $5}' scripts/templates-schema.tsv` shows
      `proposed | accepted | rejected | superseded | deprecated`
- [x] The template comment matches the TSV cell verbatim (the standalone test
      that guards it): `bash scripts/test-template-frontmatter.sh`

#### Manual Verification:

- [x] Confirm (TDD check, local) that *before* the `templates/adr.md` edit but
      *after* the TSV edit, `bash scripts/test-template-frontmatter.sh` fails with
      "status line missing pinned vocabulary" — proving the comment sync is load-bearing.
- [x] Confirm the `skip_test` in `test-skill-frontmatter-conformance.sh` still
      reports `SKIP:` for `rejected` at this phase (no assertion yet) and the
      suite remains green.

---

## Phase 2: Lock in `rejected` coverage and discharge the 0103 deferral

### Overview

Prove the new vocabulary value is actually exercised: add a corpus-validator
accept-fixture for a `status: rejected` ADR, and flip the conformance suite's
`rejected` `skip_test` to the same live `assert_check` + `assert_accepts` every
other adr target runs.

### Changes Required:

#### 1. Corpus-validator accept-fixture for `status: rejected`

**File**: `scripts/test-validate-corpus-frontmatter.sh`
**Changes**: In the failure-mode fixtures block, alongside the accept-pattern
cases (place it naturally after the `bad-status` case at `:90`), add a paired
accept/reject fixture set for the adr status axis. Use the single-token vocab
idiom — pass `rejected` as the `emit_valid` vocab argument so the first-token
rule (`frontmatter-fixtures.sh:40`) pins `status: rejected` directly, with no
`sed` override. This matches how the conformance loop pins a status
(`test-skill-frontmatter-conformance.sh:354`) and avoids the fragile
emit-`proposed`-then-rewrite two-step. Both directions are asserted so the
widened vocab is bounded: the member `rejected` accepts, and a non-member
near-miss still rejects with `BAD-STATUS`.

```bash
# adr status: rejected is a vocab member (ADR-0031 lifecycle); validator accepts.
# NOTE: this is the ONLY corpus-validator coverage for `rejected` — the generic
# per-type valid-fixture loop (:37-44) cannot reach it (emit_valid pins status to
# the first vocab token, `proposed`). Do not treat this fixture as redundant.
emit_valid adr no decision_makers "rejected" "$TMP/ok-adr-rejected.md"
assert_accepts "adr status: rejected accepted" "$TMP/ok-adr-rejected.md"

# adr status: a non-member near-miss (`reject`) still rejects after the widening.
# Same single-token idiom — pass the non-member token directly, no sed.
emit_valid adr no decision_makers "reject" "$TMP/bad-adr-status.md"
assert_rejects "adr non-member status rejected" "BAD-STATUS" "$TMP/bad-adr-status.md"
```

#### 2. Flip the conformance `skip_test` to a live assertion

**File**: `scripts/test-skill-frontmatter-conformance.sh` (lines 345-356)
**Changes**: Remove the `if [ "$tgt" = "rejected" ]` branch (`:346-351`) so
`rejected` falls through to the identical three lines every other adr target runs
(`:352-355`). The loop then becomes uniform across all `adr_targets`:

```bash
for tgt in $adr_targets; do
  assert_check "review-adr -> adr: status '$tgt' ∈ adr vocab" 0 status_in_vocab "$tgt" "$ADR_VOCAB"
  adr_fx="$TMP/adr-$tgt.md"
  emit_valid adr no decision_makers "$tgt" "$adr_fx"
  assert_accepts "review-adr -> adr: status '$tgt' fixture accepted" "$adr_fx"
done
```

The `rejected` deferral is re-encoded in **three** comment sites in this file,
all of which must be brought into line with the now-live axis (removing the
branch alone would leave two of them contradicting the code):

1. **File-header paragraph (`:20-24`)** — rewrite wholesale, not a partial trim.
   It frames `rejected` as both a "deferred to work item 0104" divergence *and*
   describes the mechanism ("represented here as an explicit `skip_test` keyed to
   that id"). After this change both halves are false. Replace the
   `rejected`-specific sentences with a description of current behaviour
   (review-adr's `rejected` target is asserted on the status axis like every
   other target), leaving the generic mutator-axis description intact.
2. **Inline comment above the loop (`:339-340`)** — drop the
   "EXCEPT `rejected` — a known schema-source divergence deferred to 0104" clause
   so the comment reads simply that each documented target status must be an
   adr-vocab member, matching the now-uniform loop below it.
3. **The branch's own comments and `skip_test` reason string (`:347-349`)** —
   removed wholesale when the `if` branch is deleted. Note this is a *fourth*
   `0104` reference (the in-branch comment "Flips to a live assert_check when
   0104 lands" at `:348`, plus the skip reason at `:349`): treat the operation as
   *deleting the entire `:346-351` block*, not editing three isolated comments,
   so no stray in-branch `0104` comment survives. The `grep -c "0104"` success
   criterion is the backstop that confirms none remain.

### Success Criteria:

#### Automated Verification:

- [ ] Config integration suites pass with the new fixture + live axis present:
      `mise run test:integration:config`
- [ ] Templates suite still green: `mise run test:unit:templates`
- [ ] The corpus-validator suite contains and passes both the rejected
      accept-fixture and the adr non-member reject-fixture:
      `bash scripts/test-validate-corpus-frontmatter.sh` shows
      `PASS: adr status: rejected accepted` and
      `PASS: adr non-member status rejected`
- [ ] The conformance suite runs a live rejected axis — assert the two PASS lines
      are **present**: `bash scripts/test-skill-frontmatter-conformance.sh 2>&1`
      shows `PASS: review-adr -> adr: status 'rejected' ∈ adr vocab` and
      `PASS: review-adr -> adr: status 'rejected' fixture accepted`
- [ ] The rejected `SKIP:` line is **gone** — assert the negative explicitly
      (a bare `grep "rejected"` is permissive and would still match a residual
      SKIP line alongside the PASS lines):
      `! bash scripts/test-skill-frontmatter-conformance.sh 2>&1 | grep -q "SKIP:.*rejected"`
      exits 0
- [ ] No remaining `skip_test` keyed to `0104` in the conformance suite:
      `grep -c "0104" scripts/test-skill-frontmatter-conformance.sh` reflects only
      intended references (the deferral skip is gone)

#### Manual Verification:

- [ ] Confirm (TDD check, local) that with Phase 2's edits applied but the Phase 1
      TSV edit reverted, both the new corpus accept-fixture and the flipped
      conformance axis FAIL with `BAD-STATUS` / non-member — proving the
      assertions are load-bearing and genuinely gated on the vocab change.
- [ ] Re-confirm the independent re-encodings still carry `rejected` and need no
      change: `status-variant.ts:7`, `adr-read-status.sh:51` (verify only).

---

## Testing Strategy

### Unit Tests:

- `test-template-frontmatter.sh` — the verbatim `grep -qF` substring check that
  the `templates/adr.md` comment matches the TSV cell (Phase 1). Run via
  `mise run test:unit:templates`.

### Integration Tests:

- `test-validate-corpus-frontmatter.sh` — the real corpus validator over a paired
  adr fixture set (Phase 2): a `status: rejected` accept-fixture and a non-member
  reject-fixture that bounds the widened vocab on both sides, plus the existing
  per-type valid-fixture loop (unchanged; still emits `proposed` for adr).
- `test-skill-frontmatter-conformance.sh` — the producer-conformance guard; the
  `rejected` axis flips from `skip_test` to live `assert_check` + `assert_accepts`
  (Phase 2). All run via `mise run test:integration:config`.

### Manual Testing Steps:

1. Per-phase TDD reversal checks (documented in each phase's Manual Verification):
   confirm each new assertion / comment-sync is load-bearing by reverting the
   driving edit and observing the targeted failure.
2. Inspect the conformance suite output to confirm `rejected` now produces PASS
   lines and no SKIP line.
3. Run the full `mise run test:integration:config` and `mise run test:unit:templates`
   once more after both phases to confirm green with all rejected coverage present.

## Performance Considerations

None. The change adds one token to a pipe-delimited vocabulary cell and one
fixture; no measurable impact on validation or test runtime.

## Migration Notes

No data migration. Existing ADRs are unaffected — the change only *widens* the
accepted vocabulary, so every currently-valid ADR remains valid. No rejected ADR
currently exists in the corpus (this plan makes one valid going forward); the
real-corpus sanity check in `test-validate-corpus-frontmatter.sh:168-179` stays
green because widening a vocab never invalidates existing values.

## References

- Original work item: `meta/work/0104-add-rejected-to-adr-status-vocabulary.md`
- Related research: `meta/research/codebase/2026-06-11-0104-add-rejected-to-adr-status-vocabulary.md`
- Schema source: `scripts/templates-schema.tsv:6` (the `adr` row, col 5)
- Coupled comment: `templates/adr.md:8`
- Verbatim check: `scripts/test-template-frontmatter.sh:232-245`
- Accept-fixture pattern to model: `scripts/test-validate-corpus-frontmatter.sh:101-103`
- `emit_valid` first-token rule: `scripts/frontmatter-fixtures.sh:31-65`
- Conformance `skip_test` to flip: `scripts/test-skill-frontmatter-conformance.sh:345-356`
- Producer this aligns to: `skills/decisions/review-adr/SKILL.md:85,191-201`
- Lifecycle authority: `meta/decisions/ADR-0031-skill-level-adr-immutability.md`
- Vocab-reconciliation map (silent on `rejected`): `meta/decisions/ADR-0042-reconciling-pre-schema-status-values.md`
- Predecessor that deferred this: `meta/work/0103-audit-skill-frontmatter-emission-against-unified-schema.md`
