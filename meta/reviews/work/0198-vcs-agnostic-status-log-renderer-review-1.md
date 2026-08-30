---
type: "work-item-review"
id: "0198-vcs-agnostic-status-log-renderer-review-1"
title: "Work Item Review: Replace vcs status/log with a VCS-agnostic library-backed renderer"
date: "2026-08-30T19:38:46+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0198"
work_item_id: "0198"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-30T22:29:25+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Replace vcs status/log with a VCS-agnostic library-backed renderer

**Verdict:** REVISE

This is a strong, self-aware draft — its sole consumer is named precisely, its
non-goals are stated repeatedly, and it pre-empts likely misreadings. It falls
short of ready for one structural reason: the output format is the story's
central deliverable (Acceptance Criterion 1), yet the decisions that define it
(log identity fields, status summary fields, log depth, conflict marker) are
still open, which produces an internal contradiction, four untestable
acceptance criteria, and a sequencing risk. A secondary gap is that the
external `gix`/`jj-lib` coupling and its dependency-policy artefacts are absent
from the Dependencies section.

### Cross-Cutting Themes

- **Unresolved output format undermines the whole story** (flagged by:
  testability, clarity, completeness, scope) — the format is AC1's deliverable
  but key field/depth/marker choices remain in Open Questions. This surfaces
  four ways: a Requirements-versus-Open-Questions contradiction on log fields
  (clarity), acceptance criteria that reference a format no verifier can check
  yet (testability), format-defining choices split across two sections
  (completeness), and a design decision sequenced inside the build rather than
  ahead of it (scope). Resolving the format first collapses all four.
- **Acceptance criteria under-specified for verification** (flagged by:
  testability) — the core no-PATH-dependency promise is verified only
  structurally (module deletion), and the parity, never-fail, and conflict
  criteria lack the fixture recipes, fault-injection mechanism, and exact
  strings that the sibling 0169 pinned for the same surface.

### Findings

#### Critical

None.

#### Major

- 🟡 **Testability**: No criterion verifies the core no-PATH-dependency goal behaviourally
  **Location**: Acceptance Criteria
  The Summary's central promise — status/log work with `jj`/`git` absent from
  `PATH` — has no criterion running the subcommands under that condition. AC6
  only asserts the subprocess module is deleted, a structural proxy that passes
  on a host where both binaries are present.

- 🟡 **Testability**: Output format deferred to Open Questions; "identical-shaped" has no comparison procedure
  **Location**: Acceptance Criteria
  AC1 requires the format "specified and recorded", but the field set, log
  depth, and conflict marker are all still Open Questions, so AC2 and AC4
  reference a format no verifier can check. Git and jj outputs genuinely differ
  (hash vs change-id, branch vs bookmark), so "identical-shaped" needs a defined
  normalisation/mask rule to yield a pass/fail.

- 🟡 **Testability**: Never-fail criterion is unbounded and lacks fault injection and exact fallback text
  **Location**: Acceptance Criteria
  AC5 ("any adapter failure yields `(... unavailable)`") does not say how a
  failure is triggered in a test, does not pin the exact fallback string, and
  does not state what a "diagnosable" `ACCELERATOR_LOG` run must assert. 0169
  was explicit: inject via a test-only failing adapter, never file permissions,
  so it cannot pass vacuously under root.

- 🟡 **Testability**: Conflict criterion leaves both the indicator and the fixture construction undefined
  **Location**: Acceptance Criteria
  AC3 says the output "surfaces the conflict state" but defines neither the
  exact indicator to assert nor how the conflict/unmerged state is produced in
  each backend. "Surfaces" could be argued as met by any incidental mention,
  and two verifiers could build different conflict fixtures.

- 🟡 **Clarity**: Requirements and Open Questions give opposite signals on the log field set
  **Location**: Requirements / Open Questions
  Requirements defines the log firmly as "a flat list of recent commits (short
  id plus subject)" with no hedge, while Open Questions reopens exactly that:
  "short id plus subject only, or also author and date?". An implementer cannot
  tell whether author/date is in scope. (Depth is fine — Requirements hedges it
  with "unless a different depth is justified".)

- 🔴 **Dependency**: External `gix`/`jj-lib` coupling and its dependency-policy artefacts are unnamed in Dependencies
  **Location**: Dependencies
  The implementation rests entirely on `gix` and `jj-lib` and needs library
  surface 0188 did not deliver (working-copy diff via gix; revset/graph/dag_walk
  via jj-lib). Per 0188 these carry a standing policy cost — a `cli/deny.toml`
  MPL licence exception, a multi-way version pin, a single-gix-version
  invariant, and `gix` pinned `default-features = false`. A status/diff summary
  may need a gix feature that selection omits, re-opening that shared review.
  The Dependencies section names only work items.

#### Minor

- 🔵 **Clarity**: Relationship between `--fail-safe` and the never-fail contract is unstated
  **Location**: Requirements
  Context shows the skill invoking `vcs status --fail-safe`, implying the
  fallback is tied to that flag, but Requirements and AC5 describe the never-fail
  behaviour without mentioning `--fail-safe`. It is unclear whether the
  `(... unavailable)` fallback is unconditional or gated on the flag.

- 🔵 **Clarity**: "rendered identically" reads as byte-identical, which is impossible
  **Location**: Summary
  Summary and Requirements say "rendered identically", while AC2 uses the
  precise "identical-shaped". Because the same repo state yields different values
  across backends (SHAs vs change-ids, branch vs bookmark), literally identical
  output cannot exist — only the shape is identical.

- 🔵 **Dependency**: 0188 captured only in References, not Dependencies
  **Location**: Dependencies
  This story extends the gix/jj-lib trees and the library-backed adapter module
  0188 delivered, and must widen 0188's cargo-pup library-reads rule to cover
  status/log — yet 0188 appears only in References, unlike the other done
  prerequisites 0169 and 0185 which sit in Dependencies.

#### Suggestions

- 🔵 **Completeness**: Fold the format-defining Open Questions into AC1's ADR deliverable
  **Location**: Requirements / Open Questions
  Several format-defining choices (author/date on log lines, ahead/behind on
  status, final depth) sit in Open Questions rather than the Requirements that
  AC1 turns into the recorded spec, so the remaining specification work is split
  across two sections.

- 🔵 **Scope**: Sequence the format decision explicitly as the first step
  **Location**: Open Questions
  The format definition is a distinct cross-cutting decision whose resolution
  gates the two backend adapters. Keeping it in-scope is fine, but sequence it
  first (the linked-ADR option AC1 already allows) so it is discharged before
  the adapters are built.

- 🔵 **Clarity**: "the consumer" and "a committer" are used interchangeably
  **Location**: Context
  The item defines the sole consumer as the `/commit` skill (software) but
  attributes the conflict-indicator benefit to "a committer" (the human),
  momentarily blurring who the requirement serves.

### Strengths

- ✅ The sole consumer is named unambiguously and repeatedly (the `/commit`
  skill, with a file path), giving "the consumer" a single stable referent.
- ✅ Scope non-goals are stated explicitly and consistently — byte-parity with
  native `jj`/`git` output is called out in Summary, Requirements, and
  Acceptance Criteria, removing an otherwise untestable comparison target.
- ✅ `status` and `log` are correctly treated as one indivisible unit: they
  share `run_vcs_text` and the subprocess module, feed one renderer, and the
  module can only be deleted once both migrate.
- ✅ The Context is exceptionally thorough — it traces the prior decisions
  (0169's deliberate subprocess choice, 0185 retiring `CommandProbe`) and
  reconciles the tension a reader might trip on (cross-VCS uniformity is a
  test convenience, not user value).
- ✅ jj-lib's pre-1.0 API instability is captured as an explicit risk across
  Open Questions, Technical Notes, and Assumptions rather than left implicit.
- ✅ Frontmatter is complete and valid, and every expected section is present
  and substantively populated.

### Recommended Changes

1. **Resolve the format Open Questions and pin them in AC1 (or a linked ADR)**
   (addresses: testability "Output format deferred", clarity "Requirements and
   Open Questions give opposite signals", completeness "Fold the format-defining
   Open Questions", scope "Sequence the format decision first")
   Settle the log field set (short id + subject, with/without author/date), the
   status summary fields (parent, ahead/behind), the final log depth, and the
   conflict marker. Move these from Open Questions into the Requirements that
   AC1 records, and make the format spec AC1's explicit first deliverable so it
   is discharged before the adapters are built. This single change closes the
   dominant cross-cutting theme.

2. **Add a behavioural no-PATH-dependency acceptance criterion** (addresses:
   testability "No criterion verifies the core goal behaviourally")
   Mirror 0169/0188's PATH-stripped test: with `jj` and `git` removed from
   `PATH`, `vcs status` and `vcs log` produce correct output across the fixture
   matrix and no subprocess spawn is recorded. AC6's module deletion stays as a
   structural check, but it is not sufficient alone.

3. **Make "identical-shaped" a concrete, maskable comparison** (addresses:
   testability "identical-shaped has no comparison procedure", clarity
   "rendered identically reads as byte-identical")
   Define "identical-shaped" as same field labels, ordering, and line
   structure, with a named mask set for volatile values (reuse
   `cli/vcs-test-support/fixtures/masks.toml`). Align the Summary/Requirements
   wording to "rendered in the same format regardless of backend".

4. **Specify the never-fail test precisely** (addresses: testability "Never-fail
   criterion is unbounded")
   Name the injection mechanism (a test-only failing adapter, not file
   permissions), pin the exact fallback string per subcommand, and state the
   concrete log-line assertion expected on the `ACCELERATOR_LOG` path.

5. **Define the conflict fixture and indicator** (addresses: testability
   "Conflict criterion leaves indicator and fixture construction undefined")
   Give the conflict-fixture recipe for each backend (e.g. a merge with
   conflicting edits to one tracked file) and state the exact indicator the
   output must contain (a defined marker plus the unmerged path name).

6. **Name the external dependencies and their policy artefacts** (addresses:
   dependency "External gix/jj-lib coupling unnamed", dependency "0188 captured
   only in References")
   Add `gix` and `jj-lib` to the Dependencies section, state whether the
   required status/diff and graph-walk surface fits the current
   `default-features = false` gix selection, and flag the `deny.toml` licence
   exception, the multi-way version pin, and the single-graph invariant as
   couplings. Promote 0188 from References to a done prerequisite, noting it
   landed the crates, the adapter module, and the pup rule this story extends.

7. **Clarify the `--fail-safe` relationship and consumer/committer framing**
   (addresses: clarity "Relationship between --fail-safe and never-fail
   unstated", clarity "the consumer and a committer used interchangeably")
   State whether the `(... unavailable)` fallback is unconditional or gated on
   `--fail-safe`, and use one consistent framing for who benefits from the
   conflict indicator (e.g. the `/commit` skill surfaces conflict state to the
   developer authoring the commit).

## Per-Lens Results

### Clarity

**Summary**: An unusually clear, self-aware work item — sole consumer defined
explicitly, non-goals stated repeatedly, and a likely misreading pre-empted
(cross-VCS uniformity is a test convenience, not user value). The main weakness
is an internal tension between Requirements (log fields stated as decided) and
Open Questions (same choice reopened), plus looser wordings around "identical"
output and the `--fail-safe`/never-fail relationship. Domain jargon is heavy but
almost entirely backed by links.

**Strengths**:
- The sole consumer is named unambiguously and repeatedly (the `/commit` skill,
  with a file path), so "the consumer" has a single stable referent.
- Scope non-goals are stated explicitly and consistently (byte-parity is a
  non-goal in Summary, Requirements, and Acceptance Criteria).
- Context actively reconciles a tension a reader might trip on — "identical
  output across backends" is a test simplification, not user value.
- Requirements use active, imperative phrasing with named data sources per
  backend (`gix` for git, `jj-lib` for jj).
- Specialised terms (`jj-lib`, `revset`, `dag_walk`, the jj/CLI crate split) are
  backed by explicit doc links, so undefined jargon rarely forces a guess.

**Findings**:
- 🟡 major (confidence: medium) — Requirements / Open Questions: Requirements
  defines the log firmly as "a flat list of recent commits (short id plus
  subject)" with no hedge, while Open Questions reopens exactly that choice
  ("short id plus subject only, or also author and date?"). The two sections
  give opposite signals about whether the field set is settled — depth is not
  affected (Requirements hedges depth to match its Open Question); only the field
  set is stated flatly then reopened. An implementer cannot tell whether
  author/date is in scope. Suggestion: mark the "short id plus subject" line as
  a provisional default pending the Open Question, or resolve the question and
  drop the alternative.
- 🔵 minor (confidence: medium) — Requirements: Context shows the skill invoking
  `vcs status --fail-safe`, implying the fallback is tied to the flag, but
  Requirements and AC5 describe never-fail without mentioning `--fail-safe`. It
  is unclear whether the `(... unavailable)` fallback is unconditional or gated
  on the flag. Suggestion: state explicitly which, and clarify the relationship
  to the flag's dispatch-layer role.
- 🔵 minor (confidence: medium) — Summary: Summary and Requirements say
  "rendered identically", while AC2 uses "identical-shaped". The same repo state
  yields different values across backends (git SHAs vs jj change-ids, branch vs
  bookmark), so literally identical output is impossible. Suggestion: align on
  the shape/format sense (e.g. "rendered in the same format regardless of
  backend").
- 🔵 suggestion (confidence: low) — Context: The item defines the sole consumer
  as the `/commit` skill (software) but attributes the conflict-indicator benefit
  to "a committer" and "that consumer" (the human), sliding between the two
  within one argument. Suggestion: use one consistent framing (the `/commit`
  skill surfaces conflict state to the developer authoring the commit).

### Completeness

**Summary**: An exemplary, highly complete story — every expected section is
present and substantively populated, frontmatter is valid and complete, and the
kind-specific content a story demands (identified consumer, clear motivation,
done-defining criteria) is all present. The Summary is a clear user-story
statement naming the sole consumer; the Context is unusually thorough. The only
observation is that a few format-defining decisions sit in Open Questions rather
than being resolved, which is acceptable for a draft.

**Strengths**:
- The Summary is an unambiguous user-story statement naming the concrete
  consumer and the no-runtime-dependency payoff, with a second paragraph pinning
  the exact technical change.
- The Context is exceptionally complete — why the work is needed, the prior
  decisions that led here, the single in-repo consumer, and an explicit
  Motivation subsection.
- Acceptance Criteria number seven specific bullets, well above the two-criterion
  bar.
- Kind-appropriate story content is fully present: beneficiary named, motivation
  argued, criteria define done.
- Frontmatter is complete and valid (kind: story, status: draft, priority,
  parent, relations).
- Optional sections relevant here (Dependencies, Assumptions, Open Questions,
  Technical Notes, Drafting Notes, References) are genuinely populated.

**Findings**:
- 🔵 suggestion (confidence: low) — Requirements / Open Questions: The status/log
  output format is the central deliverable (AC1 asks it be "specified and
  recorded") and Requirements sketch its shape, yet several format-defining
  choices — author/date on log lines, ahead/behind or parent on status, final
  log depth — are held in Open Questions rather than settled in Requirements. An
  implementer has the rough shape but must still resolve these (or produce the
  linked ADR) before the format is pinned. Acceptable for a draft; consider
  folding the format-defining Open Questions into AC1's ADR deliverable so the
  remaining work is unambiguous.

### Dependency

**Summary**: For a leaf cleanup story, 0198 is largely well-dependency-mapped —
it captures the done upstream (0169), correctly sequences the deletion of the
now-sole-user subprocess module against the completed 0185, names the single
`/commit` consumer, and captures jj-lib's pre-1.0 instability as an explicit
risk. The notable gaps are external: the gix/jj-lib crate dependencies and the
shared dependency-policy artefacts they couple to are named nowhere in the
Dependencies section, and 0188 (which landed those trees and the adapter module
this story extends) is captured only in References.

**Strengths**:
- The 0185→0198 shared-machinery coupling is captured precisely: 0185 removed
  `CommandProbe`, leaving status/log/`run_vcs_text` as the sole users of
  `vcs_adapters::subprocess`, correctly sequenced against the module deletion.
- jj-lib's pre-1.0 API instability is captured as an explicit external-dependency
  risk in Open Questions, Technical Notes, and Assumptions.
- The sole consumer (the `/commit` skill) is identified explicitly, and its
  coupling to the format change is surfaced as a cross-cutting ADR question.
- 0169 (done) is captured as the upstream that built the implementation being
  replaced, and the empty Blocks classification of 0199/0200/0201 is appropriate
  for a leaf cleanup.

**Findings**:
- 🔴 major (confidence: medium) — Dependencies: The implementation rests entirely
  on external crates `gix` and `jj-lib` and needs library surface 0188 did not
  deliver (working-copy diff via gix; revset/graph/dag_walk via jj-lib). Per
  0188 these carry a standing policy cost: a `cli/deny.toml` MPL licence
  exception, a four-to-six-way version pin, a single-gix-version invariant, and
  `gix` pinned `default-features = false`. A status/diff summary may need a gix
  feature that selection omits, re-opening that shared-policy review and the
  licence-closure re-check. The Dependencies section names only work items.
  Suggestion: name `gix` and `jj-lib` as external dependencies, state whether the
  required surface fits the current gix selection and jj-lib feature set, and
  flag the deny.toml exception, the multi-way pin, and the single-graph invariant
  as couplings.
- 🔵 minor (confidence: medium) — Dependencies: This story extends the gix/jj-lib
  trees and the library-backed adapter module 0188 (done) delivered, and per
  0169's amendment must widen the cargo-pup library-reads rule 0188 established
  to cover status/log. 0188 appears only in References, unlike the other done
  prerequisites 0169 and 0185 captured in Dependencies. A dependency graph built
  from Dependencies omits the item that landed the crates, the module, and the
  pup rule. Suggestion: add 0188 as a done prerequisite noting what it landed.

### Scope

**Summary**: 0198 is a coherent, well-bounded story — it replaces the last two
subprocess-backed `vcs` subcommands with a single in-process VCS-agnostic
renderer, and all seven requirements serve that one deliverable. The two
subcommands are genuinely indivisible (shared module and `run_vcs_text` helper;
the module can only be deleted once both migrate), so bundling them is a scope
virtue. Sizing (story) fits a single-team, single-consumer increment, and its
in/out-of-scope boundaries are unusually explicit.

**Strengths**:
- Unusually crisp scope boundary: byte-parity is an explicit non-goal, and any
  future jj-native richness is deferred to a separate structured subcommand.
- `status` and `log` are correctly treated as one unit — shared helper and
  module, one renderer, one consumer, deletable only once both migrate.
- Single-team and single-service: all work lives within the `cli/` VCS crates
  and one skill consumer, with the sibling cleanup (0185) already done.
- Cleanly separated from adjacent work (0199/0200/0201) so it does not absorb
  neighbouring concerns; the conflict indicator is scoped as a preserved signal,
  not a speculative feature.

**Findings**:
- 🔵 suggestion (confidence: low) — Open Questions: The story folds a
  cross-cutting design decision (the VCS-agnostic format definition) into an
  implementation story, and its own Open Questions flags this "changes the text
  the `/commit` skill injects ... a cross-cutting decision, not a local one"
  that may warrant an ADR. Embedding a design decision inside a build story is
  acceptable, but it is the one distinct decision unit whose resolution gates
  the rest. If the format proves contested, the whole story stalls behind a
  decision sequenced inside it. Suggestion: keep the decision in-scope but
  sequence it explicitly as the first step (the linked-ADR option AC1 allows),
  so the design is discharged before the adapters are built — no split needed.

### Testability

**Summary**: Well-framed, with several crisply verifiable criteria (module
deletion, `mise run` exit 0, and the two Given/When/Then behavioural criteria).
However, the acceptance gate hinges on a status/log format that is still a set
of unresolved Open Questions, so criteria referencing "the new format" and
"identical-shaped output" have no concrete threshold a verifier could execute
today. Most critically, the story's central promise — status/log work with no
runtime dependency on `jj`/`git` on `PATH` — is verified only structurally (by
deleting a module) rather than behaviourally, and the never-fail and conflict
criteria lack the fault-injection and fixture-construction precision that the
sibling 0169 applied to the same surface.

**Strengths**:
- AC2 and AC3 are expressed as observable Given/When/Then behaviours, the right
  framing for a story.
- AC6 (module deleted, zero occurrences) and AC7 (`mise run` exits 0) are
  unambiguous, mechanically checkable outcomes.
- Byte-parity is made an explicit non-goal, removing an untestable comparison
  target and replacing it with owned goldens.
- The conflict indicator is elevated to a hard, non-optional requirement, giving
  the conflict criterion a clear reason to exist.

**Findings**:
- 🔴 major (confidence: high) — Acceptance Criteria: The Summary's central
  promise is that status/log work "with no runtime dependency on those binaries
  being installed on `PATH`", yet no criterion runs the subcommands with
  `jj`/`git` absent from `PATH`. AC6 only asserts the module is deleted — a
  structural proxy a verifier could satisfy while tests pass with both binaries
  present. The primary stated outcome is not directly verifiable. Suggestion:
  add a PATH-stripped criterion mirroring 0169/0188 — with `jj`/`git` removed
  from `PATH`, output is correct across the fixture matrix and no subprocess
  spawn is recorded by a stub.
- 🟡 major (confidence: medium) — Acceptance Criteria: AC1 requires the format
  "specified and recorded", but the field set (parent? ahead/behind? author/
  date?), log depth ("keep five?"), and conflict marker are all still Open
  Questions — so AC2 ("identical-shaped output") and AC4 ("goldens updated")
  reference a format no verifier can check. Git and jj outputs genuinely differ
  (hash vs change-id, branch vs bookmark, ISO vs jj timestamps), so
  "identical-shaped" has no pass/fail without a defined normalisation/mask rule.
  Suggestion: resolve the format Open Questions and pin fields/ordering/markers/
  depth in AC1 (or a linked ADR); define "identical-shaped" as same labels,
  ordering, and line structure with a named mask set (`masks.toml` is the natural
  home).
- 🟡 major (confidence: medium) — Acceptance Criteria: AC5 ("any adapter failure
  yields `(... unavailable)`, diagnosable via `ACCELERATOR_LOG`") uses the
  unbounded term "any adapter failure" without specifying how a failure is
  triggered in a test, does not pin the exact fallback string (the `...` is a
  placeholder, presumably differing for status vs log), and does not say what a
  "diagnosable" run must assert. 0169 was explicit — inject via a test-only
  failing adapter or named env override, never file permissions. Suggestion:
  specify the injection mechanism, the exact fallback text per subcommand, and
  the concrete log-line assertion.
- 🟡 major (confidence: medium) — Acceptance Criteria: AC3 says the output
  "surfaces the conflict state" for a working copy "with a conflict/unmerged
  path" in git and jj, but defines neither the exact indicator to assert (marker
  string? named-path line?) nor how the conflict/unmerged state is produced in
  each backend. 0169 made such fixture states unambiguous precisely so two people
  build them identically. As written, "surfaces" could be met by any incidental
  mention, and two verifiers could construct different fixtures. Suggestion:
  define the conflict-fixture construction per backend and state the exact
  indicator the output must contain (a defined marker plus the unmerged path
  name).

---
*Review generated by /review-work-item*

## Re-Review (Pass 2) — 2026-08-30

**Verdict:** REVISE

The first-pass verdict driver is gone: all six original majors are resolved.
The re-review stays REVISE on three new testability majors — each a level
deeper than the originals, about pinning the acceptance criteria to concrete
goldens/fixtures and closing an absolute-path subprocess-spawn gap that 0188
already solved. None is structural; the item is close to implementation-ready.

### Previously Identified Issues

- 🟡 **Testability**: No behavioural no-PATH-dependency criterion — **Resolved**
  (a PATH-strip AC was added; but see the new absolute-path spawn finding).
- 🟡 **Testability**: Format deferred to Open Questions; no comparison procedure
  — **Resolved** (ADR-first AC1 plus mask-based parity against `masks.toml`).
- 🟡 **Testability**: Never-fail criterion unbounded — **Resolved** (fault
  injection via a test-only failing adapter, exact fallback text, log assertion).
- 🟡 **Testability**: Conflict criterion undefined — **Resolved** (fixture recipe
  plus ADR marker plus unmerged path name; the re-review calls it exemplary).
- 🟡 **Clarity**: Requirements vs Open Questions contradiction on log fields —
  **Resolved** (format Open Questions removed, Requirements firmed).
- 🔴 **Dependency**: External gix/jj-lib coupling unnamed — **Resolved** (0188
  promoted to blocker; crates and policy artefacts named in Dependencies).
- 🔵 **Clarity**: `--fail-safe`/never-fail relationship unstated — **Partially
  resolved** (now "unconditional, independent of `--fail-safe`"; the two failure
  domains still want one distinguishing sentence).
- 🔵 **Clarity**: "rendered identically" reads as byte-identical — **Resolved**
  (→ "rendered in that single format").
- 🔵 **Dependency**: 0188 only in References — **Resolved** (promoted to blocker).
- 🔵 **Completeness**: format choices in Open Questions — **Resolved** (folded
  into AC1's ADR deliverable).
- 🔵 **Scope**: sequence the format decision first — **Resolved** (ADR is AC1's
  first deliverable).
- 🔵 **Clarity**: consumer/committer framing — **Resolved**.

### New Issues Introduced

- 🟡 **Testability** (Acceptance Criteria): The PATH-strip AC says "produce
  correct output" across "the fixture matrix" — "correct" is undefined and the
  matrix is never enumerated. Enumerate it (cite 0169's status/log shapes) and
  replace "correct output" with "match the ADR-defined goldens".
- 🟡 **Testability** (Acceptance Criteria): Ordinary rendered content (change-type
  markers, counts, the five-entry log shape and its no-author/date/graph
  exclusions) is never pinned to a concrete golden — every criterion could pass
  on a wrong rendering. Add a dirty-repo status golden and a log golden with a
  negative assertion on author/date/graph.
- 🟡 **Testability** (Acceptance Criteria): The no-subprocess check (PATH removal
  plus a spawn stub) can pass vacuously against an absolute-path spawn from
  `gix`/`jj-lib`. 0188 rejected PATH-only shadowing and mandated absolute-path
  bind-mounts; adopt or reference that strong form.
- 🔵 **Clarity** + **Dependency** (cross-cutting, Frontmatter): The prose "Blocked
  by" 0169/0188 is not mirrored in a structured `blocked_by` frontmatter field
  (siblings carry one). Add `blocked_by: ["work-item:0169", "work-item:0188"]`.
- 🔵 **Clarity** (Acceptance Criteria): "the fixture matrix" is used with a
  definite article but never defined (same root as the testability major above).
- 🔵 **Clarity** (Requirements): "never fail", "`--fail-safe`", and "fallback"
  span two failure domains (adapter failure vs dispatch/fetch failure) with
  shared terms; add one sentence distinguishing them.
- 🔵 **Dependency** (Dependencies): Any newly-enabled `gix` feature must preserve
  `default-features = false` and not re-admit `gix-credentials`, or it breaks the
  item's own zero-subprocess premise (0188's invariant).
- 🔵 **Completeness** (Open Questions): The sole Open Question (jj-lib viability)
  is approach-invalidating but has no captured resolution path or fallback
  disposition.
- 🔵 **Scope** (Requirements): The story sits at the larger end (ADR + two
  adapters + goldens + fault injection); keep it whole, split only along the
  git/jj adapter seam if it proves too large.
- 🔵 **Clarity** (Context): "order-of-magnitude latency difference" overstates the
  ~5x cited figures (~3.6-4.7 ms vs ~23.8 ms); soften to "several-fold".

### Assessment

REVISE, but a materially better item than pass 1: all six original majors are
discharged and the previous suggestions are cleared. The remaining majors are
all testability precision — enumerate the fixture matrix, pin ordinary-content
goldens, and use 0188's absolute-path shadowing rather than PATH-only. The two
cross-cutting quick wins (`blocked_by` frontmatter; the fixture-matrix
definition) and the minor clarity/dependency notes would finish the item. One
more tightening pass should reach APPROVE.

## Re-Review (Pass 3) — 2026-08-30

**Verdict:** REVISE

All three pass-2 majors are resolved. Pass 3's three majors consolidate into two
root causes, and both were introduced by the pass-2 edits themselves: the
git-only fallback added to the Open Question makes half the acceptance criteria
contingent, and the staged-change fixture recipe is not reproducible in jj
(which has no staging area). Everything else is plan-level or belongs in the ADR
(AC1). Correcting the two self-inflicted defects should reach APPROVE.

### Previously Identified Issues

- 🟡 **Testability**: PATH-strip AC used "correct output" / undefined matrix —
  **Resolved** (fixture matrix enumerated; "match the ADR goldens").
- 🟡 **Testability**: Ordinary content never pinned to a golden — **Resolved**
  (dirty-repo status golden and log golden with negative assertion added).
- 🟡 **Testability**: No-subprocess check vacuous vs absolute-path spawns —
  **Resolved** (0188 absolute-path shadowing adopted in AC2).
- 🔵 **Clarity/Dependency**: `blocked_by` frontmatter missing — **Resolved**.
- 🔵 **Clarity**: "the fixture matrix" undefined — **Resolved** (enumerated).
- 🔵 **Clarity**: failure domains blur — **Resolved** (Requirement 7 distinguishes
  the internal fallback from dispatch `--fail-safe`).
- 🔵 **Dependency**: gix-feature/gix-credentials coupling — **Resolved**.
- 🔵 **Completeness**: jj-lib Open Question lacked a resolution path — **Resolved**
  (but the fallback it added is now the source of a new major; see below).
- 🔵 **Scope**: story is large — **Resolved** (decomposition rationale recorded).
- 🔵 **Clarity**: "order-of-magnitude" wording — **Resolved** ("several-fold").

### New Issues Introduced

- 🟡 **Testability** + 🟡 **Clarity** + 🔵 **Scope** (cross-cutting, root cause A):
  The git-only fallback added to the Open Question in pass 2 makes AC2/AC3/AC5/
  AC7/AC8 ("both backends"; subprocess deleted) unsatisfiable under that branch,
  with no reduced acceptance set and no signal that the criteria are contingent.
  Fix: state the ACs assume full migration and that the git-only outcome is a
  deliberate re-scope into a follow-up item (jj adapter + module deletion), not a
  degraded pass — or gate the ACs on resolving the Open Question first.
- 🟡 **Testability** + 🔵 **Clarity** (cross-cutting, root cause B): The
  staged-change fixture in AC3/AC4 is not reproducible in jj (no staging area),
  and a "staged" change-type marker only git can emit conflicts with cross-backend
  parity. 0169 scoped staging to git explicitly. Fix: qualify the staged
  component as git-only and have AC1's ADR state whether staging collapses to
  "modified" in the agnostic format.
- 🔵 **Testability** (Acceptance Criteria): AC1 requires the ADR to enumerate
  topics but not to fix exact verbatim marker/fallback strings, so downstream
  "matches the ADR" criteria can inherit looseness. Strengthen AC1 to require
  verbatim glyphs/strings — or accept this as ADR-authoring scope.
- 🔵 **Testability** (Acceptance Criteria): AC3's masked parity lacks 0169's
  mask-closure rule (no mask may be added to rescue a failing comparison; an
  unmasked control must show masks cover only volatile values).
- 🔵 **Testability** (Acceptance Criteria): AC1's "ADR authored first" has no
  verification mechanism; 0169 tied ordering to commit history.
- 🔵 **Dependency** (Dependencies): 0185 is a precondition for the module-deletion
  AC (clean full-module deletion depends on its CommandProbe removal) but sits in
  "Relates to", not as a completed precondition.
- 🔵 **Dependency** (Acceptance Criteria): AC2's absolute-path zero-spawn leans on
  0188's privileged Linux CI job (`check-zero-spawn`), which should be named as
  the surface this item extends.
- 🔵 **Completeness** (Requirements): The format is enumerated by field but never
  shown; an indicative example would help (consciously deferred to the ADR).
- 🔵 **Scope** (Open Questions): The git-only fallback should be marked a
  deliberate re-scope into a follow-up, not a degraded pass (same root as A).
- 🔵 **Clarity** (Context): "one exception, conflict state" is attached to a list
  conflict state is not a member of; separate the dropped-and-unused fields from
  the one droppable-but-used field.
- 🔵 **Testability** (Motivation): Performance is a stated motivation with a
  `performance` tag but no criterion; note it is motivation-only, or add a light
  measured-not-gated check.

### Assessment

REVISE, but the item is planning-ready once two small, self-inflicted defects
are fixed: the fallback-contingency (root cause A) and the jj staging fixture
(root cause B). Both were introduced by the pass-2 edits. The remaining findings
are plan-level precision (mask-closure, commit-order verification, verbatim ADR
strings) that properly belong in the ADR (AC1) and the implementation plan, not
in further work-item tightening. Recommendation: correct A and B, optionally fold
the cheap cross-referenced notes (0185 precondition, `check-zero-spawn`,
mask-closure), and stop — the tightening spiral has reached diminishing returns.

### Close-out (applied after pass 3, no re-review)

Both root-cause defects and the three cheap notes were applied to the work item:
the git-only fallback now reads as a deliberate re-scope (follow-up item), not a
degraded pass; AC3/AC4 qualify the staged change as git-only with AC1's ADR
deciding whether staging collapses to "modified"; 0185 is noted as a completed
module-deletion precondition; AC2 names 0188's `check-zero-spawn` Linux CI job;
and AC3 carries 0169's mask-closure rule. The remaining plan-level findings
(verbatim ADR strings, commit-order verification, performance-is-motivation-only,
an indicative output example, the Context "one exception" prose) were left for
`/create-plan` and AC1's ADR.

### Verdict: APPROVE (reviewer acceptance, 2026-08-30)

The reviewer (Toby Clemson) accepted the item as APPROVE after the close-out
fixes and moved the work item to `ready`. This is a reviewer override, not the
output of a fourth lens pass — no verifying pass was run after the close-out
edits. The pass-3 findings that remain open are the plan-level ones listed above,
deliberately deferred to AC1's ADR and `/create-plan`.
