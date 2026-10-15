---
type: "work-item-review"
id: "0339-tree-reap-deletes-another-artifact-s-in-progress-materialise-review-1"
title: "Work Item Review: Tree reap deletes another artifact's in-progress materialise"
date: "2026-10-15T23:23:09+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0339"
work_item_id: "0339"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: []
last_updated: "2026-10-16T00:08:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Tree reap deletes another artifact's in-progress materialise

**Verdict:** REVISE

0339 is a tightly scoped, structurally complete bug: a full Reproduction /
Actual / Expected, Given/When/Then criteria on named `.tmp-` entries, and a
recorded rationale for the lock probe over an age guard. Two major findings
cross the revise threshold, and both concern the own/other boundary: what
"artifact" identifies (name or name + platform), and whether the
own-artifact criterion holds Y's lock so a self-probe regression fails. The
remaining findings cluster on the pinned-archive rule and on edge branches
(unparseable names, probe errors, retired artifacts) that have rules but no
criteria.

### Cross-Cutting Themes

- **Own/other artifact boundary** (flagged by: clarity, testability) —
  neither "artifact" identity nor the own-artifact lock state is pinned, so
  a same-name-other-platform entry or a self-probe could reintroduce or mask
  the race without failing any criterion.
- **Pinned-archive rule** (flagged by: clarity, dependency, scope,
  testability) — "its artifact" is ambiguous, the reap would need the
  launcher's full pin set (today `keep` holds one digest), the fixture is
  undefined, and the rule widens behaviour beyond the race fix.
- **Unparseable and retired entries** (flagged by: clarity, scope,
  testability) — the Assumption keeps unparseable names forever, which reads
  against "orphans are still removed", has no criterion, and no item owns
  their eventual cleanup; "lock is free" for a retired artifact with no lock
  file is undefined.

### Findings

#### Major

- 🟡 **Clarity**: "Artifact" could mean name or (name, platform), changing
  what counts as "own" entries
  **Location**: Requirements
  The tree lock is keyed on name and platform, but Requirements speak only
  of "own" versus "another" artifact; a same-name, other-platform entry is
  "own" under one reading and protected under the other.
- 🟡 **Testability**: Own-artifact criterion omits that Y's lock is held
  during its reap
  **Location**: Acceptance Criteria
  In production Y's reap always runs under Y's lock; a lock-free test would
  not catch an implementation that probes its own artifact and keeps its
  orphans.

#### Minor

- 🔵 **Clarity**: "its artifact" in the pinned-archive rule has two
  possible referents
  **Location**: Requirements
  The fourth Expected bullet and AC2 could mean Y's pins or the archive's
  own artifact's pins.
- 🔵 **Dependency**: Reap gains an unstated dependency on the launcher's
  full pin set
  **Location**: Requirements
  `keep` holds one digest today; keeping X's pinned archive needs every
  artifact's pins at the reap call site, a source the item does not name.
- 🔵 **Testability**: Fixture for "the running launcher pins" X's digest is
  undefined
  **Location**: Acceptance Criteria
  A test seeding X's digest straight into `keep` could pass while
  production callers never supply it; AC2 also overlaps AC3.
- 🔵 **Clarity**: Unparseable-name Assumption reads against the
  always-reclaim requirement
  **Location**: Assumptions
  "Foreign and kept" versus "orphans of other artifacts are still removed";
  "foreign" is undefined, as is "lock free" for a retired artifact with no
  lock file.
- 🔵 **Testability**: Rule for unparseable entry names has no criterion
  **Location**: Assumptions
  Name parsing decides own versus other; nothing tests unparseable names or
  hyphenated artifact names.
- 🔵 **Testability**: "Neither waits" can only fail by hanging
  **Location**: Acceptance Criteria
  A blocking regression stalls the test rather than failing; state "returns
  `Ok` while X's lock is still held", optionally under a bounded timeout.
- 🔵 **Testability**: No expected outcome when the lock probe cannot answer
  **Location**: Requirements
  A probe error (e.g. no `flock` support) is a third state; removing on
  error would reintroduce the race and meet every criterion.
- 🔵 **Clarity**: Summary describes one reap trigger; Technical Notes imply
  two
  **Location**: Summary
  `resolver.rs:746-749` is a prune path; state that the Expected behaviour
  applies to every caller.
- 🔵 **Clarity**: Several domain terms used without definition
  **Location**: Requirements
  "root", "generation", "pins", "retired", "foreign", "lease", `keep`;
  `<X-generation>` versus `<generation>` placeholders.
- 🔵 **Dependency**: Mixed-launcher-version coupling on a shared root not
  captured
  **Location**: Dependencies
  An older launcher reaping the same root still deletes foreign entries;
  the race stays open until every reaping launcher carries the fix.

#### Suggestions

- 🔵 **Scope**: Keeping another artifact's pinned partial archive goes
  beyond the race fix
  **Location**: Requirements
  With X's lock free nothing is corrupted, only a resumable download lost;
  record it as a deliberate widening or split it out.
- 🔵 **Scope**: Unparseable entries are kept, but nothing owns their later
  cleanup
  **Location**: Assumptions
  Name the owner (e.g. 0218's root prune) or accept the residue explicitly.
- 🔵 **Dependency**: Interaction with 0218's tree-lock reaping not noted
  **Location**: Dependencies
  0218 must preserve the contract "held lock = materialise in progress; lock
  files not removed while held".
- 🔵 **Dependency**: Reciprocal blocked-by link on 0218 not confirmed
  **Location**: Frontmatter: blocks
- 🔵 **Testability**: No criterion for the user-visible race outcome
  **Location**: Acceptance Criteria
  All criteria use the held lock as a stand-in; nothing pins that the lock
  covers download and extraction.
- 🔵 **Completeness**: Reproduction describes a race but not how to trigger
  it
  **Location**: Requirements
  One sentence: seed X's `.tmp-` entries, hold X's lock, run Y's reap.

### Strengths

- ✅ Full bug reproduction with named actors X and Y, exact `.tmp-` paths,
  the destructive outcome and four expected behaviours.
- ✅ Context explains the lock scope (same artifact serialises, different
  artifacts run concurrently) before Requirements depend on it.
- ✅ Criteria are Given/When/Then on named filesystem entries, with
  lock-held and lock-free branches split, so each maps to a fixture plus
  existence assertions.
- ✅ Technical Notes justify the held lock as a stand-in for an in-progress
  materialise, so tests need no real concurrency.
- ✅ Single defect, right-sized; split from 0218 and recorded as its
  blocker, keeping 0218 from growing further.
- ✅ Drafting Notes record why the lock probe beat an age guard, the
  priority rationale and the blocking decision.
- ✅ Frontmatter linkage (`parent`, `blocks`, `relates_to`) matches the
  Dependencies section.

### Recommended Changes

1. **Define "artifact" as the (name, platform) pair the tree lock is keyed
   on** (addresses: "Artifact" could mean name or (name, platform))
   One sentence in Context; the own/other split then follows the lock key.
2. **State Y's lock is held in the own-artifact criterion** (addresses:
   Own-artifact criterion omits that Y's lock is held)
   "Given Y's tree lock is held by the reaping caller, …".
3. **Rewrite the pinned-archive rule and name the pin source** (addresses:
   "its artifact" referent; full pin set dependency; pin fixture undefined;
   scope widening)
   Say the reap keeps an archive whose digest the reaping launcher pins for
   that archive's own artifact; name where the full pin set comes from and
   whether the reap call site reaches it; split AC2 into an unconditional
   "unpinned X archive is removed"; add a Drafting Note that this widening
   is deliberate (or drop it to a follow-up).
4. **Settle unparseable and retired entries** (addresses: Assumption reads
   against always-reclaim; no criterion for unparseable names; nothing owns
   cleanup)
   Drop "foreign" for "unparseable", mark it as an exception to the
   reclaim bullet, name its owner or accept the residue, define "lock free"
   when no lock file exists, and add criteria for unparseable and
   hyphenated names.
5. **Add the probe-error branch and make "neither waits" assertable**
   (addresses: no outcome when the probe cannot answer; "neither waits"
   only fails by hanging)
   Probe error keeps X's entries; the reap returns `Ok` while X's lock is
   held, under a bounded timeout.
6. **State that every reap caller is covered** (addresses: one reap
   trigger versus two)
   Name the post-materialise and prune callers.
7. **Record the remaining couplings** (addresses: mixed-launcher-version
   window; 0218 lock contract; reciprocal blocker)
   Assumption on the old-launcher window; extend the 0218 Blocks note with
   the lock contract; confirm 0218 lists 0339 as a blocker.
8. **Gloss domain terms and unify placeholders; add the reproduction
   recipe** (addresses: undefined terms; reproduction trigger)

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Mostly clear, with concrete Reproduction / Actual / Expected
terms and consistent X/Y actors. Ambiguities remain over what "artifact"
means, the referent of "its" in the pinned-archive rule, the tension
between the unparseable-name Assumption and the always-reclaim requirement,
and several undefined domain terms.

**Strengths**:
- Named actors X and Y make every statement's subject unambiguous.
- Context explains the lock scope before Requirements depend on it.
- Every criterion names the reaping and the affected artifact.
- Drafting Notes justify the lock probe over an age guard.

**Findings**:
- 🟡 major / medium — Requirements — "Artifact" could mean name or (name,
  platform). The lock is keyed on both; Requirements say only "own" versus
  "another". A same-name, other-platform entry is removed unconditionally
  under one reading and protected under the other, and no criterion
  catches the wrong reading. Define "artifact" as the lock's (name,
  platform) key.
- 🔵 minor / high — Requirements — "its artifact" has two referents. The
  fourth Expected bullet and AC2 could mean Y's pins or the archive's
  artifact's pins; under the first, X's resumable archive stays exposed.
  Use the same explicit wording in all three places.
- 🔵 minor / medium — Assumptions — "Foreign and kept" reads against
  "orphans of other artifacts are still removed". "Foreign" is undefined;
  "lock free" for a retired artifact with no lock file is undefined.
- 🔵 minor / medium — Summary — one reap trigger described, two implied
  (`resolver.rs:746-749` prune path; 0218's cold-path prune). State the
  Expected behaviour applies to every caller.
- 🔵 minor / medium — Requirements — undefined terms: "root",
  "generation", "pins", "retired", "foreign", "lease", `keep`; mixed
  `<X-generation>` / `<generation>` placeholders.

### Completeness

**Summary**: Structurally complete for a bug: summary, causal context,
four-part reproduction, seven criteria, and populated Dependencies,
Assumptions, Technical Notes and Drafting Notes. The only gap is that the
reproduction describes a race rather than how to trigger it on purpose.

**Strengths**:
- Reproduction covers input, action, actual and expected outcome.
- Context explains the cause rather than restating the summary.
- Criteria cover lock held, lock free, pinned archive, non-waiting,
  retired artifacts, today's own-artifact behaviour, and `mise run`.
- Assumptions record the name-parsing assumption and its failure mode.
- Frontmatter is complete and matches Dependencies.
- Drafting Notes answer the obvious follow-up questions.

**Findings**:
- 🔵 suggestion / low — Requirements — reproduction describes a race but
  not how to trigger it. Add: seed X's `.tmp-` entries, hold X's tree lock,
  run a reap for Y.

### Dependency

**Summary**: Mostly sound: 0218 captured with rationale, 0334 / 0214 / 0136
linked, and the existing `try_single_flight` probe named. Unrecorded
couplings: protection needs every launcher on a root to carry the fix,
keeping other artifacts' archives needs the full pin set, and the probe
must coexist with 0218's tree-lock reaping.

**Strengths**:
- `blocks` matches the Dependencies bullet, with ordering rationale.
- 0334 and 0214 linked with reasons; frontmatter and body agree.
- The prerequisite probe exists and the lock-scope invariant is stated.
- Name-parsing reliance stated in Assumptions.

**Findings**:
- 🔵 minor / medium — Dependencies — mixed-launcher-version coupling. An
  older launcher on the same root still reaps foreign entries; record the
  window or a guard that closes it.
- 🔵 minor / medium — Requirements — reap gains a dependency on the full
  pin set; name its source and whether the reap call site reaches it, or
  record the plumbing as a prerequisite.
- 🔵 suggestion / low — Dependencies — 0218 must preserve the tree-lock
  probe contract; extend the Blocks note.
- 🔵 suggestion / low — Frontmatter: blocks — confirm 0218 lists 0339 as a
  blocker.

### Scope

**Summary**: Tightly scoped: one race in one component, consistent across
Summary, Requirements and criteria, and correctly split out of 0218 as a
blocker. The pinned-archive rule goes slightly beyond the race fix.

**Strengths**:
- All sections describe the same single defect.
- Kept separate from 0218 and recorded as its blocker.
- Own-artifact behaviour preserved with a regression criterion.
- Drafting Notes narrow the design up front.
- `bug` kind and `medium` priority fit the work.

**Findings**:
- 🔵 suggestion / medium — Requirements — keeping another artifact's
  pinned archive when its lock is free loses no correctness, only a
  resumable download; record it as a deliberate widening or split it out.
- 🔵 suggestion / low — Assumptions — unparseable entries are kept forever;
  name the owner of their cleanup or accept the residue.

### Testability

**Summary**: Reproduction is well specified and criteria map to
fixture-based reaper tests. Gaps: the own-artifact criterion does not hold
Y's lock, and three branches (unparseable names, probe error, blocking)
lack a defined pass/fail.

**Strengths**:
- Full reproduction with exact paths and expected behaviours.
- Criteria are precondition / action / observable outcome on named entries.
- Lock-held and lock-free branches split.
- Held lock justified as a stand-in for an in-progress materialise.
- Regression criterion for the reaper's own artifact.

**Findings**:
- 🟡 major / medium — Acceptance Criteria — own-artifact criterion omits
  Y's held lock; a self-probing implementation would keep Y's orphans and
  still pass a lock-free test. Rephrase to "Given Y's tree lock is held by
  the reaping caller, …".
- 🔵 minor / high — Assumptions — no criterion for unparseable names or
  hyphenated artifact names.
- 🔵 minor / medium — Acceptance Criteria — "neither waits" can only fail
  by hanging; assert `Ok` while X's lock is held, under a bounded timeout.
- 🔵 minor / medium — Acceptance Criteria — pin fixture undefined; AC2
  overlaps AC3. Name the observable input, split AC2, add a call-site
  criterion that Y's reap receives X's pinned digest.
- 🔵 minor / medium — Requirements — no outcome when the probe cannot
  answer; add "X's entries survive and the reap completes".
- 🔵 suggestion / low — Acceptance Criteria — no criterion pins the
  stand-in (lock covers download and extraction) or the end-to-end outcome.

## Re-Review (Pass 2) — 2026-10-15T23:34:55+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: "Artifact" could mean name or (name, platform) — Resolved
- 🟡 **Testability**: Own-artifact criterion omits Y's held lock — Resolved
- 🔵 **Clarity**: "its artifact" in the pinned-archive rule — Resolved
- 🔵 **Dependency**: Reap depends on the launcher's full pin set — Resolved
- 🔵 **Testability**: Pin fixture undefined — Partially resolved (see the
  new caller-coverage finding)
- 🔵 **Clarity**: Unparseable-name Assumption against always-reclaim —
  Resolved
- 🔵 **Testability**: No criterion for unparseable names — Resolved
- 🔵 **Testability**: "Neither waits" only fails by hanging — Resolved for
  AC1; the bounded timeout is not applied to the other held-lock criteria
- 🔵 **Testability**: No outcome when the probe cannot answer — Resolved
- 🔵 **Clarity**: One reap trigger versus two — Resolved
- 🔵 **Clarity**: Undefined domain terms — Partially resolved (`keep`,
  "root", tree lock versus single-flight lock remain)
- 🔵 **Dependency**: Mixed-launcher-version coupling — Resolved
- 🔵 **Scope**: Pinned-archive retention beyond the race fix — Still
  present, accepted deliberately and recorded in Drafting Notes
- 🔵 **Scope**: Nothing owns unparseable-entry cleanup — Resolved (residue
  accepted)
- 🔵 **Dependency**: 0218 lock contract — Partially resolved (contract
  stated; what 0218 must change is not)
- 🔵 **Dependency**: Reciprocal blocker on 0218 — Resolved (0218 already
  lists 0339 in `blocked_by`)
- 🔵 **Testability**: No end-to-end race criterion — Still present,
  deliberately skipped
- 🔵 **Completeness**: Reproduction trigger — Resolved

### New Issues Introduced

- 🟡 **Testability**: Pinned-archive rule not verified through both callers
  — AC3 names no caller; `acquire` and `prune` each build their own `keep`
  set, so one could still pass a single digest with every criterion green.
- 🔵 **Testability**: "Reap continues" not observed — AC1 and AC5 have no
  removable third-artifact entry, so an early-exit scan passes.
- 🔵 **Testability**: "Cannot be probed" precondition undefined — AC5 names
  no concrete trigger.
- 🔵 **Testability**: Bounded timeout only on AC1 — AC6, AC7, AC8 and AC10
  also reap under a held lock.
- 🔵 **Clarity**: Context implies the lock is released before the reap —
  "until the generation is renamed into place, then runs `reap_orphans`".
- 🔵 **Clarity**: Precedence between lock-free removal and pinned-archive
  keep unstated — only the unknown-platform bullet is marked an exception.
- 🔵 **Clarity**: Unclear whether another artifact's `.lock` file is an
  "entry" that gets removed.
- 🔵 **Dependency**: What 0218 must change to keep the contract is not
  stated — reuse the no-create probe; never remove a held lock file.
- 🔵 **Clarity / Testability**: Generation-name shape not stated — the
  parse rule relies on it beginning `<name>-<platform>-`.
- 🔵 **Completeness**: Own-artifact Expected bullet defers to "as today".
- 🔵 **Dependency**: No governing ADR named for tree-lock and reaper
  semantics.

### Assessment

Both pass-1 majors are resolved and the item is implementable as written.
One new major (caller coverage of the pinned-archive rule) falls below the
two-major revise threshold. It and the clarity points on lock scope, rule
precedence and lock files are worth a short edit before planning, since
each could let a wrong implementation pass every criterion.

## Re-Review (Pass 3) — 2026-10-15T23:39:02+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Testability**: Pinned-archive rule not verified through both callers
  — Resolved (AC3 covers the post-materialise reap and `prune`)
- 🔵 **Testability**: "Reap continues" not observed — Resolved (third
  artifact Z in AC1 and AC5)
- 🔵 **Testability**: "Cannot be probed" precondition undefined — Resolved
- 🔵 **Testability**: Bounded timeout only on AC1 — Resolved (preamble)
- 🔵 **Clarity**: Context implies the lock is released before the reap —
  Resolved
- 🔵 **Clarity**: Precedence between removal and pinned-archive keep —
  Resolved
- 🔵 **Clarity**: Whether `.lock` files are entries — Resolved
- 🔵 **Clarity / Testability**: Generation-name shape — Resolved
- 🔵 **Completeness**: Own-artifact rule deferred to "as today" — Resolved
- 🔵 **Dependency**: What 0218 must change — Partially resolved (stated in
  0339 only, not in 0218)
- 🔵 **Dependency**: No governing ADR named — Resolved (ADR-0072)
- 🔵 **Scope**: Pinned-archive retention beyond the race fix — Still
  present, accepted deliberately

### New Issues Introduced

- 🔵 **Clarity**: "Artifact" switches between the (name, platform) pair and
  the bare name — `<X>` in the archive pattern and "Y's name on another
  platform" use the name alone.
- 🔵 **Clarity**: Lock contract names only materialise, but `prune` holds
  the same lock.
- 🔵 **Clarity**: Unclear whether the lock is probed once per artifact or
  once per entry.
- 🔵 **Clarity**: AC6's "resumable Y archive" and "orphans" not tied to the
  defined terms.
- 🔵 **Testability**: AC8 does not test the name-prefix collision
  (`foo` versus `foo-bar`).
- 🔵 **Testability**: AC6 and AC7 test only the keep side (unpinned Y
  archive removed; other-platform Y entries removed when free).
- 🔵 **Testability**: "No reap removes a lock file" asserted only in AC2.
- 🔵 **Testability**: Held-lock protection not tested through the
  post-materialise caller.
- 🔵 **Dependency**: Other materialise routes (0222's
  `cache ensure --from`) not named as bound by the lock-scope contract.
- 🔵 **Dependency**: 0334 described as having "fixed" lease handling, but
  0334 is still `ready`; both items edit the reaper and `prune` with no
  recorded order.
- 🔵 **Scope**: 0218's constraints recorded only in 0339.
- 🔵 **Clarity**: "Lease" and "root prune" used without gloss.
- 🔵 **Completeness**: Expected states reap rules but not X's outcome.

### Assessment

No critical or major findings remain; the item is ready for planning as
written. The remaining minors tighten the test surface (prefix collision,
remove-side counterparts, lock-file survival) and correct the 0334
description, whose unshipped status leaves the two reaper changes
unsequenced.

## Re-Review (Pass 4) — 2026-10-15T23:54:12+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Clarity**: "Artifact" switches between pair and bare name —
  Partially resolved (Assumptions and Technical Notes still say "artifact
  and platform")
- 🔵 **Clarity**: Lock contract names only materialise — Resolved
- 🔵 **Clarity**: Probe once per artifact or per entry — Resolved
- 🔵 **Clarity**: AC6 terms not tied to definitions — Resolved
- 🔵 **Testability**: Name-prefix collision — Resolved
- 🔵 **Testability**: Remove-side counterparts for AC6 and AC7 — Resolved
- 🔵 **Testability**: Lock-file survival asserted once — Resolved
  (preamble)
- 🔵 **Testability**: Held lock not tested through the post-materialise
  caller — Resolved
- 🔵 **Dependency**: Other materialise routes not named — Partially
  resolved (0222 named; ownership of the obligation unstated)
- 🔵 **Dependency**: 0334 described as fixed but status `ready` — Resolved
  (0334 has shipped; its file status lags)
- 🔵 **Scope / Dependency**: 0218's constraints recorded only in 0339 —
  Still present
- 🔵 **Clarity**: "Lease" and "root prune" without gloss — Resolved
- 🔵 **Completeness**: Expected omits X's outcome — Still present
- 🔵 **Scope**: Pinned-archive retention beyond the race fix — Still
  present, accepted deliberately

### New Issues Introduced

- 🟡 **Clarity**: Unclear whether a successful probe keeps the other
  artifact's lock until its entries are removed — probe-then-release leaves
  a window in which a starting materialise of X loses its entries.
- 🔵 **Clarity**: Keep set and "resumable" defined by today's digest-only
  behaviour; Expected implies a per-artifact pin check.
- 🔵 **Testability**: "Pinned for the archive's own artifact" qualifier has
  no criterion that distinguishes it from digest-only matching.
- 🔵 **Testability**: "Once per reap" not observable by any criterion.
- 🔵 **Testability**: Directory-as-lock fixture depends on the probe's open
  mode.
- 🔵 **Clarity**: Several criteria leave Y's own lock state, or the reaping
  artifact, unstated.
- 🔵 **Scope / Completeness / Dependency**: Whether 0339 or 0222 owns the
  tree-lock obligation on the populate route is unstated.
- 🔵 **Dependency**: Protection relies on `flock` giving real mutual
  exclusion on the cache root's filesystem.
- 🔵 **Clarity**: Summary says the scan removes "every" `.tmp-` entry,
  including lock files; Orphan says "materialise" only.
- 🔵 **Testability**: Unknown-platform AC says "contains"; Requirements say
  "parses to".
- 🔵 **Clarity**: "Retired artifact … typically with no lock file" vague.

### Assessment

One new major, below the revise threshold. It is a genuine design gap —
whether the probe holds the lock across removal decides whether the race
is closed or only narrowed — and is worth settling before planning. The
remaining minors are wording and test-precision refinements.

## Re-Review (Pass 5) — 2026-10-16T00:05:28+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Probe holds the lock across removal? — Resolved
- 🔵 **Clarity / Testability**: Per-artifact pin check — Resolved (AC4)
- 🔵 **Testability**: "Once per reap" unobservable — Resolved (moved to
  Technical Notes)
- 🔵 **Testability**: Probe-error fixture open mode — Resolved
- 🔵 **Clarity**: Y's lock state and reaping artifact unstated — Resolved
- 🔵 **Dependency**: `flock` reliance — Resolved
- 🔵 **Clarity**: Summary "every `.tmp-` entry"; Orphan wording — Resolved
- 🔵 **Testability**: "contains" versus "parses to" — Resolved
- 🔵 **Clarity**: Retired artifact vague — Partially resolved
- 🔵 **Clarity**: "Artifact" versus artifact name — Resolved
- 🔵 **Completeness**: Expected omits X's outcome — Resolved
- 🔵 **Scope / Completeness / Dependency**: 0222 ownership — Partially
  resolved (owner named in 0339; 0222 itself silent — now raised as major)
- 🔵 **Scope / Dependency**: 0218 constraints only in 0339 — Still present
- 🔵 **Scope**: Pinned-archive retention beyond the race — Still present,
  accepted deliberately

### New Issues Introduced

- 🟡 **Dependency**: 0222's tree-lock obligation exists only in 0339;
  0222 neither links 0339 nor mentions the tree lock.
- 🟡 **Testability**: AC2's "lock held at each removal" names no
  observation seam, so a probe-then-release implementation passes.
- 🔵 **Testability**: AC1 and AC6's "moves on" check depends on directory
  iteration order.
- 🔵 **Testability**: Z's "lock is free" leaves lock-file presence
  unspecified.
- 🔵 **Clarity**: `prune` caller versus root prune not related.
- 🔵 **Clarity**: Keep set still defined only by current behaviour.
- 🔵 **Clarity**: "Pinned" used without saying for which artifact.
- 🔵 **Clarity**: Parsing rule silent on repeated platform strings.
- 🔵 **Clarity**: Orphan definition sits awkwardly with own-artifact
  removal.
- 🔵 **Dependency**: `adr:ADR-0072` absent from `relates_to`.
- 🔵 **Testability**: Bounded timeout has no stated bound; Expected outcome
  verified only by proxy.
- 🔵 **Scope**: Summary could name the pinned-archive bundling.

### Assessment

Both pass-4 decisions landed cleanly and completeness is now clear. The
two new majors are narrow: one needs a named test seam in AC2, the other
needs 0222 (and ideally 0218) to carry the obligations 0339 places on
them. Findings per pass are now mostly wording refinements, so further
passes have diminishing returns once these two are closed.

## Verdict Change — 2026-10-16T00:08:00+00:00

**Verdict:** APPROVE

Both pass-5 majors were addressed without a further pass: AC2 now names a
removal-observer seam for checking X's lock at each removal, and 0222 and
0218 now carry the obligations 0339 places on them. The remaining pass-5
minors were applied; the bounded-timeout value and end-to-end verification
of X's outcome were left as suggestions. Approved by the reviewer.
