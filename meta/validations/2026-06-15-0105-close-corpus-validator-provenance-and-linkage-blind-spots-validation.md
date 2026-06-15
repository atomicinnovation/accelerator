---
type: plan-validation
id: "2026-06-15-0105-close-corpus-validator-provenance-and-linkage-blind-spots-validation"
title: "Validation Report: Close the Corpus Validator Provenance and Linkage Blind Spots"
date: "2026-06-15T23:00:09+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-06-15-0105-close-corpus-validator-provenance-and-linkage-blind-spots"
target: "plan:2026-06-15-0105-close-corpus-validator-provenance-and-linkage-blind-spots"
tags: [frontmatter, schema, validator, provenance, linkage]
last_updated: "2026-06-15T23:00:09+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Close the Corpus Validator Provenance and Linkage Blind Spots

### Implementation Status

✓ Phase 1: Provenance over-emission rule — fully implemented
✓ Phase 2: Bare/unquoted typed-linkage rule — fully implemented
✓ Phase 3: Delete the bespoke conformance-guard helpers — fully implemented

Each phase landed as a discrete commit, in dependency order:

- `wrrnnkuw` — Reject provenance fields on non-anchored types (Phase 1)
- `trtxqnuu` — Reject bare/unquoted typed-linkage values (Phase 2)
- `rltoqmup` — Collapse blind-spot conformance helpers into the validator (Phase 3)

### Automated Verification Results

✓ Validator suite: `bash scripts/test-validate-corpus-frontmatter.sh` — 40 passed, 0 failed
✓ Conformance guard: `bash scripts/test-skill-frontmatter-conformance.sh` — 96 passed, 0 failed
✓ Config integration: `mise run test:integration:config` — green (runs both `test-*` suites)
✓ Templates unit: `mise run test:unit:templates` — 36 passed, 0 failed
✓ Shell lint/format: `mise run scripts:check` — clean (format + shellcheck + bashisms)
✓ No orphaned refs: `grep -rn '\[0105\]' scripts/` empty; `grep -rn 'check_no_provenance_over_emission\|check_linkage_quoted' scripts/` empty
✓ Hand-run: non-anchored work-item with `revision:`/`repository:` emits `PROVENANCE-ON-NONANCHORED` (one per field) and exits non-zero

### Code Review Findings

#### Matches Plan:

- **Phase 1** (`validate-corpus-frontmatter.sh:295-310`): the forward-only `if`
  became `if/else`; the `else` branch iterates `FM_PROVENANCE_FIELDS` and emits
  `PROVENANCE-ON-NONANCHORED` per present field. The `:295` comment now reads
  "(both directions enforced)". Verbatim match to the planned snippet.
- **Phase 1 fixtures**: both `bad-prov-nonanchored.md` (bundle) and
  `bad-prov-revision-only.md` (lone field) added via `extra_lines`, each
  asserting the specific diagnostic. As planned.
- **Phase 2** (`validate-corpus-frontmatter.sh:334-388`): the quoted-substring
  `while` was replaced with comment-strip → bracket-strip → `IFS=','` comma-split
  under `set -f`, with a `case` asserting each non-empty element is a quoted
  token; the `*)` arm emits `BAD-LINKAGE-SHAPE` with the unquoted-distinguishing
  message. `set +f` restores globbing. Verbatim match.
- **Phase 2 fixtures**: all four reject cases (bare scalar, bare path,
  bracketed-unquoted, mixed list), three accept cases (multi-element list,
  irregular spacing, trailing inline comment), the `set -f` glob-suppression
  fixture, and both no-double-flag assertions (`parent: ""` and `relates_to: []`)
  present. The `assert_absent` helper was added to `frontmatter-fixtures.sh` as
  specified.
- **Phase 3**: `check_no_provenance_over_emission` and `check_linkage_quoted`
  deleted along with the banner comment, both per-emitter call sites, and the
  "Blind-spot liveness" self-test block. Header docblock item 4 removed. The
  conditional-axis section now carries `prov-overemit`
  (`PROVENANCE-ON-NONANCHORED`) and the `link-bare` case switched from quoted
  `"0042"` to genuinely unquoted `0042`. `assert_check` retained for the status
  assertions; the "No re-encoded contract" meta-asserts still pass.

#### Deviations from Plan:

- None. The implementation matches the planned snippets line-for-line, including
  comment wording and diagnostic message text.

#### Potential Issues:

- None blocking. The pre-existing narrow gap explicitly scoped out in "What
  We're NOT Doing" (an embedded empty element inside a non-empty list, e.g.
  `relates_to: ["plan:0001", ""]`) remains unaddressed by design — the rewrite
  neither closes nor widens it, consistent with the plan.
- The "What We're NOT Doing" constraint held: `templates-schema.tsv` and
  `frontmatter-emission-rules.sh` were not modified across any of the three
  commits. The only test-helper addition (`assert_absent`) was planned.

### Manual Testing Required:

None outstanding. All manual-verification items in the plan are covered by the
automated fixtures and the hand-run above:

1. ✓ `PROVENANCE-ON-NONANCHORED` fires on a crafted non-anchored fixture
   (hand-run confirmed, two violations — one per provenance field).
2. ✓ Anchored types still require provenance — `MISSING-PROVENANCE` path
   unchanged (`prov-missing` fixture green in conformance guard).
3. ✓ Bare scalar / bare path / bracketed-unquoted / mixed-list all reject with
   `BAD-LINKAGE-SHAPE` (validator-suite fixtures green).
4. ✓ Well-formed quoted list accepts (validator-suite + guard fixtures green).
5. ✓ Empty forms emit `EMPTY-PLACEHOLDER` only, no double-flag (`assert_absent`
   assertions green).
6. ✓ `set -f` glob suppression mutation-proofed by the `bad-glob-linkage`
   fixture.

### Recommendations:

- **Ready to merge.** All three phases are complete, every automated check is
  green, and the real-corpus sanity check confirms no live artifact is newly
  rejected. No migration required.
- Consider the plan's noted follow-up: if template-*source* lint coverage of
  these two axes is wanted later (the deliberate tradeoff from deleting the
  bespoke helpers), open a separate work item for a validator-routed lint over
  `templates/*.md` frontmatter. Out of scope here; not a blocker.
