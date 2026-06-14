---
type: work-item-review
id: "0102-remove-visualiser-legacy-linkage-fallback-arms-review-1"
title: "Work Item Review: Remove Visualiser-Server Legacy Linkage Fallback Arms (Follow-on Contract)"
date: "2026-06-15T20:56:49+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0057"
target: "work-item:0102"
work_item_id: "0102"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [migration, visualiser, frontmatter, cleanup, contract]
last_updated: "2026-06-15T21:22:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Remove Visualiser-Server Legacy Linkage Fallback Arms (Follow-on Contract)

**Verdict:** REVISE

This is an exceptionally well-developed work item — precise about which arms to
remove versus keep, dense across every expected section, and disciplined in
tracking the two-migration provenance (0070 and the older 0001). The verdict is
REVISE only because two *major* testability findings converge on the same soft
spot: the migration-completion gate that defines "safe to remove" is specified
against an unnamed "reference corpus" and is, by the work item's own admission,
a proxy for an unobservable condition (every userspace repo having migrated).
Resolving the gate's definition and proxy-sufficiency question — largely a
matter of promoting prose that already exists in the body into the Acceptance
Criteria and Dependencies — would clear the path to APPROVE.

### Cross-Cutting Themes

- **The migration-completion gate is the load-bearing weak point** (flagged by:
  testability, completeness, dependency, scope, clarity) — Four of five lenses
  circle the same artefact. Testability flags it as the most consequential
  criterion yet verifiable only against an unnamed proxy; completeness flags the
  unresolved Open Question that gates clean start; clarity notes the gate is
  referred to in three places (AC-1 inline grep, AC-2, AC-4) without confirming
  they are one artefact; dependency/scope note the gate now spans two migration
  guarantees. This cluster deserves the most attention.
- **The migration-0001 / `ticket:` precondition is buried in prose, not surfaced
  as a dependency** (flagged by: dependency, scope, testability) — The `ticket:`
  arm removal rests on migration `0001` having propagated, a guarantee distinct
  from and older than the 0070 condition `blocked_by` captures. This second
  precondition is described in Requirements/Open Questions but absent from the
  Dependencies section and `blocked_by`, where a scheduler would look.
- **Acceptance Criteria reach outside themselves to be verifiable** (flagged by:
  testability, clarity) — "the legacy keys" (AC-1) and "the reference corpus"
  (AC-2/AC-4) are not enumerated or bounded within the criteria, so a verifier
  must assemble the pass condition from Requirements/Technical Notes, leaving
  room for an incomplete check to be claimed as passing.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Migration-completion gate verified against an unnamed "reference corpus"
  **Location**: Acceptance Criteria
  AC-2 and AC-4 require the gate to "pass against the reference corpus", but no
  criterion names or bounds what that corpus is, nor pins the exact grep
  patterns or directory scope. A verifier could run against a different or
  partial corpus and still claim a pass.

- 🟡 **Testability**: Gate verifies a proxy, not the stated precondition
  **Location**: Open Questions
  The work item concedes the gate is a grep over *this* repo only, while the
  real condition — every consuming userspace repo has migrated — is
  unobservable here. The single most consequential criterion (safe-to-remove)
  is testable only against a stand-in the work item has not confirmed is
  sufficient.

#### Minor

- 🔵 **Clarity**: AC bullets 2 and 4 may describe the same gate or two different things
  **Location**: Acceptance Criteria
  Bullet 2 references "the corpus-wide gate" and bullet 4 "The
  migration-completion gate"; bullet 1 embeds a third grep. It is unclear
  whether these are one artefact stated repeatedly or distinct checks.

- 🔵 **Clarity**: Ambiguous referent "that condition" in Context's closing sentence
  **Location**: Context
  "This story removes them once that condition holds" has no single named
  antecedent in the preceding sentence; the intended condition lives in
  Dependencies, breaking local readability.

- 🔵 **Clarity**: "contract" overloads the term used elsewhere for linkage shapes / identity contract
  **Location**: Summary
  The expand/migrate/**contract** phase verb collides with the `contract` tag
  and ADR-0033 "identity contract"; a reader unfamiliar with the migration
  pattern could misread the core framing.

- 🔵 **Completeness**: Unresolved gate-as-proxy judgement may block clean start
  **Location**: Open Questions
  The sole Open Question — whether the grep is an accepted proxy or a
  release-gate/deprecation-window signal is also needed — gates the central exit
  condition and is self-described as "an unresolved judgement call worth
  confirming before removal".

- 🔵 **Dependency**: `ticket:` arm removal rests on migration 0001 propagation, not surfaced in Dependencies
  **Location**: Dependencies
  Requirements/Open Questions state the `ticket:` removal rests on migration
  `0001` having propagated — distinct from and older than 0070 — but the
  Dependencies section frames the whole blocking condition around 0070, so a
  scheduler reading only that section misses the second precondition.

- 🔵 **Testability**: AC-1's "the legacy keys" not enumerated within the criterion
  **Location**: Acceptance Criteria
  The precise set (`work-item:`, `ticket:`, the `work_item_id:` branch, the
  filename fallback) lives in Requirements/Technical Notes; an incomplete grep
  (e.g. omitting `ticket:`) could be claimed as passing.

- 🔵 **Testability**: No independent signal for migration-0001 propagation
  **Location**: Requirements
  Unlike the `work-item:` arms (which carry a deprecation warn), the `ticket:`
  arm carries no warn, so the `ticket:`/`ticket_id:` clause is verifiable only
  by absence-in-this-corpus with no corroborating signal that consumers applied
  migration 0001.

#### Suggestions

- 🔵 **Testability**: Grep targets pinned to symbols the work item also mandates renaming
  **Location**: Technical Notes
  AC-1's pass condition is tied to `parent_or_legacy_id`, which Requirements
  instruct the implementer to rename, so the literal grep target may not exist
  at verification time. Frame the check behaviourally (absence of any legacy
  resolution path) instead.

- 🔵 **Clarity**: "if still needed" leaves the fate of `work_item_id:` underspecified
  **Location**: Requirements
  No criterion specifies who decides "if still needed" or against what test, so
  two implementers could reach different outcomes. State the decision rule.

- 🔵 **Clarity**: Passive "migrated out by migration 0001" leaves actor/mechanism implicit
  **Location**: Requirements
  Where "propagated" first appears, link it to the migration-completion gate
  that observes it, so the safety argument resolves to a concrete check.

- 🔵 **Completeness**: Summary states the work but not the affected user/system
  **Location**: Summary
  For a story, optionally name the system served (the visualiser server / its
  maintainers, simplified to a single canonical resolution path).

- 🔵 **Dependency**: Parent epic 0057 closure dependency not stated as a Blocks coupling
  **Location**: Dependencies
  0057 is in-progress and this is the contract step that "cannot ossify"; if
  0102 gates 0057's closure, add a Blocks-style note — otherwise confirm the
  relationship is intentionally non-blocking.

- 🔵 **Scope**: `ticket:` removal rests on a separate migration guarantee not reflected in `blocked_by`
  **Location**: Requirements
  The owner consciously folded `ticket:` in (a coherent `read_ref_keys`-chain
  cleanup), but consider noting the migration-0001 propagation condition as a
  second gating dependency so the dual-guarantee nature is visible at the
  dependency level.

### Strengths

- ✅ The KEEP-vs-REMOVE distinction for the four legacy arms plus the typed
  `target:` arm is stated explicitly and repeated consistently across
  Requirements, Acceptance Criteria, and Technical Notes.
- ✅ Potential referent collisions are pre-empted directly in the text: the bare
  `warn!` vs `tracing::warn!` in `cluster_key.rs`, the unrelated
  shape-validation warn in `indexer.rs`, and the unlabelled "retained-plan
  block".
- ✅ Every expected section is present and substantively populated — no
  placeholders.
- ✅ Context thoroughly explains the why (the expand/migrate/contract split, why
  removal was deferred, why the parent-orphaning risk is already closed).
- ✅ The deliberately-retained `target:` ADR-0034 arm is documented as a
  non-removal with its independent consumer (`target_path_from_entry`),
  pre-empting a coupling the implementer might otherwise sever.
- ✅ Frontmatter is complete and consistent with the body (kind, status,
  priority, parent, `blocked_by`, relationship fields).
- ✅ AC-1 and AC-3 are concretely verifiable (definitive grep; named runnable
  task `mise run test:unit:visualiser` in both feature modes with the exact
  test-population change stated).

### Recommended Changes

1. **Pin and consolidate the migration-completion gate in the Acceptance
   Criteria** (addresses: "verified against an unnamed reference corpus",
   "AC bullets 2 and 4 may describe the same gate", "AC-1's legacy keys not
   enumerated") — In AC-2/AC-4 state the corpus explicitly (a recursive grep
   over this repo's `meta/`) and inline the exact patterns each clause must
   match zero of (`work-item:`/`work_item_id:` own-identity shapes;
   `ticket:`/`ticket_id:` shapes; plus `parent_or_legacy_id` and the filename
   fallback for AC-1). Confirm in the text whether AC-1's inline grep, AC-2, and
   AC-4 are one gate with multiple clauses or separate checks.

2. **Resolve the gate-as-proxy Open Question before implementation**
   (addresses: "Gate verifies a proxy, not the stated precondition",
   "Unresolved gate-as-proxy judgement may block clean start", "No independent
   signal for migration-0001 propagation") — Either accept the reference-corpus
   grep as the definitive gate and say so in the AC (moving the statement out of
   Open Questions into Requirements/Dependencies), or add a concrete second
   signal (e.g. a deprecation-window / minimum-release-count gate). This is the
   change that gates the REVISE→APPROVE transition.

3. **Surface the migration-0001 / `ticket:` precondition in Dependencies**
   (addresses: "`ticket:` arm removal rests on migration 0001 propagation, not
   surfaced", "`ticket:` removal rests on a separate migration guarantee") — Add
   a Dependencies bullet naming migration `0001` propagation as a second
   precondition for the `ticket:`/`ticket_id:` arm and gate clause, mirroring
   how the existing bullet frames 0070's coupling.

4. **Frame AC-1's grep behaviourally rather than by symbol name** (addresses:
   "Grep targets pinned to symbols the work item also mandates renaming") —
   Express AC-1 as the absence of any legacy resolution path, so the check
   survives the `parent_or_legacy_id` rename the same work item mandates.

5. **Tighten residual clarity referents** (addresses: "Ambiguous referent 'that
   condition'", "'contract' overloads the term", "'if still needed' leaves
   `work_item_id:` underspecified") — Replace "that condition" in Context with
   an explicit restatement; gloss "contract" on first use; state the decision
   rule for retaining `work_item_id:`.

6. **(Optional) Clarify 0057 closure coupling and Summary beneficiary**
   (addresses: "Parent epic 0057 closure dependency not stated", "Summary states
   the work but not the affected user/system") — Note whether 0102 gates 0057's
   closure, and name the system served in the Summary.

## Per-Lens Results

### Clarity

**Summary**: Unusually precise for its complexity: names the exact arms to
remove, the exact arm to keep, and disambiguates several near-miss collisions
(bare `warn!` vs `tracing::warn!`, the retained-plan block, the typed `target:`
arm). A small number of pronoun/referent ambiguities and one heavily-loaded term
("contract") could trip a reader who has not internalised the surrounding
migration vocabulary.

**Strengths**:
- The KEEP-vs-REMOVE distinction is stated explicitly and repeated consistently
  across Requirements, Acceptance Criteria, and Technical Notes.
- Potential referent collisions (bare `warn!`, the unrelated shape-validation
  warn, the unlabelled "retained-plan block") are each called out so the
  implementer cannot conflate them.
- The two-migration provenance is tracked consistently wherever the gate is
  discussed.

**Findings**:
- 🔵 minor (medium) — **Acceptance Criteria**: AC bullets 1 and 4 may describe
  the same gate or two different things. Bullet 2 "the corpus-wide gate", bullet
  4 "The migration-completion gate", and bullet 1's inline grep make it unclear
  whether to build one gate or two. *Suggestion*: state explicitly that bullets
  2 and 4 refer to a single gate (with two clauses), or distinguish them.
- 🔵 minor (medium) — **Summary**: "contract" / "contract half" overloads the
  term used elsewhere (the `contract` tag, ADR-0033 "identity contract"). A
  reader who knows "contract" only as "interface agreement" could misread the
  framing. *Suggestion*: gloss "contract" as the third phase of
  expand/migrate/contract on first use.
- 🔵 minor (medium) — **Context**: ambiguous referent — "this story removes them
  once that condition holds" has no single named antecedent in the preceding
  sentence. *Suggestion*: replace "that condition" with an explicit restatement.
- 🔵 suggestion (low) — **Requirements**: "if still needed" leaves the fate of
  `work_item_id:` underspecified; no criterion specifies who decides or against
  what test. *Suggestion*: state the decision rule for retaining `work_item_id:`.
- 🔵 suggestion (low) — **Requirements**: passive "migrated out by migration
  0001" / "having propagated" leaves the actor/mechanism implicit until
  Dependencies/Open Questions. *Suggestion*: link "propagated" to the gate that
  observes it where it first appears.

### Completeness

**Summary**: An exceptionally complete work item. All expected sections are
present and densely populated, and the story-kind requirements (motivation,
who/what is served, done-defining criteria) are satisfied. Frontmatter is intact
with a recognised kind and appropriate status. The only observations are minor:
the Summary identifies the work but not explicitly the affected user/system, and
an Open Question flags a genuinely unresolved judgement.

**Strengths**:
- Every expected section is present and substantively populated rather than
  placeholders.
- Context thoroughly explains the why.
- Acceptance Criteria contains four specific, done-defining bullets mapping
  directly to the four Requirements, including the gate as an explicit exit
  condition.
- Frontmatter is complete and consistent with the body.
- Dependencies and References capture provenance (0070, 0057, 0064, ADR-0034,
  ADR-0033, migration 0001).

**Findings**:
- 🔵 minor (medium) — **Open Questions**: unresolved gate-as-proxy judgement may
  block clean start; it gates the central exit condition and is self-described
  as "an unresolved judgement call worth confirming before removal".
  *Suggestion*: resolve before implementation — confirm the grep is the accepted
  proxy (and promote it out of Open Questions) or specify the additional signal.
- 🔵 suggestion (low) — **Summary**: states the work but not the affected
  user/system; the beneficiary (the visualiser server / its maintainers) must be
  inferred from Context. *Suggestion*: add a half-sentence naming the system
  served.

### Dependency

**Summary**: Unusually well-dependency-mapped: the single upstream blocker
(0070) is captured in both frontmatter and the Dependencies section, the
cross-repo migrate-on-use coupling is explicitly named as gating, and the
retained `target:` arm is documented as a non-dependency. The one genuine gap is
that the `ticket:` arm removal rests on a second, distinct upstream precondition
— migration `0001` having propagated — which is described elsewhere but not
surfaced in Dependencies as a coupling on par with 0070. No uncaptured
downstream consumers or external-system couplings of concern.

**Strengths**:
- The sole upstream blocker (0070) is captured consistently in frontmatter and
  explained in Dependencies, including *why* it blocks.
- The cross-repo / cross-consumer coupling is explicitly named, with the gate
  identified as the observable proxy.
- The retained `target:` ADR-0034 arm is documented with its independent
  consumer (`target_path_from_entry`).
- 0064 and migration `0001` are named in References, so each arm's provenance is
  traceable.

**Findings**:
- 🔵 minor (high) — **Dependencies**: `ticket:` arm removal rests on migration
  0001 propagation, not surfaced in Dependencies. A reader scanning only that
  section sees a single gate (0070) and could schedule the `ticket:` removal on
  0070's status alone. *Suggestion*: add a Dependencies bullet naming migration
  `0001` propagation as a second precondition.
- 🔵 suggestion (medium) — **Dependencies**: parent epic 0057 closure dependency
  on this contract step not stated as a Blocks coupling; 0057 is in-progress and
  this is the step that "cannot ossify". *Suggestion*: if 0102 gates 0057's
  closure, add a Blocks-style note; otherwise confirm it is intentionally
  non-blocking.

### Scope

**Summary**: A well-bounded story: the contract half of a deliberate
expand/migrate/contract split, removing four deprecated legacy linkage fallback
arms (plus their pinning tests) and adding a migration-completion gate. All
requirements serve one unified purpose within a single component (the visualiser
Rust server), and parent 0070 explicitly named this as its follow-on
deliverable. The only scope-adjacent wrinkle is that the in-scope `ticket:` arm
removal rests on a different (older) migration-propagation guarantee than the
rest of the story, which the owner consciously folded in.

**Strengths**:
- Single coherent purpose: every requirement serves the one deliverable of
  contracting the deprecated legacy linkage fallback arms.
- Clear in-scope/out-of-scope boundary: the typed `target:` arm is explicitly
  carved out as load-bearing and retained.
- Confined to a single component and team (the visualiser Rust server).
- Story kind is appropriate — a focused, indivisible removal correctly split out
  from the XL parent 0070.

**Findings**:
- 🔵 suggestion (medium) — **Requirements**: `ticket:` arm removal rests on a
  separate migration guarantee (migration `0001`) not reflected in `blocked_by`,
  which lists only 0070. The completion gate spans two distinct guarantees.
  *Suggestion*: confirm the bundling is intended (Drafting Notes say it was an
  explicit owner decision) and consider noting the migration-0001 condition as a
  second gating dependency so the dual-guarantee unit of work is visible at the
  dependency level.

### Testability

**Summary**: The Acceptance Criteria are largely testable — most resolve to
grep-based assertions over named source files/functions and a named test-suite
task. The principal gap is the migration-completion gate (AC-2/AC-4): it is
specified against "the reference corpus" without that corpus being named or
bounded, and the work item's own Open Questions concede the gate may be only a
proxy for the real condition (all userspace repos migrated) — leaving the most
consequential criterion verifiable only against a stand-in. Several criteria also
pin verification to line refs and grep targets the item itself flags as needing
re-derivation.

**Strengths**:
- AC-1 is verifiable by a concrete, definitive procedure (a grep returning
  nothing, plus an explicit retained-symbol assertion that `target:` stays).
- AC-3 binds completeness to a named, runnable check (`mise run
  test:unit:visualiser`, both feature modes) and states the exact test-population
  change.
- Requirements and Technical Notes enumerate every arm, test, and survivor by
  file and function name.

**Findings**:
- 🟡 major (high) — **Acceptance Criteria**: AC-2/AC-4 require the gate to "pass
  against the reference corpus", but no criterion names or bounds that corpus,
  the exact grep patterns, or the directory scope. A verifier could run against a
  different or partial corpus and still claim a pass. *Suggestion*: state the
  corpus explicitly (a recursive grep over this repo's `meta/`) and list the
  exact patterns the two clauses must match zero of.
- 🟡 major (high) — **Open Questions**: the gate verifies a stand-in, not the
  stated precondition. A green gate does not conclusively establish that removal
  is safe across external repos. *Suggestion*: resolve the open question — accept
  the reference-corpus grep as definitive (and say so in the AC), or add a
  concrete second signal (deprecation-window / release-gate artefact).
- 🔵 minor (medium) — **Acceptance Criteria**: AC-1's "the legacy keys" is not
  enumerated within the criterion; the set lives in Requirements/Technical Notes,
  leaving room for an incomplete grep to be claimed as passing. *Suggestion*:
  inline the explicit list of grep targets into AC-1.
- 🔵 minor (medium) — **Requirements**: no independent signal corroborates
  migration-0001 propagation; unlike the `work-item:` arms, the `ticket:` arm
  carries no deprecation warn, so the clause can pass while its precondition is
  unverified for external repos. *Suggestion*: state whether the corpus grep is
  the sole accepted proxy, or add a corroborating observable.
- 🔵 suggestion (low) — **Technical Notes**: verification targets are pinned to
  source coordinates the item flags as needing re-derivation; AC-1's pass
  condition is tied to `parent_or_legacy_id`, which Requirements instruct the
  implementer to rename. *Suggestion*: frame AC-1's grep clause behaviourally
  (absence of any legacy resolution path) so it survives the rename.

---

## Re-Review (Pass 2) — 2026-06-15T21:14:04+00:00

**Verdict:** COMMENT

The revision resolved **both major findings** and every minor/suggestion raised
in the first pass. No major or critical findings remain, so the verdict moves
REVISE → COMMENT: the work item is acceptable for implementation as-is. The new
findings are all minor/suggestion — most are latent issues the deeper second
pass surfaced (independent of this revision), plus two genuinely adjacent to the
edits (the prose "Blocks: 0057" not mirrored in frontmatter, and the gate grep's
"own-identity shape" qualifier not reduced to an exact pattern).

### Previously Identified Issues

- 🟡 **Testability**: Gate verified against an unnamed "reference corpus" — **Resolved.** AC-2 now names the corpus (a recursive grep over this repo's `meta/`) and enumerates the two clauses' patterns.
- 🟡 **Testability**: Gate verifies a proxy, not the stated precondition — **Resolved.** Open Questions now records the reference-corpus grep as the *accepted, definitive proxy*; no release-gate signal required. Restated in Requirements and AC-2.
- 🔵 **Clarity**: AC bullets 2 and 4 may describe the same gate or two — **Resolved.** Consolidated into a single AC describing one gate with two clauses.
- 🔵 **Clarity**: Ambiguous referent "that condition" in Context — **Resolved.** Replaced with an explicit restatement linking to the gate.
- 🔵 **Clarity**: "contract" term overload — **Resolved.** Glossed inline in the Summary as the pattern's third phase, "not an interface agreement".
- 🔵 **Completeness**: Unresolved gate-as-proxy Open Question may block start — **Resolved.** Open Questions now reads "None outstanding" with the decision recorded.
- 🔵 **Dependency**: migration-0001 precondition absent from Dependencies — **Resolved.** Added as an explicit second precondition for the `ticket:` arm.
- 🔵 **Testability**: AC-1 "legacy keys" not enumerated — **Resolved.** AC-1 now lists the exact grep targets inline.
- 🔵 **Testability**: No independent signal for migration-0001 propagation — **Resolved.** The corpus grep (clause b) is stated as the sole accepted proxy.
- 🔵 **Testability**: Grep pinned to a symbol the item mandates renaming — **Resolved.** AC-1 reframed behaviourally ("absence of any legacy resolution path").
- 🔵 **Clarity**: "if still needed" underspecifies `work_item_id:` fate — **Resolved.** A decision rule (default to removal) was added to Requirements.
- 🔵 **Clarity**: Passive "propagated" leaves mechanism implicit — **Resolved.** "Propagated" is now linked to the observing gate in Context and Requirements.
- 🔵 **Completeness**: Summary states the work but not the beneficiary — **Partially resolved.** Summary now names "its maintainers"; re-review still suggests making the beneficiary first-class (downgraded to suggestion).
- 🔵 **Dependency**: 0057 closure not stated as a Blocks coupling — **Resolved (in prose).** Dependencies now states "Blocks: 0057 closure"; re-review notes this is not yet mirrored in structured frontmatter (see new issues).
- 🔵 **Scope**: `ticket:` rests on a separate guarantee not at dependency level — **Partially resolved.** Now described in Dependencies; re-review still suggests a symmetric structured marker (downgraded to suggestion).

### New Issues Introduced

All minor/suggestion — none force REVISE. The first three are latent (pre-existing, surfaced by the deeper second pass); the last two are adjacent to the revision edits.

- 🔵 **Clarity** (minor): Summary attributes all "now-deprecated fallback arms" to 0070's deprecate step, but the `ticket:` arm predates 0070 and carries no deprecation warn — a Summary/body lineage mismatch.
- 🔵 **Clarity** (minor): `work_item_id:` denotes several distinct roles (own-identity, foreign, indexer identity branch, gate clause) without re-disambiguation at each use.
- 🔵 **Clarity** (minor): the parenthetical "the 0070 AC-12/AC-13 deferred here" references ordinal criterion numbers that 0070 presents as an unnumbered checkbox list.
- 🔵 **Clarity / Testability** (minor): "both feature modes" (AC-3) is asserted without naming the two cargo features (`embed-dist` / `dev-frontend`), so a verifier could run only the default mode.
- 🔵 **Dependency** (minor): the prose "Blocks: 0057" and the migration-0001 precondition are not mirrored in structured frontmatter (`blocks:` absent; 0001 not in `relates_to`), so the edges are invisible to tooling that reads frontmatter.
- 🔵 **Testability** (minor): AC-2's "own-identity shapes" qualifier is not reduced to an exact grep pattern, so two verifiers could write different regexes and reach opposite verdicts.
- 🔵 **Testability** (minor): the `work_item_id:` retain-vs-remove decision rule has no AC covering the *retained* branch (no check that a retained reader is genuinely non-legacy).

### Assessment

The work item is **ready for implementation**. The substantive blockers from pass
1 — the under-specified, proxy-uncertain migration-completion gate — are fully
resolved, and the gate is now a concrete, definitive, two-clause grep over a
named corpus. The remaining findings are polish: tightening the Summary's arm
lineage, disambiguating `work_item_id:` roles at point of use, naming the two
feature modes, pinning the gate's exact regex, and mirroring two prose couplings
(`blocks: 0057`, migration-0001) into structured frontmatter. None block
implementation; they are worth a quick pass if convenient.

---

## Approval — 2026-06-15T21:22:42+00:00

**Verdict:** APPROVE (manually confirmed by reviewer)

Following the pass-2 COMMENT verdict, the reviewer applied the two structural
follow-ups and the clarity polish from the new-issues list:

- **Frontmatter** — added `blocks: ["work-item:0057"]`, mirroring the prose
  downstream edge so tooling can see it.
- **AC-2** — pinned the gate grep to a reproducible pattern anchored to the
  own-identity frontmatter key at line start, with MATCH / NO-MATCH examples
  that exclude canonical typed references.
- **Summary** — corrected the arm-lineage attribution (the set spans both the
  0070-deprecated arms and the older `ticket:` arm gated on migration `0001`)
  and made the beneficiary first-class.
- **Requirements** — added a `work_item_id:` role-disambiguation note.
- **AC-1** — replaced the uncited "AC-12/AC-13" ordinal with a content
  reference.
- **AC-3** — named the two feature modes (`embed-dist`, `dev-frontend`).

Two minor findings were consciously left as-is (migration-`0001` not in
`relates_to` — no work-item artifact exists for it, clause-b is the observable
proxy; and no AC for the retained branch of the `work_item_id:` decision rule —
the rule defaults to removal). Neither blocks implementation. The work item is
**approved for implementation**.

---
*Review generated by /accelerator:review-work-item*
