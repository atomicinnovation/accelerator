---
type: "work-item-review"
id: "0174-retire-shell-tooling-and-ci-guards-review-1"
title: "Work Item Review: Retire Shell Tooling and CI Guards"
date: "2026-08-27T23:29:38+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0174"
work_item_id: "0174"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-27T23:29:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Retire Shell Tooling and CI Guards

**Verdict:** REVISE

The story is exceptionally detailed and well-bounded — every section is densely populated, the ownership split with sibling 0171 is stated to exact counts, and each retirement is anchored to its Rust successor with file/line references. What holds it back from ready is a cluster of unresolved contradictions and unverifiable criteria, not missing content: the deferred bashisms decision leaks into the Summary and two acceptance criteria, one Rust test file is told to be both deleted and repointed, the most load-bearing change (the jira `external_id` cutover) has no verification criterion, and the 2026-08-27 widening bundled two separable production/authoring workstreams under a title that names only guard retirement. Six major findings, no critical.

### Cross-Cutting Themes

- **The deferred bashisms/thin-shell Open Question leaks into settled-looking prose** (flagged by: clarity, completeness, testability) — the unresolved decision about whether a reduced bashisms check survives makes the Summary self-contradict (retire *and* retain the linter), leaves AC-4 without a verifier, and leaves AC-5's expected `find` output undefined. This is the single most-repeated concern across lenses.
- **The 2026-08-27 scope widening outran the title and the verification surface** (flagged by: scope, testability) — relocating three production data files and porting nine guards to Python are additive, separable workstreams; the title still names only guard retirement, and the riskiest change it pulled in (the jira cutover) is unverified.
- **The jira `external_id` writeback cutover is under-specified on both dependency and verification** (flagged by: testability, dependency) — the story calls it "the most load-bearing" coupling yet names no acceptance criterion asserting the Rust path reproduces the shell behaviour, and no predecessor story is credited with delivering `sync_author.rs`.

### Findings

#### Major

- 🟡 **Clarity**: Summary both retires and retains the bashisms linter
  **Location**: Summary
  Lists "the bashisms linter" among the machinery to retire, then ends "leaving `scripts/` holding at most the bashisms linter." The conditionality only surfaces in the Open Question three headings away.

- 🟡 **Clarity**: `doc_type_single_source.rs` given two contradictory dispositions
  **Location**: Requirements (doc-type-inference deletion vs. data-file relocation)
  The doc-type-inference deletion requirement lists it among parity tests to remove; the relocation requirement lists it as the `linkage-type-pairs.tsv` consumer whose `require_file` path must be repointed — implying it survives. "Done" is undefined for this file.

- 🟡 **Clarity**: Config/hooks floor ownership stated inconsistently
  **Location**: Requirement 1 / Requirement 5 / Technical Notes (Floor ownership)
  Requirement 1 says decrements happen "in the subdomain stories"; Requirement 5 says this story's lockstep covers "the config, hooks, decisions and github floors"; Technical Notes says "the other four are this story's, alongside whichever the config and hooks clusters' own stories decrement." This is the exact double-decrement ambiguity the Drafting Notes claim to have resolved for work/integrations.

- 🟡 **Testability**: AC-4 "documented and held to bash 3.2" has no measurable verifier
  **Location**: Acceptance Criteria
  Neither clause defines a check: "documented" names no artefact, and "held to bash 3.2" has no automated verification once the bashisms linter is removed — which the Open Question explicitly weighs doing. The criterion can be claimed passed regardless of actual state.

- 🟡 **Testability**: No criterion verifies the jira `external_id` writeback cutover behaviour
  **Location**: Acceptance Criteria
  The repoint off `config_upsert_frontmatter_field` onto the Rust writeback is the sole live coupling keeping the config chain alive, yet no AC asserts it still works. `mise run` passing (AC-9) and no-dangling-references (AC-6) neither exercise the create-jira-issue path writing an `external_id`.

- 🟡 **Scope**: Retirement story bundles two separable production/authoring workstreams
  **Location**: Requirements
  Beyond guard removal, the story relocates three data files into `cli/` crates (production corpus code + drift tests) and ports nine guards into net-new Python fixtures. Neither depends on guard removal; both have independent delivery value and blast radius. The disposition buckets already isolate a clean seam if split.

#### Minor

- 🔵 **Completeness**: AC-5's end-state depends on the unresolved Open Question
  **Location**: Acceptance Criteria
  "`scripts/` holds no shell library … at most `lint-bashisms.sh` survives (per the Open Question)" defers its concrete definition of done to the thin-shell decision; acceptable for draft, close before promotion to ready.

- 🔵 **Testability**: AC-5 expected `find` output is contingent on the same Open Question
  **Location**: Acceptance Criteria
  Until the reduced-bashisms decision is recorded, a verifier cannot compute the expected `find scripts -name '*.sh'` set.

- 🔵 **Testability**: Green→red invariants (AC-1, AC-9) are not capturable by an end-state check
  **Location**: Acceptance Criteria
  "CI never goes green→red" and "no green→red gap at any step" are cross-change temporal properties; a verifier inspecting the final repo cannot distinguish a run that stayed green from one with a transient red step later repaired.

- 🔵 **Scope**: Cross-cutting cleanup sized as one story despite umbrella-like, distributed scope
  **Location**: Frontmatter: kind
  Its own Drafting Notes call it "the Phase 11 cross-cutting cleanup story" whose work "lands incrementally inside the subdomain stories." It touches build-system, CI, `cli/` crates, the Python suite, skills/, hooks/, and `.editorconfig` — a broad footprint for one story.

- 🔵 **Dependency**: Config/hooks cluster stories that own the remaining floor decrements are not enumerated
  **Location**: Technical Notes (Floor ownership)
  `_EXPECTED_CONFIG_SUITES = 15` still stands, implying config-cluster retirement is incomplete, yet Dependencies asserts "Blocked by: none remaining" and never names those sibling stories. The final frozenset/checker removal this story owns is coupled to siblings referenced only obliquely.

- 🔵 **Dependency**: Predecessor delivering the `sync_author.rs` writeback is not named
  **Location**: Requirements (live coupling cutover)
  The cutover depends on `cli/work-cli/src/sync_author.rs:145-159` reproducing `config_upsert_frontmatter_field`, but no completed predecessor (presumably 0211/0212) is credited with landing it; readiness rests on an inferred assumption.

#### Suggestions

- 🔵 **Scope**: Title names only guard retirement while scope empties `scripts/` entirely
  **Location**: Frontmatter: title
  "Retire Shell Tooling and CI Guards" understates the production-data relocation and Python-port reach the Summary now describes.

- 🔵 **Completeness**: Beneficiary of the cleanup is left implicit
  **Location**: Summary
  The story never names who benefits (build-system maintainers / the ADR-0048 target); low-risk for tech-debt work but aids triage.

- 🔵 **Clarity**: "oracle" used as undefined domain shorthand
  **Location**: Requirements / Technical Notes
  "bash oracle" / "drift-oracle tests" is inferable from context but unglossed for a contributor new to the migration's test strategy.

### Strengths

- ✅ Every expected section is present and substantively populated — Context explains the *why* (residue after cluster migration, green→red floor risk) rather than restating the Summary; Dependencies, Assumptions, and Open Questions are genuinely filled.
- ✅ The ownership boundary against sibling 0171 is stated with unusual precision — every contested floor and the eight-vs-fourteen `SHELL_LIBRARIES` split (8 + 14 = 22), so the two stories cannot double-decrement or orphan a floor.
- ✅ The disposition table classifies all 49 files into four explicit buckets with a stated basis, making the in-scope/out-of-scope line inspectable.
- ✅ Every live coupling that gates a deletion is anchored to its Rust successor with file/line references (`sync_author.rs`, `parity.rs`, `extra_keys_mirror.rs`, `schema.rs`, `cue_phrase_drift.rs`).
- ✅ Internal ordering is explicit — sever the live jira coupling *first*, then delete the `config-common.sh` source chain; lockstep floor-decrement-with-deletion stated repeatedly.
- ✅ Several ACs tie completion to observable states — grep for dangling references, `find`, `mise run` exit codes, named `include_str!`/`require_file` tests — each admitting a definitive pass/fail.
- ✅ Drafting Notes proactively surface the two most debatable scope calls (production data relocation, Python placement) and offer a fallback for each.

### Recommended Changes

1. **Resolve the thin-shell/bashisms Open Question, or make its conditionality explicit everywhere it bleeds** (addresses: bashisms Summary contradiction, AC-4, AC-5 ×2). Before promoting to ready, decide whether a reduced bashisms check survives; then restate the Summary to signal the linter is retired-in-current-form-but-may-partly-survive, pin AC-5's exact surviving file set, and give AC-4 a concrete verifier (a named doc listing the surviving shell + its bash-3.2 constraint, plus either a reduced bashisms task that exits 0 or the stated manual-review evidence that replaces it).

2. **Fix the `doc_type_single_source.rs` contradiction** (addresses: doc_type_single_source disposition clash). State once whether that test is deleted (as a doc-type-inference oracle) or retained-and-repointed (as the `linkage-type-pairs.tsv` consumer), and correct whichever requirement and AC mislabels it.

3. **Reconcile config/hooks floor ownership to one owner per floor** (addresses: floor-ownership inconsistency, unenumerated cluster stories). Align Requirement 1, Requirement 5, and the Technical Notes; name the config/hooks-cluster stories that perform incremental decrements (or state none remains and the surviving suites are permanent), and reflect that in Dependencies.

4. **Add an acceptance criterion for the jira `external_id` cutover behaviour** (addresses: unverified cutover, unnamed predecessor). Assert the repointed SKILL.md writes the `external_id` frontmatter equivalently to the retired path, verified by a named test or manual check; and credit the predecessor that delivered `sync_author.rs`.

5. **Confirm the scope bundle is deliberate, and align the title** (addresses: bundled workstreams, umbrella sizing, title mismatch). Either split the data-file relocation and nine-guard Python port into follow-on stories (the disposition buckets already give a clean seam), or explicitly confirm co-delivery and broaden the title to reflect emptying `scripts/`.

6. **Reframe the green→red invariants as per-change obligations** (addresses: temporal-invariant untestability). State that each commit removing scripts or decrementing a floor is independently verified to exit 0 on `mise run` before the next, giving the invariant a defined check.

## Per-Lens Results

### Clarity

**Summary**: Unusually detailed and, on scope, internally coherent — Summary intent matches Requirements and ACs, and the thin-shell survivor set is defined consistently throughout. Weaknesses are localised contradictions: the bashisms linter is both retired and retained, `doc_type_single_source.rs` is both removed and repointed, and config/hooks floor ownership is stated inconsistently across Requirement 1, Requirement 5, and Technical Notes. Jargon is well-controlled.

**Strengths**:
- The surviving thin-shell set (bootstrap, hook wrapper, Playwright executor) has one referent across Context, Requirements, Assumptions, and ACs.
- The ADR split is consistent: ADR-0048 thin-wrapper floor, ADR-0049 bash-3.2/bashisms floor.
- The 0171-vs-this-story ownership boundary is stated with exact counts (8 + 14 = 22).
- The Summary's stated scope (retire guards AND empty scripts/) matches the four-disposition breakdown.

**Findings**:
- 🟡 major (medium): Config/hooks floor ownership inconsistent across Requirement 1, Requirement 5, and Technical Notes — the exact double-decrement/missed-floor ambiguity the Drafting Notes claim resolved for work/integrations, reintroduced for config/hooks.
- 🟡 major (medium): `doc_type_single_source.rs` given contradictory dispositions — listed both among parity tests to remove and as the `linkage-type-pairs.tsv` consumer to repoint. ACs inherit the clash ("consuming tests pass" vs. "drift-oracle tests removed").
- 🟡 major (medium): Summary both retires and retains the bashisms linter in one passage; conditionality only appears in the Open Question three headings away.
- 🔵 suggestion (low): "oracle" ("bash oracle", "drift-oracle tests") used as undefined domain shorthand; inferable but unglossed for a new contributor.

### Completeness

**Summary**: Exceptionally complete — every expected section present and densely populated, frontmatter fully specified with `kind: story` and `status: draft`. Eight ACs, detailed requirements, and a per-file disposition table give an implementer everything needed. Only minor observations: the implicit beneficiary and one AC whose final state is contingent on an unresolved Open Question.

**Strengths**:
- All expected story sections present and substantively populated; Context explains the *why* rather than restating the Summary.
- Eight specific, distinct acceptance criteria covering lockstep decrements, checker removal, CI-job removal, reference sweeps, ported guards, relocated data files.
- Frontmatter complete and internally consistent.
- Technical Notes carries the disposition table and live-coupling anchors; Drafting Notes records scope changes and 0171 ownership boundaries.
- Dependencies, Assumptions, Open Questions all genuinely populated.

**Findings**:
- 🔵 minor (medium): AC-5 defers its concrete end-state to the first Open Question (thin-shell bashisms check); acceptable for draft, should close before ready.
- 🔵 suggestion (low): Beneficiary (build-system maintainers / ADR-0048 target) left implicit; naming it makes the story's value legible at triage.

### Dependency

**Summary**: Unusually well dependency-mapped — predecessors enumerated and confirmed done, the ownership boundary with 0171 stated for every contested floor and `SHELL_LIBRARIES` entry, internal ordering (sever the live bash coupling before deleting `config-common.sh`) called out, and each Rust successor named with anchors. The main residual gap is an obliquely-referenced coupling to the config/hooks clusters' own floor-decrementing stories, not enumerated in Dependencies.

**Strengths**:
- Upstream blockers fully captured and reconciled: every predecessor (0167–0172, 0195–0197, 0211, 0212) listed, each reached done on 2026-08-28.
- Internal ordering explicit — sever the jira SKILL.md coupling first (repoint to `sync_author.rs`) before deleting the config source chain.
- The 0171 ownership split disambiguated precisely.
- Every deletion-gating live coupling anchored to its Rust successor with file/line references.

**Findings**:
- 🔵 minor (medium): Config/hooks cluster stories that own the remaining floor decrements are never named, yet Dependencies asserts "Blocked by: none remaining"; `_EXPECTED_CONFIG_SUITES = 15` still standing implies config retirement is incomplete.
- 🔵 minor (medium): The predecessor delivering the `sync_author.rs:145-159` `external_id` writeback is not named; the cutover's readiness rests on an inferred assumption it landed with 0211/0212.

### Scope

**Summary**: Anchored by a single coherent theme — retire the shell-policing machinery as the scripts it guards disappear — and exemplary in stating boundaries against sibling 0171. But the 2026-08-27 widening pulled two substantial, separable workstreams under the same story: relocating three production data files into `cli/` crates and porting nine authoring/evals guards to Python. Both have independent delivery value and stretch the story beyond its "retire tooling and CI guards" framing.

**Strengths**:
- Boundaries against 0171 stated with unusual precision — floor ownership, the 8-vs-14 `SHELL_LIBRARIES` split, and the reason each item belongs where it does.
- The disposition table classifies all 49 files into four explicit buckets with a stated basis.
- The lockstep constraint gives the story a clear, self-contained delivery invariant.
- Drafting Notes proactively surface the two most debatable scope calls and offer a fallback for each.

**Findings**:
- 🟡 major (medium): The story bundles guard retirement, a production data-relocation, and a build-new-Python-tests effort — three independently deliverable/reviewable/rollbackable workstreams. Consider splitting into a retirement story plus follow-ons; at minimum confirm co-delivery is deliberate.
- 🔵 minor (medium): Sized as one story despite umbrella-like, distributed scope touching build-system, CI, `cli/` crates, the Python suite, skills/, hooks/, `.editorconfig`; weigh whether the collection is closer to a small epic.
- 🔵 suggestion (medium): Title names only guard retirement while scope empties `scripts/` entirely; broaden it or retitle the narrower retirement story if split.

### Testability

**Summary**: Most ACs are concrete and mechanically verifiable — grep for dangling references, `find scripts -name '*.sh'`, `mise run` exit codes, and the enumerated nine ported guards give definitive pass/fail procedures. The main gaps: AC-4's unmeasurable "documented and held to bash 3.2" (whose verifier the Open Question may itself delete), the absence of any criterion verifying the load-bearing jira `external_id` cutover, and a criterion whose expected output is contingent on an unresolved Open Question.

**Strengths**:
- AC-6 (repo-wide grep for each removed path resolves only to surviving/relocated locations) defines an exact mechanical procedure.
- AC-7 enumerates the nine ported guards by name and requires behavioural parity — a bounded, testable success condition.
- AC-2, AC-3, AC-8, AC-9 tie completion to observable states — files removed, `mise run check` / bare `mise run` exit 0, specific tests passing.

**Findings**:
- 🟡 major (high): AC-4 "documented and held to bash 3.2" defines no verification procedure; the only mechanism that could confirm it (bashisms linter) is what the Open Question weighs deleting. Split into two testable criteria.
- 🟡 major (medium): No AC asserts the jira `external_id` writeback cutover — the riskiest behavioural change — still works; `mise run` passing does not prove the Rust path produces equivalent frontmatter.
- 🔵 minor (medium): AC-5's expected `find` output is contingent on the first Open Question, so its pass/fail set is undefined until resolved.
- 🔵 minor (medium): AC-1 and AC-9 are cross-change temporal invariants not capturable by an end-state inspection; reframe as per-commit obligations with a defined check.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-27

**Verdict:** APPROVE

All six majors from pass 1 were addressed by edits, and the second-pass sweep of the same five lenses surfaced a fresh batch of refinements — all of which have now also been applied. The work item is ready; the only outstanding item is the author's pre-promotion resolution of the bashisms Open Question, which the item itself now flags as the promotion gate.

### Previously Identified Issues

- 🟡 **Clarity**: Summary retires and retains the bashisms linter — Resolved (conditionality made explicit; retired "in its current form", reduced check deferred to the bashisms Open Question).
- 🟡 **Clarity**: `doc_type_single_source.rs` contradictory disposition — Resolved (rescoped: keep the bash-free resolver case, drop the two bash oracles; anchors corrected).
- 🟡 **Clarity**: Config/hooks floor ownership inconsistent — Resolved (Requirement 1 and Technical Notes both pin the four floors to this story outright).
- 🟡 **Testability**: AC-4 "documented and held to bash 3.2" unverifiable — Resolved (split into an enumeration criterion in `tasks/README.md` plus a bash-3.2 verifier with a defined check on each branch).
- 🟡 **Testability**: No criterion for the Jira `external_id` cutover — Resolved (dedicated AC added, now covering insert and update cases against the local writeback path only).
- 🟡 **Scope**: Bundles two separable workstreams — Resolved as accepted (co-delivery recorded as a deliberate choice; title broadened; disposition buckets left as a clean seam).
- 🔵 Completeness (beneficiary), Completeness/Testability (AC-5 contingency), Dependency (unnamed siblings, unnamed cutover predecessor), Testability (green→red temporal invariants) — all Resolved.

### New Issues Introduced

- 🟡 **Clarity**: "the Open Question" singular was ambiguous once two exist — Introduced by the pass-1 edits; fixed (named "the bashisms Open Question" throughout; both Open Questions labelled with their promotion-gating status).
- 🟡 **Testability**: nine-guard AC asserted "same violations" with no reference corpus — Pre-existing gap; fixed (AC now requires a captured golden fixture set per guard, verified fail-on-violation / pass-on-conforming).
- 🔵 **Testability**: manual-review branch unbounded; Jira cutover single happy-path — Fixed (denylist checklist specified for the manual branch; cutover AC extended to insert + update).
- 🔵 **Dependency**: `check-scripts` CI-graph edges beyond the release gate; Jira-API coupling in the cutover check — Fixed (all `needs: check-scripts` edges enumerated; cutover scoped to the local writeback, no live Jira call).
- 🔵 **Clarity**: Drafting-Notes AC mislabel and a stale "three data files" count — Fixed (referenced by text; count annotated as superseded).
- 🔵 **Scope**: nine-guard port is the most separable strand; large cross-cutting capstone — Accepted, no change (co-delivery deliberate; the per-commit lockstep ACs are the delivery plan; the "Port to Python" bucket remains the seam should a split later prove warranted).

### Assessment

The work item is ready for implementation. Every actionable finding across both passes is resolved; the only residual observations are the two scope notes, explicitly accepted as deliberate co-delivery. One author action remains before promoting from `draft` to `ready`: resolve the bashisms Open Question so the surviving `scripts/*.sh` set and the thin-shell verifier are pinned exactly — a step the item now names as the promotion gate.

---
*Re-review generated by /accelerator:review-work-item*

## Re-Review (Pass 3) — 2026-08-27

**Verdict:** APPROVE

Run after the bashisms Open Question was resolved (denylist re-homed to a Python `tasks/` task; shfmt + ShellCheck retained and rescoped to the surviving thin shell). Completeness came back clean apart from one suggestion; the resolution introduced a fresh gap that both completeness and testability caught, plus a scatter of clarity/dependency refinements. All actionable findings have been applied; the only remaining observations are the standing scope notes, still accepted as deliberate co-delivery.

### Previously Identified Issues

- The pass-1/pass-2 findings all remained resolved; the pass-3 sweep confirmed the bashisms resolution is threaded consistently through Summary, Context, Requirements, ACs, and Technical Notes with no stale contradictions.

### New Issues Introduced

- 🟡 **Testability / Completeness**: the re-homed Python bashisms task and rescoped shfmt/ShellCheck were verified only by *exiting 0* — a guard on the wrong files or a denylist missing `${var,,}` would pass tautologically. Fixed: a new AC fixture-verifies the Python bashisms task against the corpus the retired `lint-bashisms.sh` flagged, and requires shfmt/ShellCheck to fail on a deliberately malformed survivor — proving the guards can fail, not just exit 0.
- 🔵 **Clarity / Testability**: "nine ported guards" conflicted with `test-evals-structure-self` folding in. Fixed: stated as **eight** standalone pytest guards, the `-self` meta-test's assertions verified through the two guards it folds into; the port requirement, the parity AC, and the Technical-Notes coverage note all aligned.
- 🔵 **Clarity**: the "Empty scripts/" title vs AC "(if any)" hedge. Fixed: confirmed no thin-shell survivor lives under `scripts/` (the launcher/hook wrapper is `hooks/launcher-link-refresh.sh`, the Playwright executor is in `cli/`), so `find scripts -name '*.sh'` returns nothing and the survivors are homed outside `scripts/`.
- 🔵 **Dependency**: a stale Technical-Notes line said the fourteen `SHELL_LIBRARIES` entries "retire under their own subdomain stories", contradicting the Requirement that this story removes them. Fixed: this story removes them directly (every owning cluster is done).
- 🔵 **Clarity / Testability**: `decisions`/`github` floors already stand at 0 (removed, not decremented); the guarded file list had no single source of truth; the `cli/` Rescope row sat in a `scripts/`-framed table. All fixed (floor verbs distinguished; the scanned list asserted equal to the `tasks/README.md` enumeration; the Rescope row annotated as a downstream consumer, not one of the 49).
- 🔵 **Scope**: nine-guard port separability and large-story sizing — Accepted, no change (deliberate co-delivery; the disposition buckets remain the decomposition seam).

### Assessment

The work item is ready for implementation, with no open question gating promotion. Every actionable finding across three passes is resolved; the residual observations are the two scope notes, explicitly accepted. The pass-3 fixes hardened the one behaviour most at risk in the resolution — the retained bash-3.2 guard now has a negative/parity criterion rather than an exit-0-only check. The `status` field is unchanged; the draft→ready transition remains a separate decision.

---
*Re-review generated by /accelerator:review-work-item*
