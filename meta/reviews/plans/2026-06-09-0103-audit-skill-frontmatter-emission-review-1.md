---
type: plan-review
id: "2026-06-09-0103-audit-skill-frontmatter-emission-review-1"
title: "Plan Review: Audit Skill Frontmatter Emission Against the Unified Schema"
date: "2026-06-09T19:53:34+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-09-0103-audit-skill-frontmatter-emission"
target: "plan:2026-06-09-0103-audit-skill-frontmatter-emission"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, portability, standards, documentation]
review_number: 1
review_pass: 2
tags: [frontmatter, schema, skills, validation, audit, test-harness]
last_updated: "2026-06-09T21:50:32+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Audit Skill Frontmatter Emission Against the Unified Schema

**Verdict:** REVISE

The plan is exceptionally well-grounded — it sources the existing three-file
contract rather than re-encoding it, picks the correct structural precedent
(`test-validate-corpus-frontmatter.sh`), sequences its three phases with a
justified merge order, and provisions count-gated liveness plus a negative
self-test so the guard cannot pass inert. However, all seven lenses converged on
a tight cluster of mechanism-level defects in the two least-specified parts of
the design — the producer-enumeration step and the `extract_literal()` /
self-test machinery — several of which were verified false against source. The
plan should be revised to close these before implementation; the issues are
fixable in place and do not require rethinking the approach.

### Cross-Cutting Themes

- **Discovery command does not reproduce the recorded producer set**
  (flagged by: correctness, standards, documentation, architecture, code-quality,
  portability) — The documented grep returns **17** files (verified live),
  including `skills/config/migrate/SKILL.md` which the plan *excludes*, and it
  does **not** surface `skills/decisions/review-adr/SKILL.md` which the plan
  *adds* as a status-axis producer. The Phase 1 AC "its output matches the
  recorded set" is therefore unsatisfiable as written, and the Phase 3 count-gate
  (`PRODUCERS count == discovered count == N`) has no consistent `N`.

- **The two status-axis-only producers cannot be read by the documented
  extractor** (flagged by: correctness, test-coverage, code-quality, architecture,
  documentation) — `extract_literal()` targets the substitute-list form
  (`` - `status:` ← `X` ``), but the load-bearing literals live elsewhere:
  `validate-plan`'s plan-status `done` is *free prose* at `:187`, and
  `review-adr`'s transitions are in a markdown *table* (`:85`) and prose (`:194`).
  The very axis that caught the original bug has no mechanically-extractable
  literal under the sketched extractor.

- **The negative self-test is a structural no-op** (flagged by: correctness,
  code-quality, test-coverage) — The illustrative
  `sed 's/status:` ← `done/.../'` targets a string that does not exist in
  `validate-plan/SKILL.md`; the "corrupted" copy is identical to the original, so
  the validator still accepts it and the wiring proof (AC5) passes for the wrong
  reason.

- **`extract_literal()` is the load-bearing unit and is left as `...`**
  (flagged by: code-quality, test-coverage, architecture, documentation,
  standards, portability) — A regex parser over free-form, human-authored prose
  whose formatting is not contract-governed. A reworded bullet silently yields an
  empty literal → placeholder fixture → green guard that no longer tests the real
  emission, which is exactly the drift the work item exists to eliminate.

### Tradeoff Analysis

- **Single oracle vs covering the validator's blind spots**: The plan rightly
  refuses to widen the validator, but then places two bespoke checks (non-anchored
  provenance over-emission; bare/unquoted linkage) inside the guard — re-creating
  a second, un-sourced definition of "conforming frontmatter". Architecture wants
  these expressed via the shared helper data (`FM_PROVENANCE_FIELDS`,
  `FM_TYPED_REF_RE`, TSV anchoring) with a tracking link to the eventual
  validator fix; test-coverage wants each blind-spot check to carry its own
  negative self-test. Recommendation: do both — derive the checks from shared
  helper symbols and give each a liveness case — so the duplication is
  attributed and convergent rather than a hidden fork.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness / Standards / Documentation / Architecture**: Discovery grep yields 17 files, not 16; the "matches the recorded set" AC is unsatisfiable
  **Location**: Phase 1 §1 (Discovery procedure) + Phase 1 Success Criteria (first automated checkbox)
  Verified live: the documented grep returns 17 SKILL.md files including `config/migrate` (explicitly excluded) and omits `review-adr` (added separately as a status-axis producer). The recorded set is neither a subset nor superset of the command output, so an implementer following the procedure verbatim is stuck. Fix by documenting an explicit `comm -23 <(grep…|sort -u) <(allowlist)` reconciliation — allowlist = the 16 emitters + the recorded exclusions (`config/migrate`) — plus a separate status-axis marker for `review-adr`, mirroring `test-skill-frontmatter-population.sh:266-273`. State the expected grep cardinality as 17.

- 🟡 **Correctness / Test-Coverage / Code-Quality / Architecture**: Status-axis-only producers expose status in prose/tables, not the substitute-list form the extractor targets
  **Location**: Phase 3 §1 (`extract_literal`)
  `validate-plan`'s plan-status `done` is prose at `:187` (no `← ` marker); `review-adr`'s transitions live in a table (`:85`) and prose (`:194`). A single substitute-list regex cannot extract either, so the status axis — the heart of this work item — is silently skipped. Define a second extraction mode for status-transition mutators with a stated per-skill anchor for `validate-plan:187` and `review-adr:194`.

- 🟡 **Correctness / Code-Quality / Test-Coverage**: The negative self-test corrupts a string that does not exist, passing as a false green
  **Location**: Phase 3 §2 (negative / liveness self-test)
  The illustrative `sed 's/status:` ← `done/…/'` is a no-op against `validate-plan/SKILL.md` (whose only substitute-list status literal is `complete` at `:161`). The "corrupt" fixture is therefore accepted, so AC5's wiring proof verifies nothing. Target a literal that exists in substitute-list form, corrupt the value *after* extraction, and assert the file actually changed before asserting rejection.

- 🟡 **Code-Quality / Architecture**: The load-bearing `extract_literal()` is a brittle prose parser left as `...`
  **Location**: Phase 3 §1 (the guard script)
  The guard's whole green-path and negative test rest on scraping verbatim YAML literals out of free-form SKILL.md prose (mixed bullet + fenced forms, the non-ASCII `←` glyph). A reworded instruction silently returns an empty literal and the guard goes green on a placeholder fixture. Specify the exact prose shapes supported and add a per-(skill, field) assertion that extraction returned a non-empty value, failing loudly on a formatting change.

- 🟡 **Test-Coverage / Documentation**: "Completeness is mechanically checkable" overstates a manual spot-check
  **Location**: Phase 1 Success Criteria (Manual Verification) + Desired End State
  AC2 demands the per-type attribute-set completeness be mechanical, but the only check provided is a Manual Verification line ("spot-checked against `templates-schema.tsv`"). No script computes base ∪ extras ∪ provenance-if-anchored ∪ linkage ∪ status per type and diffs it against the table. Either add that derivation to the guard (it already loads both contract files) or restate the AC honestly.

- 🟡 **Architecture / Test-Coverage / Correctness**: Blind-spot axes form a parallel, un-sourced contract that is itself untested
  **Location**: Phase 3 §1 (blind-spot assertions) + What We're NOT Doing
  The two checks the validator can't perform are defined only inside the guard (a partial re-encoding the plan otherwise forbids), carry no liveness self-test of their own, and the provenance-over-emission check inspects SKILL.md literals even though provenance is *template-supplied* — so it may pass vacuously. Express them via shared helper symbols, give each a negative self-test, and resolve composed emission before the provenance check (or scope it explicitly to skill-own literals).

- 🟡 **Test-Coverage**: Single-mutation negative test gives green-path-only coverage for every other producer and axis
  **Location**: Phase 3 §2
  One mutation on one skill never exercises the synthesis paths for a bad `type`, missing required extra, non-integer `schema_version`, or a status on a different producer. Parameterise the negative test over at least one mutation per axis the guard claims to cover, each asserting the specific expected diagnostic code.

- 🟡 **Test-Coverage**: AC4 conditional-axis coverage relegated to a manual checkbox
  **Location**: Phase 3 Manual Verification (AC4)
  Anchored-vs-non-anchored provenance, with/without typed-linkage, and omit-when-empty are distinct validator code paths (`EMPTY-PLACEHOLDER` at `:340-351`) that one fixture per (skill, type) cannot cover. Specify that the guard synthesizes both branch variants per applicable axis and asserts the validator's verdict on each.

- 🟡 **Documentation**: The "single source of truth" claim does not cover the table that actually drifts
  **Location**: Phase 1 §2 (conformance table — "share one source of truth")
  Only the discovery *patterns* are shared with the guard; the per-attribute table is static prose never read by the guard, yet it is the artifact most likely to go stale. Narrow the claim to "the producer enumeration is shared" and label the attribute table a point-in-time record.

- 🟡 **Portability**: Literal extraction depends on a non-ASCII U+2190 arrow under `LC_ALL=C`
  **Location**: Phase 3 §1 (`extract_literal`) + §2 (self-test sed)
  Every SKILL.md encodes emitted values with `←` (U+2190, 3 UTF-8 bytes). Under the mandated `LC_ALL=C`, a byte-literal match works on both BSD and GNU, but any pattern mixing the arrow with character classes/anchors matches UTF-8 fragments and diverges across the macOS/Linux tool matrix. Anchor extraction on the ASCII backtick-delimited tokens and treat the arrow as an opaque byte run; add an extraction-sanity assertion for one known skill.

#### Minor

- 🔵 **Correctness**: `validate-plan` emits two types (`plan-validation`/`complete`, `plan`/`done`); the sketched `extract_literal($skill, $field)` has no parameter to disambiguate which status binds to which type. Key extraction by (skill, type), not (skill, field).
  **Location**: Phase 3 §1
- 🔵 **Correctness / Test-Coverage / Portability**: The Phase 2 line-coupled greps (`grep -nE "status.*(done|complete)" … :161/:186-188` and the negated `! grep … | grep -i plan`) assert on text position and can both false-fail (the legitimate plan-validation `complete` matches `plan.*status.*complete`) and behave inconsistently across BSD/GNU grep under `pipefail`. Drive the assertion through the validator (synthesize + assert accept) instead.
  **Location**: Phase 2 §1 (Automated Verification)
- 🔵 **Architecture**: `review-adr` is in the guard's scope but absent from the established `IN_SCOPE_PRODUCERS` taxonomy; it sits in a third category (status-axis mutator) that neither the population test nor the plan's "16 full-block emitters" cleanly accommodates. Make the taxonomy (emitters / status-axis mutators / non-emitting consumers) explicit and shared.
  **Location**: Phase 3 §1 (PRODUCERS allowlist + cross-check)
- 🔵 **Code-Quality**: Fixture synthesis duplicates `emit_valid()` (`test-validate-corpus-frontmatter.sh:31-65`); a future schema tightening must be applied in two places. Factor into a shared sourced helper or extend `emit_valid()` to accept literal overrides.
  **Location**: Phase 3 §1 (fixture synthesis)
- 🔵 **Code-Quality**: The guard interleaves three assertion families (validator-accepts; two validator-bypassing blind-spot checks; status-axis membership) in one per-producer loop. Separate into labelled helpers with a comment noting which bypass the validator and why.
  **Location**: Phase 3 §1
- 🔵 **Test-Coverage**: If `rejected` is triaged schema-source (deferred to a 0057 child), the guard must still state how it treats `review-adr`'s documented-but-deferred status axis — assert only the vocab-valid transitions and record `rejected` as covered-by-the-child-item, so the guard neither goes red nor silently drops the producer.
  **Location**: Phase 3 §1 (status-axis producers)
- 🔵 **Standards / Documentation**: The appended section is coined "Audit Record", but the established convention (per the cited population-test precedent) is "Discovery Pass Record"; heading level and column schema are unspecified. Reuse/relate the existing name and pin the columns (skill, type, attribute, source [literal|template|helper], rule/fix).
  **Location**: Phase 1 §2 (Location)
- 🔵 **Documentation**: A ~200-row per-(skill, type) attribute table in a closed work-item body has a different audience and lifecycle than a scoped task record. Consider housing the durable reference alongside the contract (or as a note artifact) and state who maintains it after Phase 1 closes.
  **Location**: Phase 1 (audience) + Migration Notes
- 🔵 **Architecture**: The suite-count floor (`_EXPECTED_CONFIG_SUITES` 15→16) guarantees "at least N suites ran" but not "this specific gate ran" — a guard renamed off `test-*.sh` would vanish while the count still passes. Consider asserting the specific suite name is present in the discovered list.
  **Location**: Phase 3 §3
- 🔵 **Portability**: The Phase 2 oracle hardcodes `/tmp/plan-complete.md` / `/tmp/plan-done.md` instead of `mktemp`, diverging from the plan's own discipline and unsafe to lift verbatim into the script. Use `mktemp` + trap-EXIT.
  **Location**: Phase 2 §1 (TDD oracle snippet)
- 🔵 **Standards**: The negative self-test uses `sed -i.bak` (correct for BSD/macOS) but the sibling suite cleans up `.bak` files; note the files are trap-scoped under `$TMP` so no explicit `rm` is needed, or follow the cleanup convention.
  **Location**: Phase 3 §2

#### Suggestions

- 🔵 **Standards**: Once the producer set is reconciled, pin the expected producer count to a literal in the guard (matching the population test's exact-count idiom) so liveness fails loudly if a producer silently drops.
  **Location**: Phase 3 §2 (count-gated liveness)
- 🔵 **Documentation / Code-Quality**: Document the exact set of SKILL.md literal forms `extract_literal()` must recognise, with one real example per producing skill that deviates from the substitute-list form, so the extraction contract is reproducible rather than illustrative.
  **Location**: Phase 3 §1

### Strengths

- ✅ Sources the existing three-file contract (`templates-schema.tsv`,
  `frontmatter-emission-rules.sh`, `validate-corpus-frontmatter.sh`) and forbids
  re-encoding it, preserving the single-source seam 0070 established — with an
  automated success-criterion grep that the property holds.
- ✅ Correctly re-bases the guard on `test-validate-corpus-frontmatter.sh`
  (synthesize → run real validator → assert rc/code) rather than the
  work-item-cited population test, and documents *why* — a deliberate, justified
  departure.
- ✅ Phase 2 is genuine red→green TDD against the existing validator's real
  `BAD-STATUS` path, giving the producer fix regression protection with zero new
  infrastructure.
- ✅ Linear phase dependency (audit → fix → guard) is explicitly justified by
  merge order so the tree stays green and mergeable per-PR.
- ✅ Every cited contract line number was verified accurate (validator axes
  `:271-376`, blind spots `:314-324` / `:358`, all `frontmatter-emission-rules.sh`
  symbols), and the `validate-plan:187` `complete → done` fix is correct against
  ADR-0042.
- ✅ The composed-emission insight (skill literals + template + metadata helper)
  is documented, preventing false-positives on template-supplied base fields.
- ✅ Strong portability discipline: bash 3.2-safety, `LC_ALL=C`,
  `set -euo pipefail`, the `tail -n +2 | IFS=$'\t' read` TSV pattern (no
  `declare -A`), `mktemp -d` + trap-EXIT — all matching established idioms; no
  vendor/cloud coupling introduced.
- ✅ Scopes the audit boundary deliberately and records exclusions with rationale
  (`update-work-item`, `list-work-items`, `config/migrate`).

### Recommended Changes

1. **Reconcile the discovery procedure with the recorded set** (addresses:
   "Discovery grep yields 17 files"; "review-adr absent from taxonomy"; "two
   divergent discovery patterns"; "single source of truth"). Document the
   enumeration as `comm -23 <(grep…|sort -u) <(allowlist)` where the allowlist =
   16 emitters + recorded exclusions (`config/migrate`), plus a separate
   status-axis marker that actually surfaces `review-adr`. State the grep
   cardinality as 17. Make the producer taxonomy (emitters / status-axis mutators
   / consumers) explicit and shared with the population test, and pin the
   liveness count.

2. **Specify a second extraction mode for status-transition mutators**
   (addresses: "status-axis producers can't be extracted"; "validate-plan two
   types"). Key extraction by (skill, type); state the per-skill anchor for the
   prose/table literals at `validate-plan:187` and `review-adr:194`; add a
   per-(skill, field) non-empty-extraction assertion that fails loudly on a
   formatting change.

3. **Rebuild the negative self-test so it genuinely trips** (addresses: "self-test
   no-op false green"; "single-mutation coverage"). Corrupt the value *after*
   extraction (not by a wording-specific `sed`), assert the file actually
   changed, and parameterise at least one mutation per axis (bad type, bad status,
   missing required extra, non-integer `schema_version`), each asserting the
   specific diagnostic code.

4. **Fully specify `extract_literal()`** (addresses: "load-bearing prose parser
   left as `...`"; "non-ASCII arrow under LC_ALL=C"; "mirror population-test
   grammar"). Enumerate the exact SKILL.md literal shapes, anchor matches on the
   ASCII backtick tokens (arrow as opaque bytes), and reuse/mirror the population
   test's two documented instruction-context grammars.

5. **Make AC2/AC4 completeness mechanical, not manual** (addresses: "completeness
   overstated"; "AC4 conditional axes manual"; "blind-spot checks untested").
   Have the guard derive the enforced attribute set per type from the contract
   files and assert the fixture exercises every one, synthesize both branch
   variants per conditional axis, and give each blind-spot check a negative
   self-test expressed via shared helper symbols.

6. **Tidy the documented snippets for portability and accuracy** (addresses:
   "Phase 2 hardcoded /tmp"; "line-coupled greps"; ".bak cleanup"). Use `mktemp`
   in the Phase 2 oracle, drive the Phase 2 verification through the validator
   rather than line-coupled greps, and note the trap-scoped `.bak` handling.

7. **Clarify the Audit Record artifact** (addresses: "Audit Record naming";
   "large table in work-item body"). Reuse/relate the established "Discovery Pass
   Record" name, pin the table columns and heading placement, and decide whether
   the durable reference lives in the work-item body or alongside the contract.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally disciplined: it treats the existing
three-file contract as the single oracle and explicitly refuses to re-encode it,
preserving the no-drift seam that 0070 established. The phasing (audit → fix →
guard) respects a clean dependency ordering and keeps each PR green. The main
architectural risks are around the new guard's coupling to two parallel,
drift-prone enumeration mechanisms (a second discovery command distinct from the
population test's, and a hard-coded PRODUCERS allowlist) and the structural
fragility of extracting verbatim YAML literals out of LLM prose, which couples
the test to SKILL.md presentation rather than emission behaviour.

**Strengths**:
- Treats the existing validator + emission-rules helper + TSV as the
  authoritative contract and forbids re-encoding it, preserving the single-source
  seam 0070 built.
- Correctly identifies the producer-side blind spot: the validator gates the
  corpus but never the producers, closed here with an automated gate on the same
  CI seam.
- Linear phase dependency justified explicitly (a guard rejecting the unfixed bug
  would be red, so the fix merges first).
- Respects bash 3.2 / LC_ALL=C constraints and reuses
  `test-validate-corpus-frontmatter.sh`'s structural model.
- Scopes the audit boundary deliberately and records exclusions with rationale.

**Findings**:
- 🟡 **major** (high) — *Two independent producer-enumeration mechanisms create a
  drift seam the plan was meant to close* — Phase 1 §1 + Phase 3 §1. The new
  discovery command differs from the population test's `DISCOVERY_PATTERNS`; two
  grep-based enumerators with their own pattern sets and allowlists will
  independently classify the producer set, re-creating the latent divergence the
  work item exists to remove. Reuse the population test's patterns/allowlist or
  factor enumeration into one shared sourced fragment.
- 🟡 **major** (medium) — *Guard couples to SKILL.md prose presentation, not
  emission behaviour* — Phase 3 §1 (`extract_literal`). A reworded instruction
  silently drops a literal from extraction; the guard then validates a fixture
  with a missing/placeholder field and passes green. Make the extraction contract
  explicit and fail if a discovered producer yields no extractable literals.
- 🟡 **major** (high) — *Blind-spot axes give the guard a parallel contract the
  oracle does not own* — Phase 3 §1. The two checks the validator can't perform
  embed contract knowledge nowhere in the shared files; when the blind spots are
  later closed in the validator there will be duplicated/divergent enforcement.
  Express via `FM_PROVENANCE_FIELDS` / `FM_TYPED_REF_RE` / TSV anchoring with a
  tracking link to the 0057 child item.
- 🔵 **minor** (medium) — *review-adr is in scope but absent from the established
  producer allowlist* — Phase 3 §1. It sits in a third category neither the
  population test's taxonomy nor the "16 full-block emitters" set accommodates;
  the `comm -23` equality may need an ad-hoc exception. Make the taxonomy
  explicit and shared.
- 🔵 **minor** (high) — *Suite-count floor is an indirect coupling future work
  will fight* — Phase 3 §3. The floor catches dropped exec bits but never an
  accidentally non-discovered guard. Consider asserting the specific suite name
  is present.
- 🔵 **minor** (medium) — *Audit table placement splits one logical artifact
  across the work-item body and the guard* — Phase 1 §2. Prose table and
  executable check are separate representations kept manually in agreement; treat
  the guard as the sole enumeration source and label the body snapshot
  point-in-time.

### Code Quality

**Summary**: Unusually rigorous for a test-harness addition — models the guard on
a verified precedent, reuses the shared contract via sourcing, and mandates
count-gated liveness plus a negative self-test. The main code-quality risks are
concentrated in two undersized areas: the `extract_literal()` prose-parsing helper
(sketched only as `...`) and the producer-set reconciliation, which must combine a
discovery grep, an exclusion (`config/migrate`), and a manual addition
(`review-adr`) that the single-PRODUCERS-array sketch does not account for.

**Strengths**:
- Reuses the shared contract by sourcing the helper and reading the TSV (DRY),
  with an explicit success criterion grepping for those sources.
- Correctly identifies the structural precedent and justifies the departure from
  the work-item citation.
- Builds in anti-rot mechanisms (count-gated liveness, negative self-test on a
  corrupted real SKILL.md copy).
- Phase ordering reasoned for maintainability.

**Findings**:
- 🟡 **major** (high) — *`extract_literal()` is a brittle, under-specified prose
  parser* — Phase 3 §1. Parses human-authored docs whose formatting is not
  contract-governed; a reworded bullet → empty string → placeholder fixture →
  false green. Specify the supported shapes and add a non-empty-extraction
  assertion.
- 🟡 **major** (high) — *Producer set reconciliation is three-way, not a flat
  allowlist* — Phase 1 §1 + Phase 3 §1. Discovery surfaces 17 (incl.
  `config/migrate`) and omits `review-adr`; a single array + naive `comm -23`
  either fails the count-gate or needs ad-hoc fudging. Mirror the population
  test's named arrays (emitters / exclusions / status-axis additions).
- 🔵 **minor** (medium) — *Fixture synthesis duplicates `emit_valid()`* — Phase 3
  §1. Two near-identical synthesizers will drift; factor into a shared helper or
  extend `emit_valid()` with literal overrides.
- 🔵 **minor** (medium) — *Three assertion families interleaved in one loop* —
  Phase 3 §1. A reader must hold three mental models; the validator-bypassing
  blind-spot checks look validator-backed. Separate into labelled helpers.
- 🔵 **minor** (high) — *Negative self-test sed couples to one skill's wording* —
  Phase 3 §2. A reword makes the sed a no-op and the self-test passes for the
  wrong reason. Corrupt after extraction and assert the file changed.
- 🔵 **suggestion** (medium) — *Suite-count floor is a hand-maintained magic
  number* — Phase 3 §3 + Current State. Pre-existing smell inherited here; worth
  a note, and consider naming discovered suites rather than only counting.

### Test Coverage

**Summary**: Unusually rigorous on test design — correct precedent, genuine
red→green Phase 2 oracle, negative self-test and count-gated liveness. The most
serious gap is that the literal-extraction strategy is under-specified and faces a
real heterogeneity problem (the load-bearing validate-plan plan-status literal is
free prose, not a parseable substitution bullet). Secondary: the single-mutation
negative test, untested blind-spot assertions, and AC2/AC4 completeness resting on
manual verification.

**Strengths**:
- Correctly re-bases on the synthesize→run-real-validator pattern, keeping the
  validator as the single behavioural oracle.
- Phase 2 is genuine red→green TDD with an explicit reject-then-accept oracle.
- Provisions a count-gated aggregate to stop a zero-iteration inert pass.
- Recognises the two validator blind spots (verified at `:314-324` / `:358`) and
  routes them to dedicated assertions.
- Sources the contract files rather than hardcoding, with an automated
  no-re-encoding check.
- Insists the guard evaluate composed emission.

**Findings**:
- 🔴/major (high) — *Guard rests on an `extract_literal()` that cannot read the
  load-bearing prose case* — Phase 3 §1. Report status at `:161` is a bullet;
  the plan-mutation `done` at `:186-188` is prose; the illustrative sed matches
  only the bullet. Specify two extraction modes + an extraction-coverage
  assertion.
- 🟡 **major** (medium) — *Single-mutation negative test* — Phase 3 §2. One
  mutation on one skill leaves other producers/axes green-path-only.
  Parameterise per axis, asserting specific diagnostics.
- 🟡 **major** (medium) — *Blind-spot assertions have no liveness self-test* —
  Phase 3 §1. Hand-rolled checks with no oracle behind them can rot into
  no-ops. Add a negative case per blind-spot check.
- 🟡 **major** (medium) — *Completeness checked only manually* — Phase 1 Manual
  Verification. AC2 demands mechanical; provide an automated derivation/diff.
- 🟡 **major** (medium) — *AC4 conditional axes relegated to manual* — Phase 3
  Manual Verification. One fixture per (skill, type) can't cover anchored/
  non-anchored, with/without linkage, omit-when-empty. Synthesize both branch
  variants.
- 🔵 **minor** (medium) — *review-adr deferred-state handling unstated* — Phase 3
  §1. State how the guard treats a documented-but-deferred divergence so it
  neither goes red nor drops the producer.
- 🔵 **minor** (high) — *Phase 2 line-coupled greps are brittle* — Phase 2
  Automated Verification. Treat the synthesize-and-validate steps as
  authoritative; demote or de-line-number the greps.

### Correctness

**Summary**: The plan's factual claims are overwhelmingly accurate — validator
axes, helper symbol locations, per-type vocabularies, both blind spots, and the
`validate-plan:187` fix (confirmed against ADR-0042) all check out. But the
discovery grep does NOT yield the claimed 16-element set (it yields 17, including
`config/migrate`), making the Phase 1 AC unsatisfiable, and the guard's mechanical
extraction cannot read the two status-axis-only producers' literals (prose/table,
not substitute-list); the illustrative negative-test sed targets a string that
does not exist.

**Strengths**:
- Every cited line number verified accurate (validator `:271-376`, `:314-324`,
  `:358`; helper `:26`, `:29-30`, `:36`, `:47`, `:69`, `:83`, `:89`).
- Both blind spots are real and correctly characterised.
- The validate-plan divergence is precisely correct and the red→green oracle
  exploits the real `BAD-STATUS` path correctly.
- CI floor handling correct (at-least floor, auto-discovery via exec-bit glob).
- The composed-emission insight is correct and important.

**Findings**:
- 🔴/major (high) — *Discovery grep yields 17, not 16; the "matches the recorded
  set" AC is unsatisfiable* — Phase 1 §1 + Success Criteria. Includes
  `config/migrate` (excluded) and omits `review-adr` (added). Specify a `comm -23`
  reconciliation with an allowlist unioning emitters + exclusions; state grep
  cardinality 17.
- 🟡 **major** (high) — *Status-axis-only producers expose status in prose/tables,
  not the substitute-list form* — Phase 3 §1/§2. `validate-plan:187` prose;
  `review-adr` table `:85`/prose `:194`. Define a second extraction mode with
  stated anchors.
- 🟡 **major** (high) — *Negative self-test sed targets a non-existent string
  (false green)* — Phase 3 §2. Target an existing substitute-list literal,
  corrupt after extraction, assert rejection with `BAD-STATUS`.
- 🔵 **minor** (high) — *validate-plan emits two types; single-value extraction is
  ambiguous* — Phase 3 §1. Key extraction by (skill, type).
- 🔵 **minor** (medium) — *Phase 2 "no stray plan→complete" grep is logically
  fragile* — Phase 2 Automated Verification. `plan.*status.*complete` + `grep -i
  plan` can match the legitimate plan-validation `complete`. Drive through the
  validator or anchor to the specific line context.
- 🔵 **minor** (medium) — *Provenance over-emission blind-spot check inspects skill
  literals, but provenance is template-supplied* — Phase 3 §1. The check could
  pass vacuously. Resolve composed emission first or scope the assertion
  explicitly.

### Portability

**Summary**: Strongly portability-aware — explicit bash 3.2-safety, `LC_ALL=C`,
`set -euo pipefail`, the TSV-parse pattern (no `declare -A`), `mktemp -d`,
trap-EXIT — modelled on a script that already establishes those macOS/Linux-safe
idioms. The dominant residual risks are the non-ASCII U+2190 arrow the guard must
parse under `LC_ALL=C`, and a few verification snippets that reach for hardcoded
`/tmp` and chained-grep exit semantics instead of the mktemp discipline the rest
of the plan mandates. No vendor/cloud coupling.

**Strengths**:
- Explicit bash 3.2-safety, `LC_ALL=C`, `set -euo pipefail`, case-based (no
  `declare -A`) lookup.
- Models the guard on `test-validate-corpus-frontmatter.sh`, inheriting portable
  idioms (`mktemp -d` + trap, `sed -i.bak`, `find -print0 | read -d ''`).
- Sources the contract rather than re-encoding, reusing one host-portable parse
  path.
- Phase 2 oracle uses `sed 's/…/' file > out` (no `-i`), sidestepping BSD/GNU
  in-place differences.
- No vendor/cloud/proprietary coupling.

**Findings**:
- 🔴/major (medium) — *Literal extraction depends on a non-ASCII U+2190 arrow
  under `LC_ALL=C`* — Phase 3 §1/§2. Mixing the arrow with character classes/
  anchors matches UTF-8 byte fragments and diverges across BSD/GNU. Anchor on
  ASCII tokens, treat the arrow as opaque bytes, add an extraction-sanity
  assertion.
- 🔵 **minor** (high) — *Phase 2 oracle hardcodes `/tmp` paths* — Phase 2 §1.
  Not guaranteed writable/present; collisions on fixed names. Use `mktemp` +
  trap.
- 🔵 **minor** (medium) — *Two divergent discovery grep patterns claimed as one
  source of truth* — Phase 1 §1 vs Phase 3 §1. Unanchored `schema_version:` also
  matches fenced illustrations. Pick one anchored pattern set referenced from a
  single location.
- 🔵 **minor** (low) — *Negated-pipeline verification assertion fragile across
  grep implementations* — Phase 2 §2. Replace with a single deterministic check
  (capture to var and assert emptiness, or `grep -c` compare).

### Standards

**Summary**: Exceptionally well-grounded in the repo's shell-test conventions —
correct precedent, mandates sourcing `test-helpers.sh` + the helper, ends with
`test_summary`, exec-bit gate, `LC_ALL=C` / bash-3.2 discipline, and the
suite-count-floor bump matching `_EXPECTED_CONFIG_SUITES`. Naming follows the
`test-*.sh` discovery convention precisely. The main gaps are the
discovery-vs-recorded-set inconsistency (which breaks the `comm -23` self-check
convention the plan claims to follow) and an under-specified convention for the
"Audit Record" table.

**Strengths**:
- Naming follows the `test-*.sh` glob convention exactly and lives beside its
  sibling guards.
- Suite-count-floor bump matches the established at-least-floor mechanism, cited
  to the exact line.
- Harness conventions spelled out and correct (`set -euo pipefail`, `LC_ALL=C`,
  source helpers, `mktemp -d` + trap, `test_summary`).
- Correctly identifies the structural precedent and justifies the work-item
  departure.
- "Source the contract, never re-encode it" matches the single-source convention
  with its own automated criterion.

**Findings**:
- 🔴/major (high) — *Documented discovery command and recorded producer set are
  inconsistent, breaking the `comm -23` convention* — Phase 1 §1 / Phase 3 §2.
  17 returned incl. `config/migrate`; `review-adr` never returned. Reconcile via
  an explicit allowlist and assert `comm -23` empty rather than raw count
  equality; verify `review-adr` is reachable by the guard's marker.
- 🔵 **minor** (medium) — *"Audit Record" section diverges from the standard
  work-item body headings* — Phase 1 §2. None of the conventional H2 headings is
  "Audit Record"; consider the codebase-research artifact it derives from, or
  cite a precedent.
- 🔵 **minor** (medium) — *`extract_literal` should mirror the population test's
  two documented instruction-context grammars* — Phase 3 §1. Reuse/mirror
  `test-skill-frontmatter-population.sh:8-17` so the two producer-side tests stay
  convention-consistent.
- 🔵 **minor** (high) — *Negative self-test `.bak` cleanup convention* — Phase 3
  §2. Keep the `.bak` suffix (BSD/macOS requirement) but follow the sibling
  suite's cleanup, or note the files are trap-scoped.
- 🔵 **suggestion** (medium) — *Pin the concrete liveness count* — Phase 3 §2.
  The plan describes the gate but does not pin `N` (ambiguous given the discovery
  inconsistency). Pin to a literal once the set is reconciled.

### Documentation

**Summary**: Exceptionally thorough and well-cross-referenced, and correctly
identifies its Phase 1 deliverable as a documentation artifact. But the
documentation carrying the audit's value has three accuracy/reproducibility gaps:
the recorded discovery command does not reproduce the recorded set, the
"single source of truth" claim does not hold for the table itself, and
"completeness is mechanically checkable" overstates a manual spot-check. The
Audit Record's structure and naming (vs the existing "Discovery Pass Record"
convention) are under-specified.

**Strengths**:
- Unusually well cross-referenced — every contract claim cites a specific
  file:line, making the documentation auditable.
- Explicitly corrects the work item's mis-cited precedent and documents why.
- Desired End State / What We're NOT Doing give a clear scoped reader contract;
  the composed-emission caveat is documented.
- Divergence triage documented with a worked example and an explicitly
  un-pre-judged example.

**Findings**:
- 🔴/major (high) — *Recorded discovery command does not reproduce the recorded
  producer set* — Phase 1 §1 + Success Criteria. Raw output {16 emitters +
  config/migrate, no review-adr} ≠ recorded set {16 emitters + review-adr, no
  config/migrate}. Document command + explicit post-filter; make the AC assert
  the filtered output.
- 🟡 **major** (high) — *Single-source-of-truth claim does not cover the table
  that can drift* — Phase 1 §2. Only discovery patterns are shared; the attribute
  table is static prose with no automated tie-back. Narrow the claim and label
  the table point-in-time.
- 🟡 **major** (high) — *"Completeness is mechanically checkable" overstates a
  manual spot-check* — Phase 1 Manual Verification + Desired End State. No script
  computes/diffs the expected attribute set. Define the mechanical check or
  restate the AC honestly.
- 🔵 **minor** (medium) — *"Audit Record" conflicts with the existing "Discovery
  Pass Record" convention and is under-specified* — Phase 1 §2. Reuse/relate the
  name; pin columns and heading placement.
- 🔵 **minor** (medium) — *Audience/maintenance fit of a ~200-row table in a
  work-item body* — Phase 1 + Migration Notes. Low discoverability, high
  maintenance. Consider housing the reference alongside the contract; state who
  maintains it.
- 🔵 **suggestion** (medium) — *Guard's literal-extraction contract left as `...`
  illustrative pseudocode* — Phase 3 §1. Document the exact literal forms with a
  real example per deviating skill.

## Re-Review (Pass 2) — 2026-06-09

**Verdict:** APPROVE

All 7 lenses re-ran against the revised plan. Every pass-1 finding that was a
**verified factual defect** (the unsatisfiable discovery AC, the unreadable
status-axis literals, the no-op negative self-test) is confirmed fixed against
source — the correctness lens independently re-verified the grep returns exactly
17 files, the EMITTERS list matches `skills-schema.tsv` rows 2-17, the `comm -23`
reconciliation is logically sound, and the named anchors (`validate-plan:187`,
`review-adr:85-89`/`:194`) are accurate. The pass-2 findings are
specification-tightening refinements, not blocking defects, and the cheap/clear
ones were applied in this same pass (see "Edits applied in pass 2" below). The
plan is sound and ready for implementation.

### Previously Identified Issues

- 🟡 **Correctness/Standards/Documentation/Architecture**: Discovery grep yields 17, not 16; unsatisfiable AC — **Resolved.** Plan now states 17, documents the `comm -23` reconciliation against EMITTERS ∪ EXCLUDED, tracks the two status-axis mutators separately. Re-verified live (17 files, incl. `config/migrate`, excl. `review-adr`).
- 🟡 **Correctness/Test-Coverage/Code-Quality/Architecture**: Status-axis producers unreadable by the extractor — **Resolved.** Second extraction mode keyed by (skill, type) with named per-skill anchors; anchors confirmed accurate against source.
- 🟡 **Correctness/Code-Quality/Test-Coverage**: Negative self-test no-op false green — **Resolved.** Mutates the synthesized fixture's value after extraction with a "guard the guard" `[ "$mutated" != "$fixture" ]` check; parameterised per axis with specific diagnostics.
- 🟡 **Code-Quality/Architecture**: `extract_literal()` brittle prose parser left as `...` — **Partially resolved.** Two grammars specified, non-empty + vocab-membership liveness added. Residual: prose-anchor extraction for the two status mutators is inherently coupled to SKILL.md wording (now an explicit, loud-failing tradeoff, not a silent one).
- 🟡 **Test-Coverage/Documentation**: "Completeness mechanically checkable" overstated — **Resolved.** Guard derives the enforced set from the contract and asserts the fixture exercises it; AC2 wording now scoped to "composed emission (extracted literals ∪ template keys) covers the enforced set", not "every attribute is a verbatim literal".
- 🟡 **Architecture/Test-Coverage/Correctness**: Blind-spot axes = parallel un-sourced contract, untested — **Resolved.** Expressed via shared helper symbols, each with a liveness case; provenance check now specifies the template-loading mechanism (`config-read-template.sh` resolution) and routes its liveness through the same composed-emission path; a hard 0057-child reference added to track consolidation back to the single oracle.
- 🟡 **Test-Coverage**: Single-mutation negative test — **Resolved.** Parameterised per axis (type/status/extra/schema_version) + two blind-spot liveness cases.
- 🟡 **Test-Coverage**: AC4 conditional axes manual — **Resolved.** Promoted to automated both-branch synthesis; omit-when-empty key set now named (`FM_OPTIONAL_EXTRAS` ∩ type extras + linkage keys, `tags`-exempt) with an `EMPTY-PLACEHOLDER` liveness fixture.
- 🟡 **Documentation**: Single-source-of-truth claim over-reach — **Resolved.** Table demoted to point-in-time snapshot; guard is the live authority; staleness documented as expected-by-design.
- 🟡 **Portability**: Non-ASCII U+2190 under `LC_ALL=C` — **Resolved.** ASCII-anchored extraction, glyph as opaque bytes; reasoning now extended to U+2192 (review-adr table) and POSIX-class capture; bare `sed -i` prohibited.
- 🔵 Pass-1 minors (line-coupled greps, `/tmp` hardcoding, review-adr taxonomy, fixture-synthesis duplication, suite-count identity, `.bak` cleanup, Audit Record naming) — **Resolved or addressed**: greps replaced by validator-driven checks; `mktemp` adopted; emit_valid factored into a non-`test-`-prefixed shared helper; suite-name presence assertion promoted to required; section renamed "Discovery Pass Record".

### New Issues Introduced

- 🔵 **Correctness/Documentation** (minor): `EXCLUDED` was defined inconsistently between Phase 1 prose (3 members) and the Phase 3 array (1 member) — **fixed in pass 2** (Phase 1 now annotates the two non-surfaced consumers as documentation-only, not array members).
- 🔵 **Documentation** (minor): the Phase 1 manual-verification checkbox still carried "spot-check" framing that the narrowed single-source claim subordinated — **fixed in pass 2** (reworded to verify the snapshot renders the guard's derivation).
- 🔵 **Architecture/Code-Quality** (residual, accepted): the guard concentrates many responsibilities in one bash script and couples to two SKILL.md files' prose for the status mutators. Mitigated by the assert-family helper structure and loud-failing liveness; the prose coupling is inherent to skills being LLM prose. Left as an implementation-time concern, not a plan defect.
- 🔵 **Test-Coverage** (residual, accepted): the wiring-proof mutation loop covers 4 of the validator's ~14 diagnostic codes; the remainder are delegated to `test-validate-corpus-frontmatter.sh`'s own suite. The plan now scopes this division explicitly.

### Edits applied in pass 2

Reconciled the `EXCLUDED` definition; scoped the AC2 completeness guarantee to composed emission; specified the provenance check's template-loading mechanism + same-path liveness; extended the glyph-portability reasoning to U+2192 + POSIX-class capture + bare-`sed -i` prohibition; added a vocab-membership safeguard to status-mutator extraction; represented the deferred review-adr `rejected` axis as an explicit `skip_test` breadcrumb; committed `emit_valid` reuse to a new non-`test-`-prefixed shared helper; promoted the suite-name presence assertion to required; named the omit-when-empty key set + `EMPTY-PLACEHOLDER` liveness; added a hard 0057-child reference for blind-spot consolidation; reworded the Phase 1 manual-verification checkbox.

### Assessment

The plan is in good shape. The pass-1 defects were verified factual errors that
would have misdirected implementation; they are all cleared and re-verified. The
remaining items are precision refinements and two inherent tradeoffs (prose
extraction brittleness; multi-responsibility guard script) that are now explicit
and loud-failing rather than silent. No critical or unaddressed major findings
remain — verdict upgraded REVISE → APPROVE.

---
*Re-review generated by /accelerator:review-plan*
