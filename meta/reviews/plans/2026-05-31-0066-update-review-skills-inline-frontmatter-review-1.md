---
date: "2026-06-01T10:49:55Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-31-0066-update-review-skills-inline-frontmatter.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, documentation, standards, compatibility]
review_pass: 3
status: complete
---

## Plan Review: Move Review/Validation Skills' Frontmatter into Templates

**Verdict:** REVISE

The plan is methodical and self-consistent within the producer self-loop scope it audits — it reuses 0065's TDD scaffolding faithfully, decomposes cleanly into one foundation + four parallelisable per-skill phases + one closing phase, and documents its four design decisions with rationale. Standards conformance against ADR-0033/ADR-0034 is largely exact, and the deviation handling (queued supplementary ADR for the `pr:` prefix, AC amendment for `review_pass` omission) shows good governance hygiene. However, two critical compatibility findings disprove the plan's "no other in-source consumer" claim — the visualiser indexer already reads `target:` as a path on plan-reviews and `work_item_id:` as the work-item cross-reference key — and several test-coverage and correctness gaps weaken the contract enforcement that the plan's value proposition depends on.

### Cross-Cutting Themes

- **Downstream visualiser consumer is broken by the typed-linkage rewrite** (flagged by: compatibility, architecture, documentation) — The Migration Notes claim "No other in-source skill or script parses these frontmatter shapes" is incorrect; the visualiser's Rust indexer reads `target:` (as a path) and `work_item_id:` (as the work-item edge). The `target: "plan:<id>"` and `work_item_id:` drop will silently break plan-review→plan resolution and work-item-review aggregation until the visualiser is updated or 0070's corpus migration is paired with a visualiser change.
- **Test-coverage gaps leave per-type extras unenforced** (flagged by: test-coverage, code-quality, correctness) — `fields_to_assert` omits every review/validation extra (`verdict`, `lenses`, `target`, `review_pass`, `result`), so the SKILL-prose test cannot detect regression in the substitution instructions for the very fields that distinguish review artifacts. Combined with `forbidden_own_id_key` being single-valued (can't guard both `pr_title` AND `review_pass` absence on `pr-review`), no regex shape test on `target` values, and no mustache-token-survivor automation — the headline contract is largely manual.
- **Canonical persistence-step snippet is copy-pasted, not shared** (flagged by: code-quality, documentation) — Four near-identical text blocks land in four SKILL.md files with per-phase carve-outs (Phase 4 omits `review_pass` + adds `pr_number`; Phase 5 omits review-extras + adds `result`). Future schema changes require coordinated edits across four files; no test diffs the variants.
- **`pr:` prefix and `work_item_id` drop are unilateral schema extensions ahead of governance** (flagged by: architecture, standards, compatibility) — The `pr:` discriminator is queued as an ADR follow-up but emission ships immediately; the `work_item_id` drop on `work-item-review` makes that artifact type asymmetric in identity-key shape relative to its peers. Both are defensible but not yet ratified.
- **Re-review mutation field-set contradicts itself** (flagged by: correctness, architecture, code-quality) — The five-field mutation prose says `date` is updated but the explanatory parenthetical says `date` is preserved for original-review parity. Two implementers will produce two behaviours.

### Tradeoff Analysis

- **Schema unification (standards) vs. consumer compatibility**: The plan's goal — collapse skill→schema coupling to a single edge per artifact type — is architecturally sound, but rushing the typed-linkage emission ahead of consumer-side updates means the visualiser sees silently-orphaned artifacts during the divergence window. Recommendation: either ship a visualiser update alongside Phase 2/Phase 3 (accept both path-form and id-form per ADR-0034) or keep `work_item_id:` as a transitional alias.
- **DRY snippet (code-quality) vs. per-phase explicit variation (correctness)**: A shared snippet file is the DRY choice but reduces per-phase visibility of variations; the current copy-paste-with-carve-outs is more readable per-phase but accumulates drift surface. Recommendation: keep current structure but add a `pr_number` bullet to the canonical snippet enumeration (with `*(pr-review only)*` qualifier) so per-phase additions are anchored in the canonical shape.

### Findings

#### Critical

- 🔴 **Compatibility**: Visualiser indexer reads `target:` as a path on plan-reviews — typed-linkage rewrite breaks plan-review→plan resolution
  **Location**: Migration Notes "Consumer-side breakage surface" (lines 1278-1285); Phase 2 Design — `plan-review.target` rewrite
  `skills/visualisation/visualise/server/src/indexer.rs:744-750` (`target_path_from_entry`) reads `target:` on `PlanReviews` entries and passes it to `normalize_target_key`, which expects a project-root-relative path string. The new `"plan:<id>"` shape will not resolve to an existing file, causing `reviews_by_target` lookups to silently return zero results. The plan's Migration Notes claim "No other in-source script parses this frontmatter today" is incorrect and must be corrected; the plan should either co-ship a visualiser update accepting both path-form and `doc-type:id` form per ADR-0034 §"Forms", or sequence a visualiser-update story before 0066's emission lands.

- 🔴 **Compatibility**: Dropping `work_item_id:` breaks visualiser's work-item cross-reference aggregation
  **Location**: Phase 3, Design Decision #2
  `skills/visualisation/visualise/server/src/frontmatter.rs:305-356` (`read_ref_keys`) reads `work_item_id:` as the primary scalar key for work-item cross-reference aggregation across all artifact types (not gated on `type`). It is consumed by `work_item_refs_by_target`, `declared_outbound`, and `work_item_refs_by_id`. No code path parses `target: "work-item:<id>"` into the `work_item_refs` aggregation today. Post-0066, work-item-review artifacts will silently disappear from work-item "Referenced By" pages. Either retain `work_item_id:` as a transitional alias (mirroring the existing `work-item:` fallback at `frontmatter.rs:334-341`) or extend `read_ref_keys` to extract IDs from `target:` values before this plan ships.

#### Major

- 🟡 **Compatibility**: 0065 already shipped `validation.md` with `target: "plan:..."` and the visualiser does not yet resolve it
  **Location**: Phase 5 §1.b; Migration Notes
  The visualiser's `target_path_from_entry` is gated to `PlanReviews` and expects path-form. After Phase 5, `validate-plan` emits `target: "plan:<id>"`, but no in-source consumer reads it. The plan's claim that the `target:` shape is "finalised" for downstream consumers is one-sided. Add a Migration Notes bullet making the visualiser consumer-update an explicit co-dependency.

- 🟡 **Compatibility**: Eval fixture rewrite is narrowly scoped — assertion text strings still mention legacy `skill:` field
  **Location**: Phase 3 §3 — eval fixture updates
  Phase 3's plan replaces `"skill": "review-work-item"` JSON keys with `"producer":`, but the human-readable `expected_output` / `text` strings at `evals/evals.json:30,34` and `benchmark.json:89` contain literal `skill: review-work-item` in the assertion prose. Either the eval grader matches loosely (silent contract drift) or fails noisily. Extend Phase 3 §3 to rewrite assertion text strings too.

- 🟡 **Compatibility**: `validate-plan`'s `skill:`→`producer:` rename creates a divergence window where new and old reports differ in shape
  **Location**: Phase 5 §1.b
  No in-source consumer reads `skill:` today (confirmed by grep), so the divergence is observational, but the discovery-pass record in Phase 6 should acknowledge that re-running `validate-plan` on a previously-validated plan yields a new shape rather than an in-place update.

- 🟡 **Architecture**: `pr:` discriminator emitted before ADR-0034 vocabulary extension is accepted
  **Location**: Design Decisions §3; Phase 6 §4
  Mitigated only by a single follow-up bullet appended to epic 0057, not a tracked work item. Schema decision and formal record diverge for an unbounded window. Suggestion: raise a concrete work-item ID for the supplementary ADR and gate Phase 4 on its acceptance, or record `pr:` as a provisional vocabulary entry in ADR-0034 explicitly.

- 🟡 **Architecture**: Asymmetric foreign-reference identity model across review types
  **Location**: Design Decisions §2; Phase 3 §2.c
  `work-item-review` is the only review type whose downstream consumer must parse a relationship-named `target` to extract a foreign id; the per-type `<type>_id` uniformity is partially collapsed without an ADR amendment. Suggestion: keep `work_item_id` alongside `target` (ADR-0033 allows both since they encode different roles), or capture the principle in a supplementary ADR.

- 🟡 **Code Quality**: Persistence-step snippet is copy-pasted into four SKILL.md files with per-phase carve-outs
  **Location**: Implementation Approach §"Canonical template-inclusion + persistence-step pattern" and Phases 2-5
  Four hand-edited variants of a 16-line bullet list with phase-specific deviations. SKILL-prose test only checks four named field-name+verb pairs appear in a persistence-headed section — does not detect drift between variants. Suggestion: extract the snippet into a shared fragment referenced via the template-inclusion mechanism, or add a `test-persistence-snippet-shape.sh` that diffs the four variants against a canonical golden.

- 🟡 **Code Quality**: TSV `forbidden_own_id_key` is single-valued but `pr-review` needs to guard absence of two keys
  **Location**: Phase 1 §1; Phase 4 §1
  Single column can encode `pr_title` OR `review_pass` absence, not both. The plan picks `pr_title` for TSV and relegates `review_pass` absence to ad-hoc grep in Phase 4 Automated Verification. Suggestion: extend the TSV column to accept space-separated values, or add a `forbidden_extras` column.

- 🟡 **Test Coverage**: `fields_to_assert` omits every per-type extra, leaving review/validation extras unverified by the SKILL-prose test
  **Location**: Phase 1 §2; Phase 5 Success Criteria
  Rows list only base fields (`producer schema_version last_updated last_updated_by`). A skill could drop `verdict:`, `target:`, `review_pass:` bullets entirely from its persistence-step snippet and still PASS. Suggestion: extend each row's `fields_to_assert` to include per-type extras (`verdict target reviewer lenses review_number review_pass` for plan/work-item-review; minus `review_pass` plus `pr_number` for pr-review; `result target` for validate-plan).

- 🟡 **Test Coverage**: No automated coverage for the pinned target-value regex shape on any review template
  **Location**: Phase 4 §1; Phase 4 Manual Verification
  Plan acknowledges the gap and defers to manual verification. Visualiser-graph epic depends on the shape. Suggestion: add a fourth TSV column `target_value_regex` and assert it against any non-empty template value, or add a `test-review-artifact-shape.sh` against golden fixtures.

- 🟡 **Test Coverage**: `review_pass`-on-`pr-review` absence is not enforced by any automated assertion
  **Location**: Phase 1 §1; Phase 4 Design Decision #1
  Design Decision #1 pins absence; only the ad-hoc Automated-Verification grep enforces it. Pairs with the `forbidden_own_id_key` finding above.

- 🟡 **Test Coverage**: Phase-1 allowlist move neutralises the discovery assertion during Phases 2-5
  **Location**: Phase 1 §3
  Moving the four paths from `OWNED_BY_0066` to `IN_SCOPE_PRODUCERS` before any per-skill rewire lands makes the discovery assertion pass tautologically throughout Phases 2-5. AC #9's "reproducible discovery pass" provides no incremental signal. Suggestion: keep paths in `OWNED_BY_0066` until each per-skill phase lands and move them out as that phase's step.

- 🟡 **Test Coverage**: `validation.md`'s TSV row is not updated to require `target` as an extra
  **Location**: Phase 5
  Existing TSV row lists only `result`; Phase 5 emits `target: "plan:<id>"` but doesn't extend the TSV row. If `validation.md`'s `target: ""` slot is ever removed, no test fires.

- 🟡 **Test Coverage**: Eval fixture update edits existing assertions in place rather than adding regression cases for the old shape
  **Location**: Phase 3 §3
  After the rewrite, no fixture asserts that the old shape (`skill: review-work-item`, `work_item_id:`) is absent. Suggestion: retain existing assertions as negative assertions in addition to the new positives.

- 🟡 **Correctness**: Five-field re-review mutation contradicts itself on `date`
  **Location**: Phase 2 §2.c; Phase 3 §2.d
  Bullet text places `date` in the mutation list ("update exactly five frontmatter fields — `verdict`, `review_pass`, `date`, `last_updated`, and `last_updated_by`") but the parenthetical says "`date` is preserved alongside `last_updated` to keep parity with the original-review timestamp". Two valid resolutions, different downstream semantics. Pick one and remove the contradiction.

- 🟡 **Correctness**: Plan describes SKIP-when-both-absent fallback but proposed code does not implement it
  **Location**: Phase 1 §5
  Prose says "retain a SKIP only when both are absent" but proposed code falls into FAIL when both work-item files are missing (empty `wi_templates` vs non-empty `tsv_templates`). Add an explicit guard, or drop the SKIP claim.

- 🟡 **Documentation**: Inline-comment policy underspecifies the new per-type extras
  **Location**: Implementation Approach §"Inline-comment policy"
  Silent on `review_number`, `review_pass`, `pr_number`, `result`. Risk of inconsistent comment density across the parallel Phases 2-5. Suggestion: enumerate every per-type extra explicitly.

- 🟡 **Documentation**: `pr_number` typed as integer in template but inline comment does not state the bare-integer contract
  **Location**: Phase 4 §1 template
  ADR-0033 §Identity-value contract makes bare-integer the load-bearing rule for numeric foreign references; the `pr-description.md:11` precedent documents this explicitly. The new template should mirror that comment.

- 🟡 **Documentation**: Canonical persistence-step snippet's `target:` bullet does not pin the typed-linkage YAML-string syntax
  **Location**: Implementation Approach §"Canonical template-inclusion + persistence-step pattern"
  Bullet says `target: ← {target-shape}` but doesn't specify "single quoted YAML string in `\"doc-type:id\"` form" the way the `id:` bullet says "always quoted as a YAML string". Update the bullet so the contract propagates uniformly into all four phases.

#### Minor

- 🔵 **Architecture**: Review-family schema asymmetry without a tracked path to convergence (`review_pass` omitted from `pr-review`)
- 🔵 **Architecture**: Cross-check coupling between test driver and per-story work-item files grows linearly
- 🔵 **Architecture**: Re-review mutation field-set expansion is a behavioural change folded into a frontmatter-location story
- 🔵 **Architecture**: Template-inclusion placement convention worth pinning explicitly
- 🔵 **Code Quality**: Cross-check union read accumulates O(n) work-item paths as schema stories land
- 🔵 **Code Quality**: Re-review in-place mutation expands across two skills with implicit invariants; review-plan misses the malformed-frontmatter fallback that review-work-item retains
- 🔵 **Code Quality**: Templates carry both frontmatter contract and a body skeleton; risk of body-shape drift vs SKILL.md prose
- 🔵 **Code Quality**: Phase 6 mutates its own plan's work item to retroactively codify a Design Decision
- 🔵 **Test Coverage**: Mustache-token-survivor check exists only in prose, with no test driver
- 🔵 **Test Coverage**: Union cross-check silently dedupes — no assertion that a template appears in exactly one Schema Reference table
- 🔵 **Test Coverage**: Outside-fenced-block greps rely on rg's flag set rather than the in_fenced_block helper
- 🔵 **Test Coverage**: Five-field re-review mutation expansion has no automated regression test
- 🔵 **Test Coverage**: Phase 6 discovery-pass record is a snapshot, not an executable test
- 🔵 **Correctness**: `sort -u` in union pipeline masks duplicate-row detection inside a single work item
- 🔵 **Correctness**: `OWNED_BY_0066=()` empty-array expansion may error under `set -u` on bash 3.2
- 🔵 **Correctness**: `target` regex for work-item-review is broader (`[0-9]{4,}`) than the documented 4-digit emission shape
- 🔵 **Correctness**: Phase 4 template-inclusion placement splits two consecutive IMPORTANT notes (lines 31-36 ambiguity)
- 🔵 **Documentation**: Schema Reference table omits typed-linkage shape and `pr_number` as columns; stuffs them into parenthetical free-text
- 🔵 **Documentation**: Supplementary ADR follow-up not discoverable from the Schema Reference table
- 🔵 **Documentation**: Phase 5 does not document the legacy `skill: validate-plan` removal in the prose changes the way Phase 4 documents `pr_title:`
- 🔵 **Documentation**: Template-inclusion heading `{Artifact-Type}` capitalisation rule not pinned (PR vs Pr)
- 🔵 **Standards**: `pr_number` bare-integer shape vs ADR-0033 quoted-string foreign-reference contract — defensible by precedent but unstated
- 🔵 **Standards**: Body skeleton headings use H2 vs ADR-0033 H1 mention
- 🔵 **Standards**: AC #5 amendment duplicates info from the Schema Reference table
- 🔵 **Compatibility**: Phase-11 discovery patterns may still match legacy `pr_title:` / `work_item_id:` in other SKILL.md files outside fenced blocks
- 🔵 **Compatibility**: Re-review mutation behaviour on pre-0066 artifacts (missing `last_updated`) is not specified

#### Suggestions

- 🔵 **Code Quality**: Add a post-generation lint helper to automate the unsubstituted-token regex check
- 🔵 **Test Coverage**: Commit golden artifact fixtures per skill and add `test-golden-frontmatter.sh` to lock down regex/enum/ISO/token shapes
- 🔵 **Correctness**: Anchor `pr_number` in the canonical snippet enumeration (with `*(pr-review only)*` qualifier) rather than as a per-phase ad-hoc addition
- 🔵 **Correctness**: Align Pass-B grep recipe regex anchoring with the test driver's anchored discovery patterns
- 🔵 **Documentation**: Promote the Token-marker/fence convention into ADR-0033 (or addendum) so it survives 0066's landing
- 🔵 **Documentation**: Use four-backtick fences (or indented blocks) for the Phase 6 §2 nested markdown sample to avoid premature fence-closing
- 🔵 **Standards**: Document the `forbidden_own_id_key` `-` vs concrete-key convention in the TSV header or plan prose

### Strengths

- ✅ Architectural direction is correct: relocating frontmatter into versioned templates collapses skill→schema coupling to a single edge per artifact type, giving the corpus a single point of evolution.
- ✅ TDD with explicit RED baseline locked in Phase 1 — both test drivers fail before any per-skill phase lands and turn GREEN incrementally, giving feedback at every step.
- ✅ Strong phase cohesion (one template + one SKILL.md per phase) with explicit ordering constraints and file-disjoint Phases 2-5 enabling safe parallelisation by different engineers.
- ✅ Plan reuses the 0065 canonical inclusion-and-persistence-step pattern verbatim, preserving architectural consistency across all ten unified producers.
- ✅ Design Decisions section makes four non-obvious choices explicit with written rationale grounded in ADR-0033/0034 rather than letting them sit implicit.
- ✅ Token-marker and fence syntax conventions are pinned with regex (`\{[^{}\n]+\}`), removing ambiguity from manual verification recipes.
- ✅ Plan acknowledges and defers debt (pr: prefix follow-up ADR, 0093 typed-linkage slot extension, verdict-enum alignment) rather than smuggling it into scope.
- ✅ ADR-0033/0034 conformance is largely exact: base fields, identity-value shapes, `target` typed-linkage form, TSV row formats.
- ✅ Verdict-enum inconsistency (REVISE vs REQUEST_CHANGES) is preserved verbatim per parent epic scope — faithful respect for ADR-0033 §Out of scope.
- ✅ Migration Notes section documents the 0065→0066 gap window for plan-validation, the pr_title carryover from 0065, the 0093 sibling-story relationship, and the (claimed) consumer-side breakage surface.
- ✅ `forbidden_own_id_key` discipline correctly retires `work_item_id` (work-item-review) and `pr_title` (pr-review) via TSV-level assertion.

### Recommended Changes

1. **Address the visualiser consumer breakage before/with this story** (addresses: 2× critical compatibility findings)
   Either (a) extend the visualiser indexer in the same release to accept both path-form and `doc-type:id` form on `target:` per ADR-0034 §"Forms", and extend `read_ref_keys` to extract work-item IDs from `target:` values matching `^work-item:`, treating this as part of 0066's scope; or (b) split off a co-shipped story that updates the visualiser before 0066's emission lands; or (c) keep `work_item_id:` as a transitional alias on `work-item-review` artifacts. Either way, correct the "no other in-source consumer" claim in Migration Notes.

2. **Resolve the `date` contradiction in the re-review mutation** (addresses: correctness finding on five-field mutation)
   In Phase 2 §2.c and Phase 3 §2.d, pick one of: (a) `date` stays in the mutation list (matches current behaviour; drop the "preserved" parenthetical), or (b) `date` is removed from the mutation list (only `last_updated`/`last_updated_by` advance). State the choice in §Design Decisions explicitly.

3. **Strengthen test-coverage for per-type extras** (addresses: 6× major test-coverage findings)
   - Extend each Phase 1 skills-schema row's `fields_to_assert` to include per-type extras.
   - Extend `templates-schema.tsv` `forbidden_own_id_key` to a space-separated multi-value column (or add `forbidden_extras`), then list `pr_title review_pass` on `pr-review.md`'s row.
   - Update `validation.md`'s TSV row to add `target` as an extra.
   - Add a `target_value_regex` column or a separate golden-artifact test that locks down the per-type regex shapes.
   - Keep `OWNED_BY_0066` populated through Phases 2-5 so the discovery assertion provides incremental signal; move each path out as the corresponding phase lands.

4. **Anchor `pr_number` in the canonical snippet** (addresses: correctness finding on per-phase additions)
   Add a `pr_number:` ← bullet to the canonical snippet bullet-list with a `*(pr-review only)*` qualifier (mirroring the `*(plan-validation only)*` qualifier on `result`), so the snippet remains the complete shape.

5. **Pin documentation contracts that affect parallel implementation uniformity** (addresses: 3× major documentation findings)
   - Update the canonical snippet's `target:` bullet to specify "single quoted YAML string in `\"doc-type:id\"` form".
   - Extend the §"Inline-comment policy" to enumerate `review_number`, `review_pass`, `pr_number`, `result` explicitly (with required/recommended/omitted classification).
   - Update `pr-review.md` template's `pr_number:` comment to cite ADR-0033 and "bare integer; foreign reference" per the `pr-description.md:11` precedent.

6. **Correct the SKIP-fallback prose-vs-code mismatch in Phase 1 §5** (addresses: correctness finding)
   Either implement the "retain SKIP when both files absent" guard in the proposed code, or drop the claim and accept FAIL-on-missing as the new behaviour.

7. **Tighten eval fixture rewrite scope** (addresses: compatibility finding on eval text strings)
   Extend Phase 3 §3 to rewrite the `expected_output` / `text` assertion strings too, not just the JSON keys. Add a grep guard confirming zero remaining `skill: review-work-item` substrings.

8. **Document downstream consumer coordination explicitly in Migration Notes** (addresses: compatibility finding on validation typed-linkage gap and visualiser coordination)
   Add a Migration Notes bullet stating: "The visualiser's `target_path_from_entry` (`indexer.rs:744`) expects path-form `target:` and is gated to plan-reviews. The typed-linkage emission this story ships is unread by any in-source consumer until the visualiser is updated; that update must accept both `doc-type:id` and path form per ADR-0034 §Forms."

9. **Address minor architectural/documentation polish** (addresses: minor findings)
   - Pin Phase 4 template-inclusion placement explicitly between lines 36-38 (after both IMPORTANT notes).
   - Tighten `work-item-review` `target` regex to `^"work-item:[0-9]{4}"$`.
   - Add a Migration Notes line about pre-0066 artifacts and the re-review mutation's field-insertion behaviour.
   - Either delete `OWNED_BY_0066=()` empty array or guard expansion for bash 3.2 portability.
   - Use `sort` (not `sort -u`) in cross-check union, or add explicit within-file dedup check.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan reinforces a sound architectural direction — relocating frontmatter from skill prose into versioned template files reduces coupling between schema and producer skills and gives the corpus a single point of evolution. Structure mirrors the 0065 precedent precisely (TDD with RED baseline, TSV-driven contracts, canonical inclusion/persistence-step pattern), enabling clean parallelism across the four per-skill phases. The most consequential architectural risks are vocabulary asymmetries that the plan acknowledges but does not fully close: `pr-review.target` uses a `pr:` discriminator not yet in ADR-0034's published vocabulary, `pr-review` omits `review_pass` while peer review types carry it, and `work-item-review` drops `work_item_id` leaving relationship-named `target` as the sole identity carrier.

**Findings**: 2 major, 4 minor (see consolidated findings above).

### Code Quality

**Summary**: Methodical and reuses an established TDD scaffolding pattern from 0065 rather than reinventing infrastructure — a clear win for maintainability. However, the "canonical persistence-step snippet" is in practice four near-identical text blocks with per-phase carve-outs reused by manual copy-paste into each SKILL.md rather than by reference — a DRY violation the plan acknowledges only implicitly. Several test-asymmetries push correctness assertions out of the automated test driver into manual verification steps, which will erode under maintenance.

**Findings**: 2 major, 4 minor, 1 suggestion.

### Test Coverage

**Summary**: Reuses 0065's TSV-driven test scaffolding well: each phase has a clear RED→GREEN transition, automated assertions are explicit per phase, and unit-level field-presence/shape coverage matches what the predecessor established. However, several specific gaps reduce confidence that the plan's stated guarantees actually hold end-to-end — most notably no automated coverage for the regex shape of `target:` values, no test enforcement of the `review_pass`-absent and `pr_title`-absent decisions on `pr-review`, a `fields_to_assert` set that omits every per-type extra, and a Phase-1 allowlist move that makes the discovery assertion a no-op during Phases 2-5.

**Findings**: 6 major, 5 minor, 1 suggestion.

### Correctness

**Summary**: Line-number references match the actual SKILL.md files, TSV row formats are internally consistent, and the per-phase parameter substitutions correctly account for Phase-4/5 deviations. However, there is a significant internal contradiction in the five-field re-review mutation description (the bullet says `date` is in the mutation list, the parenthetical says `date` is preserved unchanged), a SKIP-branch behaviour described in prose but not implemented in the proposed code, and an `OWNED_BY_0066=()` empty-array survival pattern that is fragile under `set -u` on older bash versions.

**Findings**: 2 major, 4 minor, 2 suggestions.

### Documentation

**Summary**: Unusually thorough and self-contained for a documentation-heavy refactor: it pins token-marker and fence-syntax conventions, reuses 0065's canonical persistence-step snippet verbatim, articulates four design decisions with rationale, and includes a Schema Reference table for the new templates. However, several documentation gaps risk uneven implementer output across the parallelisable Phases 2-5: the inline-comment policy under-specifies which fields on each new review template should carry comments; the canonical snippet's `target:` bullet is parameterised but the YAML-string-syntax requirement is implicit; and the relationship to follow-up stories is documented in Migration Notes but is not surfaced inside the Schema Reference table where readers most likely look.

**Findings**: 3 major, 3 minor, 2 suggestions.

### Standards

**Summary**: The plan adheres closely to the established 0065 cohort conventions and the ADR-0033/ADR-0034 contracts. Base-schema fields, identity-value shape, `target` typed-linkage vocabulary, TSV row formats, and the canonical template-inclusion + persistence-step pattern are all reused verbatim. The two documented deviations (`review_pass` omitted from `pr-review`, `pr:<pr-number>` not yet in ADR-0034's published vocabulary) are handled with proper governance: an AC amendment and a queued supplementary ADR respectively, not silent adoption.

**Findings**: 0 major, 3 minor, 1 suggestion.

### Compatibility

**Summary**: Mostly well-scoped on the in-source SKILL.md self-loop, but it materially understates the downstream consumer surface: the visualiser server (Rust) already parses `target:` as a project-root-relative path on plan-reviews and reads `work_item_id:` as the primary work-item cross-reference key for all artifact types. The plan's typed-linkage rewrites will silently break the visualiser's plan-review→plan resolution and the work-item-review→work-item cross-reference aggregation, contradicting the Migration Notes claim. The corpus migration story 0070 is real and exists, but it owns rewriting `meta/` artifacts — not updating the visualiser's reader, which is a separate compatibility surface the plan does not address.

**Findings**: 2 critical, 3 major, 2 minor.

---

## Re-Review (Pass 2) — 2026-06-01

**Verdict:** REVISE

The plan has been substantially edited since Pass 1. Both critical compatibility findings are resolved (Phase 7 added, `work_item_id:` transitional alias retained, Migration Notes corrected), and most Pass-1 major findings are addressed. However, the editing pass introduced internal inconsistencies (five-field vs four-field mutation language; "Four" header above five design decisions; Phase 5 §0 array cleanup placed in a parallelisation-fragile location), and the new Phase 7 has its own real implementation feasibility gap — the proposed `resolve_plan_id_to_path` cannot work against `target_path_from_entry`'s existing synchronous free-function signature without additional plumbing the plan does not describe.

I have applied delta fixes during the re-review pass for several of the lower-effort findings (five-field→four-field everywhere, Four→Five design decisions, Phase 5 §0 array-cleanup moved to Phase 6 §0a, work-item-read-field.sh argument order corrected). The remaining majors below are the ones that warrant another pass.

### Previously Identified Issues

- 🔴 **Compatibility**: Visualiser indexer reads `target:` as path on plan-reviews — **Resolved** (Phase 7 §1 extends `target_path_from_entry` to accept typed-linkage form; Migration Notes corrected).
- 🔴 **Compatibility**: Dropping `work_item_id:` breaks visualiser cross-reference aggregation — **Resolved** (Design Decision #2 revised; `work_item_id:` kept as transitional alias; Phase 7 §2 extends `read_ref_keys` to also extract from `target:`).
- 🟡 **Correctness**: Five-field re-review mutation contradicts itself on `date` — **Resolved at the design level** (Design Decision #5 added; Phase 2 §2.c and Phase 3 §2.d rewritten to four-field) — **but stale "five-field" language at line 816 and line 1593** introduced a new inconsistency that I fixed during the re-review pass.
- 🟡 **Correctness**: SKIP-fallback prose-vs-code mismatch — **Resolved** (Phase 1 §5 now includes explicit existence-count guard; `sort -u` → `sort`).
- 🟡 **Test Coverage**: `fields_to_assert` omits per-type extras — **Resolved** (Phase 1 §2 extended; per-type extras now asserted).
- 🟡 **Test Coverage**: No automated coverage for `target` regex shape — **Partially resolved** (Phase 7 unit tests cover Rust-side; per-type regex shape on the SKILL-prose side still relegated to manual verification).
- 🟡 **Test Coverage**: `review_pass`-absence on `pr-review` not enforced — **Resolved** (Phase 1 §1a introduces multi-value `forbidden_own_id_key`; `pr-review.md` row now forbids `pr_title review_pass`).
- 🟡 **Test Coverage**: Phase-1 allowlist move neutralises discovery assertion — **Resolved** (each per-skill phase now moves its own path; Phase 1 leaves OWNED_BY_0066 unchanged).
- 🟡 **Test Coverage**: `validation.md` TSV row not updated to require `target` — **Resolved** (Phase 1 §1b adds `target` to extras).
- 🟡 **Test Coverage**: Eval fixture rewrite narrowly scoped — **Resolved** (Phase 3 §3 now rewrites assertion-text strings + adds negative `skill:`-absence assertion).
- 🟡 **Documentation**: Inline-comment policy underspecifies per-type extras — **Resolved** (now enumerates every extra with required/recommended/omitted classification).
- 🟡 **Documentation**: `pr_number` template comment lacks ADR-0033 citation — **Resolved** (now cites ADR-0033 §Identity-value shape contract).
- 🟡 **Documentation**: Canonical snippet `target:` bullet doesn't pin YAML-string syntax — **Resolved** (now states "single quoted YAML string in `\"doc-type:id\"` form").
- 🟡 **Compatibility**: validate-plan typed-linkage gap / eval text strings / Migration Notes coordination — **Resolved** (Migration Notes bullets added for re-review mutation, pre-0066 artifact handling, validate-plan divergence).
- 🟡 **Architecture**: `pr:` discriminator emitted before ADR-0034 update — **Partially resolved** (Schema Reference table now references the queued supplementary ADR; governance still rests on a single follow-up bullet rather than a tracked work item).
- 🟡 **Architecture**: Asymmetric foreign-reference identity model — **Resolved differently than originally suggested** (transitional alias retained instead of dropping; pragmatic and correct given visualiser coupling).

All minor and suggestion-level Pass-1 findings either addressed via Task #9 polish edits or accepted as known tradeoffs (the canonical persistence-step snippet copy-paste remains; documented as accepted technical debt).

### New Issues Introduced or Surfaced

- 🟡 **Correctness / Architecture (Phase 7 §1)**: `resolve_plan_id_to_path` cannot be implemented with `target_path_from_entry`'s current sync, free-function signature. There is no `plans_by_id` index, and the helper as sketched would either need a new secondary index (parallel to `work_item_by_id` at `indexer.rs:206`) or a refactor passing the entries snapshot through. Phase 7 §1 needs either a discrete sub-step adding the index plumbing or a signature refactor described explicitly.
- 🟡 **Architecture**: The divergence window between Phase 2/3 emission and Phase 7 consumer-update is not bounded by an explicit release-boundary constraint. The plan should state "all seven phases must ship in the same release" or sequence Phase 7 first (the consumer-update is back-compatible).
- 🟡 **Test Coverage**: The multi-value `forbidden_own_id_key` parser extension (Phase 1 §1a) has no self-test guarding the single-value and `-` sentinel paths against tokenisation regression.
- 🟡 **Test Coverage**: `in_imperative_section` matching remains loose — a SKILL.md could satisfy the per-field assertion by having body-content prose mention `verdict:` in any `Step N` section, without the canonical persistence-step snippet being present. Pass-1 hinted at this; Pass-2 names it explicitly as a SKILL-prose-test gap.
- 🔵 **Correctness (Phase 6 §1)**: Post-rewire, the canonical persistence-step snippet emits literals as markdown bullets (`    - \`target:\` ← ...`) which do **not** match the Pass-B regex `^[[:space:]]*target:`. The discovery-assertion reasoning that "every Pass-B hit must resolve to an allowlisted SKILL.md" becomes vacuously true rather than substantive. Either reword the reasoning or extend the Pass-B regex.
- 🔵 **Documentation**: The transitional `work_item_id:` alias retirement path is described three times with subtly different exit conditions (Phase 7 retires it / a follow-up release retires it / Phase 7 makes it redundant but doesn't remove). Align the three descriptions.
- 🔵 **Standards**: Body-skeleton headings on the three new review templates use H2 (mirroring `validation.md`) while ADR-0033 §Base schema states `title:` is "kept in sync with body H1 where applicable". Either align to H1 across the cohort (cleaner) or capture the H2 exception explicitly as a Design Decision.
- 🔵 **Standards**: `pr_number` bare-integer shape contradicts ADR-0033's quoted-string foreign-reference contract literal reading. Pre-existing per `pr-description.md` precedent, but not formally exempted; worth queuing alongside the `pr:` supplementary ADR.
- 🔵 **Code Quality**: Phase 5 §1.e "Renumber Step 3 if needed" leaves dangling prose ("Create comprehensive validation summary using this template:" with no antecedent after the template-inclusion is moved up). Specify the exact target prose.
- 🔵 **Test Coverage**: Phase 7 §3 Rust unit-test enumeration omits two useful cases — `target+work_item_id both present, both = "0042"` (alias precedence + no double-counting) and `target: "pr:123"` returns empty vec (prefix-exactness for non-work-item targets).
- 🔵 **Compatibility**: `target:`-based work-item extraction in Phase 7 §2 is dead-on-arrival while the transitional alias still wins. The new code path has no end-to-end coverage against real production artifacts during 0066's life. Acceptable but worth noting.
- 🔵 **Compatibility**: `plan-validation` target shape change (Phase 5) ships typed-linkage `target: "plan:<id>"` but Phase 7 only extends `target_path_from_entry` for `PlanReviews` (not `Validations`). validation→plan declared edges remain unreachable from the visualiser graph. Add an explicit disclaimer or extend Phase 7.

### Delta Fixes Applied During Pass 2

I corrected the lowest-effort findings during the re-review:

1. **Five-field → four-field** at Phase 2 Manual Verification (line 816) and Manual Testing Steps step 5 (line 1593) to align with Design Decision #5.
2. **"Four design decisions" → "Five design decisions"** in the Design Decisions section preamble.
3. **Phase 5 §0 array cleanup** moved to a new **Phase 6 §0a** (array cleanup runs in the closing phase, which is guaranteed to land last); Phase 5 §0 now only moves its own path and leaves the empty array for Phase 6 to remove.
4. **`work-item-read-field.sh` argument order** corrected to `id {path}` (field-name first per the script's documented usage); plan now explicitly notes this fixes the pre-existing reversed-arg invocation at SKILL.md line 343.

### Assessment

Verdict remains **REVISE**, but the plan has materially improved. The two critical compatibility findings are resolved, ~14 of 17 Pass-1 majors are resolved (most via Phase 7 + Design Decision #5 + test-coverage TSV extensions). The remaining majors are:

1. **Phase 7 §1 implementation feasibility** — load-bearing for the new Phase 7. Without explicit secondary-index plumbing or a signature refactor, the phase won't compile as described.
2. **Release-boundary constraint absence** — Phase 7 must ship with Phases 2/3 (or before them) to avoid recreating the silent-orphan window.
3. **Multi-value parser self-test** — small but worth adding.
4. **`in_imperative_section` matching loose** — the SKILL-prose test's strongest guardrail still has a slack that could let regressions through.

Recommended next iteration: address (1) and (2) substantively; (3) and (4) are smaller tightening opportunities. The plan is otherwise ready for implementation; if (1) is fixed and (2) is recorded explicitly, the plan could move to APPROVE.

### Post-Pass-2 Delta Fixes (findings 1 and 2)

Applied during this same review pass after the assessment above:

1. **Phase 7 §1 rewritten with explicit `plans_by_id` secondary-index design** (1a–1i sub-steps), parallel to the existing `work_item_by_id` plumbing at `indexer.rs:206`/`248`/`268`/`317`/`345`/`350`/`386`/`414`/`451`/`591`/`639`/`804`/`858`. Added a new `plan_id_from_entry` helper and `update_plans_by_id` / `remove_from_plans_by_id` helpers mirroring the work-item versions. Phase 7 now contains five discrete sub-steps (§1 add index, §2 refactor `target_path_from_entry`, §3 extend `read_ref_keys`, §4 unit tests, §5 cargo test).
2. **Phase 7 §2 specifies the refactored `target_path_from_entry` signature** (`entry, plans_by_id, project_root`) and enumerates all five call sites with their lock contexts and snapshot sources. Adds a two-pass rescan design that resolves the build-loop ordering dependency (plan-reviews can reference plans that appear later in the file-driver enumeration).
3. **Sequencing-note section added to Phase 7** stating the consumer-update is back-compatible (still accepts path-form), so Phase 7 may safely land *before* Phases 2-3, eliminating the divergence window. The Implementation Approach section was updated correspondingly: "Phase 7 may land before Phases 2-3 without breaking existing path-form consumers; if not landed first, all seven phases must ship in the same release."
4. **Phase 7 §4 unit-test enumeration expanded** to cover: build-loop ordering independence (validates the two-pass design), `plans_by_id` lifecycle (insert / update on id-change / remove), alias-precedence with same value (`target` + `work_item_id` both = `"0042"` returns single entry, no double-counting), and non-work-item prefix exactness (`pr:` returns empty vec).
5. **Phase 7 Automated Verification extended** with explicit grep checks confirming the `plans_by_id` field, the refactored 3-arg `target_path_from_entry` signature at every call site, and the new test cases.

With these fixes, the remaining majors are #3 (multi-value parser self-test) and #4 (`in_imperative_section` slack) — both small tightening opportunities. The plan is now ready for implementation; recommend a third pass only if (#3) and (#4) need to be locked down before merging.

---

## Re-Review (Pass 3) — 2026-06-01

**Verdict:** COMMENT

Pass 3 surfaced one self-inflicted major and a constellation of minor refinements. The major was a Pass-2-introduced contradiction: when I added `work_item_id:` back to the template/skill as a transitional alias (Task #1), I missed updating Phase 3's existing Automated/Manual Verification criteria that still asserted the field's absence. That contradiction is now fixed during this pass. With it resolved, no remaining major findings block implementation — the plan is ready to ship; remaining items are either accepted tradeoffs (documented inline) or small refinements that can be addressed in implementation review or as follow-ups.

### Previously Identified Issues (from Pass 2)

- 🟡 **Architecture / Correctness (Phase 7 §1)**: `resolve_plan_id_to_path` infeasible without restructuring — **Resolved** (Pass 2 delta fix introduced `plans_by_id` secondary-index design, 3-arg `target_path_from_entry` signature, and two-pass rescan).
- 🟡 **Architecture**: Release-boundary constraint missing — **Resolved** (Pass 2 delta fix added Phase 7 §"Sequencing note (release boundary)" and updated Implementation Approach).
- 🟡 **Test Coverage**: Multi-value parser self-test — **Acknowledged as outstanding** (Test Coverage lens still flags this; minor — see below).
- 🟡 **Test Coverage**: `in_imperative_section` matching slack — **Acknowledged as outstanding** (still a known SKILL-prose-test gap; minor and unrelated to 0066's scope).

### New Issues Surfaced and Resolution

- 🟡 **Correctness (Phase 3 Success Criteria)**: Phase 3 Automated/Manual Verification still asserted `work_item_id:` is absent from `templates/work-item-review.md` and produced artifacts — contradicting Design Decision #2's transitional alias retention. **FIXED during Pass 3**: lines 995-996 and 1003 inverted to assert `work_item_id:` IS present (template carries alias slot; skill emits same 4-digit id as `target` payload).
- 🔵 **Correctness (Phase 7 §2 table)**: Call-sites table still listed `319 (rescan loop)` as a call site after the two-pass restructuring made it disappear. **FIXED during Pass 3**: row replaced with "Pass B (new — replaces the line-319 call site, which is deleted as part of the two-pass split)"; added an explicit lock-ordering precedence note after the table.
- 🟡 **Documentation**: Insert-if-missing rule for pre-0066 review artifacts only documented in Migration Notes, not in the per-skill prose where implementers will work. **FIXED during Pass 3**: Phase 2 §2.c and Phase 3 §2.d now each carry an explicit "Pre-0066-artifact handling" callout pinning the insertion rule and the malformed-frontmatter fallback boundary.
- 🔵 **Standards (transitional alias retirement)**: `work_item_id:` alias on `work-item-review` had no tracked follow-up parallel to the `pr:` supplementary ADR. **FIXED during Pass 3**: added a second bullet under Phase 6 §4's queued-follow-ups list explicitly naming the alias retirement and its dependency on 0070 corpus migration.
- 🔵 **Compatibility**: `declared_outbound` lock-ordering description was ambiguous (said "acquire plans_by_id.read() first" when in fact it must come after the existing entries+work_item reads). **FIXED during Pass 3**: table row rewritten to enumerate the existing read holds and pin the correct acquisition order (entries → work_item → plans).

### Issues Not Addressed in This Pass (Accepted or Deferred)

- 🟡 **Code Quality**: `target_path_from_entry`'s refactored 3-arg signature ripples across 5 call sites with three different lock contexts. Documented inline (Phase 7 §2 table). The agent suggested either making it a method on `Indexer` or introducing a `TargetResolver` struct. **Accepted as tradeoff for this story** — the parallel-to-`work_item_by_id` pattern is more familiar to maintainers; a resolver-pattern refactor would be a follow-up if more typed-linkage prefixes are added.
- 🔵 **Code Quality**: Two-pass rescan invariant lacks an explicit code-comment locking it in. **Deferred to implementation**: the plan describes the pattern fully; the implementer should add a comment block in `rescan()` per the agent's suggestion.
- 🔵 **Code Quality**: Persistence-snippet flag-arguments smell (5 conditional bullets). **Accepted as known limitation** — splitting into base + per-artifact-type sections would be a larger rework; the current shape is comprehensive and the SKILL-prose test catches drops.
- 🔵 **Code Quality**: TSV column repurposed to multi-value without rename. **Deferred to implementation**: rename to `forbidden_own_id_keys` (plural) is a one-line follow-up; not blocking.
- 🔵 **Test Coverage**: Phase 3 omits mustache-token survivor check; SKILL-prose test no negative assertion; multi-value parser self-test; pre-0066 insert path test; typed-linkage target shape regex automation. **Accepted as tightening opportunities** — each is a small driver-side improvement that can be folded into a follow-up "test scaffolding polish" story without blocking 0066.
- 🔵 **Documentation**: Schema Reference table cells dense; inline-comment policy lacks automation; precedence rule not in source comment. **Accepted as readability-vs-precision tradeoff** — the table density is the cost of self-containment; the inline-comment policy is reviewable in PR review.
- 🔵 **Standards**: `pr:` deviation should be flagged in the `templates/pr-review.md` `target:` comment itself (not just Schema Reference table). **Worth doing in implementation**: a one-line edit to the template comment when the file is created in Phase 4.
- 🔵 **Compatibility**: `work_item_id:`/`target:` agreement assertion not added to tests. **Accepted as low-risk** — the persistence-step snippet ensures both come from the same source; divergence requires deliberate post-emission editing.
- 🔵 **Compatibility**: Path-form vs typed-linkage form precedence in `target_path_from_entry` could be tightened in code-comment. **Deferred to implementation** — Phase 7 §4 tests cover the precedence empirically; a code comment is a one-line polish.
- 🔵 **Compatibility**: `date` contract break for out-of-tree consumers. **Acknowledged in Migration Notes** — no in-source consumer reads `date:` as a recency signal (confirmed); out-of-tree migration is a documentation problem, not a code problem.

### Assessment

**Verdict: COMMENT.** The plan is acceptable for implementation as-is. The two critical compatibility findings from Pass 1 are closed; the substantive correctness contradiction surfaced in Pass 3 was fixed in this same pass; remaining concerns are either accepted tradeoffs (with rationale documented inline) or small refinements that don't block implementation.

The plan's value proposition is now delivered:
- Frontmatter contracts move from skill prose into versioned template files (10 + 4 = 14 unified producers post-0066).
- The visualiser's two consumer surfaces (`target_path_from_entry`, `read_ref_keys`) are updated in the same story to accept the new typed-linkage forms.
- The transitional `work_item_id:` alias preserves backward-compat for the visualiser during the rewire window, with an explicit retirement follow-up queued under epic 0057.
- Test scaffolding extensions ensure the per-type extras are enforced by the SKILL-prose test rather than slipping through silent.

Recommend proceeding to implementation. The remaining minor items can be addressed during code review or as small follow-ups; they don't merit a fourth review pass.
