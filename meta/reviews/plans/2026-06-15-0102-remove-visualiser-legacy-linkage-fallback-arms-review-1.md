---
type: plan-review
id: "2026-06-15-0102-remove-visualiser-legacy-linkage-fallback-arms-review-1"
title: "Plan Review: Remove Visualiser-Server Legacy Linkage Fallback Arms"
date: "2026-06-15T23:19:55+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-06-15-0102-remove-visualiser-legacy-linkage-fallback-arms"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, safety, compatibility, standards]
review_number: 1
review_pass: 2
tags: [migration, visualiser, frontmatter, linkage, cleanup, contract]
last_updated: "2026-06-16T07:19:46+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Remove Visualiser-Server Legacy Linkage Fallback Arms

**Verdict:** REVISE

This is an unusually disciplined contraction plan — the key/arm/field distinction
is correct, every naming trap is pre-empted, each phase is independently mergeable,
and the gate-first ordering makes migration completion an observable precondition.
The reservations are not about *what* is removed but about two systemic gaps: the
removal silently breaks un-migrated corpora while the only consumer-facing signal
(the deprecation warns) is retired in the same change, and the repeated
"DELETE or RETARGET" latitude risks quietly eroding coverage of code paths that
survive. Both are addressable with small, targeted edits to the plan rather than a
structural rethink.

### Cross-Cutting Themes

- **Silent breakage for un-migrated corpora, with the warning channel retired in
  the same change** (flagged by: Compatibility, Safety, Architecture) — Removing
  the legacy arms changes the failure mode from "resolves + `tracing::warn!`" (or,
  for `ticket:`, silent) to "silently does not resolve". The filename-fallback
  removal is the sharpest case: migration `0007` is what backfills `id:`, so an
  un-migrated work item that has only a filename-encoded id resolves to `None` and
  vanishes from the index. The in-repo `meta/` gate cannot observe external/
  userspace corpora, so the "every consuming repo has migrated" framing is stronger
  than what the gate structurally proves.

- **"DELETE or RETARGET" latitude threatens surviving-path coverage** (flagged by:
  Test Coverage, Code Quality) — Several tests pin *canonical logic* (`normalise_id`
  project-code/foreign-prefix, `id_from_value` path-shape branch, shape-validation
  rejection) through a legacy *key*. Only the key changes; the logic survives.
  Offering "delete" as an equal option to "retarget" invites the path of least
  resistance to drop coverage of live code. One retarget (shape-invalid) also
  *inverts* its assertion (fallback→`None`), which the plan does not flag.

- **"Revise the prose" defers the most error-prone edits** (flagged by: Code
  Quality) — The code changes ship as verified Rust snippets, but the doc-comment
  rewrites (identity chain `:1356-1364`, `read_ref_keys` `:312-320`) are left as
  "revise", with the inline `(was: …)` parenthetical re-narrating removed behaviour
  in surviving code.

- **Re-derivation grep over-matches body code-fences** (flagged by: Correctness,
  Safety) — `grep -rnE '^\s*ticket(_id)?:' meta/` surfaces fenced-example matches
  alongside the 10 frontmatter targets; the plan warns to confirm "line 5" by hand
  rather than computing the set unambiguously.

### Tradeoff Analysis

- **Clean contraction vs consumer observability**: Safety and Compatibility want a
  diagnostic when a work item resolves to `None` for lack of `id:`, pointing at
  `/accelerator:migrate`. The plan's intent is to *retire* the legacy/deprecation
  machinery. These are reconcilable: a one-line warn on the **surviving** `id:`-
  absent path (not the removed arms) is a new diagnostic on canonical code, not a
  re-expansion of the legacy arms — it resolves the silent-failure theme without
  compromising the contract. Recommended over leaving the failure silent.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Compatibility + Safety**: Filename-fallback removal silently breaks un-migrated corpora lacking `id:`
  **Location**: Phase 4 (indexer.rs); Migration Notes
  Collapsing the identity chain to `id:` → `None` means a work-item file that has
  only a filename-encoded id (migration `0007` not yet applied) resolves to nothing
  and silently disappears from the index — no error, no clustering, no detail page,
  and the `ticket:` arm never warned even during the expand phase. In-repo blast
  radius is nil (all 110 work-item files carry `id:`); the exposure is the
  post-upgrade/pre-migrate window for external consumers.

- 🟡 **Compatibility + Architecture**: In-repo gate is an in-repo-only proxy, overstated as a consumer guarantee
  **Location**: Implementation Approach; Testing Strategy ("observable proxy for every consuming repo has migrated")
  The gate proves *this* repo's corpus is clean; it is structurally blind to the
  external corpora that actually consume the released, version-locked binary. With
  migrate-on-use advisory and non-gated, nothing verifies a downstream corpus is
  migration-complete before it receives the arm-removed binary.

- 🟡 **Test Coverage**: Shape-invalid retarget inverts its assertion (fallback→`None`), but the plan doesn't say so
  **Location**: Phase 4 (indexer.rs — Tests)
  `work_item_id_frontmatter_shape_invalid_falls_back_to_filename` (~`:3570`)
  currently asserts a shape-invalid value falls back to the filename id. After the
  fallback is removed it must assert `→ None`. "Retarget to `id:`" mechanically
  keeps the `Some(...)` assertion and contradicts the new contract.

- 🟡 **Test Coverage + Code Quality**: Canonical-path coverage depends on choosing RETARGET over DELETE
  **Location**: Phase 3 / Phase 4 (Tests)
  Tests for `normalise_id` (project-code, foreign-prefix passthrough) and
  `id_from_value`'s `TypedRef::Path` branch exercise logic that *survives* — only
  the source key changes. Offered as "DELETE or RETARGET" with a soft preference,
  deletion would drop the only direct coverage of live code.

- 🟡 **Correctness**: "Whole-corpus run is clean today" is asserted but only the own-id clause is verified
  **Location**: Current State Analysis; Phase 1 Success Criteria
  The plan substantiates only `FORBIDDEN-OWN-ID` being at zero, but whole-corpus
  mode also runs `DANGLING-REF`, `MISSING-PROVENANCE`, `BAD-LINKAGE-SHAPE`, etc.
  over every legacy doc (incl. the 10 being scrubbed, several with old
  `repository:` provenance). If the run is *not* actually `exit 0` for an unrelated
  reason, Phase 1's success criterion fails even though the obsolete-key work is
  correct.

#### Minor

- 🔵 **Correctness**: Resolution shifts (not disappears) when a legacy key co-exists with `target:`
  **Location**: Phase 2 (read_ref_keys)
  For a doc with both a legacy key and `target:` (no `work_item_id:`), the resolved
  ref doesn't vanish — it switches from the legacy id to the parsed `target:` id.
  Intended and benign, but the manual-verification narrative only covers the "no
  longer resolves" case.

- 🔵 **Correctness + Safety**: Re-derivation grep matches body code-fences — compute the set unambiguously
  **Location**: Phase 1 §3 (corpus hygiene)
  `grep -rnE '^\s*ticket(_id)?:' meta/` returns fenced-example matches
  (`2026-04-21-list-and-update-tickets.md:172`, etc.). Filtering by an eyeballed
  "line 5" invites a mechanical deletion corrupting a documented YAML example that
  the frontmatter-only validator would never catch.

- 🔵 **Test Coverage**: Validator gate lacks a negative-discrimination case
  **Location**: Phase 1 §2 (Validator test coverage)
  The premise is that a current foreign `work_item_id:` reference must NOT be
  flagged, yet no test pins this. A future widening of `FM_OBSOLETE_LEGACY_KEYS` to
  `work_item_id` would wrongly flag the live corpus with no unit test to catch it.

- 🔵 **Test Coverage**: Precedence test (`work_item_id:` over `target:`) is implicitly relied upon
  **Location**: Phase 2 (read_ref_keys — Tests)
  After re-chaining `target:` directly off `work_item_id:`, the mutual exclusivity
  of those two arms is the one behavioural change; the guarding test
  (`read_ref_keys_prefers_work_item_id_alias_over_target`, ~`:618`) is in neither
  the KEEP nor DELETE list, so its survival is accidental.

- 🔵 **Test Coverage**: Reassessed `None`-test loses its multi-source intent
  **Location**: Phase 4 (`work_item_id_none_when_neither_frontmatter_nor_filename_matches`)
  Narrowed to "`id:` absent → `None`", it nearly duplicates the kept primary test
  and its name becomes misleading. Keep it as the negative contract and rename
  (e.g. `work_item_id_none_when_id_absent`).

- 🔵 **Code Quality**: Identity-resolution doc comment goes stale unless rewritten — specify the text
  **Location**: Phase 4 (indexer.rs `:1356-1364`)
  The current comment describes a three-source chain with per-source deprecation
  warns — all false after Phase 4. "Revise" + the `(was: …)` parenthetical
  re-narrate removed behaviour. Specify: `id:` is the sole source (via
  `normalise_id`); absence yields `None`.

- 🔵 **Code Quality**: `read_ref_keys` doc comment still enumerates removed keys — specify the text
  **Location**: Phase 2 (frontmatter.rs `:312-320`, `:343-345`)
  The doc/preference comments list `work-item:`/`ticket:`. The plan says "revise"
  without the corrected wording — the most sync-error-prone edit is left to
  implementation-time judgement.

- 🔵 **Code Quality + Architecture**: Validator now embeds multiple "forbidden key" policies — comment the distinction
  **Location**: Phase 1 §1
  `FM_OBSOLETE_LEGACY_KEYS` (forbid-anywhere) sits alongside the per-type
  `FORBIDDEN-OWN-ID` mechanism and the deliberately-tolerant `build_index` own-id
  fallback (`:213-216`). The divergence is justified but undocumented in the script,
  risking an inconsistent future edit.

- 🔵 **Compatibility**: No version-bump / BREAKING CHANGELOG guidance
  **Location**: Overview; Migration Notes
  The `ticket`→`work-item` rename was recorded BREAKING; this removal hard-requires
  migration `0007` in consuming repos, yet no version-bump expectation or BREAKING
  CHANGELOG entry is called out.

- 🔵 **Compatibility**: Loss of cross-ref aggregation for legacy keys is silent
  **Location**: Phase 2 (read_ref_keys)
  A doc linking via only a legacy `work-item:`/`ticket:` key stops aggregating into
  its work item's cross-refs/reverse index — and `ticket:` never warned. Acceptable
  for the contract step, but worth a Migration Notes line.

- 🔵 **Safety**: Gate skips out-of-scope/untyped paths — "anywhere" is narrower than stated
  **Location**: Phase 1 §1 / Success Criteria
  A file only reaches the obsolete-key check if it passes the `INVALID-TYPE` guard.
  An obsolete key in an untyped/unmapped subtree (`specs/`, `talks/`, `global/`)
  would be skipped — a fail-permissive gap relative to the "any type" claim.

- 🔵 **Architecture**: `capture_logs` couples the three "independent" phases — name an owner
  **Location**: Implementation Approach; Phases 2-4
  Independence holds only via the per-phase grep-and-conditionally-delete rule;
  `capture_logs` is the single cross-phase coupling point. Consider designating the
  indexer phase (last two callers) as the canonical owner of the fn deletion.

- 🔵 **Architecture**: `id:`-only narrows the contract with no degradation path — verify corpus universally carries `id:`
  **Location**: Phase 4
  A graceful-degradation property (filename-derived identity) is intentionally
  removed. Confirm the manual verification exercises a real synced file, not only a
  synthetic fixture.

- 🔵 **Standards**: Diagnostic message embeds a story number, unlike every sibling violation
  **Location**: Phase 1 §1
  `"… (migrated out; see story 0102)"` cites a story; no other violation message
  does. Drop the citation (keep provenance in the code comment), e.g.
  `"… present (use id:/typed references)"`.

- 🔵 **Standards**: Function-rename target left as either/or — pick the convention-matching name
  **Location**: Phase 3 §1
  `parent_id` reads like a field accessor and overloads the `*_id` vocabulary;
  `cluster_key_from_parent` describes what the function does. Commit to one and
  record it in the plan.

- 🔵 **Standards**: New array placement & `FM_` prefix ambiguity
  **Location**: Phase 1 §1
  Declare `FM_OBSOLETE_LEGACY_KEYS` at module scope with the other config arrays
  (not inside `validate_file`); the `FM_` prefix is owned by the sourced
  `frontmatter-emission-rules.sh`, so a locally-owned array may be mistaken for a
  helper-provided one.

#### Suggestions

- 🔵 **Code Quality**: `capture_logs` verification grep wording is self-contradictory ("zero matches or only the definition is gone") — state the unambiguous pass condition.
  **Location**: Phase 4 (Automated Verification)

- 🔵 **Test Coverage**: After the four deprecation tests are deleted, no test exercises the surviving shape-validation `warn!` (`indexer.rs:1372-1377`). Either retarget one `capture_logs` test to assert it fires on an invalid `id:` (which also retains `capture_logs`), or note the warn is intentionally untested.
  **Location**: Testing Strategy

- 🔵 **Safety**: The `capture_logs` rule is evaluated at edit time, not merge time — note that the integrating phase must re-run `grep -rn capture_logs src/` after any rebase/merge. (Fails loud under `-D warnings`, so contained.)
  **Location**: Implementation Approach

- 🔵 **Standards**: Keep the new check immediately after the `FORBIDDEN-OWN-ID` loop (contiguous forbidden-key block) and place the new fixtures in the `=== Failure-mode fixtures ===` section beside `bad-ownid`.
  **Location**: Phase 1 §1

### Strengths

- ✅ Correctly resolves the work item's open decision rule via the three-way
  key / arm / struct-field distinction, preventing an over-broad removal that would
  break `read_ref_keys`, the clustering resolver, and the patcher.
- ✅ Every phase is independently mergeable and leaves `mise run check` green; the
  order-independent `capture_logs` rule fails loud under `-D warnings` rather than
  silently.
- ✅ `target:` typed-linkage is correctly identified as independently load-bearing
  and preserved at all three sites; the ADR-0034 destination is protected.
- ✅ All naming traps (bare `warn!` import reused at `:61`, shape-validation warn at
  `:1372`, `work-item:` value-prefix in `typed_ref.rs`, `id_from_value`,
  `extract_id`) are explicitly flagged as keep-sites.
- ✅ The `read_ref_keys` re-link is shown as a concrete, correct snippet and
  preserves exact short-circuit semantics (both arms keyed on `m.get` presence).
- ✅ The new check reuses the existing `bk_present`/`violation`/`FORBIDDEN-OWN-ID`
  idiom and a bash-3.2-safe indexed array; `OBSOLETE-LEGACY-KEY` matches the
  established violation-code naming; the gate parses only the frontmatter fence, so
  body code-fences are excluded for free.
- ✅ Gate-first ordering makes migration completion a real CI-enforced precondition;
  a fail-loud manual probe (plant a key, confirm the gate fires) validates the
  signal.
- ✅ All re-derived line coordinates match current source, and the research's
  correction of the work item's mislabelled "retained-plan block" is internally
  consistent.

### Recommended Changes

1. **Add a consumer-facing diagnostic on the surviving `id:`-absent path**
   (addresses: filename-fallback silent breakage; in-repo-only proxy; silent
   cross-ref loss) — Emit a one-line index-time `warn!` when a `WorkItems` doc
   resolves to `None` for lack of `id:`, pointing at `/accelerator:migrate`. This is
   a new diagnostic on canonical code, not a re-expansion of the legacy arms, and
   converts the silent failure into an observable one for un-migrated corpora.

2. **Make RETARGET the default and specify the inverted/retargeted assertions**
   (addresses: shape-invalid inversion; canonical-path coverage; reassessed
   `None`-test) — For each named legacy-key test that exercises surviving logic,
   mandate retargeting to `id:`/`parent:` (delete only tests whose sole purpose was
   the legacy arm). Explicitly state the shape-invalid test now asserts
   `id: "PROJ-1.2"` → `None`, and rename the narrowed `None`-test.

3. **Verify and record the whole-corpus baseline** (addresses: "clean today"
   asserted but unverified) — Run `bash scripts/validate-corpus-frontmatter.sh
   meta/` once and record the actual exit status in the plan, rather than inferring
   it from the own-id clause alone.

4. **Compute the scrub set unambiguously** (addresses: re-derivation grep matches
   body fences) — Replace the eyeballed "line 5" check with a frontmatter-only
   command (e.g. `awk` that acts only inside the leading `---` fence) so no body
   code-fence example can be corrupted.

5. **Specify the doc-comment replacement text** (addresses: stale identity comment;
   stale `read_ref_keys` comment) — Inline the corrected doc comments in the plan
   and drop the `(was: …)` parenthetical so no removed behaviour is re-described in
   surviving code.

6. **Pin the precedence and discrimination tests** (addresses: implicit precedence
   test; missing negative-discrimination case) — Add
   `read_ref_keys_prefers_work_item_id_alias_over_target` to the explicit KEEP list,
   and add a validator fixture asserting a current foreign `work_item_id:` reference
   is NOT flagged.

7. **Add release/versioning + scope notes** (addresses: version-bump guidance;
   gate skips untyped paths; validator policy divergence; rename ambiguity; story
   number in message; array placement) — A Migration Notes line on the BREAKING
   CHANGELOG entry + version, a note on the gate's typed-path coverage boundary, an
   in-script comment distinguishing the forbidden-key mechanisms, a committed rename
   (`cluster_key_from_parent`), removal of the story number from the diagnostic, and
   module-scope placement of the new array.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: A textbook contract-phase plan: it removes four dead legacy
resolution arms while precisely preserving the load-bearing `work_item_id:` key,
`target:` typed-linkage path, and shared helpers, with each removal grounded in
verified consumer analysis. Main reservations: the gate's role as a *proxy* for
external migration completion, and the cross-phase `capture_logs` coupling.

**Strengths**:
- Correctly resolves the open decision rule via the key / arm / struct-field
  distinction.
- Each phase independently mergeable with `mise run check` green; VCS-revertible.
- `target:` correctly preserved at all three sites as ADR-0034 destination.
- Naming traps explicitly flagged as keep-sites.
- Gate reuses existing frontmatter-only validator seam (no body false positives).

**Findings**:
- 🔵 minor / high — *Shared `capture_logs` test-support dependency couples the three
  otherwise-independent server phases* (Implementation Approach; Phases 2-4). The
  independence relies on a per-phase grep-and-conditionally-delete rule; consider
  designating the indexer phase as the canonical owner of the fn deletion.
- 🔵 minor / medium — *Migration-completion gate observes only the in-repo corpus*
  (Phase 1; Desired End State). External repos still emitting legacy frontmatter are
  invisible; the gate's name presents a stronger guarantee than it provides, and the
  deprecation-warn channel is retired in the same change.
- 🔵 minor / medium — *`build_index`'s legacy own-id fallback survives the gate,
  creating divergent notions of legacy-key tolerance in one file* ("What We're NOT
  Doing"). Add a comment cross-referencing the three policies.
- 🔵 minor / low — *Collapsing identity to `id:`-only narrows the contract with no
  degradation path for filename-only files* (Phase 4). Confirm manual verification
  exercises a real synced file.

### Code Quality

**Summary**: A disciplined deletion plan: removes dead branches and their pinning
tests, collapses multi-arm chains to single canonical paths, renames a now-
misleading function. The `capture_logs` rule, `-D warnings` interaction,
`read_ref_keys` re-chain snippet, and rename are correctly reasoned against actual
source. Residual risks: stale prose, the retarget-vs-delete ambiguity, and under-
specified comment updates.

**Strengths**:
- `capture_logs` rule sound; `test_support` survives via `build_test_json_subscriber`.
- `read_ref_keys` re-chain shown as concrete correct snippet.
- Every lookalike trap correctly flagged as do-not-touch.
- Rename well-motivated and tied to doc-comment cleanup.
- Dead branch + its pinning test removed together.

**Findings**:
- 🔵 minor / high — *"DELETE or RETARGET" leaves a coverage-erosion gap for surviving
  paths* (Phases 2-4 Tests). Make retargeting the default; reserve deletion for
  tests whose entire reason to exist was the legacy arm.
- 🔵 minor / high — *Identity doc comment will go stale unless rewritten, not just
  "revised"* (Phase 4, `:1356-1364`). Specify the replacement; drop the `(was: …)`
  parenthetical.
- 🔵 minor / medium — *`read_ref_keys` doc comment still enumerates removed keys; plan
  defers wording* (Phase 2, `:312-320`/`:343-345`). Inline the corrected comment.
- 🔵 minor / medium — *New obsolete-key list is a separate concept from forbidden-own-id
  — keep the two clearly distinct* (Phase 1). Keep the rationale comment in the
  script itself.
- 🔵 suggestion / medium — *`capture_logs` verification grep wording is ambiguous about
  the pass condition* (Phase 4 Automated Verification). State the unambiguous
  expectation.

### Test Coverage

**Summary**: Unusually disciplined for a deletion-heavy change: explicit TDD
ordering, named canonical-path survivor tests, and both-feature-mode execution
pinned into every phase. Central residual risk: several "DELETE or RETARGET"
decisions are left to implementer discretion without specifying retargeted
assertions, so canonical-path coverage could be silently lost; one retarget inverts
its assertion. The gate test lacks a negative-discrimination case.

**Strengths**:
- TDD ordering explicit and correct (edit test module first, then remove, then green).
- Both feature modes pinned into every server phase's Automated Verification.
- Survivor tests that become the post-removal contract are named.
- Order-independent `capture_logs` dead-code rule well-reasoned.
- Phase 1 adds genuine new gate coverage + a fail-loud observability probe.

**Findings**:
- 🔴(severity: major) / high — *Shape-invalid retarget inverts its assertion
  (fallback→None) but the plan does not say so* (Phase 4). Specify `id: "PROJ-1.2"`
  → `None` and that the shape-validation warn still fires.
- 🟡 major / medium — *Canonical-path coverage (normalise_id branches, path-shape
  branch) depends on choosing RETARGET over DELETE* (Phases 3-4). Mandate retarget
  for these specific tests.
- 🔵 minor / medium — *Gate test lacks a negative-discrimination case (foreign
  `work_item_id` not flagged)* (Phase 1 §2). Add a fixture asserting acceptance.
- 🔵 minor / high — *Precedence coverage of `work_item_id:` over `target:` is silently
  relied upon* (Phase 2). Add `read_ref_keys_prefers_work_item_id_alias_over_target`
  to the KEEP list.
- 🔵 minor / medium — *Reassessed `None`-test loses its multi-source intent and risks
  redundancy* (Phase 4). Keep as negative contract, rename to `..._when_id_absent`.
- 🔵 suggestion / medium — *No assertion that the surviving shape-validation warn is
  still exercised after the deprecation tests are deleted* (Testing Strategy).

### Correctness

**Summary**: The core logical surgery is sound — the `read_ref_keys` re-link
preserves exact short-circuit semantics (both arms keyed on `m.get` presence, not
value extraction), and the two identity-chain collapses are clean given verified
helper liveness. The new obsolete-key check is correctly scoped (`bk_present` is
exact-match; only the frontmatter fence is parsed). The weakest claim is the
unqualified "whole-corpus run is already `exit 0`", which substantiates only the
own-id clause.

**Strengths**:
- `read_ref_keys` re-link preserves exact mutual-exclusion/short-circuit behaviour.
- Helper liveness (`id_from_value`, `extract_id`, shape-validation warn) verified.
- Obsolete-key check cannot confuse `ticket`/`ticket_id`/`work_item_id`; body fences
  excluded.
- All re-derived coordinates match source; the "retained-plan block" correction is
  internally consistent.

**Findings**:
- 🟡 major / medium — *"Whole-corpus run is clean today" is asserted but only the own-id
  clause is verified* (Current State; Phase 1 Success Criteria). Run it once and
  record the exit status.
- 🔵 minor / high — *Resolution shifts (not just disappears) when a legacy key co-exists
  with `target:`* (Phase 2). Note the `target:` shift in the manual-verification
  bullet.
- 🔵 minor / medium — *Obsolete-key check skips indented frontmatter keys
  (`parse_fm` constraint)* (Phase 1 §1). Acceptable (top-level keys only); state the
  scope limit.
- 🔵 minor / high — *Re-derivation grep pattern matches body code-fences and must be
  filtered by fence position* (Phase 1 §3). Tighten to a line-5/inside-fence filter.

### Safety

**Summary**: A low-criticality developer-tooling change structured with appropriate
safety discipline: gate-first precondition, independently mergeable phases, a
bounded 10-line corpus mutation, and VCS revert as the stated recovery path. The one
genuine accidental-harm risk is operational, not data-loss: removing the silent
legacy arms changes the failure mode for un-migrated external corpora from
"resolves + warn" to "silently does not resolve". In-repo blast radius verified nil
(all 110 work-item files carry `id:`; all 10 scrub targets have the line at
frontmatter line 5).

**Strengths**:
- Gate-first ordering makes migration completion a real CI-enforced precondition.
- Destructive mutation bounded, mechanical, VCS-reversible, re-derived before edit.
- Per-phase independent mergeability fails safe under `-D warnings`.
- New check operates only on the parsed frontmatter fence (no body false positives).
- Manual verification includes a fail-loud probe.

**Findings**:
- 🔵 minor / high — *Removing the arms changes the failure mode to silent invisibility
  for un-migrated external corpora* (Overview; Phase 4; Migration Notes). Confirm a
  BREAKING CHANGELOG entry; consider a binary-side warn.
- 🔵 minor / high — *Re-derivation command surfaces body-code-fence matches alongside
  the 10 targets* (Phase 1 §3). Tighten to a frontmatter-only command.
- 🔵 minor / medium — *Gate skips files that fail the `INVALID-TYPE` guard, so "anywhere,
  any type" is narrower than stated* (Phase 1 §1). Add a test for an obsolete key in
  an out-of-scope/untyped path.
- 🔵 suggestion / medium — *`capture_logs` rule is evaluated at edit time, not merge
  time* (Implementation Approach; Phases 2-4). Note the post-rebase re-check (fails
  loud, so contained).

### Compatibility

**Summary**: An intentional, well-reasoned breaking change (the contract phase) with
a fundamentally sound coordination story: the binary version is hard-locked to the
plugin version, so a consumer can never run a newer arm-removed binary against older
migrations — binary and migrations always ship together. The genuine exposure is the
post-upgrade/pre-migrate window: an un-migrated work-item corpus lacking `id:`
(migration `0007` backfills it) loses identity once the filename fallback is removed.
The in-repo gate is a sound proxy for *this* repo but structurally blind to external
corpora.

**Strengths**:
- Retained-vs-removed contract surface precisely and correctly drawn.
- Binary/plugin version coherence enforced mechanically.
- Correctly identified as the contract phase of a deliberate deprecation sequence.
- Degrades gracefully (`None`, not panic).

**Findings**:
- 🔴(severity: major) / high — *Filename-fallback removal silently breaks un-migrated
  external corpora that lack `id:`* (Phase 4; Migration Notes). Add a startup/index-
  time diagnostic + a Migration Notes requirement for migration `0007`.
- 🟡 major / high — *In-repo `meta/` gate cannot observe external corpora; proxy claim
  overstated for consumers* (Implementation Approach; Testing Strategy). Reframe the
  gate's scope; pair with a consumer-facing safeguard.
- 🔵 minor / medium — *Breaking removal lacks explicit version-bump / BREAKING CHANGELOG
  guidance* (Overview; Versioning). Add a Migration Notes line.
- 🔵 minor / medium — *Loss of cross-reference aggregation for legacy `work-item:`/
  `ticket:` keys is silent* (Phase 2). Note in Migration Notes; `ticket:` never
  warned.

### Standards

**Summary**: Strongly aligned with project conventions: the validator extension
reuses the existing `bk_present`/`violation`/`FORBIDDEN-OWN-ID` idioms, the
parallel-array (bash-3.2-safe) pattern, the TDD fixture style, and the Rust
doc-comment/rename discipline. `OBSOLETE-LEGACY-KEY` follows the hyphenated-uppercase
naming and the bash-4 floor is respected. A few minor deviations: a story number in
a diagnostic, an unspecified rename target, and unstated array placement.

**Strengths**:
- Proposed validator block mirrors the adjacent FORBIDDEN-OWN-ID idiom exactly.
- `FM_OBSOLETE_LEGACY_KEYS` is a literal indexed array — bash-3.2-compatible.
- `OBSOLETE-LEGACY-KEY` matches the violation-code naming convention.
- Test plan commits to the suite's `emit_valid`/`assert_rejects` conventions.
- Rust changes consistently revise doc comments alongside the code.

**Findings**:
- 🔵 minor / high — *Diagnostic message embeds a story number, unlike every other
  validator violation* (Phase 1 §1). Drop the citation; keep provenance in the code
  comment.
- 🔵 minor / medium — *Function-rename target left as either/or; pick the convention-
  matching name* (Phase 3 §1). Prefer `cluster_key_from_parent`.
- 🔵 minor / medium — *New top-level array should sit with the other config arrays and
  note its bash-3.2 rationale* (Phase 1 §1). Beware the `FM_` prefix owned by
  `frontmatter-emission-rules.sh`.
- 🔵 suggestion / medium — *Keep the new check immediately after FORBIDDEN-OWN-ID and
  place fixtures in the failure-mode section* (Phase 1 §1).

---

## Re-Review (Pass 2) — 2026-06-16T07:19:46+00:00

**Verdict:** APPROVE

The seven lenses re-ran against the revised plan. All five major findings from
the initial review are **resolved**. The pass surfaced one genuine *new* major
(missed in pass 1) — the `capture_logs` removal rule omitted the support
infrastructure that goes dead with it, which would fail `-D warnings` — now fixed.
Two safety "majors" alleging the gate is not CI-enforced were **rejected after
factual verification**: the gate *is* CI-enforced. With the new major fixed and the
remaining minor refinements folded in, the plan is ready for implementation.

### Previously Identified Issues

- 🟡 **Compatibility + Safety**: Filename-fallback removal silently breaks
  un-migrated corpora lacking `id:` — **Resolved** (documented as accepted breakage
  in a new Migration Notes subsection, per the team decision; failure mode,
  version-lock, and nil in-repo blast radius all characterised).
- 🟡 **Compatibility + Architecture**: In-repo gate overstated as a consumer
  guarantee — **Resolved** (reframed as "this repo's corpus is migration-complete —
  a release prerequisite, not a cross-repo guarantee" in Testing Strategy +
  Implementation Approach + Migration Notes).
- 🟡 **Test Coverage**: Shape-invalid retarget inverts its assertion (fallback→None)
  but the plan didn't say so — **Resolved** (Phase 4 now explicitly mandates the
  retargeted test assert `id: "PROJ-1.2"` → `None`, with the warn still firing).
- 🟡 **Test Coverage + Code Quality**: Canonical-path coverage depends on RETARGET
  over DELETE — **Resolved** (RETARGET is now the default; project-code,
  foreign-prefix, and `TypedRef::Path` tests are mandated re-pointed, not deleted,
  with concrete `id:`/`parent:` examples).
- 🟡 **Correctness**: "Whole-corpus run is clean today" asserted but only own-id
  clause verified — **Resolved** (ran it: `exit 0` across all clauses, recorded in
  Current State Analysis; re-review further clarified the pre-change vs post-check
  contingency).

All minor/suggestion findings from pass 1 were also addressed (doc-comment text
specified, precedence test pinned to KEEP, negative-discrimination + untyped-path
fixtures added, scrub re-derivation tightened, rename committed to
`cluster_key_from_parent`, story number dropped, array relocated to module scope,
`capture_logs` ownership named).

### New Issues Introduced

- 🟡→✅ **Architecture + Code Quality (major, high)**: *`capture_logs` removal rule
  omitted its exclusive support block.* `log.rs:102-164` (two `use` imports,
  `CAPTURE_BUF`, `ThreadLocalWriter`/`ThreadLocalMakeWriter` + impls, `CAPTURE_INIT`)
  is used *only* by `capture_logs`; removing just the fn leaves it dead and fails
  `-D warnings`. **Fixed this pass** — the rule now removes the entire capture
  harness as a unit, retaining `build_test_json_subscriber` (used at `log.rs:295`).
  *This was a real defect the first review missed; it is the primary value of the
  re-review pass.*
- ❌ **Safety (2× major)**: *"The whole-corpus gate is not wired into CI" / "removal
  phases gated on a precondition CI cannot verify."* **Rejected — factually
  incorrect.** `test-validate-corpus-frontmatter.sh:184-191` runs the validator
  against the **real** `$ROOT/meta` corpus and `assert_eq`s exit 0, and that suite
  runs under `test:integration:config` ⊂ `mise run check`. A missed `ticket: null`
  fails CI. The safety lens missed lines 184-191; the compatibility lens read it
  correctly. The plan now makes this enforcement mechanism explicit and recommends
  naming the suite in `_REQUIRED_CONFIG_SUITES` so it is anchored by identity, not
  only the count floor.
- 🔵 **Minor refinements** (Test Coverage, Standards, Correctness, Code Quality,
  Compatibility): validator-fixture construction details for cases (a)/(c)/(d)
  pinned; the obsolete-key boundary comment softened to "every typed/type-inferable
  doc"; a single deterministic `awk` scrub command + full-`jj diff` verification
  specified; the `b2f39a4` exit-0 claim clarified as pre-change; the
  `ACCELERATOR_VISUALISER_BIN`/`visualiser.binary` override-path caveat and the
  symptom-naming CHANGELOG wording added. All folded in this pass.

### Assessment

The plan is in good shape and ready for implementation. The re-review earned its
keep by catching the `capture_logs`-support-block defect (a genuine CI-breaker the
first pass overlooked) and by resolving a direct safety-vs-compatibility
contradiction through source verification rather than splitting the difference. No
unaddressed major findings remain; the two residual accepted trade-offs (silent
un-migrated-corpus breakage; the surviving shape-validation warn left untested by
default) are explicit, documented decisions rather than gaps.
