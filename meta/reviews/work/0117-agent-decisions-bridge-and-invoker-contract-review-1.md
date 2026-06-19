---
type: work-item-review
id: "0117-agent-decisions-bridge-and-invoker-contract-review-1"
title: "Work Item Review: Agent-Decisions Bridge and Documented Invoker Contract"
date: "2026-06-20T11:02:01+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0117"
relates_to: ["work-item:0115", "work-item:0116", "work-item:0118"]
work_item_id: "0117"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 4
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-20T12:42:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Agent-Decisions Bridge and Documented Invoker Contract

**Verdict:** REVISE

Work item 0117 is structurally complete, well-anchored to its code surfaces,
and bundles three genuinely interdependent deliverables into one coherent unit
of work — the scope and completeness lenses found little to fault. Two areas
pull the verdict to REVISE: the acceptance criteria are under-specified and in
several places unverifiable (unbounded "every", no reference corpus, a
documentation-presence criterion with no checklist, and no criterion for the
positional list↔verb ordering the whole bridge depends on), and the
sibling-task couplings are captured asymmetrically — 0116 and 0118 each record
their relationship to 0117, but 0117 declares "Blocked by: none" and lists them
only as loose "Relates to". Tightening the acceptance criteria and recording the
real ordering/precondition dependencies on 0116 and 0118 would clear the path to
implementation.

### Cross-Cutting Themes

- **Asymmetric sibling coupling** (flagged by: dependency, scope, clarity) —
  0117 is fix A decomposed from epic 0115, but its relationship to siblings is
  recorded only from the siblings' side. 0116 (the structured stall this
  documents) and 0118 (fix C, which lets 0007 reach its interactive phase at
  all) both capture their coupling to 0117 reciprocally; 0117 captures neither
  as a blocker. The clarity lens independently flagged that the "fix A" framing
  contradicts the source research's recommendation (C+D+B, with A as a
  follow-up) without explaining the override, and the scope lens noted the same
  rationale gap — together these say the *why* and *when* of this item's place
  in the 0115 bundle is not self-contained.

- **Under-specified, partly unverifiable acceptance criteria** (flagged by:
  testability, completeness) — three of the five testability findings and the
  one completeness finding converge on the AC section: "every pending
  transformation" has no reference corpus to check completeness against, the
  "context" field is undefined, AC3's documentation outcome has no enumerable
  pass condition, the positional list↔verb ordering (an explicit Assumption)
  has no AC, and the promotion of `ACCELERATOR_MIGRATE_DECISIONS_FILE` to a
  user-facing interface has no AC of its own.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency**: Documentation content-dependency on 0116's structured stall not captured as a blocker
  **Location**: Dependencies
  AC3 requires the SKILL.md contract to document "what happens when it cannot
  (links to the structured stall from 0116)", yet Dependencies asserts "Blocked
  by: none" and lists 0116 only under "Relates to". 0116 records the reciprocal
  ordering constraint explicitly, so the coupling is captured asymmetrically.

- 🟡 **Dependency**: End-to-end completion criterion has an uncaptured functional precondition on 0118 (fix C)
  **Location**: Acceptance Criteria
  AC2 requires the list→decide→resume flow to complete against an interactive
  migration; the research shows 0007 hard-fails in `self_validate_structural`
  before its interactive phase until fix C (0118) lands. 0116 captures this
  precondition; 0117 names neither 0118 nor 0119 anywhere.

- 🟡 **Testability**: AC1 "every pending transformation" is unbounded without a reference corpus
  **Location**: Acceptance Criteria
  No fixture corpus or expected enumeration is defined, so a verifier has no
  ground truth to confirm completeness — a `--list` that omits a transformation
  class would still appear to pass against any single example.

- 🟡 **Testability**: List-to-verb ordering correspondence has no Acceptance Criterion
  **Location**: Assumptions
  The Assumptions section makes the positional list↔verb mapping the core
  contract AC2 depends on, yet no AC verifies that the Nth `--list` entry
  consumes the Nth decisions-file verb. AC2 could pass on an accept-all fixture
  while an off-by-one misalignment goes undetected.

- 🟡 **Testability**: AC3 documentation criterion lacks an enumerable pass condition
  **Location**: Acceptance Criteria
  AC3 ("the invoker contract … is documented") is a presence outcome with no
  checklist; it is tautologically satisfiable by any added paragraph while
  omitting the format spec or the 0116 stall linkage that the Requirements
  enumerate.

#### Minor

- 🔵 **Completeness**: Decisions-file format promotion has no dedicated acceptance criterion
  **Location**: Acceptance Criteria
  Promoting `ACCELERATOR_MIGRATE_DECISIONS_FILE` to a user-facing interface is a
  named requirement, but no AC pins where it (and its format) becomes
  discoverable — an implementer could leave it documented only inside SKILL.md
  and still satisfy all three criteria.

- 🔵 **Dependency**: Shared edit-region merge-ordering constraint with 0116 not captured
  **Location**: Dependencies
  0117 and 0116 both edit `interactive-lib.sh` in the same region; 0116 flags
  the merge-ordering coupling explicitly, 0117 records no coordination note.

- 🔵 **Dependency**: ADR-0037 amendment dependency unresolved and not reflected as a coupling
  **Location**: Open Questions
  If the open question resolves to "amend ADR-0037", 0117 has an output coupling
  to 0092 (the amendment must accompany this work); Dependencies lists 0092 only
  as "Relates to" with no downstream linkage.

- 🔵 **Clarity**: "dry-emit" introduced as an unexplained synonym for `--list`
  **Location**: Summary
  "`--list`/dry-emit mode" uses "dry-emit" without defining it or stating it is
  the same mode, leaving the reader unsure whether one flag or two are required.

- 🔵 **Clarity**: "fix A of 0115" asserts a recommendation the source research argued against
  **Location**: Summary
  The research recommends C+D+B and calls A "better as a follow-up than the
  first move"; the Summary frames A as the work without reconciling the
  divergence, so a reader cannot tell whether the override is intentional.

- 🔵 **Clarity**: "the migration" / "interactive transformation" referent depends on the source doc
  **Location**: Context
  The work item never names the concrete interactive migration (0007) that
  drives this, so a reader of 0117 alone cannot tell whether `--list` is a
  general feature or tailored to one migration's prompts.

- 🔵 **Testability**: AC1 "context" field is unspecified, so the printed output cannot be checked
  **Location**: Acceptance Criteria
  "context" is undefined; an implementation could emit an empty or unhelpful
  context string and still claim to satisfy AC1.

- 🔵 **Testability**: AC2 does not specify the corpus/migration under which the flow is exercised
  **Location**: Acceptance Criteria
  AC2 names no precondition corpus or migration, so two verifiers could test
  different inputs and a degenerate empty-decision case could trivially
  "complete".

#### Suggestions

- 🔵 **Scope**: Record why fix A was adopted over the research's recommended C/D path
  **Location**: Summary / Drafting Notes
  The research deprioritises A; 0117 implements it as a child of 0115 without
  recording the decomposition rationale, so its standing as a delivery unit
  rests on a decision not visible in the item.

- 🔵 **Clarity**: "this amends" pre-decides the still-open ADR-0037 question
  **Location**: Dependencies / Open Questions
  Dependencies calls ADR-0037 "the interactive contract this amends", while
  Open Questions still asks whether an amendment is warranted — a minor internal
  inconsistency.

### Strengths

- ✅ The three Requirements map one-to-one onto the three Acceptance Criteria,
  reusing terminology (`--list`, `ACCELERATOR_MIGRATE_DECISIONS_FILE`, "invoker
  contract") verbatim across both sections.
- ✅ Actors are consistently named in the AC (the driver, the agent, a
  developer reading SKILL.md), so responsibility for each action is explicit.
- ✅ The decisions-file format is defined inline on first mention
  (newline-delimited, positional accept/skip/edit verbs matched by emission
  order).
- ✅ Every expected section is present and substantively populated — no empty
  headings or placeholder content; frontmatter is intact with a valid kind and
  status.
- ✅ The three deliverables are genuinely interdependent facets of one
  capability, so the bundling is justified rather than a scope smell, and
  boundaries against siblings 0116/0118/0119 are explicitly drawn.
- ✅ Technical Notes pin every code surface the change spans
  (`run-migrations.sh`, `interactive-lib.sh`, `SKILL.md`) with line anchors,
  making the coupling to runner internals traceable.

### Recommended Changes

1. **Add an acceptance criterion (or bind AC1) to a named reference corpus**
   (addresses: AC1 "every pending transformation" unbounded; AC1 "context"
   unspecified; AC2 corpus unspecified)
   Name the concrete fixture/migration (e.g. the 0007 interactive stage with a
   known set of ambiguous-band linkages), state the expected enumerated count
   and content of `--list` output, and define what "context" contains (e.g.
   source file path + the band/field being decided).

2. **Add an acceptance criterion for the positional list↔verb correspondence**
   (addresses: list-to-verb ordering has no AC)
   Exercise a multi-decision fixture with distinct verbs in a known order and
   assert each `--list` entry's index maps to the verb at the same position,
   with the resulting corpus reflecting the per-position decision.

3. **Rephrase AC3 as a checklist of required documented elements** (addresses:
   AC3 lacks enumerable pass condition; decisions-file promotion has no AC)
   Require SKILL.md to document (a) the list→decide→write→resume flow, (b) the
   decisions-file format — newline-delimited positional accept/skip/edit<value>
   matched by emission order, (c) the no-input outcome linking to 0116's
   structured stall, and (d) where the now-public env-var interface is
   discoverable (e.g. `--help` and/or a banner).

4. **Record the real dependencies on 0116 and 0118 in the Dependencies
   section** (addresses: 0116 stall content-dependency; 0118 functional
   precondition; shared edit-region merge ordering)
   Mirror the couplings the siblings already capture: a soft "Blocked by: 0116"
   (or ordering note) for documentation accuracy, an integration-ordering note
   that AC2's end-to-end-against-0007 verification depends on 0118, and a
   coordination note for the shared `interactive-lib.sh` edit region.

5. **Record the fix-A rationale and resolve the ADR-0037 question** (addresses:
   "fix A" contradicts research; scope rationale gap; "amends" pre-decides;
   ADR-0037 coupling)
   Add one line in Context or Drafting Notes noting that 0115 elected fix A
   alongside the C/D items, and either resolve the ADR-0037 open question (and
   capture the amendment as a downstream output of 0117 if so) or soften the
   Dependencies wording so it does not pre-decide it.

6. **Clarify "dry-emit" and name the concrete migration** (addresses: "dry-emit"
   synonym; "the migration" referent)
   State once that `--list` is the dry-emit mode (or drop the term), and name
   0007 in Context as the concrete interactive migration driving this.

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear and well-anchored: actors (agent,
developer, driver) are mostly named, the three requirements map cleanly to three
acceptance criteria, and domain terms reuse vocabulary established in the
referenced research. The main clarity weaknesses are an unexplained metonymic
shift in the Summary ("This is fix A of 0115" vs. the research's recommendation
against A), the term "dry-emit" used as an undefined synonym for "--list", and a
couple of pronouns/phrases whose referent depends on reading the source research
rather than the work item itself.

**Strengths**:
- The three Requirements map one-to-one onto the three Acceptance Criteria, with
  consistent terminology reused verbatim across both sections.
- Actors are consistently named in the Acceptance Criteria, so responsibility
  for each action is explicit rather than buried in passive voice.
- The decisions-file format is defined inline on first mention, so the central
  technical term does not force the reader to guess.

**Findings**:
- 🔵 minor (high) — **Summary**: "dry-emit" introduced as an unexplained synonym
  for --list. The Summary and Technical Notes use "dry-emit" interchangeably
  with `--list` without saying so; a reader cannot be sure whether it is a
  second mode, an alias, or descriptive prose. Suggestion: state once that
  `--list` is the dry-emit mode, or describe the behaviour instead.
- 🔵 minor (medium) — **Summary**: "fix A of 0115" asserts a recommendation the
  source research argued against. The research recommends C+D+B and calls A
  "better as a follow-up than the first move"; without a reconciling sentence a
  reader cannot tell whether 0115 deliberately overrode the recommendation.
  Suggestion: add a clause noting 0115 adopted fix A despite its lower priority
  in the research.
- 🔵 minor (medium) — **Context**: "the migration" / "interactive transformation"
  referent depends on the source doc. The item never names which migration (0007)
  drives this, so a reader cannot tell whether `--list` is general or
  migration-specific. Suggestion: name 0007 and the kind of prompts it emits.
- 🔵 suggestion (medium) — **Open Questions / Dependencies**: "this amends" uses
  a verb the Open Question still questions. Dependencies calls ADR-0037 the
  contract "this amends" while Open Questions asks whether to amend it.
  Suggestion: soften the Dependencies wording so it does not pre-decide the open
  question.

### Completeness

**Summary**: This Task work item is structurally complete and well-populated:
every expected section is present and carries substantive, non-placeholder
content. The frontmatter is intact with a recognised kind and a valid status,
and the content matches what a task demands — a clear definition of the work to
be done across three concrete deliverables. The only completeness gap worth
noting is minor: the Acceptance Criteria cover the three deliverables but leave
the documented decisions-file format (a named requirement) without its own
verifiable criterion.

**Strengths**:
- Frontmatter is complete and valid — kind, status, priority, parent/relates_to
  linkage and schema_version all present.
- Summary states the work as a clear compound deliverable and ties it to its
  parent (fix A of 0115).
- Context explains the motivating forces concretely (only --skip/--unskip exist,
  the decisions file is a hidden seam, SKILL.md covers author not invoker).
- Requirements are specific and implementation-ready, including the
  decisions-file format.
- Optional sections (Open Questions, Dependencies, Assumptions, Technical Notes)
  are all genuinely populated.

**Findings**:
- 🔵 minor (medium) — **Acceptance Criteria**: Decisions-file format promotion
  has no dedicated acceptance criterion. Promoting
  `ACCELERATOR_MIGRATE_DECISIONS_FILE` from hidden seam to user-facing interface
  is a named requirement, but the three AC cover only --list output, the
  end-to-end flow, and the SKILL.md contract; an implementer could leave the env
  var documented only inside SKILL.md and still pass. Suggestion: add an AC
  pinning where the public interface and its format become discoverable (e.g.
  --help and/or banner), distinct from the SKILL.md criterion.

### Dependency

**Summary**: Work item 0117 is one of five sibling tasks decomposed from epic
0115, and its core deliverable is tightly coupled to its siblings, yet its
Dependencies section captures those couplings far more loosely than the siblings
capture them. Most notably, 0117's documentation and its end-to-end acceptance
criterion depend on 0116's structured stall and 0118's 0007-validator
reconciliation respectively, and 0117 shares an edit region
(`interactive-lib.sh`) with 0116 — but 0117 lists every sibling only as "Relates
to" and asserts "Blocked by: none", while 0116 explicitly records the reciprocal
constraints. The upstream system couplings are otherwise well captured.

**Strengths**:
- The Relates-to list is rich and accurate at the system-coupling level (0069,
  0092/ADR-0037, 0116).
- Technical Notes pin every code surface the change spans with line anchors.
- The Assumptions section captures the positional/keyless ordering coupling
  between --list output and decisions-file consumption order.

**Findings**:
- 🟡 major (high) — **Dependencies**: Documentation content-dependency on 0116's
  structured stall not captured as a blocker. AC3 requires documenting "what
  happens when it cannot (links to the structured stall from 0116)", yet
  Dependencies says "Blocked by: none" and lists 0116 only under "Relates to";
  0116 records the reciprocal ordering constraint. Suggestion: record the content
  dependency on 0116 explicitly (soft "Blocked by" or ordering note) so the
  contract is not finalised against an undefined stall.
- 🟡 major (high) — **Acceptance Criteria**: End-to-end completion criterion has
  an uncaptured functional precondition on 0118 (fix C). AC2 requires the flow to
  complete against an interactive migration; the research shows 0007 hard-fails
  in `self_validate_structural` until fix C (0118) lands. 0116 captures this;
  0117 names neither 0118 nor 0119. Suggestion: add 0118 as a
  functional-precondition / integration-ordering note.
- 🔵 minor (high) — **Dependencies**: Shared edit-region merge-ordering
  constraint with 0116 not captured. Both edit `interactive-lib.sh` in the same
  region; 0116 flags the merge ordering, 0117 records nothing. Suggestion: add a
  coordination note mirroring 0116's.
- 🔵 minor (medium) — **Open Questions**: ADR-0037 amendment dependency
  unresolved and not reflected as a coupling. If the answer is "amend ADR-0037",
  0117 has an output coupling to 0092; Dependencies lists 0092 only as "Relates
  to". Suggestion: resolve the question and, if amending, capture it as a
  downstream output.

### Scope

**Summary**: Work item 0117 is a well-bounded `task` that decomposes a single
fix option (fix A) from epic 0115 into one coherent deliverable: the
agent-decisions bridge. The three requirements — `--list` mode, promoting the
decisions-file interface, and documenting the invoker contract — are genuinely
interdependent facets of a single capability, not separable concerns, so the
bundling is justified rather than a scope smell. The `task` kind and M sizing fit
the scope as described, and the boundaries against sibling work items (0116,
0118, 0119) are explicitly drawn.

**Strengths**:
- The three requirements are mutually dependent (the Assumptions section makes
  the ordering-to-verb coupling explicit), so they form one indivisible unit of
  deliverable value.
- Scope boundaries against siblings are drawn cleanly — stall to 0116, 0007
  backfill/validator to 0118, resume-safe partial failure to 0119.
- The `task` kind is consistent with a single technical capability under a
  parent epic; Summary, Requirements, and AC describe the same scope with no
  drift.
- The work is confined to a single component/ownership domain (the migrate
  skill), so there is no cross-service-boundary orchestration concern.

**Findings**:
- 🔵 suggestion (medium) — **Summary / Drafting Notes**: 0117 implements fix A
  (which the research classifies as "better as a follow-up than the first move")
  as a child of 0115 without recording why A was adopted over the recommended
  C/D path, so its standing as a delivery unit rests on a decomposition decision
  not visible here. Suggestion: add a one-line note in Context or Drafting Notes
  recording that 0115 elected fix A alongside the C/D items.

### Testability

**Summary**: The three Acceptance Criteria are framed as Given/When/Then and two
of them (AC1 --list behaviour, AC2 end-to-end list→decide→resume) describe
concrete, verifiable outcomes. However, AC1 and AC3 contain unbounded language
("every pending transformation", the full set of documented sub-topics) without
a defined reference corpus or checklist, and AC3 is a documentation-presence
criterion whose pass condition is partially subjective. The criteria also miss a
verifiable outcome for one explicit requirement — that --list and the
decisions-file consumption order match.

**Strengths**:
- All three Acceptance Criteria are framed as Given/When/Then.
- AC1 pairs a positive outcome (prints key + proposed value + context) with a
  negative outcome (exits without mutating the corpus), making non-mutation
  independently checkable.
- AC2 specifies an observable end-state rather than an implementation
  instruction.

**Findings**:
- 🟡 major (high) — **Acceptance Criteria**: AC1 "every pending transformation"
  is unbounded without a reference corpus. No fixture or expected enumeration is
  defined, so completeness cannot be verified; an implementation omitting a
  transformation class would still pass against any single example. Suggestion:
  specify a fixture corpus with a known enumerated set.
- 🟡 major (high) — **Assumptions**: List-to-verb ordering correspondence has no
  Acceptance Criterion. The Assumptions section makes the positional mapping the
  core contract AC2 depends on, yet no AC verifies the Nth list entry consumes
  the Nth verb. Suggestion: add a multi-decision fixture AC with distinct verbs
  in a known order.
- 🟡 major (medium) — **Acceptance Criteria**: AC3 documentation criterion lacks
  an enumerable pass condition. "the invoker contract … is documented" is
  tautologically satisfiable; the Requirements enumerate sub-topics but AC3 does
  not bind to them. Suggestion: rephrase AC3 as a checklist of required
  documented elements.
- 🔵 minor (medium) — **Acceptance Criteria**: AC1 "context" field is
  unspecified, so the printed output cannot be checked. Suggestion: define what
  "context" contains (e.g. source file path + band/field being decided).
- 🔵 minor (medium) — **Acceptance Criteria**: AC2 does not specify the
  corpus/migration under which the flow is exercised. Suggestion: name the
  concrete fixture/migration and the expected post-migration corpus state.

---

## Re-Review (Pass 2) — 2026-06-20

**Verdict:** COMMENT

The work item edits resolved all five major findings and every minor/suggestion
from pass 1 except the intentionally-deferred ADR-0037 open question. The
re-review surfaced no criticals and no majors — only minor polish, most of it
concentrated in the newly-added acceptance criteria (their added specificity
introduced a few small under-definitions). The item is now acceptable for
implementation.

### Previously Identified Issues

- 🟡 **Dependency**: 0116 stall content-dependency not captured as a blocker — **Resolved** (Dependencies now records a soft ordering dependency on 0116; 0116 added to `relates_to`).
- 🟡 **Dependency**: AC2 end-to-end precondition on 0118 uncaptured — **Resolved** (explicit functional-precondition note; 0118 added to `relates_to`).
- 🟡 **Testability**: AC1 "every pending transformation" unbounded — **Resolved** (rewritten against a named reference fixture with exactly N transformations).
- 🟡 **Testability**: positional list↔verb ordering had no AC — **Resolved** (new AC2 asserts the Nth entry maps to the Nth verb).
- 🟡 **Testability**: AC3 documentation criterion not enumerable — **Resolved** (now a four-part (a)–(d) checklist as AC5).
- 🔵 **Completeness**: decisions-file promotion had no AC — **Resolved** (new AC4 requires discoverability in `--help`/banner).
- 🔵 **Dependency**: shared `interactive-lib.sh` merge-ordering with 0116 — **Resolved** (coordination note added).
- 🔵 **Clarity**: "dry-emit" synonym — **Partially resolved** (glossed in Summary; one reviewer still flags the bare noun as low-severity polish).
- 🔵 **Clarity**: "fix A" contradicts research / scope rationale gap — **Resolved** (Drafting Notes records why 0115 chose A alongside C/D).
- 🔵 **Clarity**: concrete migration unnamed — **Resolved** (0007 named in Context).
- 🔵 **Testability**: AC1 "context" undefined; AC2 corpus unspecified — **Resolved** (context defined as source path + band/field; reference fixture named).
- 🔵 **Dependency**: ADR-0037 amendment unresolved — **Still present (by design)** (wording softened so it no longer pre-decides; the open question is deliberately left for a separate decision).

### New Issues Introduced

All minor or suggestion severity; none blocking:

- 🔵 **Clarity**: fix-letter→work-item-id mapping is scattered — Summary says 0118 is a "C/D" mitigation while Drafting Notes/Dependencies call it "fix C". State the full A=0117/B=0116/C=0118 mapping once.
- 🔵 **Clarity**: domain terms "band/field" and "PROMPT frames" used without definition.
- 🔵 **Clarity**: AC2 positional-mapping wording is dense/self-referential.
- 🔵 **Completeness**: criteria live under the "Requirements" heading with no separate "Acceptance Criteria" header to land on.
- 🔵 **Dependency**: parent epic 0115 not surfaced under "Blocks"; reference-fixture existence not listed as an upstream precondition; 0069 runner machinery classed as "Relates to" rather than a build-time foundation.
- 🔵 **Scope / Testability**: AC2 is anchored to the *live* 0007 stage (gated on 0118) rather than the standalone fixture, so it is not fully self-verifiable within 0117; consider re-anchoring to the fixture.
- 🔵 **Testability**: AC1 "consumption order" not operationally defined; AC3 permits two valid empty-state outputs (disjunctive); AC4 "`--help` and/or banner" leaves the required surface ambiguous; AC2 "reflects the per-position decision" doesn't enumerate the expected post-state per verb.

### Assessment

The work item is now ready for implementation. The remaining items are
optional polish — the most worthwhile being (1) pinning AC2 to the standalone
fixture so it is verifiable within this item's boundary, and (2) defining the
expected `--list` ordering and per-verb post-states so the new criteria assert
against fixed values. None of these block scheduling.

---

## Re-Review (Pass 3) — 2026-06-20

**Verdict:** REVISE

The AC2 re-anchoring succeeded: the behavioural criteria now run against a
standalone fixture independent of 0118, and the live-corpus check is split into
its own clearly-gated integration criterion. The scope lens is now clean (only a
confirming note that the bundling was correct). However, with the acceptance
criteria now far more detailed, the testability lens raised two **new major**
findings — both about pinning the criteria to deterministic, scriptable
assertions rather than leaving the fixture and documentation checks
underspecified. Two majors meets the REVISE threshold. Both are cheap fixes.

### Previously Identified Issues (pass 2 carryover)

- 🔵 **Scope/Testability**: AC2 anchored to live 0007 rather than standalone fixture — **Resolved** (behavioural ACs now run against a standalone fixture; live-corpus run split into a separate, explicitly 0118-gated integration criterion).
- 🔵 **Testability**: AC2 live-corpus runnability not its own criterion — **Resolved** (now criterion 6, flagged "verifiable only once 0118 lands").
- 🔵 **Clarity**: fix-letter→id mapping scattered — **Still present** (re-flagged; gloss A/B/C on first use).
- 🔵 **Clarity**: "band/field", "PROMPT frames" undefined — **Still present** (re-flagged).
- 🔵 **Testability**: AC1 consumption order / AC3 disjunctive output / AC4 and/or surface / AC2 per-verb post-state — **Partially addressed** (per-verb and ordering concerns now fold into the two majors below; AC3 disjunctive output remains).
- 🔵 **Dependency**: 0115 parent not in Blocks; reference-fixture precondition — **Partially present** (fixture now described as authored in-item; downstream Blocks still empty).

### New Issues Introduced

Major (drive the REVISE verdict):

- 🟡 **Testability**: Reference fixture leaves N, the per-entry verb distribution, and the canonical `--list` entry format (field order/delimiter) unspecified, so two implementers could build different fixtures and output assertions and both claim a pass. Pin e.g. "N=3, entries 1/2/3 expect accept / skip / edit <value>" and the exact `--list` line format.
- 🟡 **Testability**: The `--help`/banner and SKILL.md documentation criteria rest on a human reader judging "discoverable"/"documented" with no defined procedure. Reduce each to a grep-able assertion (e.g. `--help` contains the literal `ACCELERATOR_MIGRATE_DECISIONS_FILE`; the SKILL.md section contains the tokens `accept`/`skip`/`edit`, a link to 0116, and the list→decide→write→resume phrasing).

Minor / suggestion:

- 🔵 **Completeness/Testability**: minimum N for the fixture unspecified (overlaps the first major) — state ≥3 covering one of each verb.
- 🔵 **Testability**: criterion 6's "may be deferred" leaves its in-item pass/fail disposition ambiguous — state definitively whether it is out of scope for 0117 or a tracked follow-up.
- 🔵 **Testability**: AC2's trigger "the agent runs the … flow" embeds an agentic step; make the deterministic version (fixed decisions file → verb i applied to entry i) the binding criterion.
- 🔵 **Clarity**: "verifiable entirely within this work item" reads ambiguously; standardise on "verifiable without 0118 having landed". Number the ACs (AC1…AC6) so the Dependencies references resolve.
- 🔵 **Dependency**: the deferred live-corpus integration check (and likely sibling 0120's prevention tests) consume this bridge but "Blocks" is empty — name the downstream consumer.
- 🔵 **Scope**: decisions-file promotion is the one separable facet but is correctly bundled (confirming note, no action).

### Assessment

The work item is close. The two majors are narrow, mechanical specificity fixes
— pinning the fixture (N, verb assignment, `--list` line format) and converting
the two documentation criteria into grep-able assertions. Applying those, plus
optionally numbering the ACs and naming the downstream consumer in Blocks, would
return this to APPROVE/COMMENT. None of the findings reopen the structural
design.

---

## Re-Review (Pass 4) — 2026-06-20

**Verdict:** REVISE

The pass-3 fixes landed cleanly: the fixture is pinned to three transformations
with a position→verb table, AC2 is deterministic, AC3 is canonical, AC4/AC5 are
grep-able, AC6 is definitively out of scope, and the ACs are numbered. Scope,
completeness, clarity, and dependency all now return only minor/suggestion notes.
The testability lens, however, raised two new **major** findings — both about
the *literal values* in the fixture that pass 3 named structurally but did not
enumerate. Two majors meets the REVISE threshold.

This is the fourth pass, and the pattern is now clear and worth naming: each
round tightens the criteria and the testability lens locates the next finer
layer of specificity. The structural design has been stable since pass 2; what
remains is fixture-literal detail that is arguably implementation-level.

### Previously Identified Issues (pass 3 carryover)

- 🟡 **Testability**: fixture N / verb distribution / `--list` format unspecified — **Resolved** (three transformations, position→verb table, canonical tab-delimited line form).
- 🟡 **Testability**: documentation criteria reader-dependent — **Resolved** (AC4/AC5 now grep-able string assertions).
- 🔵 **Testability**: AC6 disposition ambiguous — **Resolved** (now explicitly out of scope for 0117, owned by 0118/epic).
- 🔵 **Testability**: AC2 agentic trigger / AC3 disjunctive — **Resolved** (AC2 deterministic decisions-file form; AC3 single canonical line).
- 🔵 **Clarity**: ACs unnumbered — **Resolved** (AC1–AC6).
- 🔵 **Clarity**: fix-letter→id mapping / "band" undefined — **Still present** (re-flagged minor).
- 🔵 **Dependency**: downstream consumer not in Blocks — **Still present** (re-flagged minor).

### New Issues Introduced

Major (drive the REVISE verdict):

- 🟡 **Testability**: AC2 leaves position-3's `edit <value>` as an unbound placeholder — pin a concrete literal (e.g. `edit kind:decision`) and the exact expected post-migration field value, so the edit verb's effect is assertable.
- 🟡 **Testability**: the positional decisions file has no error-handling criterion — count-mismatch (too few/many verbs) and unknown-verb cases, the classic off-by-one failure modes, have no defined observable outcome. Either add an AC (e.g. "two verbs for three prompts → exit non-zero naming the unmatched position, corpus unmutated") or delegate explicitly to 0116's structured-stall scope.

Minor / suggestion:

- 🔵 **Testability**: the three expected `--list` lines (literal key/proposed-value/context) aren't tabulated — add them verbatim beside the position/verb table so AC1 asserts exact strings, not just shape.
- 🔵 **Testability**: AC5 verifies keyword presence but not that the documented resume command is the literal command an agent runs.
- 🔵 **Clarity**: "consumption order" vs "emission order" used for the same concept; fixture home "in … or alongside it" undetermined; `<key>` token in the canonical form undefined (also flagged by completeness).
- 🔵 **Dependency**: 0118's AC6 integration check consumes this bridge but "Blocks" is empty; 0115 parent rollup stated only as "Relates to".

### Assessment

The work item is implementation-ready in all structural respects. The two
majors are fixture-literal pins: (1) bind the position-3 edit value and tabulate
the three expected `--list` lines, and (2) decide whether malformed/mismatched
decisions-file handling is in 0117's scope or 0116's. Both are narrow. After
this round, further testability passes are expected to yield diminishing returns
— the recommendation is to apply these final pins (and the `<key>` definition),
then close the review rather than iterate further.

---

## Approval — 2026-06-20

**Verdict:** APPROVE (closing the review)

The two pass-4 majors were applied in-story:

- **Fixture fully pinned** — concrete literals for all three positions
  (`<key>` / proposed-value / context / expected post-migration field), an
  explicit `<key>` definition, and the verbatim expected `--list` output block;
  AC1 now asserts that output byte-for-byte and AC2 binds position 3 to
  `edit work-item:0100` with the exact resulting field value.
- **Malformed-file handling kept in 0117** — new AC6 verifies fail-closed
  behaviour for count-too-few, count-too-many, and unknown-verb (each exits
  non-zero naming the offending position, corpus unmutated). A matching
  Requirement bullet and AC5(e) were added; the integration criterion is now
  AC7.

The remaining findings across all four passes are minor/suggestion polish
(fix-letter→id glosses, the "band" definition, naming the downstream consumer
in Blocks). Per the pass-4 assessment, the criteria-tightening was a ratchet
with diminishing returns, so the review is closed here by reviewer decision
rather than iterated further. The structural design has been stable since pass
2; the work item is ready for implementation.

---
*Review generated by /accelerator:review-work-item*
