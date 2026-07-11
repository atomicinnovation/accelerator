---
type: work-item-review
id: "0179-corpus-crates-parsing-conventions-review-1"
title: "Work Item Review: corpus and corpus-adapters Crates for Parsing and Conventions"
date: "2026-07-11T10:24:15+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0179"
work_item_id: "0179"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: [rust, corpus, crates]
last_updated: "2026-07-11T11:10:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: corpus and corpus-adapters Crates for Parsing and Conventions

**Verdict:** REVISE

This is an exceptionally thorough, well-researched work item: every section is
densely populated, code-symbol references carry file:line anchors, and the
ownership boundaries against sibling work items (0168, 0170, 0173, 0180) are
drawn with unusual precision. The REVISE verdict is not about missing content —
it is about a small number of high-value structural issues: several design
decisions are asserted as settled in the Requirements/Acceptance Criteria while
still listed as open (the config-adapters retrofit most acutely), the unit
bundles at least three separately-deliverable threads under one `task`, and two
named downstream consumers plus a same-code coordination coupling are recorded
only as "relates to" rather than as blocking/ordering dependencies. None is a
correctness defect; all are resolvable with targeted edits.

### Cross-Cutting Themes

- **Settled-vs-open decision inconsistency, centred on the config-adapters retrofit** (flagged by: clarity, completeness, scope) — The Requirements and Acceptance Criteria present three decisions as final — YAML-tag handling "per the resolved Open Question", the config-adapters retrofit ("no longer parses YAML independently", AC-3), and big-int-as-`String` — yet all three still appear in Open Questions. A reader cannot tell which behaviours are locked. If the retrofit is later deferred, AC-3 becomes unsatisfiable.
- **Scope breadth / decomposition of an L-sized task** (flagged by: scope, dependency, testability) — The item self-describes as "L, the heaviest of the three siblings" and carries orthogonal threads: the corpus/corpus-adapters crates, the new fifth document-format crate plus a retrofit of already-shipped 0178 code, and a "greenfield-ish" artifact-metadata family (three scripts, two VCS ports, an embedded possible-spike) that shares no parsing semantics with the rest. The 0168 ordering gap and the undefined parity-fixture corpus reinforce that the unit is carrying more than one atomically-deliverable increment.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Decisions stated as settled in Requirements/AC are still listed as open in Open Questions
  **Location**: Requirements / Acceptance Criteria / Open Questions
  Several decisions are presented as made (YAML tags "per the resolved Open Question"; retrofit "per the resolved Open Question"; big-int-as-`String`; AC-3 "config-adapters no longer parses YAML independently") yet reappear as unresolved in Open Questions with only a "lean". The phrase "the resolved Open Question" reads as already-decided but points into a still-open list. (confidence: high)

- 🟡 **Scope**: Retrofit of already-shipped 0178 config-adapters bundled into a corpus-build task
  **Location**: Requirements: Shared document-format crate
  Corpus only needs a frontmatter parser for itself; migrating shipped, working 0178 code to share it (AC-3) is a separately-deliverable, independently-rollbackable refactor. The item's own Open Question notes this "governs 0179's blast radius into shipped code." (confidence: medium)

- 🟡 **Scope**: L-sized task bundles at least three separable delivery threads
  **Location**: Technical Notes: Size / Requirements: Artifact-metadata derivation
  (a) corpus/corpus-adapters crates, (b) the new document-format crate + config-adapters retrofit, and (c) the artifact-metadata family (three scripts across two skill dirs, two VCS ports, a possible spike). Thread (c) shares no parsing/convention semantics with the rest — only a home in `corpus-adapters`. (confidence: medium)

- 🟡 **Dependency**: 0170 and 0173 are named as consumers of these crates but captured only as "relates to", not Blocks
  **Location**: Dependencies
  Context/Dependencies describe 0170 (`accelerator-work`) as "built on top of corpus" and 0173 (`accelerator-corpus` CLI) as consuming "these libraries", yet both sit under "Relates to" while 0180 — which also builds on corpus-adapters — is correctly a Blocks entry. A consumed crate is a prerequisite. (confidence: medium)

- 🟡 **Dependency**: Same-code overlap with 0168 (visualiser fold) has no captured ordering constraint
  **Location**: Dependencies
  Both 0168 and 0179 mutate the same visualiser source and both move code into `cli/` (the slug parity harness path "breaks when the crate moves into `cli/`"). Recorded only as "Relates to" with no statement of which lands first — they can proceed in either order and collide. (confidence: medium)

- 🟡 **Testability**: Parity criteria depend on an undefined "shared corpus of fixtures"
  **Location**: Acceptance Criteria
  The doc-type/linkage/slug parity criterion and the work-item-ID criterion both verify "at parity with the bash sources ... asserted against a shared corpus of fixtures", but that corpus is never enumerated or given a coverage bar — parity can be claimed passed against a trivial fixture set while real behaviour diverges. (confidence: medium)

#### Minor

- 🔵 **Completeness**: Retrofit-scope acceptance criterion presupposes an unresolved Open Question
  **Location**: Acceptance Criteria
  AC-3 asserts the 0178 retrofit is in scope ("config-adapters no longer parses YAML independently") while the retrofit-scope Open Question explicitly leaves open whether it happens now or becomes a follow-on. The definition of "done" is contingent on a pending decision. (confidence: medium)

- 🔵 **Clarity**: Parity source `linkage-parser.sh` named only in the criteria, not in the matching requirement
  **Location**: Acceptance Criteria
  AC-5 cites `linkage-parser.sh` as a parity source, but the Typed-linkage requirement and Technical Notes describe the extraction only via the Rust `typed_ref` primitive and ADR-0034; `linkage-parser.sh` appears nowhere else. The reader must infer it is the bash ground truth. (confidence: medium)

- 🔵 **Testability**: "Within a bounded time" has no threshold and the adversarial input set is not enumerated
  **Location**: Acceptance Criteria
  The adversarial-frontmatter criterion requires a clean result "within a bounded time — no panic, no abort, no hang", but names no numeric bound and references inputs only as "the adversarial inputs that panic libyml today" (one example, in Technical Notes). "No hang" cannot yield a definitive pass/fail without a timeout. (confidence: medium)

#### Suggestions

- 🔵 **Dependency**: Potential VCS-technique spike is an implied upstream investigation, not a captured dependency
  **Location**: Open Questions
  The VCS-kind vs repo-root Open Question notes reconciling marker-walk vs command-probe "may warrant a small spike", which would gate the artifact-metadata VCS ports' design — but that investigation is captured neither as scope nor as a blocker. (confidence: low)

- 🔵 **Scope**: Task kind sits at the top of the size band for its declared scope
  **Location**: Frontmatter: kind
  Filed as `kind: task` yet described as an L-sized, multi-crate consolidating rewrite, the heaviest of three sibling tasks under story 0166. If not decomposed, `story` (with the threads as child tasks) may better reflect the granularity. (confidence: low)

- 🔵 **Testability**: Title-caser consolidation criterion omits the "where they share semantics" caveat
  **Location**: Acceptance Criteria
  The final AC asserts the title-casers "are each implemented once and reused", but Requirements qualify this as collapsing them "where they share semantics". A legitimately-divergent caser left separate would be correct yet fail a literal reading. (confidence: low)

### Strengths

- ✅ Every standard section is present and substantively populated — no empty, placeholder, or stub sections; Dependencies, Assumptions, and Open Questions are genuinely filled, and Acceptance Criteria carries eight specific, individually-scoped bullets.
- ✅ Code-symbol referents (crate names, module/function names like `fence_offsets`, `typed_ref`, `discover_root`) are precise, consistent, and anchored with file:line references, leaving little room for misinterpretation.
- ✅ Sibling ownership boundaries (validation → 0173, visualiser fold → 0168, ID-pattern DSL compiler → 0170, store primitives → 0180) are explicitly demarcated, so "who owns what" is unambiguous even where the same code is touched by multiple tasks.
- ✅ Out-of-scope carve-outs are deliberate and defended (the pattern DSL compiler, the server-coupled cluster walker, the Config-coupled `DocType` projection, corpus-frontmatter validation), showing the author actively bounded the unit.
- ✅ Most acceptance criteria are tied to objective, tool-enforced checks (cargo-deny/cargo-pup, the deny.toml wrapper) or concrete observable outcomes (byte-for-byte round-trip, big-int-as-`String`, faked-port determinism) rather than subjective inspection.
- ✅ The upstream dependency on 0178 is captured along with its de-risking effect (serde-saphyr proven, pinned `=0.0.29`, cargo-deny wrapper in place), and the cross-cutting expansions are surfaced in Open Questions/Drafting Notes rather than smuggled in.

### Recommended Changes

1. **Reconcile the settled-vs-open decisions** (addresses: "Decisions stated as settled...", "Retrofit-scope acceptance criterion presupposes...") — For each of the YAML-tag policy, the config-adapters retrofit scope, and the big-int/number policy: either record it as a decision (remove from Open Questions, keep the firm Requirement/AC) or keep it open (soften the Requirement/AC to conditional language). Replace every "per the resolved Open Question" with an explicit cross-reference to the named Open Question.

2. **Decide the retrofit and the decomposition** (addresses: "Retrofit of already-shipped 0178...", "L-sized task bundles...") — Resolve whether the config-adapters retrofit and the artifact-metadata thread stay in 0179 or carve out to sibling task(s), mirroring how 0166 was split into 0178/0179/0180. If they stay, explicitly ratify the widened scope and update the 0166 epic (which planned four crates, not five). Resolve or spin out the VCS-technique spike before implementation.

3. **Promote the downstream and coordination dependencies** (addresses: "0170 and 0173 ... only 'relates to'", "Same-code overlap with 0168 ...", "Potential VCS-technique spike ...") — Move 0170 and 0173 to Blocks (or state the soft coupling explicitly if they can proceed against stubs); add an ordering note resolving the 0168 sequence; capture the VCS spike as in-scope investigation or a blocker of the artifact-metadata portion.

4. **Define the parity fixture corpus and the time bound** (addresses: "Parity criteria depend on an undefined 'shared corpus of fixtures'", "'Within a bounded time' has no threshold ...") — State the minimum coverage the fixture corpus must span (each of the 14 `DocTypeKey` variants, all three identity schemes, the optional-embedded-work-item-id cases) or mandate reuse of the existing bash suites as the baseline; name a concrete per-fixture time bound (or cite the 0178 plan's floor value).

5. **Minor polish** (addresses: "Parity source `linkage-parser.sh` ...", "Title-caser consolidation criterion ...", "Task kind ...") — Name `linkage-parser.sh` (with a path) in the Typed-linkage requirement; reword the title-caser AC to "the title-casers that share semantics are collapsed into one helper; any intentional divergence is documented"; reconsider `kind: task` vs `story` if the item is not decomposed.

## Per-Lens Results

### Clarity

**Summary**: Unusually thorough and, for its density, largely unambiguous: code-symbol references are precise and consistent, and ownership boundaries are explicitly drawn. The main weakness is an internal inconsistency between the Requirements/Acceptance Criteria — which assert several decisions as settled via "per the resolved Open Question" — and the Open Questions section, which lists those same decisions as still open. Jargon is dense but consistent with the project's Rust/hexagonal vocabulary.

**Strengths**:
- Code-symbol referents used precisely and consistently, with file:line anchors.
- Acceptance Criteria state outcomes as concrete, observable system states rather than vague properties.
- Scope boundaries against sibling work items are explicitly demarcated.

**Findings**:
- 🟡 major (high): Decisions stated as settled in Requirements/AC are still listed as open in Open Questions — Requirements / Acceptance Criteria / Open Questions. YAML-tags "per the resolved Open Question", retrofit "per the resolved Open Question", big-int-as-`String`, and AC-3 all assert decisions that Open Questions still lists as open; "the resolved Open Question" reads as decided but points into a still-open list. If the retrofit is later deferred, AC-3 becomes unsatisfiable.
- 🔵 minor (medium): Parity source `linkage-parser.sh` named only in the criteria, not in the matching requirement — Acceptance Criteria. AC-5 cites it as a parity source, but it appears nowhere in the Typed-linkage requirement or Technical Notes, forcing the reader to infer it is the bash ground truth.

### Completeness

**Summary**: A highly complete task-kind work item: every expected section is present and substantively populated, and the frontmatter carries a recognised kind, status, and priority. For a task it over-delivers with a crate-topology breakdown, per-convention extraction plan, and eight acceptance criteria. The only completeness-adjacent gap is the tension between AC-3 and the unresolved retrofit-scope Open Question.

**Strengths**:
- All standard sections present and densely populated — no empty/placeholder/stub sections.
- Frontmatter integrity complete: kind, status, priority, parent, blocks, relates_to all present and recognised.
- Acceptance Criteria contains eight specific bullets mapping cleanly onto the requirements.
- Context explains motivation rather than restating the Summary; Requirements are detailed enough to begin without follow-up.
- Open Questions surface the genuinely undecided points rather than leaving them implicit.

**Findings**:
- 🔵 minor (medium): Retrofit-scope acceptance criterion presupposes an unresolved Open Question — Acceptance Criteria. AC-3 asserts the retrofit is in scope while the retrofit-scope Open Question leaves open whether it happens now or later; "done" is contingent on a pending decision.

### Dependency

**Summary**: Captures its upstream blockers well (0166 parent, 0178 done and de-risking the parser) and cleanly models 0180 as a downstream Blocks entry. Strong dependency work carving out the pattern-DSL compiler as out-of-scope to avoid inverting the layering, and injecting the compiled regex. The main gaps are downstream: two named consumers (0170, 0173) and a same-code coupling (0168) are recorded only as "relates to".

**Strengths**:
- 0180 correctly captured as Blocks in both frontmatter and Dependencies with rationale.
- Upstream dependency on 0178 captured with its de-risking effect; 0166 captured as parent.
- Pattern DSL compiler explicitly carved out to 0170/0167 with a layering argument; compiled regex injected to avoid a hard blocker.
- The new document-format crate's cross-crate coupling and the 0178 retrofit blast radius are surfaced and flagged back to 0166.

**Findings**:
- 🟡 major (medium): 0170 and 0173 named as consumers but captured only as "relates to", not Blocks — Dependencies.
- 🟡 major (medium): Same-code overlap with 0168 (visualiser fold) has no captured ordering constraint — Dependencies.
- 🔵 suggestion (low): Potential VCS-technique spike is an implied upstream investigation, not a captured dependency — Open Questions.

### Scope

**Summary**: A broadly coherent theme with unusually clear boundaries against siblings, but a self-described "L, heaviest of the three siblings" unit that bundles several separately-deliverable threads under one task kind: the corpus crates proper, a new fifth document-format crate plus a retrofit of already-shipped 0178 code, and a greenfield-ish artifact-metadata family spanning three scripts and two VCS concerns that may itself warrant a spike. Expansions are commendably surfaced as Open Questions/Drafting Notes; the retrofit and artifact-metadata family are the clearest carve-out candidates.

**Strengths**:
- Sibling boundaries stated with unusual precision (validation → 0173, fold → 0168, DSL compiler → 0170, store → 0180).
- Out-of-scope carve-outs deliberate and defended.
- Cross-cutting expansions surfaced explicitly rather than smuggled in.

**Findings**:
- 🟡 major (medium): Retrofit of already-shipped 0178 config-adapters bundled into a corpus-build task — Requirements: Shared document-format crate.
- 🟡 major (medium): L-sized task bundles at least three separable delivery threads — Technical Notes: Size / Requirements: Artifact-metadata derivation.
- 🔵 suggestion (low): Task kind sits at the top of the size band for its declared scope — Frontmatter: kind.

### Testability

**Summary**: Unusually strong on verifiability: most acceptance criteria are tied to objective, tool-enforced checks or concrete observable outcomes. The main weakness is that the behavioural-parity criteria rest on "a shared corpus of fixtures" that is never defined or given a coverage bar, so parity can be claimed passed against a trivial fixture set. A secondary gap is the undefined numeric bound behind the "bounded time" adversarial-input criterion.

**Strengths**:
- Criteria 1 and 3 enforced by tooling (cargo-deny, cargo-pup, the deny.toml wrapper) — unambiguous automated pass/fail.
- Artifact-metadata criterion puts clock/VCS-kind/repo-root behind faked ports so every derived field is asserted deterministically.
- Round-trip criterion specifies a concrete observable ("preserves the body byte-for-byte"); big-int-as-`String` stated as a definite outcome.
- Frontmatter write-convention criterion enumerates the exact properties to preserve and mandates fixture matching.
- Parity scoped to explicitly named bash sources; "no triplication"/"implemented once" are inspectable claims.

**Findings**:
- 🟡 major (medium): Parity criteria depend on an undefined "shared corpus of fixtures" — Acceptance Criteria.
- 🔵 minor (medium): "Within a bounded time" has no threshold and the adversarial input set is not enumerated — Acceptance Criteria.
- 🔵 suggestion (low): Title-caser consolidation criterion omits the "where they share semantics" caveat — Acceptance Criteria.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-07-11T11:03:21+00:00

**Verdict:** REVISE

All five lenses were re-run against the edited work item. Every one of the six
original majors is resolved (or resolved-by-ratified-decision); the item is
markedly tighter. The verdict stays REVISE only because the edits surfaced two
new, cheap-to-fix wording/criterion defects, and the scope-sizing concern
persists as a now-explicit "confirm `task` vs `story`" question that the author's
keep-in-0179 decision answers but the lens still flags. None is structural.

### Previously Identified Issues

- 🟡 **Clarity**: Decisions stated as settled but still open — **Resolved.** The
  "per the resolved Open Question" hedges are gone; YAML-tags, retrofit, and the
  number policy are traced coherently across Requirements, Open Questions, and
  Drafting Notes with no residual contradiction.
- 🟡 **Scope**: Retrofit of shipped 0178 bundled in — **Resolved (by ratified
  decision).** The in-scope choice is now expressed consistently across Summary,
  Context, Requirements, AC, and Drafting Notes.
- 🟡 **Scope**: L-task bundles ≥3 separable threads — **Still present.** The
  author's keep-in-0179 / keep-as-`task` decision is documented, but the lens
  reiterates the sizing risk (now aggravated by the embedded VCS spike) and asks
  for explicit confirmation of `task` vs a `story` with child tasks.
- 🟡 **Dependency**: 0170/0173 only "relates to" — **Resolved.** Both are now
  Blocks entries with rationale, mirrored in frontmatter.
- 🟡 **Dependency**: 0168 ordering uncaptured — **Partially resolved.** The
  "0179 first" sequencing is now stated in prose, but the structured edge is
  still `relates_to`, not a `blocks`/ordering edge.
- 🟡 **Testability**: Undefined "shared corpus of fixtures" — **Resolved.** AC-5
  enumerates the coverage bar (14 `DocTypeKey` variants, three identity schemes,
  embedded-id cases); AC-6 reuses the `work-item-pattern.sh` test suite as the
  parity baseline.
- 🔵 Minor/suggestion carryovers: `linkage-parser.sh` naming — **Resolved**;
  AC-3 contingency — **Resolved**; bounded-time threshold — **Partially resolved**
  (per-fixture bound named, fixture set still open-ended); VCS spike capture —
  **Resolved**; title-caser wording — **Partially resolved** (caveat added, but
  which casers merge is still judgement-based).

### New Issues Introduced

- 🟡 **Clarity** (major): AC-5 says "The `corpus` crate parses frontmatter…",
  which contradicts AC-1 / the crate topology (the `corpus` domain crate imports
  no YAML library; parsing lives in `corpus-adapters`). Reword to name
  `corpus-adapters` (or "the corpus crate pair").
- 🟡 **Testability** (major): the in-scope VCS-kind-vs-repo-root investigation
  has no verifiable exit criterion — working ports could ship while the
  colocated/bare/secondary-workspace divergence stays unresolved. Add a
  criterion: chosen technique per port recorded, plus fixtures for a secondary jj
  workspace, a colocated repo, and a bare repo. (Completeness flagged the same as
  a suggestion.)
- 🔵 **Scope** (minor, high conf): parent 0166 still enumerates four crates and
  no document-format crate — the "must update 0166" action is outstanding, not
  done, so the parent/child decomposition is currently inconsistent.
- 🔵 **Clarity** (minor): "Converge … onto the crate" in the Doc-type requirement
  doesn't name which crate (should be `corpus`).
- 🔵 **Testability** (minor): the adversarial fixture set is still an open-ended
  category ("the inputs that panic libyml today") with one named example.
- 🔵 **Dependency** (minor): the hard 0179→0168 ordering is prose-only, not a
  structured `blocks` edge.
- 🔵 Low-confidence polish: ADR-0053 lacks a descriptor; "improve on both" has a
  loose referent; 0167 is named as DSL-compiler co-home but absent from
  Dependencies; "no triplication" and the title-caser collapse lack defined
  verification methods.

### Assessment

The work item is close to ready. The two new majors are pure wording/criterion
fixes (AC-5 crate attribution; a verifiable exit criterion for the VCS spike),
not design problems. The remaining scope major is a ratified author decision that
only needs an explicit confirmation recorded. The one action that reaches outside
this file — updating the parent 0166 to five crates — is a real outstanding
follow-up. With those addressed, the next pass should be clean.

## Re-Review (Pass 3) — 2026-07-11T11:10:04+00:00

**Verdict:** APPROVE

Targeted verification of the two lenses that raised new majors in pass 2
(clarity, testability), after the pass-2 fixes were applied.

### Previously Identified Issues

- 🟡 **Clarity**: AC attributed frontmatter parsing to the `corpus` domain crate
  — **Resolved.** The criterion now reads "`corpus-adapters` parses frontmatter …
  over the `corpus` domain types", consistent with the crate topology; the
  Doc-type requirement and its Technical Notes echo now both name `corpus`.
- 🟡 **Testability**: VCS-technique investigation had no verifiable exit criterion
  — **Resolved.** A dedicated criterion now requires each port's chosen technique
  recorded plus fixtures for a secondary jj workspace, a colocated repo, and a
  bare repo.
- 🔵 **Testability**: adversarial fixture set open-ended — **Resolved.** AC-4 pins
  the set to the 0178 plan's adversarial fixtures plus the visualiser regression,
  and now references the plan's bounded-time guard rather than an implied numeric
  floor.
- 🔵 **Scope**: parent 0166 stale at four crates — **Resolved.** 0166 updated to
  five crates (Summary, Requirements, Size, Drafting Notes) with 0179 owning the
  retrofit.
- 🟡 **Scope**: L-task sizing / `task` vs `story` — **Resolved (by ratified
  decision).** Confirmed as one `task`; recorded in Drafting Notes.
- 🔵 **Dependency**: 0179→0168 ordering prose-only — **Resolved.** 0168 promoted
  to a `blocks` edge with the "0179 first" ordering; a "no dependency on 0167"
  note added.

### New Issues Introduced

- None of major or minor severity. Pass 3 surfaced two low-value suggestions
  (AC-5 bundling several checks; AC-4 "floor value" wording), both applied during
  this pass: AC-5 was split into a parity criterion and a single-source
  criterion (naming the comparison mechanism), and AC-4 now references the
  bounded-time guard directly.

### Assessment

The work item is ready for implementation. All original and pass-2 findings are
resolved or resolved-by-ratified-decision, the two verification lenses confirm
the fixes land clean, and the parent 0166 epic is now consistent with the
five-crate scope. Remaining open items are intentional and captured: the crate
naming and the VCS-technique choice are deferred to the plan as scoped
investigations, not gaps.

---
*Re-review generated by /accelerator:review-work-item*
