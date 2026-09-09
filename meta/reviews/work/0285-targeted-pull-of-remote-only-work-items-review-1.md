---
type: "work-item-review"
id: "0285-targeted-pull-of-remote-only-work-items-review-1"
title: "Work Item Review: Targeted Pull of Remote-Only Work Items and Resolution Normalisation"
date: "2026-09-08T18:47:27+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0285"
work_item_id: "0285"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-08T19:24:23+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Targeted Pull of Remote-Only Work Items and Resolution Normalisation

**Verdict:** REVISE

The work item is structurally complete, well-scoped, and unusually precise on
exit codes and the two collision cases — completeness found nothing, and scope
confirms a single coherent increment correctly parented under epic 0146. It
falls short of APPROVE on two axes: several behaviours the Requirements and
Technical Notes name (report/skill rendering, watermark advance for the new
item, live `--max-pulls` bounding) have no acceptance criterion, and the work
item silently drops dependency detail its predecessor 0257 chose to make
visible. A surface contradiction between the local-precedence rule and the
collision-abort rule compounds the risk.

### Cross-Cutting Themes

- **Parity regression against predecessor 0257** (flagged by: dependency,
  testability) — 0257 explicitly captured the external tracker coupling, an
  integration watch-out against siblings 0229/0255, a watermark/resumability
  criterion, and a `--max-pulls` criterion. 0285 extends the same engine but
  carries none of these forward, despite adding a new remote path. This is the
  dominant theme and accounts for four of the six major/minor findings.
- **Local-vs-remote resolution precedence** (flagged by: clarity, scope) — the
  resolution normalisation is the work item's second capability; clarity finds
  its two precedence rules read as contradictory, and scope asks whether the
  normalisation is a separable deliverable or a necessary consequence of the
  pull.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: 'reconciled or created appropriately' is tautological
  **Location**: Acceptance Criteria
  The mixed-targets criterion uses 'appropriately', which has no defined
  pass/fail outcome — any behaviour could be argued to satisfy it, adding no
  verification value beyond the per-shape criteria it summarises.

- 🟡 **Testability**: Report-line and skill-rendering change has no acceptance
  criterion
  **Location**: Requirements
  Requirements mandate updating the report lines and `sync-work-items` skill
  rendering for the pull-create and the exit-2 collision, but no criterion
  specifies the expected output for either — the visible result of the two
  central new behaviours is unverifiable.

- 🟡 **Clarity**: Local-precedence rule appears to contradict the
  collision-abort rule
  **Location**: Requirements
  The fourth Requirement says a local match 'wins and selects that local item'
  unconditionally; the sixth says a token that is one file's id and a *different*
  file's `external_id` must abort (exit 2). The precedence rule does not carve
  out the collision exception, so the two read as conflicting.

- 🟡 **Dependency**: External tracker coupling absent from Dependencies
  **Location**: Dependencies
  The work item adds a new by-id `tracker.show(external_id)` fetch — remote
  contact where today's code aborts without it — yet Dependencies records only
  'Blocked by: none' / 'Blocks: none' and never names the remote tracker
  (Linear). Predecessor 0257 captured this external coupling explicitly.

- 🟡 **Dependency**: 0257 mischaracterised as a non-blocker
  **Location**: Dependencies
  Dependencies states 'Blocked by: none (0257 is in review)', but 0285 directly
  extends 0257's unmerged code — the exit-3 abort it inverts, the
  `resolve_targets` arm it modifies, the discovery suppression, the pull-bound
  accounting. This is a hard upstream ordering gate, not 'none'.

- 🟡 **Testability**: Watermark advance for a targeted pull-create is not
  verified
  **Location**: Acceptance Criteria
  Requirements and Technical Notes require the per-item watermark
  (`local_synced_at`) to extend to a newly-created targeted item, but no
  criterion verifies that a re-run does not re-pull it. 0257 had an explicit
  baseline-advance / resumability criterion; 0285 has none for the new item.

#### Minor

- 🔵 **Dependency**: Integration watch-out not carried forward
  **Location**: Dependencies
  0285 modifies the shared engine pull/discovery path that 0257 flagged as an
  integration watch-out against siblings 0229 (pull scope) and 0255 (chunk
  merge); 0285 carries no equivalent note, risking merge conflicts discovered at
  implementation rather than planning time.

- 🔵 **Testability**: `--max-pulls` bounding only verified under `--preview`
  **Location**: Acceptance Criteria
  AC3 asserts the pull count only under `--preview` (a zero-write path). No
  criterion verifies bounding in a real writing run when pull-creates exceed
  `--max-pulls`, leaving the safety-relevant blast-radius guarantee untested for
  the live path.

- 🔵 **Clarity**: Relationship between `--preview` and `--dry-run` undefined
  **Location**: Requirements
  The third Requirement writes '`--preview`/`--dry-run`' as if interchangeable,
  but never states whether they are one aliased flag or two. `--dry-run` appears
  once; every criterion and 0257 use only `--preview`.

- 🔵 **Clarity**: 'Missed in the definition' conflicts with Summary's
  'explicitly scoped out'
  **Location**: Context
  The Summary frames the 0257 gap as 'explicitly — and wrongly — scoped out' (a
  deliberate decision); the Context says it 'was missed in the definition' (an
  accident). The mixed framing affects how the reader reads 0257's other scope
  decisions.

#### Suggestions

- 🔵 **Scope**: Title and Summary name two capabilities
  **Location**: Summary
  The pull of a remote-only item (new feature) and 'resolution normalisation' (a
  behavioural change to shipped 0257 semantics) could in principle ship or roll
  back independently. The work item argues persuasively they share the same
  `resolve_targets` decision — if so, state explicitly that the normalisation is
  a necessary consequence, not a separable second deliverable.

- 🔵 **Testability**: 'via the same path as a discovery import' leans on
  implementation
  **Location**: Acceptance Criteria
  AC1 references an internal code path rather than an observable outcome; the
  verifiable part is carried by AC2's field-equivalence comparison. Reword AC1 to
  the observable result and let AC2 carry the equivalence claim.

- 🔵 **Clarity**: 'corpus'/'local corpus' introduced without definition
  **Location**: Context
  0285 uses 'local corpus' where 0257 says 'work directory' / 'local work items'.
  The term is inferable but the vocabulary shift could momentarily suggest a
  distinct concept. Define on first use or reuse 0257's phrasing.

### Strengths

- ✅ Every expected section is present and substantively populated; frontmatter
  is valid with a recognised kind, status, priority, and parent/relates_to
  linkage consistent with 0257 and epic 0146.
- ✅ The two collision cases are defined with precise, mutually exclusive
  criteria, and exit-code semantics (exit 2 usage error, exit 3
  `RESOLVE_NOT_FOUND`) are used consistently across Context, Requirements, and
  Acceptance Criteria.
- ✅ Most acceptance criteria state outcomes as observable system states — file
  created, nothing written, counts against `--max-pulls`, byte-identical
  full-sync report — with concrete tokens (`PP-999`) and a testable 'no remote
  call' constraint (AC5).
- ✅ Deliberately reuses existing mechanisms (`create_from_remote`, the
  discovery-import id allocation, pull-bound accounting, standard conflict
  resolution) rather than inventing parallel machinery, keeping it a single
  coherent increment confined to one engine.

### Recommended Changes

1. **Add acceptance criteria for the unverified behaviours** (addresses:
   Report-line and skill-rendering change has no acceptance criterion; Watermark
   advance not verified; `--max-pulls` bounding only verified under `--preview`)
   Add criteria pinning: the report line(s) for a targeted pull-create and the
   exit-2 collision (mirroring how 0257 pinned the discovery line to 'skipped');
   that a re-run after a pull-create does not re-pull and its watermark has
   advanced; and that pull-creates exceeding `--max-pulls N` in a real run cap at
   N with the remainder reported as bounded.

2. **Resolve the local-precedence / collision-abort contradiction** (addresses:
   Local-precedence rule appears to contradict the collision-abort rule)
   Qualify the fourth Requirement so 'a local-id match wins' explicitly excludes
   the local/local collision defined in the sixth Requirement, or cross-reference
   the collision rule as an exception.

3. **Restore the dependency detail 0257 carried** (addresses: External tracker
   coupling absent; 0257 mischaracterised as a non-blocker; Integration watch-out
   not carried forward)
   Add an 'External' line naming the remote tracker (Linear via
   `work.integration`) and the new by-id pull path's reachability dependency;
   record 0257 as 'Blocked by: 0257 until merged'; and carry forward the
   integration watch-out against 0229 and 0255 on the shared pull/discovery path.

4. **Replace the tautological mixed-targets criterion** (addresses: 'reconciled
   or created appropriately' is tautological)
   State the concrete disposition per shape — local-id/path tokens reconcile
   their existing local file, remote-only ids create-and-reconcile a new local
   file, no non-targeted item is written.

5. **Tidy the minor clarity and framing issues** (addresses: `--preview` /
   `--dry-run` undefined; 'Missed' vs 'explicitly scoped out'; 'corpus'
   undefined; two-capability framing; AC1 phrasing)
   State whether `--dry-run` is an alias for `--preview` or drop it; pick one
   framing for the 0257 gap; define or replace 'local corpus'; state in the
   Summary that the normalisation is a necessary consequence of the pull; and
   reword AC1 to an observable outcome.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Work item 0285 is unusually precise: it names concrete exit codes,
distinguishes the two collision cases with sharp definitions, and reuses domain
vocabulary consistently with its predecessor 0257. The main clarity risk is a
surface-level contradiction between the local-precedence requirement and the
collision requirement, plus a couple of minor undefined-term and framing issues.
Actors and outcomes are mostly clear, stated as observable states (file created,
exit 2/3, zero writes).

**Strengths**:
- The two collision cases are defined with precise, mutually exclusive criteria
  — 'a token that resolves to the local file carrying the matching external_id'
  (reconcile normally) versus 'a token that is one local file's id and
  simultaneously a *different* local file's external_id' (abort exit 2) — leaving
  little room for misinterpretation.
- Exit-code semantics (exit 2 usage error for local/local collision, exit 3
  RESOLVE_NOT_FOUND for no match) are used consistently across Context,
  Requirements, and Acceptance Criteria.
- Acceptance Criteria state outcomes as observable system states (file created,
  nothing written, counts against --max-pulls, byte-identical full-sync report)
  rather than vague desired properties.
- Domain terms carried over from 0257 (token, external_id, blast-radius,
  discovery suppression) are used consistently, so a reader of the predecessor
  faces no vocabulary drift on those terms.

**Findings**:
- 🟡 major (confidence: medium) — **Local-precedence rule appears to contradict
  the collision-abort rule** (Requirements). The fourth Requirement states a
  target with a local match 'wins and selects that local item', phrased
  unconditionally. The sixth Requirement then says that when a token is one local
  file's id and *also* a different local file's external_id — a case that does
  have a local-id match — the run must instead abort as a usage error (exit 2). A
  reader following the fourth bullet would select and reconcile the local-id
  item; the sixth bullet says do not, abort. Impact: an implementer or reviewer
  cannot tell from the Requirements alone whether the local-id match is honoured
  or the run aborts in the collision case. Suggestion: qualify the fourth
  Requirement so 'a local-id match wins' explicitly excludes the local/local
  collision case, or cross-reference the collision rule as an exception.
- 🔵 minor (confidence: medium) — **Relationship between --preview and --dry-run
  left undefined** (Requirements). The third Requirement writes
  '`--preview`/`--dry-run`' as though interchangeable, but never states whether
  these are two names for one flag or two distinct flags. Every criterion and
  0257 use only `--preview`; `--dry-run` appears once without explanation.
  Suggestion: state explicitly whether `--dry-run` is an alias or a separate
  flag, or drop it.
- 🔵 minor (confidence: medium) — **'Missed in the definition' conflicts with
  Summary's 'explicitly scoped out'** (Context). The Summary frames the omission
  as a deliberate exclusion ('explicitly — and wrongly — scoped out'); the
  Context frames it as an oversight ('missed in the definition'). The mixed
  framing affects how confidently the reader trusts 0257's other scope decisions.
  Suggestion: pick one framing and use it in both Summary and Context.
- 🔵 suggestion (confidence: low) — **'Corpus'/'local corpus' introduced without
  definition** (Context). 0285 uses 'local corpus' where 0257 says 'work
  directory' / 'local work items'. The meaning is inferable, but the vocabulary
  shift could momentarily suggest a distinct concept. Suggestion: define on first
  use or reuse 0257's phrasing.

### Completeness

**Summary**: Work item 0285 is a structurally complete story: every expected
section (Summary, Context, Requirements, Acceptance Criteria, Open Questions,
Dependencies, Assumptions, Technical Notes, Drafting Notes, References) is
present and substantively populated. It carries the kind-appropriate content a
story demands — an identified actor (an engineer maintaining work items
locally), a Context that explains why the work is needed, and eight specific
Given/When/Then acceptance criteria. Frontmatter is valid with a recognised
kind, status, priority, and parent linkage.

**Strengths**:
- Summary follows a clear user-story form (as-a / I-want / so-that) and states
  precisely what is being built — a targeted pull of remote-only items plus
  resolution normalisation.
- Context explains the motivation concretely: it traces the gap 0257 scoped out
  and the wrong 'suppressed remote match' framing, giving a reader the forces
  behind the work without follow-up.
- Acceptance Criteria section is dense and specific — eight Given/When/Then
  bullets covering the happy path, preview/bounds, collision, not-found, mixed
  targets, and full-sync regression.
- Requirements are detailed and actionable, each tied to a concrete behaviour
  (by-id fetch, create-from-remote reuse, exit codes), and Assumptions plus
  Dependencies are explicitly populated rather than left blank.
- Frontmatter integrity is sound: kind (story), status (draft), priority, parent
  (0146), and relates_to (0257) are all present and consistent with the
  referenced predecessor and epic.

**Findings**: None.

### Dependency

**Summary**: 0285 is a story that directly extends the targeted-sync feature
delivered by 0257, reusing its resolution, discovery-suppression, and
create-from-remote machinery, and it introduces a new by-id remote fetch
(`tracker.show`) that widens remote contact. The most significant dependency
gaps are the external tracker (Linear) coupling — captured explicitly by
predecessor 0257 but entirely absent from 0285's Dependencies despite a new
remote path — and the framing of 0257 as a non-blocker while 0285 cannot start
until 0257's code lands. Downstream Blocks are plausibly none, but the
same-engine integration watch-out 0257 flagged against 0229/0255 is not carried
forward.

**Strengths**:
- The predecessor coupling to 0257 is visible via the `relates_to` frontmatter,
  the References section, and repeated narrative acknowledgement, so the lineage
  is not hidden.
- The `tracker.show(external_id)` behavioural assumption is explicitly captured
  in Assumptions, naming the remote operation the targeted pull relies on.
- Blocks is reasoned about ('none identified') rather than left blank, and the
  full-sync preservation requirement bounds the change's reach.

**Findings**:
- 🟡 major (confidence: medium) — **External tracker coupling absent from
  Dependencies** (Dependencies). The work item introduces a new remote
  interaction — a by-id `tracker.show(external_id)` fetch to pull a remote-only
  issue where today's code aborts without contacting the remote — yet the
  Dependencies section records only 'Blocked by: none' and 'Blocks: none' and
  never names the remote tracker (Linear) as a coupling. Predecessor 0257
  explicitly captured this: 'External: the Linear tracker (via
  `work.integration`)… every… `external_id` lookup depends on the Linear API
  being reachable.' Impact: a targeted pull's success now depends on the remote
  tracker being reachable and on `tracker.show` resolving a single issue by id;
  omitting this hides a runtime dependency the equivalent predecessor made
  visible. Suggestion: add an 'External' line naming the remote tracker and the
  new by-id pull path's reachability dependency, mirroring 0257.
- 🟡 major (confidence: medium) — **0257 mischaracterised as a non-blocker**
  (Dependencies). Dependencies states 'Blocked by: none (0257 is in review)', but
  0285 directly extends 0257's just-shipped code — the exit-3 abort it inverts,
  the `resolve_targets` arm it modifies, the discovery suppression under
  `ItemSelection::Targeted`, and the pull-bound accounting all originate in 0257.
  This is a hard upstream ordering constraint: 0285 cannot begin until 0257 is
  merged, not merely 'in review'. Impact: characterising a strict predecessor as
  a non-blocker understates a real scheduling gate — if 0257 slips or changes in
  review, 0285's requirements are invalidated. Suggestion: record 0257 as an
  explicit upstream blocker ('Blocked by: 0257 until merged').
- 🔵 minor (confidence: medium) — **Integration watch-out not carried forward**
  (Dependencies). 0285 modifies the shared engine pull/discovery path (discovery
  suppression under `Targeted`, `resolve_targets`, pull-count accounting in
  `run.rs`), which is exactly the path 0257 flagged as an integration watch-out:
  '0229 (pull scope) and 0255 (chunk merge) touch the same engine pull/discovery
  path… whichever lands second should expect to integrate against the other's
  changes.' 0285 carries no equivalent note. Impact: two concurrent siblings
  touching the same code region create a merge coupling that, left unrecorded,
  risks conflicts at implementation time. Suggestion: carry forward 0257's
  integration watch-out.

### Scope

**Summary**: Work item 0285 is a well-bounded story that extends 0257's targeted
sync so a named remote-only item can be pulled, and reuses existing mechanisms
(create_from_remote, pull-bound accounting, standard conflict resolution) rather
than inventing new machinery — keeping it a single-team, single-increment unit
correctly parented under epic 0146. The one scope signal is that the title and
Summary explicitly name two capabilities — the remote-only pull and a
resolution/collision-semantics normalisation — though the work item makes a
defensible case that the second is a direct consequence of the first, since both
live in the same resolve_targets local-vs-remote decision. Sizing,
service-boundary confinement, and epic fit are all appropriate.

**Strengths**:
- Clear in-scope/out-of-scope boundaries: the targeted pull is scoped to a
  direct by-id fetch that never triggers full untracked-discovery, and full-sync
  report/write-set/watermark behaviour is explicitly preserved unchanged.
- Deliberately reuses existing paths (create_from_remote, the discovery-import id
  allocation, pull-bound accounting, standard conflict resolution) so the story
  adds no parallel mechanism and stays a single coherent increment.
- Correctly parented under epic 0146 as a sibling of 0257 that directly extends
  the targeted-sync feature — a coherent decomposition rather than a grab-bag
  addition.
- Confined to one engine (the work-cli sync resolution path) with no
  cross-service or cross-team ownership split; the remote tracker is an inherited
  external dependency, not new orchestration.

**Findings**:
- 🔵 suggestion (confidence: medium) — **Title and Summary name two
  capabilities** (Summary). The title and Summary name two capabilities — pulling
  a remote-only work item (the new feature) and 'resolution normalisation', which
  reworks the collision semantics from 0257's local-wins-with-warning into a hard
  exit-2 usage error and removes the suppressed-remote-match warning for ordinary
  synced items. The second is a behavioural change to already-shipped 0257
  behaviour that could, in principle, ship or roll back independently. Impact:
  bundling a new capability with a change to existing behaviour can blur what a
  reviewer signs off on and complicate rollback if only one half proves
  problematic. Suggestion: confirm the coupling is intentional — the work item
  argues persuasively that introducing remote lookup is exactly what forces the
  local-vs-remote decision to be redefined; if that shared-decision framing
  holds, keep them together but state explicitly in the Summary that the
  normalisation is a necessary consequence of the pull, not a separable second
  deliverable; otherwise consider splitting the collision-semantics change into
  its own item.

### Testability

**Summary**: The acceptance criteria for 0285 are largely strong from a
verification standpoint: most are framed as Given/When/Then with concrete exit
codes (2, 3), named example tokens (`PP-999`), explicit write-set assertions
('nothing written', 'zero writes'), a testable 'no remote call' constraint, and
a byte-identical comparison for full-sync preservation. Weaknesses cluster in one
vague criterion ('reconciled or created appropriately') and in gaps where
Requirements/Technical Notes describe behaviours (report/skill rendering,
watermark advance for the new item, real-run pull bounding) that no criterion
pins down.

**Strengths**:
- Most criteria specify an exact exit code and write-set outcome (exit 2 naming A
  and B with nothing written; exit 3 naming the token with zero writes), giving
  unambiguous pass/fail procedures.
- AC5's 'determined from the local corpus with no remote call' is a concrete,
  verifiable constraint (spy on the tracker) rather than a vague behavioural
  statement.
- AC2 makes the otherwise implementation-flavoured 'same path as a discovery
  import' testable by requiring the local id, filename, and baseline entry to
  match a full-sync discovery import for the same issue.
- AC8 uses a 'byte-identical to before this change' regression comparison for
  full-sync preservation — an objectively decidable check.

**Findings**:
- 🟡 major (confidence: high) — **'reconciled or created appropriately' is
  tautological** (Acceptance Criteria). The mixed-targets criterion uses
  'appropriately', which has no defined pass/fail outcome — any behaviour could
  be argued to satisfy it. Impact: a verifier cannot conclusively confirm the
  mixed-target run behaved correctly; the criterion adds no value beyond the
  per-shape criteria it summarises. Suggestion: replace 'appropriately' with the
  concrete expected disposition per shape.
- 🟡 major (confidence: high) — **Report-line and skill-rendering change has no
  acceptance criterion** (Requirements). The Requirements state 'Update the
  report lines and the `sync-work-items` skill rendering for the targeted
  pull-create and the revised collision semantics', but no criterion specifies
  the expected report/rendering output for either the pull-create or the exit-2
  collision. Impact: the visible output of the two central new behaviours is
  unverifiable, so this requirement could ship unmet. Suggestion: add a criterion
  asserting the specific report line(s) for a pull-create and for the exit-2
  collision, mirroring how 0257 pinned the discovery line to 'skipped'.
- 🟡 major (confidence: medium) — **Watermark advance for a targeted pull-create
  is not verified** (Acceptance Criteria). The Requirements and Technical Notes
  require the per-item watermark (`local_synced_at`) to extend to a newly-created
  targeted item, but no criterion verifies that a re-run after a targeted
  pull-create does not re-pull it (0257 had an explicit 'baseline has advanced' /
  'resumes without re-pushing' criterion; 0285 has none for the new item).
  Impact: resumability and baseline correctness for the new pull path has no
  test, so regressions (re-pulling on every run) would pass acceptance.
  Suggestion: add 'Given a completed targeted pull-create of PP-999, when sync
  runs again, then PP-999 is not re-pulled and its baseline/watermark has
  advanced.'
- 🔵 minor (confidence: medium) — **--max-pulls bounding only verified under
  --preview** (Acceptance Criteria). The requirement 'Count a targeted
  pull-create as a pull for blast-radius bounds (`--max-pulls`)' is only exercised
  by AC3, which asserts the count under `--preview` (a zero-write path). No
  criterion verifies bounding behaviour in a real (writing) run when targeted
  pull-creates exceed `--max-pulls`. Impact: the safety-relevant enforcement of
  the pull bound during actual writes is untested. Suggestion: add 'Given more
  targeted pull-creates than `--max-pulls N` allows, when sync runs, then at most
  N are created and the remainder are reported as bounded', matching 0257.
- 🔵 suggestion (confidence: low) — **'via the same path as a discovery import'
  phrasing leans on implementation** (Acceptance Criteria). AC1 describes the
  outcome as 'a local file is created from the remote issue via the same path as
  a discovery import', which references an internal code path rather than an
  observable outcome; the verifiable part is carried by AC2's field-equivalence
  comparison. Impact: on its own AC1 is only partly checkable, though AC2 largely
  compensates. Suggestion: reword AC1's outcome to the observable result and let
  AC2 carry the equivalence claim.

## Re-Review (Pass 2) — 2026-09-08

**Verdict:** COMMENT

Re-ran clarity, dependency, scope, and testability against the edited work item
(completeness had no findings and was not re-run). All six major findings from
pass 1 are resolved, along with every minor and suggestion. One new major
surfaced in clarity — the reconcile-vs-collision discriminator, the item's core
distinction, is worded inconsistently between Requirement 5 and AC4 — plus a
cluster of minor refinements on the newly-added criteria and relationship graph.
With one major and no criticals, the verdict drops from REVISE to COMMENT: the
work item is acceptable, and the remaining major is worth a quick pass.

### Previously Identified Issues

- 🟡 **Testability**: 'reconciled or created appropriately' is tautological —
  Resolved (mixed-targets AC now states per-shape dispositions)
- 🟡 **Testability**: Report-line and skill-rendering change has no acceptance
  criterion — Partially resolved (criterion added; exact expected text still
  open — see new minor)
- 🟡 **Clarity**: Local-precedence rule contradicts the collision-abort rule —
  Resolved (fourth Requirement carves out the collision exception)
- 🟡 **Dependency**: External tracker coupling absent from Dependencies —
  Resolved (External line names Linear via `work.integration`)
- 🟡 **Dependency**: 0257 mischaracterised as a non-blocker — Resolved, but
  over-corrected (0257 is already `done`/merged — see new minor)
- 🟡 **Testability**: Watermark advance not verified — Resolved (re-run
  baseline/watermark criterion added)
- 🔵 **Dependency**: Integration watch-out not carried forward — Resolved in
  prose (relationship-graph gap remains — see new minor)
- 🔵 **Testability**: `--max-pulls` bounding only verified under `--preview` —
  Resolved (live bounding criterion added)
- 🔵 **Clarity**: `--preview` / `--dry-run` relationship undefined — Resolved
  (`--dry-run` dropped)
- 🔵 **Clarity**: 'missed' vs 'explicitly scoped out' — Resolved (single
  deliberate-but-wrong framing)
- 🔵 **Scope**: Title and Summary name two capabilities — Resolved (Summary
  states the normalisation is a necessary consequence)
- 🔵 **Testability**: AC1 leans on implementation phrasing — Resolved (reworded
  to observable outcome)
- 🔵 **Clarity**: 'corpus' introduced without definition — Resolved (defined
  inline on first use)

### New Issues Introduced

- 🟡 **Clarity** (Requirements / Acceptance Criteria): the ordinary-synced
  reconcile case is characterised two ways — Requirement 5 as 'a token that
  resolves to the local file carrying the matching `external_id`', AC4 as 'local
  file A whose own `external_id` equals the token'. Read against AC5's collision
  wording, it is unclear whether the no-warning path triggers on any
  `external_id` match or only on a same-file id-and-`external_id` self-match. This
  is the exact discriminator between silent reconcile and exit-2 abort.
- 🔵 **Clarity** (Requirements): the condition deciding whether the remote is
  consulted is phrased three ways ('no local file', 'a path or local-id match
  wins', 'no local counterpart'); the 'local-id match wins' clause omits an
  `external_id` match, which also produces a local file and must suppress the
  remote lookup.
- 🔵 **Dependency** (Frontmatter: relates_to): the Integration watch-out names
  0229 and 0255 in prose, but `relates_to` lists only `work-item:0257`, so the
  concurrent code-path coupling is invisible to graph-traversing tooling.
- 🔵 **Dependency** (Dependencies): 'Blocked by: 0257 until merged' is stale —
  0257 is `status: done` and merged via PR #107, so the blocker is satisfied.
- 🔵 **Testability** (AC3): the preview 'counts against `--max-pulls`' outcome has
  no observable artefact, since preview writes nothing and names no bound.
- 🔵 **Testability** (AC8): the report-line/rendering criterion names no concrete
  expected text, so implementations could diverge and both claim to pass.
- 🔵 **Scope / Testability / Clarity** (low): pin the collision-change coupling
  explicitly; assert created-item field content; split compound criteria; name
  the 'both arms' antecedent.

### Assessment

The work item is now materially stronger and acceptable for planning. The single
remaining major — the reconcile-vs-collision discriminator wording — is worth
resolving before implementation, since it defines the item's central behavioural
branch; it needs a small semantic confirmation rather than a structural change.
The minor items (stale 0257 blocker, `relates_to` graph, AC3/AC8 concreteness)
are quick, unambiguous tidy-ups.

## Approval — 2026-09-08

**Verdict:** APPROVE

After pass 2, the reconcile-vs-collision discriminator (the one remaining major)
was aligned across Requirement 4, Requirement 5, and AC4 to the requester's
stated intent: a token reconciles when it resolves to a single local file — by
local id, its own `external_id`, or both — with exit-2 reserved for a token that
is one file's id and a *different* file's `external_id`. The stale 0257 blocker
and the `relates_to` graph gap were corrected. The work item is approved for
planning; the remaining suggestions (AC3/AC8 concreteness, created-item field
assertion, splitting compound criteria) are optional polish and non-blocking.

