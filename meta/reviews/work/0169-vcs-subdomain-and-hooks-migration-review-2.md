---
type: work-item-review
id: "0169-vcs-subdomain-and-hooks-migration-review-2"
title: "Work Item Review: VCS Subdomain and Hooks Migration"
date: "2026-07-31T02:03:11+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0169"
parent: "work-item:0136"
relates_to: ["work-item-review:0169-vcs-subdomain-and-hooks-migration-review-1"]
work_item_id: "0169"
reviewer: Toby Clemson
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 2
review_pass: 4
tags: [rust, vcs, hooks, migration]
last_updated: "2026-07-31T09:12:46+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: VCS Subdomain and Hooks Migration

**Verdict:** REVISE

0169 is a dense, unusually well-anchored work item — nearly every claim carries
a `path:line` reference, deliberate behavioural departures are declared rather
than smuggled in as "ports", and prior research has been visibly folded back
(the latency assumption replaced by a measurement, the `gix`/`jj-lib`
feasibility and sub-binary naming collision recorded as resolved). The problem
is that it has grown substantially since `review-1` approved it on 2026-07-20,
and the additions have outrun the criteria: the story now spans four toolchains
and five separable deliverables under one `kind: story` with 15 acceptance
criteria, several of which cannot fail as written, compare against instruments
the same change deletes, or enumerate the wrong set. Nineteen major findings
across all five lenses, no criticals.

The single most important observation is structural: **`review-1` approved a
different work item.** Its scope finding was dispositioned "defensible,
author-justified" before the library swap, the sub-binary decision, the
bootstrap fix and the skill repoint were added. That approval should not be
read as covering the current bundle.

### Cross-Cutting Themes

- **Sub-binary distribution is real, load-bearing scope that appears nowhere in
  Requirements or Acceptance Criteria** (flagged by: scope, completeness,
  testability, dependency — all four) — `accelerator-vcs` is the first
  non-visualiser dispatched sub-binary, requiring `DISPATCHED_SUBBINARIES`,
  `_SUBBINARY_MANIFESTS`, workspace members, `package.description`,
  `.gitignore`, the shared `manifest.example.json` fixture, cross-compile
  staging with `_assert_static_elf`, the `uluru` MPL-2.0 licence exception and
  the `gix` version pin. All of it lives only in Technical Notes. Nothing in the
  definition of done proves the sub-binary is buildable, signable or
  distributable — and this is the surface most likely to fail at release time
  rather than test time.

- **Several criteria compare against instruments this change deletes** (flagged
  by: testability, clarity) — the `vcs status`/`vcs log` goldens name
  `scripts/vcs-status.sh` / `scripts/vcs-log.sh` as the comparator while the
  final criterion removes both; the latency criterion baselines against
  `hooks/vcs-guard.sh`, also removed; and the detect parity gate's byte-exact
  goldens can be re-baselined against the new Rust output, at which point the
  gate asserts the implementation matches itself. No capture-before-delete step
  is stated anywhere.

- **The done-state is indeterminate in two places** (flagged by: scope,
  completeness, testability, clarity — all four bar dependency) — the
  `config-detect.sh` fold-in is "may fold in", the shim double-hash is "if in
  doubt… leave this", and the hooks floor is "adjusted" with no target value.
  A reader of a closed 0169 cannot tell whether `hooks/config-detect.sh` still
  exists, which matters because 0172 and 0174 both inherit that `hooks.json`
  state.

- **The `classify_checkout` arm list omits the two arms whose ordering is
  load-bearing** (flagged by: testability, clarity) — Requirements defines the
  taxonomy as (worktree, submodule, bare, `GIT_DIR`, plain) and the matching
  criterion verifies exactly that set, while Technical Notes states the
  load-bearing cascade is `colocated` preceding `nested-*`. The criterion that
  exists to stop arm order regressing silently does not require testing the pair
  whose order matters.

- **Which shell implementation is the parity reference is stated two ways, and a
  known defect has no disposition** (flagged by: testability, clarity) —
  Requirements pins the port to `vcs-common.sh`; Technical Notes says "Port the
  hooks' own behaviour, not just `vcs-common.sh`'s". The same note records that
  where `.git` is a *file* the guard blocks where it should warn, but never says
  whether the port preserves or corrects that. "Parity" and "correct" give
  opposite answers.

- **Decisions taken on 2026-07-30 created couplings the Dependencies section
  does not record** (flagged by: dependency) — choosing the sub-binary coupled
  the story to 0165's release pipeline; choosing the `permissionDecision`
  envelope coupled it to a specific Claude Code schema version. Neither appears
  in `blocked_by` or Dependencies.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Scope + Completeness + Testability + Dependency**: Sub-binary distribution
  and dependency-policy work is absent from Requirements and Acceptance Criteria
  **Location**: Requirements / Acceptance Criteria
  Registration across `DISPATCHED_SUBBINARIES`, `_SUBBINARY_MANIFESTS`,
  `.gitignore`, `manifest.example.json`, cross-compile staging, the `uluru`
  licence exception and the `gix` pin exists only in Technical Notes. Nothing
  proves the sub-binary ships end-to-end.

- 🟡 **Testability + Clarity**: Status/log golden parity compares against scripts
  the same change deletes, with no normalisation rule
  **Location**: Acceptance Criteria (`vcs status` / `vcs log`)
  The comparator is removed by the final criterion, so the assertion cannot be
  run afterwards; and `vcs log` output carries commit ids, dates and author
  identity that vary per fixture build, with no masking rule stated.

- 🟡 **Testability + Clarity**: `classify_checkout` arm enumeration omits
  `colocated` and `nested-*`, the arms whose ordering is load-bearing
  **Location**: Requirements / Acceptance Criteria
  "At least one ambiguous checkout" names no specific case, so any trivially
  ordered pair satisfies it. The invariant can regress with all criteria green.

- 🟡 **Testability**: The probe-absence criterion passes against the *unfixed*
  bootstrap
  **Location**: Acceptance Criteria (no write-and-exec probe)
  `probe_dir` writes, chmods, execs and removes the file within one invocation,
  so a post-hoc check for `.accelerator-probe-*` finds nothing either way. The
  criterion guarding the ~108 ms regression cannot fail.

- 🟡 **Clarity**: "Zero `jj`/`git` spawns" and the permitted per-query shell
  fallback are mutually exclusive as written
  **Location**: Requirements / Acceptance Criteria
  A fallback exercised on any of the four paths necessarily spawns `jj` or
  `git`. The implementer cannot tell whether the assertion is unconditional or
  excludes declared fallbacks.

- 🟡 **Testability**: Guard decision matrix is not enumerated and contradicts the
  story's own notes
  **Location**: Acceptance Criteria (guard decision parity)
  The criterion says fixtures cover "the allowed read-only/non-VCS patterns",
  but `git log` and `git diff` are *inside* the shell's blocked pattern. It also
  names only the call-class axis, not the repo-mode axis that determines the
  decision, and leaves the `.git`-as-file case undefined.

- 🟡 **Testability**: Detect parity gate can be satisfied by re-baselining its
  own goldens
  **Location**: Acceptance Criteria (`vcs detect` parity gate)
  The criterion does not say whether `hooks/test-fixtures/vcs-detect/*.json` are
  frozen at their shell-produced content; the referenced research notes they
  "would need re-baselining".

- 🟡 **Testability**: Latency criterion lacks a statistic, tolerance and
  enforcement mechanism, and its comparator is deleted
  **Location**: Acceptance Criteria (warm-call latency)
  No statistic, iteration count, tolerance, platform or CI-versus-manual
  disposition. Two verifiers can reach opposite verdicts from the same binary.

- 🟡 **Testability + Scope + Completeness + Clarity**: The final criterion is
  optional in one clause and undetermined in another, so it cannot fail
  **Location**: Acceptance Criteria (removals and hooks floor)
  The floor target value is unstated and depends on an unresolved Open Question;
  the `config-detect.sh` fold-in is "may".

- 🟡 **Dependency**: Claude Code's hook I/O schema is an external dependency
  named nowhere
  **Location**: Dependencies / Assumptions
  If `permissionDecision` or top-level `systemMessage` postdates the v2.1.144
  floor, the new envelope is silently ignored on supported versions —
  reproducing the exact failure mode the research found in the shell guard.

- 🟡 **Dependency**: The sub-binary decision couples the story to 0165's release
  pipeline, which is not a recorded blocker
  **Location**: Dependencies / Frontmatter: `blocked_by`
  The warm-cache criterion presupposes a first successful fetch of a published,
  signed, manifest-listed artefact. Only 0164 is named. The coupling was created
  by the 2026-07-30 decision, after Dependencies was written.

- 🟡 **Dependency**: The `vcs-common.sh` residue is handed to 0174, whose
  recorded scope does not cover it
  **Location**: Technical Notes / Dependencies
  The work item says "That residue is 0174's"; the referenced research records
  that 0174 "removes *tooling, not scripts*, and never names `vcs-common.sh` or
  any hook script". The story's largest residual liability has no real owner.

- 🟡 **Dependency**: The 0172 hand-off is recorded in one direction only, with
  nothing actioning the missing reciprocal edge
  **Location**: Dependencies
  The gap is diagnosed in prose but no requirement, criterion or follow-up
  actions it, so anyone starting 0172 sees no blocker.

- 🟡 **Dependency**: 0183's SessionStart audit and stderr-discard constraint is
  referenced but absent from Dependencies
  **Location**: Dependencies / References
  This story creates a new SessionStart output path subject to 0183's
  stdout-is-context / stderr-is-discarded finding; whichever lands first, the
  other drifts.

- 🟡 **Clarity**: Two identifiers each carry several referents
  (`accelerator-vcs`, "launcher")
  **Location**: Summary / Requirements / Acceptance Criteria
  "launcher" means both `bin/accelerator` and the cached binary it execs —
  inside the requirement that changes exec behaviour on the trust-sensitive
  bootstrap path, where resolving to the wrong artefact means removing the wrong
  check.

- 🟡 **Clarity**: "Full `classify_checkout` taxonomy" is enumerated two
  incompatible ways
  **Location**: Requirements / Technical Notes
  The definitional enumeration omits arms the Technical Notes calls
  load-bearing and the PreToolUse guard criterion depends on.

- 🟡 **Clarity**: Which shell implementation is the parity reference is stated
  two ways, and the `.git`-as-file defect has no disposition
  **Location**: Requirements / Acceptance Criteria / Technical Notes
  The guard's brand-new fixtures could permanently lock in either the buggy or
  the corrected classification without anyone noticing a choice was made.

- 🟡 **Scope**: The bootstrap `probe_dir` fix is an independently deliverable
  concern with a different risk profile
  **Location**: Requirements / Acceptance Criteria
  It shares no code, crate or test harness with the subdomain, carries three of
  the story's criteria plus its own open question, and by the story's own
  account benefits already-shipped hooks independently. *(Author has elected to
  keep this in scope — recorded here as the reviewer's independent position.)*

- 🟡 **Scope**: The story spans four toolchains and five separable deliverables
  under a single story kind
  **Location**: Requirements
  Rust subdomain, two new heavyweight dependencies, sub-binary distribution,
  bash bootstrap change, skill rewrite, and a 712-line test-suite split with
  mise task-graph rewiring — on the epic's critical path gating 0172 and 0174.

#### Minor

- 🔵 **Scope**: Context claims the parallel bash implementation is removed;
  Technical Notes says it survives and the story adds a third implementation.
- 🔵 **Completeness**: The `config-detect.sh` fold-in is an undecided deliverable
  not tracked in Open Questions where the story's other decisions carry defaults.
- 🔵 **Completeness**: The Open Question on where the 27 in-process cases live
  carries no default, unlike its two siblings, yet feeds the floor criterion.
- 🔵 **Testability**: Missing-binary parity cases have no trigger once the
  adapters stop spawning `jj`/`git` — and the `systemMessage` sibling key's only
  stated producer disappears with them.
- 🔵 **Testability**: No criterion verifies the sub-binary registration or
  dependency-policy outcomes the Requirements make load-bearing.
- 🔵 **Testability**: "The skill still renders" is not verified by the static
  frontmatter/permission lints named as its instrument.
- 🔵 **Dependency**: No ordering relationship recorded with 0168, the preceding
  epic phase touching the same visualiser-shaped registration surface.
- 🔵 **Dependency**: `gix`, `jj-lib` and the `deny.toml` licence exception are
  absent from Dependencies despite being the story's only third-party couplings.
- 🔵 **Dependency**: Frontmatter `relates_to` omits 0125, 0182 and 0183, which
  the body names.
- 🔵 **Dependency**: Implied ordering within the story's own workstreams is not
  captured — notably that the shell guard's decisions must be captured as
  fixtures *before* the rewrite, and that the `test:integration:hooks` launcher
  edge must land before the parity suite is repointed.
- 🔵 **Clarity**: The end state of `hooks/config-detect.sh` is indeterminate
  across two criteria.
- 🔵 **Clarity**: "A golden envelope fixture per hook type" under-counts — the
  adjacent criterion mandates four shapes (SessionStart with/without
  `systemMessage`, PreToolUse deny, PreToolUse warn-only).
- 🔵 **Clarity**: "The hooks suite floor … adjusted" names no target state.
- 🔵 **Clarity**: The skill call sites are written as bare relative paths, unlike
  every other invocation in the item, and would not match the permission rule
  the same criterion mandates.
- 🔵 **Clarity**: "see Technical Notes" points at the wrong section — the
  argument-splitting resolution lives in "Notes from 0167".
- 🔵 **Clarity**: "missing-binary diagnostic" does not say which binary, and
  "`!`-injection count … at 42" does not say what it counts or which guard
  enforces it.

#### Suggestions

- 🔵 **Completeness**: Assumptions is a single bullet for a story that adopts two
  new libraries and pins a criterion to a host-specific measurement.
- 🔵 **Completeness**: 0183 and ADR-0053 appear in References but are explained
  nowhere in the body.
- 🔵 **Testability**: The `noexec` criterion names no mechanism for producing a
  non-executable cache directory on CI.
- 🔵 **Dependency**: The `blocked_by` edge on 0167 will not clear without an
  out-of-band bookkeeping action that nobody is assigned.
- 🔵 **Clarity**: "The VCS half only" implies a two-way split that Context's
  five-hook, four-concern, three-owner enumeration contradicts.

### Strengths

- ✅ Nearly every factual claim is anchored to a `path:line` reference, resolving
  referents that would otherwise be ambiguous.
- ✅ Scope boundaries are stated explicitly and repeatedly, including what is
  *not* in scope — the config half, migrate-discoverability,
  `launcher-link-refresh.sh`, `vcs-common.sh`, and 0125 which it explicitly does
  not close.
- ✅ The PreToolUse envelope criterion is exceptionally unambiguous: it names the
  exact JSON shape, states why `"allow"` and `"ask"` are both wrong for the
  colocated case, and flags the user-visible consequence.
- ✅ Deliberate behavioural departures are declared as changes rather than
  smuggled in under a "port", so behavioural risk is visible.
- ✅ Prior research is visibly folded back: the unverified latency assumption is
  replaced by a measured baseline, and the `gix`/`jj-lib` feasibility and
  sub-binary naming collision are recorded as resolved.
- ✅ Each `Blocked by` entry carries rationale *and* live status, distinguishing
  "code landed" from "work item still marked ready".
- ✅ The previously-open `skills/vcs/commit` ownership question was resolved into
  the story rather than left dangling, so `vcs status`/`vcs log` now ship with a
  consumer in the same increment.
- ✅ The design space is bounded — "full taxonomy" is defined inline as
  reproducing the shell's set, explicitly not adding classifications.
- ✅ Open Questions distinguishes resolved from open with dated strikethrough, so
  decision state is readable without cross-referencing the research.
- ✅ Criteria are honest about where coverage is new rather than inherited — the
  guard has no existing suite, and AC1 states how much of the 42-case detect
  suite actually repoints (~11).

### Recommended Changes

1. **Add a distribution requirement and matching criterion** (addresses: the
   four-lens distribution theme) — cover sub-binary registration end-to-end
   (`DISPATCHED_SUBBINARIES`, `_SUBBINARY_MANIFESTS`, workspace members,
   `package.description`, `.gitignore`, `manifest.example.json`, cross-compile
   staging), plus `cargo deny` green with the `uluru` exception and a committed
   `Cargo.lock`. Add 0165 to Dependencies, or state explicitly that the pipeline
   is already token-parameterised and only `validate_dispatch_coherence` needs
   generalising.

2. **Add a capture-before-delete step** (addresses: instruments-deleted theme) —
   state that the status/log goldens, the guard decision table and the latency
   baseline are captured from the shell **before** those files are removed, and
   committed as fixtures. Name the normalisation rule for volatile fields
   (commit ids, dates, absolute paths). State that the detect goldens are frozen
   at their shell-produced content, with any regeneration justified case by case.

3. **Fix the `classify_checkout` arm list** (addresses: arm-enumeration theme) —
   state the arm list once, including `colocated` and `nested-*`, and have both
   the Requirements definition and the fixture criterion refer to that single
   list. Replace "at least one ambiguous checkout" with the named case: a
   colocated checkout nested inside another repository must classify as
   `colocated`.

4. **Make the probe criterion observable** (addresses: probe-absence finding) —
   replace the post-hoc file check with a behavioural one, e.g. point
   `ACCELERATOR_CACHE_DIR` at a directory that is executable but not writable and
   assert a warm invocation still succeeds.

5. **Resolve the two optional deliverables** (addresses: indeterminate-done-state
   theme) — decide the `config-detect.sh` fold-in in or out and pin the removal
   count; pin the hooks floor to a value per resolution of the in-process-cases
   question ("2 if they remain a `hooks/test-*.sh` suite, else 1; never 0").

6. **Name a single normative parity reference per subcommand, and dispose of the
   `.git`-as-file defect** (addresses: parity-reference theme) — state whether
   the colocated misclassification is preserved as parity or corrected here.

7. **Record the external and newly-created dependencies** (addresses: the
   dependency findings) — Claude Code's hook schema with the v2.1.144 floor and
   the fields relied upon; `gix`/`jj-lib` with the version-lockstep constraint
   and the `deny.toml` exception; 0183's constraint; and add 0125, 0182, 0183 to
   frontmatter `relates_to`.

8. **Action the two hand-offs rather than observing them** (addresses: 0172 and
   0174 findings) — add `blocked_by: 0169` to 0172 and
   `hooks/migrate-discoverability.sh` to its source list; and either extend
   0174's scope to the `vcs-common.sh` residue or state plainly that the residue
   is currently unowned debt.

9. **Disambiguate `accelerator-vcs` and "launcher"** (addresses: identifier
   overload) — reserve `accelerator-vcs` for the Cargo package, "the
   `vcs`/`vcs-adapters` crates" for the crates, "bootstrap" for `bin/accelerator`
   and "launcher binary" for the cached Rust binary.

10. **Scope the zero-spawn assertion** (addresses: zero-spawn contradiction) —
    either forbid fallbacks on the four paths outright, or assert "zero spawns
    other than from fallbacks declared in ⟨named list⟩, each covered by its own
    test".

11. **Reconsider the split** (addresses: the two scope majors) — the author has
    elected to keep the `probe_dir` fix in scope; recorded here as the reviewer's
    independent position that (a) the bootstrap fix and (b) the subdomain-versus-
    hooks seam are both independently deliverable, and that `review-1`'s scope
    disposition predates four of the story's current workstreams.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: A dense, heavily anchored work item whose clarity weaknesses
concentrate in identifiers carrying more than one referent (`accelerator-vcs`,
"launcher") and contradictions between Requirements/Acceptance Criteria and
Technical Notes about what the deliverable actually is — most sharply the
`classify_checkout` taxonomy, enumerated one way as a definition and a different
way as a load-bearing constraint.

**Findings**: 5 major, 6 minor, 1 suggestion — identifier overload (high);
taxonomy enumerated two ways (high); parity reference stated two ways with the
`.git`-as-file defect undispositioned (high); zero-spawn versus permitted
fallback (medium); status/log parity against deleted scripts (medium);
`config-detect.sh` end state; envelope fixture under-count; floor with no target;
bare relative call-site paths; mis-pointed cross-reference; undefined terms;
"VCS half" implying a two-way split.

### Completeness

**Summary**: Structurally complete and unusually dense — every expected section
present and substantively populated, frontmatter fully valid, and fifteen
criteria each naming a verification method with `path:line` anchors. The main gap
is distribution: several steps the story calls mandatory live only in Technical
Notes, absent from both Requirements and Acceptance Criteria.

**Findings**: 1 major, 2 minor, 2 suggestions — mandatory implementation steps
only in Technical Notes (medium); the optional fold-in untracked in Open
Questions (high); one Open Question with no default (medium); sparse Assumptions;
unexplained References entries.

### Dependency

**Summary**: An unusually well-developed Dependencies section — each blocker
carries rationale and live status, each Blocks entry explains the coupling
mechanism, and it deliberately records a related item (0125) it does not close.
The gaps are at the edges: the external Claude Code schema, the newly-created
0165 coupling, two unilaterally-asserted hand-offs, and a frontmatter graph that
lags the prose.

**Findings**: 5 major, 4 minor, 1 suggestion — Claude Code schema unrecorded
(high); 0165 not a recorded blocker (medium); 0172 hand-off unactioned (high);
`vcs-common.sh` residue handed to a story that does not claim it (high); 0183
absent from Dependencies (medium); no 0168 ordering; `gix`/`jj-lib` absent;
`relates_to` omits three couplings; internal ordering uncaptured; the 0167
`blocked_by` edge needs an out-of-band action.

### Scope

**Summary**: Thematically coherent with unusually explicit boundaries, but the
story has accreted materially since its 2026-07-20 review: it now also carries a
bootstrap performance/trust change, an adapter re-implementation on two new
heavyweight dependencies, a first-of-kind sub-binary registration, a skill
rewrite and a 712-line test suite split — spanning all four toolchains under one
`kind: story`.

**Findings**: 2 major, 3 minor — `probe_dir` independently deliverable (medium);
four toolchains / five deliverables, and the bundle `review-1` approved is not
the bundle that exists now (medium); two optional deliverables leaving scope
indeterminate; Context contradicting Technical Notes on whether the parallel bash
implementation is removed; first-of-kind distribution work unrepresented.

### Testability

**Summary**: Unusually strong on verification framing — most criteria name a
concrete instrument and several replace earlier unverified assumptions with
measured baselines. Weaknesses concentrate in criteria whose instrument is
deleted by the same change, unobservable after the fact, optional, or enumerated
over the wrong set.

**Findings**: 6 major, 4 minor, 1 suggestion — arm enumeration omits the
load-bearing pair (high); status/log goldens against deleted comparators with no
normalisation (high); latency criterion lacking statistic/tolerance/mechanism
(high); final criterion optional and undetermined so it cannot fail (high); guard
decision matrix unenumerated and contradicting the story's own notes (medium);
detect gate satisfiable by re-baselining (medium); missing-binary cases with no
trigger; no criterion for sub-binary registration; "skill still renders"
mismatched to its instrument; `noexec` mechanism unnamed.


## Re-Review (Pass 2) — 2026-07-31

**Verdict:** REVISE

All five lenses re-run against the revised work item. **14 of 19 pass-1 majors are
resolved**, and completeness and dependency cleared their pass-1 finding sets
entirely. The verdict does not move, for one reason: testability raised the
review's first **critical**, and it is a genuine safety hole rather than a
documentation gap.

### Previously Identified Issues

**Resolved (14 majors, 17 minors/suggestions)**

- 🟡 **Clarity**: identifier overload (`accelerator-vcs`, "launcher") — Resolved
  by the new Terminology section.
- 🟡 **Clarity**: taxonomy enumerated two incompatible ways — Resolved by the
  authoritative arm list stated once in Requirements.
- 🟡 **Clarity**: parity reference stated two ways; `.git`-as-file undispositioned
  — Resolved by "Normative parity reference, per subcommand" plus "Declared
  behavioural changes".
- 🟡 **Clarity**: zero-spawn versus permitted fallback — Resolved; the fallback
  allowance is withdrawn and the assertion is unconditional.
- 🟡 **Clarity**: status/log parity against deleted scripts — Resolved by the
  capture-before-delete criterion.
- 🟡 **Completeness**: mandatory steps only in Technical Notes — Resolved by the
  distribution requirement and its criterion.
- 🟡 **Dependency**: Claude Code schema unrecorded — Resolved (External systems).
- 🟡 **Dependency**: 0165 not a recorded dependency — Resolved.
- 🟡 **Dependency**: `vcs-common.sh` residue misattributed to 0174 — Resolved;
  now stated as unowned debt.
- 🟡 **Dependency**: 0172 hand-off unactioned — Resolved by the reciprocal
  hand-off criterion.
- 🟡 **Testability**: arm enumeration omitted the load-bearing pair — Resolved.
- 🟡 **Testability**: final criterion optional and undetermined — Resolved; floor
  pinned, fold-in decided.
- 🟡 **Scope**: two optional deliverables — Resolved.
- 🟡 **Scope**: distribution work unrepresented — Resolved.
- 🔵 Also resolved: Context/Technical Notes contradiction on the parallel bash
  implementation; `config-detect.sh` fold-in decided and tracked; Open Question
  defaults; sparse Assumptions; unexplained 0183/ADR-0053 references; envelope
  fixture under-count; bare relative call-site paths; mis-pointed "see Technical
  Notes"; missing-binary cases retired; sub-binary registration criterion; "skill
  still renders" instrument split; `noexec` mechanism named; `gix`/`jj-lib` and
  `relates_to` gaps; 0168 ordering; intra-story sequencing; 0167 bookkeeping
  action.

**Partially resolved — the fix landed but introduced a sharper successor**

- 🟡 **Testability**: probe-absence criterion — the residue check was replaced by
  a behavioural one, but the replacement **passes vacuously as root**, which
  bypasses directory write permissions.
- 🟡 **Testability + Clarity**: detect goldens — "frozen" is now stated, but
  "byte-compared", "frozen" and "whitespace is the only permitted difference"
  cannot all hold, and Technical Notes confirms the renderers differ in exactly
  that respect.
- 🟡 **Testability + Clarity**: latency criterion — gained a statistic (median of
  20) but is now simultaneously "recorded, not gated" and a hard ≤ 35 ms
  threshold.
- 🟡 **Testability**: guard decision table — now two-axis, but defined only by
  reference to a capture whose row set is unpinned, so a two-row table satisfies
  both criteria.
- 🔵 **Clarity**: the Terminology section fixed five terms but "probe" (three
  senses) and "wrapper" (undefined) escaped it.

**Still present (author decision on record)**

- 🟡 **Scope**: the bootstrap `probe_dir` fix remains an independently
  deliverable workstream — now reinforced by the story's own Sequencing
  Constraints, which state it "can land first". Author has elected to keep the
  bundle; scope suggests recording *why* the coupling is preferred to sequencing,
  since the story's own text argues the other way.
- 🔵 **Scope**: story kind stretched by a 16-criterion, four-toolchain scope —
  downgraded from major to minor in this pass.

### New Issues Introduced

- 🔴 **Testability**: **no criterion verifies the envelope is honoured at the
  v2.1.144 floor.** The criteria check that the CLI *emits* the shapes and that
  `hooks.json` *registers* the commands; both pass while Claude Code discards the
  output. If `permissionDecision` postdates the floor, the migrated guard silently
  stops blocking git commands in pure-jj repos — a safety regression against the
  shell it replaces, undetectable by the story's own test set.
- 🟡 **Dependency**: the release-artefact host is an unnamed external dependency
  on the hot path. `vcs guard` is now a *fetched* sub-binary on a hook that fires
  on every Bash call; a failed first-use fetch lands in PreToolUse where non-zero
  is a blocking error, with no fail-open posture stated.
- 🟡 **Testability**: the zero-spawn assertion's named mechanism cannot observe
  spawns originating *inside* `gix`/`jj-lib` — the most likely violation source.
  A `PATH`-stub black-box test would.
- 🟡 **Completeness**: cargo-pup is the one enforcement gate missing from the
  distribution criterion, despite a pup rule being the reason for the package
  naming requirement.
- 🟡 **Completeness**: the floor-verification risk has no Open Question or
  criterion, while the lesser `args` question has both.
- 🟡 **Dependency**: 0170/0171/0173 inherit this story's sub-binary pathfinding
  but are not listed as Blocks.
- 🟡 **Dependency**: the 0183 hand-off is acknowledged but excluded from the
  reciprocal-recording criterion that covers 0172.
- 🟡 **Clarity**: VCS adapter failure diagnostics are routed to a stdout
  `systemMessage` in Requirements and to a discarded `--fail-safe` stderr write in
  Dependencies; and the `hooks.json` command string for `vcs detect` is never
  given verbatim, so whether it carries `--fail-safe` is unstated.
- 🔵 **Scope**: the hand-off criterion makes this story's completion contingent on
  re-scoping 0172 and 0174 — work it does not own. It can *raise* those hand-offs,
  not own their resolution.
- 🔵 Further minors: `hooks.json` command strings not verbatim; `vcs detect`'s
  plain rendering unspecified; shared hook-envelope module home undecided;
  normalisation rule deferred; status/log fixtures omit colocated, jj-secondary
  and no-repository; "set to 2" versus "stays at 2"; residue-owner clause
  circular; counts asserted without enumeration; `EXPECTED_INJECTION_SKILLS` unit
  ambiguous; `CLAUDE_PLUGIN_ROOT` versus `ACCELERATOR_PLUGIN_ROOT` boundary
  unstated; "VCS hooks only" contradicted by the `config-detect.sh` deletion.

### Assessment

The revision worked: the story is materially better specified, and the classes of
defect that dominated pass 1 — vacuous criteria, deleted comparators, ambiguous
referents, unrecorded couplings — are largely gone. What remains splits into three
groups.

**Must fix before implementation (1):** the floor-verification critical. Until it
is known whether `permissionDecision` and top-level `systemMessage` are honoured
at v2.1.144, the guard migration risks trading a working shell guard for a
silently inert Rust one. This is empirically checkable in minutes and should gate
planning, not implementation.

**Should fix (7 majors):** the fail-open posture for a fetched guard on the hot
path; the root-bypass and gix-internal-spawn holes; the golden comparison rule;
the guard table's row set; the latency criterion's gated-or-not ambiguity; and
cargo-pup in the distribution gate. Each is a one-to-three-line edit.

**Accepted or deferred (2):** the scope pair. The author has decided to keep the
bundle intact, and that decision is recorded in Drafting Notes.

Two observations worth carrying forward. First, three of this pass's majors are
holes in criteria written *in response to* pass 1 — the probe test, the frozen
goldens and the guard table each fixed the stated problem while opening a
narrower one, which is the normal shape of tightening a specification and argues
for one more short pass rather than a broad rewrite. Second, both of the most
serious findings (the critical, and the fetch-failure posture) are consequences
of decisions taken on 2026-07-30 — the `permissionDecision` envelope and the
sub-binary — and neither was re-examined for failure modes when it was taken.


## Re-Review (Pass 3) — 2026-07-31

**Verdict:** REVISE

All five lenses re-run. The pass-2 **critical is resolved** — the floor-verification
criterion landed and is accepted. Dependency and completeness both converged
sharply (3→1 and 2→1 majors). But the aggregate did not move: **14 majors**,
against 15-plus-a-critical in pass 2 and 19 in pass 1.

### Previously Identified Issues

- 🔴 **Testability**: envelope unverified at the v2.1.144 floor — **Resolved**
  (Sequencing Constraint 1, an Open Question with a default, and an acceptance
  criterion requiring a real denied Bash call).
- 🟡 **Dependency**: release-artefact host unnamed / no fail-open posture —
  **Resolved**; `--fail-safe` on all three registrations with its own criterion.
- 🟡 **Dependency**: 0170/0171/0173 not in Blocks — **Resolved**.
- 🟡 **Dependency**: 0183 hand-off unactioned — **Resolved** (raised by criterion).
- 🟡 **Completeness**: floor risk untracked; cargo-pup missing from the
  distribution gate — **Both resolved**.
- 🟡 **Testability**: root-bypass on the probe test — **Resolved** (hard-fails
  rather than skips when `id -u` = 0).
- 🟡 **Clarity**: "probe"/"wrapper" overloaded — **Resolved** via Terminology.
- 🟡 **Scope**: hand-off criterion overreach — **Resolved** ("raising these is in
  scope; re-scoping those items is not").
- 🟡 **Testability + Clarity**: goldens byte-vs-whitespace — **Partially
  resolved**; `jq -S` canonicalisation added, but "must equal" and "value changes
  require justification" still contradict.
- 🟡 **Testability**: guard table row set — **Partially resolved**; the blocked
  subcommands are enumerated, but "representative allowed commands" and
  "compound commands" are not, so the claimed derivability covers 1 of 4 axes.
- 🟡 **Testability**: zero-spawn mechanism — **Partially resolved**; the
  black-box `PATH`-stub test is stronger, but an absolute-path spawn
  (`/usr/bin/git`) evades it — the exact `gix`-internal case it targets.
- 🟡 **Testability + Clarity**: latency gated-or-not — **Partially resolved**; the
  gate is now explicit, but a hard 35 ms conflicts with a baseline captured on
  whatever host runs acceptance.
- 🟡 **Scope**: the exec-probe fix and the story's span — **Still present**, with
  the bundling rationale now recorded as requested.

### New Issues Introduced

Nine of the fourteen majors are consequences of the pass-2 edits themselves.

- 🟡 **Clarity**: **direct contradiction within one edit** — "SessionStart
  failures emit nothing" (fail-open Requirement) versus adapter failure
  "produces a `systemMessage` … on stdout" (`--format=hook` Requirement), with a
  golden required for the latter.
- 🟡 **Clarity**: the `.git`-as-file correction is scoped to the guard, but the
  same `-d` test is inlined in `vcs-detect.sh`, `vcs-status.sh` and
  `vcs-log.sh`, which will share one classification port — so "Everything else is
  parity" instructs preserving a bug on paths that cannot preserve it.
- 🟡 **Scope + Completeness + Testability** (unanimous): the `corpus-adapters` /
  `CommandProbe` migration was declared in scope in pass 2 — scope says it does
  not belong (orthogonal, own risk profile, nothing in the hooks migration needs
  it); completeness and testability say that if it stays, it has **no acceptance
  criterion**, so `CommandProbe` could survive as a second shell-spawning
  implementation exactly as the Requirement forbids.
- 🟡 **Scope**: the Summary enumerates "three things" the story also lands and
  omits the library swap — the largest of the four.
- 🟡 **Scope**: the floor criterion's fallback branch ("raise the plugin's minimum
  version") admits a plugin-wide compatibility decision into a VCS story.
- 🟡 **Testability**: the fail-open Requirement names three failure classes; only
  the unreachable-host case has a criterion. Nothing verifies a degraded
  SessionStart emits zero bytes, or the unreadable-repository case.
- 🟡 **Testability**: the exclusive four-rule masking list likely misses
  abbreviated SHAs, jj change IDs and jj timestamp formats — and because the rule
  is closed, the correct fix is barred.
- 🟡 **Clarity**: "its existing parity suite" has two candidate referents and
  names no path, while "parity suite" denotes three different artefacts.
- 🟡 **Dependency**: 0182 is filed under "Completed dependencies" while its own
  entry records `in-progress`.

### Assessment

**The critical is gone and that matters.** The story no longer risks trading a
working shell guard for a silently inert Rust one without noticing.

But the major count across three passes is 19 → 15 → 14, and this pass's own
composition explains why it is not falling: **nine of fourteen majors were
created by the previous pass's fixes.** The pattern is consistent and now
well-evidenced:

- Each fix in one section contradicts a section written at a different time
  (fail-open versus diagnostic routing; the `.git` correction versus the shared
  port; "three things" versus the added fourth).
- Each tightening opens a narrower version of the same hole (probe: residue →
  root → …; zero-spawn: port → `PATH` stubs → absolute path; goldens:
  re-baseline → whitespace → mask coverage).
- Each "this isn't specified" finding has been closed by **absorbing more work**
  rather than pushing it out — which is why scope alone has moved 2 → 1 → **3**
  while everything else converged.

That is not a document-quality problem any further editing pass will fix. It is
the signature of a work item carrying more than one artefact can hold
consistently: six workstreams, four toolchains, twenty criteria, on the epic's
critical path.

**Recommendation.** Stop iterating the text. Take the two structural actions
first, then re-review:

1. **Revert the `corpus-adapters` expansion** — three lenses agree. Put that path
   explicitly out of scope, name who converges it, and let the story build over
   the existing adapter for that consumer.
2. **Split at the two seams the work item itself names** — the exec-probe
   bootstrap fix (Sequencing Constraint 5 already concedes it can land first) and
   the `gix`/`jj-lib` adapter swap (which stands on the existing
   `corpus-adapters` parity suite and has independent value: it dissolves 0125's
   lexical-fallback rationale). 0169 then keeps subdomain + hooks + sub-binary
   registration.

The remaining specification defects — the emit-nothing contradiction, the `.git`
correction's scope, the masking list, the guard table's unenumerated axes, the
absolute-path spawn hole, the latency baseline, the 42-case partition — are all
one-to-three-line fixes, and most would land in whichever child story owns them.
Fixing them in the current single document is what has produced three passes of
this shape.


## Re-Review (Pass 4) — 2026-07-31

**Verdict:** REVISE

All five lenses re-run. Three stalled on their first attempt (a stream watchdog
timeout while reading the referenced documents) and were relaunched with reading
scoped to the work item alone — so clarity, completeness and scope assessed the
document without the research and review artefacts the first batch consulted.
Noted for fairness; it narrows scope's ability to check claims against the epic.

**14 majors, no criticals — identical to pass 3.**

| Pass | Majors | Criticals |
| --- | --- | --- |
| 1 | 19 | 0 |
| 2 | 15 | 1 |
| 3 | 14 | 0 |
| 4 | **14** | 0 |

### Per-lens trajectory

| Lens | P1 | P2 | P3 | P4 |
| --- | --- | --- | --- | --- |
| completeness | 1 | 2 | 1 | **0** |
| scope | 2 | 1 | 3 | **2** |
| clarity | 5 | 4 | 4 | **3** |
| dependency | 5 | 3 | 1 | **3** |
| testability | 6 | 5+1🔴 | 5 | **6** |

Completeness has converged to zero. Scope's remaining two are its standing
split recommendation, unchanged in substance across all four passes. Clarity,
dependency and testability have not converged.

### The decisive measurement

**Eleven of pass 4's fourteen majors are defects in pass 3's fixes**, not
pre-existing problems surfacing. Only three are standing issues (scope's two
structural findings, and testability's observation that a reachable host serving
a manifest without `accelerator-vcs` is untested).

Representative examples, all introduced by the edits made to close pass 3:

- The 42-case partition was rewritten to be checkable and now **does not
  reconcile**: 27 in-process + "the remaining 15" in three disjoint buckets, plus
  a fourth disposition — either four buckets, or 27+15+1 = 43. Flagged
  independently by testability and clarity.
- The emit-nothing contradiction was fixed and became a **three-way**
  contradiction: Requirements say adapter failure emits a `systemMessage`-bearing
  object and that "emits nothing" never applies; the verifying criterion says the
  same case "writes exactly zero bytes"; the unreadable-repository criterion
  leaves the shape open, destabilising the "five in total" golden count.
- The guard decision table was enumerated to make its row count derivable; the
  trailing "× 4 repo modes" multiplier is ambiguous in scope, and the two
  readings differ by roughly 2×. The count is still not stated as a number.
- The `.git`-as-file correction was extended to all four subcommands and is
  **tested for one** — while the detect criterion, tightened in the same edit to
  "a value difference fails the criterion", now conflicts with the correction
  biting on that path.
- The `corpus-adapters` revert landed cleanly and has **no criterion asserting
  it**, so the boundary is an intention rather than a checked fact.

### Assessment

Three passes of editing have moved the major count 19 → 15 → 14 → 14. The
severity ceiling did fall — the pass-2 critical is gone and stayed gone — and the
sections describing *what the work is* have settled: completeness is at zero and
scope reports no section-to-section drift, crediting the adapter-swap boundary as
"exemplary deliberate scope limitation".

What has not settled is the verification and coupling surface. Each tightening
pass produces a comparable number of new inconsistencies, because the document
must hold six workstreams, twelve requirements and twenty-plus criteria mutually
consistent — and every fix in one section has more surface to contradict than the
last. That is the mechanism scope has been describing since pass 1, now visible
as a measurement rather than an opinion.

**Recommendation: stop editing this document. Split it.** Scope's pass-4
decomposition is concrete and uses this story's own revert as its evidence — with
`CommandProbe` now explicitly retained, the hooks can migrate over the existing
adapter and swap afterwards behind a port that "already allows the swap without
touching the domain":

1. subdomain + hooks migration + skill repoint
2. sub-binary distribution registration (unblocks 0170/0171/0173 independently)
3. the `gix`/`jj-lib` library swap
4. (already carved out) the exec-probe bootstrap fix

The eleven fix-defects are individually trivial. They are not worth another pass
against the current document, because the evidence says a fifth pass produces
about as many again. They should be fixed inside whichever child story owns them,
where each document has a surface small enough to keep consistent.
