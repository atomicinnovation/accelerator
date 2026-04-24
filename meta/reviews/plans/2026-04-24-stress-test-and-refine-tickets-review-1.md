---
date: "2026-04-25T00:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-24-stress-test-and-refine-tickets.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, test-coverage, correctness, usability, documentation, safety, standards]
review_pass: 3
status: complete
---

## Plan Review: Stress-Test and Refine Tickets — Phase 6

**Verdict:** REVISE

The plan is well-structured, grounded in established conventions, and makes good use of the TDD-via-eval loop that earlier phases validated. The stress-test-ticket subphase is near-ready: it mirrors stress-test-plan faithfully and needs only small prohibition-list tightening and eval-observability reworks. The refine-ticket subphase, by contrast, bundles five heterogeneous operations under one dispatcher and carries a cluster of under-specified interactions (multi-operation ordering, Technical Notes co-ownership, decompose clobbering parent Requirements, child frontmatter derivation rules, abort-recovery UX) that will cause skill-creator to produce a different SKILL.md on each iteration. No finding is a blocker, but the volume of major concerns concentrated in refine-ticket — plus two correctness issues where the plan's claims about sibling skills don't match their actual behaviour — warrant revision before handoff.

### Cross-Cutting Themes

- **Hierarchy rendering claim doesn't match list-tickets** (flagged by: correctness, architecture, usability, documentation, standards) — Scenario 13 and Step 5 prescribe `├──`/`└──` box-drawing characters and claim parity with `/list-tickets`, but `skills/tickets/list-tickets/SKILL.md` never specifies those glyphs — it only says children "appear indented beneath their parent". Five lenses independently flagged this. Either pin the exact format in refine-ticket standalone, or update list-tickets first and reference it.
- **Technical Notes is co-owned by enrich and size with no ordering/conflict rule** (flagged by: architecture, correctness, usability, safety) — four lenses independently flagged that Step 4 says operations run "in order" without defining order, and two operations target the same section. Multi-operation eval coverage is also missing.
- **Eager ticket-number consumption lacks UX transparency and recovery guidance** (flagged by: architecture, usability, safety) — plan commits to "no rollback" but doesn't require the skill to warn before allocation, report consumed numbers on abort, or point users to `jj restore`. Ticket-number allocation is also not atomic across concurrent sessions.
- **Decompose writes child links into parent's Requirements section** (flagged by: architecture, correctness, safety, documentation) — Requirements is the ticket's narrative content, not a structural child index. Parent→child relationships are already expressed via `parent:` frontmatter. Mixing them creates two sources of truth and risks clobbering user-authored content. Scenario 6's ambiguous "or a new Tasks subsection" alternative compounds the issue.
- **Child frontmatter derivation rules incomplete or inaccurate** (flagged by: correctness, documentation, standards) — Scenario 7 claims `author` resolves from `config-read-context.sh`, but that script outputs project context, not identity. `date` format, `tags` "where applicable", and `priority` ask-once-per-session vs per-child are all under-specified.
- **Negative and failure-mode eval coverage is thin** (flagged by: test-coverage, usability, correctness, safety) — missing: bad/missing path, malformed frontmatter, abort-mid-decompose post-condition, link-with-zero-candidates, size-called-twice idempotency, bug/spike decomposition challenge, enrich-with-no-agent-results.
- **Several evals describe behaviours not deterministically observable in a single-turn harness** (flagged by: test-coverage) — parallel agent spawning, "waits for user answer", depth-first follow-up, and exact tree indentation all need multi-turn simulation or explicit pinned-output assertions to function as real regression guards.
- **Multi-operation execution order undefined** (flagged by: architecture, correctness, usability, safety) — Step 3 allows "one, multiple, or 'all relevant'" and Step 4 says "in order", without defining order. Interacts with Technical Notes co-ownership and Step 5/Step 6 sequencing.

### Tradeoff Analysis

- **Architecture (extract shared helpers) vs. Consistency with list-tickets (inline prose)** — Architecture lens flags that hierarchy rendering and child-ticket population duplicate existing logic in prose; correctness lens flags that the duplication isn't even faithful to its claimed source. Either extract into shared scripts (e.g. `ticket-render-hierarchy.sh`, `ticket-write-child.sh`) and update list-tickets to use them, or acknowledge the prose duplication with a trigger condition for future extraction. The current "no new scripts" constraint is an asserted position, not a reasoned one.
- **Safety (diff-preview + explicit confirmation on destructive edits) vs. Usability (fewer friction steps)** — Safety lens wants a diff + second confirmation before enrich's "replace" mode and for repeat link invocations. Usability lens doesn't object but notes that the menu already feels committal. Recommendation: require diff-preview only for destructive paths (replace/clobber), keep append/skip paths friction-free.

### Findings

#### Critical

_(none)_

#### Major

- 🟡 **Correctness**: config-read-context.sh cannot resolve author — it outputs project context, not identity
  **Location**: Subphase 6B, Scenario 7 and Step 4a (author resolution)
  Scenario 7 states child `author` resolves from `config-read-context.sh`, but that script emits a `## Project Context` block, never an identity. Replace with create-ticket's chain (parent → config → git/jj identity → ask).

- 🟡 **Correctness**: Claimed `/list-tickets` tree format with `├──` / `└──` does not exist in list-tickets
  **Location**: Subphase 6B, Scenario 13 and Step 5
  `list-tickets/SKILL.md` only says children "appear indented beneath their parent" — no box-drawing glyphs specified. Either pin the format in refine-ticket standalone or update list-tickets first.

- 🟡 **Correctness**: enrich and size both write to Technical Notes with no ordering/merge rule
  **Location**: Subphase 6B, Scenario 10 / Step 4d vs Step 4b
  Important Guidelines says "enrich owns Technical Notes; size owns one line in Technical Notes" — two operations own the same section. No canonical order, no eval for multi-operation selection.

- 🟡 **Correctness**: Multi-operation execution order is under-specified
  **Location**: Subphase 6B, Step 3 and Scenario 4
  "User selects one or more; skill processes them in order" — "in order" is undefined (selection, menu, canonical?). Needs a pinned sequence plus an eval.

- 🟡 **Correctness**: Step 5 "after decompose only" vs Step 6 "after refinement" not sequenced for combined operations
  **Location**: Subphase 6B, Steps 5 and 6
  If user picks decompose + enrich + sharpen, when does the hierarchy display run and what does Step 6's review offer target?

- 🟡 **Safety**: Stress-test do-NOT-modify list omits `title`, `author`, and `type`
  **Location**: Subphase 6A, Scenario 12
  The ticket has nine frontmatter fields; Scenario 12 prohibits editing six. A stress-test session that rewrites `title` would desync the body H1; `type` and `author` have similar sync relationships. State positively: edit only body sections (Acceptance Criteria, Dependencies, Assumptions, Technical Notes).

- 🟡 **Safety**: Parent Requirements section may be clobbered during decompose
  **Location**: Subphase 6B, Step 4a and Scenario 5
  Decompose Edits the parent's Requirements to insert child links, but doesn't specify merge strategy. Parent Requirements is likely non-empty (it was the source material). Risk of full-section replacement with only link list.

- 🟡 **Safety**: Replace mode in enrich is destructive and under-specified
  **Location**: Subphase 6B, Scenario 14
  The "replace" option destroys existing Technical Notes with no diff preview or second confirmation. Users selecting it by habit can lose hand-crafted content.

- 🟡 **Safety**: Second link invocation can silently clobber first
  **Location**: Subphase 6B, Scenario 11
  Unlike enrich, link has no idempotency scenario. Dependencies written in a prior session can be overwritten on a re-run.

- 🟡 **Architecture**: Hierarchy rendering and child-ticket construction duplicated in prose rather than shared
  **Location**: Subphase 6B, Step 4a, Step 5, Key Discoveries
  Three SKILL.md files would now carry rendering logic in prose; two would carry child frontmatter population. Template changes become shotgun surgery. Justify the "no new scripts" constraint against this cost, or extract.

- 🟡 **Architecture**: Five heterogeneous operations behind one menu dispatcher reduce cohesion
  **Location**: Subphase 6B, Overview and Step 3
  Decompose, enrich, sharpen, size, link have different preconditions, failure modes, and invocation patterns. Changing any one forces re-authoring the combined SKILL.md and re-running all 15 evals.

- 🟡 **Architecture**: Decompose writes child links into parent's Requirements, conflating structural and narrative content
  **Location**: Subphase 6B, Step 4a
  Parent→child relationships are already expressed via each child's `parent:` frontmatter. Mixing them into Requirements creates two sources of truth.

- 🟡 **Test Coverage**: Several behaviours not deterministically observable from a single-turn eval
  **Location**: Subphase 6A Scenarios 3, 4, 10; Subphase 6B Scenario 3
  Parallelism, "waits for answer", depth-first follow-up, termination-after-all-branches require multi-turn simulation. The create-ticket evals model this with pre-scripted conversation history; the plan's scenarios do not.

- 🟡 **Test Coverage**: Hierarchy-tree indentation and decompose post-conditions under-specified
  **Location**: Subphase 6B Scenario 13 and fixture block
  No literal expected-tree string, no byte-level child-file assertions. "Nine frontmatter fields populated" has no enforcement mechanism.

- 🟡 **Test Coverage**: Negative and failure-mode coverage is thin
  **Location**: Overall
  Missing: bad path, malformed frontmatter, non-canonical parent in input, abort-mid-decompose post-condition, link with zero candidates, size-called-twice idempotency, bug/spike decomposition challenge, enrich-with-no-agent-results.

- 🟡 **Test Coverage**: No CI-level eval running leaves regression detection to manual re-runs
  **Location**: Current State Analysis / "Not adding CI-level eval running"
  Benchmark.json becomes a historical snapshot, not a guard. At minimum, a structural validation (`evals.json` scenarios appear in `benchmark.json`; `pass_rate.mean == 1.0`) should run under `mise run test`.

- 🟡 **Usability**: Eager number consumption and no-rollback policy not surfaced to the user
  **Location**: Subphase 6B, Scenario 12 + "What We're NOT Doing"
  No requirement to warn before allocation or report which numbers were consumed on abort. Users end up with orphaned-number anxiety.

- 🟡 **Usability**: Decompose approval verbs unspecified — no clear UX for edit/accept/reject per child
  **Location**: Subphase 6B, Step 4a ("Iterate until agreed")
  No approval grammar (approve-all, edit-N, drop-N, regenerate, add). The single most load-bearing interaction in refine-ticket is under-specified.

- 🟡 **Usability**: Idempotency only specified for enrich (and implicitly link); decompose, sharpen, size re-runs undefined
  **Location**: Subphase 6B, Step 4
  Re-running refine on a parent that already has children, re-sharpening already-testable criteria, re-sizing an already-sized ticket — all undefined.

- 🟡 **Usability**: Size-line placement inside Technical Notes is ambiguous when existing content is present
  **Location**: Subphase 6B, Scenario 10 + Step 4d
  "Append" without position guarantees means the size line can end up buried mid-paragraph. Re-runs produce duplicates.

- 🟡 **Documentation**: Agent fallback list left implicit for refine-ticket
  **Location**: Subphase 6B spec block
  stress-test-ticket enumerates seven agents; refine-ticket says "same accelerator:* list as other ticket skills". Skill-creator may omit or truncate.

- 🟡 **Documentation**: Child frontmatter derivation rules incomplete for date, tags, and priority
  **Location**: Subphase 6B, Scenario 7 and Step 4a
  `date` format ambiguous (`Z` vs `+00:00`), `tags` "where applicable" undefined, `priority` "inherited or asked once" — once per session or per child?

- 🟡 **Documentation**: Skill-flow pseudo-prose leaves gaps skill-creator will fill differently each iteration
  **Location**: Subphase 6B spec block
  Several under-specified clauses: "Iterate until agreed" termination rule, placeholder vs no-placeholder in child AC, Requirements vs Tasks subsection, replace-or-append vs net-new for enrich, "create if absent" for Technical Notes.

#### Minor

- 🔵 **Correctness**: "Small catalogues" threshold for link direct-read vs agent-spawn is undefined
- 🔵 **Correctness**: Enrich edge case unhandled: codebase agents find no relevant files
- 🔵 **Correctness**: Bug/spike decomposition challenge behaviour has no eval
- 🔵 **Correctness**: Parent link list write location is ambiguous (Requirements vs new Tasks subsection)
- 🔵 **Correctness**: Tags inheritance rule "where applicable" is undefined
- 🔵 **Correctness**: Scenario 2 "reads the template via `config-read-template.sh`" is slightly misleading — template is injected at skill-load time
- 🔵 **Correctness**: Editable section list overlap between stress-test and refine at Acceptance Criteria is undocumented
- 🔵 **Architecture**: Size written as a magic line inside Technical Notes creates a latent schema
- 🔵 **Architecture**: Child-type derivation rule hard-coded rather than template-driven (breaks for customised types)
- 🔵 **Architecture**: No-rollback decomposition lacks compensation strategy for partial writes
- 🔵 **Test Coverage**: Edit-tool invocations should be verified by before/after file comparison, not just "Edit was used"
- 🔵 **Test Coverage**: Codebase-agent spawning should be simulated, not actually performed
- 🔵 **Test Coverage**: Scenario 7 is a post-condition that should be decomposed into per-field checks
- 🔵 **Test Coverage**: Manual smoke test is the only end-to-end verification of cross-skill interaction
- 🔵 **Usability**: Box-drawing tree characters prescribed without terminal fallback
- 🔵 **Usability**: Bare-invocation prompt examples are thin compared to `/review-ticket`'s (no numeric shorthand, no `/list-tickets` discovery hint)
- 🔵 **Usability**: Error-path coverage is missing from the eval scenarios
- 🔵 **Usability**: Menu doesn't preview what each operation will actually propose before picking
- 🔵 **Documentation**: Box-drawing characters in code fence — literal vs illustrative is ambiguous
- 🔵 **Documentation**: Missing boundary cases in "What We're NOT Doing": tag updates, child deletion mid-flow, re-running refine on already-refined tickets, bug/spike decompose challenge
- 🔵 **Documentation**: Fragile line-range reference likely to drift (research doc "lines 781–795")
- 🔵 **Documentation**: Template-substitution tokens `{codebase locator agent}` not explained to human readers
- 🔵 **Documentation**: stress-test-ticket spec does not say whether to include the template block
- 🔵 **Safety**: Ticket number allocation is not atomic; concurrent refine sessions can collide
- 🔵 **Safety**: Edit failure path is not specified; partial-write scenarios are possible
- 🔵 **Safety**: No-rollback claim relies implicitly on jj/git; plan should make that explicit
- 🔵 **Standards**: Refine-ticket agent fallback is specified implicitly while stress-test-ticket is explicit
- 🔵 **Standards**: Relationship block uses `/create-plan <ticket>` — sibling skills list `/create-plan` without argument
- 🔵 **Standards**: Template injection labelled "static" but is bang-executed like sibling skills — misleading
- 🔵 **Standards**: Frontmatter grep check permits a wrong field order

#### Suggestions

- 🔵 **Architecture**: Edit-tool coupling to body section headings assumes template stability
- 🔵 **Usability**: Lifecycle ordering of refine vs. review vs. stress-test could confuse new users
- 🔵 **Standards**: Parent-child linking format not grounded in an existing precedent
- 🔵 **Standards**: Specs list preamble as ordered pseudocode; sibling SKILL.md files have no heading between frontmatter and preamble

### Strengths

- ✅ Strong alignment with sibling conventions: frontmatter ordering, preamble bang-execution order, path injection, agent fallback paragraph, instructions injection, fixture directory structure, evals/benchmark sibling pattern all match precedents
- ✅ `allowed-tools` correctly split: stress-test-ticket (config-only, mirroring stress-test-plan) vs refine-ticket (config + tickets/scripts, mirroring create-ticket/update-ticket)
- ✅ stress-test-ticket faithfully mirrors stress-test-plan's conversational discipline and complementary framing with /review-ticket
- ✅ Per-operation section ownership in refine-ticket gives each operation a single reason to change
- ✅ "What We're NOT Doing" enumerates architectural negative space with precision — no new frontmatter fields, no new config keys, no auto-invocation of review
- ✅ Scenario 14 (enrich replace/append/skip) and Scenario 11 (link references only real numbers) encode good idempotency and safety patterns
- ✅ No-redesign guard (Scenario 13 in stress-test-ticket) and "offers but does not invoke /review-ticket" (Scenario 15) are the kind of negative behaviour constraints that catch drift
- ✅ Success Criteria cleanly separate automated (grep-/test-verifiable) from manual checks with concrete commands
- ✅ Correctly identifies `ticket-next-number.sh --count N` batch allocation; parent canonicalisation to four-digit zero-padded string; nine-field frontmatter; ten-section body
- ✅ References section is comprehensive: structural model, composition sources, fixture pattern, pass-rate convention

### Recommended Changes

1. **Fix the author-resolution chain in refine-ticket** (addresses: "config-read-context.sh cannot resolve author")
   Replace the `config-read-context.sh` reference in Scenario 7 and Step 4a with create-ticket's chain: inherit from parent → config → git/jj identity → ask user once. Cite create-ticket/SKILL.md Quality Guidelines as the source.

2. **Resolve the hierarchy-rendering parity claim** (addresses: tree format claim, box-drawing ambiguity, rendering duplication)
   Choose one: (a) pin the exact indentation/glyph string in refine-ticket as authoritative and drop the "matches /list-tickets" claim, or (b) update `list-tickets/SKILL.md` first to commit to a literal format (e.g. `├──`/`└──` or plain two-space indent) and reference that verbatim. Consider extracting a shared `ticket-render-hierarchy.sh` helper and noting the extraction rationale.

3. **Specify a canonical multi-operation order and add a multi-op eval** (addresses: multi-op order under-specified, enrich+size co-ownership, Step 5 vs Step 6 sequencing)
   Fix a canonical order (suggestion: decompose → enrich → sharpen → size → link), pin placement rules for size within Technical Notes (first or last line; replace any existing `**Size**:` line on re-run), sequence Step 5 hierarchy display after decompose-then-other-ops, and add Scenario 16 covering a multi-operation selection with expected post-conditions.

4. **Rework decompose to avoid clobbering parent Requirements** (addresses: decompose writes to Requirements, parent-clobber safety, Requirements vs Tasks ambiguity)
   Pick one: (a) rely solely on each child's `parent:` frontmatter for traversal and drop the parent body edit entirely, or (b) specify an append-only subsection (`### Child tickets`) at the end of Requirements, never touching existing content, with a diff-confirm step before writing. Remove the "or a new Tasks subsection" alternative from Scenario 6.

5. **Tighten the stress-test edit prohibition list** (addresses: prohibition omits title/author/type)
   Rewrite Scenario 12 and the spec's "Capturing Changes" prose as: "Edit only these body sections: Acceptance Criteria, Dependencies, Assumptions, Technical Notes. Never touch any frontmatter field nor the body `**Type**`/`**Status**`/`**Priority**`/`**Author**` labels." This closes the title/author/type gap and the body-label sync gap.

6. **Complete child frontmatter derivation rules** (addresses: date/tags/priority rules, child-type template-driven)
   For each of the nine fields give a one-line rule with exact format: `date` via `date -u +%Y-%m-%dT%H:%M:%S+00:00` (matching create-ticket), `tags` = verbatim copy of parent's tags, `priority` asked once per session (or inherited), `author` per the chain from change #1. Note that child-type mapping (epic→story, story→task) is hardcoded and a future enhancement could template-drive it.

7. **Add idempotency and failure-mode scenarios** (addresses: idempotency gaps, negative coverage thin, error paths missing, link clobber, multi-op absence, bug/spike challenge)
   Add scenarios for: (a) bad/missing path, (b) malformed frontmatter, (c) abort-mid-decompose post-condition, (d) link with zero candidates, (e) second-run link (replace/append/skip mirroring enrich), (f) size called twice (replace in place), (g) decompose on bug/spike challenging the user, (h) sharpen when every AC is already testable, (i) enrich when codebase agents return nothing concrete, (j) multi-operation selection with canonical order.

8. **Rework eval prompts for single-turn observability** (addresses: non-observable scenarios, decompose post-conditions, agent spawning)
   Follow the create-ticket pattern of pre-scripted multi-turn simulation inside a single prompt. For scenarios asserting "waits", "depth-first", or "parallel", script the relevant history in the prompt and assert on the skill's *next* output. For decompose scenarios 5/6/7/12, check in expected child ticket fixtures and assert byte-level match on the produced files. Simulate codebase agent results inline rather than actually spawning agents.

9. **Warn before, and report after, ticket-number allocation** (addresses: eager consumption UX, no-rollback safety, allocation atomicity)
   Require refine-ticket to (a) state "this will allocate N ticket numbers; aborting mid-write leaves partial state — use `jj restore <file>` to discard" before the write step, (b) on completion/interrupt print a concise ledger of which NNNN were written and which (if any) were allocated but skipped, (c) re-check filename availability immediately before each child write and abort-with-diagnostic on collision. State the single-session assumption explicitly in "What We're NOT Doing".

10. **Specify a decompose approval grammar** (addresses: approval verbs unspecified)
    Define the iteration interface: numbered children with commands like `approve all`, `edit N: <new title>`, `drop N`, `add: <title>`, `regenerate`. Include a short example transcript in Step 4a so skill-creator produces a consistent menu across iterations.

11. **Add diff preview + second confirmation for destructive edit paths** (addresses: enrich replace destructive, link clobber)
    For enrich's "replace" option and any Edit path that overwrites existing user content, require a unified-diff preview followed by an explicit y/n. Mirror update-ticket's diff-confirm pattern.

12. **Enumerate the refine-ticket agent fallback list verbatim** (addresses: agent fallback implicit)
    Replace "same accelerator:* list as other ticket skills" with the full seven-name paragraph used in stress-test-ticket's spec and every sibling SKILL.md.

13. **Add a lightweight eval-structure check to `mise run test`** (addresses: no CI-level eval running)
    Not full eval re-running, but a structural validation: for each skill with `evals/`, verify (a) `evals.json` and `benchmark.json` both exist, (b) every scenario name in evals.json appears in benchmark.json, (c) `benchmark.json.run_summary.with_skill.pass_rate.mean == 1.0`. This is a cheap guard against stale/truncated benchmarks.

14. **Minor documentation tidying** (addresses: minor docs findings)
    - Anchor the research-doc reference by heading, not line range
    - Note placeholder-token convention (`{codebase locator agent}` = runtime substitution; `<rationale>` = spec placeholder)
    - State whether stress-test-ticket should include a template block (prob. no, matching stress-test-plan — but say so)
    - Replace "static" with "rendered inline via bang-execution" for the refine-ticket template section
    - Fix `/create-plan <ticket>` to `/create-plan` in the relationship block
    - Add missing boundary cases to "What We're NOT Doing" (tag updates, child deletion mid-flow, re-run decompose on already-refined)

---

*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally disciplined overall: stress-test-ticket cleanly mirrors the established stress-test-plan pattern, and refine-ticket is carefully composed from three existing sibling skills with explicit section ownership rules. However, refine-ticket concentrates five heterogeneous operations behind a single menu dispatcher, which raises cohesion concerns, and several cross-skill couplings (hierarchy rendering, parent canonicalisation, child-creation logic) are duplicated in prose rather than factored into shared scripts. The 'no new script' constraint is the central architectural tradeoff and deserves more explicit justification.

**Strengths**: see aggregated list.

**Findings**:
- 🟡 **major / high**: Hierarchy rendering and child-ticket construction duplicated in prose rather than shared — Subphase 6B, Step 4a, Step 5, Key Discoveries. Template changes become shotgun surgery across three SKILL.md files. Either extract a shared script or explicitly justify the "no new scripts" constraint with a trigger condition for future extraction.
- 🟡 **major / medium**: Five heterogeneous operations behind one menu dispatcher reduce cohesion — Subphase 6B, Overview and Step 3. Different preconditions, failure modes, invocation patterns. Either justify with a "why one skill, not five" paragraph or split the bigger/riskier operations (decompose, link) into their own skills.
- 🟡 **major / high**: Decompose writes child links into parent's Requirements, conflating structural and narrative content — Subphase 6B, Step 4a. Parent→child already expressed via `parent:` frontmatter; mixing creates two sources of truth and clobber risk. Consider dropping the parent body edit, or using a dedicated `### Child tickets` subsection.
- 🔵 **minor / high**: Size written as a magic line inside Technical Notes creates a latent schema — Subphase 6B, Step 4d. Downstream parsers will regex-match `**Size**:` out of prose. Either accept and document the limitation, or migrate to a frontmatter field when a second consumer appears.
- 🔵 **minor / medium**: Child-type derivation hard-coded rather than template-driven — Subphase 6B, Step 4a. `else→story` is a footgun for customised type configurations.
- 🔵 **minor / high**: No-rollback decomposition lacks compensation strategy for partial writes — Subphase 6B, Step 4a and Scenario 12.
- 🔵 **suggestion / medium**: Edit-tool coupling to body section headings assumes template stability — Subphases 6A and 6B.

### Test Coverage

**Summary**: The plan adopts the established TDD-via-eval pattern consistently and covers the major happy-path behaviors of both skills with reasonable breadth (13 + 15 scenarios). However, several scenarios describe behaviors that are fundamentally hard to observe deterministically in a single-turn eval harness (parallel agent spawning, 'depth-first' follow-up, exact tree indentation), fixture specifications for decomposition scenarios are under-specified for deterministic assertion, and negative/failure-mode coverage is largely absent.

**Findings**:
- 🟡 **major / high**: Several behaviours not deterministically observable from a single-turn eval — Scenarios 6A-3, 6A-4, 6A-10, 6B-3. Need multi-turn simulation in prompt or pinned output assertions.
- 🟡 **major / high**: Hierarchy-tree indentation and decompose post-conditions under-specified — Scenario 13, decompose fixture block. Pin the literal expected tree string; provide checked-in expected child ticket files; enumerate per-field checks rather than blanket "all nine populated".
- 🟡 **major / high**: Negative and failure-mode coverage is thin — Missing invalid path, malformed frontmatter, non-canonical parent input, abort-mid-decompose post-condition, link-with-zero-candidates, size-idempotence, enrich template-placeholder, sharpen-no-op.
- 🟡 **major / medium**: No CI-level eval running — Benchmark.json becomes a historical snapshot. Add a structural validation (evals.json scenarios match benchmark.json; pass_rate == 1.0) to `mise run test`.
- 🔵 **minor / high**: Edit invocations should be verified by before/after file comparison.
- 🔵 **minor / medium**: Codebase-agent spawning should be simulated, not actually performed.
- 🔵 **minor / medium**: Scenario 7 (all-nine-fields) should be decomposed into per-field checks.
- 🔵 **minor / medium**: Manual smoke test is the only end-to-end verification.

### Correctness

**Summary**: The plan is largely internally consistent and aligns with most codebase facts, but contains verifiable correctness issues: the hierarchy-tree rendering format claimed to match list-tickets does not; the author-resolution script named in Scenario 7 (config-read-context.sh) outputs project context, not author identity; and multiple operations (enrich, size) own overlapping writes to the Technical Notes section without an ordering/conflict rule.

**Findings**:
- 🟡 **major / high**: config-read-context.sh cannot resolve author.
- 🟡 **major / high**: Claimed list-tickets tree format with ├── / └── does not exist in list-tickets.
- 🟡 **major / high**: size and enrich both write to Technical Notes with no defined ordering/merge rule.
- 🟡 **major / medium**: Multi-operation execution order is under-specified.
- 🟡 **major / medium**: Step 5 "after decompose only" vs Step 6 "after refinement" not sequenced.
- 🔵 **minor / high**: "Small catalogues" threshold for link is undefined.
- 🔵 **minor / medium**: Enrich edge case: agents find no relevant files.
- 🔵 **minor / high**: Bug/spike decomposition challenge behaviour has no eval.
- 🔵 **minor / medium**: Parent link list location ambiguous (Requirements vs Tasks).
- 🔵 **minor / medium**: Tags inheritance "where applicable" undefined.
- 🔵 **minor / high**: Scenario 2 wording about template read is slightly misleading.
- 🔵 **minor / medium**: Editable section overlap between stress-test and refine at Acceptance Criteria is undocumented.

### Usability

**Summary**: The plan establishes two skills with sensible invocation surfaces and mirrors the well-understood stress-test-plan UX model. However, several refine-ticket interactions are under-specified: eager ticket-number consumption has no user-facing transparency or rollback communication, the decompose iteration/approval verbs aren't defined, idempotency is only addressed for enrich, and the hierarchy rendering prescribes box-drawing characters without a fallback.

**Findings**:
- 🟡 **major / high**: Eager number consumption and no-rollback policy not surfaced to user.
- 🟡 **major / high**: Decompose approval verbs unspecified.
- 🟡 **major / medium**: Idempotency only specified for enrich; decompose/sharpen/size re-runs undefined.
- 🟡 **major / medium**: Size-line placement inside Technical Notes ambiguous when existing content present.
- 🔵 **minor / high**: Box-drawing tree characters prescribed without terminal fallback.
- 🔵 **minor / high**: Bare-invocation prompt examples thin compared to /review-ticket (no numeric shorthand).
- 🔵 **minor / high**: Error-path coverage missing.
- 🔵 **minor / medium**: Menu doesn't preview what each operation will propose.
- 🔵 **suggestion / medium**: Lifecycle ordering of refine vs review vs stress-test could confuse new users.

### Documentation

**Summary**: The plan is well-structured, thorough, and authored in a clear voice. Its TDD specification approach works well. The main documentation risks sit in the refine-ticket specification: several clauses offload detail to "same as other skills" without restating it, a few child-ticket fields have under-specified derivation rules, placeholder tokens could confuse a human reader, and a fragile line-range reference could drift.

**Findings**:
- 🟡 **major / high**: Agent fallback list left implicit for refine-ticket.
- 🟡 **major / high**: Child frontmatter derivation rules incomplete for date, tags, priority.
- 🟡 **major / medium**: Skill-flow pseudo-prose leaves gaps skill-creator will fill differently.
- 🔵 **minor / high**: Box-drawing characters in code fence — literal vs illustrative ambiguous.
- 🔵 **minor / high**: Missing boundary cases in "What We're NOT Doing".
- 🔵 **minor / medium**: Fragile line-range reference likely to drift.
- 🔵 **minor / medium**: Template-substitution tokens not explained to human readers.
- 🔵 **minor / high**: stress-test-ticket spec silent on template block inclusion.

### Safety

**Summary**: The plan's safety posture is appropriate for a low-stakes developer tool operating on version-controlled markdown files, and it explicitly acknowledges the "no rollback" model. However, several guardrails are incomplete: the do-NOT-modify list for stress-test edits omits three frontmatter fields, decompose may clobber user-authored Requirements, two operations both target Technical Notes with no conflict rule, and ticket-number allocation has no locking.

**Findings**:
- 🟡 **major / high**: Stress-test do-NOT-modify list omits title, author, and type.
- 🟡 **major / high**: Parent Requirements may be clobbered during decompose.
- 🟡 **major / high**: enrich and size both write to Technical Notes with no ordering rule.
- 🟡 **major / medium**: Replace mode in enrich is destructive and under-specified.
- 🟡 **major / medium**: Second link invocation can clobber first.
- 🔵 **minor / high**: Ticket number allocation not atomic; concurrent sessions can collide.
- 🔵 **minor / medium**: Edit failure path not specified; partial-write scenarios possible.
- 🔵 **minor / medium**: No-rollback claim relies implicitly on jj/git; plan should make explicit.

### Standards

**Summary**: The plan demonstrates strong alignment with sibling skill conventions. Frontmatter ordering, preamble bang-execution order, path injection, agent fallback, instructions injection, fixture structure, and evals/benchmark sibling pattern all match precedents. The allowed-tools split between stress-test-ticket and refine-ticket correctly mirrors stress-test-plan and the script-using ticket skills. A few minor inconsistencies are noted.

**Findings**:
- 🔵 **minor / medium**: Refine-ticket agent fallback implicit while stress-test-ticket explicit.
- 🔵 **minor / high**: Relationship block uses `/create-plan <ticket>` instead of `/create-plan`.
- 🔵 **minor / medium**: Template injection labelled "static" but is bang-executed — misleading.
- 🔵 **minor / high**: Frontmatter grep check permits wrong field order.
- 🔵 **suggestion / medium**: Parent-child linking format not grounded in existing precedent.
- 🔵 **suggestion / low**: Specs list preamble as ordered pseudocode; sibling files have no heading between frontmatter and preamble.

## Re-Review (Pass 2) — 2026-04-25

**Verdict:** REVISE (no critical findings; 4 new majors — narrowly above the 3-major threshold; all confined to the new spec material added in Pass 1's edits)

The plan iteration substantially closes Pass 1's findings: ~46 of the ~58 prior findings are fully resolved, ~10 are partially resolved with documented tradeoffs, and ~3 remain as explicit acknowledged tradeoffs (the dispatcher cohesion, hardcoded child-type mapping, and Edit-tool/heading coupling — all flagged but consciously deferred). The verdict remains REVISE, but the gap is narrow and entirely about the *new* spec material introduced in Pass 1's edits — Subphase 6C's validator has no self-tests, Scenario 5a's grammar bundles six verbs into one assertion, Scenario 23's tool-failure simulation uses an untested SIMULATE primitive, and decompose's `### Child tickets` Edit operation never specifies the `old_string` anchor. None of these are conceptual problems; all are completeness fixes within the patterns the plan has already adopted.

### Previously Identified Issues

#### Architecture (3 of 7 resolved; 2 partial; 2 acknowledged tradeoffs)

- 🟡 **Hierarchy rendering and child-ticket construction duplicated in prose** — Partially resolved. Subphase 6.0 makes parity verifiable; duplication remains an acknowledged tradeoff. No automated drift check.
- 🟡 **Five operations behind one menu dispatcher** — Still present; explicitly retained as an acknowledged tradeoff (canonical order + per-op idempotency mitigate but don't split).
- 🟡 **Decompose writes child links into Requirements** — Resolved. New `### Child tickets` subsection appended, never modifies existing prose.
- 🔵 **Size as magic line creates latent schema** — Partially resolved. Re-run replace-in-place + `size unchanged` no-op formalise the latent schema; remains in prose as an explicit tradeoff.
- 🔵 **Child-type derivation hardcoded** — Still present; bug/spike confirmation added but no template-driven mapping mechanism.
- 🔵 **No-rollback decomposition lacks compensation** — Resolved. Pre-write warning, collision check, parent-Edit-last, ledger, jj-restore guidance.
- 🔵 **Edit-tool coupling to body section headings** — Still present; graceful Edit-failure diagnostics added but no abstraction layer.

#### Test Coverage (6 of 8 resolved; 2 partial)

- 🟡 **Non-observable behaviours** — Resolved. Single-turn multi-turn-simulation pattern adopted across Scenarios 6A-3/4/8/10 and 6B-3.
- 🟡 **Hierarchy & decompose post-conditions under-specified** — Resolved. Subphase 6.0 pins the format; Scenario 13 pins exact tree string; checked-in expected-child/expected-parent fixtures with byte-level assertion.
- 🟡 **Negative/failure-mode coverage thin** — Resolved. 8 new failure-mode scenarios (14, 17, 18, 19, 20, 21, 22, 23) plus 5a, 10a, 11a, 16 idempotency/ordering coverage.
- 🟡 **No CI-level eval running** — Partially resolved. Subphase 6C structural validation added; full LLM eval re-running still deferred.
- 🔵 **Edit invocations not byte-level verified** — Resolved. Stress-test 15 and refine fixture spec require `expected-*.md` byte-level diff.
- 🔵 **Codebase-agent spawning should be simulated** — Resolved. SIMULATE blocks adopted in Scenarios 6A-8 and 6B-3/22.
- 🔵 **Scenario 7 should be split per field** — Resolved. Fixture-block guidance splits per-field byte-level checks across decompose scenarios.
- 🔵 **Manual smoke is the only e2e verification** — Partially resolved. Subphase 6C and Scenario 16 add some coverage; cross-skill interaction (e.g. refine→list-tickets hierarchy parity) still manual.

#### Correctness (11 of 12 resolved; 1 partial)

- 🟡 **config-read-context.sh cannot resolve author** — Resolved (with a small mis-citation flagged separately).
- 🟡 **list-tickets tree format does not exist** — Resolved via Subphase 6.0.
- 🟡 **size and enrich Technical Notes ordering** — Resolved (with a "trailing"/"leading" wording inconsistency flagged separately).
- 🟡 **Multi-operation execution order under-specified** — Resolved (canonical order pinned in 4 places).
- 🟡 **Step 5 vs Step 6 sequencing** — Resolved (Step 5 inline after decompose; Step 6 once at end).
- 🔵 **Small catalogues threshold** — Resolved (≤30).
- 🔵 **Enrich edge case: empty agent results** — Resolved (Scenario 22).
- 🔵 **Bug/spike decompose challenge no eval** — Resolved (Scenario 19 + Step 4a confirmation).
- 🔵 **Parent link write location ambiguous** — Resolved (Tasks subsection alternative removed; pinned to `### Child tickets`).
- 🔵 **Tags inheritance "where applicable"** — Resolved (verbatim copy).
- 🔵 **Scenario 2 template-read wording** — Resolved.
- 🔵 **Editable section overlap stress-test vs refine** — Partially resolved; per-op ownership explicit, no cross-skill comparison drawn.

#### Usability (7 of 9 resolved; 2 partial)

- 🟡 **Eager number consumption not surfaced** — Resolved (warn + ledger + jj-restore guidance).
- 🟡 **Decompose approval verbs unspecified** — Resolved (Scenario 5a grammar).
- 🟡 **Idempotency limited to enrich** — Resolved (covered for all 5 operations).
- 🟡 **Size-line placement ambiguous** — Resolved (FIRST line, replace in place).
- 🔵 **Box-drawing characters without fallback** — Still present; deliberate tradeoff documented in Subphase 6.0.
- 🔵 **Bare-invocation prompts thin** — Resolved (number-or-path, /list-tickets hint).
- 🔵 **Error-path coverage missing** — Partially resolved (Scenario 18 diagnostic does not yet suggest a recovery action).
- 🔵 **Menu has no preview** — Resolved (per-operation previews + no-op marking in Step 3).
- 🔵 **Lifecycle ordering confusion** — Partially resolved (both skills now have a relationship paragraph; minor asymmetry in ordering wording).

#### Documentation (7 of 8 resolved; 1 partial)

- 🟡 **Agent fallback list implicit for refine-ticket** — Resolved (verbatim seven-name list; success-criterion grep added).
- 🟡 **Child frontmatter rules incomplete** — Partially resolved. All nine fields specified; the `author` chain reference to "configured author from accelerator config" lacks a config-key name (new finding flagged below).
- 🟡 **Skill-flow pseudo-prose has gaps** — Resolved (Scenario 5a grammar replaces "iterate until agreed").
- 🔵 **Box-drawing literal vs illustrative** — Partially resolved (Scenarios 13/Step 5 fenced; Subphase 6.0 example uses indented prose, flagged below).
- 🔵 **Missing boundary cases in NOT-Doing** — Resolved (4 boundary cases added).
- 🔵 **Fragile line-range reference** — Resolved (heading anchor).
- 🔵 **Template-substitution tokens not explained** — Resolved (brace vs angle-bracket explanation in Implementation Approach).
- 🔵 **Stress-test-ticket silent on template block** — Resolved (intentionally-omitted note added).

#### Safety (8 of 8 resolved)

- 🟡 **Stress-test prohibition omits title/author/type** — Resolved (all 9 frontmatter fields + 4 body labels enumerated; Scenario 15 byte-level).
- 🟡 **Parent Requirements clobber** — Resolved (append-only `### Child tickets` subsection; abort-with-diagnostic on Edit failure).
- 🟡 **enrich+size Technical Notes ordering** — Resolved (canonical order; size always FIRST line; enrich preserves leading `**Size**:`).
- 🟡 **Replace mode destructive and under-specified** — Resolved (unified diff + second y/n confirmation).
- 🟡 **Second link invocation clobber** — Resolved (Scenario 11a replace/append/skip with diff + confirmation).
- 🔵 **Number allocation not atomic** — Resolved as documented limitation (concurrent invocations explicitly out-of-scope; collision check named as safety net).
- 🔵 **Edit failure path** — Resolved (per-edit abort with diagnostic; decompose writes children first, parent last).
- 🔵 **No-rollback claim relies implicitly on jj/git** — Resolved (jj restore / jj undo surfaced in NOT-Doing and abort ledger).

#### Standards (6 of 6 resolved)

- 🔵 **Refine-ticket agent fallback implicit** — Resolved.
- 🔵 **`/create-plan <ticket>` wording** — Resolved.
- 🔵 **Template injection "static"** — Resolved.
- 🔵 **Frontmatter grep order check** — Resolved (field-ORDER assertion added).
- 🔵 **Parent-child linking format not grounded** — Resolved (Subphase 6.0 pins format).
- 🔵 **Preamble layout convention implicit** — Resolved (explicit Layout note in both specs).

### New Issues Introduced

#### Major (4)

- 🟡 **Test Coverage**: Subphase 6C lacks self-tests and may itself silently regress
  **Location**: Subphase 6C
  No fixtures of intentionally-broken evals.json/benchmark.json pairs. A faulty validator gives false confidence.
- 🟡 **Test Coverage**: Scenario 5a's six-verb grammar lacks fixture-driven assertion strategy
  **Location**: Subphase 6B Scenario 5a
  Six command paths (approve all, edit N, drop N, add, regenerate, cancel) collapsed into one eval — same bundled-assertion anti-pattern Scenario 7 was split to avoid.
- 🟡 **Test Coverage**: Scenario 23's `SIMULATE the Write tool failing` is not an established simulation primitive
  **Location**: Subphase 6B Scenario 23
  SIMULATE pattern is established only for agent return values; the model may simply narrate the failure rather than halt the write sequence.
- 🟡 **Correctness**: Edit anchor for new `### Child tickets` subsection is unspecified
  **Location**: Subphase 6B Step 4a / Scenario 5
  Edit requires unique exact-match string; the natural anchor (next `## ` H2) is implicit in template ordering but never stated. Skill-creator will improvise a brittle anchor.

#### Minor (15)

- 🔵 **Architecture**: Hard-coded `pass_rate.mean == 1.0` invariant in Subphase 6C couples test infra to a specific benchmark schema with no central contract.
- 🔵 **Architecture**: Pinned tree format duplicated across two SKILL.md files; drift between Subphase 6.0 and Scenario 13 is unguarded.
- 🔵 **Architecture**: Canonical order entangles target selection — multi-op (decompose + enrich) refines only the parent, not the freshly-written children; user mental model may differ.
- 🔵 **Test Coverage**: Scenario 16 lacks a concrete `expected-final-ticket.md` post-multi-op fixture.
- 🔵 **Test Coverage**: Scenario 11's ≤30-vs-agent threshold is not directly tested (no Scenario 11b for the >30 path).
- 🔵 **Test Coverage**: Edit-failure path (target string unmatched) lacks dedicated coverage in either skill.
- 🔵 **Test Coverage**: Cross-skill end-to-end interaction (refine→list-tickets hierarchy) remains manual-only.
- 🔵 **Test Coverage**: Date and author fields are environment-dependent; byte-level checking in Scenario 7 needs explicit carve-out.
- 🔵 **Correctness**: Author chain claims to follow create-ticket but prepends a parent step — claim is technically wrong.
- 🔵 **Correctness**: Step 5 trigger condition silent on cancelled-decompose case (5a `cancel` or 19 `n` answer).
- 🔵 **Correctness**: `**Size**:` preservation rule in Scenario 14 says "trailing" but Scenario 10 pins it as "leading/first line".
- 🔵 **Correctness**: Append-children Edit anchor for Scenario 20 not specified.
- 🔵 **Correctness**: Subphase 6.0 grep success criteria are weaker than the rule they pin (presence ≠ correctness).
- 🔵 **Usability**: Approval-grammar legend not shown up-front; users discover verbs by error rather than design.
- 🔵 **Usability**: Confirmation cascades on multi-operation runs may feel heavy; no power-user bypass documented.
- 🔵 **Usability**: Menu preview density on rich tickets (5 non-trivial previews) unverified by any scenario.
- 🔵 **Usability**: Unicode-only tree rendering accepts mojibake on legacy terminals — same concern now affects refine-ticket post-decompose display.
- 🔵 **Documentation**: 23-scenario refine-ticket section would benefit from an in-section index.
- 🔵 **Documentation**: Author chain references "configured author from accelerator config" without naming the config key.
- 🔵 **Documentation**: Subphase 6.0 tree example shown as indented prose, not a fenced code block (inconsistent with Scenarios 13 and 5).
- 🔵 **Safety**: Size in-place replace skips the second-confirmation gate the rest of the plan enforces consistently — destructive replacement of user-authored rationale prose without confirmation.
- 🔵 **Safety**: Multi-op selection (decompose + enrich) writes parent twice with no consolidated abort point in the ledger format.

#### Suggestions (5)

- 🔵 **Usability**: Allocation warning is correct but slightly verbose for the 2-child case; consider folding into the proposal display.
- 🔵 **Usability**: Lifecycle ordering described twice — slightly inconsistently between the two skills' relationship paragraphs.
- 🔵 **Usability**: Error message phrasing diverges across no-op scenarios (11, 21, 22, 10a) — pick a single template.
- 🔵 **Standards**: argument-hint `[ticket number or path]` diverges from every sibling skill's phrasing — consider adopting `[ticket-ref]` or accepting the divergence with rationale.
- 🔵 **Standards**: Expected-fixture naming asymmetric within the plan (`expected-ticket.md` for stress-test-ticket vs `expected-parent.md` / `expected-child-N.md` for refine-ticket); pin the convention.
- 🔵 **Standards**: Subphase 6C `tasks/scripts/test-evals-structure.sh` location uses an uncommitted convention with an "or choose at implementation time" escape hatch — pin the directory.
- 🔵 **Standards**: Hard-coded `pass_rate.mean == 1.0` invariant should be precondition-verified against existing ~11 benchmark files before wiring into `mise run test`.
- 🔵 **Standards**: Stress-test-ticket `expected-ticket.md` vs refine-ticket `expected-parent.md` — unify on one role name (suggested: `expected-target.md` for both).

### Assessment

The plan has moved from "needs significant revision" to "needs targeted polish on the newly-added material". The conceptual gaps from Pass 1 are closed; what remains is filling in the implementation details of the new patterns (the `### Child tickets` Edit anchor, the Scenario 5a per-verb assertions, the Scenario 23 simulation primitive, and self-tests for the Subphase 6C validator). A third pass could plausibly bring the verdict to APPROVE with around two hours of edits — none of the new findings require structural rethinking.

Recommended next-step bundle (in priority order):
1. Specify the `### Child tickets` Edit anchor (use the next H2 — likely `## Acceptance Criteria` — as `old_string`); add expected-parent fixtures for Scenarios 5/6/12/16/20.
2. Add fixture-driven self-tests for Subphase 6C's `test-evals-structure.sh` (broken-pair / missing-benchmark / wrong-mean fixtures).
3. Decompose Scenario 5a into per-verb sub-scenarios (5a-approve, 5a-edit, 5a-drop, 5a-add, 5a-regenerate, 5a-cancel, 5a-unknown).
4. Replace Scenario 23's `SIMULATE the Write tool failing` with a multi-step transcript using a user-driven cancel after partial writes.
5. Fix the wording inconsistencies: author chain "extends" rather than "follows" create-ticket; Scenario 14 "leading" not "trailing"; Subphase 6.0 example in fenced code block.
6. Pin remaining conventions: Subphase 6C script location, expected-fixture naming (`expected-target.md` vs `expected-child-N.md`), argument-hint, and a Scenario 7 carve-out for environment-dependent fields (date/author).

The size-replace confirmation gate (safety minor) is the one finding I'd flag as a judgment call rather than a clear fix — adding the gate keeps the destructive-path policy uniform; skipping it preserves the "single-line magic field" frictionless re-run. I'd lean toward adding the gate for consistency, but either choice is defensible if documented.

## Re-Review (Pass 3) — 2026-04-25

**Verdict:** COMMENT (1 new major finding, below the 3-major REVISE threshold; plan is acceptable but could be improved — see findings below)

The plan iteration substantially closes Pass-2's findings: ~21 of ~32 prior findings fully resolved, ~5 partially resolved with explicit tradeoffs, and ~6 still present (5 of which are unaddressed Pass-2 usability concerns the user explicitly excluded from the Pass-3 next-step bundle, plus the canonical-order target ambiguity). All four Pass-2 majors are resolved. Pass-3 introduces one new major (safety: anchor uniqueness) and a cluster of minor findings — all about completeness of the new spec material itself rather than conceptual gaps. The plan is in good shape to ship with one targeted fix on the Edit anchor uniqueness concern and a few small wording polishes; the remaining minor findings are appropriate to defer or capture as follow-up work.

### Previously Identified Issues

#### Architecture (1 of 3 resolved; 1 partial; 1 still present)
- 🔵 **pass_rate.mean == 1.0 invariant coupling** — Partially resolved. Precondition step + soften path documented; literal `1.0` still hard-coded with no shared schema contract.
- 🔵 **Tree format duplicated across two SKILL.md files** — Resolved. New `test-hierarchy-format.sh` provides automated drift guard.
- 🔵 **Canonical order entangles target selection** — Still present. Subsequent operations target invocation target only; mental model implicit (encoded only in test data, not normative prose).

#### Test Coverage (6 of 8 resolved; 2 partial)
- 🟡 **Subphase 6C lacks self-tests** — Resolved (5 fixture pairs + self-test script).
- 🟡 **Scenario 5a six-verb grammar bundled** — Resolved (split into 8 sub-scenarios).
- 🟡 **Scenario 23 `SIMULATE Write failing` undefined** — Resolved (reworked to user-driven cancel pattern).
- 🔵 **Scenario 16 lacks final-state fixture** — Resolved (`expected-parent.md` now covers FINAL multi-op state).
- 🔵 **Scenario 11 threshold not directly tested** — Resolved (Scenario 11b added).
- 🔵 **Edit-failure path lacks coverage** — Resolved (Scenario 24 added).
- 🔵 **Cross-skill e2e remains manual-only** — Partially resolved (`test-hierarchy-format.sh` covers tree-format drift; behavioural integration still manual smoke).
- 🔵 **Date/author env-dependent and resist byte-level checking** — Partially resolved (regex + recency for date, parent-equality for author; recency mechanism still under-specified).

#### Correctness (6 of 6 resolved)
- 🟡 **Edit anchor for `### Child tickets` unspecified** — Resolved (`\n## Acceptance Criteria\n` pinned).
- 🔵 **Author chain mis-citation** — Resolved (now "EXTENDS" with section anchor).
- 🔵 **Step 5 cancelled-decompose case** — Resolved (trigger condition enumerates skip paths).
- 🔵 **Scenario 14 "trailing" wording** — Resolved (now "LEADING").
- 🔵 **Append-children Edit anchor** — Resolved (last `- NNNN — title` line pinned).
- 🔵 **Subphase 6.0 grep success criteria weak** — Resolved (4 content checks + byte-level fence diff).

#### Usability (1 of 7 resolved; 1 partial; 5 still present)
- 🔵 **Approval grammar legend up-front** — Resolved (Scenario 5a-legend).
- 🔵 **Confirmation cascades on multi-op** — Still present (Pass-3 added a sixth gate via size-replace; no bypass documented).
- 🔵 **Menu preview density unverified** — Still present.
- 🔵 **Unicode-only tree mojibake** — Still present (deliberate tradeoff documented).
- 🔵 **Allocation warning verbose for 2-child case** — Still present (suggestion).
- 🔵 **Lifecycle ordering paragraphs slightly asymmetric** — Partially resolved (both skills now have a paragraph; stress-test ends in `/create-plan`, refine ends in `/update-ticket` — same ordering, different terminator).
- 🔵 **No-op message phrasing diverges** — Still present (suggestion).

#### Documentation (3 of 3 resolved)
- 🔵 **23-scenario section needs index** — Resolved (scenario index added; 34 scenarios grouped by operation).
- 🔵 **Author config key not named** — Resolved (`config-read-context.sh`'s `author` field).
- 🔵 **Subphase 6.0 example as indented prose** — Resolved (now fenced code block; matches Step 5 / Scenario 13).

#### Safety (1 of 2 resolved; 1 partial)
- 🔵 **Size in-place replace skips confirmation gate** — Resolved (diff + second y/n required).
- 🔵 **Multi-op consolidated ledger** — Partially resolved. Per-op status implicit; no consolidated end-of-flow status line documented.

#### Standards (5 of 5 resolved)
- 🔵 All five Pass-2 suggestions resolved: argument-hint divergence rationalised, fixture naming unified, script location pinned, `pass_rate.mean == 1.0` precondition-verified, `expected-target.md` / `expected-parent.md` naming consistent.

### New Issues Introduced

#### Major (1)

- 🟡 **Safety**: Pinned anchor `## Acceptance Criteria` may not be unique within parent body
  **Location**: Subphase 6B, Step 4a — Edit anchor `\n## Acceptance Criteria\n`
  Edit requires uniqueness — if the parent's Requirements prose, Technical Notes, or any code fence contains the same heading text (quoted ticket fragment, referenced template snippet, example block), Edit will fail with `not unique`, not the diagnostic the plan pins for `not found`. Same concern for the append-children case where the LAST `- NNNN — title` is the anchor: Requirements that mentions another ticket reference will produce a duplicate match. Recommended fix: change to a contextual multi-line `old_string` including a few lines of preceding context (last line of Requirements + blank line + `## Acceptance Criteria`), OR pre-check uniqueness in the parent before Edit and surface the same diagnostic when count is not exactly 1, OR extend Scenario 24's diagnostic-mapping to also cover the `not unique` Edit-tool error.

#### Minor (16)

**Architecture**:
- 🔵 **`scripts/` directory mixes config helpers, ticket helpers, and disparate test scripts** without categorisation; flag for future split into `scripts/config/` and `scripts/test/` once test-script count exceeds ~10.
- 🔵 **Tree-fence extraction heuristic in `test-hierarchy-format.sh`** is brittle: list-tickets uses abstract `NNNN — parent title`, refine-ticket uses literal `0042 — User Auth Rework`. Different anchors per file = exactly the cross-file drift the script should detect. Suggestion: add HTML-comment marker (`<!-- canonical-tree-fence -->`) in both SKILL.md files, OR require both files to use the same canonical example.
- 🔵 **Scenario 5a sub-scenario expansion creates implicit fixture coupling** — eight prompts each duplicate the proposal-state shape; suggestion: extract to a single `proposal-state.txt` fixture.

**Test Coverage**:
- 🔵 **5a sub-scenarios depend on simulated proposal state being honoured by the harness** — pattern not yet validated for refine-ticket; suggestion: encode as `[Step N: skill emitted: <proposal>]` history line per create-ticket Scenario 14 pattern, plus harness-pattern validation note.
- 🔵 **Scenario 24 section-mutation simulation reliability unspecified** — does the harness physically rewrite the fixture, intercept Edit, or instruct the model to behave as if read was stale? Suggestion: pin mechanism (two physical fixtures + harness file-swap, or assert tool-use trace shows Edit was attempted with the original old_string).
- 🔵 **Scenario 11b Glob-count-referenced-in-reasoning is a soft assertion** — drop reasoning-content claim; keep crisp tool-use observations only. Add paired Scenario 11c with exactly 30 tickets to bracket the threshold.
- 🔵 **Validator fixtures lack a stale-benchmark-scenario case** — scenario in benchmark.json but absent from evals.json. Also lacks malformed-benchmark-json (only malformed-evals-json present). Validator's containment direction is one-way; either tighten to set-equality or document the asymmetry.
- 🔵 **±60s recency check for `date` field has no specified mechanism** — eval harnesses don't typically ship a recency primitive. Suggestion: drop recency and rely on regex match (acknowledging looseness) or move recency assertion to post-hoc shell check.
- 🔵 **Tree-fence extraction self-tests missing** — add `mismatched-fences` and `empty-extraction` fixture pairs for `test-hierarchy-format.sh` itself; fail loudly on empty extraction.
- 🔵 **Cross-skill behavioural e2e still manual** — `test-hierarchy-format.sh` covers documentation drift only. Suggestion: add a 30-line shell test that creates parent + N children fixtures, runs list-tickets rendering in subshell, asserts byte-level tree match.

**Correctness**:
- 🔵 **Initial-decompose anchor edge cases unhandled** — trailing whitespace after `Acceptance Criteria`, anchor-occurs-twice (uniqueness failure), file ending without trailing newline. The pinned diagnostic assumes `not found` but Edit can also fail with `not unique`.
- 🔵 **Scenario 11b "single Bash invocation to glob" contradicts allowed-tools** — refine-ticket's allowed-tools only permits `Bash(scripts/config-*)` and `Bash(skills/tickets/scripts/*)`; neither admits a generic `ls`/`find`/`wc` invocation. Step 4e correctly says "via Glob" but Scenario 11b says "Bash invocation". Reword Scenario 11b to "single `Glob` invocation".

**Usability**:
- 🔵 **Size in-place replace adds friction to a previously frictionless re-run** — accepted as written for destructive-path consistency. Suggestion: render the diff inline as a single colored before→after line rather than a full `diff -u` block.

**Documentation**:
- 🔵 **5a sub-scenario count internally inconsistent** — Scenario 5a's intro prose says "seven focused sub-scenarios" but the index and grammar list eight (with -legend). Off-by-one error.
- 🔵 **Stress-test-ticket section lacks scenario index** — refine-ticket has one (added Pass-3); stress-test should have parity.
- 🔵 **"34 scenarios" vs "34 evals" terminology unclear** — Scenario 5a is one numbered entry but expands to 8 evals; the totals should be reconciled (e.g. "34 evals across 24 scenario blocks").

**Safety**:
- 🔵 **Append-children anchor disambiguation strategy described but not pinned as a concrete `old_string`** — suggestion: pin the literal multi-line construction `'<last-child-link>\n\n## Acceptance Criteria\n'` to use the H2-immediately-after structural invariant for disambiguation.

**Standards**:
- 🔵 **Minor wording redundancy** in fixture-naming convention — `expected-parent.md` is special-case override of `expected-target.md`. Tighten parenthetical to a one-line decision rule.

#### Suggestions (3)

- 🔵 **Usability**: Lifecycle ordering paragraphs end with different anchors (`/create-plan` vs `/update-ticket`) — pick one canonical 6-entry ordering and use verbatim in both skills.
- 🔵 **Usability**: 5a-legend repetition after every `edit N` and unrecognised command may produce cumulative noise — relax to "restate after unrecognised input only", keeping FIRST-turn requirement.
- 🔵 **Architecture**: Promote `pass_rate.mean` threshold to a per-skill `evals.json` field (e.g. `expected_pass_rate: 1.0`) so the contract lives with the skill rather than the validator (follow-up).

### Assessment

The plan has matured from "needs significant revision" (Pass 1) to "needs targeted polish" (Pass 2) to "acceptable as-is, with one targeted fix recommended" (Pass 3). The single new major (anchor uniqueness in Edit) is concrete, well-scoped, and has three viable fixes outlined above; addressing it would land a clean APPROVE on a fourth pass. Most Pass-3 minors are about robustness of new test infrastructure (extraction heuristics, simulation primitives, validator coverage edges) — appropriate to capture as follow-up work or polish during implementation.

Recommended pre-implementation fix:
1. **Address the anchor uniqueness concern** — pick one of: (a) multi-line contextual `old_string`, (b) pre-Edit uniqueness check, or (c) extend Scenario 24's diagnostic-mapping to cover `not unique`. This is the only finding that materially affects implementation correctness on real tickets.

Recommended polish (any subset):
2. Fix Scenario 11b's "Bash invocation" → "Glob invocation" wording (correctness consistency).
3. Fix Scenario 5a "seven" → "eight" sub-scenario count (off-by-one).
4. Add stress-test-ticket scenario index for parity.
5. Reconcile "34 scenarios" vs "34 evals" terminology.
6. Add `<!-- canonical-tree-fence -->` HTML markers in both SKILL.md files for the drift-check extractor.

Defer to follow-up work:
- Test-coverage robustness on simulated harness primitives (5a state, 24 section-mutation, 11b, recency, tree extraction self-tests, validator stale-benchmark case)
- Cross-skill behavioural e2e shell test
- `scripts/` categorisation refactor
- Lifecycle paragraph alignment
- 5a-legend repetition relaxation
- `pass_rate` threshold promotion to evals.json
- Usability concerns (cascade fatigue, menu density, mojibake fallback, no-op phrasing) — explicitly excluded from the Pass-3 bundle by the user

The plan is implementable as-is; the anchor fix should be made before handing off to skill-creator.

