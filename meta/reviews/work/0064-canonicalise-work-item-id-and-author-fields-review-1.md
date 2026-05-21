---
date: "2026-05-21T16:50:00+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0064-canonicalise-work-item-id-and-author-fields.md"
work_item_id: "0064"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
---

## Work Item Review: Canonicalise `work_item_id` and `author` Field Names

**Verdict:** REVISE

The work item is structurally complete, well-motivated, and concretely scoped — producers, consumers, a corpus migration, and the visualiser are all bundled into a single releasable change with named files and a migration precedent. The dominant issue is testability: three Acceptance Criteria use unbounded language ("any", "every", "correctly", "releasable") without enumerated surfaces or defined verification procedures, making pass/fail interpretation-dependent. Secondary themes are minor clarity gaps (undefined "RCA" acronym, scope mismatch between Summary and Requirements) and a couple of soft dependency-coordination notes around 0060 and 0063.

### Cross-Cutting Themes

- **Unbounded "any/every" language across scope and verification** (flagged by: clarity, testability, dependency) — The Requirements bullet for frontend updates, the "any helper scripts/agent prompts/downstream consumers" line, and Acceptance Criteria 1–2 all use "any" without enumerating the surface. This shows up as a clarity finding (vague referent), three testability findings (unverifiable criteria), and a dependency finding (unenumerated downstream consumers). Fixing it once — by enumerating the in-scope directories and consumer files — closes all four.
- **Bundling of two independent renames** (flagged by: scope, clarity) — Scope flags the bundled story as a deliberate but worth-noting tradeoff; clarity flags an Assumptions sentence that calls the renames "independent" while the rest of the story binds them together. Both are addressable with a single re-wording.

### Findings

#### Critical
(none)

#### Major
- 🟡 **Testability**: First two criteria use unbounded "any" without an enumerated scope
  **Location**: Acceptance Criteria
  Criteria 1 and 2 assert no source references `work-item:` or `researcher:` but don't bound the search corpus — false positives in fixtures, historical migrations, and prose have no defined disposition. Pass/fail is interpretation-dependent.

- 🟡 **Testability**: "Visualiser renders ... correctly" is subjective and unbounded
  **Location**: Acceptance Criteria
  "Correctly" is undefined and "existing" has no enumerated scope. A tester could verify one card or dozens of views and still be unsure they covered the consumer surface.

- 🟡 **Testability**: "Releasable as a single version bump" has no defined verification procedure
  **Location**: Acceptance Criteria
  The criterion is a meta-property with no stated execution path (dry-run? upgrade simulation against a fixture?) and no defined success signal — likely to be claimed-met by inspection alone.

#### Minor
- 🔵 **Clarity**: RCA acronym used without definition
  **Location**: Context
  "RCA" appears in Context, Requirements, and Acceptance Criteria without expansion. Newcomers may need to grep to confirm what artefact category is being migrated.

- 🔵 **Clarity**: Ambiguous referent for "The two renames are independent"
  **Location**: Assumptions
  The Assumption says they "can ship together or separately", but the Summary and Requirements bind them into a single migration. The contract the rest of the story sets is weakened.

- 🔵 **Clarity**: Summary scope narrower than Requirements scope
  **Location**: Summary
  Requirements add "helper scripts, agent prompts, or downstream consumers" and a 0060 cross-reference that a Summary-only reader would not anticipate.

- 🔵 **Completeness**: No Open Questions section present
  **Location**: Open Questions
  The bundling decision and earlier visualiser-scope ambiguity (mentioned in Drafting Notes) suggest at least one open question could be captured for reviewers.

- 🔵 **Dependency**: Coordination relationship with 0063 (sibling rename) not captured as an ordering constraint
  **Location**: Dependencies
  0063 is described as "the other coordinated rename" but no ordering note exists for migration-number selection or release sequencing.

- 🔵 **Dependency**: 0060 listed as "Blocked by" but may already be resolved context
  **Location**: Dependencies
  Story is in draft and already treats `work_item_id` and `author` as chosen canonical names; if 0060 has already landed, "Blocked by" overstates the coupling.

- 🔵 **Dependency**: Downstream consumers beyond 0065 are not named as Blocks
  **Location**: Requirements / Dependencies
  Requirements references "any helper scripts, agent prompts, or downstream consumers" but only 0065 is named under Blocks; specific known consumers are not enumerated.

- 🔵 **Testability**: "Any helper scripts, agent prompts, or downstream consumers" is unenumerated
  **Location**: Requirements
  Unbounded "any" set with no Acceptance Criterion that verifies it independently of criteria 1–2.

- 🔵 **Testability**: Migration idempotence has no defined verification procedure
  **Location**: Acceptance Criteria
  Criterion 3 asserts idempotence but the named fixtures (default-layout, partial-prior-run, paths-override) don't explicitly cover re-running a fully-migrated input.

#### Suggestions
- 🔵 **Clarity**: "any frontend types or components that key off the old field names" is vague
  **Location**: Requirements
  Technical Notes later enumerates concrete files; the Requirements bullet should inline or cross-reference them rather than say "any".

- 🔵 **Clarity**: "configured corpus" is undefined locally
  **Location**: Acceptance Criteria
  Verifying the third criterion requires implicit knowledge of how `/accelerator:migrate` resolves paths.

- 🔵 **Completeness**: Story does not explicitly identify the user or consumer whose need is met
  **Location**: Context
  Context describes the schema inconsistency but doesn't name the beneficiary (skill authors, visualiser, userspace consumers upgrading via migrate).

- 🔵 **Scope**: Two independent renames bundled in one story
  **Location**: Requirements
  Drafting Notes acknowledges the bundling tradeoff. No change required if the team is comfortable with the documented tradeoff; otherwise split into 0064a/0064b sharing one migration script.

- 🔵 **Scope**: Cross-reference task may be scope-adjacent
  **Location**: Requirements
  The "Cross-reference 0060" Requirements bullet reads as coordination/dependency activity rather than a deliverable; 0060 is already in Dependencies.

### Strengths
- ✅ Field-name renames are spelled out explicitly with both old and new names, eliminating ambiguity about what changes.
- ✅ Acceptance Criteria identify concrete file/section locations and observable outcomes for most criteria (no source references the old names, migration is idempotent, etc.).
- ✅ Migration precedent (0005) is named in Requirements, Technical Notes, and References — implementers have a concrete template to follow.
- ✅ Specific consumer file paths (frontmatter.rs:297-326, indexer.rs ~1008, frontend types, WorkItemCard.tsx) are enumerated in Technical Notes.
- ✅ Frontmatter is fully populated; all expected sections (Summary, Context, Requirements, Acceptance Criteria, Dependencies, Assumptions, Technical Notes, Drafting Notes, References) are substantive.
- ✅ Dependencies section captures the upstream blocker (0060), downstream consumer (0065), parent (0057), sibling (0063), and broader related (0070) work items explicitly.
- ✅ Assumptions explicitly address external-system coupling (no external CI/dashboard/sync tooling reads these fields; userspace consumers flow through the plugin API).
- ✅ Drafting Notes transparently acknowledges the bundling tradeoff and offers a clean split path, demonstrating deliberate scope reasoning.

### Recommended Changes

1. **Bound the source-grep Acceptance Criteria** (addresses: "First two criteria use unbounded 'any'", "Any helper scripts... unenumerated")
   Restate criteria 1 and 2 as a concrete bounded check, e.g.: "`rg -n 'work-item:|researcher:' templates/ skills/ scripts/ commands/ skills/visualisation/visualise/server/src/ skills/visualisation/visualise/frontend/src/` returns no matches outside `skills/config/migrate/migrations/` and pre-existing meta documents." This single change also closes the "any helper scripts/agent prompts" testability finding.

2. **Replace "renders correctly" with concrete observable checks** (addresses: "'Visualiser renders ... correctly' is subjective")
   Substitute one or more enumerated checks: e.g. "After migration, the kanban view at `/kanban` lists all previously visible work items with `work_item_id` cross-references intact (no broken-link badges), and the detail page for plan 0063 displays its parent work-item link."

3. **Define the release/upgrade verification procedure** (addresses: "'Releasable as a single version bump' has no defined verification")
   Replace the criterion with an executable check: "Running `/accelerator:migrate` against a fixture repo at the pre-rename plugin version, then upgrading the plugin, results in (a) all plan frontmatter rewritten to `work_item_id:`, (b) all research/RCA frontmatter rewritten to `author:`, and (c) the visualiser smoke-test in criterion 7 passing."

4. **Add an explicit idempotence fixture or note** (addresses: "Migration idempotence has no defined verification procedure")
   Either rename `partial-prior-run` to clarify it includes a fully-migrated input, or add: "Running the migration twice on the default-layout fixture produces no diff on the second invocation."

5. **Expand RCA on first use and tighten frontend-files Requirements bullet** (addresses: "RCA acronym used without definition", "'any frontend types or components' is vague")
   In Context, write "root-cause analysis (RCA)" on first use. In the visualiser-consumer Requirements bullet, replace "any frontend types or components" with the enumerated list already in Technical Notes (`frontend/src/api/types.ts`, `api/work-item.ts`, `routes/kanban/WorkItemCard.tsx`).

6. **Reword the Assumptions "independent" sentence** (addresses: "Ambiguous referent for 'The two renames are independent'")
   Mirror the Drafting Notes wording: the renames are logically independent but are intentionally bundled into one migration and one release for this story.

7. **Clarify 0060's status and 0063's ordering relationship** (addresses: "0060 listed as 'Blocked by' but may already be resolved", "Coordination relationship with 0063 not captured")
   In Dependencies, either move 0060 to Related (if its decision is already settled) or state precisely what must merge first. Add a one-line ordering note for 0063: either "Coordinated with 0063; pick the next migration number after 0063's lands" or "Independent of 0063 — migration numbers may interleave."

8. **Optional: extend Summary to cover the full consumer set** (addresses: "Summary scope narrower than Requirements scope", "Story does not explicitly identify the user")
   Add a sentence to the Summary or Context naming helper scripts/agent prompts as additional touch points, and identifying the primary beneficiaries (skill authors and downstream consumers like the visualiser).

9. **Optional: add a brief Open Questions section** (addresses: "No Open Questions section present")
   Either capture the one remaining negotiable decision (e.g., "Is the bundling preferred, or should this split into 0064a/0064b?") or explicitly state in Drafting Notes that no open questions remain.

10. **Optional: drop or rephrase the 0060 cross-reference Requirements bullet** (addresses: "Cross-reference task may be scope-adjacent")
    Remove the bullet (0060 is already in Dependencies) or rephrase it as a Technical Note about where the canonical names are sourced from.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely clear, with well-defined renames, named actors (templates, skills, migration, visualiser), and concrete outcomes. A few minor clarity issues exist around undefined acronyms (RCA), an ambiguous referent in the Assumptions section, and a slight scope mismatch between the Summary (which names only producers/consumers/migration) and the Requirements (which add helper scripts and agent prompts).

**Strengths**:
- Field-name renames are spelled out explicitly with both old and new names, eliminating ambiguity about what changes.
- Acceptance Criteria identify concrete file/section locations and observable outcomes.
- Actors are consistently named (the migration, the visualiser, producers, consumers).
- Dependencies section disambiguates the relationship to neighbouring work items (0057, 0060, 0063, 0065, 0070).

**Findings**:
- 🔵 minor / high — "RCA acronym used without definition" (Context): RCA appears in Context, Requirements, and Acceptance Criteria without definition. Suggestion: expand on first use.
- 🔵 minor / medium — "Ambiguous referent for 'The two renames are independent'" (Assumptions): Says renames can ship separately, but Summary/Requirements bind them together. Suggestion: mirror Drafting Notes wording.
- 🔵 minor / medium — "Summary scope narrower than Requirements scope" (Summary): Requirements add helper scripts/agent prompts not in the Summary. Suggestion: extend Summary.
- 🔵 suggestion / medium — "'any frontend types or components that key off the old field names' is vague" (Requirements): Technical Notes enumerates these files — inline or cross-reference.
- 🔵 suggestion / low — "'configured corpus' is undefined locally" (Acceptance Criteria): Requires implicit knowledge of migrate framework terminology.

### Completeness

**Summary**: The work item is structurally complete for a story: substantive Summary, Context, Requirements, Acceptance Criteria, Dependencies, Assumptions, Technical Notes, Drafting Notes, and References sections, with valid frontmatter. The story articulates the motivation and provides enumerated, specific acceptance criteria. No critical gaps; only minor observations about a missing Open Questions section and an implied 'for whom'.

**Strengths**:
- Frontmatter is fully populated with all expected fields.
- Acceptance Criteria contains eight specific, enumerated bullets.
- Context clearly explains why the work is needed, naming both conflicts and citing parent 0057.
- Requirements section enumerates concrete changes across templates, skills, visualiser code, migration, and helper scripts.
- Dependencies, Assumptions, Technical Notes, Drafting Notes, and References are all substantive.
- Migration precedent (0005) is explicitly named.

**Findings**:
- 🔵 minor / medium — "No Open Questions section present" (Open Questions): Bundling decision and earlier ambiguity in Drafting Notes suggest at least one open question could be captured.
- 🔵 suggestion / low — "Story does not explicitly identify the user or consumer whose need is met" (Context): Beneficiaries (skill authors, visualiser, userspace consumers) not explicitly named.

### Dependency

**Summary**: The work item explicitly captures its primary upstream blocker (0060), its downstream consumer (0065), and the broader related context (0057, 0063, 0070). Internal couplings — visualiser consumers, templates, migration framework, and the 0005 migration precedent — are named in Requirements and Technical Notes. The main gap is implicit ordering relative to 0063 and whether 0060 is genuinely a hard blocker versus already-decided context.

**Strengths**:
- Upstream blocker named explicitly: 0060 with rationale.
- Downstream consumer named: 0065.
- Related work items (0057, 0063, 0070) explicitly captured, including the note that 0070 does not block this story.
- Migration precedent (0005) named in Requirements, Technical Notes, and References.
- Specific consumer file paths enumerated.
- Assumptions explicitly addresses external-system coupling.

**Findings**:
- 🔵 minor / medium — "Coordination relationship with 0063 (sibling rename) not captured as an ordering constraint" (Dependencies): No ordering note for migration number selection or release sequencing.
- 🔵 minor / medium — "0060 listed as 'Blocked by' but may already be resolved context" (Dependencies): Story already treats canonical names as chosen; clarify whether 0060 is a live blocker.
- 🔵 minor / low — "Downstream consumers beyond 0065 are not named as Blocks" (Requirements / Dependencies): "Any helper scripts, agent prompts" is unbounded.

### Scope

**Summary**: The work item bundles two distinct field renames into a single story, with the Drafting Notes explicitly acknowledging the bundling decision as a judgement call. The renames share a risk profile and migration pattern; they could ship separately. The story's overall scope — producers, consumers, visualiser updates, and corpus migration bundled into one releasable unit — is coherent and appropriately sized for a story.

**Strengths**:
- Scope boundaries are explicit and well-articulated.
- Drafting Notes transparently acknowledges the bundling tradeoff and offers a clean split path.
- Clear demarcation against adjacent work (0070 is called out as separate workstream).
- Summary, Requirements, and Acceptance Criteria align consistently on the same scope.
- Story is sized appropriately for its declared kind.

**Findings**:
- 🔵 suggestion / medium — "Two independent renames bundled in one story" (Requirements): Logically independent concerns. Either keep as-is (current author preference) or split into 0064a/0064b sharing one migration script.
- 🔵 suggestion / medium — "Cross-reference task may be scope-adjacent" (Requirements): The "Cross-reference 0060" bullet reads as dependency coordination, not a deliverable.

### Testability

**Summary**: The Acceptance Criteria are largely concrete and verifiable — most reduce to grep-style negative assertions or fixture-checked migration behaviours. However, several criteria use unbounded language ("any", "every", "correctly") without enumerating the surfaces in scope, and the final "releasable as a single version bump" criterion lacks a defined verification procedure.

**Strengths**:
- Criteria 1, 2, and 4-6 admit concrete pass/fail procedures (source grep, fixture-based migration test, YAML shape check).
- Migration testability is well-specified — three fixture scenarios named explicitly.
- Requirements call out specific files and line numbers anchoring verification scope.

**Findings**:
- 🟡 major / high — "First two criteria use unbounded 'any' without an enumerated scope" (Acceptance Criteria): Restate as a bounded `rg` check across enumerated directories with explicit exclusions.
- 🟡 major / high — "'Visualiser renders ... correctly' is subjective and unbounded" (Acceptance Criteria): Replace with concrete route/item checks.
- 🟡 major / high — "'Releasable as a single version bump' has no defined verification procedure" (Acceptance Criteria): Replace with an executable upgrade-fixture check.
- 🔵 minor / medium — "'Any helper scripts, agent prompts, or downstream consumers' is unenumerated" (Requirements): Either enumerate or add an Acceptance Criterion grep.
- 🔵 minor / medium — "Migration idempotence has no defined verification procedure" (Acceptance Criteria): Add an explicit second-run-produces-no-diff fixture.

## Re-Review (Pass 2) — 2026-05-21T16:35:00+00:00

**Verdict:** COMMENT

The pass-1 edits resolved nearly all previously-identified issues. Only one major finding remains, and it is a residual variant of the recurring "enumerate the producer/consumer surface" theme — this time landing on "every plan-producing skill" / "any helper scripts" in Requirements rather than on the Acceptance Criteria (which are now bounded). The work item is acceptable as-is for implementation; the residual finding is worth addressing but does not block.

### Previously Identified Issues

- 🟡 **Testability**: First two criteria use unbounded "any" — **Resolved** (ACs 1–2 now specify the exact `rg` command and exclusions).
- 🟡 **Testability**: "Visualiser renders ... correctly" subjective — **Resolved** (AC 7 now lists three concrete observable checks: kanban listing, plan detail view, research/RCA listings).
- 🟡 **Testability**: "Releasable as a single version bump" no defined verification — **Resolved** (AC 8 now spells out the migrate→upgrade procedure and three post-conditions).
- 🔵 **Clarity**: RCA acronym used without definition — **Resolved** (defined in Summary; minor pluralisation inconsistency surfaced as a new suggestion).
- 🔵 **Clarity**: "renames are independent" ambiguous referent — **Resolved** (reworded to "logically independent but intentionally bundled").
- 🔵 **Clarity**: Summary scope narrower than Requirements — **Resolved** (Summary now names helper scripts, agent prompts, frontend, and beneficiaries).
- 🔵 **Clarity**: "any frontend types or components" vague — **Resolved** (three frontend files enumerated inline in Requirements).
- 🔵 **Clarity**: "configured corpus" undefined — **Resolved** (AC 3 now says "paths resolved by the migrate skill").
- 🔵 **Completeness**: No Open Questions section — **Resolved** (section added, explicitly states none remain).
- 🔵 **Completeness**: Beneficiary not identified — **Resolved** (Summary names skill authors, visualiser, helper scripts, userspace projects).
- 🔵 **Dependency**: 0063 ordering not captured — **Partially resolved** (now noted in Related as "already landed; sequences after"; a new dependency finding requests promoting this to Dependencies as an explicit ordering constraint).
- 🔵 **Dependency**: 0060 may already be resolved — **Resolved** (moved from "Blocked by" to "Related" with "already settled" annotation).
- 🔵 **Dependency**: Downstream consumers beyond 0065 not named — **Partially resolved** (Requirements now references the bounded grep, but specific helper scripts/agent prompts and the migrate skill itself still not enumerated).
- 🔵 **Testability**: "Any helper scripts" unenumerated — **Partially resolved** (the verification is bounded by AC 1's grep; the producing-skill surface in Requirements remains unbounded — see new major finding).
- 🔵 **Testability**: Migration idempotence has no defined verification — **Resolved** (new `fully-migrated` fixture + explicit "no diff on second invocation").
- 🔵 **Scope**: Two independent renames bundled — **Still present (deliberate)** (acknowledged trade-off; user chose to keep bundled).
- 🔵 **Scope**: Cross-reference task scope-adjacent — **Resolved** (Requirements bullet removed; 0060 is in Dependencies/Related).

### New Issues Introduced

- 🟡 **Testability**: "every plan-producing skill", "every producing skill", and "any helper scripts or agent prompts" in Requirements remain unbounded. AC 1's grep bounds verification, but the producing-skill set itself is not enumerated, so an implementer cannot tell up front which producers must be edited. Suggestion: enumerate the producing skills in Requirements (a short bullet list) or add an AC that runs a defined enumeration like "`rg -l 'work-item' skills/` returns only files in an enumerated allow-list".
- 🔵 **Clarity**: AC 8's upgrade-path procedure reads "migrate, then upgrade" but the intent is "upgrade, then migrate". Restate as a numbered 4-step sequence so the order has exactly one interpretation.
- 🔵 **Clarity**: The Requirements helper-scripts bullet says "the consumer surface is bounded by the directories enumerated there" — this is circular (the AC's directories were chosen to match the consumer surface). Define the consumer-search scope directly in Requirements (e.g., scripts/, commands/, skills/**/SKILL.md) instead.
- 🔵 **Clarity**: "pre-existing `meta/` artifacts" exclusion clause in ACs 1–2 is undefined (pre-this-story? pre-rename commit?). Since the rg command already excludes `meta/` from search roots, the exclusion clause is redundant — simplify or pin to a concrete reference point.
- 🔵 **Testability**: "the detail view for at least one plan (e.g. plan 0063)" in AC 7 gives only a sampling guarantee. Pin to a specific plan or strengthen to "no plan detail page shows a broken-link badge".
- 🔵 **Testability**: AC 7 says "all previously-visible work items" but no pre-migration snapshot is captured. Define an expected count or list.
- 🔵 **Testability**: AC 8 references "a fixture repo at the pre-rename plugin version" without defining the fixture. Reference a named fixture path or describe its contents (one plan with `work-item:`, one research doc with `researcher:`, one userspace override).
- 🔵 **Dependency**: The migrate skill (`skills/config/migrate/`) is exercised by AC 8 but not listed as a coupled component in Dependencies. Add a line noting the coupling to its path-resolution and override semantics.
- 🔵 **Dependency**: The 0063 ordering relationship is noted in Related but reads more like a sequencing constraint; consider promoting to a "Depends on (ordering)" line in Dependencies.

### Assessment

The work item is **ready for implementation**. All three major findings from pass 1 are resolved; the single new major is a residual variant of the same enumeration theme and is addressable with a short bullet list of producing skills (or an additional grep-based AC). The remaining minor and suggestion findings are polish — they would tighten the verification procedure but do not change what is being built or how it ships. Recommended polish (in priority order):

1. Enumerate the producing skills in Requirements (closes the residual major).
2. Renumber AC 8 as a 4-step procedure with explicit "upgrade then migrate" ordering.
3. Pin the visualiser smoke-test plan (AC 7) and the upgrade-path fixture (AC 8) to concrete identifiers.
4. De-circularise the helper-script Requirements bullet by listing search directories directly.
5. Drop or pin the "pre-existing meta/ artifacts" exclusion clause in ACs 1–2.

## Re-Review (Pass 3) — 2026-05-21T16:50:00+00:00

**Verdict:** APPROVE

All polish items from pass 2 applied:

1. **Producing skills enumerated** — Requirements now names `skills/planning/create-plan/SKILL.md` (plan producer) and `skills/research/research-codebase/SKILL.md` + `skills/research/research-issue/SKILL.md` (research/RCA producers), closing the residual major.
2. **AC 8 restated as 4-step sequence** — Now reads (1) start at pre-rename version, (2) upgrade plugin, (3) run migrate, (4) verify — ordering has exactly one interpretation.
3. **AC 7 plan pinned** — Now reads "plan detail view for plan 0063 displays its parent work-item link to 0057" plus the stronger "no plan detail page shows a broken-link badge"; AC 8 fixture pinned to a named path under `skills/config/migrate/scripts/test-fixtures/<NNNN>-canonicalise-work-item-id-and-author/upgrade-path/` with required contents enumerated.
4. **Helper-script Requirements bullet de-circularised** — Now states the consumer search scope directly (`scripts/`, `skills/**/SKILL.md`, `skills/**/scripts/`) rather than referring forward to AC 1.
5. **"Pre-existing meta/" exclusion clause simplified** — ACs 1–2 now use search roots that already exclude `meta/` and `skills/config/migrate/migrations/`; the exclusion rationale is preserved as a parenthetical note rather than a redundant clause.

### Assessment

The work item is **ready for implementation**. All majors are resolved; remaining minors are minor enough to address during implementation or post-merge. Approving.
