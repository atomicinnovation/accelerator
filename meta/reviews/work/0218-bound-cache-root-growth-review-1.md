---
type: "work-item-review"
id: "0218-bound-cache-root-growth-review-1"
title: "Work Item Review: Bound a shared cache root's growth"
date: "2026-10-15T10:13:38+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0218"
work_item_id: "0218"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 6
tags: ["cli", "launcher", "cache"]
last_updated: "2026-10-15T22:15:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Bound a shared cache root's growth

**Verdict:** REVISE

The item is structurally complete and unusually precise: entry shapes are
literal patterns, every requirement maps to a Given/When/Then criterion, and
accepted risks are recorded explicitly. It needs revision because six major
findings remain: an unpassable performance criterion, an unrecorded dependency
on the measurement harness and on 0334, an overloaded "prune" term that leaves
the verb's lock behaviour undefined, no criterion protecting the plugin's own
non-removable names, and an unexplained divergence from ADR-0063 over the
dev-checkout root and automatic prune.

### Cross-Cutting Themes

- **Divergence from ADR-0063** (flagged by: completeness, scope, dependency,
  clarity) — ADR-0063 makes `accelerator cache prune` a user-invoked verb that
  owns both the override and dev-checkout roots. This item makes prune
  automatic after a cold-path `store` and excludes dev-checkout roots, but
  neither requires an ADR amendment nor explains the departure outside
  Drafting Notes.
- **"Prune" is overloaded** (flagged by: clarity, testability) — the automatic
  single-file prune, the `accelerator cache prune` verb and the tree reap share
  one word, so the lock rule, the `trees/` exclusion and the verb's
  default-root behaviour are each ambiguous and untested.
- **Allowlist versus "names the plugin produces"** (flagged by: clarity,
  testability) — the plugin also produces the lock directory, probe files, log
  and `trees/`, which must survive; no criterion asserts that.
- **Warm-dispatch measurement** (flagged by: testability, dependency) — the
  interval cell is unnamed and ~0.12 ms wide; the harness may not accept an
  override root or tolerate claim/prune churn in its witness; the "before"
  baseline must be captured before implementation starts.

### Findings

#### Major

- 🟡 **Testability**: Warm-dispatch performance criterion names no interval
  cell and its threshold is too narrow to pass reliably
  **Location**: Acceptance Criteria
  The harness reports C1–C6 and ratio intervals; C1 in
  `warm-dispatch-3.json` spans ~0.12 ms, so run-to-run noise alone can fail a
  correct implementation.
- 🟡 **Dependency**: Measurement harness from 0189 is relied on and affected
  but not captured; "before" ordering unstated
  **Location**: Acceptance Criteria, Dependencies
  The harness checks `accelerator_env_keys` as a precondition and witnesses the
  root's entry set, which claims and prune now change; the existing baseline
  was taken without an override, so a new "before" must precede any code.
- 🟡 **Clarity / Testability**: "Prune" refers to three operations; verb
  behaviour only partly specified
  **Location**: Requirements, Technical Notes
  Whether the verb blocks, skips or bypasses the root lock, whether the
  cold-path prune touches trees, and what the verb does against
  `${ACCELERATOR_PLUGIN_ROOT}/bin` are all open; "never touches `trees/`"
  contradicts the verb covering trees.
- 🟡 **Testability / Clarity**: Protected plugin-owned names have no criterion;
  "names the plugin produces" is broader than the allowlist
  **Location**: Acceptance Criteria, Requirements
  An over-broad prune deleting `.accelerator-lock-<platform>/` or `trees/`
  passes every listed criterion.
- 🟡 **Clarity / Scope**: Dev-checkout root is described as unbounded, then
  silently excluded, with no owning follow-up
  **Location**: Context, Drafting Notes
  The motivating 0189 evidence comes from the excluded root, and ADR-0063
  assigns that root to `prune`; the exclusion surfaces only in Drafting Notes.
- 🟡 **Dependency**: 0334 is a de facto prerequisite but recorded only as
  "Relates to"
  **Location**: Dependencies
  This item lifts `AGE_BACKSTOP` and extends the prune verb that 0334 changed;
  0334 still reads `ready` and lists "Blocks: none".

#### Minor

- 🔵 **Completeness / Scope / Dependency**: Automatic cold-path prune changes
  an ADR-0063 decision without ADR work in scope
  **Location**: Requirements, Dependencies
  ADR-0063 says prune is "a user-invoked verb, not a scheduled collector";
  nothing requires amending or superseding it.
- 🔵 **Scope**: Declared kind `task` understates a multi-mechanism change
  **Location**: Frontmatter: kind
  Claims, single-file prune, shim prune, a new lock, `ENOENT` re-resolve, a
  verb extension and a measurement span warm path, cold path and CLI.
- 🔵 **Scope**: `ENOENT` re-resolve is a separable safety increment
  **Location**: Requirements
  It is testable without prune and could land first.
- 🔵 **Dependency**: `ENOENT` recovery protects only launchers that include
  this change
  **Location**: Assumptions
  A pre-upgrade launcher caught between `find` and `exec` fails outright rather
  than re-fetching; 0334 recorded the equivalent rollout constraint.
- 🔵 **Testability**: Claim throttling and ignored foreign/symlinked claims are
  not verified
  **Location**: Requirements
  "Refreshed (or found fresh)" passes whether or not throttling works.
- 🔵 **Testability**: "Never absent" in the corrupt-entry criterion has no
  observation procedure
  **Location**: Acceptance Criteria
  It asserts a negative over a time window without saying what observes a gap.
- 🔵 **Clarity**: Removal of old `.tmp-` staging entries is only implied by
  the requirement
  **Location**: Requirements
  Stated negatively, unlike the parallel shim-residue rule.
- 🔵 **Clarity**: "Override root" and "shared root" used interchangeably
  **Location**: Requirements
  The gate is "root is not `${ACCELERATOR_PLUGIN_ROOT}/bin`", so a
  single-version override is in scope; the text does not say so.

#### Suggestions

- 🔵 **Testability**: Concurrent store safety and prune mutual exclusion are
  not exercised
  **Location**: Acceptance Criteria
- 🔵 **Scope**: Corrupt-entry replacement reads as new scope rather than a
  preserved invariant
  **Location**: Requirements
- 🔵 **Clarity**: Integrity witness, shim and 0334's "lease" used without
  definition
  **Location**: Context, Dependencies
- 🔵 **Completeness**: Status still `draft` although no open questions remain
  **Location**: Frontmatter: status

### Strengths

- ✅ Scope deliberately narrowed to the override root, with the reasoning
  (0223, orphan sweep) recorded.
- ✅ Every requirement maps to at least one Given/When/Then criterion,
  including negative cases.
- ✅ Entry shapes are literal patterns, giving precise fixtures, including
  adversarial look-alikes.
- ✅ `ENOENT` recovery is bounded and testable at both edges.
- ✅ Reuses `CLAIM_WINDOW`, `AGE_BACKSTOP` and the tree-claim model rather than
  inventing a new policy.
- ✅ Accepted risks are tied to concrete race windows in Assumptions; rejected
  alternatives are recorded in Drafting Notes.
- ✅ Measurement figures cited in Context match `warm-dispatch-3.json`.

### Recommended Changes

1. **Make the performance criterion passable and its harness dependency
   explicit** (addresses: performance criterion; measurement harness)
   Name the cell and statistic (e.g. C1 median) and a pass rule (overlapping
   intervals or delta ≤ an explicit threshold); state whether the harness
   accepts an override root and tolerates claim/prune churn, adding harness
   work to scope or as a blocker; require the "before" capture first.
2. **Split "prune" into named operations** (addresses: overloaded prune; verb
   behaviour)
   Distinguish "single-file prune" from "the prune verb"; state the verb's
   lock behaviour, that the cold-path trigger runs single-file prune only, and
   that the verb leaves the default root's single-file entries untouched; add
   criteria for each.
3. **State the removable allowlist and protect the rest** (addresses:
   protected names; names the plugin produces; `.tmp-` removal)
   Replace "names the plugin produces" with the named removable shapes, state
   positively that old `.tmp-` entries are removed, and add a criterion that
   the lock directory, probe files, log, `trees/` and unknown-platform launcher
   names survive.
4. **Record the ADR-0063 position and the dev-checkout exclusion**
   (addresses: dev-checkout root; automatic prune vs ADR-0063)
   Say in Summary/Context that dev-checkout roots stay unbounded and why; add a
   requirement or Dependencies entry for an ADR amending ADR-0063, or state why
   none is needed; reference or create a follow-up for dev-checkout growth.
5. **Make 0334 a blocker and add a rollout note** (addresses: 0334
   prerequisite; `ENOENT` rollout coupling)
   Record 0334 as "Blocked by" (complete in substance) and reconcile its
   status; note that safe concurrent dispatch holds only once every launcher on
   the root includes this change.
6. **Tighten the remaining criteria** (addresses: throttling; corrupt entry;
   concurrent store; override vs shared root)
   Add throttle and symlinked/foreign-claim criteria, make the corrupt-entry
   observation concrete and phrase it as a preserved invariant, add a
   concurrent-store-survives-prune criterion, and define "override root" once.
7. **Consider sequencing or re-kinding** (addresses: kind `task`; separable
   `ENOENT` re-resolve)
   Either file as a story or record an increment order (re-resolve → claims →
   prune → auto-prune on store).

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Precise and well argued, with concrete entry shapes, constants and
failure modes. The main risks are terms whose referent shifts: "prune" covers
three operations, the dev-checkout root is called unbounded yet excluded
against ADR-0063, and "names the plugin produces" is used as an allowlist
despite protected plugin-produced names.

**Strengths**:
- Entry shapes are literal patterns.
- Acceptance criteria name the trigger, actor and observable outcome.
- Accepted risks sit in Assumptions, each tied to a race window.
- Technical Notes define throttling, claim keying and why claims are
  per-version.

**Findings**:
- 🟡 major (high) — Requirements — "Prune" refers to three different
  operations. The lock rule leaves the verb's behaviour open; "never touches
  `trees/`" contradicts the verb covering trees; the cold-path scope is
  unclear. Name the operations separately and specify the verb's lock
  behaviour.
- 🟡 major (medium) — Context — Dev-checkout root is described as unbounded,
  yet silently excluded, contradicting ADR-0063, which assigns it to `prune`.
  State the exclusion and its reason in Summary/Context.
- 🔵 minor (high) — Requirements — "Names the plugin produces" is broader than
  the allowlist; lock, probe, log and `trees/` are plugin-produced but
  protected.
- 🔵 minor (medium) — Requirements — Removal of old `.tmp-` entries is only
  implied; state it positively like the shim-residue rule.
- 🔵 minor (medium) — Requirements — "Override root" and "shared root" are used
  interchangeably; define "override root" once.
- 🔵 suggestion (medium) — Context — Integrity witness, shim and 0334's "lease"
  are used without definition or link.

### Completeness

**Summary**: Structurally complete; every section present and substantive,
frontmatter consistent with the body. The only gap is no provision for
updating the decision record now that prune becomes automatic.

**Strengths**:
- Summary names its scope boundary and the reason.
- Context explains the forces, including the "keep newest N" hazard.
- Every Requirement has a matching criterion, including negative cases.
- Assumptions list each accepted risk.
- Open Questions explicitly "None remaining"; rejected alternatives recorded.
- References are actionable; cited figures match the measurement file.

**Findings**:
- 🔵 minor (medium) — Requirements — No requirement to amend ADR-0063 for the
  shift from user-invoked to automatic prune.
- 🔵 suggestion (low) — Frontmatter: status — Status reads `draft` though no
  open questions remain.

### Dependency

**Summary**: Code-level couplings are mapped unusually well. Gaps are in
Dependencies: 0334 is a prerequisite recorded only as "Relates to"; the 0189
harness is both relied on and affected; before-measurement ordering, rollout
scope of `ENOENT` recovery and the ADR-0063 position are unrecorded.

**Strengths**:
- Each "Relates to" entry carries its reason.
- The 0334 status discrepancy is surfaced in Drafting Notes.
- The shell-bootstrap coupling is recorded and reasoned through.
- The orphan-sweep dependency is traced to 0223 and ADR-0063.
- Cross-version rollout and multi-user cases are in Assumptions.

**Findings**:
- 🟡 major (high) — Dependencies — 0334 is a de facto prerequisite (shared
  `AGE_BACKSTOP` lift, same prune verb) but listed as "Relates to"; 0334 says
  "Blocks: none". Record as "Blocked by" and reconcile.
- 🟡 major (medium) — Acceptance Criteria — The measurement harness treats
  `accelerator_env_keys` as a precondition and witnesses the entry set this
  item mutates. State whether it supports an override root and tolerates
  churn, or add the harness change.
- 🔵 minor (medium) — Acceptance Criteria — The override-root "before"
  measurement must be captured before implementation; the existing baseline
  cannot serve.
- 🔵 minor (medium) — Assumptions — `ENOENT` recovery protects only launchers
  with this change; pre-upgrade launchers fail an in-flight dispatch. Add a
  rollout note.
- 🔵 minor (low) — Dependencies — Decision coupling to ADR-0063 not recorded;
  add a "Decisions" line.

### Scope

**Summary**: Clear goal and explicit boundary, with requirements serving it.
Heavier than the `task` kind suggests; the `ENOENT` re-resolve could ship
first; the dev-checkout root is left unowned; the automatic prune changes an
ADR-0063 decision without ADR work in scope.

**Strengths**:
- Deliberate narrowing to the override root, with recorded reasoning.
- Tight requirement-to-criterion alignment.
- Reuses the existing retention policy rather than designing a new one.
- Accepted risks recorded rather than absorbed into scope.

**Findings**:
- 🔵 minor (medium) — Frontmatter: kind — `task` understates a
  multi-mechanism change; re-file as story or record increments.
- 🔵 minor (medium) — Requirements — The `ENOENT` re-resolve is a separable
  safety increment; split it or sequence it first.
- 🔵 minor (high) — Drafting Notes — The excluded dev-checkout root has no
  owning item, though it is the cited evidence.
- 🔵 minor (medium) — Requirements — Automatic cold-path prune changes an
  ADR-0063 decision without ADR work in scope.
- 🔵 suggestion (low) — Requirements — Corrupt-entry replacement reads as new
  scope; phrase it as a preserved invariant.

### Testability

**Summary**: Strong overall; near-every requirement has a concrete
Given/When/Then criterion and fixture-ready entry shapes. Weak spots: the
performance criterion's undefined, very narrow interval; no criteria for
protected names, throttling, the verb's default-root and lock behaviour; and
an unobservable "never absent".

**Strengths**:
- Consistent precondition/action/outcome with positive and negative cases.
- Exact entry shapes, including adversarial look-alikes.
- `ENOENT` recovery testable at both edges.
- Named constants instead of vague thresholds.

**Findings**:
- 🟡 major (high) — Acceptance Criteria — Performance criterion names no cell;
  C1's interval is ~0.12 ms (35.47–35.58 ms). Name the statistic and define an
  explicit pass rule and recording location.
- 🟡 major (high) — Acceptance Criteria — No criterion protects
  `.accelerator-lock-<platform>/`, `.accelerator-probe-*`,
  `.accelerator-unverified.log`, `trees/` or unknown-platform launcher names.
- 🔵 minor (medium) — Acceptance Criteria — "Never absent" has no observation
  procedure.
- 🔵 minor (medium) — Requirements — Throttling and ignored foreign/symlinked
  claims unverified.
- 🔵 minor (medium) — Acceptance Criteria — The plain verb, its default-root
  behaviour and its lock contention behaviour lack criteria.
- 🔵 suggestion (low) — Acceptance Criteria — Concurrent store during prune and
  prune mutual exclusion not exercised.

## Re-Review (Pass 2) — 2026-10-15T18:25:11+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: Warm-dispatch performance criterion — Partially
  resolved. C5 is named with an explicit tolerance, but C5 and the unit of
  0.01 are undefined, and `warm-dispatch-4.json` records C5 = 1.277 against
  run 3's 1.326, a spread about 5× the tolerance.
- 🟡 **Dependency**: Measurement harness — Resolved. The default-root
  measurement, the "before" ordering and the harness limitation are recorded.
- 🟡 **Clarity / Testability**: "Prune" overloaded — Partially resolved. The
  operations are named, but the root-level lock has two names and the
  command's "bounded" wait has no bound.
- 🟡 **Testability / Clarity**: Protected names / allowlist — Partially
  resolved. There is an explicit allowlist, but its criteria run only single-file
  prune.
- 🟡 **Clarity / Scope**: Dev-checkout root — Resolved.
- 🟡 **Dependency**: 0334 prerequisite — Resolved in 0218. 0334's own file
  still reads "Blocks: none" (its `done` status is in commit `41b7d01c62c4`,
  not yet in this workspace's history).
- 🔵 **Completeness / Scope / Dependency**: ADR-0063 departure — Resolved;
  scope now flags the ADR as both a precondition and a deliverable.
- 🔵 **Scope**: Kind `task` — Resolved (story); superseded by the new
  bundling finding.
- 🔵 **Scope**: `ENOENT` re-resolve separable — Resolved by the Ordering line.
- 🔵 **Dependency**: `ENOENT` rollout coupling — Resolved.
- 🔵 **Testability**: Claim throttling and ignored claims — Resolved.
- 🔵 **Testability**: Corrupt-entry "never absent" — Partially resolved; no
  stated way to observe the operations.
- 🔵 **Clarity**: `.tmp-` removal implied — Resolved.
- 🔵 **Clarity**: Override vs shared root — Partially resolved; the title and
  Summary still say "shared".
- 🔵 **Testability**: Concurrent store and prune exclusion — Partially
  resolved; the interleaving is unspecified (now major).
- 🔵 **Clarity**: Undefined terms — Partially resolved; shim, claim and lease
  are defined, but pointer, generation, layout version, cold/warm path and the
  value of `AGE_BACKSTOP` are not.
- 🔵 **Completeness**: Status `draft` — Unchanged by design.

### New Issues Introduced

- 🟡 **Scope**: The widening bundles about five independently deliverable
  concerns (re-resolve, claims plus single-file prune, tree-prune completeness,
  residue sweep, cold-path triggers) and about 33 criteria into one story;
  promote to an epic or split along the Ordering line.
- 🟡 **Clarity / Scope**: Default-root exclusion is ambiguous — it is unclear
  whether existing tree claims and reaps stop there, and whether the command's
  widened tree prune and residue sweep apply there.
- 🟡 **Testability**: Allowlist criteria exercise single-file prune, not root
  prune; an over-broad tree prune or residue sweep passes.
- 🟡 **Testability**: Concurrent claim-versus-prune criterion does not name
  the interleavings or require a deterministic hook.
- 🟡 **Testability / Completeness**: The command's "bounded" lock wait has no
  bound and no stated outcome on expiry.
- 🔵 **Clarity**: Summary's "everything … is eventually reclaimed"
  contradicts the files kept on purpose.
- 🔵 **Clarity**: The root-level lock is also called "the single-file prune
  lock"; "each lock" is not enumerated.
- 🔵 **Scope**: The ADR is both a precondition and an acceptance criterion.
- 🔵 **Dependency**: ADR-0064 and ADR-0061 are extended but not named under
  Decisions.
- 🔵 **Dependency**: Reliance on Claude Code's orphan sweep (known defects
  per 0223) is not recorded as an external dependency.
- 🔵 **Testability**: Cold-path skip-when-held is specified only for `store`,
  not for tree materialise.
- 🔵 **Testability**: Default-root criteria lack a seeded test setup;
  `ACCELERATOR_CACHE_DIR` equal to `${ACCELERATOR_PLUGIN_ROOT}/bin` is
  unclassified.
- 🔵 **Testability**: The "before" measurement ordering has no checkable
  criterion (e.g. `baseline_commit` predating the change).
- 🔵 **Suggestions**: an end-to-end bounded-growth scenario (testability);
  the materialise criterion should say root prune (clarity); state why dev
  checkouts follow from the override-root rule (clarity); check whether 0136
  is the right parent (scope).

### Assessment

The six round-1 majors are resolved or narrowed, and the item is now explicit
about scope, ordering, dependencies and accepted risks. The widening to the
whole override root introduced five new majors, mostly about precision in the
new tree-prune and lock surface and about the size of a single story. It is not
yet ready for planning: decide on decomposition first, since splitting would
move several of the new findings into child items.

## Re-Review (Pass 3) — 2026-10-15T19:07:18+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Scope**: Single story too large — Accepted by the author; kept as one
  story.
- 🟡 **Clarity / Scope**: Default-root ambiguity — Resolved in Requirements;
  the testability gap in its criterion is now a new major (below).
- 🟡 **Testability**: Allowlist tests only single-file prune — Resolved.
- 🟡 **Testability**: Claim-versus-prune ordering — Resolved (list before
  reading claims, test hook).
- 🟡 **Testability / Completeness**: Unbounded wait for the command — Resolved
  (no time limit; root lock named).
- 🔵 **Testability / Clarity**: C5 tolerance and undefined C5 — Resolved by
  the filesystem-call criterion; the smoke-run criterion is now flagged as
  verifying nothing specific to this change.
- 🔵 **Clarity / Scope**: "Shared" / "everything" — Partially resolved; the
  Summary's "exceptions under Assumptions" misses the layout-version
  exception.
- 🔵 **Scope**: ADR as prerequisite and deliverable — Partially resolved;
  three lenses now ask who owns the ADR and how it is tracked.
- 🔵 **Dependency**: ADR-0064 not named — Resolved.
- 🔵 **Dependency**: Orphan sweep — Resolved.
- 🔵 **Clarity / Completeness**: Undefined terms and `AGE_BACKSTOP` —
  Resolved; overloaded terms remain (below).
- 🔵 **Dependency**: 0334's own file — Left as is by the author.
- 🔵 **Suggestions**: end-to-end scenario, materialise criterion,
  corrupt-entry observation — Resolved; parent 0136 — kept by the author.

### New Issues Introduced

- 🟡 **Clarity / Completeness**: Version-claim files have no stated path or
  reclaim rule; they accumulate per retired version, are missing from the
  removable shapes, and the end-state criterion omits C's claim and A's and
  B's expired ones.
- 🟡 **Testability**: Default-root criterion does not seed the inputs that
  would show the widened tree prune, claim and lock reaping, and residue sweep
  stay out; nor does it show the existing tree prune still works there.
- 🔵 **Scope / Clarity**: `ENOENT` re-resolve, the `store` invariant and the
  command's no-new-locks change are not limited to override roots, despite
  "nothing else runs" in the default root.
- 🔵 **Completeness / Dependency / Scope**: The prerequisite ADR has no
  identifier, owner or tracking item; `blocked_by` exists in the work-item
  template but 0218 records the 0334 edge only in prose.
- 🔵 **Clarity**: Materialise "tree prune under the locks it takes" vs
  root prune trying every tree lock; no criterion for a held tree lock on the
  cold path (testability).
- 🔵 **Clarity**: The layout-version exception is missing from Assumptions.
- 🔵 **Clarity**: Overloaded terms — claim, launcher, lock, probe.
- 🔵 **Clarity**: Residue shapes, "seams", C5 and "verified structurally"
  undefined.
- 🔵 **Testability**: No filesystem-call bound for the override-root warm
  path; the measurement smoke criterion verifies nothing specific to this
  change; the end state does not account for C's own fresh residue;
  foreign-owner and dead-`pid` setups need test seams.
- 🔵 **Suggestions**: concrete `--older-than` pair; deterministic setup for
  concurrent prunes and `ENOENT`; "the ADR" used before it is introduced;
  user-facing docs as a consumer; dev-checkout growth unowned.

### Assessment

Pass 2's five majors are resolved or accepted. Two new majors remain, both
narrow: an unspecified lifecycle for version-claim files (a growth leak this
item itself introduces) and an incomplete default-root test setup. The rest is
precision work in test setups and terminology. One more editing round should
make the item ready for planning.

## Re-Review (Pass 4) — 2026-10-15T21:45:09+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Completeness**: Version-claim lifecycle — Resolved.
- 🟡 **Testability**: Default-root test setup — Resolved for the cold
  dispatch and the verb; tree materialise and the orphan reap in the default
  root are still unexercised (minor, below).
- 🔵 **Scope / Clarity**: Changes that apply in every root — Partially
  resolved; named in Scope, but the Summary still says the default root is out
  of scope, and the lock-creation change still sits under Tree prune.
- 🔵 **Completeness / Dependency / Scope**: ADR tracking and `blocked_by` —
  Resolved.
- 🔵 **Clarity / Testability**: Tree-lock coverage — Resolved; the lock-skip
  criteria could still pass trivially (minor, below).
- 🔵 **Clarity**: Layout-version exception — Resolved.
- 🔵 **Clarity**: Terms with more than one meaning — Partially resolved; bare
  "launcher" is still used for the launcher entry, and `.tmp-` covers three
  shapes.
- 🔵 **Testability**: Override-root warm cost; smoke run; end-state timing;
  CI test seams — Resolved.
- 🔵 **Suggestions**: `--older-than`, concurrency setups, user docs,
  dev-checkout growth — Resolved.

### New Issues Introduced

- 🟡 **Testability / Completeness**: The Documentation requirement has no
  acceptance criterion.
- 🟡 **Testability**: Dispatch-level criteria (root prune before `exec`, held
  root lock, read-only root) omit "Given an override root"; the read-only one
  passes trivially.
- 🔵 **Clarity**: Tree reclaim — unclear whether it requires "unclaimed" only,
  or "unclaimed and superseded"; generations with no pointer unaddressed.
- 🔵 **Clarity**: The `store` invariant is worded as preserved, not changed;
  "`store` takes no lock" reads as covering its prune; the corrupt-entry
  criterion's re-resolve trigger is unnamed.
- 🔵 **Completeness / Clarity**: The root lock has no path.
- 🔵 **Dependency**: The uid and `pid`-liveness seams are new, not from 0334;
  `flock` support on the override root's filesystem (`ENOLCK`) is not
  captured.
- 🔵 **Testability**: Ordering criteria don't name how the order is observed;
  the A/B/C "cold dispatch" setup is underspecified; the criteria for changes
  that apply in every root don't name the default root.
- 🔵 **Suggestions**: name the user the story serves in Context; mark the
  smoke run as not blocking completion; mixed-launcher window as an accepted
  Assumption; link 0196; fixed baseline for the warm-path call list; C5
  label.

### Assessment

Both pass-3 majors are resolved. The two remaining majors are criterion
completeness fixes, not design gaps. The substantive open point is the tree
reclaim condition (unclaimed alone, versus unclaimed and superseded). The rest
is test-setup precision and wording. The item is close to ready for planning.

## Re-Review (Pass 5) — 2026-10-15T22:02:22+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability / Completeness**: The Documentation requirement has no
  criterion — Resolved.
- 🟡 **Testability**: Dispatch criteria don't name their root — Resolved.
- 🔵 **Clarity**: Tree reclaim condition — Partially resolved; now explicit,
  but the sentence can be read several ways (now major, below).
- 🔵 All other pass-4 minors and suggestions — Resolved, except that the
  Summary's wording on the `store` invariant and the default root still
  disagrees with Scope (minor, below).

### New Issues Introduced

- 🟡 **Clarity / Testability / Completeness**: The tree-prune eligibility
  sentence can be read several ways; "unknown" lease and "own digest" are
  undefined; the keep conditions (held lease, unknown-and-young lease, own
  digest) have no criteria.
- 🟡 **Clarity**: Tree prune visits only artifacts with a pointer or
  generation, so the orphan tree locks and sidecars it must remove are never
  reached; no lock is named for removing them.
- 🟡 **Testability / Completeness**: The non-contention lock failure
  (`ENOLCK`) has no criterion and no lock-failure seam.
- 🔵 **Clarity / Scope**: The Summary says "claims and pruning stay out" of
  the default root and calls the `store` invariant a change.
- 🔵 **Clarity**: "Probe" and "liveness seam" are still ambiguous in the
  criteria; version-claim removal order differs between Requirement and
  criterion; "the verb" is used before it is introduced, and
  `--older-than` has no unit.
- 🔵 **Testability**: The interrupted-store mechanism and the fetch source for
  the A/B/C scenario are unnamed; age-boundary keep cases are missing for
  bootstrap lock directories, tree locks and override-root orphan
  generations; the smoke-run criterion cannot fail; capture of the fixed call
  list can't be checked from history.
- 🔵 **Scope**: The smoke run should move out of the criteria; one Context
  sentence still describes the scope as single-file only.
- 🔵 **Dependency**: The 0196 link points to a work item that never describes
  claims or leases; 0214 and the 0196 plan are the real sources; References
  omits 0196.
- 🔵 **Suggestion**: warm-path call list for `ACCELERATOR_CACHE_DIR` naming
  the default root.

### Assessment

All pass-4 majors are resolved. Three majors remain, all in the tree-prune
and lock-failure rules: one ambiguous condition, one gap in which artifacts are
visited, and one untested error path. These are precise and fixable in one
edit round.

## Re-Review (Pass 6) — 2026-10-15T22:07:57+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Tree-prune eligibility — Resolved in
  Requirements; two criteria still don't isolate the condition they claim to
  test (below).
- 🟡 **Clarity**: Which artifacts tree prune visits — Resolved.
- 🟡 **Testability**: `ENOLCK` path — Resolved for the root lock on `store`;
  tree locks and materialise are not covered (minor, below).
- 🔵 Pass-5 minors and suggestions — Resolved. The smoke run moved to
  Technical Notes, which scope now flags as a deliverable outside the criteria.

### New Issues Introduced

- 🟡 **Clarity**: A launcher entry the bootstrap writes before the version's
  first claim can be pruned immediately; the Technical Notes rationale ("far
  shorter than `CLAIM_WINDOW`") contradicts the claim-only rule.
- 🟡 **Clarity**: `<name>-<version>-<64hex>` cannot be split unambiguously
  now that names are arbitrary and versions contain hyphens.
- 🟡 **Clarity**: Removal of a stale version claim whose version still has a
  fresh sibling claim can be read two ways.
- 🟡 **Clarity**: The A/B/C "root holds only" list omits C's sidecars and
  the directories; "A's trees" has no defined referent; "unclaimed" doesn't
  say which kind of claim.
- 🟡 **Testability**: The pinned-digest guard can pass without being
  exercised (the pinned tree's lease state is unspecified; no free-lease
  control).
- 🟡 **Testability**: The layout-version criterion lacks setup that isolates
  the condition and doesn't check the pointer.
- 🔵 **Completeness / Scope**: The authoritative allowlist sits in Technical
  Notes rather than Requirements; the measurement record is a deliverable
  outside the criteria; the `AGE_BACKSTOP` note is duplicated.
- 🔵 **Clarity**: Lease-probe outcomes worded three ways; the command's root
  prune and "tree prune" are not qualified as override-only; tree-lock
  granularity is inconsistent; Summary pronouns and its reclaim condition;
  "orphan reap" and "pin" undefined.
- 🔵 **Testability**: Lock failure on tree locks and on materialise;
  materialise pruning its own artifact; claim path and ordering; checkout
  override, `trees/.tmp-*` and lock-held removal have no criteria; A/B/C
  back-dating unspecified.
- 🔵 **Dependency**: The first ordering step mixes the call list (which needs
  no launcher change) with the seams; the filesystem-call recorder is not
  listed as existing or new; the 0334 entry omits the executor's lease hold;
  0222 and ADR links are absent from `relates_to`.

### Assessment

Pass 5's majors are resolved. Two of pass 6's majors are real design gaps the
growing precision has exposed: an unclaimed version's fresh launcher entry is
prunable, and splitting entry names is ambiguous. The other four are about
precision in the end-state and isolating criteria. One more round of edits
resolves them.

## Verdict Change — 2026-10-15T22:15:18+00:00

**Verdict:** APPROVE

The author approved the work item after all pass-6 findings were addressed in
it, without a further re-review pass.

