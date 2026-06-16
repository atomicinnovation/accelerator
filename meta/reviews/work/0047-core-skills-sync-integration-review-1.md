---
type: work-item-review
id: "0047-core-skills-sync-integration-review-1"
title: "Work Item Review: Core Skills Sync Integration"
date: "2026-06-15T16:23:34+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0047"
work_item_id: "0047"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-06-15T21:12:34+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Core Skills Sync Integration

**Verdict:** REVISE

This is a structurally complete, densely-written story with strong
acceptance-criteria coverage of the `/create-work-item` push paths and a
clear unifying convention (numeric vs remote-format `work_item_id`). The
reason for REVISE is a single theme that all five lenses converge on: the
story *specifies* a five-state sync display but *can only deliver* two states
(synced/unsynced) within its own boundary — the other three depend on
`last-sync.json`, which story 0051 produces. That mismatch surfaces as an
internal contradiction (clarity), an inverted dependency (dependency),
unverifiable acceptance criteria (testability), and a scope/deliverable blur
(scope). A secondary gap is that "content parity" — the basis for the synced
and modified labels — is never given a comparison procedure.

### Cross-Cutting Themes

- **Five-state spec vs two-state deliverable** (flagged by: clarity,
  completeness, dependency, scope, testability) — The Requirements define five
  sync states, but Assumptions and Technical Notes concede that locally
  modified, remotely modified, and conflict cannot be derived until 0051 ships
  `last-sync.json`. Every lens hits a different face of this: the definition of
  "synced" assumes a baseline it claims not to need (clarity); two of five
  states have no dedicated AC and three are unverifiable without a stated
  precondition (completeness, testability); 0051 is listed as a downstream
  *Blocks* when it is functionally an upstream artefact dependency
  (dependency); and the story's stated scope exceeds what it can land (scope).
  Resolving this — by either narrowing the story's in-scope deliverable to
  synced/unsynced or restructuring the criteria/dependencies to make the
  partial-delivery boundary explicit — addresses the bulk of the findings.

- **"Content parity" is undefined** (flagged by: clarity, testability) — The
  synced and modified labels hinge on content being "logically equivalent" to
  the remote, but no comparison procedure is given (which fields, frontmatter
  exclusions, timestamp handling). This makes "synced" both ambiguous to read
  and impossible to test deterministically.

- **Plan-deferred decisions not surfaced or pinned** (flagged by:
  completeness, testability) — The retry count before fallback is deferred to
  the plan, but AC6 reads as a single retry, and neither the retry count nor
  the colour scheme appears in Open Questions.

### Findings

#### Critical

- (none)

#### Major

- 🟡 **Clarity**: Definition of "synced" contradicts the no-baseline derivability claim
  **Location**: Requirements / Assumptions
  Requirements define synced as "no changes on either side since last sync" — a
  definition that needs a baseline — while Assumptions claim synced is shown
  even with no `last-sync.json`. An implementer cannot tell whether the
  no-baseline "synced" label is content-based (impossible without a baseline)
  or merely ID-format-based.

- 🟡 **Dependency**: 0051 dependency is inverted — upstream prerequisite, not a pure downstream Blocks
  **Location**: Dependencies
  0051 is listed only as a downstream "Blocks", but the body states 0047
  consumes the `last-sync.json` artefact 0051 produces ("can't be derived until
  that file exists"). A planner reading Dependencies alone would schedule 0047
  first and find three of its five states underivable.

- 🟡 **Dependency**: Push flow depends on a concrete integration's create capability that 0046 alone does not provide
  **Location**: Requirements
  `/create-work-item`'s push-on-accept needs the remote to create the issue and
  return its key, but 0046 only declares the `work.integration` config key. The
  actual create-then-write-back capability lives in a per-system integration
  (Jira complete; others are sibling stories) — an unnamed prerequisite.

- 🟡 **Testability**: "Synced"/"modified" states rely on undefined "content parity" with no comparison procedure
  **Location**: Acceptance Criteria (AC2) / Requirements
  AC2 defines synced as "logically equivalent to the remote" and modified as
  content "changed since last sync", but gives no rule for what makes two
  versions equivalent. A tester cannot construct a definitive
  synced-vs-locally-modified case.

- 🟡 **Testability**: No AC establishes the `last-sync.json` precondition needed to verify the three baseline-dependent states
  **Location**: Acceptance Criteria (AC2, AC3) vs Assumptions
  AC2/AC3 require verifying all five states, but no AC states the precondition
  (last-sync.json present) under which the three baseline-dependent states
  become observable — so those criteria are unverifiable within this story's
  scope.

#### Minor

- 🔵 **Clarity**: Remote identifier called both "issue ID" and "key" interchangeably
  **Location**: Summary
  The Summary says "issue ID"; Requirements/AC say "key"/"remote-allocated
  key"; the epic says "remote issue key". An implementer may write the wrong
  value into `work_item_id` for trackers that distinguish internal ID from key.

- 🔵 **Clarity**: "the remote" used before any remote system is named in this story
  **Location**: Summary / Context
  The `work.integration` binding lives only in epic 0045; a standalone reader
  of 0047 must infer what "the remote" denotes.

- 🔵 **Completeness**: Locally-modified and synced label states have no dedicated acceptance criterion
  **Location**: Acceptance Criteria
  AC3 covers remotely-modified distinctness, but synced and locally-modified
  are only named collectively inside AC2's parenthetical, leaving their
  per-state rendering without an explicit done-condition.

- 🔵 **Completeness**: Retry count deferred to plan but not surfaced as an open question
  **Location**: Requirements
  The retry-count and colour-scheme deferrals live only in Technical/Drafting
  Notes; Open Questions (empty) would be the discoverable home for plan-time
  decisions.

- 🔵 **Dependency**: Remote issue tracker (external system) not named as a coupling
  **Location**: Dependencies
  Both surfaces couple to an external tracker API (read for sync state, write
  for push), but Dependencies lists only sibling work items — the external
  availability dependency is invisible at planning time.

- 🔵 **Scope**: Two independently deliverable skill changes bundled in one story
  **Location**: Requirements
  Read-side sync labels in `/list-work-items` and the write-side push state
  machine in `/create-work-item` share only the `work_item_id` convention;
  neither depends on the other and each could ship/revert separately.

- 🔵 **Testability**: Retry count in the push-failure path is unbounded in the AC
  **Location**: Acceptance Criteria (AC6)
  AC6 reads as a single retry while Requirements defer the count to the plan;
  the failure-path test has an undefined boundary.

- 🔵 **Testability**: "Local file not written until success/decline/fallback" asserted in Requirements but not verified by an AC
  **Location**: Acceptance Criteria (AC5) / Requirements
  AC5 verifies the write happens "once" on success, but no AC verifies the
  negative (no premature file on disk during/after a failed push before
  fallback) — the central no-premature-write invariant could pass all ACs yet
  still be violated.

#### Suggestions

- 🔵 **Clarity**: "after the work item is drafted" vs "created"/"completes" — when does the draft exist?
  **Location**: Requirements: /create-work-item
  "Drafted", "created", and "completes" all refer to the pre-write in-memory
  state; one consistent term would avoid implying the file exists before the
  prompt.

- 🔵 **Scope**: Scope expanded from three sync states to five versus the parent epic
  **Location**: Requirements
  Epic 0045 names three states; this story defines five, while three of them
  are deferred to 0051 — blurring the one increment of value this story
  produces.

### Strengths

- ✅ All five sync states are defined inline in Requirements with no reliance on
  external glossaries, and the two affected skills are named precisely and
  consistently throughout.
- ✅ Structurally complete: every expected section is present and substantively
  filled, with well-formed frontmatter (kind, status, priority, parent,
  blocked_by, blocks).
- ✅ The `/create-work-item` push criteria (AC5/AC6/AC7) fully specify
  precondition, action, and outcome for the accept-succeed, accept-fail-fallback,
  and decline branches, including the named follow-up `/sync-work-items`.
- ✅ AC3 is an exemplary testable invariant: no two of the five states may share
  an identical label+colour pairing — checkable deterministically regardless of
  the chosen scheme.
- ✅ The upstream blocker on 0046 is captured in both frontmatter and the
  Dependencies section, and the 0051 last-sync.json coupling is at least named
  and flagged for plan-time resolution.
- ✅ Drafting Notes pre-empt reader confusion by explicitly flagging deviations
  from the epic (ID-centric vs content-centric; three states vs five).

### Recommended Changes

1. **Reconcile the five-state spec with the two-state deliverable**
   (addresses: "synced contradicts no-baseline", "0051 dependency inverted",
   "no AC for last-sync.json precondition", "states without dedicated AC",
   "scope expanded 3→5") — Decide the story's true in-scope deliverable. The
   cleanest fix: scope this story to the synced/unsynced states it can derive
   today, state explicitly that without `last-sync.json` a remote-format ID is
   *presumed* synced (ID-format-based), and document the three
   baseline-dependent states as forward-looking spec realised in 0051. Then
   split AC2/AC3 by precondition (no-baseline case verifiable here; baseline
   case deferred to 0051), and capture in Dependencies that those three states
   are *blocked by* 0051's `last-sync.json` (an upstream artefact dependency),
   not merely blocking it.

2. **Define the content-parity comparison basis** (addresses: "content parity
   undefined") — Add an AC or Assumption stating what makes local and remote
   "logically equivalent" — e.g. tracked frontmatter fields + body
   byte-equivalent to the last-synced snapshot, with timestamps/identity fields
   excluded — or explicitly defer the comparison rule to 0051 alongside the
   baseline-dependent states.

3. **Name the missing dependencies** (addresses: "push flow depends on
   integration create capability", "remote tracker not named") — Add a
   Dependencies note that the push-on-accept path requires at least one
   configured integration's create capability (e.g. Jira, complete), and that
   both skills couple to the configured external tracker's API.

4. **Pin or reframe the deferred decisions** (addresses: "retry count
   unbounded", "retry count not surfaced") — Either pin the retry count in AC6
   ("after one retry that also fails") or reword it to assert the observable
   outcome independent of count ("after the configured retries are exhausted").
   Optionally move the retry-count and colour-scheme deferrals into Open
   Questions.

5. **Add the no-premature-write negative AC** (addresses: "no-premature-write
   not verified") — Add an AC asserting that while a push is in progress and
   has not succeeded or been declined, no local file with the work item's ID
   exists in `meta/work/`.

6. **Tighten terminology** (addresses: "issue ID vs key", "the remote unnamed",
   "drafted vs created") — Use the epic's "remote issue key" consistently;
   add a Context clause noting "the remote" means the single system declared by
   `work.integration` (per 0046); and use one term for the pre-write in-memory
   state.

7. **Decide whether to split the story** (addresses: "two deliverables
   bundled") — Either split into two stories (list-work-items sync status;
   create-work-item push offer) under 0045, or add a one-line justification in
   Technical Notes for keeping them together (shared M sizing / single PR).

## Per-Lens Results

### Clarity

**Summary**: The work item is largely unambiguous and internally coherent: it
names its two skill surfaces explicitly, defines all five sync states inline,
and consistently anchors the unsynced state to the numeric-`work_item_id`
convention. The main clarity weakness is a terminology drift between "issue ID"
and "key" for the remote-allocated identifier, plus a latent tension between
the Requirements' definition of "synced" (requires a since-last-sync baseline)
and the Assumptions' claim that "synced" is derivable without a
`last-sync.json` baseline. Actor and outcome clarity are generally strong, with
only minor passive-voice constructions that do not obscure responsibility.

**Strengths**:
- All five sync states are explicitly defined inline in Requirements, with no
  reliance on external glossaries.
- The two affected skills are named precisely and consistently throughout.
- Drafting Notes pre-empt reader confusion by flagging deviations from the
  parent epic's language.
- Technical Notes resolve `work_item_id` vs `id` schema-key ambiguity by
  pointing to `work-item-read-field.sh` as the bridging reader.

**Findings**:
- 🟡 major (medium): **Definition of "synced" contradicts the no-baseline
  derivability claim** (Requirements / Assumptions) — Requirements define synced
  as "no changes on either side since last sync" (baseline-dependent), but
  Assumptions claim items with no `last-sync.json` "show only synced or
  unsynced". Both cannot hold. An implementer cannot tell whether the
  no-baseline "synced" label is content-based or merely ID-format-based.
- 🔵 minor (high): **Remote identifier called both "issue ID" and "key"
  interchangeably** (Summary) — Summary says "issue ID"; Requirements/AC say
  "key". A reader cannot be certain they denote the same value, risking the
  wrong field written into `work_item_id`.
- 🔵 minor (medium): **"the remote" used before any remote system is named in
  this story** (Summary / Context) — the `work.integration` binding lives only
  in epic 0045; a standalone reader of 0047 must infer the referent.
- 🔵 suggestion (medium): **"after the work item is drafted" vs "created"** —
  (Requirements: /create-work-item) "drafted", "created", and "completes" all
  refer to the pre-write in-memory state; one consistent term would avoid
  implying the file exists before the prompt.

### Completeness

**Summary**: This story is structurally complete and densely populated: every
expected section is present and substantively filled, and the frontmatter
carries all required fields with a recognised kind. As a story it identifies
the user, explains the motivating forces, and provides seven specific
acceptance criteria. The only notable gaps are a couple of behavioural branches
that appear in the Requirements but are not mirrored as acceptance criteria.

**Strengths**:
- All standard sections present and substantively populated (Open Questions
  legitimately empty with a dash marker).
- Frontmatter integrity is strong: kind, status, priority, parent, blocked_by,
  blocks all present and well-formed.
- Clearly identifies the actor (a developer using core work-management skills)
  and the why.
- Seven acceptance criteria mapping closely to the two skill surfaces,
  including the not-configured negative case and push success/failure/decline.
- Unusually rich Assumptions and Technical Notes reduce implementer follow-up.

**Findings**:
- 🔵 minor (medium): **Locally-modified and synced label states have no
  dedicated acceptance criterion** (Acceptance Criteria) — only remotely-modified
  has a distinctness criterion; the other states are named collectively inside
  AC2's parenthetical, leaving their rendering without an explicit
  done-condition.
- 🔵 minor (medium): **Retry count deferred to plan but not surfaced as an open
  question or assumption** (Requirements) — the retry-count and colour-scheme
  deferrals live only in Technical/Drafting Notes; Open Questions would be the
  discoverable home.

### Dependency

**Summary**: The work item captures its primary upstream blocker (0046) cleanly
and names the downstream consumer 0051. However, the body repeatedly describes a
functional dependency ON 0051's `last-sync.json` artefact that the Dependencies
section inverts as a pure "Blocks", and the remote tracker plus the integration
story that provides the create capability are implied couplings not captured as
dependencies.

**Strengths**:
- The upstream blocker on 0046 is explicitly captured in both frontmatter and
  the Dependencies section.
- The 0051 coupling is at least named with an annotation, and Technical Notes
  flag the last-sync.json location discrepancy for plan-time resolution.
- Push-failure handling (retry-then-fallback) is fully specified, so the
  remote-unreachable path is behaviourally defined.

**Findings**:
- 🟡 major (high): **0051 dependency is inverted** (Dependencies) — listed only
  as downstream "Blocks", but the body states 0047 consumes the `last-sync.json`
  0051 produces. A planner reading Dependencies alone would schedule 0047 first
  and find three of five states underivable — a hidden ordering trap.
- 🟡 major (high): **Push flow depends on a concrete integration's create
  capability that 0046 alone does not provide** (Requirements) — push-on-accept
  needs the remote to create the issue, but 0046 only declares the config key;
  the create capability lives in a per-system integration. If scheduled after
  0046 but before any integration's create op, the push path has nothing to
  call.
- 🔵 minor (medium): **Remote issue tracker (external system) not named as a
  coupling** (Dependencies) — both surfaces couple to an external tracker API;
  Dependencies lists only sibling work items, leaving the availability/SLA
  implications invisible.

### Scope

**Summary**: Work item 0047 bundles two independently deliverable skill
enhancements — sync-status display in `/list-work-items` (read-side,
baseline-dependent) and a push-on-create flow in `/create-work-item` (write-side
state machine) — under a single "sync awareness" theme. The two share a
convention but have no implementation dependency and could be delivered,
deployed, or rolled back separately. The declared kind (story) and M size are
otherwise reasonable, but the bundling is the dominant scope signal.

**Strengths**:
- Correctly scoped beneath epic 0045 with a coherent unifying theme.
- Boundaries explicitly drawn: only synced/unsynced derivable until 0051
  delivers `last-sync.json`.
- Sensibly defers separable concerns (0046 config key, 0051 baseline) rather
  than absorbing them.

**Findings**:
- 🔵 minor (medium): **Two independently deliverable skill changes bundled in
  one story** (Requirements) — read-side ANSI rendering + frontmatter-scan
  extension vs a write-side state machine; neither depends on the other. Harder
  to estimate, review, and partially land.
- 🔵 suggestion (low): **Scope expanded from three sync states to five versus
  the parent epic** (Requirements) — the story's stated scope (five states)
  exceeds what it can deliver (synced/unsynced), inviting scope drift toward
  baseline logic that belongs to 0051.

### Testability

**Summary**: The Acceptance Criteria are well-framed as Given/When/Then
observable behaviours and the create-work-item push paths are concretely
verifiable. The main gaps concern the five sync-status labels: the
synced/modified states hinge on an undefined notion of "content parity" with no
defined comparison procedure, and the three baseline-dependent states have no AC
establishing the precondition (presence of last-sync.json) required to verify
them — so the criteria cannot collectively confirm the full five-state
behaviour described in the Summary and Requirements.

**Strengths**:
- AC3 is an exemplary testable invariant: no two of the five states may share an
  identical label+colour pairing — deterministically checkable.
- AC5/AC6/AC7 fully specify precondition, action, and expected outcome for each
  push branch, including the "written once" assertion and the named
  `/sync-work-items` follow-up.
- AC1 establishes a clean negative-case verification (integration not configured
  → no label) with no remote dependency.

**Findings**:
- 🟡 major (high): **"Synced"/"modified" states rely on undefined "content
  parity" with no comparison procedure** (AC2 / Requirements) — no rule for which
  fields are compared, frontmatter/timestamp exclusions, or what counts as a
  change; a tester cannot construct a definitive synced-vs-locally-modified
  case.
- 🟡 major (high): **No AC establishes the `last-sync.json` precondition needed
  to verify the three baseline-dependent states** (AC2, AC3 vs Assumptions) —
  AC2/AC3 require all five states but no AC states the baseline precondition, so
  those criteria are unverifiable within this story's deliverable scope.
- 🔵 minor (medium): **Retry count in the push-failure path is unbounded in the
  AC** (AC6) — AC6 reads as a single retry while Requirements defer the count to
  the plan; the failure-path test has an undefined boundary.
- 🔵 minor (medium): **"Local file is not written until push
  succeeds/declines/fallback confirmed" asserted in Requirements but not directly
  verified by an AC** (AC5 / Requirements) — the no-premature-write invariant
  could pass all ACs while a stray file is written before the push resolves.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-15

**Verdict:** COMMENT

All five major findings from Pass 1 are resolved; the verdict moves from REVISE
to COMMENT. The work item is now acceptable for planning. Remaining items are
all minor/suggestion-level polish — verification-procedure tightening and
presentational consolidation — none of which block implementation.

### Previously Identified Issues

#### Major (all resolved)

- 🟡 → ✅ **Clarity**: "synced" definition contradicts no-baseline derivability —
  **Resolved.** Context and Assumptions now define the no-baseline case as
  *presumed synced* (existence by ID format, not content parity); the clarity
  lens now cites the synced/unsynced/presumed-synced distinction as a strength.
- 🟡 → ✅ **Dependency**: 0051 dependency inverted —
  **Resolved.** The `last-sync.json` coupling is now modelled as a deliberately
  non-blocking forward data dependency with an explicit cycle-avoidance
  rationale; verified against 0051's `blocked_by` list, the edge is acyclic. The
  dependency lens now cites this as a strength.
- 🟡 → ✅ **Dependency**: push flow depends on integration create capability not
  provided by 0046 —
  **Resolved.** Dependencies now states the create-capability prerequisite
  (Jira complete; Linear/Trello/GitHub are 0048-0050). Downgraded to a minor
  suggestion to also model 0048-0050 as a non-blocking edge in frontmatter.
- 🟡 → ✅ **Testability**: "content parity" undefined comparison procedure —
  **Resolved.** The content-parity computation moved to 0051 along with the
  modified states; this story renders only ID-format-derivable labels.
- 🟡 → ✅ **Testability**: no AC establishes the `last-sync.json` precondition —
  **Resolved.** AC2 is split by precondition (no-baseline → only
  synced/unsynced), and the baseline-dependent states are carved out as
  explicitly not verifiable here. The testability lens cites the carve-out as a
  strength.

#### Minor / Suggestion (Pass 1)

- 🔵 → ✅ **Clarity**: "issue ID" vs "key" — **Resolved.** "remote issue key" used
  consistently.
- 🔵 → ✅ **Clarity**: "the remote" unnamed — **Resolved.** Defined in Context;
  now a strength.
- 🔵 → ✅ **Completeness**: synced/locally-modified without dedicated AC —
  **Resolved.** Dedicated synced/unsynced ACs added; locally-modified deferred.
- 🔵 → ✅ **Completeness**: retry count not surfaced — **Resolved.** Now in Open
  Questions.
- 🔵 → ✅ **Dependency**: remote tracker not named — **Resolved.** Now in
  Dependencies.
- 🔵 → ✅ **Testability**: retry count unbounded in AC — **Resolved.** AC rewritten
  count-independent.
- 🔵 → ✅ **Testability**: no-premature-write not verified — **Resolved.** Negative
  AC added.
- 🔵 → ✅ **Clarity**: "drafted" vs "created" — **Resolved.** Consistent pre-write
  terminology.
- 🔵 → 🔵 **Scope**: two skill surfaces bundled — **Partially resolved.** A
  deliberate-bundling justification was added; the lens now considers it
  defensible and explicitly says no split is needed, but still flags it as a
  minor packaging observation.
- 🔵 → 🔵 **Scope**: 3→5 state expansion — **Partially resolved.** In/out-of-scope
  is now clearly demarcated; the lens suggests consolidating the deferred-state
  material into one labelled subsection to keep the committed deliverable crisp.

### New Issues Introduced

All minor or suggestion severity; none block implementation.

- 🔵 **Completeness**: the Acceptance Criteria section's closing "deferred to
  0051" paragraph is presentational — consider moving it to a dedicated
  "Deferred to 0051" subsection so the AC list contains only criteria verifiable
  here. (Pairs with the scope lens's consolidation suggestion.)
- 🔵 **Completeness**: the story names the skills and remote but not the human
  persona; a one-line user-framing would complete the story shape.
- 🔵 **Testability**: "synced/unsynced" hinges on classifying a `work_item_id` as
  numeric vs remote-format, but the classification rule is not pinned — adding a
  rule (e.g. `^[0-9]+$` → unsynced, else synced) with one example per system
  would make the two core labels deterministically verifiable.
- 🔵 **Testability**: the "extensible per-item status slot" AC asserts an
  architectural property; rephrasing it as an observable (adding a status→{text,
  colour} entry yields a rendered label with no call-site edit) or moving it to
  Technical Notes would strengthen it.
- 🔵 **Testability**: "the synced and unsynced labels differ in colour" can't get
  a definitive pass until the plan fixes the palette; restate as "distinct,
  non-empty ANSI colour codes" to keep the shape verifiable independent of the
  scheme.
- 🔵 **Testability**: "the user is informed to run `/sync-work-items`" — tighten
  to a checkable observable (output names the `/sync-work-items` skill).
- 🔵 **Dependency**: the 0048-0050 create-capability coupling and 0051's
  conflict-UX reuse are described in prose; optionally surface them as
  "Related (non-blocking)" edges so the graph mirrors the prose.
- 🔵 **Clarity**: minor wording — "ANSI", "seams", and "held in memory" could each
  be glossed once for a standalone reader.

### Assessment

The work item is ready for planning. The scope-narrowing decision (synced/
unsynced committed; the three baseline-dependent states deferred to 0051 with an
acyclic dependency graph) is now sound and praised across the clarity,
dependency, scope, and testability lenses. The remaining suggestions are
optional polish — the highest-value of which is pinning the numeric-vs-remote-
format `work_item_id` classification rule, since that underpins the two labels
this story actually ships. None require another review pass before `/create-plan`.

## Approval — 2026-06-15

**Verdict elevated to APPROVE** by the reviewer. The Pass 2 COMMENT findings are
all optional polish; the work item is accepted for implementation planning. The
remaining suggestions may be folded into `/create-plan` rather than gated here.
