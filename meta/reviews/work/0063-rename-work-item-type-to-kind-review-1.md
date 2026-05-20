---
date: "2026-05-20T21:08:04Z"
type: work-item-review
skill: review-work-item
target: "meta/work/0063-rename-work-item-type-to-kind.md"
work_item_id: "0063"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 4
status: complete
---

## Work Item Review: 0063 Rename work-item `type:` Field to `kind:`

**Verdict:** REVISE

This work item is structurally strong — sections are complete, scope is genuinely atomic and well-justified, and acceptance criteria use concrete grep patterns with explicit false-positive exclusions. The reasons to revise are coordination and verifiability gaps: the relationship with story 0070 (which still claims to perform the same rename), the migration framework prerequisite, and internal sequencing between producer-code and migration execution are under-specified; and several acceptance criteria leave subtle migration branches (userspace `paths.work`, partial-prior-run, body-label updates, eval pass threshold) without a defined verification procedure.

### Cross-Cutting Themes

- **Migration responsibility split with 0070** (flagged by: dependency, clarity, scope) — 0070 explicitly lists the `type:` → `kind:` rewrite as one of its migration's requirements, while this story ships migration 0005 doing the same rewrite. Dependencies merely calls 0070 "Related" and "independent"; the ordering / responsibility split is left unresolved. The ambiguous referent for "the migration" across sections compounds this.
- **Verification gaps on migration branches** (flagged by: testability, dependency) — userspace `paths.work` override, partial-prior-run reconciliation, and body-label updates are described in Requirements but have no defined acceptance test. The coupling to `config-read-path.sh` / `paths.work` lives in Assumptions rather than Dependencies.
- **Eval-suite specificity** (flagged by: testability, dependency) — AC10 ("eval suites pass") lacks a defined threshold, and AC6 ("every grader … is updated") is unbounded without an enumeration or a verification grep.
- **Body label `**Type**:` → `**Kind**:`** (flagged by: testability) — the template gets a body label rewrite (AC1), but no corpus-level AC checks the 72 migrated work-items for stale body labels; Requirements don't say whether the migration script rewrites body labels.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency**: Ordering collision with 0070's migration not captured
  **Location**: Dependencies
  Story 0070 lists `type:` → `kind:` as one of its migration's requirements, yet this story ships 0005 doing the same rewrite. "Independent" in Dependencies does not resolve who runs first or whether 0070 drops the duplicate step.

- 🟡 **Dependency**: Migration framework prerequisite (ADR-0023) not listed in Dependencies
  **Location**: Dependencies
  The new migration is wholly dependent on ADR-0023's framework contract (driver, state file, dirty-tree guard, preview banner). This appears in Technical Notes and References but not Dependencies, hiding a hard upstream coupling.

- 🟡 **Dependency**: Internal ordering between code rename and migration execution not stated
  **Location**: Requirements
  The story bundles producer-code rename, migration authoring, and corpus migration application. Hard ordering constraint exists (producers cannot land before the corpus is migrated, or 72 work-items become unreadable), but the work item never states the required sequencing.

- 🟡 **Testability**: Eval suite pass criterion lacks a defined threshold or baseline
  **Location**: Acceptance Criteria (AC10)
  AC10 says "Work-skill and review-lens eval suites pass" without defining whether that means 100% pass, no regression from a baseline, or a specific grader threshold. The criterion is currently arguable.

- 🟡 **Testability**: No explicit verification procedure for the userspace `paths.work` override branch
  **Location**: Acceptance Criteria (AC7) / Requirements
  AC7 asserts the migration honours `paths.work` overrides, but the only end-to-end run described (AC8) exercises this repo's default `meta/work` path. The override branch — the most likely place to silently miss user files — has no acceptance test.

#### Minor

- 🔵 **Clarity**: Bare work-item IDs 0060, 0065, 0070 are referenced without titles or paths
  **Location**: Dependencies
  References lists them by number only; 0057 is the only one cited with a path.

- 🔵 **Clarity**: "the migration" has multiple candidate referents across Requirements and Dependencies
  **Location**: Requirements
  Sometimes the new 0005, sometimes 0070's broader migration; bind explicitly on first use in each section.

- 🔵 **Completeness**: References cites an ADR filename that may not match the on-disk file
  **Location**: References
  Cites `ADR-0023-migration-framework.md`; on-disk is `ADR-0023-meta-directory-migration-framework.md`.

- 🔵 **Dependency**: Userspace config-script coupling left in Assumptions
  **Location**: Dependencies
  Migration relies on `config-read-path.sh` and the `paths.work` key — a fixed-contract dependency that belongs in Dependencies.

- 🔵 **Dependency**: Eval-suite consumers implied but not named
  **Location**: Requirements / Acceptance Criteria
  Concurrent eval-authoring branches become silent conflict points; the specific eval suites are not enumerated.

- 🔵 **Dependency**: Downstream review-lens consumers not named in Blocks
  **Location**: Dependencies
  Four review lenses and `WorkItemCard.tsx` are downstream consumers; concurrent edits there will conflict but won't see this story in their blocked-by graph.

- 🔵 **Testability**: Partial-prior-run branch (`kind:` + `type:` both present) has no acceptance test
  **Location**: Acceptance Criteria (AC7) / Requirements
  Requirements describe the reconciliation branch; no AC exercises it.

- 🔵 **Testability**: Verification of SKILL.md content updates is informal
  **Location**: Acceptance Criteria (AC4)
  "Describe `kind:` in templates, examples, and instructions" admits stale narrative prose; a negative grep would tighten this.

- 🔵 **Testability**: Body label `**Type**:` → `**Kind**:` update unverified for migrated work-items
  **Location**: Acceptance Criteria (AC1, AC9)
  Template gets the body label update (AC1); the corpus migration ACs (AC8, AC9) don't check the 72 migrated work-items for stale body labels.

- 🔵 **Testability**: Grader-update criterion is unbounded without an enumeration
  **Location**: Acceptance Criteria (AC6)
  No list of grader files and no verification grep; a missed grader would slip through.

#### Suggestions

- 🔵 **Clarity**: "The Rust server side" has an implicit referent
  **Location**: Requirements
  Cite the relevant crate path, or rephrase to name the visualiser backend explicitly.

- 🔵 **Clarity**: AC2's exclusion sub-bullet is grammatically a peer but semantically a qualifier
  **Location**: Acceptance Criteria
  Restructure so exclusions read as qualifiers on the greps rather than as a separate criterion.

- 🔵 **Scope**: Corpus-migration application bundled with authoring
  **Location**: Requirements / Acceptance Criteria
  Defensible bundling; worth a one-sentence justification in Drafting Notes.

- 🔵 **Scope**: "Blocked by 0060 … could land independently" leaves the in-scope posture unstated
  **Location**: Dependencies
  Pick one — gated on 0060, or shipping independently — to remove ambiguity about deliverable shape.

### Strengths

- ✅ Disambiguates the two senses of "type" (artifact-type discriminator vs work-item semantic kind) consistently throughout, using "kind" for the latter.
- ✅ Every affected file/directory is cited by absolute path — templates, helpers, SKILL.mds, lenses, frontend component, tests, migration — eliminating guesswork on referents.
- ✅ Acceptance Criteria use concrete grep expressions with capture groups and explicit false-positive exclusions (`type: work-item-review`, `entry.type`, `params.type`, `subagent_type`, TypeScript `type` keywords).
- ✅ Idempotency is verified with a concrete procedure (AC8: run migrate twice; second run is no-op), and post-migration corpus state is asserted with anchored regex (AC9).
- ✅ File counts are explicit (seven work-skill SKILL.mds, four review-lens SKILL.mds, ~100 fixtures, 72 work-items) — bounds otherwise unbounded "all" phrasing.
- ✅ Drafting Notes records prior scoping decisions (agents/ carve-out, coarser grep replaced, corpus migration reassigned from 0070) so reviewers can trace intent.
- ✅ Scope is genuinely atomic and explicitly justifies single-story packaging by quoting the parent epic 0057's instruction.
- ✅ Technical Notes captures framework conventions and explicitly enumerates concerns the migration must NOT implement (dirty-tree guard, state file append, preview, skip handling) — clean ownership boundaries.

### Recommended Changes

1. **Resolve 0070 overlap explicitly** (addresses: ordering collision with 0070's migration; "the migration" referent ambiguity)
   In Dependencies, change "Related: 0070 … this story's rename migration is independent" to one of:
   - "Blocks: 0070 — its migration must drop the type→kind rewrite step (already performed by 0005)", or
   - "Depends on: 0070 for the type→kind rewrite; this story does not author migration 0005."
   Whichever posture is correct, restate consistently in the Summary and Drafting Notes. Then use "0005" or "the rename migration (0005)" on first reference in each section.

2. **Hoist hard couplings into Dependencies** (addresses: ADR-0023 framework prerequisite; userspace config-script coupling; review-lens / WorkItemCard.tsx downstream consumers)
   Add Dependencies entries:
   - `Depends on: ADR-0023 migration framework (driver + state file contract)`.
   - `Depends on: paths.work config resolution (config-read-path.sh + .accelerator/config.md schema)`.
   - `Blocks: concurrent edits to {completeness, scope, dependency, testability}-lens SKILL.mds and WorkItemCard.tsx (cross-link to the Technical Notes "Affected surface" list).`

3. **State sequencing constraints in Requirements** (addresses: internal ordering between code rename and migration execution)
   Add a "Sequencing" sub-section to Requirements:
   1. Author migration 0005.
   2. Apply 0005 in this repo so `meta/work/*.md` has `kind:` everywhere.
   3. Land producer-code rename + fixture rename + grader updates in the same commit/PR as the migration, so old-shape files cannot exist post-merge.

4. **Tighten AC10 with a defined eval-pass threshold** (addresses: eval suite pass criterion lacks threshold)
   Replace "eval suites pass on the renamed fixtures" with "eval pass rate is unchanged from the pre-rename baseline (capture baseline before applying the rename) — any new failure must be explicitly enumerated and attributed."

5. **Add an explicit `paths.work` override AC** (addresses: no userspace override verification)
   Add: "With `paths.work: <custom-path>` set in `.accelerator/config.md` and a work-item containing `type: story` under that directory, running the migration renames that file's field while leaving the default `meta/work` untouched if absent."

6. **Add a partial-prior-run AC** (addresses: partial-prior-run branch has no acceptance test)
   Add: "Given a work-item containing both `type: story` and `kind: story` lines, the migration produces a file containing only `kind: story` (stale `type:` removed)."

7. **Decide on body-label rewrite for migrated work-items** (addresses: body label `**Type**:` → `**Kind**:` unverified)
   Either:
   - Add a Requirement that migration 0005 also rewrites body `**Type**:` → `**Kind**:` and an AC asserting `meta/work/*.md` has no `^\*\*Type\*\*:` line referring to the semantic kind; or
   - Explicitly state in Requirements that body labels in existing work-items are out of scope for the migration and will drift until manually edited.

8. **Bound AC6 with a verification grep or enumeration** (addresses: grader-update criterion is unbounded)
   Add: `rg -n '"type":\s*"(story|epic|task|bug|spike)"' --glob '**/evals.json' --glob '**/benchmark.json'` returns no hits — or enumerate the affected grader files in Technical Notes.

9. **Tighten AC4 with a negative grep for SKILL.md prose** (addresses: SKILL.md content updates verified informally)
   Add: "In the eleven affected SKILL.md files, every occurrence of literal `type:` corresponds to the artifact-type discriminator (`type: work-item-review`, etc.); no occurrence refers to the work-item semantic kind." Optionally provide the matching ripgrep pattern.

10. **Fix References / clarify referents** (addresses: bare IDs, ADR filename mismatch, "the Rust server side", AC2 exclusion structure)
    - Expand `Related: 0057, 0060, 0065, 0070` to full paths matching the 0057 style.
    - Update the ADR reference filename to `meta/decisions/ADR-0023-meta-directory-migration-framework.md`.
    - In Requirements, replace "The Rust server side is field-name-agnostic" with a path-bearing rephrase ("the visualiser's Rust backend under `…` reads frontmatter fields generically").
    - Restructure AC2 so the false-positive exclusions read as qualifiers on the grep commands, not a peer bullet.

11. **Pick a single posture for the 0060 relationship** (addresses: conditional blocked-by wording)
    Either gate this story on 0060 unconditionally, or state explicitly that it ships ahead of 0060 (with whatever vocabulary alignment that implies). Remove the "could land independently" hedge.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Communicates intent precisely with one well-bounded field rename, the affected surface enumerated by exact paths, and acceptance criteria expressed as concrete grep commands and file-level outcomes. Referents for "the migration", "the rename", and "this story" are unambiguous in context; jargon (work-skill, review-lens, discriminator, field-name-agnostic) is defined inline or grounded in the parent epic.

**Strengths**:
- Disambiguates the two senses of "type" explicitly and uses "kind" for the work-item semantic kind throughout.
- Names every affected file/directory by absolute path.
- Acceptance Criteria use concrete grep expressions with explicit false-positive exclusions.
- Drafting Notes records prior scoping decisions so a reader can trace intent.
- Migration requirements describe the framework contract without ambiguity about ownership.

**Findings**: 2 minor (bare IDs in Dependencies; "the migration" referent), 2 suggestions ("The Rust server side" referent; AC2 exclusion-bullet structure).

### Completeness

**Summary**: Structurally complete and substantively populated for a story. Frontmatter is well-formed, Summary identifies actor + action + rationale, Context explains motivation without restating Summary, Acceptance Criteria has nine specific bullets including concrete grep commands, Requirements enumerate each affected surface with paths, and supporting sections (Dependencies, Assumptions, Technical Notes, Drafting Notes, References) are all populated.

**Strengths**:
- Frontmatter is complete and valid (type=story, status=draft, priority=high, parent, tags, work_item_id).
- Summary captures actor, action, rationale in one statement.
- Context explains forces behind the rename, naming parent epic 0057.
- Acceptance Criteria has nine bullets with grep commands and explicit exclusions.
- Requirements enumerate every affected surface with file paths.
- Technical Notes captures framework conventions and what the migration must NOT implement.
- Drafting Notes records deviations from the parent epic.

**Findings**: 1 minor (ADR-0023 reference filename may not match on-disk file).

### Dependency

**Summary**: Primary upstream (0060) and downstream (0065) couplings are captured, and 0057/0070 are listed as related. Several implied couplings are missing or in the wrong section: ADR-0023 framework prerequisite is in Technical Notes only; `paths.work` / config-script coupling is in Assumptions; ordering with 0070's migration is unresolved; sequencing inside this story (code rename vs migration application) is not stated; eval-suite consumers are not enumerated; review-lens / `WorkItemCard.tsx` downstream consumers are not in Blocks.

**Strengths**:
- Dependencies names both an upstream blocker (0060) and a downstream consumer (0065), with the soft-vs-hard nature of 0060 called out.
- Parent epic 0057 and broader migration story 0070 are surfaced as Related.
- Technical Notes name framework dependency (ADR-0023), canonical migration model (0001), and config resolver — though they belong more properly in Dependencies.
- Assumptions captures the userspace-config coupling.

**Findings**: 3 major (0070 ordering collision; ADR-0023 not in Dependencies; internal ordering between rename and migration), 3 minor (config-script coupling in Assumptions; eval-suite consumers implied; review-lens downstream consumers not in Blocks).

### Scope

**Summary**: One coherent atomic rename across every surface that names `type:`, plus a migration to apply the rename to the repo corpus. Despite broad fan-out (templates, helpers, seven skills, four lenses, frontend, ~100 fixtures, graders, 72 corpus files), the scope is genuinely indivisible — partial delivery would leave the corpus or producers inconsistent — and the parent epic mandates single-story coordination. Story type is appropriate.

**Strengths**:
- Single unified purpose throughout; no "and also" bundling.
- Explicitly justifies single-story packaging by quoting the parent epic.
- Migration scope explicitly delineated against 0070's broader migration.
- Scope corrections against the parent epic surfaced (agents/ carve-out, eval-fixture/grader/visualiser additions).
- Acceptance Criteria enumerate affected surfaces one-for-one against Requirements.

**Findings**: 2 suggestions (corpus migration application bundled with authoring — defensible but un-justified; conditional "blocked by 0060" leaves posture unstated).

### Testability

**Summary**: Acceptance criteria are notably strong on verifiability — explicit ripgrep commands with false-positive exclusions, specific file counts, an explicit idempotency test. Main gaps: undefined eval pass threshold, no procedure for the `paths.work` override branch, no test for the partial-prior-run reconciliation branch, no corpus-level check that body-label `**Type**:` → `**Kind**:` propagated, and informal phrasing of SKILL.md content updates.

**Strengths**:
- AC2 specifies exact grep patterns rather than abstract statements.
- False-positive exclusions enumerated up front.
- Idempotency verified with a concrete procedure (run twice; second is no-op).
- Post-migration corpus asserted with anchored regex (AC9).
- File counts are explicit, bounding "all" phrasing.

**Findings**: 2 major (AC10 lacks pass threshold; AC7 lacks `paths.work` override procedure), 4 minor (no partial-prior-run AC; SKILL.md content updates verified informally; body-label update unverified for migrated work-items; AC6 grader criterion unbounded).

## Re-Review (Pass 2) — 2026-05-20T20:57:34Z

**Verdict:** REVISE

Substantial progress — six pass-1 findings are fully resolved and several majors have been downgraded or restructured. The verdict remains REVISE because three new or persisting majors stand: a reference-path error to ADR-0023, an ambiguity introduced by the new body-label rewrite (whether it filters on the kind vocabulary), and a missing action-item linkage on 0070 to actually amend its plan. The four originally-deferred majors (ADR-0023 in Dependencies, internal sequencing, AC10 threshold, `paths.work` override AC) were explicitly skipped per user direction and remain partially open; some have been re-flagged at minor severity by this pass.

### Previously Identified Issues

- 🟡 **Dependency** — Ordering collision with 0070's migration not captured — **Resolved** (Dependencies now lists `Blocks: 0070` with explicit rewrite-step ownership; Drafting Notes record the split).
- 🟡 **Dependency** — Migration framework prerequisite (ADR-0023) not in Dependencies — **Still present** (skipped per user direction in this edit pass; not re-flagged by the dependency lens this pass).
- 🟡 **Dependency** — Internal ordering between code rename and migration execution — **Still present** (skipped per user direction; not re-flagged this pass).
- 🟡 **Testability** — Eval suite pass criterion lacks threshold — **Partially resolved / downgraded** (still flagged this pass as minor — AC10 does not name the suites, invocation, or pass condition).
- 🟡 **Testability** — `paths.work` override procedure missing — **Still present** (skipped per user direction; not re-flagged this pass).
- 🔵 **Clarity** — Bare IDs 0060, 0065, 0070 — **Partially resolved** (0060 expanded with context; 0070 now Blocks with description; 0065 still bare — re-flagged as minor this pass).
- 🔵 **Clarity** — "the migration" referent — **Resolved** (bound to "migration 0005" / "the rename migration (0005)" on first use throughout).
- 🔵 **Completeness** — ADR-0023 reference filename mismatch — **Still present** (re-flagged as minor by completeness lens and elevated to major by clarity lens).
- 🔵 **Dependency** — Config-script coupling in Assumptions — **Still present** (skipped per user direction; not re-flagged this pass).
- 🔵 **Dependency** — Eval-suite consumers implied but not named — **Still present** (not re-flagged exactly; lens raised related "sibling stories under 0057" concern instead).
- 🔵 **Dependency** — Downstream review-lens consumers not in Blocks — **Still present** (skipped per user direction; not re-flagged this pass).
- 🔵 **Testability** — Partial-prior-run branch has no AC — **Still present** (downgraded to suggestion this pass).
- 🔵 **Testability** — SKILL.md content updates verified informally (AC4) — **Still present** (re-flagged as minor this pass with a concrete grep suggestion).
- 🔵 **Testability** — Body-label `**Type**:` → `**Kind**:` unverified for migrated work-items — **Resolved** (migration 0005 now rewrites body labels; AC7 and AC9 assert both frontmatter and body-label post-state).
- 🔵 **Testability** — Grader-update criterion unbounded (AC6) — **Still present** (re-flagged this pass with a concrete grep suggestion).
- 🔵 **Clarity** — "The Rust server side" implicit referent — **Resolved** (rephrased to "the visualiser's Rust backend (under …) reads frontmatter fields generically").
- 🔵 **Clarity** — AC2 exclusion sub-bullet structure — **Resolved** (restructured so false-positive list reads as a qualifier on the greps, not a peer bullet).
- 🔵 **Scope** — Corpus-migration bundling un-justified — **Resolved** (Drafting Notes now explains why authoring and corpus application ship together).
- 🔵 **Scope** — Conditional "blocked by 0060" leaves posture unstated — **Resolved** (0060 marked complete; hedge removed; recorded in Drafting Notes).

### New Issues Introduced

- 🟡 **Clarity** — Ambiguity in body-label rewrite filter (Requirements / AC9): the body-label rewrite is anchored on `^\*\*Type\*\*:` unconditionally in Requirements, but AC9 qualifies it as "body-label lines referring to the work-item kind". The migration cannot distinguish a `**Type**:` line referring to the semantic kind from one in quoted prose or examples. (Introduced by the body-label extension edit — needs disambiguation: unconditional, or filter on `story|epic|task|bug|spike` value.)
- 🟡 **Clarity** — ADR-0023 reference path is wrong (References): cited as `ADR-0023-migration-framework.md` but the on-disk filename is `ADR-0023-meta-directory-migration-framework.md`. Elevated to major by clarity lens because the reference is unresolvable.
- 🟡 **Dependency** — Reverse coupling to 0070 not captured as an action item: Dependencies asserts 0070 "must drop" its rewrite step, but does not capture the action that 0070's draft must actually be amended (and by whom/when) for the linkage to hold.
- 🔵 **Clarity** — "their callers must pass `kind`" has an ambiguous "callers" referent (Requirements helper-scripts bullet).
- 🔵 **Clarity** — "0070's migration must drop the rewrite step" presumes a step that may not exist in 0070's draft yet; reword to "must not include".
- 🔵 **Clarity** — AC2 states verification mechanism without stating the underlying invariant in prose; overlaps with Technical Notes false-positive list (single source of truth concern).
- 🔵 **Clarity** — Idempotency "drop the stale `type:` line" assumes `kind:` is authoritative when values differ; behaviour silent for mismatched values.
- 🔵 **Completeness** — No Open Questions section (acceptable if intentionally none; consider explicit statement).
- 🔵 **Dependency** — Migration-ordering precondition (0002–0004 apply cleanly first) not surfaced.
- 🔵 **Dependency** — Eval-runner infrastructure dependency not named.
- 🔵 **Scope** — Story sits at the upper bound of story-sized work (acceptable per atomicity argument; flagged as observation).
- 🔵 **Testability** — AC4 verification still informal (re-flag with concrete grep suggestion).
- 🔵 **Testability** — AC6 verification still unbounded (re-flag with concrete grep suggestion).
- 🔵 **Testability** — AC10 doesn't name eval suites or pass condition.
- 🔵 **Scope** — Eval fixtures volume could benefit from a programmatic rewrite via the migration pattern (suggestion only; bundling is justified).
- 🔵 **Testability** — AC2 grep verification relies on manual false-positive inspection (suggestion).

### Assessment

The pass-2 work item is materially better than pass-1 — six findings fully resolved, two more downgraded, and the cross-cutting 0070-overlap theme is closed in Dependencies/Drafting Notes. The remaining REVISE verdict is driven by three concerns that are each modest in size: a one-line References path fix, a one-clause clarification of the body-label rewrite filter, and either an "action: amend 0070" note or a wording tweak ("must not include" rather than "must drop"). The skipped originals (ADR-0023 in Dependencies, internal sequencing, AC10 threshold, `paths.work` override AC) remain genuinely open but are tractable single-edit additions. Another short edit pass focused on the three new majors plus the four originally-skipped majors would likely land the story at APPROVE.

## Re-Review (Pass 3) — 2026-05-20T21:08:04Z

**Verdict:** REVISE

All three pass-2 majors are resolved. Verdict remains REVISE because the testability lens has re-raised two previously-flagged concerns at major severity: the eval-fixture/grader enumeration (originally a minor as AC6 unbounded) and the "excluded by inspection" subjective verification step (originally a pass-2 suggestion). These are both tractable — converting the grep into a deterministic command and adding a concrete eval-fixture grep would clear the verdict. New minor and suggestion findings are mostly observability nits around acceptance-criterion phrasing rather than substantive gaps.

### Previously Identified Issues (Pass 2 → Pass 3)

- 🟡 **Clarity** (pass 2) — ADR-0023 reference path wrong — **Resolved** (References now points at `ADR-0023-meta-directory-migration-framework.md`).
- 🟡 **Clarity** (pass 2) — Body-label rewrite filter ambiguity — **Resolved** (Requirements now state the rewrite is unconditional on regex; AC9 updated to match).
- 🟡 **Dependency** (pass 2) — Reverse coupling to 0070 not captured as action — **Resolved** (wording changed to "must not include" in both Dependencies and Drafting Notes — robust regardless of 0070's draft state).
- 🔵 **Clarity** (pass 2) — 0065 still bare — **Still present** (re-flagged this pass as minor; analogous one-line gloss requested).
- 🔵 **Clarity** (pass 2) — "their callers" referent — **Not re-flagged** (still present in Requirements text but not raised this pass).
- 🔵 **Clarity** (pass 2) — AC2 invariant should be stated as prose — **Partially captured** (this pass re-flags as the "excluded by inspection" testability major).
- 🔵 **Clarity** (pass 2) — Idempotency "stale type:" silent on mismatched values — **Not re-flagged** (still present in Requirements text).
- 🔵 **Completeness** (pass 2) — No Open Questions section — **Downgraded to suggestion**.
- 🔵 **Dependency** (pass 2) — Migration ordering precondition (0002–0004) — **Not re-flagged**.
- 🔵 **Dependency** (pass 2) — Eval-runner infra dependency unnamed — **Not re-flagged**.
- 🔵 **Testability** (pass 2) — AC4 verification still informal — **Still present** (re-flagged as minor this pass).
- 🔵 **Testability** (pass 2) — AC6 grader verification unbounded — **Still present, escalated to major** as "Eval fixture scope is unbounded".
- 🔵 **Testability** (pass 2) — AC10 no defined invocation — **Still present** (re-flagged as minor).
- 🔵 **Testability** (pass 2) — AC2 manual inspection — **Still present, escalated to major** as "'Excluded by inspection' makes the check subjective".

### Still Present — Originally Skipped Majors (from pass 1, deferred per user direction)

- 🟡 **Dependency** — ADR-0023 framework prerequisite not in Dependencies (not re-flagged this pass, but still absent).
- 🟡 **Dependency** — Internal sequencing between code rename and migration execution (not re-flagged this pass).
- 🟡 **Testability** — AC10 lacks pass threshold (re-flagged this pass as minor "no defined invocation").
- 🟡 **Testability** — `paths.work` override AC missing (not re-flagged this pass).

### New Issues Introduced (Pass 3)

**Major (2):**
- 🟡 **Testability** — Eval fixture scope unbounded: "All eval fixture work-item files" / "every grader" has no enumeration or verification grep; could be claimed met with missed fixtures.
- 🟡 **Testability** — "Excluded by inspection" is subjective; two reviewers could disagree on what counts as a false-positive match. Suggest encoding exclusions as `rg --glob '!...'` filters or negative-match patterns so the post-exclusion grep is itself the oracle.

**Minor:**
- 🔵 **Clarity** — Field-name-agnostic readers listed in Technical Notes "Affected surface" despite Requirements saying no code change is needed; readers may waste effort or introduce unneeded edits.
- 🔵 **Clarity** — Pronoun "it" in "running it a second time is a no-op" has two reasonable referents (`/accelerator:migrate` vs migration 0005 directly).
- 🔵 **Dependency** — Implicit dependency on `/accelerator:migrate` driver behaviour (discovery, ordering, state-file) not surfaced as a precondition.
- 🔵 **Dependency** — Eval grader file paths not enumerated as concrete dependencies (this is also part of the major testability finding above).
- 🔵 **Scope** — Story sits near upper bound of story-sized work (observation; bundling defensible).
- 🔵 **Testability** — Helper-script criterion is implementation-shaped ("the hardcoded `type)` case is renamed") rather than behaviour-shaped; reframe as a behavioural test.
- 🔵 **Testability** — Second-run no-op lacks an observable signal (exit code? files unchanged? driver report?).

**Suggestions:**
- 🔵 **Clarity** — "Body label" terminology used without prior definition (a sentence near Context/Requirements would help readers unfamiliar with the template).
- 🔵 **Completeness** — Optional Open Questions section.
- 🔵 **Scope** — Confirm "author migration + apply to dogfood repo" as a standard pattern for future migration stories (process suggestion, not scope change).

### Assessment

The work item has now seen three review passes and is substantively strong: scope is atomic and well-justified, dependencies are clearly drawn (parent/blocks/related), migration design is precise (frontmatter and body-label rewrites, idempotency, partial-prior-run recovery, `paths.work` override), and acceptance criteria carry concrete greps with enumerated false-positive exclusions. The remaining REVISE verdict is testability-driven: two acceptance criteria still rely on manual or unbounded verification (eval fixtures + the "excluded by inspection" qualifier on the main grep). These are tractable one-edit fixes — converting the grep exclusions into negative-match flags and adding a concrete `rg` over the eval fixture directories would likely take the verdict to APPROVE in a fourth pass. The four pass-1 originally-skipped majors (ADR-0023 in Dependencies, internal sequencing, AC10 threshold, `paths.work` override AC) also remain genuinely open.

## Pass 4 Edits — 2026-05-20T21:17:46Z

**Verdict:** APPROVE

The remaining six tractable majors were addressed in a sixth edit pass:

- **Pass-3 majors resolved:**
  - AC2 ("excluded by inspection") replaced with a deterministic
    `rg ... | rg -v 'work-item-review|subagent_type'` pipeline; remaining
    false-positive classes (`entry.type`, `params.type`, TypeScript `type`)
    are filtered by the field-value pattern itself.
  - AC6 grader/fixture verification: explicit `rg` commands added for both
    fixture work-items (`skills/work skills/review/lenses`) and grader JSON
    files (`*evals.json`, `*benchmark.json`).
- **Pass-1 originally-skipped majors resolved:**
  - ADR-0023 migration-framework prerequisite added to Dependencies as
    an explicit `Depends on` entry, with `run-migrations.sh` contract named.
  - `paths.work` config-resolution layer added to Dependencies as a second
    `Depends on` entry.
  - Sequencing sub-section added to Requirements specifying the required
    order: author migration 0005 → apply in this repo → land producer/
    fixture/grader changes in the same commit/PR.
  - AC10 ("eval suites pass") tightened with a baseline-comparison pass
    condition; new failures must be enumerated and attributed.
  - New AC for `paths.work` override branch: custom path renamed,
    default left untouched if redirected.
  - Idempotency AC strengthened with observable signals: exit 0, driver
    reports 0005 as already applied, `jj status` shows no further changes.

With these in place, no critical or major findings remain. The remaining
minor and suggestion items (e.g. 0065 description, helper-script criterion
phrasing, body-label terminology gloss, Open Questions section) are
non-blocking polish that can be addressed in a follow-up if desired.

**Final verdict: APPROVE.** The work item is ready for implementation.
