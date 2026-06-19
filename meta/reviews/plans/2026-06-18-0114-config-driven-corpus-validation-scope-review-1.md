---
type: plan-review
id: "2026-06-18-0114-config-driven-corpus-validation-scope-review-1"
title: "Plan Review: Config-Driven Corpus Validation Scope"
date: "2026-06-18T18:40:21+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-18-0114-config-driven-corpus-validation-scope"
reviewer: "Toby Clemson"
verdict: REVISE
lenses: [architecture, correctness, code-quality, test-coverage, portability, safety, compatibility, performance]
review_number: 1
review_pass: 2
tags: [validator, frontmatter, config, doc-type-inference, allowlist]
last_updated: "2026-06-19T00:10:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Config-Driven Corpus Validation Scope

**Verdict:** REVISE

The denylist→allowlist inversion is the right architectural move (fail-closed
is the correct default for a plugin whose downstream consumers invent their own
`meta/` subtrees), the phasing is genuinely incremental and independently
mergeable, and the plan reuses the existing config machinery rather than
inventing a registry. However, the review surfaced **four critical issues** that
must be resolved before implementation: a path-form mismatch (project-relative
configured dirs vs the absolute paths both consumers actually walk) that could
silently skip the entire corpus; a degenerate-matching hazard from empty/odd
config values; the absence of a non-empty-table guard before the 0007 migration
mutates frontmatter in place; and the fact that in the migration `infer_type_from_path`
drives the *type written into frontmatter*, not just scope — so config-aware
inference would break a shipped migration's historical reproducibility. Several
of these reinforce each other across lenses.

### Cross-Cutting Themes

- **Path-form mismatch: relative configured dirs vs absolute walked paths**
  (flagged by: correctness 🔴, architecture 🟡, portability 🟡) — The resolver
  emits project-relative dirs (`meta/work`), but the validator is invoked as
  `validate-corpus-frontmatter.sh "$(pwd)/meta"` and the migration walks
  `find "$META_ABS"` — both absolute. The plan's match contract ("`$1` is
  `*/D/*` or `D/*`") only reliably fires the `*/D/*` arm for absolute inputs;
  the `D/*` arm is unreachable in production. If an implementer anchors to the
  start of `$1`, *every* file classifies out-of-scope → fail-closed total
  silent skip, while every relative-path unit fixture passes. This is the
  single highest-risk item.

- **Empty / odd config values degenerate the matcher** (correctness 🔴 + 🟡,
  test-coverage 🔵, portability 🔵) — `config-read-value.sh` returns the empty
  string (not the default) when a key is present but blank (`:126-130`). An
  empty `DOC_TYPE_DIRS` entry plus a `*/D/*` pattern degenerates to `*//*`
  (matches everything) or matches nothing. Glob metacharacters, whitespace, and
  trailing slashes in config values similarly corrupt `case`-glob matching.

- **Migration scope vs type-derivation, and the no-mutation guard** (safety 🔴,
  compatibility 🔴) — Two distinct criticals on the same consumer: (1) the
  migration mutates frontmatter at ten sites; with a fail-closed empty table it
  would classify the whole corpus out-of-scope and exit 0 having migrated
  nothing — indistinguishable from a clean idempotent re-run, no guard. (2)
  `infer_type_from_path`'s return value *is* the `type` written into each file
  and drives schema lookups (`0007-…sh:319,404`), so config-aware inference
  changes a shipped migration's historical output for any consumer with path
  overrides.

- **Project-root resolution divergence** (safety 🟡, compatibility 🟡) —
  `config-read-value.sh` discovers config CWD-relative via `config_find_files`;
  the migration operates on a canonicalised `PROJECT_ROOT` (`pwd -P`) that need
  not equal CWD. The resolver offers no root seam, so the table can be resolved
  against the wrong (or default) config.

- **Fallback equivalence is not genuinely pinned** (test-coverage 🟡 + 🟡,
  compatibility 🟡) — "byte-for-byte unchanged" is asserted by a *new* test
  written alongside the refactor, and the matching discipline changes from
  case-arm order to longest-match. A transcription error would make
  implementation and test agree on the wrong answer, defeating the
  independent-mergeability safety argument.

- **Registry coherence / drift guards incomplete** (architecture 🟡,
  code-quality 🟡, correctness 🔵, test-coverage 🔵) — The Phase 1 drift guard
  checks only `DOC_TYPE_NAMES` vs the TSV in one direction. It does not pin
  `DOC_TYPE_PATH_KEYS ⊆ PATH_KEYS`, the index-coupling/equal-length of the two
  parallel arrays, the exclusion of `global`, or equal-length tie-breaks.

### Tradeoff Analysis

- **Config-awareness vs migration reproducibility**: Making inference fully
  config-aware is cleaner and fixes the latent override-ignoring bug for the
  *validator*. But for the *migration*, the same inference result is baked into
  emitted frontmatter, and a shipped migration must reproduce its historical
  output forever. Recommendation: **split the concern in the migration** — keep
  *type derivation* on the fixed default mapping (historical contract intact)
  and use the config-aware table only for the *scope* (`out_of_scope`) decision.
  The validator can be fully config-aware; the migration cannot, without
  breaking idempotency for override users.

### Findings

#### Critical

- 🔴 **Correctness**: Project-relative configured dirs vs arbitrary-prefix find paths break segment matching
  **Location**: Phase 2 §1; Phase 3/4 resolve+inject
  Resolver emits relative dirs; consumers feed absolute paths. The `D/*` arm never fires for absolute input; a start-anchored implementation skips the whole corpus (fail-closed) while relative fixtures pass. Specify `*/D/*` OR `D/*` and add an absolute-root test.

- 🔴 **Correctness**: Empty configured path value produces a degenerate dir that matches everything or nothing
  **Location**: Phase 1 §2 (resolver); Phase 2 (out_of_scope)
  `config-read-value.sh:126-130` returns `""` for a present-but-blank key. `*/""/*` → `*//*`. Resolver should reject/skip empty resolved dirs or fall back to the registry default; add a `paths.work:` empty test. Mirror the migration's existing `assert_safe_relpath` guard.

- 🔴 **Safety**: No non-empty guard on the resolved table before the migration mutates frontmatter
  **Location**: Phase 4 §1
  A silent resolution failure → fail-closed everything-out-of-scope → migration exits 0 having mutated nothing, looking like a clean re-run. Assert the table has the expected 13 rows (or >0) and abort loudly before any mutation, co-located with the existing `fm_assert_schema_columns` pre-mutation guard.

- 🔴 **Compatibility**: Config-aware inference changes the 0007 migration's historical emitted output
  **Location**: Phase 4
  `infer_type_from_path`'s return is the `type` written into regenerated frontmatter and drives schema lookups (`:319,404`); the migration documents a reproduce-forever contract (`:18-22`). Keep type-derivation on the fixed default map; use the config table only for scope. (See Tradeoff Analysis.)

#### Major

- 🟡 **Architecture**: Path-form mismatch — resolved dirs project-relative, consumers walk absolute (the architecture framing of the correctness critical; make the normalisation contract explicit at the injection boundary).
  **Location**: Phase 2 §1
- 🟡 **Architecture / Code Quality**: New `config-defaults.sh ↔ templates-schema.tsv` coupling and the index-coupled parallel arrays are under-guarded — drift test is one-directional; nothing pins `DOC_TYPE_PATH_KEYS ⊆ PATH_KEYS`, equal length, or the name↔key pairing.
  **Location**: Phase 1 §1
- 🟡 **Correctness / Compatibility**: Announcements-arm removal in Phase 3 mutates the *shared fallback denylist* while the migration is still in fallback mode — a cross-consumer change landing in the wrong phase.
  **Location**: Phase 3 §2
- 🟡 **Correctness**: Two doc-types configured to the same directory has an undefined (iteration-order) tie-break; define and test it.
  **Location**: Phase 2 §1
- 🟡 **Correctness**: Glob metacharacters / whitespace in config values corrupt `case`-glob matching; match dirs as literals, add metachar tests.
  **Location**: Phase 2 §1
- 🟡 **Code Quality**: Implicit global-array injection turns documented "pure functions" into stateful mode-switching collaborators with no injected-vs-not signal; make the contract explicit and fix the header comment.
  **Location**: Phase 2 §1
- 🟡 **Test Coverage**: Migration "processed/skipped set unchanged" equivalence has no automated guard (manual dry-run only) — add a snapshot/diff test over a fixture corpus with all doc-type dirs + out-of-scope subtrees.
  **Location**: Phase 4 manual verification
- 🟡 **Test Coverage**: "Fallback byte-for-byte unchanged" is pinned only by a fresh test, not against actual pre-refactor output — golden-snapshot the current helper's output, especially order-sensitive `reviews/prs`-vs-`prs`.
  **Location**: Phase 2 automated verification
- 🟡 **Test Coverage**: Config-override fixture could pass for the wrong reason (override silently ignored) — assert *both* halves (flagged under configured dir AND skipped under default dir).
  **Location**: Phase 1 + Phase 3
- 🟡 **Portability**: Longest-match risks a non-portable string-length/`sort`/`${//}` construct — specify integer `${#dir}` comparison in a loop; replay on the 3.2 floor.
  **Location**: Phase 2 §1
- 🟡 **Portability**: Phase 6 awk `-v` table parsing must avoid gawk-only features (macOS ships BWK awk) — constrain to single-char separators, no `gensub`/`length(arr)`; run the alignment test under macOS awk.
  **Location**: Phase 6 §1
- 🟡 **Safety / Compatibility**: Resolver re-derives its own project root; config resolution is CWD-bound while the migration uses a canonical `PROJECT_ROOT` — give the resolver an explicit root seam; test CWD≠root and symlinked checkout.
  **Location**: Phase 1 §2 / Phase 4 §1
- 🟡 **Safety**: A misconfigured `paths.*` value can pull a freeform directory into the in-place mutation set — document/test the "configured dir holds only that type" assumption; surface newly-in-scope files in the dry-run.
  **Location**: Phase 4 / Phase 6
- 🟡 **Compatibility**: Fallback (case-arm order) vs injected (longest-match) equivalence on the default tree is non-trivial — add a full-corpus equivalence test across both paths, not just spot cases.
  **Location**: Phase 2 §1

#### Minor / Suggestions

- 🔵 **Architecture**: Third directory→type encoding (awk `path_to_typed`) remains a permanent drift surface; if Phase 6 is deferred, correct the alignment-test/header comment so the invariant isn't over-claimed (agreement holds only under default config).
- 🔵 **Architecture**: Walk-all-then-filter means corpus boundary = full walk ∩ resolved table; confirm Desired End State documents that a typed file outside all configured dirs is silently unvalidated.
- 🔵 **Code Quality**: Phase 5 (fallback + dead-denylist removal) is the only thing retiring the two-headed classifier but has no stated mandatory trigger — state it must land with Phases 3–4 (unlike deferrable Phase 6) and automate the grep guard.
- 🔵 **Code Quality**: New resolver's TSV output contract diverges from sibling `config-read-all-paths.sh` (markdown) and its exclusion set differs (also excludes `global`) — document the contrast; derive the allowlist from `DOC_TYPE_PATH_KEYS` so "excludes global too" is self-evident.
- 🔵 **Correctness**: Nested configured dirs need trailing-slash normalisation before longest-match; normalise in the resolver, add a trailing-slash nested-dir test.
- 🔵 **Correctness**: Extend the drift guard to assert every `DOC_TYPE_PATH_KEYS` entry exists in `PATH_KEYS` (a typo'd key resolves to empty → the empty-dir hazard, no test trips).
- 🔵 **Correctness**: Make explicit that array population happens once before `build_index` and is immutable for the run, so the index and validate passes observe identical scope.
- 🔵 **Test Coverage**: Index-scoping change lacks a negative assertion — add a whole-corpus case where a typed ref targets an out-of-scope file and assert `DANGLING-REF`.
- 🔵 **Test Coverage**: Resolver test should assert the exclusion of `global`/`templates`/`tmp`/`integrations` as an automated negative, not just a manual check.
- 🔵 **Test Coverage**: Keep the awk alignment test's literal `type:id` expectations (don't make it tautological by driving both sides from one table) and add the non-default probe alongside.
- 🔵 **Portability**: Mandate `printf '%s\t%s\n'` emission + `IFS=$'\t' read -r` consumption for the resolver's TSV lines.
- 🔵 **Portability**: State the match pattern literally as `case "$f" in */"$dir"/*)` (quoted, slash-anchored) so the 3.2 replay verifies the exact construct.
- 🔵 **Performance**: Reword Performance Considerations — the cost is 13 `config-read-value.sh` *forks* (each re-sourcing the config library), not "VCS detections"; confirm the new resolver loops keys in-process (collapsing to one fork).
- 🔵 **Performance**: Make explicit the table is resolved once at top-level scope (not per walk pass / per migration phase); optionally assert a constant resolver-spawn count.
- 🔵 **Performance**: Ensure the O(13)-per-file match uses bash pattern matching, no `grep`/`sed`/`awk` per array entry inside the loop (it runs twice per file across the walk).

### Strengths

- ✅ Correct fail-closed default: replacing a fail-open denylist with an allowlist is the right model for a plugin consumed by projects that invent their own `meta/` subtrees; Phase 5 explicitly asserts the fail-closed property.
- ✅ Disciplined, genuinely independently-mergeable phasing: additive registry → backward-compatible injection → per-consumer flip → dead-code removal, each merge point green and bounded.
- ✅ Strong DRY improvement: the allowlist is derived from the existing `PATH_KEYS`/config-resolution machinery rather than a new registry, and the new mapping is placed in `config-defaults.sh`, the canonical home for path keys.
- ✅ Preserves the functional-core/imperative-shell separation: `doc-type-inference.sh` stays pure and bash-3.2 safe; config resolution is pushed to the consumer shells via injection.
- ✅ Correctly identifies the precise matching hazards by name (most-specific match for `reviews/prs` vs `prs`; segment-boundary safety against `meta/prs-archive`) and removes the latent case-arm-ordering fragility.
- ✅ Test-conscious throughout: each phase names concrete test scripts and run commands, lists cross-phase "still passes" regressions, and adds a drift guard and a fail-closed guard.
- ✅ Relies correctly on the migration's existing VCS-based safety nets (clean-tree refusal, zero-mutation precondition pre-pass, `cmp -s` idempotency, atomic writes) as the recovery path.
- ✅ Proportionate performance stance: the corpus is a few hundred files, so per-file O(13) matching and one-time startup resolution are negligible; walk-narrowing is correctly deferred.

### Recommended Changes

1. **Define the path-normalisation contract explicitly** (addresses: Correctness 🔴 path-form; Architecture 🟡; Portability 🟡). State the form of both `DOC_TYPE_DIRS` entries and the `$1` argument; either normalise `$f` to project-relative before matching or commit to `*/D/*` OR `D/*` with both arms. Add unit + integration tests that feed an **absolute-prefixed root** (not just relative fixtures) to both the classifier and both consumers.

2. **Harden the resolver against degenerate config values** (addresses: Correctness 🔴 empty value, 🟡 metachars, 🔵 trailing slash; Portability 🔵 TSV). Reject/skip empty resolved dirs (or fall back to the registry default); normalise leading/trailing slashes and `//`; treat dirs as **literal** strings in matching (no glob interpolation); emit with `printf '%s\t%s\n'`. Add tests for blank, metachar, whitespace, and trailing-slash values.

3. **In the migration, decouple scope from type-derivation and add a non-empty guard** (addresses: Safety 🔴; Compatibility 🔴; Safety 🟡 freeform-dir). Keep `type` derivation on the fixed default mapping (historical contract) and use the config-aware table only for the `out_of_scope` decision. Before any mutation, assert the resolved table is fully populated (13 rows) and abort loudly otherwise.

4. **Give the resolver an explicit project-root seam** (addresses: Safety 🟡; Compatibility 🟡). Pass the migration's canonical `PROJECT_ROOT` to `config-read-doc-type-paths.sh` (arg/env) rather than letting subprocesses re-derive it. Add fixtures running from a CWD ≠ root and a symlinked checkout, asserting identical scope.

5. **Re-sequence the announcements-arm removal to Phase 5** (addresses: Correctness 🟡; Compatibility 🔵). Leave the shared fallback denylist byte-complete until its last consumer migrates; the validator stops skipping-by-name the instant it flips to the allowlist regardless.

6. **Golden-pin the fallback equivalence** (addresses: Test-coverage 🟡 ×2; Compatibility 🟡). Snapshot the current helper's full-corpus output before the refactor, check it in as a fixture, and assert both the fallback path and the injected longest-match path reproduce it for every file — including the order-sensitive pairs.

7. **Complete the registry-coherence guards** (addresses: Architecture 🟡; Code-quality 🟡; Correctness 🔵; Test-coverage 🔵). Assert `DOC_TYPE_PATH_KEYS ⊆ PATH_KEYS`, equal-length/index-coupled arrays, both-direction TSV drift, and the exclusion of `global`/`templates`/`tmp`/`integrations`. Define and test the equal-length-dir tie-break.

8. **Make the injection contract explicit** (addresses: Code-quality 🟡; Correctness 🔵; Performance 🔵). Use a sentinel (or function args) to signal "table injected"; update the `doc-type-inference.sh` header to drop the now-false "pure functions" language and document the required precondition and resolve-once-immutable lifecycle. Mark Phase 5 mandatory (not deferrable like Phase 6).

9. **Minor hardening** (addresses remaining 🔵). Longest-match via integer `${#dir}` (no `sort`/`${//}`); Phase 6 awk parsing BWK-awk-safe + tested on macOS awk; keep the awk alignment test's literal expectations; add the index-scoping `DANGLING-REF` negative test; correct the Performance wording (13 forks, not VCS detections) and assert a constant resolver-spawn count.

## Per-Lens Results

### Architecture

**Summary**: Sound move (fail-open denylist → fail-closed allowlist) with the new type→path-key mapping placed in the registry that already owns path keys, and genuinely independently-mergeable phasing that preserves the single-source invariant. Main risks: unaddressed path-form mismatch (relative dirs vs absolute walked paths), a new test-only-guarded cross-file coupling, and the awk left as a third drifting encoding.

**Strengths**: Mapping located in `config-defaults.sh`; helper purity preserved via injection (functional-core/imperative-shell); transitional fallback decouples consumer migrations; fail-open→fail-closed is the correct default; single-source guard preserved.

**Findings**:
- 🟡 (high) Path-form mismatch: resolved dirs project-relative but consumers walk absolute — `D/*` arm unreachable in production; make the normalisation contract explicit and add absolute-root tests. (Phase 2 §1)
- 🟡 (high) New `config-defaults.sh ↔ templates-schema.tsv` coupling guarded only by a one-directional drift test; consider deriving `DOC_TYPE_NAMES` from the TSV and asserting both directions. (Phase 1 §1)
- 🔵 (medium) Awk `path_to_typed` remains a permanent drift surface; if Phase 6 deferred, correct the over-claimed alignment invariant. (Phase 6 / Current State)
- 🔵 (medium) Walk-all-then-filter: confirm Desired End State documents that typed files outside configured dirs are silently unvalidated. (What We're NOT Doing)
- 🔵 (low) Resolver re-runs VCS detection per key; consider reading from a single resolved config snapshot. (Performance Considerations)

### Correctness

**Summary**: The inversion is logically sound and the match requirements are correctly identified, but several hazards are under-specified: project-relative dirs vs absolute/arbitrary-prefix paths, tie-breaking/empty/duplicate/nested/whitespace config values, and the fail-open-vs-fail-closed boundary between Phase 3 and Phase 4 while the shared arrays carry global state.

**Strengths**: Most-specific-match correctly identified; segment-boundary hazard named; purity preserved; default-corpus behaviour invariant until both inject; Phase 5 fail-closed guard.

**Findings**:
- 🔴 (high) Project-relative configured dirs vs arbitrary-prefix find paths break segment matching. (Phase 2 §1; Phase 3/4)
- 🔴 (high) Empty configured path value (`config-read-value.sh:126-130` returns `""`) produces a degenerate dir. (Phase 1 §2; Phase 2)
- 🟡 (high) Phase 3 removes the announcements denylist arm while the migration is still in fallback mode → migration starts processing announcements mid-sequence. (Phase 3 vs 4)
- 🟡 (medium) Two doc-types configured to the same directory has undefined (iteration-order) resolution. (Phase 2 §1)
- 🟡 (medium) Glob metacharacters/whitespace in config values corrupt `case`-glob matching. (Phase 2 §1)
- 🔵 (medium) Nested dirs need trailing-slash normalisation before longest-match. (Phase 2 §1)
- 🔵 (high) Drift guard doesn't assert `DOC_TYPE_PATH_KEYS ⊆ PATH_KEYS` — a typo'd key resolves empty silently. (Phase 1 §2)
- 🔵 (medium) Make array population once-before-`build_index` and immutable explicit, so index and validate passes share scope. (Phase 3 §1)

### Code Quality

**Summary**: Well-structured, phased, with a thoughtful transitional fallback. Strongest move: collapsing the denylist into a config-derived allowlist (DRY). Principal risks: a fourth hand-maintained copy of the type→dir facts with incomplete drift guarding; implicit global-array injection converting a documented pure function into a stateful one; and a dual-mode helper that becomes a long-lived smell if Phase 5 slips.

**Strengths**: Incremental/reversible phasing; DRY allowlist; purity intent + test seams preserved; drift + fail-closed guards; Phase 6 correctly isolated.

**Findings**:
- 🟡 (high) Fourth hand-maintained encoding; parallel-array index coupling and `DOC_TYPE_PATH_KEYS ⊆ PATH_KEYS` not pinned. (Phase 1 §1; Phase 6)
- 🟡 (high) Implicit global-array injection; "pure function" header now false; no injected-vs-not signal. (Phase 2 §1; Phase 3/4 §1)
- 🔵 (medium) Dual-mode classifier persists until Phase 5, which has no mandatory trigger. (Phases 2–5)
- 🔵 (medium) Resolver TSV contract + exclusion set diverge subtly from `config-read-all-paths.sh`. (Phase 1 §2)
- 🔵 (medium) Resolve-once-at-startup snapshot is a behavioural-shape change not called out as a contract. (Performance / Phase 3 §1)

### Test Coverage

**Summary**: Unusually test-conscious for a shell change, with concrete per-phase scripts and explicit cross-phase regressions. But three key guarantees rest on weaker evidence than the prose implies: the migration equivalence is manual-only, the fallback equivalence isn't pinned against pre-refactor output, and the config-override fixture's cwd interaction is untested. Several config-driven match edge cases are named but not pinned to assertions.

**Strengths**: Per-phase automated-verification blocks; edge cases named; Phase 5 fail-closed assertion; drift + single-source guards.

**Findings**:
- 🟡 (high) Migration "processed/skipped unchanged" has no automated guard. (Phase 4 manual + Testing Strategy)
- 🟡 (high) "Fallback byte-for-byte unchanged" asserted by a fresh test, not golden-pinned to pre-refactor output. (Phase 2)
- 🟡 (medium) Config-override fixture/cwd interaction untested — assert both halves. (Phase 1 + 3)
- 🔵 (medium) Empty/odd config values and overlapping dirs not in the named edge-case set. (Phase 2)
- 🔵 (medium) Index-scoping change lacks a `DANGLING-REF` negative assertion. (Phase 3)
- 🔵 (medium) Awk alignment risks becoming tautological if both sides read one table. (Phase 6)
- 🔵 (low) Resolver test doesn't pin exclusion of global/templates/tmp/integrations as a negative. (Phase 1)

### Portability

**Summary**: Well aware of the bash 3.2 floor and LANG=C discipline. Risks are cross-platform shell-construct fragility rather than vendor lock: the longest-match logic, the awk `-v` parsing, and the absolute-vs-relative path mismatch are all places a macOS/3.2-only divergence could hide, and are under-specified.

**Strengths**: Parallel arrays over associative; `lint:scripts:check` every phase; LC_ALL=C context; config resolution kept out of the pure helper.

**Findings**:
- 🟡 (medium) Absolute find paths vs project-relative injected dirs may break segment matching cross-environment. (Phase 2 §1; Phase 3 §1)
- 🟡 (medium) Longest-match risks a non-portable `${#}`/`sort`/`${//}` construct — specify integer length comparison. (Phase 2 §1)
- 🟡 (medium) Phase 6 awk `-v` parsing must avoid gawk-only features (macOS BWK awk). (Phase 6 §1)
- 🔵 (medium) TSV tab emission/parsing varies across `echo`/`printf`/locale — mandate `printf '%s\t%s\n'`. (Phase 1 §2)
- 🔵 (low) Pin the match pattern literally as `*/"$dir"/*` (quoted, slash-anchored). (Phase 2/3)

### Safety

**Summary**: The allowlist is fail-closed by design — the safer direction for an in-place mutator — and the migration's existing dirty-tree refusal + idempotency + zero-mutation pre-pass provide VCS-based recovery. The dominant residual hazard: a new hard dependency on config resolution with no guard that the resolved table is non-empty before mutation. A secondary hazard: a misconfigured `paths.*` could pull a freeform dir into the mutation set.

**Strengths**: Fail-closed model; strong existing VCS recovery nets; empty table can only skip (not wrongly mutate freeform); bounded blast radius per phase; Phase 4 dry-run retained.

**Findings**:
- 🔴 (high) No non-empty guard on the resolved table before the migration mutates frontmatter. (Phase 4 §1)
- 🟡 (medium) Resolver re-derives its own project root → divergence from the migration's canonical `PROJECT_ROOT`. (Phase 1 / Phase 4)
- 🟡 (medium) Misconfigured `paths.*` can pull a freeform directory into the in-place mutation set. (Phase 4 / Phase 6)
- 🔵 (medium) Self-validation shares the same allowlist that scoped the mutation, weakening it as an independent check. (Phase 3/4)

### Compatibility

**Summary**: Changes the contract of a helper single-sourced by two consumers and pinned by a single-source test. Phasing is well-conceived for independent mergeability, but two consumer-facing risks stand out: the migration's inferred type drives schema-shaped output (so a config override changes a shipped migration's historical output), and config resolution is CWD-bound while the migration uses an arbitrary `PROJECT_ROOT`. The "byte-for-byte" fallback claim is under-specified around most-specific-match vs case-arm order.

**Strengths**: Phase sequencing preserves cross-consumer compatibility; helper purity/sourcing contract preserved; Phase 5 fail-closed guard for future consumers; drift guard protects the registry.

**Findings**:
- 🔴 (high) Config-aware inference changes the 0007 migration's historical emitted frontmatter (type derivation, not just scope). (Phase 4)
- 🟡 (medium) Config resolution is CWD-bound; migration operates on an arbitrary `PROJECT_ROOT` — resolver has no root seam. (Phase 4 §1)
- 🟡 (medium) Fallback (case-arm) vs injected (longest-match) equivalence on the default tree is non-trivial — add a full-corpus equivalence test. (Phase 2 §1)
- 🔵 (high) Announcements-arm removal in Phase 3 mutates the shared fallback for the still-fallback migration — defer to Phase 5. (Phase 3 §2)
- 🔵 (medium) Registry is a fourth encoding; awk deferred to Phase 6 leaves a non-default-config drift gap — land Phase 6 or bound/guard the gap. (Phase 1 §1 / Phase 6)

### Performance

**Summary**: Sound at the actual data scale (a few hundred files) — the per-file most-specific-match loop and one-time startup resolution are negligible. The plan's verdict is right but slightly misstates the cost model: the dominant cost is 13 process forks each re-sourcing the config library, not the cheap ancestor-walk "VCS detection".

**Strengths**: Helper kept pure, config resolved once via injection (avoids O(files×13) subprocesses); declines to narrow the walk (no premature optimisation); correct negligible-cost verdict; proportionate to corpus size.

**Findings**:
- 🔵 (high) Startup cost is 13 `config-read-value.sh` forks re-sourcing the config library, not "VCS detections" — reword; confirm the resolver loops keys in-process. (Performance Considerations)
- 🔵 (medium) Confirm resolution is invoked once at top-level, not per find-walk pass / per migration phase. (Phase 3 §1)
- 🔵 (high) Per-file O(13) match must use bash pattern matching, no `grep`/`sed`/`awk` per array entry (runs twice per file). (Phase 2 §1)

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-19

**Verdict:** REVISE

The revision is a major improvement: **all four pass-1 criticals and every
structural major are confirmed resolved** across the eight lenses, with concrete,
well-specified mechanisms (explicit prefix-agnostic match contract, `${#}`
longest-match, resolver value-hardening, bidirectional coherence guard,
config-aware migration derivation + non-empty guard, deferred announcements-arm
removal, explicit injection contract + sentinel, Phase 5 mandatory). The deeper
second pass surfaced a **new, tightly-clustered set of findings around
config-resolution robustness** — no criticals, but enough majors to keep the
verdict at REVISE for one more focused pass.

### Previously Identified Issues
- 🔴 **Correctness**: path-form mismatch (relative dirs vs absolute paths) — **Resolved** (explicit `*/"$D"/* | "$D"/*` contract, absolute-path test).
- 🔴 **Correctness**: empty config value degenerates the matcher — **Resolved** (empty→default + normalisation in resolver).
- 🔴 **Safety**: no non-empty guard before the migration mutates — **Resolved** (Phase 4 §2 guard, co-located with `fm_assert_schema_columns` at `:720`, before the prepass).
- 🔴 **Compatibility**: config-aware inference breaks migration output — **Resolved** (derivation kept config-aware *by design* with sound reproducibility reasoning; default-layout byte-equivalence test + applied-once/post-typed idempotency).
- 🟡 Path-form/architecture, resolver hardening, registry coherence, fallback golden-pin, both-halves override fixture, index-scoping `DANGLING-REF`, phase-resequencing (announcements→Phase 5), injection contract + sentinel, Phase 5 mandatory, performance cost wording, awk-alignment literals — **All Resolved**.

### New Issues Introduced / Surfaced
- 🟡 **Architecture + Safety** (one root cause): the "explicit project-root argument" the plan now requires has **no mechanism** — `config-read-doc-type-paths.sh` delegates to `config-read-value.sh`, which resolves config strictly from CWD (`config_find_files` → `config_project_root` → `find_repo_root`/`$PWD`), with no root parameter. Worse, the migration's `self_validate_referential` (`:742`) spawns `bash "$VALIDATOR" "$META_ABS"` as a *separate process* that re-resolves its own table from *its* CWD — so the final post-mutation integrity gate can resolve a different scope than the one that drove the mutation when CWD ≠ `PROJECT_ROOT`.
- 🟡 **Safety**: resolved dirs bypass the migration's existing `assert_safe_relpath` guard (`:48-57`). Value-hardening rejects empty/tab/newline but not `..`, leading `/`, or `.` — a traversal/absolute `paths.*` override (e.g. `paths.work: ..`) survives normalisation and widens the in-place mutation set. (Correctness flagged the same absolute/`..` gap as minor.)
- 🟡 **Safety**: self-validation shares the same injected table that scoped the mutation, so it cannot catch a scope-resolution error (the checker validates the same misclassification). Pass-1 finding (4), still un-addressed — needs an explicit "shared scope is accepted" note plus an independent guard (e.g. path-safety + non-zero typed-file count).
- 🟡 **Portability**: Phase 6 awk `-v` table parsing still has no BWK-awk (macOS one-true-awk) safety constraint (no `gensub`/`length(arr)`, single-literal-char `split` separator). Deferrable with Phase 6, but unspecified.
- 🔵 **Minors**: Phase 4 byte-equivalence baseline should be captured *pre-refactor* (mirror the Phase 2 golden wording); `DOC_TYPE_NAMES` name collides between the static registry and the injected array (use distinct names or state the overwrite); factor a shared `load_doc_type_table()` parse helper for both consumers; empty→default should emit a stderr note (cannot disable a doc-type via blank config); add a Phase 4 `precondition_prepass` REFUSE-verdict parity assertion; mirror the constant-resolver-spawn-count check in Phase 4; Phase 6 split-state caveat for override-repo linkage typing.

### Assessment
The plan is close. The new findings are a coherent cluster — **(1) make `PROJECT_ROOT` the consistent resolution CWD** (resolver `cd`s into the passed root; the migration spawns its self-validator from the root), and **(2) route resolved dirs through an `assert_safe_relpath`-equivalent** — plus a documentation note on self-validation's shared scope and a BWK-awk constraint on the deferrable Phase 6. None are criticals; all are addressable in one more focused revision, after which APPROVE is expected.

---
*Re-review generated by /accelerator:review-plan*
