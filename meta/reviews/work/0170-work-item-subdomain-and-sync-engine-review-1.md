---
type: "work-item-review"
id: "0170-work-item-subdomain-and-sync-engine-review-1"
title: "Work Item Review: Work-Item Subdomain and Sync Engine"
date: "2026-08-05T18:07:37+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0170"
work_item_id: "0170"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-05T19:12:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Work-Item Subdomain and Sync Engine

**Verdict:** REVISE

0170 is a mature, detailed draft — its Acceptance Criteria are unusually
thorough for a story and its Technical Notes fully work through a resolved
design decision. However, it bundles two independently-deliverable efforts
(lifecycle CRUD and the sync engine/tracker crate) into one story, leaves the
0171 relationship under-specified in the formal Dependencies/frontmatter
fields despite describing real coupling in prose, and contains several
referent and enumeration gaps (an undefined "the dispatcher," an
inconsistently-listed ~14-script set, missing Acceptance Criteria for half
the committed subcommand surface) that would need resolving before
implementation.

### Cross-Cutting Themes

- **The ~14 untested-script set is never authoritatively enumerated**
  (flagged by: clarity, testability) — Context/Requirements say "~14," AC5
  names eight scripts plus "etc.," and the Technical Notes' internal-only
  list names a different seven. Both lenses independently concluded this
  can't be verified as written — clarity because the referent is ambiguous,
  testability because the acceptance criterion has no fixed roster to check
  against.
- **The 0170/0171 boundary and the sync/lifecycle bundling are
  under-specified together** (flagged by: dependency, scope) — dependency
  found that 0171's consumption of the `RemoteTracker` port isn't captured
  as a `Blocks` relationship despite being a real ordering constraint, while
  scope found the sync engine and lifecycle CRUD are bundled into one story
  despite being separately deliverable. Splitting the story (per scope's
  recommendation) would also clarify the 0171 wiring question, since the
  in-process real-provider wiring described in the Summary would move with
  the sync half.

### Findings

#### Major

- 🟡 **Scope**: Story bundles lifecycle CRUD and the sync engine as two
  independently deliverable concerns
  **Location**: Summary
  The Summary itself signals this ("lifecycle operations plus the remote
  sync engine"); the CRUD surface and the tracker crate's port/state machine
  are not mutually dependent for delivery, making this read as epic-scale
  work filed as a single story.

- 🟡 **Scope**: Declared "story" kind looks undersized for the described
  scope
  **Location**: Frontmatter: kind
  Two new crates, a provider-agnostic port verified by dependency-graph
  inspection, a five-state sync pipeline, and a 14-script characterization
  backlog together exceed what the kind-specific guidance treats as
  story-sized.

- 🟡 **Dependency**: 0171 is a downstream consumer of the tracker port but
  is not captured as a Blocks entry
  **Location**: Dependencies
  Context states 0171's clients implement the `RemoteTracker` port this
  story defines, but Dependencies lists 0171 only as "Relates to," and
  frontmatter has no `blocks` field — the ordering constraint is invisible
  to anyone scheduling from Dependencies alone.

- 🟡 **Dependency**: In-process wiring of 0171's client adapters is
  described as in-scope while 0171 is still draft, but this coupling isn't
  captured as a blocker
  **Location**: Summary
  The Summary and Context describe `accelerator-work` linking 0171's
  provider adapters in-process; if that wiring is genuinely required for
  completion (not just the faked-port ACs), 0171 is an unstated upstream
  blocker.

- 🔴 **Completeness**: Open Questions declares "None" while Drafting Notes
  admits an unresolved design risk
  **Location**: Open Questions
  Drafting Notes flags the subcommand-vocabulary grouping as "a judgment
  call, not confirmed against an actual implementation spike," which is a
  live open question not surfaced where readers expect to find one.

- 🔴 **Testability**: Characterization-test criterion has no fixed,
  enumerable scope
  **Location**: Acceptance Criteria
  The same ~14-script ambiguity noted under Clarity means a verifier cannot
  produce a definitive pass/fail — there is no complete list to check "all
  of them" against.

- 🟡 **Testability**: No Acceptance Criteria for three of the six
  user-facing subcommands
  **Location**: Acceptance Criteria
  Technical Notes commits to six subcommands (`create`, `update`, `sync`,
  `show`, `resolve`, `diff`); only the first three have Given/When/Then
  criteria, leaving `show`/`resolve`/`diff` without a defined verification
  procedure.

- 🟡 **Clarity**: "the dispatcher" in AC1 has no defined referent
  **Location**: Acceptance Criteria
  "Dispatcher" is a term of art elsewhere in the epic (launcher-level
  sub-binary dispatch, ADR-0054) but AC1 appears to mean the remote
  create call succeeding — as written it risks an incorrect no-partial-file
  guard condition.

- 🟡 **Clarity**: The "~14 previously-untested work-item-* scripts" set is
  enumerated inconsistently
  **Location**: Acceptance Criteria / Technical Notes
  Context/Requirements say "~14," AC5 names eight scripts plus "etc.," and
  the Technical Notes' internal-only list names a different, non-identical
  seven — no section gives one authoritative, closed enumeration.

- 🟡 **Clarity**: Dispatch-token naming example appears to contradict the
  resolved subcommand vocabulary
  **Location**: Technical Notes
  The 2026-08-01 note implies a `work-item` dispatch token, while the rest
  of the document (including the 2026-08-05 resolution) establishes `work`
  as the subcommand namespace — unclear whether this is stale or signals a
  genuinely different registered token.

#### Minor

- 🔵 **Clarity**: Bare "resolved Q2"/"resolved Q7" references require
  chasing an external document's numbering
  **Location**: Summary, Requirements, Assumptions
  Neither question is restated inline, so a reader relying on this work
  item alone can't verify what was resolved without opening and counting
  through the source research document.

- 🔵 **Completeness**: Story does not identify who or what benefits from
  the work
  **Location**: Context
  The Summary/Context describe technical scope but no beneficiary (e.g.,
  skill authors, future maintainers) is named, as the Story kind calls for.

- 🔵 **Completeness**: Status remains "draft" despite content that reads
  as implementation-ready
  **Location**: Frontmatter: status
  Both named blockers are done, the sole open question is resolved, and
  Acceptance Criteria/Technical Notes are implementation-level detailed —
  more typical of `ready` than `draft`.

- 🔵 **Dependency**: 0187's blocker-resolved status rests on informal
  confirmation the work item itself flags as unconfirmed
  **Location**: Dependencies
  Dependencies asserts 0187 is "done," but Drafting Notes notes 0187's own
  frontmatter still shows `status: ready` and that updating it was left
  out of scope — the blocker-cleared claim rests on informal confirmation
  rather than the authoritative field.

- 🔵 **Scope**: Legacy-script characterization work is a distinct
  engineering activity bundled into the feature story
  **Location**: Requirements
  Writing characterization tests for ~14 untested bash scripts is
  test-debt closure ahead of a port, a different kind of work from
  building the new CRUD/sync capability, even though it's a reasonable
  precondition for a safe port.

- 🔵 **Testability**: "A characterization test captures its pre-port
  behaviour" sets no coverage bar
  **Location**: Acceptance Criteria
  No minimum bar (flags covered, error paths, etc.) is specified, so a
  single trivial test would technically satisfy the letter of the
  criterion.

- 🔵 **Testability**: "fully populated frontmatter" lacks an in-document
  definition
  **Location**: Acceptance Criteria
  The `create` criterion doesn't state which fields constitute "fully
  populated," nor reference a schema, leaving a verifier unable to
  determine a failing case from this document alone.

#### Suggestions

- 🔵 **Clarity**: Lowercase "the remote tracker" risks conflation with the
  capitalised `RemoteTracker` port
  **Location**: Context
  The generic external-system phrasing sits close to the capitalised
  trait name introduced in the same document, inviting a quick
  misreading.

- 🔵 **Clarity**: "commit sequence" in AC3 could be misread as a VCS
  operation
  **Location**: Acceptance Criteria
  In a codebase where jj/git commits are a heavily-discussed concept,
  "the per-item commit sequence" could briefly read as a VCS commit rather
  than the sync pipeline's own write sequencing.

- 🔵 **Testability**: "explicitly-tagged contract/integration suite"
  doesn't name the tag or mechanism
  **Location**: Acceptance Criteria
  The parity-suite criterion doesn't name the specific tagging convention
  (e.g., a nextest filter or cargo feature) a verifier would check network
  isolation against.

### Strengths

- ✅ Acceptance Criteria are unusually thorough for a story, using
  consistent Given/When/Then framing and anchoring several criteria to
  concrete pre-existing parity fixtures (e.g., `work-item-sync-classify.sh`,
  `work-item-sync-decide.sh`).
- ✅ Dependencies is fully worked and consistently referenced: prior
  blockers (0166, 0187) are marked done with dates, and crate/binary naming
  (`accelerator-work`, `tracker`) is used consistently throughout.
- ✅ Technical Notes captures a complete, dated resolution of the
  subcommand-vocabulary open question, including a concrete naming gotcha
  (dispatch-token underscore restriction) and a mapping of each legacy
  script to its new home.
- ✅ The Drafting Notes proactively acknowledge that the tracker/
  accelerator-work split is "tightly related but separable," showing
  self-awareness of the decomposition question the scope lens later raises
  independently.
- ✅ The tracker crate's public-API purity criterion ("no `reqwest` or
  provider-crate types in public signatures") is a strong example of a
  testable, mechanically-checkable design constraint.

### Recommended Changes

1. **Split the story into a lifecycle-CRUD story and a tracker+sync story**
   (addresses: "Story bundles lifecycle CRUD and the sync engine," "story
   kind looks undersized," "in-process wiring of 0171's client adapters")
   Separate `accelerator-work`'s CRUD surface (`create`/`update`/`show`/
   `resolve`/`diff`/`next-number`/`normalise`/`section-diff`) from the
   `tracker` crate and `sync` command; sequence sync after CRUD since it
   depends on local file operations, and let each land independently.

2. **Consolidate the ~14-script list into one authoritative enumeration**
   (addresses: "the ~14 ... enumerated inconsistently," "characterization-
   test criterion has no fixed, enumerable scope")
   Give a single exact list referenced consistently by Requirements, AC5,
   and the Technical Notes' internal-only list, rather than three
   overlapping partial subsets.

3. **Add an explicit Blocks relationship from 0170 to 0171 and clarify
   real-provider wiring timing** (addresses: "0171 ... not captured as a
   Blocks entry," "in-process wiring of 0171's client adapters")
   Add `blocks: ["work-item:0171"]` to frontmatter and a Dependencies
   bullet naming the `RemoteTracker`-port coupling; state explicitly
   whether this story's own completion requires real-provider wiring or
   only the faked port.

4. **Replace "the dispatcher succeeds" with a concrete named condition**
   (addresses: "'the dispatcher' in AC1 has no defined referent")
   Name the actual gate, e.g. "the remote create call via the wired
   `RemoteTracker` client succeeds," to avoid confusion with launcher-level
   dispatch.

5. **Reconcile the Technical Notes' dispatch-token example with the
   resolved subcommand vocabulary** (addresses: "Dispatch-token naming
   example appears to contradict the resolved subcommand vocabulary")
   Confirm the registered token is `work` (dropping the now-irrelevant
   hyphen/underscore example) or explain why a different token applies.

6. **Add Acceptance Criteria for `show`, `resolve`, and `diff`**
   (addresses: "No Acceptance Criteria for three of the six user-facing
   subcommands")
   Give each a Given/When/Then criterion, ideally anchored to the bash
   scripts they replace (`work-item-read-field.sh`/`work-item-read-status.sh`,
   `work-item-resolve-id.sh`, `work-item-section-diff.sh`).

7. **Move the subcommand-grouping caveat from Drafting Notes into Open
   Questions** (addresses: "Open Questions declares 'None' while Drafting
   Notes admits an unresolved design risk")
   Surface it as a live, lower-priority question to validate once
   `accelerator-work` scaffolding starts.

8. **Reconcile status fields with actual readiness** (addresses: "Status
   remains 'draft' despite content that reads as implementation-ready,"
   "0187's blocker-resolved status rests on informal confirmation")
   Update 0170's `status` to `ready` if intended, and update 0187's
   frontmatter `status` field to `done` rather than relying on informal
   confirmation.

## Per-Lens Results

### Clarity

**Summary**: 0170 is a mature, well-structured draft: consistent naming of
the accelerator-work binary, the tracker crate, and the sync pipeline stages
keeps most referents unambiguous, and the Given/When/Then Acceptance
Criteria are largely self-contained. The main clarity gaps are a handful of
undefined or inconsistent terms — an unexplained "the dispatcher" in AC1, a
dispatch-token example in Technical Notes that appears to contradict the
resolved subcommand vocabulary, an inconsistently enumerated "~14 scripts"
set across three sections, and bare "resolved Q2/Q7" cross-references that
require the reader to chase an external document's numbering.

**Strengths**:
- Crate and binary naming (accelerator-work, tracker, "the work binary") is
  used consistently throughout, so the reader never has to guess which
  component a reference points to.
- The five-state sync classification (unsynced / local-ahead / remote-ahead
  / in-sync / conflict) is spelled out explicitly in AC3 rather than left
  implicit.
- The Open Questions section explicitly states "None" and points to
  exactly where the one resolved question is documented (Technical Notes),
  rather than leaving a stale placeholder.

**Findings**:
- 🟡 major/high — "the dispatcher" in AC1 has no defined referent.
  **Location**: Acceptance Criteria. AC1 gates the no-partial-file
  substitution on "the dispatcher" succeeding, but that term is a term of
  art elsewhere in the epic (launcher dispatch) and never defined here.
- 🟡 major/medium — The "~14 previously-untested work-item-* scripts" set
  is enumerated inconsistently. **Location**: Acceptance Criteria /
  Technical Notes. Three sections give three different partial lists with
  no single closed enumeration.
- 🟡 major/medium — Dispatch-token naming example appears to contradict the
  resolved subcommand vocabulary. **Location**: Technical Notes. The
  2026-08-01 note implies token `work-item`; the rest of the document
  establishes `work`.
- 🔵 minor/medium — Bare "resolved Q2"/"resolved Q7" references require
  chasing an external document's numbering. **Location**: Summary,
  Requirements, Assumptions.
- 🔵 suggestion/low — Lowercase "the remote tracker" risks conflation with
  the capitalised `RemoteTracker` port. **Location**: Context.
- 🔵 suggestion/low — "commit sequence" in AC3 could be misread as a VCS
  operation. **Location**: Acceptance Criteria.

### Completeness

**Summary**: The work item is structurally strong: every expected section
is present and most are unusually well populated, especially Acceptance
Criteria (eight specific Given/When/Then criteria) and Technical Notes (a
fully worked subcommand-vocabulary resolution). The main completeness gaps
are a status field that appears to lag the content's actual readiness, an
acknowledged design uncertainty that lives only in Drafting Notes rather
than being surfaced in Open Questions, and the absence of an explicit
statement of who benefits from this story, which the Story kind calls for.

**Strengths**:
- Acceptance Criteria are exceptionally thorough for a story — eight
  Given/When/Then criteria covering create, update, sync classification/
  decision tables, characterization testing of untested scripts, parity-
  suite passage, the tracker crate's dependency-graph purity, and script/
  suite removal.
- Dependencies is fully worked: prior blockers (0166, 0187) are explicitly
  marked done with a date, the 0171 relationship is characterized, and the
  parent epic is named.
- Technical Notes captures a complete, dated resolution of the subcommand-
  vocabulary open question, including the mapping of each legacy script to
  its new home (subcommand vs. private function) and a concrete naming
  gotcha (dispatch token underscore restriction).
- Requirements and Context together give an implementer enough
  architectural grounding to begin work without further clarification on
  scope.

**Findings**:
- 🔴 major/high — Open Questions declares "None" while Drafting Notes
  admits an unresolved design risk. **Location**: Open Questions.
- 🔵 minor/medium — Story does not identify who or what benefits from the
  work. **Location**: Context.
- 🔵 minor/medium — Status remains "draft" despite content that reads as
  implementation-ready. **Location**: Frontmatter: status.

### Dependency

**Summary**: 0170 does a good job resolving its two named upstream
blockers (0166, 0187) with dated confirmation, but the downstream/lateral
coupling with 0171 is inconsistently captured: the Context and Assumptions
describe 0170's binary as linking 0171's client adapters in-process, yet
Dependencies only lists 0171 as "Relates to... still draft" rather than as
a blocker or a Blocks entry, and there is no frontmatter `blocks` field
pointing at 0171. There is also an unresolved internal inconsistency about
whether 0187 is genuinely "done" as claimed.

**Strengths**:
- Dependencies section names both prior blockers (0166, 0187) explicitly
  and dates their resolution (2026-08-05), rather than leaving them as
  vague prose.
- Technical Notes proactively surfaces the no-underscore dispatch-token
  constraint inherited from 0187, preventing a foreseeable registration
  mistake.
- Context and Requirements are explicit about which crate (`tracker`) is
  shared with 0171 and what contract it exposes, giving a concrete basis
  for coordinating the two stories even where the formal Dependencies
  fields fall short.

**Findings**:
- 🟡 major/high — 0171 is a downstream consumer of the tracker port but is
  not captured as a Blocks entry. **Location**: Dependencies.
- 🟡 major/medium — In-process wiring of 0171's client adapters is
  described as in-scope while 0171 is still draft, but this coupling isn't
  captured as a blocker. **Location**: Summary.
- 🔵 minor/medium — 0187's blocker-resolved status rests on informal
  confirmation the work item itself flags as unconfirmed in the
  authoritative record. **Location**: Dependencies.

### Scope

**Summary**: This story bundles two substantial, architecturally distinct
efforts — a CRUD-style work-item lifecycle subdomain and a five-stage
remote sync engine backed by a brand-new `tracker` crate — under a single
story declaration, a bundling the Summary itself flags with "plus." The
eight acceptance criteria, two new crates, a provider-agnostic port/state-
machine, and a 14-script characterization backlog together describe multi-
week, multi-capability scope more consistent with an epic-level unit than a
single story. The item is otherwise well-bounded in its detail (resolved
open questions, explicit dependency state), but the unit of delivery itself
looks too large and too internally heterogeneous.

**Strengths**:
- The Drafting Notes explicitly acknowledge the tracker/accelerator-work
  split is "tightly related but separable," showing the author already
  recognises the decomposition question rather than glossing over it.
- The resolved subcommand-vocabulary section gives a concrete, bounded
  shape to what would otherwise be an open-ended 22-script consolidation,
  meaningfully narrowing the scope of the CRUD side of the story.
- Dependencies are cleanly resolved (0166, 0187 done) and the relationship
  to the sibling 0171 story is named explicitly rather than left implicit.

**Findings**:
- 🔴 major/high — Story bundles lifecycle CRUD and the sync engine as two
  independently deliverable concerns. **Location**: Summary.
- 🟡 major/medium — Declared "story" kind looks undersized for the
  described scope. **Location**: Frontmatter: kind.
- 🔵 minor/medium — Legacy-script characterization work is a distinct
  engineering activity bundled into the feature story. **Location**:
  Requirements.

### Testability

**Summary**: Most Acceptance Criteria are strongly testable: they use
Given/When/Then framing, tie to existing bash parity fixtures, and one
criterion (the tracker crate's dependency-graph purity check) is a model of
a concrete, mechanically-verifiable design constraint. The two significant
gaps are an incompletely-enumerated criterion for the ~14 characterization
tests (using an approximate count plus "etc.") that cannot be conclusively
checked off, and the absence of any Acceptance Criteria for three of the
six user-facing subcommands described in the Technical Notes (`show`,
`resolve`, `diff`).

**Strengths**:
- Acceptance Criteria consistently use Given/When/Then framing appropriate
  for a story, with explicit preconditions and observable outcomes rather
  than implementation instructions.
- Several criteria anchor verification to concrete, pre-existing artefacts
  (e.g., "work-item-sync-classify.sh parity fixtures," "work-item-sync-
  decide.sh's forbidden-write cells"), giving a tester an unambiguous
  oracle to compare against.
- The tracker crate's public-API purity criterion is an excellent example
  of a testable, mechanically-checkable design constraint rather than a
  subjective quality statement.

**Findings**:
- 🔴 major/high — Characterization-test criterion has no fixed, enumerable
  scope. **Location**: Acceptance Criteria.
- 🟡 major/medium — No Acceptance Criteria for three of the six user-facing
  subcommands (`show`, `resolve`, `diff`). **Location**: Acceptance
  Criteria.
- 🔵 minor/medium — "A characterization test captures its pre-port
  behaviour" sets no coverage bar. **Location**: Acceptance Criteria.
- 🔵 minor/low — "fully populated frontmatter" lacks an in-document
  definition. **Location**: Acceptance Criteria.
- 🔵 suggestion/low — "explicitly-tagged contract/integration suite"
  doesn't name the tag or mechanism. **Location**: Acceptance Criteria.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-05T18:18:52+00:00

**Verdict:** APPROVE *(overridden from the suggested COMMENT verdict by Toby
Clemson on 2026-08-05T19:12:57+00:00 — the two remaining items are an
explicitly by-design status-field deferral and an accepted scope
trade-off, neither a quality gap in the work item itself.)*

All eight recommended changes were addressed, most substantially via a
story split: 0170 (Work-Item Subdomain and Sync Engine) split into 0170
(Work-Item Lifecycle Subdomain) and a new sibling, 0194 (Tracker Crate and
Remote Sync Engine). 0171 and the parent epic 0136 were updated in lockstep
to keep the dependency graph coherent. All five lenses were re-run against
the updated 0170; the split resolved every major finding from Pass 1, and
the re-review surfaced a small number of new issues — mostly artefacts of
the split itself — all of which were fixed in this same pass.

### Previously Identified Issues

- 🟡 **Scope**: Story bundles lifecycle CRUD and the sync engine — Resolved
  (split into 0170 + 0194).
- 🟡 **Scope**: Declared "story" kind undersized — Resolved (each split
  item is story-sized; confirmed by the scope re-review).
- 🟡 **Dependency**: 0171 not captured as a Blocks entry — Resolved (0194
  now `blocks: ["work-item:0170", "work-item:0171"]`).
- 🟡 **Dependency**: In-process wiring of 0171's client adapters not
  captured as a blocker — Resolved (dependency direction clarified: 0194's
  `RemoteTracker` port blocks both 0170's `--push` flows and 0171).
- 🔴 **Completeness**: Open Questions declares "None" while Drafting Notes
  admits an unresolved risk — Resolved (the subcommand-grouping caveat now
  lives in Open Questions on both 0170 and 0194).
- 🔴 **Testability**: Characterization-test criterion has no fixed,
  enumerable scope — Resolved (verified against the actual
  `skills/work/scripts/` inventory: 11 lifecycle-side scripts in 0170, 4
  sync-side scripts in 0194, cross-checked against which already have a
  dedicated `test-work-item-*.sh` suite).
- 🟡 **Testability**: No Acceptance Criteria for `show`/`resolve`/`diff` —
  Resolved (three new Given/When/Then criteria added).
- 🟡 **Clarity**: "the dispatcher" in AC1 has no defined referent —
  Resolved (reworded to name the `RemoteTracker` port explicitly).
- 🟡 **Clarity**: The ~14-script set enumerated inconsistently — Resolved
  (same fix as the testability finding above).
- 🟡 **Clarity**: Dispatch-token example contradicts the resolved
  vocabulary — Resolved (Technical Notes reconciled to confirm the token is
  `work`).
- 🔵 **Clarity**: Bare "resolved Q2"/"resolved Q7" references — Resolved,
  then further tightened in this pass after the re-review found the
  "Resolved QN" labels themselves had become orphaned once the original
  numbered Open Questions list no longer existed in the document; both
  items now state the decisions as plain prose with no dangling Q-number.
- 🔵 **Completeness**: No beneficiary named for a story-kind item —
  Resolved (Context now names plugin maintainers as the beneficiary).
- 🔵 **Completeness**: Status remains "draft" despite ready-looking content
  — Still present, by design: status transitions are a separate workflow
  decision, not made during work item editing.
- 🔵 **Dependency**: 0187's "done" status rests on informal confirmation —
  Still present, by design (same reason).
- 🔵 **Scope**: Characterization-test debt bundled with feature delivery —
  Still present; re-confirmed as a defensible, explicitly-accepted
  trade-off rather than fixed.
- 🔵 **Testability**: "captures its pre-port behaviour" sets no coverage
  bar — Resolved (both items' characterization ACs now require covering
  each flag/argument combination and at least one error path).
- 🔵 **Testability**: "fully populated frontmatter" undefined — Resolved
  (AC1 now references the `create-work-item` template schema).
- 🔵 **Clarity**: Lowercase "the remote tracker" risked conflation with the
  port — Resolved as a side effect of the split (the phrase no longer
  appears in 0170's rewritten Context).
- 🔵 **Clarity**: "commit sequence" could be misread as a VCS operation —
  Resolved as a side effect of the split (0194's classify/decide AC now
  says "write sequence").
- 🔵 **Testability**: Contract/integration suite tag unnamed — Partially
  resolved (both items now name the mechanism — a cargo-nextest filter
  excluded from the default `cargo test`/`cargo nextest run` invocation —
  though not a specific filter name).

### New Issues Introduced

- 🔴 **Clarity**: "Resolved Q2"/"Resolved Q7" labels became orphaned
  references once the original numbered Open Questions list no longer
  existed in the document — Resolved (both items restate the decisions as
  plain prose).
- 🟡 **Clarity**: Dependencies' "no remaining blockers of its own" read as
  self-contradictory next to the following "Blocked by: 0194" bullet —
  Resolved (reworded to scope the claim to the pre-split blockers
  explicitly).
- 🟡 **Clarity**: "the existing push state machine" was undefined and
  risked conflation with 0194's distinct sync state machine — Resolved
  (AC1 now names `work-item-create-remote.sh`'s existing outcome table
  instead of an invented "state machine" term).
- 🔴 **Testability** / 🔵 **Scope**: `work-item-fetch-remote.sh`'s test
  suite was named in 0170's parity gate and removal criterion, but no
  command in either split item actually absorbs its behaviour — it is a
  dependency of `work-item-sync-apply.sh`, not of any lifecycle command —
  Resolved (moved to 0194's scope; 0170's parity/removal criteria no
  longer reference it).
- 🔵 **Clarity**: "RemoteTracker client" (AC1) vs. "RemoteTracker port"
  (Requirements/Assumptions) — inconsistent terminology — Resolved
  (AC1 now says "port" throughout).
- 🔵 **Completeness**: The known stale `external_id` (still reflecting the
  pre-split scope) was recorded only in Drafting Notes, not as a trackable
  follow-up — Resolved (added to Open Questions).
- 🔵 **Dependency**: 0171 named in Assumptions as part of the real-wiring
  path but absent from Dependencies/`relates_to` — Resolved (added a
  `relates_to` entry and a Dependencies bullet).
- 🔵 **Testability**: Push-failure branches for `create`/`update --push`
  were only implied, not stated as their own observable outcome —
  Resolved (both criteria now verified against the actual documented
  failure behaviour — `create`'s outcome table, `update`'s
  retryable/terminal exit taxonomy — rather than an invented claim).
- 🔵 **Clarity**: The filename `0170-work-item-subdomain-and-sync-engine.md`
  still names the pre-split scope even though the title and body now read
  "Work-Item Lifecycle Subdomain" — Not resolved; accepted as a low-cost
  cosmetic trade-off (renaming would require updating an inbound reference
  in 0179) rather than a substantive gap.

### Assessment

0170 and its new sibling 0194 are both in good shape for planning. The
story-scope, dependency-graph, and enumeration issues that drove the
original REVISE verdict are resolved through the split; the issues the
split itself introduced (mostly terminology drift and one script
mis-assigned to the wrong item) were caught by the re-review and fixed in
this same pass. What remains is two by-design deferrals (status-field
transitions, left to a separate workflow) and one accepted scope trade-off
(characterization-test debt bundled with feature work) — neither blocks
implementation. Recommended next step: `/update-work-item` to reconcile
0170's and 0187's `status` fields, and a future `/sync-work-items` pass to
push 0170's narrowed scope to its existing remote issue (PP-191) and create
a new remote issue for 0194.
