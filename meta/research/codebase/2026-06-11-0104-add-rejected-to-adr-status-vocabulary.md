---
type: codebase-research
id: "2026-06-11-0104-add-rejected-to-adr-status-vocabulary"
title: "Research: Add rejected to the ADR Status Vocabulary in the Unified Schema"
date: "2026-06-11T13:10:20+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0104"
parent: "work-item:0104"
relates_to: ["codebase-research:2026-06-09-0103-skill-frontmatter-emission-audit"]
topic: "Add rejected to the ADR Status Vocabulary in the Unified Schema"
tags: [research, codebase, frontmatter, schema, adr, status, validator]
revision: "bc20205b3e486dd259502991396187eafaab482e"
repository: "build-system"
last_updated: "2026-06-11T13:10:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Add rejected to the ADR Status Vocabulary in the Unified Schema

**Date**: 2026-06-11T13:10:20+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: bc20205b3e486dd259502991396187eafaab482e
**Branch**: workspace `build-system`, change `yt` (no bookmark)
**Repository**: build-system

## Research Question

For work item 0104 ("Add `rejected` to the ADR Status Vocabulary in the Unified
Schema"): verify, against the live codebase, every claim the work item makes
about where the `adr` status vocabulary is defined, which read-sites
auto-propagate from the TSV, which re-encodings must be hand-edited, what test
fixtures must be added or flipped, and that the change is sanctioned by the
accepted ADR lifecycle rather than deliberately excluded.

## Summary

**Every claim in the work item is confirmed.** The `adr` `status_vocab` is
defined once in `scripts/templates-schema.tsv:6` and currently reads
`proposed | accepted | superseded | deprecated` — `rejected` is absent. The
validator and its test harness are fully data-driven from that cell; adding
`rejected` auto-propagates into the runtime validation with no code change. Two
re-encodings must be hand-edited in lockstep (the template comment and the
conformance `skip_test`), and two test fixtures must be added/flipped to prove
the new value is actually exercised.

Key confirmations:

1. **Schema source** (`templates-schema.tsv:6`) omits `rejected`; this is the
   one cell to edit.
2. **The producer is right**: `review-adr` persists `status: rejected` +
   `rejected_reason` on the `proposed → rejected` transition
   (`SKILL.md:85,191-201`), exactly as ADR-0031 (accepted) sanctions.
3. **`rejected`'s omission is an oversight, not a deliberate exclusion**:
   ADR-0042 (the 0070 vocab-reconciliation ADR) never mentions `rejected` and
   does not touch the `adr` type at all.
4. **Auto-propagating read-sites** (validator BAD-STATUS path, `ADR_VOCAB`,
   per-type valid-fixture loop) need no edit — verified data-driven.
5. **Hand-edit re-encodings**: `templates/adr.md:8` comment (verbatim-checked)
   and the `rejected` `skip_test` in the conformance suite (keyed to 0104).
6. **Two test additions are required and would NOT be auto-created**: a
   corpus-validator accept-fixture, and flipping the conformance `skip_test` to
   a live `assert_check` + `assert_accepts`.
7. **Independent re-encodings already carry `rejected`** (visualiser
   `status-variant.ts`, `adr-read-status.sh` docstring) — verify only.
8. **Test reachability confirmed**: `test:integration:config` reaches all three
   relevant scripts via glob discovery (with count-floor + named guard);
   `test:unit:templates` reaches `test-template-frontmatter.sh`.

The work item's plan is accurate, complete, and correctly scoped. No surprises
or additional edit-sites were discovered.

## Detailed Findings

### 1. The schema source — `scripts/templates-schema.tsv:6`

The `adr` row (line 6), column 5 (`status_vocab`), currently reads:

```
proposed | accepted | superseded | deprecated
```

`rejected` is **not** present. Other adr-row columns relevant to fixtures:
`anchored=no`, `extras=decision_makers`. The required edit makes it
`proposed | accepted | rejected | superseded | deprecated`. This is the single
authoritative definition; everything else either reads it at runtime or
re-encodes it.

### 2. The producer (`review-adr`) already implements the rejected lifecycle

`skills/decisions/review-adr/SKILL.md` — verified correct against ADR-0031:

- Mutability/transition table (`:83-89`): `| proposed | Yes | → accepted, → rejected |`
  at `:85`, and a terminal `| rejected | No | None (terminal) |` row at `:87`.
- Reject action (`:191-201`): step 194 changes `status: proposed` →
  `status: rejected`; step 195 adds `rejected_reason: "[reason]"`; step 197
  updates the in-body `**Status**: Rejected` line. The terminal-status read-back
  block (`:226-238`) reads `rejected_reason` (`:237`).

`skills/decisions/create-adr/SKILL.md` always writes `status: proposed`
(`:165,205,245`) and only ever transitions other ADRs to `superseded`
(`:192-199`); it mentions rejection only by deferring to review-adr (`:211`). So
`review-adr` is the sole producer of `status: rejected`.

### 3. `rejected`'s omission is an oversight, not deliberate exclusion

- **ADR-0031** (`meta/decisions/ADR-0031-skill-level-adr-immutability.md`,
  **accepted**, 2026-03-18): explicitly adopts the lifecycle vocabulary
  `proposed, accepted, rejected, superseded, deprecated` (`:27-29`); transition
  table has `proposed → accepted, rejected` (`:75`) and `rejected` as terminal
  (`:78`); defines the `rejected_reason` field written atomically on rejection
  (`:56,64-68`). This is the authoritative lifecycle definition `review-adr`
  implements.
- **ADR-0042** (`meta/decisions/ADR-0042-reconciling-pre-schema-status-values.md`,
  **accepted**, 2026-06-08, parent `work-item:0070`): the document that widened
  per-type vocabularies during the 0070 migration. It **never mentions
  `rejected`** and its scope is only four legacy types that carried
  nonconforming values — **plan, plan-review, design-gap, design-inventory**
  (`:31-37`); `:38` states "All other types already conform". The `adr` type is
  not among them. No passage drops or argues against `rejected`.

Conclusion: ADR-0031 sanctions `rejected`; ADR-0042 (the vocab-reconciliation
authority) is silent on it and never touches `adr`. Its absence from the TSV is
most consistent with an oversight during migration. (Caveat surfaced by the
docs agent: ADR-0042 cites **ADR-0033** as the owner of the unified
`status_vocab` per type, not ADR-0031 — but the editable schema source is
`templates-schema.tsv`, which is what 0104 amends.)

### 4. Data-driven read-sites — auto-propagate, NO edit needed

**`scripts/validate-corpus-frontmatter.sh`** — confirmed fully data-driven:
- Loads column 5 verbatim into the parallel array `SCHEMA_STATUS` in the TSV
  read loop (`:47-54`, specifically `SCHEMA_STATUS+=("$status_vocab")` at `:51`),
  selected per-type by index at `:281`.
- BAD-STATUS path (`:315-330`) sets `IFS='|'`, iterates the unquoted
  `$status_vocab`, trims each token's surrounding whitespace (`:322-323`), and
  matches against the file's `status`. **No hardcoded adr status list exists.**
  Adding `rejected` to the TSV cell makes it an accepted token automatically.

**`scripts/test-validate-corpus-frontmatter.sh`** — the per-type valid-fixture
loop (`:37-44`) walks every TSV row via `emit_valid` and asserts the directory
is accepted. **Caveat**: `emit_valid` emits only the *first* vocab token as the
status (`frontmatter-fixtures.sh:40`, `cut -d'|' -f1`), so for `adr` it always
emits `status: proposed`. This loop therefore does **not** exercise `rejected`
even after the TSV edit — which is exactly why a dedicated accept-fixture is
required (see §6). The row-count assertion (`:42-43`) is count-based, not
status-aware, so the TSV edit doesn't disturb it.

**`scripts/test-skill-frontmatter-conformance.sh`** — `ADR_VOCAB` is read from
`SCHEMA_STATUS[$ADR_IDX]` (`:341-342`), populated from the TSV at `:64-75`
(`SCHEMA_STATUS+=("$vocab")` at `:72`). `status_in_vocab` (`:155-168`) splits on
`|` and trims. Data-driven; the TSV edit propagates here automatically.

### 5. Coupled re-encodings — MUST be hand-edited

**`templates/adr.md:8`** — the `status:` line, exact current text:

```
status: proposed                             # proposed | accepted | superseded | deprecated
```

`scripts/test-template-frontmatter.sh:232-245` extracts the `status:` line and
runs `grep -qF -- "$status_vocab"` against it, where `$status_vocab` is the TSV
cell (column 5, destructured at `:142`). This is a **fixed-string substring**
check (not anchored, not full-line): the template's comment must contain the TSV
cell *verbatim and contiguously*, including the ` | ` (space-pipe-space)
separators. The padding spaces and `# ` prefix are not part of the match. When
`rejected` is added, the comment after `# ` must contain
`proposed | accepted | rejected | superseded | deprecated` byte-identically to
the TSV cell. **The templates suite fails until this comment is synced** — the
TSV edit alone is insufficient. (No other line in `templates/adr.md` encodes the
vocab; the body `**Status**: Proposed` at `:23` is a single fixed value and is
not vocab-checked.)

**`scripts/test-skill-frontmatter-conformance.sh:345-356`** — the `rejected`
`skip_test` branch (`:346-351`):

```bash
for tgt in $adr_targets; do
  if [ "$tgt" = "rejected" ]; then
    # Deferred: adr vocab lacks `rejected` though ADR-0031 adopts it and
    # review-adr persists it. Flips to a live assert_check when 0104 lands.
    skip_test "review-adr -> adr: status 'rejected' ∈ vocab" "schema-source divergence deferred to work item 0104"
    continue
  fi
  assert_check "review-adr -> adr: status '$tgt' ∈ adr vocab" 0 status_in_vocab "$tgt" "$ADR_VOCAB"
  adr_fx="$TMP/adr-$tgt.md"
  emit_valid adr no decision_makers "$tgt" "$adr_fx"
  assert_accepts "review-adr -> adr: status '$tgt' fixture accepted" "$adr_fx"
done
```

Confirmed: the comment (`:347-348`) and skip reason (`:349`) both name work item
0104; the header comment (`:22-24`) corroborates. `skip_test`
(`test-helpers.sh:304-309`) only prints `SKIP:` and increments a counter — it
runs **no** assertion. The `adr_targets` set is derived from `review-adr`'s prose
by `extract_review_adr_targets` (`:141-144`), which greps
`` to `status: <token>` `` from `SKILL.md` — picking up `accepted`,
`deprecated`, and `rejected` (sorted). So `rejected` is discovered from the
producer's prose independently of the TSV; the divergence is precisely that it's
in the prose but not the vocab.

### 6. Required test additions (would NOT be auto-created)

**(a) Corpus-validator accept-fixture** in
`scripts/test-validate-corpus-frontmatter.sh`. The accept-pattern to model on is
the "Z (zulu) timestamp" case (`:101-103`): `emit_valid` + `sed` one field +
`assert_accepts`. Concretely (placed in the failure-mode fixtures block,
naturally after `:90`):

```bash
emit_valid adr no "decision_makers" "proposed | accepted | rejected | superseded | deprecated" "$TMP/ok-adr-rejected.md"
sed -i.bak 's/^status: .*/status: rejected/' "$TMP/ok-adr-rejected.md"
assert_accepts "adr status: rejected accepted" "$TMP/ok-adr-rejected.md"
```

(`emit_valid` would otherwise pin `status: proposed` via its first-token rule;
the `sed` overrides to `rejected`. Helpers: `assert_accepts` passes only when the
real validator exits 0 — `frontmatter-fixtures.sh:87-99`.)

**(b) Flip the conformance `skip_test` to a live axis**. Delete the
`if [ "$tgt" = "rejected" ]` branch (`:346-351`) so `rejected` falls through to
the same three lines every other target runs (`:352-355`): `assert_check
… status_in_vocab "$tgt" "$ADR_VOCAB"` (membership) + `emit_valid adr no
decision_makers "$tgt"` + `assert_accepts` (real validator over the fixture).
The TSV edit is what makes both assertions pass. The identical shape is already
used by the `validate-plan` axis (`:333-337`) and the non-`rejected` adr targets.

### 7. Independent re-encodings — verify only, already carry `rejected`

- `skills/visualisation/visualise/frontend/src/api/status-variant.ts:7` —
  `const RED = new Set(['blocked', 'rejected', 'deprecated', 'superseded', 'abandoned'])`;
  self-contained, does not read the TSV. Tests cover it:
  `StatusBadge.test.tsx:50` (`['rejected', 'red']`) and `status-variant.test.ts:29`.
- `skills/decisions/scripts/adr-read-status.sh:51` — docstring/error enum lists
  `rejected`; the script only reads/echoes `status` (`:37-46`) and never
  validates against the enum (the enum string appears solely in a
  no-status-found error message).

### 8. Test task reachability (mise → invoke)

- `mise.toml:132-134` → `test:integration:config` → `invoke test.integration.config`
  → `run_shell_suites(context, "scripts")` (`tasks/test/integration.py:45-64`,
  `tasks/test/helpers.py:13-40`), which glob-discovers and runs every executable
  `scripts/**/test-*.sh`. This reaches **all three** relevant scripts
  (`test-validate-corpus-frontmatter.sh`, `test-skill-frontmatter-conformance.sh`,
  `test-template-frontmatter.sh`). Hardened by a count floor of 16 suites
  (`integration.py:14,49-56`) and a named-presence guard for
  `test-skill-frontmatter-conformance.sh` (`integration.py:21,57-64`).
- `mise.toml:109-111` → `test:unit:templates` → `invoke test.unit.templates`
  (`tasks/test/unit.py:34-50`) runs a fixed list including
  `test-template-frontmatter.sh` (`:38`). The other two are **not** in this
  task's list (they are covered by `test:integration:config`).

Implication: the acceptance-criterion that both tasks "stay green **with the new
rejected fixture and live conformance assertion present**" is meaningful — the
new corpus accept-fixture and the flipped conformance axis are both reached, so
green-ness with them present is genuine coverage.

## Code References

- `scripts/templates-schema.tsv:6` — the `adr` row; column 5 `status_vocab` to edit.
- `scripts/validate-corpus-frontmatter.sh:47-54,281,315-330` — TSV load + data-driven BAD-STATUS path (no edit).
- `scripts/test-validate-corpus-frontmatter.sh:37-44,88-90,101-103` — per-type loop; reject pattern; accept-pattern to model the new fixture on.
- `scripts/frontmatter-fixtures.sh:31-65,40,87-99` — `emit_valid` (first-token status rule), `assert_accepts`.
- `scripts/test-skill-frontmatter-conformance.sh:141-144,341-342,345-356` — prose-target parse, `ADR_VOCAB`, the `rejected` `skip_test` to flip.
- `scripts/test-helpers.sh:304-309` — `skip_test` (no-op assertion).
- `templates/adr.md:8` — the verbatim-checked `status:` vocab comment to sync.
- `scripts/test-template-frontmatter.sh:142,232-245` — the `grep -qF` substring check.
- `skills/decisions/review-adr/SKILL.md:85,87,191-201,226-238` — producer of `status: rejected`.
- `skills/decisions/create-adr/SKILL.md:211` — defers rejection to review-adr.
- `skills/visualisation/visualise/frontend/src/api/status-variant.ts:7` — `rejected → red` (verify only).
- `skills/decisions/scripts/adr-read-status.sh:37-46,51` — reads/echoes status; enum in error text only.
- `mise.toml:109-111,132-134`; `tasks/test/unit.py:34-50`; `tasks/test/integration.py:14,21,45-64`; `tasks/test/helpers.py:13-40` — test task wiring.

## Architecture Insights

- **Single source of truth, fan-out re-encodings.** The vocab lives once in the
  TSV; the validator, conformance suite, and valid-fixture generator all read it
  at runtime (bash 3.2 parallel arrays, no associative arrays). The only manual
  sync points are *re-encodings*: the template comment (a human-facing copy,
  guarded by a verbatim substring test) and the conformance `skip_test` (a
  deliberate, work-item-keyed deferral). This is a clean design — the cost of
  adding a vocab value is one data edit plus two intentional re-encoding edits,
  each independently test-guarded.
- **Producer-prose as conformance oracle.** The conformance suite derives the
  expected adr status targets by parsing `review-adr`'s prose, then checks them
  against the TSV-derived vocab. This is what *surfaces* schema-source
  divergences automatically (prose says `rejected`, vocab doesn't) — and the
  `skip_test` keyed to 0104 is the codified, non-silent acknowledgement of the
  gap, designed to flip live when this work item lands.
- **Defence against silent coverage loss.** The integration task's count floor
  and named-presence guard mean a dropped exec bit on a fail-closed gate fails
  loudly rather than silently shrinking coverage — relevant because this change
  relies on those gates running.
- **First-token fixture rule is a sharp edge.** `emit_valid` always emits the
  first vocab token as `status`, so widening a vocab never auto-tests the new
  value via the generic per-type loop. New non-first values must get an explicit
  accept-fixture — which the work item correctly requires.

## Historical Context

- `meta/work/0103-audit-skill-frontmatter-emission-against-unified-schema.md` —
  the audit that surfaced this divergence and left the `skip_test` keyed to 0104.
- `meta/research/codebase/2026-06-09-0103-skill-frontmatter-emission-audit.md` —
  the 0103 codebase research.
- `meta/plans/2026-06-09-0103-audit-skill-frontmatter-emission.md` — specifies
  the schema-source-vs-producer triage and the deferred `skip_test`.
- `meta/validations/2026-06-09-0103-audit-skill-frontmatter-emission-validation.md`
  — records the triage raising 0104/0105 and the `skip_test` location.
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` —
  parent epic.
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` and
  `meta/decisions/ADR-0042-reconciling-pre-schema-status-values.md` — the
  migration and vocab-reconciliation ADR that did not carry `rejected` into the
  adr vocab.
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — the lifecycle this
  aligns the schema to.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — the unified base
  schema (cited by ADR-0042 as the per-type `status_vocab` owner).
- `meta/reviews/work/0104-add-rejected-to-adr-status-vocabulary-review-1.md` —
  prior review of this work item.

## Related Research

- `meta/research/codebase/2026-06-09-0103-skill-frontmatter-emission-audit.md`
- `meta/research/codebase/2026-06-07-0070-meta-corpus-unified-schema-migration.md`
- `meta/research/codebase/2026-05-30-0065-update-artifact-templates-to-unified-schema.md`

## Open Questions

- **Reader-surface treatment of rejected ADRs** (carried from the work item, out
  of scope): the visualiser already renders `rejected → red`, so a rejected ADR
  displays correctly today. Unresolved: should `rejected` ADRs be *filtered out*
  of active-decision listings or shown with a distinct treatment? This change
  only makes a rejected ADR schema-valid.
- **ADR-0033 vs the TSV as the documented vocab owner**: ADR-0042 names ADR-0033
  as the per-type `status_vocab` owner. ADR-0033 is accepted and immutable, so it
  cannot be edited to record `rejected`. Worth a glance to confirm ADR-0033 does
  not itself enumerate the adr vocab in a way that would now contradict the TSV
  (the editable schema source). Not a blocker for the TSV edit, but a
  consistency check.
