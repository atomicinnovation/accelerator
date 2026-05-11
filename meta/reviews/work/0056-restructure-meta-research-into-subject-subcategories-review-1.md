---
date: "2026-05-11T21:29:22+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0056-restructure-meta-research-into-subject-subcategories.md"
work_item_id: "0056"
review_number: 1
verdict: COMMENT
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
---

## Work Item Review: Restructure meta/research/ into Subject Subcategories

**Verdict:** REVISE

The work item is exceptionally complete and well-structured: every template
section is substantively populated, the story-vs-epic decision is defended on
atomicity grounds, and Technical Notes provide file-level pointers (with line
numbers) for every edit site. The two areas needing revision before
implementation are (1) acceptance-criteria coverage — five Requirements
behaviours have no matching AC and several existing ACs leave verification
signals unpinned — and (2) the Dependencies section, which reads `Blocked
by: none` while the body itself names an in-flight prerequisite (uncommitted
modifications to 0030/0052) and an implicit ordering constraint between the
in-tree plugin migration and the userspace migration shipping.

### Cross-Cutting Themes

- **Acceptance criteria coverage gaps** (flagged by: testability × 5,
  clarity, scope) — multiple lenses noted that several Requirements (paths
  skill surfaces five keys, configure paths-table updated, in-tree plugin
  application, `scripts/research-metadata.sh` audit, README narrative
  prose / ASCII diagram updates, new keys' default values, VCS rename-history
  preservation, default-value migration path) have no matching AC, and that
  some existing ACs (refuses to run, reports no pending changes, every
  reference rewritten) do not pin the observable signal.
- **Dependencies under-represented in the Dependencies section** (flagged
  by: dependency × 4) — the in-flight 0030/0052 modifications, the in-tree
  ↔ userspace ordering constraint, the three exemplar migrations
  (0001/0002/0003) the new migration combines, and the conditional
  framework-capability assumption are all visible in the body but absent
  from the Dependencies section that planners read.
- **Compound criterion readability** (flagged by: clarity, testability) —
  AC4 (inbound-reference rewriting) packs corpus, match rule, coverage
  shapes, and a negative-control invariant into a single sentence; splitting
  improves both readability and testability.

### Findings

#### Major
- 🟡 **Dependency**: Uncommitted in-flight modifications to 0030 and 0052 not captured as blockers
  **Location**: Dependencies
  `Blocked by: none` contradicts the parenthetical "uncommitted modifications to those files should land before this work starts". The git status confirms both files are modified locally. Replace with an explicit blocker entry or land the in-flight commits first.

- 🟡 **Dependency**: Ordering constraint between in-tree plugin migration and userspace migration not captured
  **Location**: Dependencies / Technical Notes
  Technical Notes implies an internal ordering — the plugin's own `meta/` must restructure in the same commit that ships the migration, because the migration is modelled on the plugin layout. Without this captured in Dependencies, an implementer could split the work into commits that leave the plugin's own skills unable to resolve their output paths.

- 🟡 **Testability**: Reference-rewrite criterion uses unbounded "every such reference" over a user-customisable corpus
  **Location**: Acceptance Criteria (AC4)
  Both the scan corpus and the legacy/new path values are runtime-resolved, so a tester has no fixed fixture to verify against. The "leaving non-matching content byte-identical" invariant has no defined negative-control corpus.

- 🟡 **Testability**: Silent-rewrite notification requirement only partially covered by acceptance criteria
  **Location**: Requirements (Migration) ↔ Acceptance Criteria
  AC3 verifies the notification format when customised values are present, but there is no criterion for the default-value path (likely the majority case): does the notification still fire? Are keys still renamed?

- 🟡 **Testability**: New keys' default values not pinned in acceptance criteria
  **Location**: Requirements (Configuration changes) ↔ Acceptance Criteria
  Requirements state the new keys are added "with default values" but no AC pins what those defaults should be. An implementation could ship `paths.research_ideas = meta/random/` and pass.

- 🟡 **Testability**: VCS rename-history preservation has no acceptance criterion
  **Location**: Requirements (Directory restructure) ↔ Acceptance Criteria
  AC2 only asserts that files land in their new locations, which is also true of a delete-and-add implementation. The stated reason for using move semantics has no verification gate.

- 🟡 **Testability**: Several Requirements lack matching acceptance criteria
  **Location**: Requirements ↔ Acceptance Criteria
  Uncovered: `accelerator:paths` surfaces five new keys; `accelerator:configure` paths-table updated; in-tree plugin migration; `scripts/research-metadata.sh` audit; README narrative prose / ASCII diagram updates (only the README directory table is covered by AC10).

#### Minor
- 🔵 **Clarity**: AC4 (inbound-reference rewriting) is densely nested and hard to parse
  **Location**: Acceptance Criteria
  Four parenthetical config-resolution clauses, a coverage list, a corpus definition, and a byte-identical invariant in one sentence. Splitting improves both readability and verifiability.

- 🔵 **Clarity**: "If framework additions are needed, scope expands" leaves trigger and consequence vague
  **Location**: Assumptions
  Unclear who decides, what "scope expands" means operationally, and which artefact captures that decision.

- 🔵 **Clarity**: "Silent rewrite with a per-key one-line notification" is internally contradictory at first read
  **Location**: Requirements: Migration via accelerator:migrate
  "Silent" and "notification" conflict until the reader infers "silent" means "no prompt". Reword as "non-interactive rewrite (no prompt) with a per-key one-line stdout notification".

- 🔵 **Clarity**: In-tree plugin migration mechanism is ambiguous
  **Location**: Technical Notes
  Unclear whether the plugin's own `meta/` is reorganised by running the migration script against the plugin checkout, or by hand-applying equivalent moves in the same commit.

- 🔵 **Dependency**: Migration-pattern exemplars (0001, 0002, 0003) not surfaced in Dependencies
  **Location**: Dependencies
  These are upstream code dependencies the new migration cannot be implemented without — currently only mentioned in Technical Notes.

- 🔵 **Dependency**: Implicit dependency on migrate framework capability not captured as risk-bearing coupling
  **Location**: Requirements (Migration) / Assumptions
  The framework-capability assumption lives in Assumptions but is not echoed in Dependencies; a planner committing on "Blocked by: none" may not see that a framework gap could materially change size.

- 🔵 **Dependency**: Downstream consumer named without work item identifier
  **Location**: Dependencies: Blocks
  `Blocks: forthcoming idea/concept research skill` has no work item ID — the link is one-way and a search for what depends on 0056 will miss it.

- 🔵 **Scope**: `paths.research_ideas` and `meta/research/ideas/` prepared for explicitly out-of-scope skill
  **Location**: Requirements: Directory restructure
  A small slice of the work has no consumer until a future work item lands, weakening the atomicity argument for those specific lines. Either tighten the rationale in Context or defer to the idea/concept skill's own work item.

- 🔵 **Testability**: "Sourced from accelerator:paths rather than inline directory map" is not externally observable
  **Location**: Acceptance Criteria (documents-locator grouping)
  The sourcing claim is an implementation detail — verifiable only by code inspection, not by behavioural test.

- 🔵 **Testability**: Unresolved open question affects a verifiable outcome
  **Location**: Open Questions
  Whether `meta/research/ideas/` is default-created post-migration is observable behaviour that cannot currently be tested against the spec.

- 🔵 **Testability**: Idempotency criterion uses "reports no pending changes" without defining the signal
  **Location**: Acceptance Criteria (idempotency)
  Pin the signal: exit code, specific message, or empty preview.

- 🔵 **Testability**: "Refuses to run" lacks defined refusal signal
  **Location**: Acceptance Criteria (dirty working tree)
  Pin the refusal: non-zero exit, message naming dirty paths, no filesystem changes.

#### Suggestions
- 🔵 **Clarity**: "jj/git move semantics" assumes the reader knows what distinguishes a move from delete+add in jj
  **Location**: Requirements: Directory restructure
  Link to jj rename-tracking docs or the existing migration helper (e.g., `_move_if_pending` from 0003) so the reader knows where to look.

- 🔵 **Clarity**: "design-convergence outputs" is used without definition
  **Location**: Context
  Add a brief parenthetical on first use, e.g. "design-convergence outputs (the design-inventories/ and design-gaps/ artifacts)".

- 🔵 **Clarity**: "three-phase inbound-link rewriter" named without a phase breakdown
  **Location**: Technical Notes
  Three phases vs four coverage shapes — spell out the mapping or drop "three-phase".

- 🔵 **Scope**: Size L work item touching 9+ files plus the most complex migration shipped — verify this is the right unit
  **Location**: Technical Notes: Size
  Sanity check rather than structural concern. Confirm during planning that the in-tree plugin rename and userspace migration code are sequenced atomically.

### Strengths
- ✅ Summary, Requirements, and Acceptance Criteria all describe one unified restructure with no section drift
- ✅ Subcategory names and path key names map 1:1, eliminating ambiguity
- ✅ Acceptance Criteria use Given/When/Then with named actors (accelerator:migrate, documents-locator, research-codebase, research-issue)
- ✅ Explicit Out of Scope section bounds the work
- ✅ Drafting Notes explicitly defend the story-vs-epic decision on atomicity grounds
- ✅ Per-category file-vs-directory policy stated as a discrete table-like list
- ✅ Technical Notes provide file-level pointers (with line numbers) for every edit site
- ✅ Notification format pinned to an exact string template
- ✅ Idempotency and dirty-tree refusal have dedicated criteria
- ✅ All template sections substantively populated; frontmatter integrity intact
- ✅ Upstream blockers 0030/0052 and downstream idea/concept skill explicitly captured
- ✅ Related-but-not-blocking items 0021/0022 surfaced as Related, not mis-classified as blockers

### Recommended Changes

1. **Reform Dependencies into a planner-readable coupling list** (addresses: in-flight 0030/0052 modifications; in-tree↔userspace ordering; exemplar migrations 0001/0002/0003; framework-capability conditional; idea/concept skill ID)
   Replace `Blocked by: none (parenthetical)` with explicit entries; add an Ordering note stating the in-tree directory moves, config-key rename/additions, skill/agent updates, and migration code must all land in a single atomic commit; surface 0001/0002/0003 as code dependencies; reference the idea/concept skill's work-item ID (or "TBD") in `Blocks:`.

2. **Add missing acceptance criteria** (addresses: paths-skill surfaces five keys; configure paths-table updated; in-tree plugin migration; research-metadata.sh audit; README narrative/ASCII updates; new keys' default values; VCS rename-history preservation; default-value migration path)
   For each uncovered requirement, add a Given/When/Then AC. Specifically: default values for all five new keys; `jj log --follow <new-path>` shows pre-migration commits; `accelerator:paths` block lists the five new keys; configure help renders the new key set; README narrative + ASCII diagram updated alongside the table; default-value migration emits the same notification line.

3. **Split AC4 into per-shape sub-criteria with a defined fixture** (addresses: unbounded "every such reference"; AC4 density)
   Replace the single dense AC4 with three or four ACs — one for the scan corpus (config-resolved document paths), one for the match-and-rewrite rule, and per-shape coverage ACs (markdown links, frontmatter scalars, frontmatter list entries, prose mentions). Reference a fixture set of files with named example inputs and expected outputs.

4. **Pin observable signals on remaining ACs** (addresses: idempotency report; dirty-tree refusal; documents-locator sourcing)
   Idempotency: specify exit code and message. Dirty-tree refusal: specify exit code, message naming dirty paths, no filesystem changes. Documents-locator: split into observable behaviour (group labels match `accelerator:paths` keys) and inspection check (agent definition contains no inline directory map).

5. **Tighten three clarity issues** (addresses: silent-rewrite contradiction; framework-additions vagueness; in-tree plugin mechanism)
   - Reword "silent rewrite with a per-key one-line notification" → "non-interactive rewrite (no prompt) with a per-key one-line stdout notification"
   - Restate the framework-additions Assumption with explicit trigger and response
   - State explicitly whether the plugin's `meta/` restructure is hand-applied in the same commit or run via the migration script

6. **Resolve the Open Question and rationalise `paths.research_ideas`** (addresses: out-of-scope-skill pre-staging; unresolved open question)
   Confirm whether `meta/research/ideas/` is default-created; add the corresponding AC if yes; either tighten the Context to justify pre-staging the ideas key, or defer it to the idea/concept skill's own work item.

7. **Minor clarity polish** (addresses: jj/git move semantics; design-convergence term; three-phase rewriter)
   Link or name-drop the move helper; gloss "design-convergence outputs" on first use; align the rewriter phase/shape terminology between Requirements, AC, and Technical Notes.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear with well-defined directory names, path keys, and acceptance criteria that name the actor (migration, skill, agent). However, a few referent ambiguities exist around 'this work' vs framework, the meaning of 'inbound references' in one criterion is dense to parse, and a couple of terms (jj move semantics, 'three-phase inbound-link rewriter') assume reader familiarity without a link.

**Strengths**:
- Subcategory names and path key names map 1:1, eliminating ambiguity about which key corresponds to which directory.
- Acceptance Criteria use Given/When/Then form with named actors (accelerator:migrate, documents-locator, research-codebase, research-issue), keeping responsibility unambiguous.
- Out of Scope section explicitly lists adjacent concerns (other meta/ dirs, meta/strategy/, idea/concept skill build) so the boundary of 'this work' is unmistakable.
- Per-category file-vs-directory policy is stated as a discrete table-like list, removing ambiguity about which subcategories are flat vs nested.

**Findings**:
- 🔵 minor / high confidence — Inbound-references acceptance criterion is densely nested and hard to parse (Acceptance Criteria)
- 🔵 minor / medium confidence — "If framework additions are needed, scope expands" leaves the trigger and consequence vague (Assumptions)
- 🔵 minor / medium confidence — "Silent rewrite with a per-key one-line notification" is internally contradictory at first read (Requirements: Migration)
- 🔵 minor / medium confidence — "The plugin repo itself will need an in-tree application of the same migration" is ambiguous about what runs (Technical Notes)
- 🔵 suggestion / medium confidence — "jj/git move semantics" assumes the reader knows what distinguishes a move from delete+add in jj (Requirements: Directory restructure)
- 🔵 suggestion / low confidence — "design-convergence outputs" is used without definition (Context)
- 🔵 suggestion / low confidence — "three-phase inbound-link rewriter" is named without a phase breakdown (Technical Notes)

### Completeness

**Summary**: The work item is exceptionally thorough across all completeness dimensions: Summary, Context, Requirements, Acceptance Criteria, Open Questions, Dependencies, Assumptions, Technical Notes, Drafting Notes, and References are all present and substantively populated. Type-appropriate story content is well-formed — the actor (Accelerator plugin author) is named, motivation is articulated, and acceptance criteria are specific and given/when/then-structured. Frontmatter is complete and uses recognised values.

**Strengths**:
- Summary is a clear, unambiguous story statement that names the actor, the change, and three concrete motivations.
- Context explains why the work is needed now (forcing function: the forthcoming idea/concept research skill) and references prior deferral reasoning.
- Requirements are decomposed into named sub-sections with concrete bullets in each.
- Acceptance Criteria contains 10 distinct Given/When/Then criteria.
- Dependencies, Assumptions, and Open Questions are all populated with substantive content rather than placeholders.
- Technical Notes provides file-level pointers (with line numbers) for every edit site.
- References section lists source note, prior work items, and target files.
- Frontmatter integrity is intact.

**Findings**: (none)

### Dependency

**Summary**: The work item captures its primary upstream blockers (0030, 0052) accurately and names the forthcoming idea/concept skill as a downstream consumer. However, the Dependencies section under-represents several couplings that are explicitly named or implied elsewhere in the body: the uncommitted in-flight modifications to 0030 and 0052 are mentioned but not formalised as blockers, the three exemplar migrations (0001/0002/0003) whose patterns must be combined are only mentioned in Technical Notes, and a known cross-cutting ordering constraint (in-tree plugin application must precede userspace migration shipping) is implied but not stated.

**Strengths**:
- Upstream blockers 0030 and 0052 are explicitly named in Dependencies with their done status correctly reflected.
- Downstream consumer captured as a Blocks entry, matching the forcing-function narrative in Context.
- Related-but-not-blocking items 0021 and 0022 surfaced as Related rather than mis-classified as blockers.
- Cross-component lockstep couplings catalogued in Technical Notes.

**Findings**:
- 🟡 major / high confidence — Uncommitted in-flight modifications to 0030 and 0052 not captured as blockers (Dependencies)
- 🟡 major / high confidence — Ordering constraint between in-tree plugin migration and userspace migration not captured (Dependencies)
- 🔵 minor / medium confidence — Migration-pattern exemplars (0001, 0002, 0003) not surfaced in Dependencies (Dependencies)
- 🔵 minor / medium confidence — Implicit dependency on migrate framework capability not captured as risk-bearing coupling (Requirements: Migration)
- 🔵 minor / low confidence — Downstream consumer named without work item identifier (Dependencies: Blocks)

### Scope

**Summary**: Work item 0056 describes a tightly-coupled directory restructure plus migration that must ship as one unit to avoid leaving userspace repos in a broken state. The Summary, Requirements, and Acceptance Criteria all describe the same scope — a single coherent restructuring of meta/research/ — and the Drafting Notes explicitly justify the story-over-epic choice on atomicity grounds. The work item is large (L) and crosses several files, but the parts are genuinely indivisible; the main scope risk is a single Requirements bullet that overlaps with explicitly out-of-scope future work.

**Strengths**:
- Summary, Requirements, Acceptance Criteria, and Drafting Notes consistently describe one unified restructure — no section drift
- Explicit Out of Scope section bounds the work
- Drafting Notes explicitly defend the story-vs-epic decision on indivisibility grounds
- Dependencies block lists prerequisite work (0030, 0052) as already done

**Findings**:
- 🔵 minor / medium confidence — `paths.research_ideas` key and `meta/research/ideas/` directory prepared for explicitly out-of-scope skill (Requirements: Directory restructure)
- 🔵 suggestion / low confidence — Size L work item touching 9+ files plus the most complex migration shipped — verify this is the right unit (Technical Notes: Size)

### Testability

**Summary**: The work item provides a strong set of mostly testable Given/When/Then acceptance criteria covering fresh-install configuration, migration behaviour, idempotency, dirty-tree refusal, agent grouping, and skill output paths. A few criteria use unbounded language ('every such reference', 'all five subcategories') without enumerating the verification scope, and several requirements (silent-rewrite notification format, in-tree plugin migration, scripts/research-metadata.sh audit, README narrative prose updates) lack matching acceptance criteria, leaving verification gaps.

**Strengths**:
- Most acceptance criteria use explicit Given/When/Then framing with concrete observable outcomes.
- Notification format is pinned to an exact string template.
- Idempotency and dirty-working-tree refusal have dedicated criteria.
- Acceptance criteria cover both fresh-install state and migration-from-legacy state.
- The reference-rewriting criterion enumerates the four reference shapes.

**Findings**:
- 🟡 major / high confidence — Reference-rewrite criterion uses unbounded 'every such reference' over a user-customisable corpus (Acceptance Criteria: AC4)
- 🟡 major / high confidence — Silent-rewrite notification requirement only partially covered by acceptance criteria (Requirements ↔ Acceptance Criteria)
- 🟡 major / high confidence — New keys' default values not pinned in acceptance criteria (Requirements ↔ Acceptance Criteria)
- 🟡 major / medium confidence — VCS rename-history preservation has no acceptance criterion (Requirements ↔ Acceptance Criteria)
- 🟡 major / medium confidence — Several requirements lack matching acceptance criteria (Requirements ↔ Acceptance Criteria)
- 🔵 minor / high confidence — 'Sourced from accelerator:paths rather than inline directory map' is not externally observable (Acceptance Criteria)
- 🔵 minor / medium confidence — Unresolved open question affects a verifiable outcome (Open Questions)
- 🔵 minor / medium confidence — Idempotency criterion uses 'reports no pending changes' without defining the signal (Acceptance Criteria)
- 🔵 minor / medium confidence — 'Refuses to run' lacks defined refusal signal (Acceptance Criteria)


## Re-Review (Pass 2) — 2026-05-11T21:29:22+00:00

**Verdict:** COMMENT

### Previously Identified Issues

#### Major (pass 1) — all resolved
- ✅ **Dependency**: Uncommitted in-flight modifications to 0030 and 0052 not captured as blockers — Resolved (reframed: changes will land in same commit as this work, no separate blocker needed)
- ✅ **Dependency**: Ordering constraint between in-tree plugin migration and userspace migration not captured — Resolved (Dependencies > Ordering section now explicitly states the atomic-commit requirement)
- ✅ **Testability**: Reference-rewrite criterion uses unbounded "every such reference" — Resolved (AC4 split into three criteria: scan corpus, match rule, per-shape coverage)
- ✅ **Testability**: Silent-rewrite notification only partially covered — Resolved (new absent-keys AC covers the default-value case)
- ✅ **Testability**: New keys default values not pinned — Resolved (new AC pins each default explicitly)
- ✅ **Testability**: VCS rename-history preservation has no AC — Resolved (new AC requires `jj log --follow` / `git log --follow` to show pre-migration commits)
- ✅ **Testability**: Several requirements lack matching ACs — Resolved (new ACs for paths-skill output, configure paths-table, plugin meta tree, research-metadata.sh audit, README narrative/ASCII)

#### Minor / suggestion (pass 1) — selectively addressed
- ✅ **Clarity**: AC4 densely nested — Resolved (split into three)
- ✅ **Clarity**: "Silent rewrite" contradiction — Resolved (reworded as "non-interactive rewrite (no prompt) with a per-key one-line stdout notification")
- ✅ **Clarity**: In-tree plugin migration ambiguity — Resolved (Technical Notes now explicit: hand-applied in same commit, script only runs in userspace)
- ✅ **Clarity**: jj/git move semantics — Resolved (now references `_move_if_pending` helper pattern with file:line)
- ✅ **Clarity**: "design-convergence outputs" undefined — Resolved (glossed inline on first use)
- ✅ **Clarity**: "three-phase rewriter" terminology — Resolved (dropped "three-phase", aligned with four coverage shapes)
- ✅ **Dependency**: Exemplar migrations 0001/0002/0003 not in Dependencies — Resolved (now under "Builds on (code, not work items)")
- ✅ **Dependency**: Framework-capability conditional not captured — Resolved (Assumption reframed: any new primitive ships in-scope here)
- ✅ **Dependency**: Downstream consumer without work item ID — Resolved (Blocks entry now says "work item TBD — when drafted, its `Blocked by` should reference 0056")
- ✅ **Scope**: `paths.research_ideas` for out-of-scope skill — Resolved (key and directory removed from this work item entirely; deferred to idea/concept skill's own work item)
- ✅ **Testability**: Documents-locator sourcing not externally observable — Resolved (split into behavioural AC + inspection AC)
- ✅ **Testability**: Open question affected verifiable outcome — Resolved (question closed; ideas/ infrastructure deferred)
- ✅ **Testability**: Idempotency signal undefined — Resolved (exit 0, no preview line, state-file recorded)
- ✅ **Testability**: "Refuses to run" signal undefined — Resolved (exit non-zero, message naming dirty paths, no changes)
- ⏸ **Clarity**: "Framework additions" trigger and consequence — Partially addressed (Assumption reframed to absorb framework additions in-scope; "trigger" still implicit but lower-stakes now)
- ⏸ **Scope**: Size L sanity check — Acknowledged in pass 1, no structural change needed

### New Issues Introduced

#### Major
- 🟡 **Testability**: Fixture corpus referenced but not defined or located (Acceptance Criteria) — The new per-shape coverage AC gates on "a fixture corpus containing reference shapes" but does not name where the fixture lives or specify minimum coverage. Consider naming a fixture path (e.g. `skills/config/migrate/migrations/0004-*/fixtures/`) and the minimum examples per shape × key combination.

#### Minor
- 🔵 **Clarity**: Stale "five new keys" / "two additions are appended rows" references in Technical Notes (paths/SKILL.md edit bullet and PATH_KEYS bullet) — leftover from the scope reduction that removed `paths.research_ideas`. Should be "four new keys" and "one addition".
- 🔵 **Dependency**: Intra-migration ordering not specified (Acceptance Criteria) — When the scan corpus is computed from `accelerator:paths`, and `accelerator:paths` reflects the renamed keys, the migration must order (a) scan-corpus computation from old-key paths, (b) directory moves, (c) inbound-link rewriting, (d) config-key rewriting carefully. Worth a Technical Notes bullet.
- 🔵 **Testability**: Documents-locator subcategory labels use "e.g." making them illustrative rather than required strings — A stricter reader could fail or pass any label format. Either drop "e.g." or specify a construction rule.
- 🔵 **Dependency**: ADR-0023 (meta-directory-migration-framework) not in References, even though it underpins the migration framework being extended.
- 🔵 **Dependency**: Test harness coupling not captured — Which test file/harness this work extends is not stated; the fixture-corpus AC implies test infrastructure exists.
- 🔵 **Clarity**: "its inbound-link scan corpus" antecedent slightly ambiguous; "scan corpus" coined term not defined inline.
- 🔵 **Testability**: "Names the dirty paths" threshold (one path vs all) unstated.
- 🔵 **Testability**: README narrative AC uses unbounded "all narrative prose, tables, and ASCII diagrams" — consider grep-style invariant or pin to Technical Notes line ranges.
- 🔵 **Testability**: `accelerator:paths` configured-paths block layout (ordering, presence of unrelated keys) not pinned in the new AC.
- 🔵 **Testability**: `jj/git log --follow` outcome lacks a deterministic pass/fail rule (heuristic rename detection).
- 🔵 **Testability**: Inbound-link match algorithm (tokenisation, code-fence exclusion, prefix-match handling) not defined.
- 🔵 **Scope**: `paths.research_issues` (new key) bundled with renames — author's deliberate choice; suggestion-level only.

### Assessment

The work item is now in good shape for implementation. All seven major findings from pass 1 are resolved; the surviving 1 major + 11 minors are tightenable polish items rather than structural blockers. The most actionable pre-implementation fixes are (a) the stale "five new keys" / "two additions" counts in Technical Notes (real bug from the scope-reduction edit), and (b) defining or locating the fixture corpus referenced by the per-shape coverage AC. Everything else is COMMENT-grade — improve if convenient, otherwise safe to plan against.
