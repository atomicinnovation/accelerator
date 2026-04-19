---
date: "2026-04-19T00:00:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-19-ticket-creation-skills.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, correctness, test-coverage, standards, usability, safety]
review_pass: 2
status: complete
---

## Plan Review: Ticket Creation Skills (Phase 2)

**Verdict:** REVISE

The plan is well-structured and follows the established plugin conventions at a high
level — the two-skill decomposition mirrors the ADR category pattern faithfully and
the TDD approach with skill-creator evals is sound in principle. However, three
categories of issue require revision before implementation:

1. A critical correctness gap in `extract-tickets` (unhandled `--count 0` crash when
   the user skips all candidates) and a pervasive inconsistency in `create-ticket`
   (ticket number allocated before approval, contrary to the safer deferred pattern
   used by `extract-tickets`).
2. Two YAML frontmatter conventions are wrong in the skill specifications: `allowed-tools`
   uses a block scalar instead of the inline format, and the agent fallback list omits
   the required `accelerator:` namespace prefix — both would produce broken skills if
   followed literally.
3. The eval scenarios have significant coverage gaps (no epic type scenario, no type
   inference scenario for `extract-tickets`, section names that don't exist in the
   template) that would allow a passing eval run to miss important correctness defects.

### Cross-Cutting Themes

- **Early ticket number allocation** (Architecture, Usability, Safety) — `create-ticket`
  calls `ticket-next-number.sh` in Step 3 before the approval loop, creating gap risk,
  collision risk, and usability inconsistency with `extract-tickets`. Fix: defer
  numbering to Step 4 (after approval), using XXXX as placeholder during review —
  identical to `extract-tickets`' correct pattern.
- **Eval scenario gaps for critical behaviours** (Test Coverage, Correctness) — The epic
  type, type inference, and the all-skip (`N=0`) case all have no scenario to catch
  them. The scenarios that do exist often use section names that don't match the
  template.
- **YAML convention violations** (Standards) — Block scalar `>` for `allowed-tools` and
  missing `accelerator:` namespace prefix in the agent fallback list are both present
  in both skill specifications and would both produce broken output if used verbatim.

### Tradeoff Analysis

- **Deferred numbering vs. numbered draft display**: Moving the `ticket-next-number.sh`
  call to Step 4 means the draft shows `XXXX` rather than the real number during
  review. This is a minor UX degradation (user sees `XXXX-my-ticket.md` in the draft)
  but is the correct safety tradeoff and is already the pattern in `extract-tickets`.
  The plan should make this explicit.

### Findings

#### Critical

- 🔴 **Correctness**: `ticket-next-number.sh --count 0` is invalid but unhandled
  **Location**: Subphase 2.2 — extract-tickets, Skill flow Step 4
  If the user selects candidates but skips all of them during draft review, N=0 is
  passed to `ticket-next-number.sh --count 0`, which rejects 0 as an invalid count
  and exits non-zero. The skill will receive a script error rather than completing
  cleanly. Fix: add an explicit guard before Step 4 — if N=0, skip the numbering
  call and print "No tickets approved — nothing written."

#### Major

- 🟡 **Architecture / Usability / Safety**: `create-ticket` allocates ticket number before approval — deferred numbering needed
  **Location**: Subphase 2.1 — Skill flow Steps 3 and 4
  `ticket-next-number.sh` is called at the start of Step 3 (draft), before the
  approval loop. This creates three compounding risks: (a) if the session is
  abandoned, the number is consumed with no file written; (b) under concurrent
  sessions, both obtain the same number and the second write silently overwrites
  the first; (c) it is inconsistent with the correctly deferred pattern in
  `extract-tickets`. The `create-adr` reference also has an explicit pre-write path
  existence check ("verify the target path does not already exist") that the
  specification does not carry forward. Fix: use `XXXX` as placeholder during draft
  review; call `ticket-next-number.sh` in Step 4 immediately before writing; add a
  pre-write path existence check.

- 🟡 **Correctness**: Eval scenarios 3 and 8 reference section names that do not exist in `templates/ticket.md`
  **Location**: Subphase 2.1 — Eval Scenarios 3 and 8
  Scenario 3 expects "The draft uses the `Reproduction Steps` section". Scenario 8
  expects "Requirements/Reproduction Steps and Acceptance Criteria/Research Questions
  sections are non-empty." The template has no `Reproduction Steps` or `Research
  Questions` sections — it has a `Requirements` section with type-specific inline
  guidance and a single `Acceptance Criteria` section. These scenarios will
  conflict with the template the skill is instructed to follow.

- 🟡 **Test Coverage**: Epic type has no eval scenario despite distinct structural requirements
  **Location**: Subphase 2.1 — Eval Scenarios
  Scenarios 3 and 4 cover bug and spike; the epic type — which has a distinct
  `Stories` decomposition section and different clarifying questions — has no
  corresponding scenario. A SKILL.md that omits the epic's Stories section passes
  all 8 evals.

- 🟡 **Test Coverage**: `extract-tickets` type inference has no eval scenario
  **Location**: Subphase 2.2 — Eval Scenarios
  The flow spec describes a four-branch type inference rule, but none of the 8
  scenarios test any inference path. A skill that always assigns `type: story`
  (ignoring all inference rules) passes all 8 evals.

- 🟡 **Test Coverage**: Ticket number gap has no scenario
  **Location**: Subphase 2.1 — Eval Scenarios
  No scenario tests that a cancelled or abandoned draft session does not leave a
  consumed number with no corresponding file. Without this, the early-allocation
  design flaw goes undetected by the eval suite.

- 🟡 **Standards**: `allowed-tools` specified as YAML block scalar instead of inline single-line format
  **Location**: Subphase 2.1 and 2.2 — Changes Required, Frontmatter spec
  Every existing skill uses inline format:
  `allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(...)`.
  The plan uses `>` folded block scalar for both skills. If followed literally,
  the generated SKILL.md will diverge from convention and may break the plugin's
  tool permission parser.

- 🟡 **Standards**: Agent fallback list omits `accelerator:` namespace prefix
  **Location**: Subphase 2.1 and 2.2 — Changes Required, Agent fallback block
  All three reference skills use fully-qualified names (`accelerator:reviewer`,
  `accelerator:codebase-locator`, etc.). The plan specifies bare names (`reviewer`,
  `codebase-locator`, etc.) for `create-ticket` and defers to this list for
  `extract-tickets`. Agent dispatch will fail at runtime.

- 🟡 **Usability**: Ticket type asked before context gathering
  **Location**: Subphase 2.1 — Skill flow Steps 1 and 2
  The flow asks for ticket type (Step 1) before spawning context agents (Step 2).
  The `create-adr` model gathers context before clarifying questions because
  discovered context often changes the answer. The user may commit to `story` then
  discover context showing it should be an `epic` or is already tracked elsewhere.

- 🟡 **Usability**: No guidance on vague topics or near-duplicate existing tickets
  **Location**: Subphase 2.1 — Eval Scenarios and Quality Guidelines
  The research (§5.2) describes a "Socratic approach: challenges vague requirements".
  Neither the skill specification nor any eval scenario describes what happens when a
  topic is extremely vague or when context gathering finds a nearly-identical existing
  ticket. A skill authored from this spec may silently accept poor input.

#### Minor

- 🔵 **Architecture**: Type inference logic not anchored to the template as authoritative source
  **Location**: Subphase 2.2 — Skill flow Step 3
  Type inference rules are hardcoded in the skill prose. If the ticket type taxonomy
  evolves, the template and the skill diverge silently.

- 🔵 **Architecture**: No overwrite guard for `extract-tickets` batch writes
  **Location**: Subphase 2.2 — Skill flow Step 4
  The `create-adr` pre-write path existence check is not specified for the batch write
  path in `extract-tickets`. A concurrent session completing between `--count N` and
  the batch write could silently overwrite files.

- 🔵 **Correctness**: Template described as having seven frontmatter fields; actual count is eight
  **Location**: Current State Analysis
  `templates/ticket.md` has eight fields (`ticket_id`, `date`, `author`, `type`,
  `status`, `priority`, `parent`, `tags`). The plan says seven.

- 🔵 **Correctness**: Template injection placement spec differs from `create-adr` model
  **Location**: Subphase 2.1 — Changes Required, Template injection
  The spec says "at draft step" (implying conditional injection); `create-adr`
  uses a static always-present `## ADR Template` section in the skill body.

- 🔵 **Correctness**: `approve-all-remaining` path could trigger writes in Step 3 before Step 4 numbering call
  **Location**: Subphase 2.2 — Skill flow Steps 3 and 4
  Step 3 says "write remaining without further prompts" for approve-all-remaining,
  but the write step is Step 4. If interpreted literally, files would be written in
  Step 3 before the `--count N` call, corrupting the numbering.

- 🔵 **Test Coverage**: Frontmatter field completeness not verified in any scenario
  **Location**: Subphase 2.1 — Eval Scenarios, Scenario 6
  Scenario 6 checks path/naming but not that all eight frontmatter fields are
  populated with plausible values (e.g., `status: draft`, `ticket_id` matching NNNN).

- 🔵 **Test Coverage**: `approve-all-remaining` shortcut not verified as a scenario
  **Location**: Subphase 2.2 — Eval Scenarios, Scenario 5
  Scenario 5 lists it as an option but does not verify that selecting it actually
  bypasses subsequent per-draft confirmations.

- 🔵 **Test Coverage**: No scenario covers zero candidates extracted from a document
  **Location**: Subphase 2.2 — Eval Scenarios
  A document with only structural content and no actionable items is a realistic
  input. No scenario verifies the skill exits cleanly rather than crashing.

- 🔵 **Test Coverage**: Several scenario assertions too vague for meaningful LLM evals
  **Location**: Subphase 2.1 and 2.2 — Eval Scenarios (multiple)
  Assertions like "Does not ask the user to supply context that the agents can
  discover" and "substantive content" lack observable pass/fail criteria. LLM evals
  need concrete verifiable assertions.

- 🔵 **Standards**: `description` field uses `>` block scalar; reference skills use plain multi-line
  **Location**: Subphase 2.1 and 2.2 — Changes Required, Frontmatter spec
  Minor style inconsistency; low functional risk but diverges from convention.

- 🔵 **Standards**: Integration verification does not check agent fallback prefix
  **Location**: Subphase 2.3 — Verification Steps
  The automated checks grep for `allowed-tools` and `disable-model-invocation` but
  not for `accelerator:reviewer` (which would catch the missing namespace prefix).

- 🔵 **Usability**: Epic clarifying questions underspecified
  **Location**: Subphase 2.1 — Skill flow Step 2, epic questions
  Only one question is specified for epics ("What are the initial stories?").
  The other types get 3–4 questions. The epic template has `Summary`, `Context`,
  `Requirements`, and `Stories` sections — all need input.

- 🔵 **Usability**: `approve-all-remaining` UX unclear when drafts generated on-demand
  **Location**: Subphase 2.2 — Skill flow Step 3
  If drafts are generated one at a time, "approve all remaining" at draft #2 commits
  the user to unseen drafts. Spec should clarify whether all drafts are pre-generated.

- 🔵 **Usability**: Confirmation message references `/review-ticket` and `/refine-ticket`, which don't exist in Phase 2
  **Location**: Subphase 2.1 — Skill flow Step 4
  Directing users to non-existent skills at the moment of success creates friction.
  Omit the next-step suggestions until those skills are implemented.

- 🔵 **Usability**: Deduplication UX presentation unspecified for multi-document scans
  **Location**: Subphase 2.2 — Skill flow Step 2
  When the same requirement appears in two documents, which source is cited? The user
  is not shown which document(s) a deduplicated item came from.

- 🔵 **Safety**: Partial write failure in `extract-tickets` batch leaves numbering inconsistent
  **Location**: Subphase 2.2 — Skill flow Step 4
  If writing 5 files and the third fails, files 1–2 are on disk with consumed numbers;
  files 3–5 are absent. No recovery guidance is specified.

- 🔵 **Safety**: Neither skill handles `ticket-next-number.sh` exiting non-zero due to 9999 overflow
  **Location**: Subphase 2.1 and 2.2 — Skill flow Step 3/4
  If the script exits 1 and emits partial output, a skill that proceeds will assign
  wrong or missing numbers. Quality guidelines should instruct: abort on non-zero
  exit and surface the error message verbatim.

### Strengths

- ✅ The `./skills/tickets/` plugin.json entry already covers all subdirectories —
  no further registration work needed for any Phase 2 skill.
- ✅ The `allowed-tools` scope (config-* and tickets/scripts/* only) is correctly
  constrained and the absence of `research-codebase/scripts/*` is correctly justified.
- ✅ The batch-numbering strategy for `extract-tickets` (collect all approvals,
  then one `--count N` call, sequential substitution) is the right pattern and
  correctly carried over from `extract-adrs`.
- ✅ `disable-model-invocation: true` is specified for both skills and included
  in the automated verification checks.
- ✅ The configuration preamble order (config-read-context → config-read-skill-context
  → config-read-agents) is correctly specified for both skills.
- ✅ Instructions injection is correctly placed at the end of the file for both skills.
- ✅ Path injection format follows the bold-label convention from `create-adr`/`extract-adrs`.
- ✅ TDD order is explicit and correctly sequenced — scenarios defined in the plan
  before implementation begins, enforcing specification-first discipline.
- ✅ The Phase 1 regression suite (44 tests) is used as a gate after every subphase.
- ✅ The plan correctly identifies that `research-metadata.sh` is not needed, avoiding
  an unnecessary cross-category dependency.
- ✅ The structural symmetry between create-ticket/create-adr and extract-tickets/extract-adrs
  reduces cognitive overhead for future maintainers.

### Recommended Changes

Ordered by impact:

1. **Fix the --count 0 crash** (addresses: Correctness critical — `--count 0` unhandled)
   In Subphase 2.2 Skill flow Step 4, add: "If N is 0 (all candidates were skipped),
   skip the numbering call and print 'No tickets approved — nothing written.' This is
   a valid terminal state, not an error."

2. **Defer create-ticket numbering to Step 4** (addresses: Architecture/Usability/Safety merged, Test Coverage gap)
   Replace the Step 3 `ticket-next-number.sh` call with XXXX placeholder throughout
   the draft/review loop. Move the call to Step 4 (immediately before writing). Add a
   pre-write path existence check matching `create-adr`'s Important Note. Add a
   Scenario 9: "Abandoned draft does not consume a ticket number — XXXX is still
   available for the next invocation."

3. **Fix eval scenario section names** (addresses: Correctness major — wrong section names)
   Rewrite Scenario 3: "The Requirements section is populated with reproduction steps,
   expected behaviour, actual behaviour, and environment — not acceptance criteria."
   Rewrite Scenario 8: "The Summary, Requirements, and Acceptance Criteria sections
   have substantive content appropriate to the ticket type (no unfilled `[...]` text)."

4. **Fix allowed-tools YAML format** (addresses: Standards major — block scalar)
   Change both allowed-tools entries to inline format:
   `allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)`

5. **Add accelerator: prefix to agent fallback list** (addresses: Standards major — missing prefix)
   Change the fallback list to:
   `accelerator:reviewer, accelerator:codebase-locator, accelerator:codebase-analyser,
   accelerator:codebase-pattern-finder, accelerator:documents-locator,
   accelerator:documents-analyser, accelerator:web-search-researcher`

6. **Add epic type eval scenario** (addresses: Test Coverage major — epic missing)
   Add Scenario 9 (or 10 after numbering changes): "Input: Topic with type `epic`.
   Expected: Skill asks for initial story decomposition and high-level goals. The
   draft includes a Stories section."

7. **Add type inference eval scenario for extract-tickets** (addresses: Test Coverage major)
   Add a scenario: "Input: Document containing a clear bug report. Expected: The
   extracted draft has `type: bug` and the Requirements section contains reproduction
   steps rather than acceptance criteria."

8. **Move type selection after context gathering** (addresses: Usability major — type before context)
   In the `create-ticket` skill flow, restructure so context agents are spawned
   immediately after the topic is known (Step 1), and the type question is asked
   alongside the context summary (new Step 2: "Based on what I found, what type is
   this — story, epic, task, bug, or spike?").

9. **Add guidance for vague topics and near-duplicates** (addresses: Usability major)
   Add a quality guideline: "If the topic is vague (no clear deliverable), challenge
   the user with 'What does done look like?' before proceeding to type selection. If
   context gathering finds an existing ticket with a highly similar title, surface it
   and ask the user to confirm they are not creating a duplicate."

10. **Clarify approve-all-remaining generates all drafts first** (addresses: Correctness
    minor — ambiguous write phase, Usability minor — on-demand generation)
    Add to Step 3 spec: "Generate all selected drafts before beginning the review
    loop. Present them one at a time. 'Approve all remaining' marks remaining drafts
    approved without further display — the actual write occurs in Step 4 as normal."

11. **Remove non-existent next-step suggestions from create-ticket confirmation**
    (addresses: Usability minor)
    Remove the "e.g., /review-ticket, /refine-ticket" from Step 4. Replace with just
    the written file path and a generic "you can now reference this ticket in plans."

12. **Add agent fallback prefix check to Subphase 2.3** (addresses: Standards minor)
    Add: `grep 'accelerator:reviewer' skills/tickets/create-ticket/SKILL.md` and the
    same for `extract-tickets` as automated verification steps.

---

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally well-conceived: it follows established conventions,
keeps the two new skills structurally symmetric with their ADR counterparts, and correctly
identifies the plugin registration mechanism as already sufficient. The primary architectural
concern is a race condition in the `create-ticket` numbering design. A secondary concern is
that type inference rules are embedded in the skill prose rather than anchored to the template.

**Strengths**:
- Plugin registration already covers subdirectories; no further changes needed
- allowed-tools scope correctly constrained to least-privilege
- Deferred batch-numbering strategy for extract-tickets is the right pattern
- research-metadata.sh correctly excluded
- disable-model-invocation: true present and verified
- Structural symmetry with ADR skills reduces maintenance overhead

**Findings**:
- 🟡 MAJOR (high): Ticket number reserved at draft time — Location: Subphase 2.1 Skill flow Step 3
- 🔵 MINOR (high): Type inference logic not anchored to template — Location: Subphase 2.2 Step 3
- 🔵 MINOR (medium): Agent fallback includes unused agents — Location: Subphase 2.1 Step 2
- 🔵 MINOR (medium): No overwrite guard for extract-tickets batch files — Location: Subphase 2.2 Step 4

---

### Correctness

**Summary**: Logically sound on the happy path but has several correctness gaps. The most
significant is the --count 0 crash. Two eval scenarios describe section names that don't
exist in the template. A factual count error (7 vs 8 fields) is minor but could cause
confusion.

**Strengths**:
- Batch numbering invariant correctly specified
- --count N mechanism correctly understood
- TOCTOU handling consistent with create-adr
- allowed-tools correctly scoped
- Quality guideline explicitly forbids early --count N call

**Findings**:
- 🔴 CRITICAL (high): --count 0 unhandled when all items skipped — Location: Subphase 2.2 Step 4
- 🟡 MAJOR (high): Eval scenarios 3 and 8 reference nonexistent section names — Location: Subphase 2.1 Scenarios 3 and 8
- 🔵 MINOR (high): 7 vs 8 frontmatter fields — Location: Current State Analysis
- 🔵 MINOR (medium): Template injection placement differs from create-adr model — Location: Subphase 2.1 Template injection
- 🔵 MINOR (medium): approve-all-remaining could trigger writes before numbering step — Location: Subphase 2.2 Steps 3 and 4

---

### Test Coverage

**Summary**: Solid foundation (TDD order, 44-test regression harness, multi-layer gates) but
significant gaps: epic type, type inference, and the all-skip case all have no scenarios. The
early-allocation design flaw in create-ticket has no scenario to catch it. Scenario assertions
are often too vague for meaningful LLM evals.

**Strengths**:
- TDD order explicit and correct
- Phase 1 regression suite as gate after every subphase
- Scenarios 6 and 7 tightly specify numbering contract
- Success criteria multi-layer (automated + manual)
- Automation limitations acknowledged

**Findings**:
- 🟡 MAJOR (high): Epic type has no eval scenario — Location: Subphase 2.1 Eval Scenarios
- 🟡 MAJOR (high): Ticket number gap has no scenario — Location: Subphase 2.1 Scenarios/Step 3
- 🟡 MAJOR (high): Type inference has no eval scenario — Location: Subphase 2.2 Eval Scenarios
- 🔵 MINOR (high): Frontmatter field completeness not verified — Location: Subphase 2.1 Scenario 6
- 🔵 MINOR (high): approve-all-remaining not verified as scenario — Location: Subphase 2.2 Scenario 5
- 🔵 MINOR (high): No scenario for zero candidates — Location: Subphase 2.2 Eval Scenarios
- 🔵 MINOR (high): Scenario assertions too vague for LLM evals — Location: Multiple scenarios
- 🔵 MINOR (medium): No coverage for allowed-tools boundary enforcement — Location: Testing Strategy

---

### Standards

**Summary**: Close adherence to conventions overall, with correct preamble order, injection
placement, and path format. Two YAML frontmatter deviations — block scalar allowed-tools and
missing accelerator: prefix — would produce broken output if followed literally.

**Strengths**:
- Configuration preamble order correct
- Instructions injection at end of file
- Template injection placement correct (matches create-adr body position)
- research-codebase/scripts/* correctly excluded from allowed-tools
- Path injection follows bold-label convention
- All five frontmatter fields in correct order

**Findings**:
- 🟡 MAJOR (high): allowed-tools block scalar instead of inline format — Location: Subphase 2.1 and 2.2 Frontmatter specs
- 🟡 MAJOR (high): Agent fallback missing accelerator: prefix — Location: Subphase 2.1 and 2.2 Fallback blocks
- 🔵 MINOR (medium): description uses > block scalar — Location: Subphase 2.1 and 2.2 Frontmatter specs
- 🔵 MINOR (medium): Integration check doesn't validate agent fallback prefix — Location: Subphase 2.3

---

### Usability

**Summary**: Broadly well-structured interaction flows consistent with the reference models.
Primary concerns: type selection before context gathering reverses the natural information
order; ticket numbering before approval creates gaps; no guidance for vague or duplicate
topics.

**Strengths**:
- extract-tickets correctly defers numbering
- Both skills enforce never-write-without-approval
- Candidate list format scannable for batch decisions
- Full approve/revise/skip/approve-all decision space
- Deduplication requirement anticipates real user frustration
- Source reference requirement provides traceability
- Bare invocation handled gracefully
- TDD codifies usability expectations as acceptance criteria

**Findings**:
- 🟡 MAJOR (high): Type asked before context gathering — Location: Subphase 2.1 Steps 1 and 2
- 🟡 MAJOR (high): Ticket number consumed before approval — Location: Subphase 2.1 Step 3
- 🟡 MAJOR (high): No guidance for vague topics or near-duplicates — Location: Subphase 2.1 Eval Scenarios
- 🔵 MINOR (high): Epic clarifying questions underspecified — Location: Subphase 2.1 Step 2
- 🔵 MINOR (medium): approve-all-remaining unclear for on-demand draft generation — Location: Subphase 2.2 Step 3
- 🔵 MINOR (high): Confirmation references non-existent skills — Location: Subphase 2.1 Step 4
- 🔵 MINOR (low): Deduplication UX presentation unspecified — Location: Subphase 2.2 Step 2

---

### Safety

**Summary**: Correctly structured for a local developer tool. The deferred --count N pattern
is right. Key gaps: create-ticket lacks the pre-write collision check that its create-adr
model explicitly requires; neither skill handles non-zero exit from ticket-next-number.sh.

**Strengths**:
- extract-tickets deferred --count N correctly timed
- Both skills create directory if missing
- ticket-next-number.sh handles missing directory gracefully
- Quality guideline explicitly forbids early numbering
- 9999 overflow emits actionable error

**Findings**:
- 🟡 MAJOR (high): Number obtained before review loop with no pre-write collision check — Location: Subphase 2.1 Steps 3 and 4
- 🟡 MAJOR (high): Stale number risk on long-lived create-ticket sessions — Location: Subphase 2.1 Steps 3 and 4 (merged with above in summary)
- 🔵 MINOR (high): Partial write failure leaves numbering inconsistent — Location: Subphase 2.2 Step 4
- 🔵 MINOR (high): Neither skill handles ticket-next-number.sh non-zero exit — Location: Subphase 2.1 and 2.2 Steps 3/4

---

## Re-Review (Pass 2) — 2026-04-19

**Verdict:** COMMENT

The plan is now acceptable for implementation. All 1 critical and 8 of the 9
major findings from pass 1 are resolved. The previously blocking issues — the
`--count 0` crash, early number allocation in `create-ticket`, missing YAML
namespace prefixes, wrong eval section names, and missing type/epic eval
scenarios — are all addressed. Two major findings remain that the implementer
should be aware of, and a collection of minor polish items was identified.

### Previously Identified Issues

- ✅ **Architecture**: Ticket number reserved at draft time — **Resolved**. Numbering deferred to Step 4 with XXXX placeholder throughout.
- ✅ **Architecture**: No overwrite guard for extract-tickets batch — **Resolved**. Per-file pre-write check added to Step 4.
- ✅ **Architecture**: Type inference not anchored to template — **Resolved**. Both skills now read valid types from the template frontmatter.
- ✅ **Correctness**: `--count 0` crash when all items skipped — **Resolved**. Explicit N=0 guard in Step 4 and quality guidelines.
- 🟡 **Correctness**: Eval scenarios 3 and 8 wrong section names — **Partially resolved**. Scenarios 3 and 4 are fixed; Scenario 8 still directs the skill to produce a `Stories` section that does not exist in `templates/ticket.md`. Quality guidelines also reference "Stories subsection".
- ✅ **Correctness**: Seven vs eight frontmatter fields — **Resolved**.
- ✅ **Correctness**: Template injection placement — **Resolved**.
- ✅ **Correctness**: `approve-all-remaining` triggering writes before Step 4 — **Resolved**.
- ✅ **Test Coverage**: Epic type has no scenario — **Resolved**. Scenario 8 added.
- ✅ **Test Coverage**: Number gap has no scenario — **Resolved**. Scenario 9 added.
- ✅ **Test Coverage**: Type inference has no scenario — **Resolved**. extract-tickets Scenario 9 added.
- ✅ **Test Coverage**: Scenario assertions too vague — **Largely resolved**. Most scenarios now have concrete observable assertions.
- ✅ **Test Coverage**: No scenario for zero candidates — **Resolved**. extract-tickets Scenario 11 added.
- ✅ **Standards**: `allowed-tools` block scalar format — **Resolved**. Inline format used.
- ✅ **Standards**: Agent fallback missing `accelerator:` prefix — **Resolved**. Prefix present throughout and noted in Key Discoveries.
- ✅ **Standards**: `description` uses `>` block scalar — **Resolved**.
- ✅ **Standards**: Integration check doesn't validate agent prefix — **Resolved**. `grep "accelerator:reviewer"` checks added.
- ✅ **Usability**: Type asked before context gathering — **Resolved**. Step 1 now gathers context; Step 2 selects type.
- ✅ **Usability**: Number consumed before approval — **Resolved**. Deferred to write step.
- ✅ **Usability**: No guidance for vague topics / near-duplicates — **Resolved**. Step 0, Step 1, Scenarios 10 and 11 added.
- ✅ **Usability**: Epic questions underspecified — **Resolved**. Three questions now specified.
- ✅ **Usability**: `approve-all-remaining` UX unclear — **Resolved**. Pre-generation mandated; write deferred to Step 4.
- ✅ **Usability**: Confirmation references non-existent skills — **Resolved**.
- ✅ **Usability**: Deduplication UX unspecified — **Resolved**. Source documents shown per candidate.
- ✅ **Safety**: Number allocated before review loop — **Resolved**.
- ✅ **Safety**: Stale number risk — **Resolved**.
- ✅ **Safety**: `ticket-next-number.sh` non-zero exit unhandled — **Resolved**. Abort-and-surface-error in both skills.
- 🔵 **Safety**: Partial write failure leaves inconsistent state — **Partially resolved**. Path-collision mid-batch is now handled; true I/O failure mid-batch still leaves consumed slots with no recovery guidance.

### New Issues Introduced

- 🟡 **Architecture**: `extract-tickets` pre-checks path existence mid-write rather than before the `ticket-next-number.sh` call — if a collision is discovered after numbering, the unwritten tickets' numbers are consumed but no files exist. Fix: verify all target paths are free *before* calling `--count N`.
- 🔵 **Correctness**: epic `Stories` section (Scenario 8, quality guidelines) still references a non-existent template section — should use `Requirements` like bug and spike types do.
- 🔵 **Test Coverage**: Frontmatter field completeness not verified for extract-tickets path (only create-ticket Scenario 6 checks this).
- 🔵 **Test Coverage**: Concurrent write collision path has no eval scenario in either skill.
- 🔵 **Test Coverage**: extract-tickets Scenario 5 `approve-all-remaining` doesn't explicitly assert `ticket-next-number.sh` is not called at that point.
- 🔵 **Standards**: `description` continuation uses 4-space indent; reference skills use 2-space.
- 🔵 **Usability**: Vagueness check only specified for the argument path, not bare-invocation responses.
- 🔵 **Usability**: Offering to "update the existing ticket" in Step 1 is outside `allowed-tools` scope — skill cannot fulfil that option.
- 🔵 **Usability**: `revise` action in extract-tickets batch review loop has no specified behaviour.
- 🔵 **Safety**: Partial-batch collision stop does not surface the pre-assigned numbers for unwritten tickets, making manual recovery harder.

### Assessment

The plan is ready to implement. All findings from pass 2 were subsequently
addressed in a further round of targeted edits:

- Scenario 8 and quality guidelines updated: epic story decomposition goes in
  `Requirements`, not a non-existent `Stories` section
- `extract-tickets` Step 4 restructured: all N target paths verified free
  *before* calling `ticket-next-number.sh --count N`, preventing orphaned
  number slots on collision
- Mid-batch write error now surfaces allocated numbers, written files, and
  unwritten files for manual recovery
- `create-ticket` Step 0 vagueness check extended to bare-invocation responses
- Step 1 near-duplicate option corrected: skill offers to exit (not update
  inline, which is outside `allowed-tools` scope)
- Step 3 of `extract-tickets` now specifies `revise` behaviour: accept
  instructions, update draft, re-present before advancing
- Scenario 5 of `extract-tickets` now explicitly asserts `ticket-next-number.sh`
  is not called during `approve-all-remaining`
- `description` continuation indent corrected to 2-space (matching reference skills)
- `extract-tickets` success criteria now includes frontmatter field completeness check

---

*Review generated by /review-plan*
