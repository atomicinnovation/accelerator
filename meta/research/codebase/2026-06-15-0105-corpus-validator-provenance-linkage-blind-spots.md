---
type: codebase-research
id: "2026-06-15-0105-corpus-validator-provenance-linkage-blind-spots"
title: "Research: Close the Corpus Validator Provenance and Linkage Blind Spots (0105)"
date: "2026-06-15T16:38:06+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0105"
parent: "work-item:0105"
relates_to: ["codebase-research:2026-06-09-0103-skill-frontmatter-emission-audit"]
topic: "Folding the two known validate-corpus-frontmatter.sh blind spots (non-anchored provenance over-emission; bare/unquoted typed-linkage values) into the single oracle"
tags: [research, codebase, frontmatter, schema, validator, provenance, linkage]
revision: "d8d49046e5a3b3f9b9650a673c0ecf4edc3e7cfb"
repository: "build-system"
last_updated: "2026-06-15T16:38:06+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Close the Corpus Validator Provenance and Linkage Blind Spots (0105)

**Date**: 2026-06-15T16:38:06+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: d8d49046e5a3b3f9b9650a673c0ecf4edc3e7cfb
**Branch**: build-system (jj workspace)
**Repository**: build-system

## Research Question

For work item 0105 — fold two known blind spots into
`scripts/validate-corpus-frontmatter.sh` (the single corpus-frontmatter
oracle) so it enforces them directly, then collapse the bespoke guard helpers
in `scripts/test-skill-frontmatter-conformance.sh`. Map the exact code the task
touches: the provenance enforcement block, the typed-linkage shape loop, the
data-driven contract source, the test harnesses and their shared fixtures, the
schema TSV, and every diagnostic consumer — so implementation can proceed
without re-discovery.

## Summary

The work item's two blind spots are real and verified at source:

1. **Provenance over-emission** — the validator enforces the provenance "iff"
   in only the forward direction (`anchored=yes ⇒ revision+repository present`).
   There is no reverse rule rejecting `revision`/`repository` on a
   non-anchored type. The fix is a complementary loop beside the existing one.
2. **Bare/unquoted typed-linkage values** — the shape loop extracts tokens with
   `while [[ "$rest" =~ \"([^\"]*)\" ]]`, which only ever matches text inside
   double quotes. A genuinely unquoted scalar (`parent: 0042`) or unquoted path
   produces **zero tokens**, so the body never runs and `BAD-LINKAGE-SHAPE`
   never fires.

Both rules live in `validate-corpus-frontmatter.sh`, read their data from
`frontmatter-emission-rules.sh` (the sourced contract) and
`templates-schema.tsv` (per-type facts), and are exercised through the shared
`frontmatter-fixtures.sh` assertion library. The architecture cleanly supports
the planned change — the new rules stay data-driven, and the two bespoke guard
helpers that currently cover these axes can collapse to liveness checks or be
deleted.

**Three corrections to the work item surfaced during research** — see
[Discrepancies](#discrepancies-work-item-vs-source). The most important: the
work item's line references for the provenance block (`:314-324`) and the
linkage loop (`:355-376`) are stale; the actual current locations are
**295–305** and **334–357**. The bespoke helper names in the work item
(`assert_no_provenance_over_emission` / `assert_linkage_shape`) do not exist —
the real names are `check_no_provenance_over_emission` and
`check_linkage_quoted`. And the direct `grep -qF "BAD-LINKAGE-SHAPE"` consumer
is in `test-validate-corpus-frontmatter.sh:163`, **not** in the conformance
guard as the work item's Open Questions claims.

## Detailed Findings

### Validator structure — `validate-corpus-frontmatter.sh`

A bash-3.2-safe validator. All violations funnel through one helper; a counter
drives the exit code — there is **no error array**.

- `violation()` (`scripts/validate-corpus-frontmatter.sh:35-38`): prints
  `file: CODE — message` to stderr and increments the integer `VIOLATIONS`
  (declared `:34`). The diagnostic code (`MISSING-PROVENANCE`,
  `FORBIDDEN-PROVENANCE`, `BAD-LINKAGE-SHAPE`, `DANGLING-REF`, …) is passed as
  `$2` and printed verbatim. New rules emit via the same call.
- Entry: `main()` (`:361-390`). Two modes — whole-corpus (single dir arg,
  builds a referential index, `referential=yes`) and file-list (structural
  only, `referential=no`). Final exit at `:383-387`: `exit 1` if
  `VIOLATIONS > 0`.
- Frontmatter parse is bash-native, not yq: `extract_frontmatter()` (`:105-114`,
  an awk fence state-machine), `parse_fm()` (`:156-173`, fills parallel arrays
  `BK_KEYS`/`BK_VALS`), accessors `bk_present` (`:175-181`), `bk_value` (sets
  global `BK_VAL`, `:183-193`), `fm_inner` (strips one quote layer, `:127-140`).
- Per-type facts load once into parallel arrays
  `SCHEMA_TYPES/SCHEMA_ANCHORED/SCHEMA_EXTRAS/SCHEMA_STATUS/SCHEMA_FORBIDDEN/SCHEMA_LINKKEYS`
  from the TSV (`:41-54`), indexed by `schema_index()` (`:57-66`). Inside
  `validate_file`, `:242` sets `anchored="${SCHEMA_ANCHORED[$idx]}"` and `:246`
  sets `linkkeys="${SCHEMA_LINKKEYS[$idx]}"`.
- The contract source is resolved and sourced at `:25-26`
  (`FM_EMISSION_RULES="${FM_EMISSION_RULES:-$SCRIPT_DIR/frontmatter-emission-rules.sh}"`);
  the TSV at `:28` (`SCHEMA_TSV="${SCHEMA_TSV:-$SCRIPT_DIR/templates-schema.tsv}"`).
  Both are env-overridable — the tamper test exploits this.

### Blind spot 1 — provenance enforcement block (`:295-305`)

```bash
295	  # Provenance bundle iff code_state_anchored=yes; git_commit/branch never.
296	  if [ "$anchored" = "yes" ]; then
297	    for f in "${FM_PROVENANCE_FIELDS[@]}"; do
298	      bk_present "$f" ||
299	        violation "$file" "MISSING-PROVENANCE" "anchored type missing provenance field '$f'"
300	    done
301	  fi
302	  for f in "${FM_FORBIDDEN_PROVENANCE_FIELDS[@]}"; do
303	    bk_present "$f" &&
304	      violation "$file" "FORBIDDEN-PROVENANCE" "legacy provenance field '$f' present"
305	  done
```

- `anchored` traces to TSV column `code_state_anchored` (col 3) →
  `SCHEMA_ANCHORED[$idx]` → local `anchored` (`:242`).
- Only the forward direction is enforced (`:296-301`): anchored requires the
  bundle. There is **no** `anchored != yes ⇒ provenance absent` rule. That is
  the blind spot.
- The legacy-forbid loop (`:302-305`) runs **unconditionally** over
  `FM_FORBIDDEN_PROVENANCE_FIELDS` (`git_commit branch`) and emits
  `FORBIDDEN-PROVENANCE`. **This is the natural template** for the new rule: a
  complementary `if [ "$anchored" != "yes" ]; then … FM_PROVENANCE_FIELDS …
  FORBIDDEN-PROVENANCE-NONANCHORED` block placed beside it.
- The comment at `:295` ("Provenance bundle iff…") overstates what is enforced
  today; update it once the reverse direction is real (the work item's
  Technical Notes already flag this).

### Blind spot 2 — typed-linkage shape loop (`:334-357`)

```bash
334	  # Typed-linkage values: doc-type:id shape + (corpus mode) referential.
335	  local key rest tok
336	  for key in $linkkeys; do
337	    bk_value "$key" || continue
338	    rest="$BK_VAL"
339	    while [[ "$rest" =~ \"([^\"]*)\" ]]; do
340	      tok="${BASH_REMATCH[1]}"
341	      rest="${rest#*\""${tok}"\"}"
342	      [ -n "$tok" ] || continue
343	      if [[ ! "$tok" =~ $FM_TYPED_REF_RE ]]; then
344	        violation "$file" "BAD-LINKAGE-SHAPE" "$key: '$tok' is not a typed \"doc-type:id\" reference"
345	        continue
346	      fi
347	      if [ "$referential" = "yes" ]; then
348	        case "$tok" in
349	          pr:*) : ;; # tolerated external-entity prefix
350	          *)
351	            index_has "$tok" ||
352	              violation "$file" "DANGLING-REF" "$key: '$tok' resolves to no artifact in the corpus"
353	            ;;
354	        esac
355	      fi
356	    done
357	  done
```

- `$linkkeys` is **per-type** (TSV col 7 `typed_linkage_keys` →
  `SCHEMA_LINKKEYS[$idx]`), word-split on whitespace (`:336`). It is *not* the
  union `FM_LINKAGE_VOCABULARY` from the contract file.
- The `while` (`:339`) only matches `"…"` tokens. Lists and scalars are handled
  identically — brackets/commas are never parsed structurally; every quoted
  substring becomes a token. So `relates_to: ["adr:0001", "note:0009"]` yields
  two tokens; `parent: "plan:0042"` yields one.
- **The escape:** an unquoted scalar `parent: 0042` (or unquoted path) carries
  no quotes ⇒ the `while` never matches ⇒ zero tokens ⇒ no shape check. By
  contrast a *quoted* path `parent: "meta/work/0042.md"` produces a token and
  *does* fail the shape regex (because `/` is outside the id charset). This is
  exactly why the existing "path-shape" fixture (which is quoted) is already
  caught and the genuinely-unquoted case is not.
- Fix shape (per Technical Notes): pre-split the value on commas/brackets and
  assert each non-empty element is a quoted token matching `FM_TYPED_REF_RE`; a
  bare element fails the quoting assertion before the type-shape check. Reuse
  `BAD-LINKAGE-SHAPE` (resolved in the work item's Open Questions).

### The data-driven contract — `frontmatter-emission-rules.sh`

A pure data/function library (no side effects, bash-3.2-safe), sourced by both
the validator and the template-shape test so they cannot drift.

- `FM_PROVENANCE_FIELDS=(revision repository)` — `:34`. The *required* bundle
  for anchored types. Consumed at validator `:297`. The work item's assumption
  that this array is the complete forbid-set for non-anchored types holds, and
  keeping the new rule iterating this array (rather than a literal pair) is what
  keeps it data-driven.
- `FM_FORBIDDEN_PROVENANCE_FIELDS=(git_commit branch)` — `:35`. The *legacy*
  pair forbidden on **every** artifact. Consumed at validator `:302`.
- `FM_TYPED_REF_RE="^(${FM_SOURCE_TYPE_RE}):[A-Za-z0-9.-]+$"` — `:88`. Matches
  the **already-unquoted inner** token `doc-type:id`. It does **not** itself
  require quotes — quoting is enforced by the caller's tokenizer (`:339-341`).
  Id charset `[A-Za-z0-9.-]` admits dotted version stems but excludes `/`
  (deliberately keeps paths out). `FM_SOURCE_TYPE_RE` (`:41`) is the
  pipe-joined 14-member doc-type vocabulary.
- `FM_LINKAGE_VOCABULARY` (`:47`) — union of permitted linkage key names;
  `fm_is_linkage_key()` (`:94-100`) and `fm_linkage_cardinality()` (`:52-58`,
  returns `single`/`list`) expose it. **Note:** the validator's per-file loop
  uses the per-type TSV `linkkeys`, not this union — so a list-cardinality vs
  single-cardinality distinction is *not* currently enforced by the shape loop
  (relevant if the fix wants to be cardinality-aware, though the work item does
  not require that).
- Anchoring is deliberately **not** in this file — it is a per-type TSV fact.
- All `FM_*` constants carry a file-level `# shellcheck disable=SC2034`
  (`:21-24`) because they are consumed by sourcing surfaces.

### Schema TSV — `templates-schema.tsv`

- Header (`:1`): `template, type, code_state_anchored, extras, status_vocab,
  forbidden_own_id_key, typed_linkage_keys`.
- `code_state_anchored` = **column 3** (`yes`/`no`). Anchored (`yes`): `plan`,
  `pr-description`, `codebase-research`, `issue-research`, `design-inventory`,
  `note`. Non-anchored (`no`): `work-item`, `plan-validation`, `adr`,
  `design-gap`, `plan-review`, `work-item-review`, `pr-review`. The non-anchored
  set is exactly the population the new provenance rule will newly police.
- `typed_linkage_keys` = **column 7** (per-type). E.g. `work-item`:
  `parent blocks blocked_by derived_from relates_to source`.
- Consumed by `validate-corpus-frontmatter.sh:28`,
  `test-template-frontmatter.sh:30`, `test-skill-frontmatter-conformance.sh:52`,
  `test-validate-corpus-frontmatter.sh:20`, and migration
  `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:26`.

### Test harnesses and the shared fixture library

The assertion/fixture helpers are **not** in the test files — they live in the
sourced `scripts/frontmatter-fixtures.sh`, shared by both suites.

- `emit_valid <type> <anchored> <extras> <vocab> <outfile> [extra_lines]`
  (`frontmatter-fixtures.sh:31-66`) writes a minimal *valid* artifact; the 6th
  arg is appended verbatim before the closing fence (`:63`).
- `run_validator` (`:69-73`) captures stderr only into `VALIDATOR_ERR`, rc into
  `VALIDATOR_RC`.
- `assert_rejects <name> <code> <args…>` (`:75-88`) requires **both** non-zero
  exit **and** `grep -qF -- "$code"` on stderr — so it asserts a *specific*
  diagnostic, not merely non-zero exit. This is exactly what the work item's
  third acceptance criterion demands; no new helper is needed.
- `assert_accepts <name> <args…>` (`:90-103`) checks rc 0 only.

**Existing fixtures (`test-validate-corpus-frontmatter.sh`):**
- `FORBIDDEN-PROVENANCE` covered at `:64-65` (a `plan` with `git_commit:`).
- `MISSING-PROVENANCE` is **not** covered here — only in
  `test-skill-frontmatter-conformance.sh:359-361` (emit anchored `plan`, then
  `sed` out `revision:`/`repository:`). If a `MISSING-PROVENANCE` or new
  `FORBIDDEN-PROVENANCE-NONANCHORED` fixture is wanted in *this* suite, the
  nearest pattern to mirror is that sed-delete idiom.
- Linkage fixtures at `:70-81`:
  - `:73` `parent: "0030"` — **quoted** scalar, bare-*number ref payload* →
    already `BAD-LINKAGE-SHAPE`.
  - `:76` `parent: "meta/work/0030-foo.md"` — **quoted** path payload → already
    caught.
  - **Confirmed:** the work item's claim is correct — both existing
    "bare-number"/"path-shape" fixtures pass *quoted* scalars. There is no
    fixture feeding a genuinely YAML-unquoted scalar (`parent: 0030`) in this
    file. New fixtures must use unquoted values + a mixed list with one unquoted
    element.
- Fixture idiom: emit valid + pass bad line via `extra_lines`, or emit valid +
  `sed -i.bak` mutate. `.bak` siblings cleaned with `rm -f "$DIR"/*.bak`.

**Bespoke guard helpers (`test-skill-frontmatter-conformance.sh`) — to collapse:**
- Banner naming work item 0105 at `:176-177` ("BYPASS the validator — fold into
  the oracle under work item 0105").
- `check_no_provenance_over_emission` (`:180-188`) — greps the template + skill
  substitute-list text for any `FM_PROVENANCE_FIELDS` member on a non-anchored
  type. Uses the contract's *field list* but re-implements the *rule*; never
  calls the validator.
- `check_linkage_quoted` (`:190-209`) — parses each template linkage line's
  value via parameter expansion + `case`, rejecting a bare scalar. Never calls
  the validator.
- Both driven via generic `assert_check` (`:212-224`); call sites carry inline
  `[0105]` tags (`:312-320`); liveness self-tests at `:404-422` (synthetic
  non-anchored-with-provenance trips rc 1; bare `parent: 0042` trips rc 1; with
  anchored / quoted controls).
- "No re-encoded contract" meta-asserts at `:426-429` (guard must source the
  rules file and read the TSV) — these constrain how the collapse is done.

### Diagnostic consumers (impact surface of reusing `BAD-LINKAGE-SHAPE`)

- **Producer:** `validate-corpus-frontmatter.sh:344`.
- **Asserted in:** `test-validate-corpus-frontmatter.sh:74,77` (via
  `assert_rejects`); `test-skill-frontmatter-conformance.sh:369` (via
  `assert_rejects`).
- **Direct `grep -qF "BAD-LINKAGE-SHAPE"`:** `test-validate-corpus-frontmatter.sh:163`
  — the single-source tamper guard (inline, not via `assert_rejects`).
  **This is the only literal grep consumer, and it is in the corpus-validator
  suite, not the conformance guard.** Reusing `BAD-LINKAGE-SHAPE` (rather than a
  new `BARE-LINKAGE-VALUE`) avoids disturbing this consumer — which is the right
  conclusion, though the work item's Open Questions misattributes the consumer's
  location to "the conformance guard."

### Task wiring (regression gate)

- `test:integration:config` (`mise.toml:138-140` → `invoke test.integration.config`,
  `tasks/test/integration.py:46-64`) glob-runs **every** executable
  `scripts/**/test-*.sh` (`tasks/test/helpers.py:13-35`), with a count floor of
  16 and a by-name requirement for `test-skill-frontmatter-conformance.sh`.
  **Both** `test-validate-corpus-frontmatter.sh` and
  `test-skill-frontmatter-conformance.sh` run here — so new fixtures are
  automatically exercised, tying suite greenness to the new rules executing
  (the work item's fifth criterion).
- `test:unit:templates` (`mise.toml:115-117` → `tasks/test/unit.py:34-41`) runs
  an explicit driver list: `test-template-frontmatter.sh`,
  `test-skill-frontmatter-population.sh`, `test-metadata-helpers.sh`. The
  validator suites do **not** run here; `test-template-frontmatter.sh` (the
  other contract-sourcing surface) does.

## Discrepancies (work item vs source)

These were verified at the current revision and should be reconciled during
planning/implementation:

1. **Stale line references.** The work item's "Blind-spot detail (verified at
   source)" and References cite provenance at `:314-324` and the linkage loop at
   `:355-376`. Actual current locations: **provenance 295–305**, **linkage
   334–357** (confirmed by grep). The Technical Notes cite `:296-302`/`:303-307`
   (provenance — close/correct) and `:355-376` (linkage — stale), and Drafting
   Notes say `:295-307`/`:355-376` — internally inconsistent. The provenance
   refs are roughly right; the linkage refs are ~16 lines high.
2. **Wrong helper names.** The work item refers to bespoke helpers
   `assert_no_provenance_over_emission` and `assert_linkage_shape`. The real
   names are **`check_no_provenance_over_emission`** (`:180-188`) and
   **`check_linkage_quoted`** (`:190-209`), driven through `assert_check`.
3. **Misattributed diagnostic consumer.** The work item's Open Questions says
   reuse "avoids updating the `grep -qF "BAD-LINKAGE-SHAPE"` consumer in the
   conformance guard." The only literal grep consumer is in
   **`test-validate-corpus-frontmatter.sh:163`** (the corpus-validator suite's
   tamper test), not the conformance guard. The conclusion (reuse the code) is
   still sound; the location attribution is off.

None of these change the task's shape — they are reference-accuracy fixes that
will save the implementer a round of re-discovery.

## Code References

- `scripts/validate-corpus-frontmatter.sh:35-38` — `violation()` (sole
  diagnostic + counter mechanism).
- `scripts/validate-corpus-frontmatter.sh:295-305` — provenance block (forward
  rule + legacy-forbid template for the new reverse rule).
- `scripts/validate-corpus-frontmatter.sh:334-357` — typed-linkage shape loop
  (the quoted-tokens-only escape).
- `scripts/validate-corpus-frontmatter.sh:242,246` — where `anchored` and
  `linkkeys` are read from the schema arrays.
- `scripts/frontmatter-emission-rules.sh:34-35` — `FM_PROVENANCE_FIELDS`,
  `FM_FORBIDDEN_PROVENANCE_FIELDS`.
- `scripts/frontmatter-emission-rules.sh:88` — `FM_TYPED_REF_RE` (and
  `FM_SOURCE_TYPE_RE` at `:41`).
- `scripts/frontmatter-emission-rules.sh:52-58,94-100` — cardinality + vocab
  helpers (not currently used by the validator loop).
- `scripts/templates-schema.tsv:1` — header; `code_state_anchored` col 3,
  `typed_linkage_keys` col 7.
- `scripts/frontmatter-fixtures.sh:31-103` — `emit_valid`, `run_validator`,
  `assert_rejects` (specific-code check), `assert_accepts`.
- `scripts/test-validate-corpus-frontmatter.sh:64-81` — existing provenance +
  linkage fixtures; `:163` — direct `BAD-LINKAGE-SHAPE` grep.
- `scripts/test-skill-frontmatter-conformance.sh:176-224,312-320,404-422` —
  bespoke helpers, call sites, liveness self-tests; `:426-429` — no-re-encoded-
  contract meta-asserts.
- `tasks/test/integration.py:46-64`, `tasks/test/helpers.py:13-35`,
  `tasks/test/unit.py:34-41` — task wiring.

## Architecture Insights

- **Single-oracle contract with a deliberately split data source.** The rule
  *logic* lives in `validate-corpus-frontmatter.sh`; cross-cutting *data*
  (provenance bundles, typed-ref grammar, linkage vocabulary) in
  `frontmatter-emission-rules.sh`; per-type *facts* (anchoring, applicable
  linkage keys) in `templates-schema.tsv`. The work item's "keep it
  data-driven" constraint maps directly onto this: iterate `FM_PROVENANCE_FIELDS`
  and gate on `SCHEMA_ANCHORED`, never literal field names.
- **bash 3.2 floor everywhere** — parallel arrays instead of associative arrays,
  `case` instead of `declare -A`. New code must follow suit (no `${var,,}`, no
  assoc arrays); suspect the 3.2 floor first for any macOS-only failure.
- **One diagnostic code per violation class, specifics in the message.** The
  validator bundles `git_commit`/`branch` under one `FORBIDDEN-PROVENANCE` and
  `revision`/`repository` under one `MISSING-PROVENANCE`. Reusing
  `BAD-LINKAGE-SHAPE` for the unquoted sub-case (message text distinguishing it)
  matches this convention; a distinct `BARE-LINKAGE-VALUE` would be the only
  code splitting one class on a syntactic axis.
- **Specific-diagnostic assertions are already first-class** — `assert_rejects`
  takes the expected code and `grep -qF`s for it, so the third acceptance
  criterion ("not merely a non-zero exit") needs no new tooling.
- **Three-authority temporary state** is the thing being unwound: validator-
  doesn't-enforce → guard-does (bespoke helpers) → table-records (0103 audit).
  Folding both rules into the validator returns the contract to one authority;
  the guard helpers can then collapse to liveness or be deleted, and the 0103
  by-inspection axes become validator-checkable.

## Historical Context

- `meta/work/0103-audit-skill-frontmatter-emission-against-unified-schema.md` —
  the audit that surfaced both blind spots and introduced the bespoke guard
  helpers; the "0103 audit table" (AC1/AC2 auditable artifact) is embedded in
  this work item (~line 192), not a standalone file.
- `meta/research/codebase/2026-06-09-0103-skill-frontmatter-emission-audit.md` —
  maps the 11 validator axes and the provenance "iff" one-directionality;
  origin of the AC2 by-inspection table.
- `meta/plans/2026-06-09-0103-audit-skill-frontmatter-emission.md` — 0103 plan;
  details the runtime gate and the guard-helper precedent.
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` +
  `meta/research/codebase/2026-06-07-0070-meta-corpus-unified-schema-migration.md`
  — the migration that shipped the validator.
- `meta/work/0104-add-rejected-to-adr-status-vocabulary.md` (+ its research/plan)
  — sibling schema-vocab follow-on under 0057; touches the same contract files
  (`templates-schema.tsv`, `frontmatter-emission-rules.sh`), so 0104 and 0105
  should be merge-ordered rather than landed blind in parallel.
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` —
  parent epic.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`,
  `ADR-0034-typed-linkage-vocabulary.md`,
  `ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md` —
  the schema this oracle enforces.

## Related Research

- `meta/research/codebase/2026-06-09-0103-skill-frontmatter-emission-audit.md` —
  the direct predecessor; this document extends it with the verified current
  state of the two blind spots and the collapse surface.
- `meta/research/codebase/2026-06-02-0093-extend-templates-with-typed-linkage-slots.md`
  — typed-linkage slots in `templates-schema.tsv`.
- `meta/research/codebase/2026-06-11-0104-add-rejected-to-adr-status-vocabulary.md`
  — sibling contract-file change.

## Open Questions

- **Helper collapse: delete vs liveness-reduce?** Still an implementation choice
  (the work item's first Open Question). The "no re-encoded contract"
  meta-asserts (`test-skill-frontmatter-conformance.sh:426-429`) and the
  by-name suite requirement in `tasks/test/integration.py:21` constrain the
  shape but do not force the decision.
- **Cardinality awareness.** The shape loop ignores `fm_linkage_cardinality`
  (single vs list). The fix to pre-split on commas/brackets could optionally
  enforce that a `single`-cardinality key carries no list — but 0105 does not
  require it, so this is out of scope unless deliberately pulled in.
- **Should the work item's stale line references and helper names be corrected
  in 0105 itself** before planning, so the plan inherits accurate anchors?
  (Recommended — the body is the implementer's map.)
