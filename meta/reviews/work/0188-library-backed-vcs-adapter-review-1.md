---
type: work-item-review
id: "0188-library-backed-vcs-adapter-review-1"
title: "Work Item Review: Library-Backed VCS Adapter over gix and jj-lib"
date: "2026-08-01T21:16:19+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0136"
target: "work-item:0188"
work_item_id: "0188"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 4
tags: [rust, vcs, dependencies]
last_updated: "2026-08-02T14:52:24+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Library-Backed VCS Adapter over gix and jj-lib

**Verdict:** APPROVE (author decision, 2026-08-02 — see Approval below;
the four lens passes below recorded REVISE and are left unaltered)

0188 is a strong, evidence-backed work item: every section is present and
substantively filled, the split rationale is a genuine orthogonality argument
rather than an arbitrary size cut, feasibility is measured (dated, versioned)
rather than assumed, and the non-goals — `CommandProbe` retained, no consumer
converged, 0125 not closed — are each pinned by an acceptance criterion. Its
weakness is concentrated in the surface it owes downstream: the taxonomy query
set is described by intent rather than as an enumerated contract with a defined
home and an oracle, and three of the story's substantive criteria (the
`detection.rs` pass, the boundary-containment fixture, the zero-spawn
assertion) can each be reported as passing without exercising what they claim
to verify. No critical findings; ten major findings across four lenses take
this past the two-major REVISE threshold.

### Cross-Cutting Themes

- **The taxonomy query surface is a contract with no shape** (flagged by:
  clarity, completeness, dependency, scope, testability) — all five lenses
  landed on the same seam. Requirements say "extend the adapters with the
  queries the `classify_checkout` taxonomy needs, so 0169 can build
  classification on them", but the work item never says *where* those queries
  live (port methods, which would contradict "the domain crate is untouched",
  vs inherent adapter methods, which 0169's domain-side classifier could not
  call through a port), *what* the full set is (0169 separately requires
  `GIT_DIR` handling that 0188's list omits), or *what answers are correct*
  (AC2 demands "available and unit-tested" with no expected values). This is
  the single highest-value revision: fixing it closes four of the ten majors.

- **Criteria that can pass without exercising what they verify** (flagged by:
  testability, clarity) — three of the eight ACs have a vacuous-pass route.
  AC1 leans on "the existing `detection.rs` suite", whose entry point the
  referenced research records as hard-wired to `CommandProbe`. AC4's boundary
  fixture is undermined by the Technical Notes' own suggestion to set
  `GIT_CEILING_DIRECTORIES` in fixtures — the environment would stop discovery,
  not the adapter. AC3's "any path that cannot be shadowed is recorded" has no
  floor, and on a SIP-protected macOS the normal case is that none of the
  absolute paths can be shadowed.

- **Requirements stated without a matching definition of done** (flagged by:
  completeness, testability, clarity, scope) — the bolded "avoid
  `Workspace::load`" requirement has no criterion at all, though its sibling
  bolded requirement does. The `cli/pup.ron` rule is conditional on an
  undefined "warrants", and its criterion is satisfied by adding no rule. The
  `deny.toml` inline-comment obligation and the `GIT_CEILING_DIRECTORIES`
  fixture practice sit in advisory prose ("should", "consider") outside both
  Requirements and ACs.

- **Version pinning is asymmetric** (flagged by: clarity, completeness,
  testability) — `gix` gets an explicit pin, a rationale and a direct lockfile
  assertion; `jj-lib`, on whose 0.43 loader internals the entire design rests,
  is never named in Requirements or ACs at all. The `gix` pin is defined
  *relative* to jj-lib ("the version `jj-lib` depends on (currently 0.85)"), so
  an unpinned jj-lib silently decays the gix pin too.

- **Downstream consequences observed but unowned** (flagged by: dependency,
  scope) — 0188 is the change that dissolves 0125's lexical-fallback rationale,
  but 0169 carries the acceptance criterion to append the hand-off note.
  Similarly, 0188 builds the zero-spawn harness that 0185 plans to extend,
  while 0185's text still attributes it to 0169.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: "The domain crate is untouched" conflicts with adding the
  taxonomy queries 0169 must build on
  **Location**: Requirements
  The ports live in `cli/vcs`, so new query methods on them would touch the
  domain crate; inherent adapter methods would not be callable through a port
  by 0169's classifier. AC2's "available" never says available through what, so
  the two stories can be built to incompatible designs.

- 🟡 **Testability**: Taxonomy-query criterion states no expected values,
  fixtures or exposed surface
  **Location**: Acceptance Criteria (second bullet)
  Five queries are named with no oracle for any of them and no defined fixture
  set. Any test that calls each query and asserts anything satisfies the
  criterion — providing no evidence the queries answer *correctly*, which is
  the entire basis on which 0169 builds `classify_checkout`.

- 🟡 **Dependency / Scope**: `GIT_DIR`/common-dir override handling is required
  by 0169 but absent from the delivered query surface
  **Location**: Requirements (taxonomy queries)
  0169's Requirements name "the submodule, bare and `GIT_DIR` handling that
  feeds them"; 0188's enumerated list omits it. `GIT_DIR`/`GIT_COMMON_DIR` are
  environment overrides sitting inside 0188's discovery-bounding requirement,
  not in 0169's domain logic. Two sequential stories claim an overlapping query
  surface defined by intent, so a query can fall between them or be built twice
  with divergent semantics.

- 🟡 **Testability**: "Passes the existing detection.rs suite" does not specify
  how the new adapter is exercised
  **Location**: Acceptance Criteria (first bullet)
  The referenced research records `vcs_adapters::facts(start)` as hard-wiring
  `MarkerWalkRoot` + `CommandProbe::new()` "with no injection variant". Running
  the suite unchanged verifies the retained subprocess adapter; the criterion
  could be truthfully reported as passing with none of the new code exercised.

- 🟡 **Clarity / Testability**: Bounded `gix` discovery is neither defined as a
  rule nor verifiable as written
  **Location**: Requirements (bound gix discovery) / Acceptance Criteria
  (fourth bullet)
  "The intended boundary" is never defined — nearest marker, jj workspace root,
  or caller-supplied ceiling are all readings. AC4 then asserts "`gix::discover`
  cannot escape a workspace boundary", which contradicts the requirement's own
  verified finding that it *does*; the intended subject is the adapter's bounded
  use. Worse, if the boundary fixture inherits the `GIT_CEILING_DIRECTORIES`
  that Technical Notes recommends, discovery is stopped by the environment and
  the criterion passes against an adapter that does no bounding at all.

- 🟡 **Completeness / Testability**: The "avoid `Workspace::load`" requirement
  has no acceptance criterion
  **Location**: Requirements (fourth bullet) / Acceptance Criteria
  Its sibling bolded requirement gets a criterion, so the omission looks
  unintentional. The zero-spawn criterion's empty `JJ_CONFIG`/`HOME` is partial
  coverage at best, and only if that run traverses the path in question. A
  regression to `Workspace::load` — the trap the research spent a probe
  identifying — would surface as a runtime panic on a user's machine.

- 🟡 **Dependency / Testability**: The in-process performance and binary-size
  cost is never measured, yet 0169's gate depends on it
  **Location**: Summary / Context / Acceptance Criteria
  0169 carries a hard gate (`G ≤ 1.1 × B`, ≈38.6 ms) that its own Dependencies
  already flags as at risk from a ~41 ms warm bootstrap. Whether it passes is
  largely set by this story's in-process discovery cost and by the sub-binary
  size the two new trees produce. If loading `gix`/`jj-lib` state costs more
  than the spawns it replaces, that is discovered in 0169 *after* the trees,
  the licence exception and the API bet have landed — the opposite of the
  "rolled back on its own terms" property the split was made for.

- 🟡 **Testability**: Unbounded escape hatch in the zero-spawn criterion
  **Location**: Acceptance Criteria (third bullet)
  "Any path that cannot be shadowed in the test environment is recorded in
  Validation Results" has no floor. On macOS with SIP a read-only `/usr/bin` is
  the normal case, so in the limit every absolute path goes unshadowed, leaving
  only the PATH stub — reducing the story's central safety claim to the weak
  assertion it was written to strengthen, while still reporting as passed.

- 🟡 **Dependency**: Toolchain preconditions — the real `jj`/`git` binaries and
  the rustc floor — are uncaptured upstream dependencies
  **Location**: Assumptions / Dependencies (External systems)
  Every criterion is validated against repositories written by the *installed*
  `jj` CLI, whose on-disk format must be readable by the pinned `jj-lib 0.43` —
  a version-coherence coupling between a toolchain pin and a crate pin that is
  never stated. Separately, two large pre-1.0 trees impose an MSRV the repo's
  pinned Rust toolchain must satisfy; the Assumptions record `cargo deny` and
  musl-static results but say nothing about the compiler floor.

#### Minor

- 🔵 **Clarity**: "Its parity suite" has three candidate referents
  **Location**: Acceptance Criteria (seventh bullet)
  The possessive could refer to `CommandProbe`, `cli/corpus-adapters`, or the
  story. 0169 warns that two distinct things in this codebase are called
  "parity" surfaces (the hooks parity gate and the `corpus-adapters` metadata
  parity suite), so the ambiguity is not academic.

- 🔵 **Clarity**: The single-`gix`-version criterion names neither the checking
  mechanism nor the scope of "one version"
  **Location**: Acceptance Criteria (fifth bullet)
  Who asserts it (Rust test, `tasks/` lint, CI step, human observation) and
  whether "one `gix` version" covers the facade crate only or every `gix-*`
  crate are both open. A check against the facade alone would pass while
  duplicate `gix-hash`/`gix-object` trees persist.

- 🔵 **Clarity / Completeness / Testability**: No requirement states which
  `jj-lib` version is adopted, yet "the pinned version" is referenced
  **Location**: Requirements (dependency policy) / Assumptions
  The gix pin is defined relative to an unstated jj-lib version, so "currently
  0.85" decays the moment jj-lib moves. A caret range resolving forward at any
  `cargo update` silently invalidates the loader-internals evidence with no
  failing check.

- 🔵 **Clarity**: "The adapters" (plural) and "the library-backed adapter"
  (singular) refer to an unstated set
  **Location**: Requirements / Acceptance Criteria
  Unclear whether one adapter type spans both libraries or a pair does, and
  whether "the adapters" includes the retained `CommandProbe` — which would
  have to grow the same queries if they become port methods, conflicting with
  "changes no existing consumer".

- 🔵 **Clarity**: Shell-side terms `classify_checkout`, `BOUNDARY` and
  `JJ_PARENT` are used without definition or a path reference
  **Location**: Requirements / Technical Notes
  0169 carries a Terminology section covering this vocabulary; 0188 does not,
  and References does not point at `scripts/vcs-common.sh`. A reviewer cannot
  check the loader-parity claim without independently locating an unreferenced
  bash function.

- 🔵 **Clarity / Testability / Scope / Completeness**: "If its import surface
  warrants one" gives no criterion for whether the cargo-pup rule is required
  **Location**: Requirements (final bullet)
  No statement of what warrants it or who decides, and the matching criterion
  ("any new rule is demonstrably non-vacuous") passes vacuously if no rule is
  added. A required architectural guard could be silently dropped with no
  grounds to object.

- 🔵 **Clarity**: "Run each query against fixture repos" does not identify the
  set of queries
  **Location**: Acceptance Criteria (third bullet)
  Could mean the two port traits' methods, the five taxonomy extensions, or
  both. The story's strongest guarantee is asserted over an unbounded set.

- 🔵 **Testability**: Zero-spawn test asserts success, not correct answers
  **Location**: Acceptance Criteria (third bullet)
  "Every query still succeeds" — an adapter that silently degrades under the
  stubbed environment (returning `None`/empty facts) writes no marker and
  "succeeds". The run most likely to expose hidden external-state dependence
  has the weakest oracle.

- 🔵 **Completeness**: Two obligations sit only in advisory prose
  **Location**: Dependencies / Technical Notes
  The `deny.toml` inline comment citing this story ("should") is the only
  durable in-repo record of why a workspace-wide licence exception exists;
  `GIT_CEILING_DIRECTORIES` in fixtures is a "consider". Neither is in
  Requirements or ACs.

- 🔵 **Completeness**: Frontmatter does not record the research document or the
  extraction from 0169
  **Location**: Frontmatter: relates_to
  References cites the 2026-07-29 research §9 as the source of every empirical
  claim, and names 0169 as the origin; `relates_to` lists only 0125.
  Relationship-graph traversal will not connect this story to its evidence base.

- 🔵 **Dependency**: Shared repo-wide Rust artefacts are contended with in-flight
  sibling stories, with no ordering captured
  **Location**: Requirements (dependency policy / pup.ron) / Dependencies
  `cli/deny.toml`, `cli/pup.ron` and `cli/Cargo.lock` are also touched by 0168
  (folds the visualiser into the workspace) and 0187. The "exactly one `gix`
  version" invariant can be silently broken by whichever branch lands second.

- 🔵 **Dependency**: 0185 depends on the zero-spawn harness this story builds,
  but the record still attributes it to 0169
  **Location**: Dependencies (Blocks: 0185)
  0185's criteria say "the zero-spawn black-box assertion introduced by 0169" —
  stale after the split. If 0188 builds the stubs as a private test helper,
  0185 rebuilds them and the shadow list drifts between suites.

- 🔵 **Dependency / Scope**: The 0125 hand-off is identified but has no owner
  **Location**: Dependencies (Related: 0125)
  0169 carries the AC requiring a dated note appended to 0125, but 0188 is now
  the item that dissolves the rationale and lands first. Between 0188 landing
  and 0169 completing, 0125 carries a rationale the codebase no longer supports.

#### Suggestions

- 🔵 **Clarity**: "Passes the existing detection.rs suite" is ambiguous about
  whether the suite may change
  **Location**: Acceptance Criteria (first bullet)
  "Existing" reads as "unchanged", but exercising a second adapter necessarily
  requires parameterising the hard-wired entry point.

- 🔵 **Clarity**: "Scope's argument for the split" names an actor the reader
  cannot identify
  **Location**: Context
  Capitalised and possessive, it reads as a person; it means the scope review
  lens of 0169 review-2, which is not in References.

- 🔵 **Clarity**: "The swap" in Context sits uneasily with "adds an adapter; it
  does not remove one"
  **Location**: Summary / Context
  A reader skimming for scope could conclude the subprocess adapter is being
  replaced here rather than in 0185.

- 🔵 **Clarity**: "Repo-wide" describes a workspace-scoped file
  **Location**: Summary / Dependencies
  `cli/deny.toml` governs the `cli/` Rust workspace, not the repository; the
  blast radius is overstated to a reviewer assessing the policy change.

- 🔵 **Completeness**: No requirement states how a caller selects the new
  implementation
  **Location**: Requirements
  AC7 pins existing consumers to `CommandProbe`, but nothing states the
  composition surface 0169 builds on — risking unplanned wiring work moving
  into 0169 at pickup.

- 🔵 **Dependency**: Ongoing advisory and version-drift surface of the new trees
  has no named owner
  **Location**: Dependencies (External systems)
  A future RustSec advisory anywhere in the `gix`/`jj-lib` closure fails the
  repo-wide check for every unrelated change, and the pin rule makes any
  `jj-lib` bump a coordinated two-crate bump.

### Strengths

- ✅ Every section a story needs is present and substantively populated — the
  only "pending" markers are deliberate evidence slots in Validation Results
  tied to specific criteria, so those outputs cannot be quietly omitted.
- ✅ The scope boundary is expressed as a single memorable invariant — "the
  existing `CommandProbe` is **retained**, not replaced" / "this story adds an
  adapter; it does not remove one" — restated consistently across Summary, ACs
  and Dependencies, and pinned as an observable by AC7.
- ✅ The split rationale is a genuine orthogonality argument, not a size cut:
  dependency adoption fails at build level, hook-envelope changes fail
  user-visibly, and bundled neither could be accepted or rolled back alone.
- ✅ Feasibility is measured, dated and versioned (2026-07-29, `gix 0.85` +
  `jj-lib 0.43`), with `cargo deny` verdicts, the musl static-ELF result and the
  loader-parity probe recorded — and Assumptions cleanly separate measured fact
  from the residual bet on jj-lib's self-declared unstable API.
- ✅ AC3 is an exemplary anti-evasion criterion: it names the stub mechanism,
  neutralises the config environment, and explicitly closes the absolute-path
  escape a PATH-only assertion would miss.
- ✅ AC5 states not just the assertion but *why* a direct one is needed (the
  duplicate-version policy is warn-level), making it resistant to being
  "satisfied" by an existing check that cannot fail.
- ✅ AC6 demands the pup rule be demonstrably non-vacuous — a mutation-style
  check most work items omit.
- ✅ Both library traps are stated as named, bolded requirements with the
  observed behaviour, the verification date and the concrete failing case, so
  the reader understands why each constraint exists rather than trusting it.
- ✅ Dependencies are fully populated across every axis, each with a stated
  reason rather than a bare identifier, and the edges are bidirectionally
  consistent with 0169 and 0185 — the split left no dangling reference.
- ✅ The coupling *between* the two external dependencies is captured as an
  explicit requirement (pin `gix` to jj-lib's version) and backed by a direct
  lockfile criterion.

### Recommended Changes

1. **Fix the taxonomy query surface — enumerate it, home it, and give it an
   oracle** (addresses: "domain crate untouched" conflict; taxonomy criterion
   states no expected values; `GIT_DIR` absent from the surface; "the adapters"
   plural/singular; "each query" unidentified)
   This is the highest-leverage revision — it closes four majors and three
   minors. In Requirements, replace "the queries the `classify_checkout`
   taxonomy needs" with a numbered list that *is* the delivery contract, adding
   `GIT_DIR`/`GIT_COMMON_DIR` and the ceiling-vs-environment precedence (or
   stating explicitly that env-override handling is 0169's). State where they
   live — new methods on `RepoRoot`/`VcsProbe`, a new port trait, or inherent
   adapter methods — and reconcile that with "the domain crate is untouched".
   Then give AC2 an oracle: Technical Notes already maps every query to a shell
   reference, so "for each query, against the colocated / secondary / worktree /
   submodule / bare fixtures, the adapter's answer equals the named
   `vcs-common.sh` function's answer" is available almost for free. Reuse that
   named set in AC3's "each query". Consider a matching reword in 0169 so it
   composes the taxonomy from this fixed list rather than "extending the
   crates".

2. **Close the three vacuous-pass routes in the acceptance criteria**
   (addresses: `detection.rs` suite doesn't exercise the new adapter; boundary
   test passes on fixture hygiene; zero-spawn escape hatch has no floor)
   For AC1, state the injection seam: "each of the eight enumerated cases runs
   against *both* `CommandProbe` and the library-backed adapter and produces
   identical `RepoFacts`" — which also buys a differential parity check. For
   AC4, name the environment ("with `GIT_CEILING_DIRECTORIES` unset or set above
   the parent repository…") and fix the subject so it describes the adapter's
   bounded discovery, not `gix::discover` itself; a paired negative case showing
   unbounded `gix::discover` *does* escape the same fixture would make it
   airtight. For AC3, add an unwaivable floor: the PATH stub is always in force
   *and* at least one non-PATH mechanism (all listed absolute paths shadowed, or
   a process-level spawn counter) proves absence — so an unshadowable path
   degrades the evidence without removing it.

3. **Define "the intended boundary" as a rule** (addresses: bounded discovery
   neither defined nor verifiable)
   State it normatively — e.g. "discovery stops at the first ancestor containing
   a `.jj` or `.git` marker; ceilings are set from that path" — since the
   boundary rule determines behaviour in exactly the topologies this story
   exists to serve, and the single AC4 fixture pins only one of them.

4. **Give the `Workspace::load` requirement a criterion** (addresses: bolded
   requirement with no definition of done)
   A source-level guard plus environment coverage is cheap: "no
`cli/vcs-adapters`
   source path references `jj_lib::workspace::Workspace::load`, and every jj
   query succeeds with `HOME`, `JJ_CONFIG` and `XDG_CONFIG_HOME` at empty temp
   dirs" — the second half reusing AC3's environment.

5. **Record the cost this story hands to 0169** (addresses: performance
   rationale never measured; 0169 latency and binary-size budget uncaptured)
   Add a *recording* criterion, not a threshold — "median of 20 invocations of
   each library-backed query against a fixture repo on one host, alongside the
   same query via `CommandProbe`, plus the sub-binary size delta, recorded in
   Validation Results with host and OS" — and name 0169's `G ≤ 1.1 × B` gate in
   the Blocks entry. This preserves the story's independent-rollback property
   without importing 0169's gate.

6. **Make the version pinning symmetric** (addresses: jj-lib version never
   named; single-gix criterion's mechanism and scope; toolchain preconditions)
   State the jj-lib version and pin strictness in the dependency-policy
   requirement, and extend AC5 to assert both: "`cli/Cargo.lock` resolves no
   `gix` *or* `gix-*` package at more than one version, and `jj-lib` at 0.43",
   naming the mechanism (committed test/lint, not a manual observation). Add an
   upstream-dependency bullet for the installed `jj` CLI ↔ `jj-lib` on-disk
   format coherence and the MSRV of both crates against the pinned toolchain.

7. **Resolve the conditional pup rule and promote the advisory obligations**
   (addresses: "if its import surface warrants one"; obligations in advisory
   prose)
   Decide now — state the rule and its import constraint, or state that no rule
   is needed and why (the existing `^vcs($|::)` pattern not reaching
   `vcs_adapters` is the argument). Promote the `deny.toml` inline comment to a
   "must" in Requirements or AC6, and resolve `GIT_CEILING_DIRECTORIES` from
   "consider" to a yes/no — noting its interaction with Change 2's AC4 fix.

8. **Assign the downstream hand-offs** (addresses: 0125 hand-off unowned; 0185
   harness attribution stale)
   Either add an AC here requiring the dated note appended to 0125's
   Dependencies when the adapter lands, or state in the Related entry that 0169
   retains that obligation. Extend the 0185 Blocks entry to name the zero-spawn
   harness as a consumed deliverable and say whether it is a shared fixture;
   append a dated correction to 0185 repointing the attribution from 0169.

9. **Tidy the referents and the frontmatter** (addresses: "Its parity suite";
   `classify_checkout`/`BOUNDARY`/`JJ_PARENT` undefined; "Scope's argument";
   "the swap"; "repo-wide"; relates_to)
   Name the parity suite by path; gloss the shell vocabulary on first use or add
   `scripts/vcs-common.sh` to References; name the review document behind
   "Scope's argument" and add it to References; avoid "swap" in this item; say
   "the `cli/` workspace's dependency policy"; and add the research document to
   `relates_to` with a `derived_from` for 0169.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: 0188 is a dense, well-referenced work item whose central scope
boundary — "this story adds an adapter; it does not remove one" — is stated
crisply and repeated consistently across Summary, Acceptance Criteria and
Dependencies. Clarity weakens around the new surface the story delivers: it is
unclear whether the taxonomy queries are new ports (contradicting "the domain
crate is untouched"), whether "the adapters" includes the retained
`CommandProbe`, and what "the intended boundary" for bounded `gix` discovery
actually is. A handful of ambiguous referents ("Its parity suite", "each
query", "the pinned version", "Scope's argument") and shell-side terms used
without definition (`classify_checkout`, `BOUNDARY`, `JJ_PARENT`) would each
make a reader stop and ask the author.

**Strengths**:

- The story's scope boundary is expressed as a single memorable invariant — "The
  existing `CommandProbe` is **retained**, not replaced" / "this story adds an
  adapter; it does not remove one" — and is restated consistently in Summary,
  Acceptance Criteria and Dependencies, so the reader cannot mistake the intent.
- The two library traps are stated as named, bolded requirements with the
  observed behaviour, the verification date and the concrete failing case
  (`gix::discover` returning the parent repo's `.git` from inside
  `workspaces/build-system`), so the reader understands why each constraint
  exists rather than having to trust it.
- The zero-spawn acceptance criterion names the exact evasion it defends against
  (an absolute-path spawn bypassing a `PATH`-only stub) and enumerates the
  specific paths to shadow, leaving no room for a weaker interpretation of "zero
  spawns".
- Every dependency edge is stated with its relationship in words rather than as
  a bare number — 0179 "delivered the ports this story implements", 0185
  "converges `corpus-adapters` onto them and deletes `CommandProbe`" — so the
  reader never has to open another work item to know why an edge exists.
- The Assumptions section explicitly separates what was measured (with a date
  and the exact crate versions probed) from what remains a genuine bet (jj-lib's
  unstable API), which is unusually honest labelling.

**Findings**:

**🟡 major (high confidence) — Requirements — "The domain crate is untouched"
conflicts with adding the taxonomy queries 0169 must build on**

The first requirement states "Implement the `vcs` crate's `RepoRoot` and
`VcsProbe` ports … The domain crate is untouched", while the second requires
"Extend the adapters with the queries the `classify_checkout` taxonomy needs, so
0169 can build classification on them". Since the ports live in the domain crate
(`cli/vcs/src/lib.rs`), a reader cannot tell whether these new queries are
additional port methods (which would touch the domain crate), inherent methods
on the concrete adapter types (which 0169's domain-side classifier could not
call through a port), or something else — and Acceptance Criterion 2's "The
taxonomy queries 0169 needs are **available**" does not say available through
what.

*Impact*: The implementer must guess the shape of the interface this story
delivers, and 0169 — which is blocked on it — cannot know whether it inherits
ports or must define them itself, so the two stories can be built to
incompatible designs.

*Suggestion*: State explicitly where the new queries live (new methods on the
existing `RepoRoot`/`VcsProbe` traits, a new port trait in `cli/vcs`, or
inherent
adapter methods) and reconcile that with the "domain crate is untouched" claim —
noting, if inherent methods are intended, that 0169 owns the port definitions
that expose them.

**🟡 major (high confidence) — Requirements / Acceptance Criteria — "The
intended boundary" for bounded gix discovery is never defined, and AC4 restates
it as a property of `gix::discover` itself**

The requirement "Bound `gix`'s discovery explicitly" concludes "Discovery must
stop at the intended boundary rather than trusting the library's default walk",
but "the intended boundary" is never defined — candidate readings include the
nearest `.jj` or `.git` marker, the jj workspace root specifically, or a
caller-supplied ceiling. The corresponding acceptance criterion then asserts
"`gix::discover` cannot escape a workspace boundary", which literally
contradicts the requirement's own verified finding that `gix::discover` *does*
walk up past a jj workspace boundary; the intended subject is presumably the
adapter's bounded use of it, not the library function.

*Impact*: The boundary rule determines behaviour in exactly the topologies this
story exists to serve — nested git-in-jj, jj-in-git, worktrees and submodules —
so two implementers could produce different, both-defensible results, and the
single acceptance fixture (a jj secondary workspace inside a git repository)
pins only one of them.

*Suggestion*: State the bounding rule as a rule (e.g. "discovery stops at the
first ancestor containing a `.jj` or `.git` marker; `GIT_CEILING_DIRECTORIES`-
equivalent ceilings are set from that path") and rephrase the criterion so its
subject is the adapter's discovery rather than `gix::discover`.

**🔵 minor (high confidence) — Acceptance Criteria — "Its parity suite" has
three candidate referents**

The criterion reads "`CommandProbe` still exists and `cli/corpus-adapters` still
resolves through it — this story adds an adapter and changes no existing
consumer. Its parity suite passes unchanged." The possessive "Its" could refer
to `CommandProbe`, to `cli/corpus-adapters`, or to the story; the immediately
preceding pronoun "it" refers to `CommandProbe`, which is the one candidate that
has no parity suite of its own. The blocking story 0169 additionally warns that
two distinct things in this codebase are called "parity" surfaces (the hooks
parity gate and the `corpus-adapters` metadata parity suite), so the ambiguity
is not academic.

*Impact*: An implementer could assert the wrong suite — or the unrelated hooks
parity gate — and believe the criterion is satisfied.

*Suggestion*: Name the suite by path (e.g.
"`cli/corpus-adapters/tests/parity.rs`
passes unchanged") instead of using a possessive pronoun.

**🔵 minor (high confidence) — Acceptance Criteria — The single-`gix`-version
criterion names neither the checking mechanism nor the scope of "one version"**

The criterion "Exactly **one** `gix` version resolves in `cli/Cargo.lock`
(asserted directly — the repo's duplicate-version policy is warn-level, so a
drifted pin would otherwise pass silently)" leaves two things open: who or what
performs the assertion (a Rust test, a Python lint task under `tasks/`, a CI
step, or a human recording the result), and whether "one `gix` version" means
one version of the `gix` facade crate only or a single version of every `gix-*`
crate in the resolved graph. The Requirements phrase the goal as "one `gix`
graph, not two", which suggests the whole family, while Validation Results
carries "**`gix` version resolved in `Cargo.lock`** — _pending_", which suggests
a one-off manual observation.

*Impact*: A check written against the `gix` facade alone would pass while
duplicate `gix-hash`/`gix-object` trees persist, defeating the stated purpose of
the pin; and a manually recorded observation would not prevent later drift.

*Suggestion*: State the mechanism ("a committed test/lint asserts…") and the
scope ("…that no `gix` or `gix-*` package appears at more than one version in
`cli/Cargo.lock`").

**🔵 minor (medium confidence) — Requirements / Acceptance Criteria — "The
adapters" (plural) and "the library-backed adapter" (singular) refer to an
unstated set**

The Requirements say "Extend **the adapters** with the queries the
`classify_checkout` taxonomy needs", while the Acceptance Criteria consistently
say "**The** library-backed **adapter**" (singular) implements the ports and is
the subject of the zero-spawn assertion. It is therefore unclear whether this
story delivers one adapter type spanning both `gix` and `jj-lib` or a pair of
them, and — more consequentially — whether "the adapters" includes the retained
`CommandProbe`, which would have to grow the same queries if they become port
methods.

*Impact*: If `CommandProbe` is in scope for the extension, that conflicts with
the story's repeated "changes no existing consumer / parity suite passes
unchanged" posture; if it is out of scope, the port surface cannot be widened,
which constrains the design decision in the first finding.

*Suggestion*: Fix the count and the membership in one place — e.g. name the new
type(s) and add "`CommandProbe` gains no new methods" (or the converse) to the
Requirements.

**🔵 minor (high confidence) — Requirements / Assumptions — No requirement
states which `jj-lib` version is adopted, yet "the pinned version" is
referenced**

The dependency-policy requirement pins only `gix`, and does so *relative* to
jj-lib: "pin `gix` to the version `jj-lib` depends on (currently 0.85)". No
requirement states which `jj-lib` version is adopted or whether it is pinned at
all, yet the Assumptions say "`jj-lib`'s loader API remains stable across **the
pinned version**. Verified against 0.43" — a phrase with no antecedent, which
could equally be read as referring to the `gix` pin.

*Impact*: The gix pin is defined in terms of an unstated jj-lib version, so
"currently 0.85" silently decays the moment jj-lib moves; and since the story
rests on jj-lib's explicitly unstable loader internals, the reader cannot tell
whether an exact pin is a requirement or a caretaker's discretion.

*Suggestion*: State the jj-lib version and its pin strictness as a requirement
(e.g. "depend on `jj-lib` 0.43 with an exact pin"), and make the Assumptions
sentence name which crate's pin it means.

**🔵 minor (high confidence) — Requirements / Technical Notes — Shell-side terms
`classify_checkout`, `BOUNDARY` and `JJ_PARENT` are used without definition or a
path reference**

The Requirements hinge on "the queries the `classify_checkout` taxonomy needs"
and the Technical Notes assert that jj-lib's loader answers "match
`classify_checkout` exactly: `workspace_root()` equals the shell's `BOUNDARY`,
and `repo_path()` minus `/.jj/repo` equals `JJ_PARENT`". None of the three terms
is defined or given a file reference in this work item: `classify_checkout` is a
bash function in `scripts/vcs-common.sh`, and `BOUNDARY`/`JJ_PARENT` are keys of
the `KEY=VALUE` record it emits. The blocking story 0169 carries a Terminology
section that covers this vocabulary; 0188 does not, and its References list does
not point at the shell source.

*Impact*: A reviewer assessing whether the adapter's semantics are correct — the
whole point of this story — cannot check the claim without independently
locating an unreferenced shell function and inferring the meaning of two
undefined record keys.

*Suggestion*: Add a one-line gloss with a path on first use (e.g.
"`classify_checkout` (`scripts/vcs-common.sh:177-280`), whose record fields
`BOUNDARY` (the checkout root) and `JJ_PARENT` (the main repo directory) …"), or
add `scripts/vcs-common.sh` to References.

**🔵 minor (high confidence) — Requirements — "If its import surface warrants
one" gives no criterion for whether the cargo-pup rule is required**

The final requirement reads "Add a `cli/pup.ron` rule for the new adapter module
**if its import surface warrants one**", with no statement of what would warrant
it or who decides. The matching acceptance criterion is likewise conditional
("**any** new rule is demonstrably non-vacuous"), so the work item can be
satisfied with or without the rule and never says which outcome is correct.

*Impact*: A required architectural guard could be silently dropped as "not
warranted", and no reviewer would have grounds to object.

*Suggestion*: Replace the vague conditional with a decision rule and its owner —
e.g. "add a rule denying the adapter module any import outside `std`, `gix`,
`jj_lib`, `kernel::Error` and `crate::`" — or state plainly that no rule is
required and why.

**🔵 minor (medium confidence) — Acceptance Criteria — "Run each query against
fixture repos" does not identify the set of queries**

The zero-spawn criterion instructs "run **each query** against fixture repos …
Assert no marker is written and **every query** still succeeds", but the work
item never defines the set of "queries". It could mean the two port traits'
methods (`RepoRoot::discover`/`repository_root`, `VcsProbe::kind`/`revision`),
the five taxonomy extensions listed in the preceding criterion, or both — and
the word "query" is used loosely for both surfaces elsewhere in the Requirements
and Technical Notes.

*Impact*: The strongest guarantee in the story — zero subprocess spawns — is
asserted over an unbounded set, so a partial implementation could satisfy the
letter of the criterion while a spawning code path goes untested.

*Suggestion*: Enumerate the queries once (or point at the enumeration in the
taxonomy criterion plus the port methods) and use that named set in both
criteria.

**🔵 suggestion (medium confidence) — Acceptance Criteria — "Passes the existing
detection.rs suite" is ambiguous about whether the suite may change**

The first criterion requires the new adapter to pass "**the existing**
`cli/vcs-adapters/tests/detection.rs` suite". "Existing" reads as "unchanged",
but the referenced research records that the current entry point hard-wires
`MarkerWalkRoot` + `CommandProbe::new()` with no injection variant — so
exercising a second adapter through that suite necessarily requires
parameterising it.

*Impact*: An implementer may either believe the suite is off-limits and write a
parallel duplicate, or modify it and be unsure whether that breaches the
criterion.

*Suggestion*: Say what may change — e.g. "the cases in `detection.rs` are
parameterised over both adapters and all listed cases pass for the
library-backed one".

**🔵 suggestion (medium confidence) — Context — "Scope's argument for the split"
names an actor the reader cannot identify**

Context states "**Scope's** argument for the split is that the two changes carry
very different risk profiles". Capitalised and possessive, "Scope" reads as a
person or a named section, but it appears to mean the scope review lens from the
review that drove the split (0169's Drafting Notes attribute the split to
"review-2 pass 4"). That review document is not in this work item's References.
The same sentence's "the two changes" also has no antecedent until the following
clause supplies them.

*Impact*: A reader outside the review workflow cannot tell whose judgement is
being cited or verify it, which weakens the stated justification for the split.

*Suggestion*: Name the source explicitly ("the scope lens of
`meta/reviews/work/0169-…-review-2.md`") and add it to References, and name the
two changes before referring to them collectively.

**🔵 suggestion (medium confidence) — Summary / Context — "The swap" in Context
sits uneasily with "adds an adapter; it does not remove one"**

The Summary is emphatic that nothing is replaced — "The existing `CommandProbe`
is **retained**, not replaced … This story adds an adapter; it does not remove
one" — but Context then says "the **swap** is an adapter change by
construction", using a term the sibling story 0169 also uses for this work ("the
`gix`/`jj-lib` adapter swap"). The intended meaning is presumably that swapping
*implementations* would not touch the domain, but as written the two sections
appear to describe different operations.

*Impact*: A reader skimming for scope could conclude the subprocess adapter is
being replaced here rather than in 0185.

*Suggestion*: Rephrase the Context sentence in terms of the property being
claimed (e.g. "a library-backed implementation is an adapter-level change by
construction") and avoid "swap" in this work item.

**🔵 suggestion (medium confidence) — Summary / Dependencies — "Repo-wide"
describes a workspace-scoped file, and the inline-comment obligation is only a
"should"**

The Summary calls the licence exception "a repo-wide `cli/deny.toml` licence
exception" and Dependencies repeats "a repo-wide policy change", although the
file is scoped to the `cli/` Rust workspace rather than the repository.
Dependencies then adds that the exception "**should** carry an inline comment
citing this story so a future dependency audit can find the justification" — an
obligation stated in a non-normative modal, placed outside Requirements, and not
covered by any acceptance criterion.

*Impact*: The blast radius of the policy change is overstated to a reviewer
assessing it, and the audit-trail comment reads as optional advice, so it may
plausibly be omitted.

*Suggestion*: Say "the `cli/` workspace's dependency policy" (or state that
`cli/deny.toml` governs all Rust in the repo, if that is the case), and promote
the inline comment to a Requirement with "must" if it is intended to be
delivered.

### Completeness

**Summary**: Work item 0188 is a densely populated story: every section a story
needs is present and substantively filled — Summary, Context, Requirements,
eight specific Acceptance Criteria, Dependencies, Assumptions, Technical Notes,
Validation Results and References — with frontmatter complete and valid (`kind:
story`, `status: ready`, `priority: high`, parent/blocked_by/blocks all set).
Its strongest completeness feature is that Requirements, Acceptance Criteria and
Validation Results are wired to each other: the two criteria demanding recorded
evidence have pre-declared slots in Validation Results. The residual gaps are
coverage rather than presence — one explicit Requirement has no matching
criterion, one Requirement is left conditional with no Open Questions section to
hold the undecided call, the `jj-lib` version the whole feasibility case rests
on is never named in Requirements or AC, and two obligations sit only in
advisory prose.

**Strengths**:

- Every section a story requires is present and substantively populated — no
  empty or placeholder sections; the only 'pending' markers are in Validation
  Results, where they are deliberate evidence slots tied to specific acceptance
  criteria.
- The Summary states both what is added and, unusually, what is explicitly not
  changed ('The existing CommandProbe is retained, not replaced'), pre-empting
  the most likely misreading of an adapter-swap story.
- Context explains three distinct forces behind the work — the current
  subprocess implementation, the differing risk profiles that justified
  extracting this from 0169, and the dissolution of 0125's lexical-fallback
  rationale — rather than restating the Summary.
- Assumptions cleanly separate measured fact from residual risk, dating the
  feasibility measurement (2026-07-29) and naming the exact remaining bet
  (jj-lib's self-declared unstable API).
- Acceptance criteria are unusually concrete for an infrastructure story — AC3
  enumerates the specific evasion its zero-spawn assertion must resist
  (absolute-path spawns at three named locations) and requires unshadowable
  paths to be recorded rather than silently skipped.
- The Dependencies section is fully populated across every axis — blocked_by,
  blocks, external systems, related and parent — each with a stated reason
  rather than a bare identifier.

**Findings**:

**🟡 major (medium confidence) — Acceptance Criteria**

Work item 0188 (library-backed VCS adapter over `gix` and `jj-lib`) carries an
explicit Requirement to avoid `jj_lib::workspace::Workspace::load` on detection
paths and use `DefaultWorkspaceLoaderFactory` instead, but none of the eight
acceptance criteria references it. Every other Requirement bullet has a matching
criterion — ports → AC1, taxonomy queries → AC2, bounded `gix` discovery → AC4,
dependency policy → AC5/AC6 — so this is the one stated obligation with no
definition of done.

*Impact*: One of the two hard-won traps the story exists to encode is absent
from the acceptance set, so an implementation could satisfy every criterion
while taking the fragile settings-dependent path.

*Suggestion*: Add a criterion asserting the detection path constructs no
`UserSettings` and calls no `Workspace::load`. AC3 already runs queries with
`JJ_CONFIG` and `HOME` pointed at empty temp dirs, which would likely surface
the failure, so making that coverage explicit should be cheap.

**🔵 minor (high confidence) — Requirements**

In work item 0188 the dependency-policy Requirement pins `gix` explicitly ("to
the version `jj-lib` depends on (currently 0.85)") and AC5 asserts exactly one
`gix` version resolves in `cli/Cargo.lock`, but neither Requirements nor
Acceptance Criteria ever names the `jj-lib` version to adopt. Version 0.43
appears only in Assumptions (as what feasibility was measured against) and in
Technical Notes source citations.

*Impact*: The story's entire evidence base — the `cargo deny` verdicts, the musl
static-link result, and the loader-API-matches-`classify_checkout` probe — is
measured against jj-lib 0.43, yet nothing in the requirements or the definition
of done ties the implementation to it.

*Suggestion*: State the `jj-lib` version in the dependency-policy Requirement
alongside the `gix` pin, and extend AC5 to assert the resolved `jj-lib` version
as well as the single-`gix` graph.

**🔵 minor (medium confidence) — Requirements**

The final Requirement in work item 0188 says to add a `cli/pup.ron` rule for the
new adapter module "if its import surface warrants one", with no stated
criterion for what warrants one and no decision recorded anywhere in the
document. The work item has no Open Questions section where such an undecided
item would normally live (the sibling story 0169 it was extracted from does have
one).

*Impact*: The implementer inherits an unstated architectural-policy call with
nowhere in the document recording the default, and AC6's "any new rule is
demonstrably non-vacuous" is trivially satisfied by adding no rule at all.

*Suggestion*: Either resolve it in Requirements ("add a rule denying X", or "no
new rule is needed because the existing `^vcs($|::)` pattern does not reach
`vcs_adapters`"), or add an Open Questions section that records the question
with a stated default if unresolved.

**🔵 minor (medium confidence) — Dependencies**

Work item 0188 places two obligations in commentary sections rather than in
Requirements or Acceptance Criteria: the Dependencies section says the
`cli/deny.toml` licence exception "should carry an inline comment citing this
story so a future dependency audit can find the justification", and Technical
Notes says to "Consider `GIT_CEILING_DIRECTORIES` in fixtures".

*Impact*: Obligations stated only in advisory prose are easy to drop during
implementation and nothing in the definition of done catches their omission —
the audit comment in particular is the only durable in-repo record of why a
repo-wide licence-policy exception exists.

*Suggestion*: Promote the inline-comment obligation into the dependency-policy
Requirement bullet or into AC6, and resolve the `GIT_CEILING_DIRECTORIES`
"consider" into a yes/no statement in Requirements.

**🔵 minor (medium confidence) — Frontmatter: relates_to**

Work item 0188's References section cites
`meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md` §9
as the source of every empirical claim in the story (the `cargo deny` verdicts,
the musl static-link result, the jj-lib loader probe, the `uluru` finding) and
names 0169 as the item it was extracted from — but the frontmatter records
neither. `relates_to` lists only `work-item:0125`, and there is no
`derived_from` or `source` field. The sibling story 0169 does carry that
research document in its `relates_to`.

*Impact*: Relationship-graph traversal of the corpus will not connect this story
to its evidence base, so a future reader auditing the licence exception or a
`jj-lib` version bump has to rediscover the research through prose.

*Suggestion*: Add
`codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration` to
`relates_to`, and record the extraction from 0169 in a `derived_from` (or
`source`) field.

**🔵 suggestion (medium confidence) — Requirements**

Work item 0188 requires implementing the `RepoRoot` and `VcsProbe` ports
"alongside the existing `CommandProbe`", and AC7 pins that existing consumers
keep resolving through `CommandProbe` — but no Requirement or criterion states
how a caller selects the new library-backed implementation, which is precisely
what the blocked story 0169 has to build on.

*Impact*: The downstream consumer may discover at pickup that the enabling story
delivered types with no composition surface, moving unplanned wiring work into
0169.

*Suggestion*: Add a sentence to Requirements naming the selection surface this
story delivers — e.g. a composition helper taking the library-backed adapters,
or an explicit "public constructors only; consumer wiring belongs to 0169".

### Dependency

**Summary**: 0188 captures its principal couplings well: the upstream 0179 edge
is named and marked done, both downstream consumers (0169, 0185) are listed as
Blocks with a one-line reason each, and the External systems entry names gix and
jj-lib, their crates.io origin, the pre-1.0 unstable-API risk, and the repo-wide
`cli/deny.toml` licence-policy change including the requirement to leave a
citation comment for future audits. The gaps are at the edges of the contract it
owes downstream and at the environment it depends on: the story's runtime and
binary-size cost is on the critical path of 0169's numeric latency gate but is
nowhere captured; the enumerated query surface omits
`GIT_DIR`/`--git-common-dir` override handling that 0169's taxonomy explicitly
declares it needs; and the toolchain preconditions (the real `jj`/`git` binaries
that build the fixtures, and the rustc floor the two new trees impose) are
absent. Secondary gaps are shared-artefact contention with in-flight sibling
stories on `cli/Cargo.lock`/`deny.toml`/`pup.ron`, and an ownerless hand-off
note to 0125.

**Strengths**:

- The Blocks/Blocked-by edges are bidirectionally consistent with the referenced
  items: 0169's frontmatter lists 0188 in `blocked_by`, and 0185's Dependencies
  explicitly records that its edge was repointed from 0169 to 0188 when the
  split happened — the split did not leave a dangling reference.
- The External systems entry is unusually complete for a library-adoption story:
  it names both crates, their registry, the pre-1.0 API-break risk with the
  specific reason (the design leans on loader internals), and the repo-wide
  `cli/deny.toml` licence exception as a policy change, plus a requirement that
  the exception carry an inline comment citing this story so a future dependency
  audit can find the justification.
- The coupling *between* the two external dependencies is captured as an
  explicit requirement — pin `gix` to the version `jj-lib` depends on so one
  graph resolves — and is backed by a direct acceptance criterion asserting
  exactly one `gix` version in `cli/Cargo.lock`, with the reason the repo's own
  duplicate-version policy cannot be relied on (warn-level).
- The Summary and Requirements state explicitly that `CommandProbe` is retained
  and no existing consumer changes, which protects the downstream
  `corpus-adapters` consumer and makes the boundary with 0185 unambiguous rather
  than implicit.
- Feasibility is recorded as measured with a date (2026-07-29) against named
  versions, so the dependency bet is evidence-backed rather than assumed, and
  the one licence rejection is carried into the Requirements as concrete work.

**Findings**:

**🟡 major (medium confidence) — Dependencies (Blocks: 0169) / Acceptance
Criteria — Downstream 0169 latency and binary-size budget depends on this story
but is not captured**

This story (0188, the library-backed `gix`/`jj-lib` VCS adapter) records its
downstream edge to 0169 as only "builds the subdomain's classification on these
adapters", but 0169 carries a hard numeric acceptance gate — warm-call latency
`G ≤ 1.1 × B` — and a hand-off note from 0186 already warns the budget is at
risk (~41 ms warm bootstrap against a ≈38.6 ms gate, before a sub-binary exec
and verify). Whether 0169 can pass that gate is largely determined by this
story's in-process discovery cost and by the sub-binary size the two new
dependency trees produce (which also feeds the fetch/verify path), yet 0188 has
no performance or size criterion and its Dependencies section does not name the
constraint it must satisfy.

*Impact*: 0188 can be accepted as "green" and then cause 0169 to fail acceptance
on a criterion 0188 has no visibility of, forcing a reopen of the dependency
work after it has been reviewed and merged as a self-contained change.

*Suggestion*: Name 0169's latency gate and the sub-binary size implication in
the Blocks entry, and add an acceptance criterion here that records the measured
per-query cost of the library-backed adapter (and the resulting binary-size
delta) in Validation Results, so 0169's budget can be checked before this story
closes.

**🟡 major (medium confidence) — Requirements (taxonomy queries / bound gix
discovery) — `GIT_DIR`/common-dir override handling is required by 0169 but
absent from the delivered query surface**

0188 commits to "extend the adapters with the queries the `classify_checkout`
taxonomy needs, so 0169 can build classification on them" and enumerates them:
bare check, worktree detection, superproject/submodule resolution, jj workspace
root, main-vs-secondary. 0169's own Requirements state the taxonomy needs "the
submodule, bare and **`GIT_DIR`** handling that feeds them", and
`GIT_DIR`/`GIT_COMMON_DIR` are environment overrides that sit inside this
story's discovery-bounding requirement (gix honours discovery-affecting
environment by default), not inside 0169's domain logic.

*Impact*: 0169 starts on the assumption that the adapter answers correctly under
a `GIT_DIR` override, discovers it does not, and either reopens 0188 or grows an
unplanned adapter change inside the hooks-migration story — exactly the coupling
the split was meant to eliminate.

*Suggestion*: Add `GIT_DIR`/`GIT_COMMON_DIR` (and the ceiling-versus-environment
precedence) to the enumerated query surface in Requirements and to the
taxonomy-query acceptance criterion, or state explicitly in the Blocks entry
that env-override handling is 0169's responsibility so the boundary is agreed in
advance.

**🟡 major (medium confidence) — Assumptions / Dependencies (External systems) —
Toolchain preconditions — the real `jj`/`git` binaries and the rustc floor — are
uncaptured upstream dependencies**

Two environment dependencies this story rests on are never named in
Dependencies. First, the Technical Notes state the `bash-parity` gate means
"needs real `jj`/`git` binaries to build fixtures", so every acceptance
criterion here is validated against repositories written by the *installed* `jj`
CLI, whose on-disk format must be readable by the pinned `jj-lib 0.43` — a
version-coherence coupling between a toolchain pin and a crate pin that is not
stated. Second, adopting two large pre-1.0 dependency trees imposes an MSRV that
the repo's pinned Rust toolchain must satisfy; the Assumptions record `cargo
deny` and musl-static results but say nothing about the compiler floor.

*Impact*: If the installed `jj` writes a repo format `jj-lib 0.43` will not
load, or if either crate's MSRV exceeds the pinned toolchain, the story is
blocked at first build or its fixtures fail in a way that looks like an adapter
defect rather than a version-pin mismatch — and the fix (bumping a shared
toolchain pin) is a repo-wide change with its own blast radius.

*Suggestion*: Add an upstream-dependency bullet naming the installed `jj` CLI
version and the pinned Rust toolchain as preconditions, stating the `jj` CLI ↔
`jj-lib` version-coherence rule and recording the observed MSRV of `gix 0.85` /
`jj-lib 0.43` against the pinned toolchain in Validation Results.

**🔵 minor (medium confidence) — Requirements (dependency policy / pup.ron) /
Dependencies — Shared repo-wide Rust artefacts are contended with in-flight
sibling stories, with no ordering captured**

This story edits three repo-wide artefacts — `cli/deny.toml` (licence
exception), `cli/pup.ron` (a possible new rule) and `cli/Cargo.lock` (which must
be committed in the same change because clippy runs `--locked`) — but
Dependencies records no coupling to the sibling epic-0136 stories that touch the
same workspace concurrently, notably 0168 (folds the visualiser into the `cli/`
workspace, restructuring membership and the lock) and 0187 (generalises the
sub-binary registration surface). The epic's decomposition implies 0168 precedes
0188 by list order only; no explicit ordering constraint exists on either item.

*Impact*: Two in-flight branches adding large dependency trees and restructuring
the same workspace produce non-trivial `Cargo.lock` conflicts and a lock that
must be regenerated rather than merged, and the "exactly one `gix` version"
invariant can be silently broken by whichever branch lands second.

*Suggestion*: Add a sequencing note in Dependencies stating this story's
relationship to 0168/0187 on the shared `cli/` workspace files (or explicitly
that no ordering is required and lock conflicts are regenerated), so whoever
schedules the two knows the contention exists.

**🔵 minor (high confidence) — Dependencies (Blocks: 0185) / Acceptance Criteria
(zero-spawn assertion) — 0185 depends on the zero-spawn test harness this story
builds, but the record still attributes it to 0169**

0188's third acceptance criterion introduces the zero-spawn black-box harness
(marker-writing `git`/`jj` stubs on `PATH` plus shadowed absolute paths, with
`HOME`/`GIT_CONFIG_*`/`JJ_CONFIG` redirected). 0185's acceptance criteria
require
extending "the zero-spawn black-box assertion **introduced by 0169**" to a
`corpus-adapters` metadata read — an attribution left stale by the split. 0188's
own Blocks entry describes the 0185 coupling only as "converges
`corpus-adapters` onto them and deletes `CommandProbe`" and does not mention
that it also hands 0185 a reusable test harness.

*Impact*: 0185 is planned against a harness it believes 0169 owns; if 0188
builds the stub set as a private test helper rather than a reusable fixture,
0185 rebuilds it, and the absolute-path shadow list can drift between the two
suites.

*Suggestion*: Extend the Blocks entry for 0185 to name the zero-spawn harness as
a deliverable 0185 consumes, and state whether it is exposed as a shared test
fixture; append a dated correction to 0185 repointing the harness attribution
from 0169 to 0188.

**🔵 minor (medium confidence) — Dependencies (Related: 0125) — The 0125
hand-off is identified but has no owner or action after the split**

Both the Context and the Dependencies section state that this story dissolves
0125's stated rationale for the shell lexical fallback "without closing it".
0169 carries an acceptance criterion requiring a dated hand-off note to be
appended to 0125's Dependencies on exactly this ground ("the in-process adapter
dissolves its lexical-fallback rationale") — but the in-process adapter is now
0188's deliverable, not 0169's, and 0188 has no requirement or criterion for
recording the hand-off.

*Impact*: Neither item unambiguously owns the note, so the consequence for 0125
can be observed by both stories and recorded by neither, leaving a downstream
item whose stated justification is silently obsolete.

*Suggestion*: Either add an acceptance criterion here requiring a dated note
appended to 0125's Dependencies when the adapter lands, or state explicitly in
the Related entry that 0169 retains that hand-off obligation.

**🔵 suggestion (medium confidence) — Dependencies (External systems) — Ongoing
advisory and version-drift surface of the new trees has no named owner**

The External systems entry captures the one-off adoption risks (pre-1.0 API
break, the `uluru` licence exception) but not the ongoing coupling the adoption
creates: two large transitive trees enter `cargo deny`'s `advisories` scope, so
a future RustSec advisory against any crate in the `gix`/`jj-lib` closure fails
the repo-wide check for every unrelated change, and the "pin `gix` to `jj-lib`'s
version" rule means any future `jj-lib` bump is a coordinated two-crate bump
that the single-version criterion will otherwise fail.

*Impact*: A dependency-driven CI break lands on whoever is next to push rather
than on an owner who understands the pin rule, and the coupling between the two
pins is discoverable only from this work item.

*Suggestion*: Add a line to the External systems entry noting the expanded
advisory surface and the coordinated-bump rule, and require the `gix` pin
declaration to carry an inline comment stating it must track `jj-lib`'s `gix`
version — the same treatment already given to the `deny.toml` exception.

### Scope

**Summary**: 0188 is a coherent, well-bounded unit of work: it adopts two
dependency trees, implements the existing `vcs` ports over them, and carries the
one repo-wide policy change (the `uluru` MPL exception) that adoption forces —
all of which stand or fall together and share one risk profile (build-level, not
user-visible). The declared `story` kind fits the scope, and the item states its
non-goals explicitly (`CommandProbe` retained, no consumer converged, 0125 not
closed), each backed by an acceptance criterion. The one scope weakness is the
seam with its downstream consumer 0169: the taxonomy-query surface is the only
part of 0188 with no in-story consumer, and its boundary is described by intent
("the queries the `classify_checkout` taxonomy needs") rather than as an
enumerated contract, while 0169 separately requires extending the same crates
with the taxonomy.

**Strengths**:

- The Summary, Requirements and Acceptance Criteria describe the same scope —
  every requirement (port implementation, taxonomy queries, bounded discovery,
  gix pin + uluru exception, optional pup rule) has a corresponding acceptance
  criterion, with no AC introducing work the Requirements do not state.
- The split rationale is recorded explicitly and is a genuine orthogonality
  argument rather than an arbitrary size cut: dependency adoption fails at build
  level, hook-envelope changes fail user-visibly, and the two cannot be accepted
  or rolled back independently if bundled.
- Non-goals are stated positively and gated: `CommandProbe` is retained,
  `cli/corpus-adapters` is untouched, and AC7 asserts 'this story adds an
  adapter and changes no existing consumer' — so the delivery boundary against
  0185 is unambiguous.
- The repo-wide `cli/deny.toml` licence exception is kept inside this item
  rather than orphaned into a separate policy change, which is correct — it has
  no meaning without the dependency it unblocks.
- The relationship to 0125 is scoped explicitly ('dissolves its stated rationale
  for the shell lexical fallback without closing it'), preventing a downstream
  item from being silently absorbed.
- Feasibility is measured rather than assumed (deny checks, musl static ELF,
  jj-lib loader probe against real fixtures), which materially reduces the risk
  that the item's scope turns out to be the wrong shape mid-delivery.

**Findings**:

**🔵 minor (medium confidence) — Requirements (second bullet: taxonomy queries)**

Work item 0188 ("Library-Backed VCS Adapter over gix and jj-lib") requires
extending the adapters with "the queries the `classify_checkout` taxonomy
needs, so 0169 can build classification on them", listing bare check, worktree
detection, superproject/submodule resolution, jj workspace-root and
main-vs-secondary. This is the only part of 0188 with no in-story consumer, and
its boundary against downstream story 0169 is defined by intent rather than as
an enumerated contract — meanwhile 0169's own Requirements say "Extend the
crates with the `classify_checkout` taxonomy … with the submodule, bare and
**GIT_DIR handling** that feeds them", naming a query (GIT_DIR) that 0188's list
omits.

*Impact*: Two sequential stories both claim to extend the same crate pair with
an overlapping query surface, so a query such as GIT_DIR handling can fall
between them, or be implemented twice with divergent semantics — a seam gap that
only surfaces mid-0169.

*Suggestion*: Replace the intent-based phrasing with an enumerated query list
that is the delivery contract for 0188 (adding GIT_DIR handling if it belongs on
the adapter side), and reword 0169's corresponding requirement to compose the
taxonomy from that fixed list rather than to "extend the crates".

**🔵 suggestion (high confidence) — Requirements (final bullet: cli/pup.ron
rule)**

The final requirement of work item 0188 reads "Add a `cli/pup.ron` rule for the
new adapter module **if its import surface warrants one**", leaving whether this
work is in or out of scope to an implementation-time judgement call.

*Impact*: A conditional requirement means the item's boundary cannot be stated
before work starts, and the corresponding acceptance criterion ('any new rule is
demonstrably non-vacuous') passes vacuously if the implementer decides no rule
is warranted.

*Suggestion*: Either decide now (state that a rule is added, with its intended
import constraint) or move it out of Requirements into Technical Notes as a
discretionary implementation consideration, so the Requirements list contains
only committed scope.

**🔵 suggestion (medium confidence) — Dependencies (Related: 0125)**

Work item 0188 states that going in-process "dissolves 0125's stated rationale
for the shell lexical fallback" but assigns no action for it, while sibling
story 0169 carries the acceptance criterion that appends the dated hand-off note
to 0125. Since 0188 lands first and is the change that actually dissolves the
rationale, the causing item and the item that records the consequence are
different.

*Impact*: Between 0188 landing and 0169 completing, 0125 carries a rationale the
codebase no longer supports, with no work item accountable for saying so — and
if 0169 is deferred or re-scoped, the hand-off is lost entirely.

*Suggestion*: Move the 0125 hand-off note into 0188's acceptance criteria (it is
a one-line note append), or state explicitly in 0188's Dependencies that the
note is deliberately deferred to 0169 and why.

### Testability

**Summary**: 0188 is one of the more verifiable work items of its type: the
zero-spawn criterion (AC3), the single-`gix`-version assertion (AC5) and the
non-vacuity demand on any new cargo-pup rule (AC6) each name a concrete
procedure and explicitly close an evasion route, and the Assumptions record
measured rather than presumed feasibility. The weaknesses are concentrated in
the two criteria that carry the story's substance: AC1 leans on "passes the
existing `detection.rs` suite" without stating how that suite comes to exercise
the *new* adapter (the referenced research records the existing wiring as
hard-coded to `CommandProbe`), and AC2 asks for taxonomy queries to be
"available and unit-tested" with no expected values, no named fixtures and no
stated surface — so any test at all would satisfy it. Secondary gaps: the bolded
`Workspace::load` requirement has no criterion, AC4's boundary test can pass
vacuously if the fixture sets `GIT_CEILING_DIRECTORIES` as Technical Notes
suggests, and the Summary's in-process performance rationale is nowhere measured
despite 0169 inheriting a tight latency gate.

**Strengths**:

- AC3 is an exemplary anti-evasion criterion: it names the mechanism (stub
  binaries that write a marker and exit non-zero), neutralises the config
  environment (`HOME`, `GIT_CONFIG_*`, `JJ_CONFIG` at empty temp dirs), and
  explicitly closes the absolute-path escape (`/usr/bin/git`,
  `/usr/local/bin/git`, `/opt/homebrew/bin/git` and jj equivalents) that a
  PATH-only assertion would miss.
- AC5 states not just the assertion (exactly one `gix` version in
  `cli/Cargo.lock`) but why a direct assertion is required — the repo's
  duplicate-version policy is warn-level, so a drifted pin would pass silently.
  That reasoning makes the criterion resistant to being 'satisfied' by an
  existing check that cannot fail.
- AC6 demands the new cargo-pup rule be demonstrably non-vacuous (a deliberately
  forbidden import must fail it) — a mutation-style check that most work items
  omit, and which prevents a rule that matches nothing from being counted as
  passing.
- AC7 pins the story's 'adds an adapter, removes none' invariant as an
  observable (`CommandProbe` still exists, `corpus-adapters` still resolves
  through it), turning an easily-drifted scope boundary into something a
  verifier can check.
- Assumptions distinguish measured facts (dated 2026-07-29 `cargo deny` results,
  the musl `_assert_static_elf` outcome, the single `uluru` rejection) from the
  standing risk (`jj-lib`'s self-declared unstable API), so a verifier knows
  exactly which claims were empirically established and which were not.
- The Validation Results section pre-declares the two facts the criteria require
  to be recorded (shadowed/unshadowable absolute paths, resolved `gix` version),
  so those outputs cannot be quietly omitted at acceptance.

**Findings**:

**🟡 major (medium confidence) — Acceptance Criteria (first bullet) — "Passes
the existing detection.rs suite" does not specify how the new adapter is
exercised**

Work item 0188 (library-backed VCS adapter over `gix`/`jj-lib`) states as its
first acceptance criterion that the library-backed adapter "implements
`RepoRoot` and `VcsProbe` and passes the existing
`cli/vcs-adapters/tests/detection.rs` suite", but does not say how that suite
comes to run against the new adapter. The referenced research
(`meta/research/codebase/2026-07-29-0169-...` §1) records that
`vcs_adapters::facts(start)` hard-wires `MarkerWalkRoot` + `CommandProbe::new()`
"with no injection variant", so running the suite unchanged would verify the
retained subprocess adapter, not the new one.

*Impact*: The criterion can be truthfully reported as passing while none of the
new `gix`/`jj-lib` code is exercised by any of the eight enumerated cases — the
story's primary correctness evidence would be vacuous.

*Suggestion*: Restate as a parameterised requirement — e.g. "each of the eight
enumerated cases in `detection.rs` runs against *both* `CommandProbe` and the
library-backed adapter and produces identical `RepoFacts`, with the adapter
supplied by an injection seam" — so the pass depends on the new implementation
and doubles as a differential parity check.

**🟡 major (high confidence) — Acceptance Criteria (second bullet) —
Taxonomy-query criterion states no expected values, fixtures or exposed
surface**

In work item 0188, the criterion "The taxonomy queries 0169 needs are available
and unit-tested against real fixtures: bare check, worktree detection,
submodule/superproject resolution, jj workspace root, jj main-vs-secondary"
names five queries but gives no expected output for any of them, does not define
the fixture set, and does not state on what surface the queries must be
"available" (domain port methods versus inherent adapter methods).

*Impact*: Any test that calls each query and asserts anything at all satisfies
the criterion, so it provides no verification that the queries answer
*correctly* — which is the entire basis on which 0169 will build
`classify_checkout`.

*Suggestion*: Give each query an oracle and a fixture. The Technical Notes
already map every query to a shell reference (`vcs-common.sh:207` bare,
`:217-219` worktree, `:140-146` superproject, `:74-81` jj secondary) — make
those
the expected values, e.g. "for each of the five queries, against the colocated /
secondary-workspace / worktree / submodule / bare fixtures, the adapter's answer
equals the named `vcs-common.sh` function's answer", and state whether the
queries are added to the `vcs` ports or exposed only on the adapter.

**🟡 major (medium confidence) — Acceptance Criteria (third bullet — zero spawns)
— Unbounded escape hatch in the zero-spawn criterion**

Work item 0188's zero-spawn criterion closes the absolute-path evasion by
requiring `/usr/bin/git`, `/usr/local/bin/git`, `/opt/homebrew/bin/git` and the
`jj` equivalents to be stubbed, but then adds "Any path that cannot be shadowed
in the test environment is recorded in Validation Results, not silently skipped"
with no floor on how many may go unshadowed.

*Impact*: In the limit every absolute path could be unshadowable (a read-only
`/usr/bin` is the normal case on macOS with SIP), leaving only the PATH-based
stub in force and reducing the story's central safety claim to the weak
assertion it was written to strengthen — while the criterion still reports as
passed.

*Suggestion*: Add a floor that cannot be waived — e.g. "the PATH stub is always
in force, and at least one non-PATH mechanism proves absence of spawns: either
every listed absolute path is shadowed, or a process-level seam (spawn counter /
`PanicExec`-style null, per the research's
`cli/launcher/tests/crypto_provider.rs` template) asserts zero invocations" — so
recording an unshadowable path degrades the evidence but never removes it.

**🔵 minor (medium confidence) — Acceptance Criteria (third bullet — zero spawns)
— Zero-spawn test asserts success, not correct answers**

Work item 0188's zero-spawn criterion ends "Assert no marker is written and
every query still succeeds" — success, not correctness. An adapter that silently
degrades under the stubbed environment (returning `None`/empty facts rather than
the real answer) writes no marker and "succeeds".

*Impact*: The one test run in the hostile environment — no usable `git`/`jj`,
empty `HOME`/`GIT_CONFIG_*`/`JJ_CONFIG` — is exactly the run most likely to
expose a hidden dependency on external state, yet it is the run with the weakest
oracle.

*Suggestion*: Require value equality, not just success: "every query returns the
same value under the stubbed environment as it does in the unrestricted
`detection.rs` run" — reusing the expected values from the first two criteria
rather than introducing a separate, weaker assertion.

**🟡 major (medium confidence) — Acceptance Criteria (fourth bullet) / Technical
Notes — Boundary-containment criterion can pass on fixture hygiene rather than
adapter behaviour**

Work item 0188 requires "`gix::discover` cannot escape a workspace boundary — a
fixture placing a jj secondary workspace inside a git repository resolves to the
workspace, not the parent's `.git`", while its Technical Notes separately
recommend setting `GIT_CEILING_DIRECTORIES` in fixtures "so a stray `.git` above
the temp dir cannot leak into a probe". If the boundary fixture inherits that
ceiling, discovery is stopped by the environment, not by the adapter's own
bounding.

*Impact*: The criterion that verifies the story's first bolded requirement
("Bound `gix`'s discovery explicitly") could pass against an adapter that does
no bounding at all, and the defect would surface only in real repositories where
no ceiling is set.

*Suggestion*: State the environment explicitly in the criterion — e.g. "with
`GIT_CEILING_DIRECTORIES` unset (or set above the parent repository), discovery
from inside the nested jj workspace resolves to the workspace root, not the
parent's `.git`" — and, if useful, add the paired negative case showing that an
unbounded `gix::discover` call on the same fixture does escape.

**🟡 major (medium confidence) — Requirements (fourth bullet) / Acceptance
Criteria — The "avoid Workspace::load" requirement has no acceptance criterion**

Work item 0188 states as a bolded requirement "Avoid
`jj_lib::workspace::Workspace::load` on detection paths" — because it needs a
fully-populated `UserSettings` whose defaults are private and are "discovered
one panic at a time" — but no acceptance criterion verifies it. Its sibling
bolded requirement (bound `gix` discovery) does get a criterion, so the omission
looks unintentional. The zero-spawn criterion's empty `JJ_CONFIG`/`HOME`
environment is partial coverage at best, and only if that run happens to
traverse the code path in question.

*Impact*: A regression to `Workspace::load` — the trap the research spent a
probe identifying — would be caught only by a runtime panic on some user's
machine with unusual jj config, not by anything in this story's acceptance set.

*Suggestion*: Add a criterion with a definite procedure, e.g. "no
`cli/vcs-adapters` source path references `jj_lib::workspace::Workspace::load`
(asserted by a source-level guard), and every jj query succeeds with `HOME`,
`JJ_CONFIG` and `XDG_CONFIG_HOME` pointed at empty temp dirs" — the second half
being cheap to state as a reuse of the zero-spawn environment.

**🟡 major (medium confidence) — Summary / Context / Acceptance Criteria — The
in-process performance rationale is never measured**

Work item 0188's Summary and Context justify the change partly on cost — reading
git and jj "in-process instead of spawning `jj`/`git` subprocesses", and
dissolving 0125's rationale that "probing costs 1-3 subprocesses per call" — but
no acceptance criterion measures the library-backed adapter's per-query cost,
and the Validation Results section records only shadowed paths and the `gix`
version. The consuming story 0169 carries a hard gate (`G ≤ 1.1 × B`, ≈ 38.6 ms)
that its own Dependencies section already flags as at risk from a ~41 ms warm
bootstrap.

*Impact*: If loading `gix`/`jj-lib` state costs more than the subprocess spawns
it replaces, that is discovered in 0169 at its latency gate — after the
dependency trees, the licence exception and the API bet have all landed and are
expensive to reverse, which is the opposite of the "reviewed and rolled back on
its own terms" property the split was made for.

*Suggestion*: Add a measurement criterion with a recorded output rather than a
threshold, e.g. "median of 20 invocations of each library-backed query against a
fixture repo on one host, alongside the same query via `CommandProbe`, recorded
in Validation Results with host and OS" — enough for 0169 to budget against,
without importing 0169's gate into this story.

**🔵 minor (high confidence) — Requirements (final bullet) — "Add a pup rule if
its import surface warrants one" has no decision criterion**

Work item 0188's final requirement — "Add a `cli/pup.ron` rule for the new
adapter module if its import surface warrants one" — leaves "warrants"
undefined, and the matching acceptance clause is conditional ("any new rule is
demonstrably non-vacuous"). If no rule is added, both requirement and clause are
satisfied trivially.

*Impact*: A verifier cannot determine whether the requirement was met or quietly
skipped, so the architectural-boundary question the requirement raises is
resolved by implementer discretion with no record.

*Suggestion*: Either state the rule concretely (e.g. "a rule scoped to
`vcs_adapters` permitting `gix`, `jj_lib`, `std`, `kernel::Error` and `crate::`,
mirroring `vcs_domain_imports_only_permitted`'s shape"), or make the decision
itself the deliverable: "the decision on whether a `vcs_adapters` pup rule is
needed is recorded in Validation Results with its rationale".

**🔵 minor (medium confidence) — Assumptions / Acceptance Criteria (fifth
bullet) — No criterion pins the jj-lib version the feasibility evidence rests
on**

Work item 0188 asserts "exactly **one** `gix` version resolves in
`cli/Cargo.lock`" as a directly-asserted criterion, but nothing equivalent
covers `jj-lib`, whose Assumptions entry reads "`jj-lib`'s loader API remains
stable across the pinned version. Verified against 0.43; unstable by the crate's
own declaration." No criterion states what "the pinned version" is or that the
lock resolves to it.

*Impact*: The story's entire loader-internals design
(`DefaultWorkspaceLoaderFactory`, `WorkspaceLoader::{workspace_root,
repo_path}`) was validated against 0.43 specifically; a caret range resolving
forward at any later `cargo update` silently invalidates that evidence with no
failing check.

*Suggestion*: Extend the lock-file criterion — "`cli/Cargo.lock` resolves
exactly one `gix` version and `jj-lib` at the version the feasibility probe
validated (0.43), both recorded in Validation Results" — and state whether the
manifest requirement is exact (`=0.43`) or ranged.

## Re-Review (Pass 2) — 2026-08-01

**Verdict:** REVISE

All five lenses re-run against the revised work item. **Every one of the ten
pass-1 majors is resolved or substantially resolved at the structural level** —
the query surface is enumerated and homed, the vacuous-pass routes are closed,
the missing criteria exist, the pins are symmetric, and the hand-offs are
assigned. The verdict stays REVISE because the revision introduced a new layer
of *specification-detail* defects, several of them squarely attributable to the
pass-1 fixes themselves. This is a materially better work item that is not yet
done.

The re-review had one advantage pass 1 lacked: adding `scripts/vcs-common.sh`
to References (a pass-1 recommendation) meant the lenses could read the shell
oracle. Two of the strongest new findings come directly from that.

### Previously Identified Issues

- 🟡 **Clarity**: "domain crate untouched" vs taxonomy queries — **Resolved**.
  Queries are inherent methods; `CommandProbe`/`MarkerWalkRoot` explicitly gain
  none. Residual: no criterion asserts `cli/vcs/src/**` is unmodified, so an
  *added* trait would still pass (new minor).
- 🟡 **Testability**: AC2 states no expected values/fixtures/surface —
  **Partially resolved**. Oracle, fixture matrix and surface are now stated, but
  two of the four cited shell references are not callable functions (new major).
- 🟡 **Dependency/Scope**: `GIT_DIR` absent from the surface — **Resolved as
  scope, incomplete as specification**. Query 6 exists, but whether the override
  is honoured or scrubbed is left to the implementation (new major).
- 🟡 **Testability**: AC1 doesn't exercise the new adapter — **Resolved**.
  Injection seam plus identical-`RepoFacts` differential check. Residual: four
  of the eight cases are oracled only differentially (new minor).
- 🟡 **Clarity/Testability**: bounded discovery undefined and unverifiable —
  **Resolved, with two new defects in the fix**. The rule is now normative and
  AC4 has its paired negative assertion, but the rule says "first ancestor of
  the start path", which literally excludes the start path itself, and queries
  2-3 must legitimately resolve outside the boundary (both new).
- 🟡 **Completeness/Testability**: `Workspace::load` has no AC — **Resolved**.
  Residual: "detection path" is undefined and the crate-wide guard is broader
  than the path-scoped rule (new major).
- 🟡 **Dependency/Testability**: in-process cost never measured — **Resolved as
  an obligation, unsatisfiable as written**. The criterion exists, but the
  sub-binary it sizes does not exist in this story (new major, four lenses).
- 🟡 **Testability**: zero-spawn escape hatch — **Partially resolved**. The
  floor is added, but neither alternative reliably observes the target violation
  class (new major).
- 🟡 **Dependency**: toolchain preconditions uncaptured — **Resolved for jj,
  partially for the rest**. MSRV is named but has no owner or fallback, and the
  symmetric `git` CLI ↔ `gix` coupling is missing (new).
- 🟡 **Dependency/Scope**: 0125 hand-off unowned — **Resolved**. Now an AC here.
  Residual: the note's claim is contingent on shell-caller migration that has
  not happened, and 0169's duplicate criterion drop has no owner (new minor).

### New Issues Introduced

Ordered by how many lenses independently flagged them.

- 🟡 **Clarity + Completeness + Scope + Testability**: **"The sub-binary size
  delta" has no artefact.** The story delivers "public constructors and nothing
  more" and adds no selection mechanism, so nothing links the new trees. The
  same root defeats the musl `_assert_static_elf` criterion — with no caller,
  dead-code elimination may mean neither check exercises `gix`/`jj-lib` at all.
  Both criteria can pass while proving nothing. *Introduced by the pass-1
  measurement fix.*
- 🟡 **Completeness + Dependency + Scope + Testability**: **The 0185
  obligations are prose-only.** Exposing the zero-spawn harness as a shared
  fixture, and the dated correction repointing 0185's attribution, are both
  declared "part of closing this story" in Dependencies with no Requirement and
  no criterion — the precise failure mode pass 1 flagged for the `deny.toml`
  comment. Dependency also notes the correction is understated: 0185 attributes
  the adapter to 0169 in its Summary, Context, Assumptions and Technical Notes,
  not only its criteria. *Introduced by the pass-1 hand-off fix.*
- 🟡 **Testability**: **Two of the four cited shell oracles are not callable.**
  `vcs-common.sh:207` and `:217-219` are inline expressions inside
  `classify_checkout` setting locals (`is_bare`, `git_worktree`,
  `git_common_dir`) that never appear in the `KEY=VALUE` record it emits; and no
  shell function returns the common git directory —
  `find_git_main_worktree_root` returns its parent via `dirname` (`:154`). For
  queries 1 and 2 the stated oracle cannot be executed. *Introduced by the
  pass-1 oracle fix, which promoted a Technical Notes mapping to a normative
  oracle without checking the targets were functions.*
- 🟡 **Clarity + Testability**: **`GIT_DIR` semantics unspecified, and the shell
  oracle contradicts itself on this exact point.** `find_git_main_worktree_root`
  deliberately scrubs a caller-set `GIT_DIR` and re-enters (`:130-135`), so
  query 3 ignores the override, while the inline bare/worktree checks inherit it
  through `git rev-parse` and honour it. The criterion requires testing the
  override-vs-ceiling disagreement with no stated expected value. "The discovery
  ceiling" also has two candidate referents (the marker-derived ceiling vs
  `GIT_CEILING_DIRECTORIES`).
- 🟡 **Clarity**: **The pup rule permits `std` and denies `std::process`.**
  `std::process` is not "everything else" — it is inside the permitted `std`, so
  under a prefix-matching allow-list the two clauses cannot both hold. This rule
  is the stated reason zero-spawn is "structural rather than only
  test-asserted". *Introduced by the pass-1 pup fix.*
- 🟡 **Clarity**: **"Detection path" is undefined and the guard is wider than
  the rule.** It is unstated whether the `VcsProbe::revision` read is a
  detection path, while the verifying guard is crate-wide over
  `cli/vcs-adapters` and cannot distinguish one path from another.
- 🟡 **Testability**: **Neither zero-spawn floor alternative observes the target
  violation.** A process-level seam inside the crate cannot intercept a spawn
  originating inside `gix` or `jj-lib` (and the library-backed module has no
  exec port by design); shadowing `/usr/bin/git` is impossible on SIP-protected
  macOS, the primary dev platform. The floor can be satisfied by a mechanism
  that cannot fail.
- 🟡 **Dependency**: **The six-query contract may under-serve 0169.** 0169's arm
  list includes `nested-jj-in-git` and `nested-git-in-jj`, which need to know
  whether a marker exists *above* the boundary — a capability the bounding rule
  explicitly forbids and no query provides. 0169 also owns `vcs status`/`vcs
  log` while stating it "adds no dependencies", so those in-process reads must
  come from these trees but are in neither the six queries nor the ports. The
  change-control clause converts either gap into a reopening of a closed story.
- 🟡 **Completeness + Dependency**: **MSRV is an unresolved precondition with no
  owner, no fallback and no Open Questions section.** If `jj-lib` 0.43's MSRV
  exceeds the pinned toolchain, the exact `=0.43` pin that the whole dependency
  policy rests on is not viable, and the item states no position on what
  happens. `mise.toml`'s Rust pin is also absent from the shared-artefact list.
- 🟡 **Testability**: **The cost measurement is not comparable to 0169's gate.**
  No fixture named, no statement of whether an "invocation" is an in-process
  call or a process launch, warm or cold. An in-process microbenchmark yields
  microsecond numbers against 0169's millisecond process-level baseline.

Minor and suggestion-level items also surfaced: three internal contradictions
of the form "X and nothing more" beside a bullet requiring more ("public
constructors and nothing more" vs the six methods; "0125 is not otherwise
modified" vs appending a note; the `std`/`std::process` pair above); the
bounding rule's "first ancestor" excluding the start path; no stated comparison
rule between shell strings and typed values (canonicalisation, exit-1-as-`None`
— a real macOS `/var` vs `/private/var` trap); the reciprocal nested-git-in-jj
boundary fixture untested; the zero-spawn fixture set not tied to the two
preceding matrices; the shell oracle declared authoritative with no disposition
for where it is known to be wrong; "probe" carrying three senses; and 0187
likely miscited in the shared-artefact contention list (its Requirements are
entirely `tasks/`-side and name none of the three `cli/` artefacts).

### Assessment

The work item is **not yet ready for implementation**, but the remaining
distance is much shorter than pass 1's. The structural questions — what is
delivered, where it lives, what "done" means, who owns each hand-off — are all
answered now. What remains is a specification-detail pass concentrated in four
places:

1. **Name the reference artefact** for the musl and size criteria (a test-only
   binary calling the six queries, mirroring the 2026-07-29 probe), and pin the
   cost measurement's fixture, unit and warm/cold state to what 0169 measures.
   Closes two majors and makes the hand-off numbers usable.
2. **Fix the oracle** — substitute callable commands (`git rev-parse
   --is-bare-repository`, `--git-dir` vs `--git-common-dir`) for the two
   non-function references, state the comparison rule (canonicalised paths,
   exit-1 = `None`), and decide `GIT_DIR` precedence normatively rather than
   deferring it to adapter documentation.
3. **Promote the 0185 obligations to criteria**, matching the treatment 0125
   already gets, and widen the correction to 0185's Summary/Context/Assumptions.
4. **Repair the internal contradictions** — the pup allow/deny overlap, "first
   ancestor", the two "and nothing more" clauses — and define "detection path".

Two items need a decision rather than an edit: the `GIT_DIR` honour-vs-scrub
semantics, and whether the six-query contract is reconciled against 0169's
nested arms and its `status`/`log` surface before pickup. The second is the more
consequential — it is the one finding that could change what this story
delivers rather than how it is described.

## Re-Review (Pass 3) — 2026-08-02

**Verdict:** REVISE

Most pass-2 findings are resolved. But this pass changes the recommendation
rather than extending it: **the iteration method itself is now the main source
of new defects, and the document has acquired the growth signature that forced
its parent 0169 to be split.** Three passes of closing findings by adding text
have taken it from 181 to 516 lines, and its Requirements section now describes
roughly half of what its thirteen acceptance criteria deliver.

### Previously Identified Issues

- 🟡 **Sub-binary size delta had no artefact** — **Resolved**; a reference
  artefact is named. Residual: nothing asserts it actually *links*
  `gix`/`jj-lib`, so LTO can still eliminate the calls and both the musl and
  size criteria pass vacuously — the same vacuity, moved one level down.
- 🟡 **0185 obligations prose-only** — **Resolved**; both are criteria now.
  Two new consequences: repointing 0185's attribution leaves it holding
  composition-root work it believes someone else does, and nothing in this
  story compiles the "shared fixture" across a crate boundary.
- 🟡 **Two shell oracles not callable** — **Half resolved, half regressed.**
  Queries 1-2 are correctly repointed to `git rev-parse
  --is-bare-repository` / `--git-dir` vs `--git-common-dir`. But the fix
  introduced **two new non-executable oracles** (below).
- 🟡 **`GIT_DIR` semantics** — **Resolved.** Scrub-everywhere, with the shell's
  internal asymmetry recorded as a deliberate non-reproduction.
- 🟡 **pup allow/deny overlap** — **Resolved** via the two-clause form.
- 🟡 **"Detection path" undefined** — **Resolved** by enumeration. Residual: the
  paragraph concludes "the verifying guard is *correspondingly* crate-wide"
  immediately before a requirement insisting the *other* guard is module-scoped
  — adjacent opposite scopings joined by a connective implying they agree.
- 🟡 **Zero-spawn floor** — **Partially resolved.** Platform-scoped now, but
  "on at least one platform where it is achievable" is self-judged: if the
  Linux runner's `/usr/bin` is read-only, every platform legitimately degrades
  and the strong form runs nowhere while the criterion still reads as met.
- 🟡 **Six-query contract under-serving 0169** — **Resolved.** Query 6 added,
  `status`/`log` recorded as 0169-authored.
- 🟡 **MSRV ownerless** — **Resolved** via Open Questions with escalation
  defaults. New: Open Questions and Dependencies now give *opposite* answers on
  whether `mise.toml` may be edited in this story.
- 🟡 **Cost measurement not comparable** — **Partially resolved.** Shape pinned
  (median of 20, three figures), but the fixture anchor is circular (below).

### New Issues Introduced

**Two more non-executable oracles — verified directly against
`scripts/vcs-common.sh`, not inferred:**

- 🟡 **`BOUNDARY` is the wrong oracle for queries 4 and 6.** The record contract
  documents it as "realpath of the active workspace; **empty for main and
  none**" (`:165`), and the `nested-git-in-jj` arm sets it to
  `$git_worktree_root` — the *git* root (`:259`). Both the `colocated` and
  `nested-jj-in-git` arms gate on `jj_secondary=1` (`:242`, `:248`), so a main
  workspace falls through to `KIND=main` with `BOUNDARY=""` even where query 4
  has a real answer. On at least three of the eight required fixtures the
  stated oracle returns empty or the wrong root — precisely the fixtures query
  6 exists to distinguish. The correct oracle is `jj workspace root`, which
  `classify_checkout` itself probes with at `:184`.
- 🟡 **`find_git_main_worktree_root` cannot express query 3's "not a
  submodule".** It returns the superproject only when
  `--show-superproject-working-tree` is non-empty (`:142-146`); otherwise it
  falls through to `dirname(--git-common-dir)` and exits 0 with an ordinary
  root. So on seven of eight fixtures it returns a value the adapter must *not*
  return, and the empty-equals-`None` clause does not rescue it. The correct
  oracle is `git rev-parse --show-superproject-working-tree` directly.
- 🔵 **A citation points at the wrong function.** "queries 4-5 →
  `classify_checkout`'s `BOUNDARY` and `JJ_PARENT` record fields (`:74-81`)" —
  `:74-81` is `_jj_workspace_is_secondary`. The record contract is `:164-171`,
  emission `:274-279`.

**Structural contradictions from layering:**

- 🟡 **"The query set" is defined twice with different extents** — Requirements
  says "exactly" the seven numbered items; the AC preamble says the seven
  "**plus** the port methods". Three criteria iterate this set.
- 🟡 **Item 7 is not a query.** "`GIT_DIR`/`GIT_COMMON_DIR` are scrubbed" is an
  invariant over items 1-6, yet the set must be delivered as "inherent
  methods", the reference artefact must "call every query in the set", and cost
  figures are required "for the seven queries".
- 🟡 **Open Questions vs Dependencies on `mise.toml`** — escalate the toolchain
  bump out of scope, or edit it here; the document says both.
- 🟡 **One adapter or a pair, still stated both ways** — "one implementation of
  each port" and "adapter **types**" against "**The** library-backed adapter
  implements `RepoRoot` and `VcsProbe`" and "both adapter **pairs**". With two
  types it is unstated which carries queries 1-3, which 4-5, and where the
  dual-root query 6 lives.
- 🔵 The cost criterion's figure matrix does not match the Validation Results
  slots, and figure (a) "library initialisation cost" is meaningless for the
  subprocess pair it is required for.

**Circular and unscoped dependencies:**

- 🟡 **The cost fixture anchor is circular** (4 lenses). "The same pure-jj
  fixture 0169 measures against" — 0169 names no fixture; it lists the fixture
  as something to *record* at acceptance, and 0188 lands first. Neither
  document contains enough to build it twice identically, so the comparability
  that is "the point" is not guaranteed.
- 🟡 **The strong-form zero-spawn run depends on unscoped CI infrastructure**
  (3 lenses). A Linux container with a writable `/usr/bin` and bind-mount
  privileges is an infrastructure precondition with no named job, no owner and
  no fallback — and nothing else in this story touches CI.
- 🟡 **Obligations handed forward to 0169 live only here.** 0169 must widen the
  pup rule, define its own port over inherent methods, and honour a closed
  seven-query contract — but the correction criterion appends only the 0125
  strike-out, so 0169's implementer will not learn any of it from 0169.
- 🟡 **`tasks/` is an uncaptured shared artefact.** `_assert_static_elf` and
  cross-compile staging live in `tasks/build.py`, which 0187 also rewrites — so
  the parenthetical de-listing 0187 tests only the three `cli/` files and
  misses the plausible overlap.
- 🟡 **"No selection mechanism" has no verifying criterion** — a new
  `[features]` entry or composition helper inside `cli/vcs-adapters` would
  satisfy every existing criterion.
- 🔵 **"No TLS stack enters the graph"** is claimed in prose and never checked;
  it could ride the existing lockfile guard.

**Scope:**

- 🟡 **Seven of thirteen criteria deliver work Requirements never states** — the
  injection seam, the zero-spawn harness, the lockfile check, the reference
  binary, the shared fixture, the cost measurement, the sibling hand-offs.
  Zero-spawn, the story's central safety claim, appears in Requirements only as
  a subordinate clause explaining why the pup rule has two parts.
- 🔵 **The growth is absorbed deliverables, not clarification** — five of them,
  only one of which is the adapter the story is named for. This is the pattern
  that split 0169 after four passes.

### Assessment

**Stop patching; consolidate.** Two things make another incremental pass the
wrong move.

First, **I have now written non-executable oracle mappings twice**, in the same
section, by reasoning from line numbers instead of running the commands. Pass 2
fixed two; pass 3 found two more that pass 2 introduced. The oracle table cannot
be written correctly from inference — it needs to be built by executing each
candidate command against each fixture and recording what comes back. That is a
different activity from editing prose, and it should happen before the criterion
is written again.

Second, the document has outgrown its Requirements section. Sizing or planning
from Requirements now under-reads the story by half its acceptance surface, and
the fix is restructuring — moving delivered artefacts into Requirements,
collapsing the duplicate definitions, resolving the three contradictions — not
adding more text.

Recommended next step is a **consolidation pass that adds no new obligations**:

1. Build the oracle table empirically against real fixtures; write the criterion
   from the results.
2. Promote the seven undeclared deliverables into Requirements so Summary,
   Requirements and Criteria describe one unit.
3. Resolve the three contradictions (`mise.toml`, query-set extent, one adapter
   vs a pair) by deleting one side of each.
4. Break the circular fixture anchor by defining the pure-jj fixture here.
5. Decide the two genuinely open scope questions rather than absorbing them:
   whether the shared fixture and the CI container belong in this story at all.

If the item is still this large after consolidation, the scope lens's split
suggestion deserves a hearing — dependency adoption plus ports (the
risk-isolation core, which is what the split was *for*) separated from the
taxonomy query surface and its oracle apparatus.

## Re-Review (Pass 4) — 2026-08-02

**Verdict:** REVISE

The consolidation worked. Two things pass 3 could not resolve are now settled,
and the character of what remains has changed: pass 3's findings were structural
and methodological, pass 4's are mostly small, concrete and mechanical — plus a
cluster of stale text the consolidation itself left behind.

### Previously Identified Issues

- 🟡 **Should it be split?** — **Resolved: no.** The scope lens is unambiguous
  this pass: "a coherent, well-bounded single unit of work … it should **not**
  be split", because each candidate seam "produces a fragment with no
  independently verifiable value". It notes the item sits at the upper end of
  story sizing but that the weight is verification of one indivisible bet.
- 🟡 **Oracle mapping written from inference** — **Resolved.** Testability calls
  the deferral "well framed and actionable", crediting it for naming the oracle
  set, widening it from the shell wrappers to the underlying `git`/`jj`
  invocations (which is what makes the inline-local queries tractable), and
  citing the two wrong inferences. Residual: the mapping's own delivery has no
  checkbox and "with evidence" is undefined.
- 🟡 **Requirements described half the criteria** — **Resolved** for the seven
  promoted deliverables. Two more surfaced that are still AC-only: the
  cost-measurement work and the sibling hand-off notes.
- 🟡 **Query set defined twice / item 7 not a query** — **Resolved.** Six
  queries, scrub restated as an invariant over them.
- 🟡 **One adapter or a pair** — **Resolved in Requirements, regressed in AC.**
  The first criterion still says "both adapter pairs" (below).
- 🟡 **`mise.toml` contradiction** — **Resolved for the Rust pin** (MSRV fits),
  **reintroduced for the `jj` pin** (below).
- 🟡 **Circular pure-jj fixture anchor** — **Resolved** in the criterion (it is
  defined here, 0169 reuses it), **but Validation Results still attributes it to
  0169**.
- 🟡 **CI infrastructure for strong-form zero-spawn** — **Still open, third
  consecutive pass.** Now flagged by dependency *and* scope as a hard gate with
  no owner and no confirmed runner capability.

### New Issues Introduced

**Stale text the consolidation left behind — all mine, all one-line fixes:**

- 🟡 **Validation Results contradicts the body** (3 lenses). It still records
  "`mise.toml` pins 0.36.0, installed 0.36.0 … coherence unverified (see Open
  Questions)" — pointing at an Open Question the document declares retired, and
  contradicting Dependencies, Requirements and the actual file. It also heads a
  slot "Cost, against **0169's** pure-jj fixture" when the criterion defines the
  fixture here.
- 🟡 **The `jj` pin state is stated four ways** — a standing obligation in
  Requirements, already-done in Dependencies and Open Questions, an edited file
  in Shared-artefact contention, and not-done in Validation Results.
- 🟡 **"Both adapter pairs"** in the first criterion contradicts the single
  library-backed type Requirements spent a sentence justifying.

**Genuinely new, and good — mostly one-line criterion tightenings:**

- 🟡 **The fixture matrix never fixes the start directory.** All six queries are
  start-path-relative and the oracle is directory-parameterised. Queried from a
  superproject root the submodule query answers "not a submodule"; from inside
  the submodule it answers with the superproject. Same for linked-worktree
  (worktree vs main) and both nesting directions. An implementer could run all
  six from each fixture's root, produce 54 green cells, and never exercise the
  distinctions the queries exist to make.
- 🟡 **The scrub invariant can pass vacuously.** It requires equal answers "with
  and without `GIT_DIR`/`GIT_COMMON_DIR` set" but never says what they are set
  *to*. Pointed at a non-existent path both are ignored and the test proves
  nothing; the invariant only bites when they point at a real git directory that
  would produce a different answer. This is the one criterion in the item with
  no non-vacuity control, in a document that applies them carefully elsewhere.
- 🟡 **"Assert the delta is non-trivial" is an unthresholded gate** inside a
  criterion that opens "measured and recorded, **not gated**" — and the two
  builds being differenced are not defined. It is the clause guarding the
  story's headline false-pass (dead-code elimination) and currently the least
  decidable in the item.
- 🟡 **The composition root is handed to two dependants with no ordering.**
  Requirements says wiring "belongs to 0169 and 0185"; the hand-off
  criterion
  tells 0185 it is "0185's own work". 0185's `blocked_by` is 0188 only, so both
  are simultaneously unblocked to change `vcs_adapters::facts` on contradictory
  assumptions — 0185's Technical Notes still assume 0169 goes first.
- 🟡 **0185's deletion invalidates the dual-adapter suite this story builds.**
  The first criterion runs `detection.rs` against both pairs; 0185 deletes
  `CommandProbe` and believes those pins hold "unchanged". Collapsing the
  comparison to a single adapter is unowned work in 0185's "wiring plus
  deletion" sizing.
- 🟡 **The nine-shape fixture matrix is not a listed deliverable**, though four
  criteria range over it and Test-support deliverables is explicitly the list
  that is "sized as such". Bare, submodule, linked-worktree and both nesting
  shapes are substantial to build, and the item never says which exist today.
- 🟡 **"One consumer inside this story" names no crate**, and the only suite the
  story touches is inside the crate that owns the fixture — so it cannot
  demonstrate a crate boundary. "Consumer" is also overloaded against its
  production sense used throughout.

Minor and suggestion level: an ambiguous "its" in the 0185 note with a section
list that disagrees with Dependencies; `G`/`B` never expanded; the lockfile
check omits the `gix` 0.85 pin itself (single-version can hold vacuously if
`jj-lib`'s `gix` feature is off) and the `mise.toml` lockstep; the Rust pin is
the only leg of the three-pin coupling with no inline comment; the
`UserSettings` guard has no non-vacuity demonstration while the pup rule does;
no out-of-repository fixture; 0168's restructuring has in fact already landed so
the contention is overstated; MSRV and SIP unexpanded.

### Assessment

**Converging, but not by finding count.** Pass 3 raised roughly thirteen majors
and pass 4 raises about twelve — yet they are not comparable. Pass 3's were
"the method is wrong" and "Requirements describes half the story". Pass 4's
split cleanly into three groups: **stale text from the consolidation** (three
findings, all one-line, all mine), **criterion tightenings** (start directories,
poisoning value, size threshold — each a sentence), and **two genuine open
decisions** that have now survived multiple passes without being resolved:

1. **Who owns the CI job** for the strong-form zero-spawn run, and does the
   runner actually permit replacing or bind-mounting `/usr/bin/git`? Flagged in
   passes 2, 3 and 4. It is a hard gate on a capability nobody has confirmed.
2. **Who changes `vcs_adapters::facts`** — 0169 or 0185 — and in what order.

Everything else is editing. The recommendation is to fix the stale text and the
criterion tightenings directly, put those two decisions to the author, and then
stop reviewing: the remaining findings are the kind a plan resolves by being
written, and a fifth pass would be measuring diminishing returns. "Ready" at
this point is a judgement call about the two decisions above, not a lens
verdict.

## Approval — 2026-08-02

**Verdict changed to APPROVE by author decision**, overriding the pass-4 lens
verdict of REVISE. The four pass sections above are left exactly as written;
this section records the decision, not a fifth pass.

The basis is the pass-4 assessment's own conclusion: after the consolidation,
what remained divided into stale text (since swept), one-line criterion
tightenings (since applied), and two decisions that no further review pass could
resolve. With the first two groups closed, readiness turned on those two
decisions — and that is an author call, not a lens verdict.

**Approved with two Open Questions outstanding.** Both are recorded in the work
item with stated defaults, and both gate planning rather than approval:

1. **Who owns the CI job for the strong-form zero-spawn run, and does the runner
   permit replacing or bind-mounting `/usr/bin/git`?** Raised in passes 2, 3 and
   4. It is the one acceptance gate resting on an unconfirmed infrastructure
   capability. Default if it cannot be arranged: carve provisioning into its own
   item that 0188 declares `blocked_by`, rather than weakening the criterion to
   `PATH`-only everywhere — which would leave the property unproven on any
   platform.
2. **Who changes `vcs_adapters::facts`, 0169 or 0185, and in what order?** Both
   are currently unblocked by 0188 alone, on contradictory assumptions.

**One implementation-time obligation also carries forward**: the `mise.toml`
`jj` pin was bumped 0.36.0 → 0.43.0 during this review, but `mise install` and a
re-run of the jj-fixture shell suites are still outstanding — recorded as
_pending_ in the work item's Validation Results.

**Approval does not signal that the oracle mapping is settled.** It is
deliberately deferred to planning, to be built by executing candidates against
fixtures; two attempts to write it from line references during this review were
both wrong, and the criteria now require command-and-verbatim-output evidence
per cell.
