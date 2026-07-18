---
type: work-item-review
id: "0167-config-command-and-invocation-contract-migration-review-1"
title: "Work Item Review: Built-in config Command and Invocation-Contract Migration"
date: "2026-07-18T20:29:30+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0136"
target: "work-item:0167"
work_item_id: "0167"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 5
tags: [rust, config, skills, invocation-contract, migration]
last_updated: "2026-07-19T18:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Built-in config Command and Invocation-Contract Migration

**Verdict:** REVISE

0167 is an unusually well-researched work item — the surface is quantified
rather than gestured at (247 call sites, 46 SKILL.md files, one glob across 35
frontmatter blocks, 337 assertions in 6,289 lines), exclusions are stated
affirmatively with reasons, and the Drafting Notes record which requirements
were dropped or narrowed and why. The problems are concentrated on two seams.
First, the story's dominant risk — irreversible behavioural-parity loss when
`scripts/test-config.sh` is deleted in the same change — rests on an
unenumerable criterion ("every documented behaviour is covered") whose source of
truth is never named. Second, the Summary, Requirements, and Acceptance Criteria
disagree with each other on three material scope questions: whether the 0107
guard is built here, whether "every SKILL.md call site" means the config cluster
or the whole plugin, and whether the exit-code taxonomy is settled or open.

### Cross-Cutting Themes

- **The parity denominator is undefined** (flagged by: testability, clarity) —
  AC1 and the Assumptions both make parity conditional on "the documented bash
  behaviour", but no artefact is named as that documentation. Since the
  Assumptions also say the suites encoding the behaviour are deleted in the same
  story, the phrase reduces circularly to "port the suites". This is the
  highest-severity finding in the review and the one most likely to cause
  irreversible loss.
- **Requirements exist that no acceptance criterion covers** (flagged by:
  completeness, scope, testability) — the `config-detect`/SessionStart hook
  migration, the `configure`-first round-trip proof (ADR-0045), and the
  byte-exact output contract (bare value + single newline, three byte-exact
  injection blocks, help-text-as-contract) are all stated as requirements and
  absent from the seven criteria. A verifier working the AC list would mark the
  story done with hooks still on bash.
- **0107's disposition is stated three incompatible ways** (flagged by: clarity,
  scope) — the Summary says the guard is rewritten "in lockstep", Dependencies
  says it is "not yet implemented; this story either builds it or defers it",
  and AC4 only requires that the chosen disposition be *recorded*. The story's
  size swings by a whole independent work item depending on which is true.
- **Unqualified "every"** (flagged by: clarity, scope, testability) — Summary and
  Requirements say "every SKILL.md call site" without qualification; only a
  parenthetical in AC3 narrows it to the config cluster. The two readings differ
  by an order of magnitude and change the boundary against 0173.
- **The frontmatter dependency graph disagrees with the prose** (flagged by:
  dependency, completeness) — `blocked_by` is absent entirely despite two named
  blockers, and `blocks` omits 0174 even though 0174 declares itself blocked by
  0167.

### Findings

#### Critical

- 🔴 **Testability + Clarity**: AC1's parity criterion is unbounded and its
  source of truth is undefined
  **Location**: Acceptance Criteria (AC1) / Assumptions
  AC1 reads "every documented behaviour is covered", but the work item never
  enumerates that behaviour, names where it lives, or defines what "covered"
  means. The Assumptions concede the point — parity is measured against
  "documented" bash behaviour precisely because the suites encoding it are
  deleted in the same change — yet no inventory artefact is named as a
  deliverable. The story's single largest risk rests on a criterion that can
  neither be definitively passed nor failed.

#### Major

- 🟡 **Clarity + Scope**: 0107's disposition is contradictory between Summary,
  Dependencies, and Acceptance Criteria
  **Location**: Summary / Dependencies / Acceptance Criteria (AC4)
  The Summary places the guard inside this story's boundary; no Requirement
  mentions building it; AC4 asks only that the disposition be recorded. 0107 is
  a separate task item with six criteria of its own and an unresolved
  glob-semantics prerequisite.

- 🟡 **Clarity + Scope + Testability**: "every SKILL.md call site" is ambiguous
  between the config cluster and all plugin-script invocations
  **Location**: Summary / Requirements / Acceptance Criteria (AC3)
  Only AC3's parenthetical narrows it. The narrow reading is corroborated by the
  0173 boundary, but the broad reading is what the Summary and Requirements
  literally say.

- 🟡 **Clarity**: Exit-code taxonomy is a settled requirement and an open
  question simultaneously
  **Location**: Requirements / Open Questions
  Two Requirements pin the 0/1/2 taxonomy (ADR-0021); the final Open Question
  asks whether to preserve it or match luminosity's uniform exit 1. Notably the
  `--format` question was handled by the opposite move — dropped from
  Requirements *before* being listed as open — which makes this look like an
  oversight rather than deliberate tension.

- 🟡 **Completeness + Scope + Testability**: Hook migration and the
  `configure`-first round trip have no acceptance criteria
  **Location**: Requirements / Acceptance Criteria
  Both are explicit Requirements. AC3 covers SKILL.md call sites only; hooks are
  a separate surface, and the `configure`-first proof has no stated evidence
  artefact.

- 🟡 **Testability**: The output contract has no criterion
  **Location**: Requirements / Acceptance Criteria
  "Scalar reads emit one bare value plus a single newline", the three byte-exact
  injection blocks, and help-text-as-contract are all specified in Requirements
  and absent from the criteria. A stray trailing newline silently corrupts
  injected prompts across 247 call sites with no check to catch it.

- 🟡 **Testability**: AC2's blanket exit-0 expectation contradicts the required
  fail-closed exceptions
  **Location**: Acceptance Criteria (AC2)
  AC2 says a config error at a spliced call site "exits 0", unqualified. The
  Requirements mandate retaining three deliberate fail-closed exceptions
  (frontmatter writeback primitives, the `work.integration` enum, doc-type path
  safety) which by definition must not. A test written to AC2 would enforce
  exactly the wrong behaviour for those three.

- 🟡 **Testability**: "No added permission prompts" (AC3) has no verification
  procedure
  **Location**: Acceptance Criteria (AC3)
  Permission prompting is runtime harness behaviour with no stated observation
  method, and Open Question 1 admits the matcher's glob semantics are unknown —
  so even a static coverage check cannot be computed. 0106 solved exactly this
  framing problem by reframing the runtime "no prompt" criterion into inspectable
  conditions.

- 🟡 **Scope**: The story bundles four independently deliverable bodies of work,
  and documents the seam itself
  **Location**: Requirements
  Command implementation, the 6,289-line test port, the 247-call-site rewrite,
  and hook migration. The Requirements supply the separating seam — bash and
  `accelerator` call sites may coexist behind a dual `allowed-tools` set — which
  weakens the "must land together" rationale. No intermediate deliverable can be
  reviewed, merged, or reverted on its own.

- 🟡 **Dependency + Completeness**: Frontmatter omits `blocked_by` and
  under-reports `blocks`
  **Location**: Frontmatter
  Dependencies names 0166 and 0164 as blockers; the frontmatter has no
  `blocked_by` field. Siblings do record it (0166, 0169). 0174 declares
  `blocked_by: ["work-item:0167", …]` while 0167's `blocks` omits 0174. Any
  scheduling or sync that reads the graph sees the epic's highest-blast-radius
  story as unblocked.

- 🟡 **Dependency**: 0165 (distribution pipeline) is an implied blocker but is
  not listed
  **Location**: Dependencies
  This is the first production exercise of a launcher that is fetched and
  verified at first use. If the rewrite ships before 0165 publishes signed
  artefacts for every platform, every migrated call site fails at skill-load
  time across all 46 SKILL.md files.

- 🟡 **Dependency**: The conditional ordering against 0107 is not expressed as a
  dependency
  **Location**: Open Questions / Dependencies
  If "build-then-migrate" is chosen, 0107 becomes an upstream blocker — and 0107
  itself declares `blocked_by: ["work-item:0106"]`, so 0167 inherits the chain.
  Neither item records this from either side.

- 🟡 **Completeness**: The browser-executor glob re-homing is noted but never
  becomes a requirement or criterion
  **Location**: Technical Notes
  The item identifies a concrete obligation created by its own change — the
  `config-read-browser-executor.sh` glob coverage must be re-homed when the
  `config-*` pattern is retired — then leaves it unassigned. An implementer
  following only Requirements and Criteria retires the glob and reintroduces the
  permission-prompt symptom 0106/0107 exist to prevent.

- 🟡 **Clarity**: Whether this story builds or consumes the bootstrap path is
  contradictory, and "the wrapper" is an undefined synonym
  **Location**: Requirements / Dependencies / Assumptions
  Requirement 5 says "**Provide** the stable bash bootstrap path"; Dependencies
  says the story is blocked by 0164 "(the bootstrap + launcher)". Separately,
  "0169's wrapper model" introduces "the wrapper" with a definite article though
  no wrapper is named anywhere — it may or may not be the bootstrap path.

- 🟡 **Clarity**: "The config reader entrypoints" names a set whose membership is
  never pinned down
  **Location**: Acceptance Criteria (AC7)
  This is the criterion gating shell deletion, but the set is never enumerated.
  The Technical Notes list includes items that are plainly not readers
  (template-management scripts mutate; `init.sh` bootstraps), and
  `config-read-browser-executor.sh` matches the pattern while being explicitly
  out of scope.

- 🟡 **Testability**: AC5 can pass while `config set` destroys the surrounding
  Markdown body
  **Location**: Acceptance Criteria (AC5) / Open Questions
  A round-trip read is satisfied equally by an implementation that splices the
  body back verbatim and one that rewrites the file down to bare frontmatter.
  The most destructive plausible defect in a net-new write path is invisible to
  the criterion guarding it.

#### Minor

- 🔵 **Clarity + Completeness + Testability**: "`_EXPECTED_CONFIG_SUITES` is
  decremented from 21" states no target value
  **Location**: Acceptance Criteria (AC6)
  Two suites are removed and one (`test-init.sh`) is added in the same change,
  so 19 or 20 are both plausible — and it is unclear whether `test-init.sh`
  counts toward this constant at all. The guard's whole value is exactness.

- 🔵 **Clarity**: "(resolved Q3)" is a dangling reference
  **Location**: Assumptions
  There is no numbered question list in this work item; the Open Questions are
  unnumbered bullets and none concerns the bootstrap path's location.

- 🔵 **Clarity**: Singular "the `allowed-tools` glob" presumes a resolution Open
  Question 1 says is unknown
  **Location**: Requirements / Acceptance Criteria (AC3)
  If the per-subcommand outcome obtains, the criterion as phrased is not
  satisfiable.

- 🔵 **Dependency**: Blocker cited as 0166 rather than the concrete children that
  gate the work
  **Location**: Dependencies
  0166 has been decomposed (0178, 0179, 0180), only some complete. Sibling 0169
  already cites the concrete child alongside the parent.

- 🔵 **Dependency**: Downstream subdomain migrations 0170/0171/0172 are not
  listed as blocked
  **Location**: Dependencies
  All three rewrite call sites and globs against the contract this story defines,
  so a contract change here has no visible list of consumers to re-check.

- 🔵 **Dependency**: The external coupling on Claude Code's `allowed-tools`
  matcher semantics is not recorded as a dependency
  **Location**: Open Questions
  A vendor behaviour that decides between one glob and per-subcommand globs
  across 35 frontmatter blocks appears only as an open question, with no owner,
  resolution route, or verified Claude Code version.

- 🔵 **Dependency**: The runtime dependency on binary fetch availability at
  skill-load time is not captured
  **Location**: Requirements
  The splice-safety requirement covers config errors but not an unfetchable or
  unverifiable launcher, which produces a non-zero exit at all 247 call sites
  regardless of how well the config layer degrades.

- 🔵 **Scope**: Wiring `test-init.sh` into `run_shell_suites` is an orthogonal
  CI-discovery chore
  **Location**: Requirements
  It also sits oddly beside the rest of the scope: the Technical Notes list
  `init.sh` as part of the bash surface being migrated, so this newly wires a
  shell suite into CI for a script the same story replaces.

- 🔵 **Scope**: Hook migration is asserted in scope by Requirements and absent
  from Acceptance Criteria
  **Location**: Requirements / Acceptance Criteria
  Makes the boundary against 0169's wrapper model ambiguous at completion time.

- 🔵 **Testability**: AC4's 0107 clause is self-satisfying
  **Location**: Acceptance Criteria (AC4)
  Any text written to 0107 satisfies "is recorded"; "0106's contract is updated"
  is verifiable only as "the file changed".

- 🔵 **Testability**: "The write was atomic (temp file then rename)" specifies
  mechanism, not observable outcome
  **Location**: Acceptance Criteria (AC5)
  And "malformed" is undefined — unterminated frontmatter? invalid YAML?
  duplicate keys? the symlink escape the Technical Notes require?

- 🔵 **Clarity**: The 46-files versus 35-frontmatter-blocks gap is unexplained
  **Location**: Context
  Since the central risk is keeping call sites and globs in lockstep, an
  unexplained eleven-file mismatch leaves the true surface uncertain.

#### Suggestions

- 🔵 **Testability**: AC3 lacks a canonical enumeration command as its
  denominator
  **Location**: Acceptance Criteria (AC3)
  0106 — the direct predecessor for this check — pinned its denominator with an
  explicit `grep` and stated that the grep result *is* the denominator. Without
  one, "every" is unbounded and two verifiers can compute different totals.

- 🔵 **Testability**: The built-in-vs-external decision rests on a latency band
  with no criterion
  **Location**: Assumptions
  The 20-30ms band justifies a significant architectural choice and is never
  verified; a native implementation materially slower than the bash it replaces
  would pass every criterion.

- 🔵 **Scope**: `config set` is net-new capability inside a parity cutover
  **Location**: Requirements
  It carries its own sub-scope (atomic writes, malformed refusal, symlink-escape
  refusal, gitignoring, an undecided body-preservation question) and nothing
  else in the story depends on it.

### Strengths

- ✅ Every expected section is present and substantively populated — no
  placeholders, and seven Given/When/Then acceptance criteria, well above the
  two-criterion floor.
- ✅ The surface is quantified rather than gestured at (247 call sites, 46 files,
  one glob across 35 frontmatter blocks, 337 assertions in 6,289 lines), which
  makes the sizing claim auditable.
- ✅ The uniformity claim carries its evidence inline ("every one of the shape
  … — no wrappers, no pipes, no quoting variants"), so the de-risking argument is
  verifiable rather than asserted.
- ✅ Drafting Notes are exemplary: they name which requirements were dropped or
  narrowed and why, so a reader can distinguish deliberate decisions from
  oversights. The `--format` investigation is a model of this.
- ✅ Exclusions are stated affirmatively with reasons and residual obligations —
  `config-read-browser-executor.sh` (naming accident), `config-common.sh`
  (44 consumers, retirement deferred to 0173/0174), VCS hooks (0169).
- ✅ The three fail-closed exceptions are enumerated explicitly rather than left
  as "certain safety-critical cases".
- ✅ AC5, AC6, and AC7 are individually strong — each names concrete artefacts or
  an existing gate and admits a definitive pass/fail procedure.
- ✅ Requirements prescribe a concrete test harness design (spawn the real
  binary, assert exact bytes, fixture workspaces with a `.git` marker bounding
  root discovery) rather than leaving it open.
- ✅ The incremental-migration escape hatch is explicit, with a stated
  coexistence mechanism.

### Recommended Changes

1. **Make the parity behaviour inventory an explicit deliverable and AC1's
   denominator** (addresses: AC1 unbounded; "documented bash behaviour"
   undefined)
   Add a requirement to extract a behaviour inventory from
   `scripts/test-config.sh` and `scripts/test-config-read-doc-type-paths.sh`
   (one row per assertion group) and commit it *before* deletion. Rewrite AC1 so
   every row must map to at least one named Rust test, with the mapping checked
   in. This converts "every documented behaviour" into a countable, reviewable
   set and is the single highest-value change in this review.

2. **Settle the 0107 disposition before the story starts and state it
   consistently** (addresses: 0107 contradiction; conditional ordering not a
   dependency; AC4 self-satisfying)
   Either drop "and the 0107 lint guard" from the Summary and keep only the
   disposition-recording criterion, or add an explicit Requirement plus criterion
   for building the guard. If "build-then-migrate" is chosen, record 0107 — and
   transitively 0106 — as blockers in Dependencies and frontmatter.

3. **Qualify "every SKILL.md call site" at every occurrence** (addresses:
   ambiguous scope of the rewrite)
   Say "every SKILL.md call site *to the config cluster*" in the Summary and
   Requirements, and state explicitly that non-config call sites remain on bare
   paths until 0173.

4. **Add the missing acceptance criteria** (addresses: hooks, `configure` round
   trip, output contract)
   Three additions: (a) the SessionStart hook emits a summary produced by
   `accelerator config summary` matching the pre-migration bash output
   byte-for-byte; (b) the `configure` skill resolves every call site through
   `accelerator` end-to-end against a fixture workspace, with a committed golden
   capture; (c) scalar reads emit exactly the value plus a single `\n` with empty
   stderr, the three injection blocks match committed golden files byte-for-byte,
   and `--help` matches a committed snapshot.

5. **Scope AC2 to the fail-open surface and add a fail-closed companion**
   (addresses: AC2 contradicts the three exceptions)
   "Given a fail-open config error, then exit 0 with an `## … Unavailable`
   notice; given a frontmatter-writeback, `work.integration`-enum, or
   doc-type-path error, then exit non-zero with nothing on stdout." Name the
   specific error trigger for each of the three.

6. **Reframe AC3's runtime clause into inspectable conditions, following 0106's
   precedent** (addresses: "no added permission prompts" unverifiable; AC3 lacks
   a denominator; singular-glob presumption)
   Replace the prompt clause with a static assertion over a canonical `grep`
   denominator, phrased in terms of coverage rather than glob cardinality so
   either resolution of Open Question 1 satisfies it. Make resolving that
   question a stated precondition.

7. **Reconcile the frontmatter dependency graph with the prose** (addresses:
   missing `blocked_by`; `blocks` omits 0174; 0165 unlisted; 0166 too coarse)
   Add `blocked_by: ["work-item:0164", "work-item:0166"]`, extend `blocks` with
   `work-item:0174`, name the concrete gating children of 0166 (0178/0179), and
   either add 0165 to Blocked by or state why the migration only needs a locally
   built binary until release.

8. **Consider splitting along the seam the item already documents** (addresses:
   four bundled bodies of work; `config set` net-new in a parity cutover)
   One story for the built-in command, bootstrap path, Rust test port, and the
   widened dual `allowed-tools` set (nothing user-visible changes); a second for
   the call-site/glob cutover, suite deletion, and the counter decrement. If the
   split is rejected, record in Drafting Notes why the documented coexistence
   mode is insufficient as a delivery boundary. Consider extracting `config set`
   to a follow-on so this story stays a pure parity cutover.

9. **Pin the underdetermined values and referents** (addresses: counter target;
   "reader entrypoints" set; "resolved Q3"; bootstrap-vs-wrapper naming; 46-vs-35
   gap)
   State `_EXPECTED_CONFIG_SUITES`' target value and whether `test-init.sh`
   counts toward it; enumerate the reader-entrypoint deletion set explicitly (or
   give a rule plus its named exceptions); replace "(resolved Q3)" with a pointer
   to the resolving document; use one consistent name for the bootstrap
   path/wrapper artefact; explain the eleven-file gap between call sites and
   frontmatter blocks.

10. **Resolve the `config set` body-preservation question and extend AC5**
    (addresses: AC5 passes while prose is destroyed; atomicity as mechanism;
    "malformed" undefined)
    Add either "all content outside the edited key is byte-identical" or a
    committed expected-rewrite fixture; reframe atomicity as an observable
    outcome (a concurrent reader sees complete pre- or post-write state, never
    partial; no temp artefacts remain after success or failure); enumerate two or
    three named malformed fixtures including the symlink-escape case.

11. **Assign or defer the browser-executor glob re-homing** (addresses: noted
    obligation with no owner)
    Add a requirement stating whether this story retains a narrow
    `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-read-browser-executor.sh)` rule or
    defers it, and mirror the disposition in a criterion.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: An unusually well-written work item: prose is dense but purposeful,
quantities are precise, and the Drafting Notes record where earlier ambiguities
were resolved. Remaining clarity problems concentrate in three places — the
Summary overstates scope relative to Requirements/Open Questions on two points
(the 0107 guard, "every SKILL.md call site"), one decision is simultaneously
settled and reopened (exit-code taxonomy), and several load-bearing noun phrases
("the stable bash bootstrap path", "the documented bash behaviour", "the config
reader entrypoints") are used as if defined when their referent or set boundary
is not pinned down anywhere.

**Strengths**: Drafting Notes distinguish deliberate decisions from oversights;
quantities precise rather than vague; uniformity claim states its evidence; the
three fail-closed exceptions enumerated explicitly; exclusions carry rationale
and residual obligations inline.

**Findings**:

- 🟡 **major / high** — Summary claims the 0107 lint guard is rewritten in
  lockstep; the rest of the item says it may not be built at all
  (*Location*: Summary). Requirements never mention the guard; Dependencies calls
  it "not yet implemented"; AC4 only requires the disposition be recorded. A
  reader who stops at the Summary scopes a new CI lint guard into the story.

- 🟡 **major / high** — Exit-code taxonomy is stated as a settled requirement and
  simultaneously reopened as an open question (*Location*: Requirements).
  Requirements 1 and 3 pin the 0/1/2 taxonomy; the final Open Question asks
  whether to preserve or match luminosity. The `--format` question was handled by
  the opposite move, suggesting oversight.

- 🟡 **major / high** — "every SKILL.md call site" is ambiguous between all skill
  call sites and config-cluster call sites only (*Location*: Summary /
  Requirements / Acceptance Criteria). Only AC3's parenthetical narrows it; the
  two readings differ enormously in size and change the 0173 boundary.

- 🟡 **major / medium** — "the documented bash behaviour" has no identified
  referent and reads circularly (*Location*: Assumptions / Requirements /
  Acceptance Criteria). The Assumptions imply the documentation *is* the deleted
  suites, reducing "write tests against the documentation" to "port the suites".

- 🟡 **major / medium** — Whether this story builds or merely consumes the stable
  bash bootstrap path is contradictory, and "the wrapper" is an undefined synonym
  (*Location*: Requirements / Assumptions). Requirement 5 says "Provide";
  Dependencies says blocked by 0164 "(the bootstrap + launcher)".

- 🟡 **major / medium** — "the config reader entrypoints" names a set whose
  membership is never pinned down (*Location*: Acceptance Criteria). The
  Technical Notes list includes non-readers, and
  `config-read-browser-executor.sh` matches the pattern while being out of scope.
  This is the criterion gating deletion.

- 🔵 **minor / high** — "(resolved Q3)" is a dangling reference with no Q3 in
  this work item (*Location*: Assumptions).

- 🔵 **minor / medium** — "`_EXPECTED_CONFIG_SUITES` is decremented from 21" does
  not state the resulting value, and a second unrelated "21" appears nearby
  ("flag parsing across all 21 entrypoints") (*Location*: Acceptance Criteria).

- 🔵 **minor / medium** — Singular "the `allowed-tools` glob" presumes a
  resolution the Open Questions say is unknown (*Location*: Requirements /
  Acceptance Criteria).

- 🔵 **minor / low** — The 46-files versus 35-frontmatter-blocks gap is stated
  without explanation (*Location*: Context).

### Completeness

**Summary**: Densely and unusually well-populated: every expected section is
present with substantive, quantified content, and the Drafting Notes record why
scope decisions were taken. The main gaps are on the Requirements→Acceptance
Criteria seam — several requirements (config hook migration, the
`configure`-skill round-trip proof, help-text-as-contract) have no corresponding
criterion, and the `config-read-browser-executor.sh` glob dependency is described
in Technical Notes but never becomes a requirement or criterion. Frontmatter
omits `blocked_by` despite the body naming two blockers.

**Strengths**: Every expected section present and substantively populated;
Context exceptionally concrete and quantified; seven Given/When/Then criteria,
well above the two-criterion floor; Assumptions and Open Questions carry
load-bearing content rather than filler; Drafting Notes explain scope reversals;
frontmatter carries all required identity and lifecycle fields.

**Findings**:

- 🟡 **major / high** — Several requirements have no corresponding acceptance
  criterion (*Location*: Acceptance Criteria). Hook migration, the
  `configure`-first round trip, and help-text-as-contract are all unmatched. For
  the story its own author calls "the highest-blast-radius story in the epic",
  parts of the stated work have no definition of done.

- 🟡 **major / medium** — browser-executor glob re-homing is noted but never
  becomes a requirement or criterion (*Location*: Technical Notes). The item
  identifies a follow-on obligation created by its own change and leaves it
  unassigned, reintroducing the permission-prompt symptom 0106/0107 prevent.

- 🔵 **minor / medium** — Frontmatter omits `blocked_by` despite the body naming
  two blockers (*Location*: Frontmatter: blocked_by). The outbound edge is
  machine-readable while the inbound edge exists only in prose; the item is
  tracker-synced (`external_id: PP-188`).

- 🔵 **minor / medium** — Suite-count criterion does not state the target value
  (*Location*: Acceptance Criteria). Two removals and one addition in the same
  change make the arithmetic non-obvious.

- 🔵 **minor / medium** — Open Questions gate acceptance criteria but carry no
  resolution path (*Location*: Open Questions). The glob-semantics question has
  been open across two work items with no owner, route, or timing.

### Dependency

**Summary**: 0167 captures its principal couplings in prose well — 0166/0164 as
blockers, 0169/0173 as dependants, the 0106/0107 relationship, and a clear
ordering rationale for migrating config hooks here while leaving VCS hooks to
0169. The gaps are threefold: the frontmatter graph is incomplete and asymmetric
relative to siblings, the distribution/runtime couplings implied by being the
first production exercise of a fetched launcher (0165, GitHub Releases
availability at skill-load time) are absent, and the conditional ordering against
0107 sits in Open Questions rather than Dependencies.

**Strengths**: Dependencies names both upstream blockers and downstream
dependants with rationale; the hook-migration ordering is explicit and reciprocal
with 0169's own `blocked_by`; the 0106/0107 relationship is deliberately reframed
rather than assumed; the browser-executor exclusion carries a forward-looking
coupling note; incremental migration is permitted with an explicit coexistence
mechanism.

**Findings**:

- 🟡 **major / high** — Frontmatter dependency graph omits `blocked_by` entirely
  and under-reports `blocks` (*Location*: Frontmatter). Siblings 0166 and 0169
  both record `blocked_by`; 0174 declares itself blocked by 0167 while 0167's
  `blocks` omits it.

- 🟡 **major / medium** — 0165 (distribution and release pipeline) is an implied
  blocker but is not listed (*Location*: Dependencies). The launcher is fetched
  and verified at first use; shipping before 0165 publishes signed artefacts
  fails every migrated call site at skill-load time.

- 🟡 **major / medium** — Conditional ordering against 0107 (and transitively
  0106) is not expressed as a dependency (*Location*: Open Questions). A late
  "build-then-migrate" decision silently introduces two upstream items into the
  critical path.

- 🔵 **minor / medium** — Blocker cited as 0166 rather than the concrete children
  that gate the work (*Location*: Dependencies). 0166 is now an umbrella over
  partially-complete children (0178/0179/0180); 0169 already cites the concrete
  child.

- 🔵 **minor / medium** — Downstream subdomain migrations 0170/0171/0172 are not
  listed as blocked (*Location*: Dependencies).

- 🔵 **minor / high** — External coupling on Claude Code's `allowed-tools` matcher
  semantics is not recorded as a dependency (*Location*: Open Questions). No
  owner, resolution route, or verified Claude Code version.

- 🔵 **minor / medium** — Runtime dependency on binary fetch availability at
  skill-load time is not captured (*Location*: Requirements). The splice-safety
  guarantee is only as strong as its weakest layer.

### Scope

**Summary**: A deliberately bundled cutover story: it builds the native
`accelerator config` command and simultaneously rewrites the plugin-wide
invocation contract, ports a 6,289-line shell suite, migrates config hooks, and
adds a net-new `config set`. The bundling rationale is sound and explicitly
argued, and boundaries with 0169/0173/0174 and with
`config-common.sh`/`config-read-browser-executor.sh` are unusually well drawn.
The main concerns: the item's own escape hatch (coexisting call sites behind a
dual `allowed-tools` set) undermines the indivisibility claim and reveals a clean
seam; the 0107 guard's in-scope status is contradictory; and one orthogonal
CI-hygiene chore has been folded in.

**Strengths**: Exclusions stated affirmatively with reasons; the bundling
decision is argued rather than accidental; scope is quantified and auditable; the
item's position in the epic's dependency spine is coherent; Requirements draw a
defensible line between parity migration and new invention.

**Findings**:

- 🟡 **major / high** — Summary places the 0107 lint guard inside the boundary
  while Acceptance Criteria require only that the disposition be recorded
  (*Location*: Summary / Acceptance Criteria). The story's size swings by a whole
  independent work item depending on an undecided Open Question.

- 🟡 **major / medium** — The story bundles at least four independently
  deliverable bodies of work, and its own Requirements supply the separating seam
  (*Location*: Requirements). A story its author calls the highest-blast-radius
  in the epic has no intermediate deliverable that can be reviewed, merged, or
  reverted alone. Suggested split: command + bootstrap + test port + widened
  dual glob (nothing user-visible changes), then the cutover + deletion +
  counter decrement.

- 🔵 **minor / high** — Wiring `test-init.sh` into `run_shell_suites` is an
  orthogonal CI-discovery fix (*Location*: Requirements), and may be undone
  within the same story since `init.sh` is listed as part of the migrated bash
  surface.

- 🔵 **minor / medium** — Hook migration is in scope per Requirements and absent
  from Acceptance Criteria (*Location*: Requirements / Acceptance Criteria),
  making the boundary against 0169's wrapper model ambiguous at completion time.

- 🔵 **suggestion / medium** — `config set` is net-new capability with its own
  substantial sub-scope inside a story whose stated purpose is a risky
  like-for-like cutover (*Location*: Requirements).

### Testability

**Summary**: Several Acceptance Criteria are genuinely strong — AC5, AC6, and AC7
each admit a definitive pass/fail procedure. The critical weakness is AC1, which
anchors the whole parity claim on "every documented behaviour is covered" without
any enumerated inventory, making the story's dominant risk the least verifiable
criterion in the item. Several stated requirements — the byte-exact output
contract, hook migration, the `configure`-first round trip, and exit-code
taxonomy — have no corresponding criterion at all, and AC3's "no added permission
prompts" has no defined verification procedure despite 0106 having already solved
exactly that framing problem.

**Strengths**: AC5 names two file targets and four independently checkable
outcomes; AC6 pins verification to named artefacts and a named constant; AC7
defines completion against an existing gate with a precisely scoped carve-out;
AC2 names the exact output token, exit code, and a second-order property;
Requirements prescribe a concrete, workable harness design.

**Findings**:

- 🔴 **critical / high** — AC1's parity criterion is unbounded and has no
  enumerable pass condition (*Location*: Acceptance Criteria AC1). The story's
  largest risk — irreversible loss of behaviour encoded in 337 assertions across
  6,289 lines deleted in the same change — rests on a criterion that can neither
  be definitively passed nor failed. Suggested fix: make the behaviour inventory
  an explicit deliverable committed before deletion, and the AC's denominator.

- 🟡 **major / high** — "No added permission prompts" has no defined verification
  procedure (*Location*: Acceptance Criteria AC3). 0106 explicitly reframed the
  runtime "no prompt" criterion into inspectable conditions; follow that
  precedent.

- 🟡 **major / high** — AC2's blanket exit-0 expectation contradicts the required
  fail-closed exceptions (*Location*: Acceptance Criteria AC2). A test engineer
  writing to AC2 would enforce exactly the wrong behaviour for doc-type path
  safety.

- 🟡 **major / high** — The output contract (byte-exact blocks, bare value +
  single newline, help text) has no criterion (*Location*: Requirements /
  Acceptance Criteria). The most consequential regression surface for
  `!`-preprocessor splicing has no pass/fail check.

- 🟡 **major / high** — Hook migration and the `configure`-first round trip have
  no acceptance criteria (*Location*: Requirements / Acceptance Criteria).

- 🟡 **major / medium** — AC5 can pass while `config set` destroys the surrounding
  Markdown body (*Location*: Acceptance Criteria AC5 / Open Questions).

- 🔵 **minor / high** — "Decremented from 21" gives no target value (*Location*:
  Acceptance Criteria AC6).

- 🔵 **minor / high** — AC4's 0107 clause is self-satisfying (*Location*:
  Acceptance Criteria AC4). Any text written to 0107 satisfies it, and "updated"
  is verifiable only as "the file changed".

- 🔵 **minor / medium** — "The write was atomic (temp file then rename)" specifies
  mechanism, not observable outcome, and "malformed" is undefined (*Location*:
  Acceptance Criteria AC5).

- 🔵 **suggestion / medium** — AC3 lacks a canonical enumeration command as its
  denominator (*Location*: Acceptance Criteria AC3 / Context). 0106 pinned its
  denominator with an explicit `grep`.

- 🔵 **suggestion / medium** — The built-in-vs-external decision rests on a
  latency band (20-30ms) with no criterion (*Location*: Assumptions /
  Acceptance Criteria).

## Re-Review (Pass 2) — 2026-07-18

**Verdict:** REVISE

All five lenses re-run against the revised work item. The pass-1 findings are
substantially resolved — but the revision itself introduced a new critical
finding and one outright contradiction between two acceptance criteria, so the
verdict does not clear.

### Previously Identified Issues

**Resolved**

- 🔴 **Testability**: AC1 parity criterion unbounded — Resolved. The committed
  behaviour inventory is now a requirement, an assumption, and AC1's denominator,
  with "no suite is deleted before its rows are mapped" as an ordering invariant.
  (But see the new critical finding on its *scope*.)
- 🟡 **Clarity + Scope**: 0107 disposition contradictory — Resolved. The Summary
  claim is gone, Dependencies states 0107 is not on the critical path, and the
  Drafting Notes record the migrate-then-build decision with rationale.
- 🟡 **Clarity**: Exit-code taxonomy settled and open simultaneously — Resolved
  as a contradiction. The Open Question is deleted and the Requirement states the
  divergence from luminosity deliberately. (New finding: it still has no
  criterion.)
- 🟡 **Completeness + Scope + Testability**: Hook migration and `configure`
  round trip lack criteria — Resolved. Both now have criteria. (New findings on
  the *mechanism* each depends on.)
- 🟡 **Testability**: Output contract has no criterion — Resolved. AC4 pins bare
  value plus single `\n`, empty stderr, byte-exact goldens, and help snapshots.
- 🟡 **Testability**: AC2 blanket exit-0 contradicts fail-closed exceptions —
  Resolved. Split into separate fail-open and fail-closed criteria, each with
  named triggering fixtures.
- 🟡 **Testability**: AC5 can pass while `config set` destroys the body —
  Resolved. Body prose must now be byte-identical; three named malformed
  fixtures added including the symlink escape.
- 🟡 **Completeness**: browser-executor glob re-homing unassigned — Resolved as
  a requirement. (New finding: its *timing* anchor is undefined, and the
  exclusion set it belongs to is only half documented.)
- 🔵 **Clarity**: "(resolved Q3)" dangling — Resolved; now points at the source
  research.
- 🔵 **Clarity**: "reader entrypoints" set unenumerated — Partially resolved.
  AC13 now lists it, but two members remain categories rather than names.
- 🔵 **Dependency**: 0166 too coarse as a blocker — Partially resolved. The
  prose names 0178/0179; the frontmatter still does not.
- 🔵 **Dependency**: 0170–0172 not listed as contract consumers — Resolved via
  the "0169–0174" sweep clause. (New finding: it skips 0168.)
- 🔵 **Dependency**: Claude Code matcher not recorded as external dependency —
  Resolved, with a version-recording obligation. (New finding: that obligation
  has no home.)
- 🔵 **Clarity**: 46-vs-35 gap unexplained — Resolved. Surfaced in Context and
  promoted to blocking Q2.
- 🔵 **Testability**: AC3 lacks a grep denominator — Resolved. AC5 now pins the
  exact command; three lenses independently called it the item's strongest
  criterion.

**Still present**

- 🔵 **Clarity + Completeness + Testability**: `_EXPECTED_CONFIG_SUITES` target
  value — Still present, and arguably worse. The revision replaced "decremented
  from 21" with "21 to 19, plus one if `test-init.sh` falls inside this counter's
  scope — confirm at implementation", which defers the criterion's own pass
  condition to the implementer. All three lenses flagged it again.
- 🔵 **Scope**: `test-init.sh` wiring is an orthogonal chore — Still present, and
  now actively contradictory (see new findings).
- 🔵 **Testability**: latency band has no criterion — Still present; not
  addressed in the revision.
- 🔵 **Clarity**: Summary omits scope elements — Still present and now broader.
  The Summary names the command and the cutover; the Requirements also carry
  `config set`, hook migration, and the suite replacement.

### New Issues Introduced

- 🔴 **Testability**: The parity denominator's scope is narrower than the
  deletion set. The behaviour inventory covers two suites
  (`test-config.sh`, `test-config-read-doc-type-paths.sh`) while AC13 removes the
  whole `config-read-*` family, `config-dump.sh`, `config-summary.sh`, the
  per-skill readers, the template-management scripts, and `init.sh`. Behaviour
  belonging to template management, per-skill readers, or `init` may sit in one
  of the nineteen surviving suites, in the inventory, or nowhere — the item does
  not say. This reproduces the pass-1 critical finding one level down: parity can
  be declared while irrecoverable behaviour is lost.

- 🟡 **Testability + Completeness + Scope**: AC12 and AC13 directly contradict
  each other. AC12 requires `test-init.sh` "discovered and passing"; AC13 moves
  `init.sh` to `accelerator config init`, i.e. deletes the script that suite
  exercises. Both cannot hold. This was introduced by the revision — the pass-1
  review flagged the tension and the revision enumerated the removal set without
  reconciling it.

- 🟡 **Dependency**: 0180 (atomic-store primitives) is unnamed despite owning the
  exact mechanism `config set` requires. 0180 delivers `atomic_write` —
  same-directory temp file plus atomic rename with interruption cleanup — which
  is verbatim what the Technical Notes and the `config set` criterion demand.
  Either it is built twice or this story stalls on an unrecorded gate.

- 🟡 **Clarity**: The inserted "to the config cluster" qualifier garbles both
  sentences it was added to. In "migrate every skill from bare script-path
  invocations **to the config cluster** onto `accelerator …` calls" it parses as
  a third destination rather than a restriction on which call sites are in scope
  — damaging the exact sentence that bounds the blast radius.

- 🟡 **Clarity**: "0169's wrapper model" survives two requirements after the
  revision declares the term retired in favour of "bootstrap path", undercutting
  the disambiguation it just added.

- 🟡 **Testability**: AC6's `allowed-tools` coverage check names no procedure.
  With 0107 deferred and Q1 open, the highest-risk property — that 247 rewritten
  call sites do not start prompting — has neither automation nor a stated manual
  procedure. The grep in AC5 proves only the *absence* of old paths.

- 🟡 **Testability**: AC10's concurrency clause has no verification procedure. A
  single interleaved read that happens not to tear proves nothing.

- 🟡 **Testability**: AC9's absence check has no exact target string, and taken
  literally would delete 0106 guidance that must survive — 0106 also governs
  `artifact-*` call sites, which this story leaves on bare paths until 0173.

- 🟡 **Testability**: The settled exit-code taxonomy has no criterion. AC3 only
  requires "non-zero", which exit 1 satisfies — so the decision the revision
  deliberately settled could be implemented as uniform exit 1 and still pass.

- 🟡 **Testability**: No criterion asserts the enumerated subcommand surface
  exists; a subcommand could be omitted or stubbed and still pass.

- 🟡 **Testability**: The behaviour inventory is a self-defined denominator with
  no granularity floor — "one row per assertion group" is unbounded, so a 20-row
  and a 300-row inventory over 337 assertions satisfy AC1 identically.

- 🟡 **Completeness + Scope**: The hook migration omits the hook output-envelope
  contract. The source research resolved this via a `--format=hook` switch, which
  this item's surviving `--format` open question dismisses without mentioning the
  hook consumer.

- 🔵 **Dependency**: The 0165 bullet is in the wrong tense. Both 0164 and 0165
  are `status: done`; the prose frames the ship-gate as an open precondition.

- 🔵 **Dependency + Completeness**: 0178/0179 are named in the Dependencies prose
  as what "actually gate this work" but absent from `blocked_by`, reproducing the
  prose/frontmatter split the pass-1 review raised.

- 🔵 **Completeness + Dependency**: The "`config-common.sh` consumers" grep
  exclusion is undocumented — `config-common.sh` is *sourced* by 44 scripts,
  which would not appear in a SKILL.md grep at all.

- 🔵 **Clarity**: The Drafting Notes cite "AC2" and "AC3", which no longer
  resolve — the criteria are an unnumbered checkbox list and the labels refer to
  pre-revision numbering.

- 🔵 **Clarity**: "in the same change" for the browser-executor re-homing has no
  referent, since the migration is explicitly permitted to span commits and PRs.

- 🔵 **Testability**: AC7 requires a byte-for-byte match against pre-migration
  bash output, but does not require that output be captured as a golden before
  the producing script is deleted in the same story.

- 🔵 **Testability**: AC5's zero-result grep has no known-positive precondition —
  0106 made exactly this precondition explicit.

- 🔵 **Testability**: AC2's notice string is left as `## … Unavailable` with an
  ellipsis placeholder, and omits the stderr assertion its requirement states.

- 🔵 **Scope**: The release-atomicity rationale for keeping one story is not
  enforced by the story's own boundary — 0165 is explicitly not a blocker, so the
  story can complete in a state its own rationale calls unacceptable.

### Assessment

The revision resolved the pass-1 critical finding and roughly two-thirds of the
majors, and three lenses independently praised the grep denominator and the
fail-open/fail-closed split as genuinely strong. But it is not ready for
implementation. Two defects are blocking: AC12 and AC13 cannot both be satisfied,
and the behaviour inventory — the mechanism introduced to prevent irrecoverable
loss — is scoped to two suites while the story deletes a much wider script
family, so the critical risk was narrowed rather than closed.

A third pass should be tightly targeted rather than another full sweep. The
substantive work is: settle `test-init.sh`/`init.sh` disposition; widen the
inventory to cover every script in AC13's removal set and give it a granularity
floor tied to the 337 assertions; add criteria for the exit-code taxonomy and the
subcommand surface; name a procedure for AC6's permission-coverage check; and
resolve the 0180 `atomic_write` overlap. The remaining items are editorial —
several of them repairs to text this pass introduced.

## Re-Review (Pass 3) — 2026-07-19

**Verdict:** REVISE

All five lenses re-run. Both pass-2 blockers are resolved, and the item's
verification design is now genuinely strong in places. But the pass-3 edits
introduced a new critical of their own — flagged independently by four of the
five lenses — and the scope lens raises a structural objection that has now
survived three passes without being answered.

### Previously Identified Issues

**Resolved**

- 🔴 **Testability**: Inventory scope narrower than the deletion set — Resolved.
  The inventory now covers every script in the removal set, requires scripts with
  no covering suite to be inventoried by reading them, and carries a granularity
  floor (337 assertions, one row each). The surviving-suite audit was added.
- 🟡 **AC12/AC13 contradiction on `test-init.sh`** — Resolved via
  characterise-then-retire, with the rationale (the suite has never run, so we
  don't know it passes) recorded.
- 🟡 **Dependency**: 0180 `atomic_write` overlap — Resolved as a decision
  (relocate the primitive to a shared lower-level crate). But see the new
  critical: the decision is recorded without naming the crate or gating the
  agreement.
- 🟡 **Clarity**: "to the config cluster" garble — Resolved; both sentences
  rewritten around the defined term "config-cluster call site".
- 🟡 **Clarity**: "0169's wrapper model" contradiction — Resolved.
- 🟡 **Testability**: Exit-code taxonomy had no criterion — Resolved, and the
  criterion names its own falsifier ("a uniform-exit-1 implementation fails
  this criterion"), which the testability lens singled out as a strength.
- 🟡 **Testability**: Subcommand surface had no criterion — Resolved via a
  parametrised input table. (New finding: it collides with the exit-2 criterion.)
- 🟡 **Testability**: AC6 permission coverage had no procedure — Resolved; a
  committed coverage script is now required.
- 🔵 **Testability**: Latency band had no criterion — Resolved as a criterion.
  (New findings on its internal definition.)
- 🔵 **Dependency**: 0164/0165 wrong tense — Resolved; both marked done and the
  0165 gate reduced to a verification step.
- 🔵 **Dependency**: 0178/0179 prose-only — Resolved; now in `blocked_by`.
- 🔵 **Dependency**: `config-common.sh` retirement had two owners — Resolved;
  0174 alone.
- 🔵 **Completeness**: `config-common.sh` as a grep exclusion — Resolved and
  explained; it is sourced, never invoked from a SKILL.md, so it is not an
  exclusion.
- 🔵 **Clarity**: Drafting Notes citing non-existent AC numbers — Resolved.
- 🔵 **Testability**: AC5 known-positive precondition — Resolved.
- 🔵 **Testability**: AC7 golden not captured before deletion — Resolved.
- 🔵 **Testability**: AC9 would have deleted surviving `artifact-*` guidance —
  Resolved; the directive is now explicitly retained for non-config families.
- 🔵 **Testability**: AC2 ellipsis placeholder — Resolved; all three notice
  strings named, stderr assertion added.
- 🔵 **Scope**: browser-executor had no migration owner — Resolved; 0173.
- 🔵 **Scope**: 0168 unexplained omission — Resolved as a deliberate exclusion.

**Still present**

- 🟡 **Scope**: The single-story rationale covers the command↔cutover coupling
  but not the test-migration programme, which the revisions have made the
  dominant thread. Raised in pass 1, partially answered in pass 2 (the
  coexistence-seam argument), and now sharpened: the forcing function for the
  inventory work is *script deletion*, not contract integrity — and script
  deletion is demonstrably schedulable separately, since `config-common.sh`'s
  retirement is already deferred to 0174 on exactly that reasoning. This
  objection has survived three passes without being answered on its merits.
- 🟡 **Clarity + Scope**: The Summary still under-describes the item, and now by
  more than before — it omits the suite audit, the `run_shell_suites` build-system
  change, hook-envelope ownership, the coverage script, the round-trip harness,
  the latency benchmark, and the mandated edits to 0106/0107/0180/0170–0173.
- 🔵 **Testability**: `_EXPECTED_CONFIG_SUITES` — the self-checking rewrite fixed
  the deferred-pass-condition problem but is tautological on its own; its
  verification value is entirely in the companion clause about the superseded
  suites, which should be the primary assertion.

### New Issues Introduced

- 🔴 **Clarity + Completeness + Dependency + Testability** (flagged by four
  lenses independently): **The hook-envelope requirement instructs the document
  to make a decision and never makes it** — "Decide which mechanism produces the
  Claude Code hook I/O envelope, state it here" — while the acceptance criterion
  asserts the hook "emits the hook envelope **defined in Requirements**". The
  referent does not exist. The criterion is unfalsifiable by construction: any
  envelope the implementer invents satisfies it. Worse, it is tracked nowhere —
  not in Open Questions, and Q3 explicitly disclaims it — so the gap is
  invisible. This reproduces exactly the failure the requirement was added to
  prevent, with 0169 inheriting an undocumented shape.

- 🔴 **Dependency**: **The `atomic_write` relocation posits an unnamed, unowned
  crate and gates it on an agreement nothing enforces.** The target crate is
  never named; no work item is named as its creator; and "must be agreed on 0180
  before either story starts" is unenforceable because 0180 is a `blocked_by`
  entry and therefore starts first. Three consequential couplings are absent: the
  `cargo-deny`/`cargo-pup` rules a new workspace crate requires (0162's), the
  retrofit of 0178's `config-adapters` onto a crate that did not exist when 0178
  was written, and symlink-escape ownership — left conditional ("if 0180 wants
  it") while an acceptance criterion here depends on it.

- 🟡 **Testability + Clarity**: The subcommand-surface criterion requires every
  subcommand including `templates eject|diff|reset` to exit 0, while the next
  criterion requires those same three to exit 2 against a not-customised fixture.
  Neither names its fixture state, so they return opposite verdicts on the same
  invocation — undercutting the ADR-0021 criterion pass 3 added to protect the
  taxonomy.

- 🟡 **Testability**: The atomicity criterion forbids racing ("asserted
  structurally rather than by racing") and then prescribes a racing observation
  (100 iterations observing no intermediate state). The "structural" mechanism is
  never defined, and a negative observation over 100 iterations cannot
  conclusively fail.

- 🟡 **Clarity + Completeness**: The grep denominator does not span the removal
  set. "Config-cluster call site" is defined as any SKILL.md invocation of a
  script in the removal set — which includes `init.sh` and the per-skill readers,
  neither matching `scripts/config-`. Removal-set scripts can be deleted with
  their call sites left on bare paths and the headline check still passes.

- 🟡 **Clarity + Completeness + Testability**: The "nineteen surviving suites"
  count contradicts the three superseded suites named elsewhere, and cannot be
  reconciled with `test-init.sh` being wired in mid-story (registered set becomes
  22, survivors 20, then 19). The audit's denominator is undetermined — and it
  also contradicts the deliberately count-free `_EXPECTED_CONFIG_SUITES`
  criterion.

- 🟡 **Testability**: The SessionStart golden is captured against a fixture but
  asserted against a live session, which draws from a real repository — the two
  cannot match, so the criterion is unsatisfiable as written and will be waived
  at verification time.

- 🟡 **Dependency**: The hook-envelope decision and the bootstrap-path naming
  alignment are both said to be inherited by 0169, but nothing requires either to
  be *recorded* on 0169 — unlike the 0107 disposition, which has a criterion.

- 🟡 **Testability**: "Each scalar read command" names an unenumerated set. `path`
  plainly qualifies; `paths`/`dump`/`agents` plainly do not; `get`/`agent`/
  `template` are undecidable from the text.

- 🟡 **Scope**: The story unilaterally expands the scope of 0180 — one of its own
  blocking dependencies — which is not a thing a downstream story can safely do.

- 🟡 **Testability**: The same-commit browser-executor re-homing constraint has no
  verification procedure; both the grep and the coverage script evaluate final
  tree state, where a late-added rule is indistinguishable from a same-commit one.

- 🔵 **Dependency**: The config→`document::render` coupling survives the reasoning
  that motivated the `atomic_write` relocation — `document` is a 0179 corpus-family
  crate, so the cross-domain edge the relocation was created to avoid is only
  half-avoided.

- 🔵 **Dependency**: `migrate-discoverability`, the third SessionStart hook, has
  no owner — this story rewrites `hooks.json` registrations and names only
  `config-detect` and the VCS hooks.

- 🔵 **Clarity**: Undefined referents introduced by pass 3 — "the named fixture
  workspace", "the reference machine", "these `blocks` edges" (whose nearest
  antecedent doesn't match the frontmatter), and the 0106 criterion's "identify
  which and say so", which defers its own target artefact.

- 🔵 **Completeness**: Q2 is labelled blocking but, unlike Q1, has no criterion
  requiring its answer be recorded. The Q1 Assumptions slot is an unfilled
  `{does / does not}` template that asserts nothing.

- 🔵 **Clarity**: "Config cluster" is now used in both a strictly defined sense
  (invocations of removal-set scripts) and a looser informal one (the bash family
  including `config-common.sh`), and two scope boundaries hinge on which applies.

- 🔵 **Scope**: The latency criterion's failure has no in-scope remedy — the
  built-in-vs-sub-binary decision is fixed by ADR-0054, so missing the band blocks
  completion for a reason the story cannot act on.

### Assessment

Not ready for implementation, but the trajectory is clear and the remaining work
is smaller than it looks: most of the twenty findings are editorial repairs to
pass-3 text, and the two criticals are the same *kind* of defect — a criterion
pointing at a decision the document promises to make and doesn't.

Two observations worth recording. First, **the hook envelope and the shared crate
should be settled before another editing pass**, not during one. Both are genuine
design decisions with downstream consumers (0169, 0180, 0178, 0162); writing
criteria around them while they are undecided is what produced both criticals.
Neither belongs to this work item's author acting alone.

Second, **each pass has resolved most findings while introducing new ones**
(pass 2 introduced a critical; pass 3 introduced two). The item is converging —
pass 3's findings are markedly less structural than pass 1's — but the pattern
suggests the remaining editorial work would be better done as a single careful
read-through by the author than as another review-and-patch cycle, with the
lenses re-run once afterwards to confirm rather than to drive.

The scope lens's objection also deserves a decision rather than a fourth
deferral: the inventory-and-port programme is now the dominant thread, its
forcing function is script deletion rather than contract integrity, and the
`config-common.sh`/0174 precedent shows deletion is schedulable separately. Either
answer it on the merits in Drafting Notes or act on it.

## Re-Review (Pass 4, scoped) — 2026-07-19

**Verdict:** REVISE

Scoped to **clarity and testability** — the two lenses that caught essentially all
of pass 3's regressions. Completeness, dependency, and scope were omitted: their
objections had been answered and the intervening changes were definitional and
criteria-level.

Between passes 3 and 4 the work item received two design settlements (the hook
envelope, the `atomic_write` crate), an answer to the scope objection, and a batch
of roughly thirty editorial repairs. Both pass-3 criticals are resolved. The
editorial batch introduced one new critical, found independently by both lenses.

### Previously Identified Issues

**Resolved**

- 🔴 **Hook envelope pointing at a definition that did not exist** — Resolved. The
  envelope is stated concretely with all three output states; scope narrowed to
  SessionStart, PreToolUse assigned to 0169; the dropped jq branch explained.
  Both lenses now cite it as a strength.
- 🔴 **`atomic_write` crate unnamed and unowned** — Resolved. `store` named,
  consolidation owned here, ordering-independence recorded, 0180 moved to
  `relates_to`.
- 🟡 **Exit-0/exit-2 collision** — Partially resolved. The fixture-state
  qualification landed, but `templates diff` now sits in two classes (new finding
  below).
- 🟡 **Atomicity criterion forbidding then prescribing a race** — Resolved.
  Structural assertion over an injected filesystem port; both lenses cite it.
- 🟡 **"Nineteen surviving suites"** — Resolved as enumeration. (New finding: the
  measurement point is unpinned.)
- 🟡 **SessionStart golden asserted against a live session** — Resolved; split into
  a fixture golden plus a smoke capture. (New finding: the smoke capture has no
  procedure.)
- 🟡 **Scalar-read set unenumerated** — Resolved via the output-class partition.
  (New findings on the partition itself.)
- 🔵 **Undefined referents** ("the named fixture workspace", "the reference
  machine", "identify which and say so", the `blocks` antecedent) — Resolved. Both
  lenses cite the named-fixtures table as a strength.
- 🔵 **Build-system jargon, Q1/Q2 recording criteria, RESERVED slot, `--format`
  moved out of Open Questions, "removal set" reserved as the precise term** — all
  Resolved and cited as strengths.
- 🟡 **Scope objection** — Answered on the merits (inventory is upstream of the
  command, not downstream of the cutover; phase order recorded). The answer
  surfaced the repointing question, which is now the live item.

### New Issues Introduced

- 🔴 **The widened grep contradicts the residual it is checked against** (both
  lenses, independently). The denominator criterion now specifies a pattern
  spanning "every path in the committed removal-set file list" — but
  `config-read-browser-executor.sh` is explicitly *not* on the removal set, so
  that pattern can never match it, while the next criterion requires the
  post-migration run to return "exactly the recorded
  `config-read-browser-executor.sh` occurrence count". No single grep satisfies
  both. The pre-migration arithmetic is wrong for the same reason: 247 was
  measured with `scripts/config-`, which *includes* the browser-executor hits the
  new pattern excludes. Technical Notes still calls browser-executor "the only
  exclusion", which only made sense under the old pattern. Both lenses propose the
  same fix — two named greps with separate expected values.

- 🟡 **The per-commit replay does not verify what it was added to verify**
  (testability). The coverage script extracts "each `accelerator` invocation";
  `config-read-browser-executor.sh` is a *bare-path* invocation, so the script
  never examines it — yet the replay criterion claims it "is what verifies the
  same-commit re-homing requirement". A commit that drops the glob and adds the
  narrow rule three commits later still passes. Fix: extend extraction to all
  `!`-preprocessor invocations, bare-path and `accelerator` alike.

- 🟡 **`templates diff` has two contracts** (both lenses). The output-class table
  puts it in **block**; the ADR-0021 criterion calls `eject|diff|reset` "the three
  template-mutation paths" and requires exit 2. The partition existed precisely so
  every subcommand has one contract. Testability's fix is cleaner than a note:
  rename the class to **customisation-state** (`eject|diff|reset`), distinct from
  **mutation** (`set|init`), and have the exit-code criterion reference it by name
  so the lists cannot drift.

- 🟡 **The latency bound is self-cancelling** (testability). One criterion sets
  p95 ≤ 30 ms; the next says a miss is "not blocked: record, ship, raise a
  follow-up". A criterion whose failure has no consequence is not a criterion. The
  rationale is also incoherent — it claims no reference machine is needed because
  the comparison is self-relative, then asserts an absolute millisecond figure.
  Fix: one binding self-relative criterion (≤ the bash p95 captured in the same
  run on the same host); demote 30 ms to a target in Assumptions; delete the
  waiver.

- 🟡 **The document asserts and denies that 0180's scope expands** (clarity).
  Dependencies: "0180 needs no amendment, no scope expansion". Drafting Notes:
  "expanding 0180's scope … must be recorded". This reopens the exact cross-item
  gate question the settlement closed.

- 🟡 **"Q1 and Q2 are the only outstanding items" is false** (clarity). The
  repointing question is explicitly flagged "worth revisiting before
  implementation starts" and governs whether the inventory is needed at all; the
  reciprocal-edges directive ("do one or the other") is also unresolved.

- 🟡 **The 44 `config-common.sh` consumers are described three incompatible ways**
  (both lenses). Assumptions says 44 *non-config* consumers; the removal-set
  criterion says 44 *today*, expected to drop; Technical Notes says 44 sourcers
  neutrally. 44 is the number justifying the library's survival, so which reading
  governs matters.

- 🟡 **247 is used as both the complete population and a partial count**
  (clarity). Context says *every* call site has the `scripts/config-` shape; the
  denominator criterion says `init.sh` and the per-skill readers have call sites
  outside that pattern. Both cannot hold — and if 247 is not the total, the
  uniformity claim Context calls "the main de-risking factor" is overstated.

- 🟡 **No criteria for two Summary-declared outputs** (testability): the 0166
  amendment, and the reciprocal dependency edges.

- 🟡 **The `test-init.sh` characterise step is unasserted** (testability). Nothing
  requires the wiring to have happened; an implementer who deletes the suite
  alongside `init.sh` satisfies every criterion, defeating the sequence's whole
  purpose.

- 🟡 **The audit table's measurement point is unpinned** (testability). The phase
  order runs the audit at phase 2 and deletion at phase 8, so at close the table
  contains `test-init.sh` and discovery does not — the required equality cannot
  hold. Fix: pin to a recorded revision, plus a final-state run with differences
  attributed.

- 🟡 **The surface table has an escape hatch that cannot fail** (testability): "if
  a subcommand's class is wrong at implementation time, fix the table" lets the
  spec be edited to match whatever was built. Nine block-class and two
  mutation-class commands also have no asserted output shape.

- 🟡 **The `configure` harness contradicts the scope boundary** (both lenses):
  "every extracted command resolves through `accelerator`" versus non-config
  families staying on bare paths until 0173.

- 🔵 Minors: "Q3" is now a dangling reference (the decision moved to Closed
  Decisions) and collides with the research's Q3; the Summary's "duplicate
  implementations" presupposes two when there may be one; the three injection
  commands are identified only by output heading, never by subcommand name;
  "fail-safe mode" still has no stated selection mechanism (default or flag) —
  carried unresolved since pass 1; "the baseline fixture in the state its success
  path requires" names two different fixtures; "notice **plus a trailing
  newline**" doubles the `\n` already in the quoted literals; the Open Questions
  footnote asserts the literal "21" the audit criterion bans; the 337 figure
  conflates static call-site count with runtime assertion count; "exactly one
  implementation" overstates what a "none outside `store`" check proves.

### Assessment

The two pass-3 criticals are properly closed, and both lenses now cite as
strengths several mechanisms that were findings two passes ago — named fixtures,
the known-positive grep discipline, the structural atomicity assertion, the
stated envelope. That is real convergence.

But the pattern has now held for three consecutive passes: **each round resolves
most findings and introduces a new critical of its own**, and each new critical is
an interaction between a repair and text elsewhere that the repair did not
account for. Pass 4's critical is a clean example — widening the grep pattern was
correct in isolation and broke the residual criterion added in the same batch.

This is the signature of patch-by-patch editing on a document that has grown to
32 acceptance criteria with dense cross-references. The remaining findings are
individually small, but they interact, and fixing them serially will very likely
produce a pass-5 critical by the same mechanism.

**Recommendation: stop reviewing and rewrite.** The Acceptance Criteria section in
particular should be rebuilt in one pass by an author holding the whole document
in mind — resolving the grep/residual arithmetic, the class partition, the
latency binding, and the missing criteria together rather than one at a time.
Then run the lenses once to confirm, not to drive.

Two decisions should be settled first, because they change what the criteria say:
**the repointing question** (which may remove the inventory programme entirely)
and **whether the reciprocal-edge convention is one-sided or bidirectional**.

## Re-Review (Pass 5, confirmation) — 2026-07-19

**Verdict:** COMMENT — acceptable for planning; remaining items are recorded
rather than blocking.

Both decisions flagged at the end of pass 4 were settled with evidence
(repointing is viable: `test-config.sh` binds each script path once and invokes
uniformly; reciprocal edges added on the three `blocks` items), the Acceptance
Criteria were rebuilt in one pass rather than patched, and clarity plus
testability were re-run to confirm.

### Outcome

The rebuild held structurally. Both lenses confirmed: output classes no longer
overlap, fixtures resolve one-to-one against a single definition list, the two
greps are mutually consistent, and criteria that were previously satisfiable by
construction are now anchored to artefacts that can fail — the surface table as
parametrised input, the known-positive grep floor, the per-commit coverage
replay, the recorded green run for `test-init.sh`. Both lenses listed these as
strengths rather than findings.

The confirmation pass found nine live defects, of which one was serious: **the
rewrite dropped the `configure` round-trip criterion while its own Drafting Note
claimed the criterion had been narrowed** — a note describing an edit that was
never made. That is a documentation-integrity failure as much as a coverage gap,
and it is worth recording as such: a reader reconciling notes against criteria
would have been actively misled.

All nine were repaired in the same session:

- 🔴 `configure` round-trip restored as its own **End-to-end proof** group, now
  also asserting the `config set` round trip through the skill — the only
  criterion tying read and write paths together outside unit fixtures. The false
  Drafting Note was corrected.
- 🟡 `test-init.sh` fell between both parity gates (not repointed; `init.sh` has a
  covering suite so it escaped "scripts with no covering suite"). Now the explicit
  fourth remainder member, with the two gates stated as exhaustive.
- 🟡 Depth floor restored for the inventory's unbounded members — the rewrite had
  dropped pass 2's 337-assertion floor as inapplicable without replacing it,
  leaving one hand-wavy row per script sufficient to make it deletable.
- 🟡 Live hook equivalence compared a JSON envelope against one of its string
  fields and could never pass; now parses the field first.
- 🟡 Fail-open's antecedent was a superset of the three fail-closed triggers, so
  the same input satisfied two criteria demanding opposite exit codes.
- 🟡 Grep A's corpus was undefined, making "exactly 0" unreachable tree-wide and
  gameable by a scope chosen afterwards; now the same literal corpus as Grep B.
- 🔵 Three fail-closed fixtures named and added to Technical Notes; preamble
  softened where it overclaimed.
- 🔵 Superseded-suite absence pinned to the final state.
- 🔵 Reciprocal-edge scope narrowed to 0169/0173/0174; stale Assumptions parity
  bullet updated to the repoint-first strategy.

### Assessment

**The rewrite-then-confirm approach worked where four rounds of patching had
not.** Pass 5's findings were fewer, more localised, and — with one exception —
consequences of the rebuild's scope rather than fresh contradictions between
distant sections. That is the signature of a document that has converged.

The item is now suitable for planning. Two things remain open by design rather
than omission: **Q1 and Q2** (empirical probes, both with resolution paths and
recording criteria, both blocking the rewrite phase specifically), and the
**`--format=hook` naming exception**, recorded with its reasoning and a stated
escape hatch.

One process note worth carrying forward: across five passes, every incremental
patching round introduced a new critical, and the single rewrite round did not
introduce one — it dropped a criterion, which is a different and more visible
failure mode. On a document with this density of cross-references, rebuilding a
section beats patching it, and the confirmation run is what catches what the
rebuild dropped.

### Verdict: APPROVED

**Approved by Toby Clemson, 2026-07-19.** The lens-derived verdict on this pass
was COMMENT — no blocking findings, with Q1, Q2, and the `--format=hook` naming
exception open by design. The reviewer accepted it as APPROVE, closing the review.
Recorded this way so the distinction between the mechanical verdict and the
reviewer's decision stays visible: the lenses did not return APPROVE, and nothing
in the pass-5 findings was overridden — the two differ only in whether the
recorded-but-open items warrant a COMMENT.

Work item status transitioned draft → ready separately, per the review workflow's
separation of verdict from lifecycle state.
