---
type: "work-item-review"
id: "0172-migration-engine-subdomain-review-1"
title: "Work Item Review: Migration Engine Subdomain"
date: "2026-07-29T16:18:55+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0172"
work_item_id: "0172"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: ["rust", "migration-engine", "concurrency", "interactive"]
last_updated: "2026-08-01T11:59:07+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Migration Engine Subdomain

**Verdict:** REVISE

0172 is a well-researched, densely populated story: every section is
substantively written, requirements are anchored to concrete file paths and line
counts, deliberate scope-outs (`migrate status`, the bash author API, the
SIGTERM→SIGKILL mechanism) carry their rationale, and the load-bearing
interpretation — that the seven migrations become Rust rather than staying as
bash children — is recorded in Assumptions so it can be challenged. The
inherited 0180 clean-cutover constraint is captured with rare precision,
including its escalation path.

What holds it back is verification and sizing rather than intent. One criterion
is structurally unimplementable (the non-repointable remainder is never given a
membership, so its "no gaps against a fresh extraction" check has no domain), the
interactive framework's headline parity gate uses as its byte-for-byte oracle a
document this same story rewrites, and the retirement criterion's directory check
misses roughly 900 lines of the retirement inventory the item itself enumerates —
including the very files that hold the FIFO/fd IPC, so a Rust engine that merely
*relocates* the IPC across a Rust↔bash boundary would pass. Four contracts that
completed siblings explicitly delegated here (0180's `outcome=edited` ⇔
`user_value` rule, 0119's per-run path manifest, 0167's invocation contract,
ADR-0037's declaration mechanism) are not recorded, and the story carries ~11.4k
retired lines as a single `story` on a bare "kept as one story by decision" while
its own Assumptions concede it "grows materially" if the interactive suites turn
out not to repoint.

### Cross-Cutting Themes

- **Self-referential and absent oracles** (flagged by: testability, clarity,
  completeness) — the transcript oracle is a document the story rewrites; the
  ledger parity baseline is never captured as a golden before the bash is
  deleted; the fixture the transcript criterion names is on the retirement list.
  Three of the story's strongest-looking parity gates can be satisfied by
  editing the thing they check against.
- **The retirement inventory and the retirement criteria do not agree** (flagged
  by: completeness, testability, dependency) — the criterion checks two
  directories for "shell", but Technical Notes lists root-level
  `scripts/interactive-harness.sh`, `scripts/interactive-protocol.sh`, three
  `.awk` helpers and `hooks/migrate-discoverability.sh`, and the
  `# INTERACTIVE: yes` header is a convention no directory check can see. The
  hooks-suite floor is also missing from the lockstep guard edits.
- **Acceptance thresholds stated as negatives or as undefined observables**
  (flagged by: testability, clarity) — "diagnosable message" (×3), "the
  configured timeout", "not silently reused", "the applied ledger matches". In
  each case two opposite implementations satisfy the criterion, so a verifier
  gets no definitive pass/fail.
- **Contracts delegated by completed siblings are not carried forward** (flagged
  by: dependency, testability) — 0180's `outcome=edited` ⇔ `user_value`
  enforcement, 0119's per-run path manifest and `ACCELERATOR_MIGRATE_FORCE`
  bypass, 0167's invocation contract and unowned `/migrate` call-site rewrite,
  ADR-0037's declaration mechanism. Each can be dropped in the port without any
  criterion failing.
- **The command shape is simultaneously open and presupposed** (flagged by:
  clarity, completeness, testability) — Requirements invite settling
  subcommand-vs-flag against 0164's conventions; every Command-surface criterion
  is written against flags; the decision is absent from Open Questions.
- **Summary understates the unit of work** (flagged by: clarity, scope) — the
  author-API retirement plus documentation rewrite, the hook port and the
  build-system guard edits are all in Requirements but absent from the Summary,
  which is the section anyone re-litigating the sizing decision will read first.

### Findings

#### Critical

- 🔴 **Testability + Clarity**: Non-repointable remainder has no defined
  membership, so its "no gaps" check is unimplementable
  **Location**: Acceptance Criteria: Parity and retirement (second)
  The criterion requires a committed check asserting "no duplicates and no gaps
  against a fresh extraction" but never says what the remainder comprises or
  what the extraction extracts from. The named exemplar 0167 enumerates its
  remainder as four named members, asserts the gates are exhaustive over every
  superseded assertion, and adds a depth floor (every top-level branch, every
  distinct exit code as its own row) precisely for members with no covering
  suite — none of which appears here, across ~11.4k retiring lines. Clarity adds
  that the check's *form* (script, test, task) and the CI-green commit's
  recording location are equally unbound.

#### Major

- 🟡 **Testability + Clarity**: The transcript oracle is a document this story
  rewrites, so the criterion can be edited to pass
  **Location**: Acceptance Criteria: Interactive framework (first)
  The criterion asserts the transcript "matches the one documented in
  `skills/config/migrate/SKILL.md` byte-for-byte", while Requirements bullet 2
  makes rewriting that file part of this story's work. The oracle and the
  artefact under test are both authored by the change being verified — 0167
  guarded its analogous criterion explicitly ("the specification cannot be
  edited to match whatever was built").

- 🟡 **Completeness + Testability**: Retirement criteria do not span the
  retirement inventory the Technical Notes enumerate
  **Location**: Acceptance Criteria: Parity and retirement (third)
  The criterion asserts only that `skills/config/migrate/scripts/` and
  `skills/config/migrate/migrations/` "contain no shell". Outside those
  directories sit `scripts/interactive-harness.sh` (688),
  `scripts/interactive-protocol.sh` (169) and `hooks/migrate-discoverability.sh`
  (73); the three `.awk` helpers fall under an undefined reading of "shell"; and
  the `# INTERACTIVE: yes` opt-in is a header convention no directory check sees.
  Because the FIFO/fd protocol lives in exactly those files, an engine that
  relocates the IPC across a Rust↔bash boundary — the failure the Assumptions
  call "the story's headline goal is not met" — passes every criterion.

- 🟡 **Completeness**: The replacement for the retired author-facing migration
  API is named but never described, and its documentation rewrite has no
  criterion
  **Location**: Requirements (second bullet)
  The story retires a **published** author API (`# INTERACTIVE: yes`,
  `harness_run`, `harness_reject`, `migration_validate_edit`) and promises to
  rewrite `skills/config/migrate/SKILL.md` "to describe its replacement" —
  but nothing states what the replacement is (how a Rust migration declares
  itself interactive, registers a transformation, supplies a validator), no
  criterion covers the rewrite, and it is not recorded in Open Questions as a
  deliberate deferral. Testability adds that nothing verifies the replacement
  authoring surface is *usable*.

- 🟡 **Clarity + Completeness + Testability**: Requirements defer the
  subcommand-vs-flag shape while the criteria presuppose flags
  **Location**: Requirements (first bullet) / Acceptance Criteria: Command
  surface
  "Settle subcommand-vs-flag shape against the dispatch conventions 0164
  established" leaves the shape open; the criterion then reads "Every **flag** in
  the Requirements surface has a test…", which has no subject if `--skip`
  becomes `migrate skip <id>`. The decision is also absent from Open Questions,
  where a planner would look for it.

- 🟡 **Clarity**: "The hook wrapper 0169 establishes" names an artefact 0169
  explicitly disclaims
  **Location**: Context / Requirements / Acceptance Criteria: Discoverability
  hook
  0169's Drafting Notes state the registration names the *bootstrap path*,
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, "not a 'universal wrapper'", and 0167
  says outright it "fixes its name to 'bootstrap path'". 0169 does not build the
  artefact (0164 ships it); it rewrites the `hooks.json` registration. An
  implementer may look for a non-existent deliverable and build a bash shim
  instead.

- 🟡 **Testability**: "Each decision persisted durably before any artefact
  mutation" has no criterion
  **Location**: Requirements (fourth bullet)
  The resume criterion observes only the outcome (first two decisions not
  re-prompted), which an implementation that batches decisions and writes the
  session log *after* mutating the tree also satisfies on a clean run. The
  invariant that makes resume safe is the one property a successful run cannot
  evidence — and the one an in-process concurrency rewrite is most likely to
  reorder.

- 🟡 **Testability + Clarity**: "Not silently reused" names no observable
  outcome
  **Location**: Acceptance Criteria: Resume and staleness
  Abort non-zero, discard-and-re-prompt-everything, and warn-then-reuse all
  satisfy the criterion as written, and they differ materially. The
  complementary positive case (same `change_id`/`HEAD` ⇒ the log *is* reused) is
  also absent.

- 🟡 **Testability + Clarity**: "The configured timeout" is never defined,
  bounded, or made settable for tests
  **Location**: Acceptance Criteria: Timeout
  The criterion declares "the bound … [is] the contract" without naming the
  bound, the default (bash had 30s), or the configuration mechanism. An
  undefined threshold cannot definitively fail, and without a test knob the only
  implementable test blocks for the production timeout. "Configured" also
  implies a user-facing config surface no requirement establishes.

- 🟡 **Testability**: "Written by exactly one implementation" is a
  deployment-history property, not a checkable one — and the bash-written-log
  case is uncovered
  **Location**: Acceptance Criteria: JSON (second)
  No procedure a verifier can run confirms that no file was ever written by both
  implementations, so the obligation 0180 propagated here — the sole
  justification for scoping bash↔Rust byte parity out — can be reported met
  without anything being checked. Nothing covers what the Rust engine does with
  a session log a bash writer produced before the cutover, though Dependencies
  acknowledges "records that outlive the cutover".

- 🟡 **Testability + Dependency**: Only the fail-closed branch of the dirty-tree
  ownership check is asserted; the 0119 guarded-resume contract is unrecorded
  **Location**: Acceptance Criteria: Agent invocation (second) / Requirements
  (fifth bullet)
  The only criterion covers a tree dirtied by a file the run does *not* own, so
  an implementation that refuses on *any* dirty tree — discarding ownership
  discrimination — passes while regressing what 0119 shipped. 0119 defined that
  contract concretely (a per-run **path manifest**, distinct from the session log
  and applied ledger; fail-closed on absent/empty/unreadable/stale; a
  resume-affordance message listing owned dirty paths; the retained
  `ACCELERATOR_MIGRATE_FORCE` bypass), and 0172 names none of it — 0119 appears
  only as a bare parenthetical in References.

- 🟡 **Testability + Clarity**: Ledger parity rests on an unnamed fixture, an
  uncaptured bash baseline, an undefined comparison basis, and no per-migration
  assertion
  **Location**: Acceptance Criteria: Command surface (first)
  "The applied ledger matches what `run-migrations.sh` produced on that same
  fixture" names no fixture (0167 defines every fixture it references), does not
  require the bash baseline be committed as a golden before the bash is deleted
  (so the criterion is unrunnable at the final state), and never says whether
  "matches" ranges over the set-and-order of entries or the bytes — the latter
  reading contradicts the 0180 carve-out this story relies on. No criterion
  asserts the fixture exercises all seven migrations, leaving 0007 (856 lines)
  with no stated parity procedure.

- 🟡 **Dependency**: The 0167 invocation-contract dependency is unrecorded, and
  the `/migrate` call-site + `allowed-tools` rewrite is unowned
  **Location**: Dependencies
  0167 owns the invocation contract (bare script path → `accelerator …` call
  sites, `allowed-tools` rewrites, the permission-coverage script, the
  bootstrap-path naming) and records the reciprocal edge ("0170, 0171, and 0172
  also consume the invocation contract established here"), while deliberately
  scoping the migrate cluster's call sites out of its own removal set; 0173
  claims only the corpus/design/collaboration call sites. So no item owns
  rewriting `skills/config/migrate/SKILL.md`'s invocations of the deleted scripts
  or the permission globs covering them — and a stale glob fails at skill-load
  time in production.

- 🟡 **Dependency**: 0180's delegated `outcome=edited` ⇔ `user_value`
  obligation is not recorded
  **Location**: Dependencies
  0180's AC-7 states `user_value` emission is presence-based and that "the
  `outcome=edited` coupling is enforced by the migrate consumer 0172, not this
  primitive". Nothing in 0172 mentions enforcing it, so a malformed record class
  can ship enforced by no one. 0180 also names the visualiser/0168 refactor as a
  *reader* of the canonical field order, which 0172 does not list among its
  downstream consumers.

- 🟡 **Dependency**: The hooks-suite floor is missing from the lockstep guard
  edits
  **Location**: Requirements (last bullet)
  The story retires `hooks/test-migrate-discoverability.sh` but its guard
  obligation names only `_EXPECTED_MIGRATE_SUITES` and the three
  `SHELL_LIBRARIES` entries. 0169 establishes exactly this lockstep for the
  hooks group and leaves `migrate-discoverability.sh` for 0172; 0174 requires
  that "CI never goes green→red on a floor mismatch".

- 🟡 **Dependency**: ADR-0037 — the accepted interactive contract whose
  declaration mechanism this story replaces — is not referenced
  **Location**: References
  ADR-0037 (from work item 0092) fixes the framework-level interactive
  primitives including *how a migration declares the hook*, and its §5 requires
  any new framework primitive to route back as a supplementary ADR. 0172 lists
  ADR-0023, 0038, 0047, 0052, 0053 but not 0037, cites 0092 only as a bare
  related item, and captures no obligation to supplement it. `docs/migrations.md`
  is likewise a reference-only entry although it is a second author-facing
  document describing the retired harness.

- 🟡 **Scope**: `kind: story` for ~11.4k retired lines, with the split axes
  named but the rejection unargued
  **Location**: Frontmatter: kind / Drafting Notes: Sizing
  One unit commits to: a six-flag command surface; seven ported migrations
  (2,632 lines, largest 856); replacing a 984-line FIFO/fd concurrency library
  and its watchdog; retiring a published author API (688) plus its docs; a hook
  port; repointing six suites (~7,276 lines) plus a checked-in inventory with a
  committed no-gaps check; and two build-guard edits. Drafting Notes concede the
  surface exceeds 0167's — the epic's "highest-blast-radius story" — name a
  viable three-way split, and reject it with only "Kept as one story by
  decision". Sibling 0166 was in fact decomposed into 0178/0179/0180 when it
  proved too large: a precedent inside this epic.

- 🟡 **Scope**: The sizing rests on an unverified repointability assumption with
  no bounded step to settle it
  **Location**: Assumptions (third)
  The item assumes the suites are "repointable in bulk, as 0167 found for the
  config suites" and immediately concedes the story "grows materially" if the
  interactive suites drive the FIFO protocol directly — yet no requirement or
  criterion classifies the suites before commitment, and the names
  (`test-migrate-interactive.sh`, 2,081; `test-interactive-protocol.sh`, 108)
  point the wrong way. 0167 handled the analogous unknown structurally
  (characterise-then-retire at a recorded green commit).

#### Minor

- 🔵 **Clarity**: "Single-pending-migration scoping" is an undefined term
  carrying behaviour
  **Location**: Requirements (fourth bullet)
  Used once, never explained: it could mean the decisions-file flow is only
  supported with exactly one pending migration, that `--list` emits one
  migration at a time, or that a run consumes decisions for one migration per
  invocation. The three readings imply different engine behaviour.

- 🔵 **Clarity**: "Applied ledger", "skip-list" and `<id>` are never bound to
  artefacts or units
  **Location**: Acceptance Criteria: Command surface
  Nothing says which on-disk artefacts the ledger and skip-list are, and `<id>`
  is ambiguous between a migration id (`0001`–`0007`) and a transformation id —
  sharpened by `--list` being specified as emitting *transformations*.
  `--unapply 0007` and `--unapply <transformation-key>` are materially different
  features.

- 🔵 **Clarity**: "`ACCELERATOR_MIGRATION_MODE`'s two halves" does not resolve
  against the three locations listed
  **Location**: Requirements (sixth bullet)
  "Two halves" is attached to the variable but three code locations follow, and
  Drafting Notes implies a different partition again (the variable **and** the
  `.claude/accelerator.md` fallback). The reader cannot tell what the
  already-ported path consists of — which is exactly what they are told not to
  re-port.

- 🔵 **Clarity + Scope**: The Summary's scope is narrower than the Requirements'
  surface
  **Location**: Summary
  "Port `skills/config/migrate/`" omits the root-level `scripts/` harness and
  protocol files, the `# INTERACTIVE: yes` opt-in and its documentation rewrite,
  `hooks/migrate-discoverability.sh`, and the `tasks/` guard edits. Anyone
  triaging or re-litigating the sizing decision from the Summary will
  under-count the work.

- 🔵 **Clarity**: The CI-green criterion leaves actor, artefact and recording
  location unnamed
  **Location**: Acceptance Criteria: Parity and retirement (first)
  "Observed green in CI at a recorded commit" — recorded where? The change
  description, a checked-in file, the work item? 0167 binds this; 0172 leaves it
  passive.

- 🔵 **Completeness + Testability**: The interactive fixture tree's disposition
  is contradictory
  **Location**: Technical Notes / Acceptance Criteria: Interactive framework
  One criterion names `scripts/test-fixtures/interactive/doc-example/` as its
  input; Technical Notes lists "the `scripts/test-fixtures/interactive/` fixture
  tree" among the surface to retire. Nothing says whether it survives, moves
  under `cli/`, or is regenerated — so the byte-for-byte criterion may have no
  input at the final state.

- 🔵 **Dependency**: No coupling recorded to the machinery that makes a new
  sub-binary shippable
  **Location**: Dependencies
  `accelerator-migrate` is a new sub-binary and at least one new crate, but
  nothing names the 0165 distribution pipeline (per-target cross-compile,
  per-binary checksum + minisign, the `manifest.json` entry driving launcher
  help and fetch-verify-cache) or the 0162 enforcement policy (`cargo-pup` rule,
  `cli/deny.toml` entry, workspace-member addition). 0167 treats this knock-on
  set as "none of it optional". A binary absent from the manifest cannot be
  fetched or verified at first use.

- 🔵 **Dependency**: No External entry for the Claude Code vendor behaviour two
  decisions rest on
  **Location**: Open Questions / Dependencies
  The deferred transport choice is constrained by the no-TTY / EOF-stdin
  semantics of agent invocation (the defect 0115 exists to fix), and the
  discoverability hook inherits 0169's unresolved probe into whether
  `hooks.json`'s `command` expands `${CLAUDE_PLUGIN_ROOT}` and splits argument
  tokens. 0167 records this class under an explicit "External:" bullet with the
  verified Claude Code version.

- 🔵 **Dependency**: Migration 0007's cross-cluster couplings are unnamed
  **Location**: Requirements
  Per 0115, 0007's `self_validate_structural` gate runs against
  `scripts/validate-corpus-frontmatter.sh` — a script **0173** owns, and 0173 is
  the later phase, an ordering inversion. 0167's audit also records that
  `test-migrate-0007.sh:2208` writes an `exec` stub hard-coding the config
  resolver path, so that suite breaks at 0167's deletion rather than at this
  cutover, undermining the "repointed suites are the oracle" strategy. The
  consumed-crate list also omits `document` despite the frontmatter-rewrite work.

- 🔵 **Scope**: The discoverability hook is an orthogonal bolt-on gated behind
  the epic's riskiest port
  **Location**: Requirements / Acceptance Criteria: Discoverability hook
  A 73-line SessionStart reminder with a 106-line suite touching none of the
  IPC, watchdog, session-log JSON, interactive contract or seven migrations. It
  is correctly *owned* here (0169 deferred it), but it could land independently
  as soon as 0169 does.

- 🔵 **Testability**: "Diagnosable message" is used as an acceptance threshold
  three times without definition
  **Location**: Acceptance Criteria: Agent invocation, Timeout
  No stream, exit code, or content is specified — any non-empty string can be
  argued diagnosable. The failure paths most likely to be hit by real users are
  gated on a subjective judgement. 0167 pinned exact stdout bytes and required
  diagnostics on stderr only.

- 🔵 **Testability**: The per-flag criterion under-specifies its observables and
  omits `--decisions-file` and error exits
  **Location**: Acceptance Criteria: Command surface (second)
  `--list`'s "one tab-delimited line each" never names the fields (0115 records
  key + proposed value + context, and emission order is the agent contract's
  mapping key); "`--help` exits 0" is satisfied by almost any binary;
  `--decisions-file` is in the Requirements surface but not the enumeration; and
  there is no exit-code taxonomy for unknown ids, a missing decisions file, or
  an unrecognised decision verb.

- 🔵 **Testability**: Named contract elements with no covering criterion
  **Location**: Requirements (fourth and sixth bullets)
  The `accept` verb of the accept/edit/skip loop is never exercised (the
  scripted transcript uses only `edit` and `skip`); single-pending-migration
  scoping has no criterion though the `#`-comment rule from the same bullet
  does; 0115's CRLF tolerance for decisions files is uncovered; and nothing
  asserts the legacy read path is *consumed* rather than re-implemented.

#### Suggestions

- 🔵 **Clarity**: The "~11.4k lines" total does not reconcile with the
  enumerated counts
  **Location**: Technical Notes
  The figures sum to roughly 12.5k (source ≈5,233; suites ≈7,276) before the awk
  helpers and the fixture tree, and no stated subset reaches 11.4k. The headline
  number is the reader's fastest size proxy and Drafting Notes leans on it.

- 🔵 **Clarity**: A few cross-references cannot be resolved from the item
  **Location**: Requirements / References
  0164 — whose dispatch conventions a requirement defers to — appears in neither
  Dependencies nor References; ADR-0047, 0052 and 0053 are listed bare while
  every neighbour carries a gloss.

- 🔵 **Completeness**: Frontmatter `status` and `priority` look understated
  **Location**: Frontmatter
  `status: draft` / `priority: medium` sit against content at a comparable
  refinement level to 0167 (`ready` / `high`), on the critical spine of 0136,
  blocked only by a `high`-priority item and blocking 0174 — with the Drafting
  Notes themselves noting the retired surface exceeds 0167's.

- 🔵 **Dependency**: The 0166 → 0172 edge is one-sided
  **Location**: Frontmatter: blocked_by
  0166 declares `blocks: [… 0172 …]`; 0172's `blocked_by` lists only 0169.
  Sibling 0169 keeps completed blockers in `blocked_by`, so this is a convention
  divergence rather than a scheduling risk — but a graph query from 0166 reports
  a dependant that does not acknowledge the edge.

- 🔵 **Scope**: The Sizing note omits the largest separable sub-unit as a split
  axis
  **Location**: Drafting Notes: Sizing
  Migration 0007 is 856 of the 2,632 ported lines and owns a dedicated awk
  helper and a dedicated 2,229-line suite — close to a story-sized unit in its
  own right, separate from the other six.

- 🔵 **Testability**: The resume criterion supplies no deterministic way to
  produce the interruption
  **Location**: Acceptance Criteria: Interactive framework (second)
  "Interrupted after the second of three decisions" does not say how; killing
  mid-run in an in-process model is racy. Sibling 0180 added a fault-injection
  seam for exactly this invariant.

### Strengths

- ✅ Every expected section is present and substantively populated — Open
  Questions, Dependencies, Assumptions and Drafting Notes all carry real content
  rather than boilerplate; frontmatter is complete and coherent.
- ✅ Requirements are anchored to concrete artefacts with line counts
  (`run-migrations.sh` 687, `interactive-lib.sh` 984, `cli/config/src/legacy.rs`,
  `_EXPECTED_MIGRATE_SUITES`, `SHELL_LIBRARIES`), so almost no requirement
  depends on the reader inferring which code is meant.
- ✅ Out-of-scope declarations are unusually crisp and reasoned: no `status`
  command (no bash antecedent), the legacy read path is "consume, do not
  re-port", and the SIGTERM→SIGKILL escalation is explicitly demoted to an
  implementation detail.
- ✅ The Timeout criterion is exemplary lens practice — it separates contract
  from mechanism, so it survives whatever transport planning chooses.
- ✅ The load-bearing interpretation (the seven migrations become Rust, else the
  IPC is relocated rather than removed) is surfaced in both Drafting Notes and
  Assumptions as a falsifiable premise with its consequence stated.
- ✅ The inherited 0180 constraint is captured in full — the clean-cutover
  premise, the no-interleaving obligation, canonical field order and escaper, and
  an explicit escalation path if the cutover cannot be guaranteed — and 0180
  reciprocates the edge.
- ✅ The Parity and retirement group mirrors 0167's proven pattern: suites
  repointed at the compiled binary and observed green in CI at a recorded commit
  *before* any covered script is deleted, plus an inventory keyed by
  `<file>:<line>` with per-row dispositions.
- ✅ Internal ordering constraints are explicit, including the guard edits
  landing "in the same change as the deletions" — the lockstep 0174 needs to
  avoid a green→red CI gap.
- ✅ Acceptance criteria are grouped into named themes and mostly written as
  Given/When/Then with named fixtures and pinned expected lines
  (`[interactive] empty value not allowed`); the JSON group enumerates its
  adversarial round-trip inputs and pairs a positive assertion with a negative
  structural one.
- ✅ Drafting Notes record every departure from the extracted source plus a
  sizing rationale and a pre-named split axis, so a later reader can see what was
  decided deliberately.
- ✅ Frontmatter edges (0169, 0174, 0136, 0180) are reciprocated from the other
  end, and the blocker is stated with its reason rather than as a bare id.

### Recommended Changes

1. **Decide the sizing question first — promote to epic or argue the
   non-split** (addresses: `kind: story` for ~11.4k retired lines; sizing rests
   on an unverified assumption; discoverability hook is an orthogonal bolt-on;
   0007 split axis omitted)
   Follow the 0166 → 0178/0179/0180 precedent, or keep one story and argue
   against each named axis explicitly (e.g. that splitting engine from
   interactive framework forces the bash driver and Rust engine to coexist over
   the very Rust↔bash bridge the Assumptions reject). Either way, add a first
   criterion that classifies each of the six suites as repointable-or-not
   against the compiled binary *before* any deletion, with a stated trigger for
   splitting if the interactive remainder exceeds a threshold.

2. **Give the non-repointable remainder a membership and a depth floor**
   (addresses: the critical finding; CI-green recording location unnamed)
   Enumerate the members as 0167 does — assertions driving the FIFO/fd protocol
   or the `# INTERACTIVE: yes` harness directly, the `test-migrate-0007.sh:2208`
   `exec`-stub region, the awk-helper assertions, and any retiring script with no
   covering suite — state what the fresh extraction scans, assert the repointed
   gate plus inventory are exhaustive over every superseded assertion, add
   0167's per-branch/per-exit-code floor, and name the check's form and where the
   CI-green commit is recorded.

3. **Break the self-referential oracles** (addresses: transcript oracle is a
   document this story rewrites; ledger parity has no captured baseline; fixture
   disposition contradictory)
   Capture the bash transcript and the bash ledger (plus post-run corpus state)
   for each named fixture as committed goldens at a recorded pre-deletion commit,
   assert the Rust output against those goldens, and make the SKILL.md transcript
   a rendering of the golden rather than the oracle. Name the fixtures in
   Technical Notes and state which survive retirement and where they live at the
   final state.

4. **Restate the retirement criterion as an explicit file-absence list**
   (addresses: retirement criteria do not span the inventory; hooks-suite floor
   missing)
   List the files that must be gone (`run-migrations.sh`, `interactive-lib.sh`,
   `migrations/0001`–`0007`, the three `.awk` helpers,
   `scripts/interactive-harness.sh`, `scripts/interactive-protocol.sh`,
   `hooks/migrate-discoverability.sh`), name the extensions the directory check
   enforces, add a residual grep returning exactly 0 for `mkfifo`,
   `# INTERACTIVE:` and the harness entrypoint names with its pre-migration run
   recorded as a known-positive floor, and add the hooks-suite floor decrement to
   the same-change guard set.

5. **Replace undefined observables with stated ones** (addresses: "diagnosable
   message" ×3; "the configured timeout"; "not silently reused"; ledger
   comparison basis; per-flag criterion; "written by exactly one
   implementation")
   For each failure path state the stream (stderr), exact exit code, empty
   stdout, and a committed snapshot or required substring. Name the timeout's
   default bound and its test mechanism, and restate the criterion as an elapsed
   bound plus tolerance. Give the staleness criterion a positive outcome plus its
   complementary same-revision case. Say whether ledger "matches" means
   set-and-order or bytes. Pin `--list` to a golden, add a `--help` snapshot,
   fold in `--decisions-file`, and add an error-condition → exit-code table.
   Convert the one-implementation criterion into (a) a static post-cutover check
   that no bash JSONL writer targets the session log and (b) a behavioural
   criterion for a pre-existing bash-written log.

6. **Record the four inherited contracts and add their criteria** (addresses:
   0180's `outcome=edited` ⇔ `user_value`; 0119's path manifest and FORCE
   bypass; 0167's invocation contract and the unowned `/migrate` call-site
   rewrite; ADR-0037 and `docs/migrations.md`)
   Extend the 0180 Dependencies note with the delegated validation obligation and
   name the visualiser/0168 reader; name 0119's per-run path manifest and
   `ACCELERATOR_MIGRATE_FORCE` as inherited, with a criterion for the
   fully-owned guarded-resume branch; add 0167 to Dependencies plus a requirement
   and criterion that the `/migrate` call sites and `allowed-tools` rules are
   rewritten in the change that deletes the scripts they name; and add ADR-0037
   to References with an obligation to record the replacement declaration
   mechanism as a supplementary ADR per its §5, alongside the
   `docs/migrations.md` rewrite.

7. **Specify or explicitly defer the replacement authoring surface** (addresses:
   replacement API named but never described; no criterion for the docs rewrite;
   durable-decision ordering has no criterion)
   Sketch the Rust equivalents of the opt-in header and the three harness calls
   (or record the gap in Open Questions as deferred to planning), and add a
   criterion that a new interactive migration authored solely from the rewritten
   SKILL.md runs green in a committed test. Separately, add a structural
   criterion in 0167's injected-port style asserting the session-log append
   completes before the first artefact mutation for each decision, with a
   fault-injection seam that aborts between the two — which also gives the resume
   criterion a deterministic interruption.

8. **Settle the vocabulary and the command shape** (addresses: subcommand-vs-flag
   deferred while criteria presuppose flags; "single-pending-migration scoping";
   "applied ledger"/"skip-list"/`<id>` unbound; "two halves" vs three locations;
   `accept`/CRLF/legacy-path coverage)
   Move the shape decision into Open Questions and either fix the flag surface
   (deleting the settle clause) or restate the criteria shape-neutrally. Define
   the ledger and skip-list artefacts once with the unit `<id>` names, spell out
   the single-pending-migration rule, and map each named half of the legacy read
   path to its file. Extend the scripted transcript to include `accept`, and add
   criteria for the scoping rule, a CRLF decisions file, and the absence of a
   second `ACCELERATOR_MIGRATION_MODE` implementation.

9. **Align the Summary, the naming, and the housekeeping** (addresses: Summary
   scope narrower than Requirements; "hook wrapper 0169 establishes"; ~11.4k
   total; 0164 and bare ADRs unreferenced; status/priority understated; 0166 edge
   one-sided)
   Extend the Summary to name the author-API retirement plus docs rewrite, the
   hook port and the guard edits. Replace "the hook wrapper 0169 establishes"
   with the settled "bootstrap path, invoked from the `hooks.json` registration
   0169 rewrites". Recompute the line total and say what it includes. Add 0164 to
   References with the relevant convention, gloss ADR-0047/0052/0053, reconcile
   `status`/`priority` with the item's refinement and spine position, and make
   the 0166 edge read the same from both ends.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Work item 0172 is unusually disciplined about meaning: nearly every
requirement names a concrete file path, deliberate scope-outs (`migrate status`,
the bash author API) carry their rationale, and the load-bearing interpretation
of "port the 7 migrations" is recorded in Assumptions so it can be challenged
rather than guessed. The residual clarity problems are three cross-section
contradictions — the item calls 0169's artefact a "hook wrapper" when 0169 and
0167 explicitly fixed its name as the "bootstrap path"; the Requirements invite
settling subcommand-vs-flag shape while the Acceptance Criteria presuppose flags;
and the interactive-transcript criterion uses as its byte-for-byte oracle a
document (`skills/config/migrate/SKILL.md`) that another requirement rewrites.
Beyond those, a cluster of undefined vocabulary ("applied ledger", "skip-list",
`<id>`, "single-pending-migration scoping", "the configured timeout") and a few
numeric/referent mismatches leave the reader guessing at points where the guess
changes the implementation.

**Strengths**:

- Requirements are anchored to concrete artefacts, so almost no requirement
  depends on the reader inferring which code is meant.
- The Timeout criterion explicitly separates contract from mechanism.
- The load-bearing interpretations are surfaced rather than buried.
- The 0180 clean-cutover obligation is written out in full with a named
  escalation path.
- Summary, Context and Requirements tell one consistent causal story.

**Findings**:

- 🟡 major, high — *"The hook wrapper 0169 establishes" names an artefact 0169
  explicitly disclaims* (Context / Requirements / AC: Discoverability hook).
  0169's Drafting Notes: the registration names the bootstrap path,
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, "not a 'universal wrapper'"; 0167
  "fixes its name to 'bootstrap path'". 0169 does not build the artefact (0164
  ships it, 0167 is its first consumer); it delivers the `hooks.json` rewrite.
  Risk: an implementer builds a bash shim — the residue 0169 and 0167 coordinated
  to avoid. Suggestion: use the settled term and state which of entrypoint or
  registration is depended on.
- 🟡 major, medium — *Requirements defer the subcommand-vs-flag shape while the
  criteria presuppose flags* (Requirements / AC: Command surface). "Every
  **flag** in the Requirements surface has a test…" has no subject if `--skip`
  becomes `migrate skip <id>`. Suggestion: fix the flag shape (deleting the
  settle clause) or restate the criterion shape-neutrally and record where the
  decision is made.
- 🟡 major, medium — *Transcript oracle is a document the same story rewrites*
  (AC: Interactive framework). Under one reading the criterion is
  self-satisfying. Suggestion: pin the oracle to a pre-migration golden and say
  whether the rewrite may change the transcript at all.
- 🔵 minor, medium — *"Single-pending-migration scoping" is an undefined term
  carrying behaviour* (Requirements). Three readings imply different engine
  behaviour and different agent-facing contracts.
- 🔵 minor, medium — *"Applied ledger", "skip-list" and `<id>` are never bound to
  concrete artefacts or units* (AC: Command surface). `--unapply 0007` versus
  `--unapply <transformation-key>` are materially different features.
- 🔵 minor, medium — *"The applied ledger matches what run-migrations.sh
  produced" leaves the comparison basis open* (AC: Command surface). A byte
  reading contradicts the 0180 carve-out the story relies on.
- 🔵 minor, medium — *"The configured timeout" implies a configuration surface
  nothing else establishes* (AC: Timeout). Context says 30s watchdog; nothing
  says whether the bound becomes a config key, a flag, or stays hard-coded.
- 🔵 minor, medium — *"`ACCELERATOR_MIGRATION_MODE`'s two halves" does not
  resolve against the three locations listed* (Requirements). Drafting Notes
  implies a different partition again; risk is duplicating or missing a half.
- 🔵 minor, medium — *Summary's scope is narrower than the Requirements' surface*
  (Summary). Omits the root `scripts/` harness, the hook and the build-system
  edits.
- 🔵 minor, medium — *The inventory and CI-green criteria leave actor, artefact
  and range unnamed* (AC: Parity and retirement). 0167 binds all of these and
  enumerates its four remainder members.
- 🔵 minor, medium — *"Not silently reused" states only what must not happen*
  (AC: Resume and staleness). Abort, discard-and-re-prompt, and warn-then-reuse
  all pass.
- 🔵 suggestion, medium — *The "~11.4k lines" total does not reconcile with the
  enumerated counts* (Technical Notes). Source ≈5,233 + suites ≈7,276 ≈ 12.5k,
  before awk helpers and fixtures.
- 🔵 suggestion, low — *A few cross-references cannot be resolved from the item*
  (Requirements / References). 0164 is absent from both Dependencies and
  References; ADR-0047/0052/0053 are unglossed; "the 0136 spine" and
  "dry-emits" are used once without explanation.

### Completeness

**Summary**: 0172 is a densely populated story: every expected section is present
and substantively written, the story frame names its beneficiary, and the
frontmatter is complete with a recognised `kind`, coherent dependency edges and
an `external_id`. Kind-appropriate content is strong — Context explains the
forces with concrete line counts, Assumptions record the load-bearing
interpretation so it can be challenged, and the Acceptance Criteria are grouped
by theme with Given/When/Then shape. The gaps are localised rather than
structural: the replacement for the retired author-facing migration API is named
but never described (and has no criterion), and the retirement criteria's
file-scope does not span the retirement inventory the Technical Notes enumerate.

**Strengths**:

- All expected sections present and substantively populated — no
  placeholder-only sections.
- Frontmatter complete and internally coherent.
- The Summary uses a full story frame (beneficiary, want, rationale).
- Context is unusually informative for a port story — file-by-file line counts
  and the specific mechanisms being replaced.
- Acceptance Criteria grouped into named themes, mostly Given/When/Then with
  named fixtures.
- Drafting Notes record every departure from the extracted source plus sizing
  rationale and a split axis.
- Dependencies carries the inherited 0180 byte-parity constraint in full,
  including its escalation instruction.

**Findings**:

- 🟡 major, high — *Replacement for the retired author-facing migration API is
  named but never described, and its documentation rewrite has no acceptance
  criterion* (Requirements). Context itself calls the harness "a **published
  author-facing API**" whose shape "determines whether the IPC can be removed or
  only relocated", yet nothing states how a Rust migration declares itself
  interactive, registers a transformation, or supplies a validator. Suggestion:
  add a Requirement sketching the replacement (or record it in Open Questions),
  plus a criterion that SKILL.md and `docs/migrations.md` document the new
  surface with no residual reference to the retired harness.
- 🟡 major, medium — *Retirement criteria do not span the retirement inventory
  the Technical Notes enumerate* (AC: Parity and retirement). Roughly 900 lines
  of the named surface — root-level `scripts/` harness and protocol, three awk
  helpers, `hooks/migrate-discoverability.sh` — could survive every criterion as
  written, and 0174 would inherit them unowned.
- 🔵 minor, medium — *A third unsettled decision (subcommand-vs-flag shape) is
  buried in Requirements instead of Open Questions* (Open Questions). A planner
  reading Open Questions will miss the decision every Command-surface criterion
  keys off.
- 🔵 minor, medium — *Disposition of the interactive fixture tree is left
  unstated* (Technical Notes). One criterion names it as input; the notes list it
  as retirement surface.
- 🔵 suggestion, medium — *Frontmatter status and priority look understated
  relative to the item's refinement and position* (Frontmatter: status). 0167 is
  `ready`/`high` at comparable refinement; 0172 sits on the 0136 spine with a
  larger retired surface.

### Dependency

**Summary**: 0172's dependency record is unusually strong on the two couplings it
chose to document — the 0169 blocker (with its stated reason) and the inherited
clean-cutover constraint from 0180 — and its frontmatter edges are reciprocated
by 0169, 0174 and 0180. The gaps are all couplings the body implies but
Dependencies never names: the 0167 invocation-contract/bootstrap-path dependency
and the unowned `/migrate` SKILL.md call-site + `allowed-tools` rewrite, a second
obligation 0180 explicitly delegated to this story, the hooks-suite floor that
must fall in lockstep when `hooks/test-migrate-discoverability.sh` retires, and
the accepted ADR-0037 interactive contract whose declaration mechanism this story
retires. There is also no External entry for the Claude Code vendor behaviour
(no-TTY/EOF stdin, hooks.json argument expansion) on which the deferred transport
decision and the discoverability hook both rest.

**Strengths**:

- The inherited 0180 constraint is captured with rare precision, and 0180
  reciprocates it.
- Frontmatter edges agree with the prose and are reciprocated from the other end.
- The blocker is stated with its reason rather than as a bare id.
- Satisfied upstream couplings are recorded rather than silently dropped.
- Internal ordering constraints are explicit, including the same-change guard
  lockstep 0174 requires.
- Assumptions carry the load-bearing dependency risks with consequences stated.

**Findings**:

- 🟡 major, high — *0167's invocation contract is not in Dependencies, and the
  `/migrate` call-site + `allowed-tools` rewrite is unowned* (Dependencies).
  0167 records the reciprocal edge and scopes the migrate cluster's call sites
  out of its own removal set; 0173 claims only corpus/design/collaboration sites.
  A stale glob or call site fails at skill-load time in production. Suggestion:
  add 0167 as the source of the contract (transitively ordered via 0169) plus a
  requirement and criterion for the rewrite landing with the deletions.
- 🟡 major, high — *0180's delegated `outcome=edited` ⇔ `user_value` obligation
  is unrecorded* (Dependencies). 0180's AC-7 explicitly pushes it onto 0172;
  0180 also names the visualiser/0168 refactor as a reader of the canonical field
  order, which 0172 does not list as a downstream consumer.
- 🟡 major, medium — *The hooks-suite floor is never mentioned* (Requirements).
  0169 establishes the lockstep for the hooks group and leaves
  `migrate-discoverability.sh` here; 0174 requires CI never go green→red on a
  floor mismatch.
- 🟡 major, medium — *ADR-0037 is not referenced and no obligation to supplement
  it is captured* (References). ADR-0037 fixes the framework-level interactive
  primitives including how a migration declares the hook, and its §5 requires new
  primitives to route back as a supplementary ADR. `docs/migrations.md` is also
  reference-only although it is a second author-facing document.
- 🔵 minor, medium — *No coupling recorded to the machinery that makes a new
  sub-binary shippable* (Dependencies). 0165's manifest/signing and 0162's
  pup/deny/workspace-member set — which 0167 calls "none of it optional". A
  binary absent from the manifest fails for every installed user at first use.
- 🔵 minor, medium — *No External entry for Claude Code vendor behaviour* (Open
  Questions). No-TTY/EOF-stdin semantics constrain the deferred transport
  choice; 0169's `hooks.json` expansion/argument-splitting probe is inherited.
  0167 records this class under an explicit "External:" bullet with a verified
  version.
- 🔵 minor, medium — *Migration 0007's cross-cluster couplings are unnamed*
  (Requirements). 0007's `self_validate_structural` gate depends on a
  0173-owned bash validator (an ordering inversion), and
  `test-migrate-0007.sh:2208`'s `exec` stub breaks at 0167's deletion; the
  consumed-crate list omits `document`.
- 🔵 minor, medium — *The 0119 guarded-resume contract is compressed to one
  clause* (Requirements). 0119 defined a per-run path manifest (explicitly not
  the session log or applied ledger), fail-closed treatment of
  absent/empty/unreadable/stale, a resume-affordance message listing owned dirty
  paths, and the retained `ACCELERATOR_MIGRATE_FORCE` bypass. None is named, and
  the criterion covers only the non-owned refusal case.
- 🔵 suggestion, high — *The 0166 → 0172 edge is readable from one end only*
  (Frontmatter: blocked_by). 0166 declares `blocks: 0172`; 0172 does not
  acknowledge it. A convention divergence, not a scheduling risk.

### Scope

**Summary**: 0172 is thematically coherent — every requirement serves the single
goal of making the meta-directory migration engine native Rust, and the
boundaries it draws (no `status` command, consume-not-port the legacy read path,
timeout-as-contract rather than watchdog-as-mechanism) are unusually crisp. The
concern is sizing rather than cohesion: the item retires ~11.4k lines of bash
including a 984-line concurrency library, seven migrations, a published
author-facing API, ~7.3k lines of test suite, a hook, and build-system guards,
and is declared a `story` on a bare "kept as one story by decision" with no
reasoning — while a sibling of comparable weight (0166) was decomposed into child
work items. Compounding that, the item's own Assumptions concede the story "grows
materially" if the interactive suites turn out not to repoint, with no bounded
step named to settle that before commitment.

**Strengths**:

- Exceptionally clear out-of-scope declarations, each with a stated reason.
- The load-bearing scope interpretation is recorded in Assumptions with its
  consequence spelled out.
- Sizing was deliberately considered rather than overlooked, with a three-way
  split axis pre-named.
- The 0172/0174 boundary is drawn cleanly — exactly the lockstep guard work 0174
  delegates, and no more.
- Constraints inherited from neighbours are recorded as constraints rather than
  absorbed as additional work.
- The interactive-transport question is a genuine design deferral inside a fixed
  scope, not an unbounded hole.

**Findings**:

- 🟡 major, medium — *An epic-sized effort carried as a single story*
  (Frontmatter: kind / Drafting Notes: Sizing). Drafting Notes concede the
  retired surface exceeds 0167's — the epic's "highest-blast-radius story" — name
  a viable three-way split, and reject it with only "Kept as one story by
  decision", recording no reason; 0167 by contrast argues its own non-split
  explicitly, and 0166 was decomposed into 0178/0179/0180 when it proved too
  large. There is no intermediate releasable state and 0174 is blocked behind all
  of it.
- 🟡 major, high — *The story's size is unknown by its own admission at the point
  of commitment* (Assumptions). No requirement or criterion classifies the six
  suites before commitment, and `test-migrate-interactive.sh` (2,081) plus
  `test-interactive-protocol.sh` are named for retirement — suites whose names
  suggest they exercise the protocol being deleted. 0167 handled the analogous
  unknown with an explicit characterise-then-retire step. Suggestion: add the
  classification step plus an explicit split trigger.
- 🔵 minor, medium — *The discoverability hook shares only a topic name with the
  rest of the story* (Requirements / AC: Discoverability hook). Correctly owned
  here, but a small independent deliverable gated behind the epic's riskiest
  port.
- 🔵 minor, medium — *Summary/Requirements scope mismatch* (Summary). The
  author-API retirement and documentation rewrite, the hook port and the guard
  edits are absent from the Summary; a documented author-contract break should be
  visible at the top.
- 🔵 suggestion, medium — *The Sizing note omits the axis the Technical Notes make
  most visible* (Drafting Notes: Sizing). Migration 0007 is 856 of 2,632 ported
  lines with a dedicated awk helper and a dedicated 2,229-line suite. Also
  suggests recording the rejection reason against each axis.

### Testability

**Summary**: The criteria are unusually strong on the epic's parity machinery —
repointed suites observed green in CI at a recorded commit before any deletion,
an inventory keyed by `<file>:<line>` with recorded dispositions and a committed
no-duplicates/no-gaps check, and a timeout criterion deliberately reframed from
the SIGTERM→SIGKILL mechanism to an observable bound plus resumability. Where
they weaken is on the properties unique to this port: the non-repointable
remainder is never given a membership, so its mechanical gap check has no
extraction domain and none of 0167's depth floor; the interactive framework's
headline verification is a byte-for-byte match against a document this same story
rewrites; and several load-bearing Requirements (decision durability before
mutation, single-pending-migration scoping, the ownership check's own-output
branch, the deletion of the top-level harness scripts) have no covering criterion
at all. Roughly a third of the criteria also lean on undefined observables —
"diagnosable message", "the configured timeout", "not silently reused" — each of
which two opposite implementations could both satisfy.

**Strengths**:

- The Timeout criterion is exemplary lens practice — contract over mechanism.
- The Parity and retirement group mirrors 0167's proven pattern (green in CI at a
  recorded commit before deletion; `<file>:<line>` inventory with dispositions).
- The interactive transcript criterion supplies concrete scripted inputs and pins
  an exact expected line.
- The JSON group enumerates its adversarial round-trip inputs and pairs a
  positive assertion with a negative structural one.
- The command-surface meta-criterion requires a test per flag asserting both
  effect and exit code.
- Deferring the transport shape while naming the binding contract keeps the
  criteria mechanism-independent.
- `status` is scoped out with a stated reason, so its absence is not ambiguous.

**Findings**:

- 🔴 critical, high — *The non-repointable remainder has no defined membership,
  so its "no gaps" check is unimplementable* (AC: Parity and retirement). 0167
  enumerates four named members, asserts exhaustiveness over every superseded
  assertion, and adds a per-branch/per-exit-code depth floor for members with no
  covering suite; none appears here across ~11.4k retiring lines. The inventory
  can be satisfied by a handful of hand-wavy rows and every covered script then
  becomes deletable.
- 🟡 major, high — *The transcript oracle is a document this story rewrites, so
  the criterion can be edited to pass* (AC: Interactive framework, first). 0167
  guarded its analogue explicitly, with goldens captured before the bash is
  deleted.
- 🟡 major, high — *"Each decision persisted durably before any artefact
  mutation" has no criterion* (Requirements 4th bullet). A batching
  implementation passes the resume criterion on a clean run. Suggestion: a
  recording-port criterion showing the session-log append completes before the
  first mutation, plus a fault-injection seam that aborts between the two.
- 🟡 major, high — *"Not silently reused" names no observable outcome* (AC:
  Resume and staleness). Refuse versus restart versus warn-and-reuse all pass;
  the positive same-revision case is also missing.
- 🟡 major, high — *"The configured timeout" is never defined, bounded, or made
  settable for tests* (AC: Timeout). An undefined threshold cannot definitively
  fail, and without a knob the only implementable test blocks for the production
  timeout.
- 🟡 major, high — *"Written by exactly one implementation" is a
  deployment-history property; the bash-written-log case is uncovered* (AC:
  JSON, second). Suggestion: split into a static post-cutover writer check with a
  known-positive floor, plus a behavioural criterion for a pre-existing
  bash-written log.
- 🟡 major, medium — *Only the fail-closed branch of the dirty-tree ownership
  check is asserted; a fail-always implementation passes* (AC: Agent invocation,
  second). Regresses what 0119 shipped without any criterion failing.
- 🟡 major, medium — *The retirement criterion's directory check misses the
  top-level harness scripts, the hook, and the opt-in header* (AC: Parity and
  retirement, third). Because the FIFO/fd protocol lives in those files, a
  relocated-IPC implementation passes every criterion. Suggestion: explicit
  file-absence list, defined extensions, and a residual grep with a
  pre-migration known-positive floor.
- 🟡 major, medium — *Ledger parity rests on one unnamed fixture, an uncaptured
  bash baseline, and no per-migration assertion* (AC: Command surface, first).
  Unrunnable once the bash is deleted; "same migrations in the same order" is
  satisfiable by a one-migration fixture, leaving 0007 (856 lines) with no stated
  parity procedure. Suggestion: named fixtures, committed goldens at a recorded
  pre-deletion commit, and a per-migration fixture matrix.
- 🔵 minor, high — *"Diagnosable message" is used as an acceptance threshold
  three times without definition* (AC: Agent invocation, Timeout). 0167 pinned
  exact stdout bytes and required diagnostics on stderr only.
- 🔵 minor, medium — *The per-flag criterion under-specifies its observables and
  omits `--decisions-file` and error exits* (AC: Command surface, second).
  `--list`'s fields are unnamed though emission order is the agent contract's
  mapping key; "`--help` exits 0" is near-vacuous.
- 🔵 minor, medium — *Named contract elements with no covering criterion:
  `accept`, single-pending-migration scoping, CRLF tolerance, legacy read path*
  (Requirements 4th and 6th bullets). Each is a stated requirement with no
  verification procedure.
- 🔵 minor, medium — *The criterion's named fixture is on the retirement list*
  (AC: Interactive framework, first vs Technical Notes). The only named input may
  not exist at the final state.
- 🔵 suggestion, medium — *The resume criterion supplies no deterministic way to
  produce the interruption* (AC: Interactive framework, second). Killing mid-run
  in an in-process model is racy; 0180 added a fault-injection seam for exactly
  this.
- 🔵 suggestion, medium — *Two deliverables with no criterion: the settled
  invocation shape and the rewritten authoring documentation* (Requirements 1st
  and 2nd bullets). Suggestion: record the settled shape before the surface is
  built, and prove the documentation by authoring a new interactive migration
  from it in a committed test.

## Re-Review (Pass 2) — 2026-07-30

**Verdict:** REVISE

All five lenses re-run against the revised work item. **37 findings: 0 critical,
14 major, 20 minor, 3 suggestions** — down from 48 with the critical cleared.
The verdict stays REVISE on the major-count rule alone.

The revision landed its structural work. The critical is gone: the
non-repointable remainder now has an enumerated membership, a depth floor, a
fixed extraction domain and a committed no-gaps check, and testability lists it
as a strength. The self-referential oracles are broken — bash-captured goldens at
a recorded pre-deletion commit, with SKILL.md's transcript checked against the
golden rather than serving as it. Durable-decision ordering, the staleness
positive case, the guarded-resume branch, the 0167 and 0180 delegations, the
hooks floor and the sizing argument all now exist. Scope now lists the axis
analysis as a strength; completeness now considers `status: draft` *appropriate*
given the classification gate, withdrawing its pass-1 suggestion.

What keeps it at REVISE is different in character from pass 1. Pass 1 found
things missing; pass 2 mostly finds the newly-added material internally
inconsistent — and **five of the fourteen majors are defects the revision itself
introduced**. Two are factual errors about sibling items, three are
contradictions between a new clause and an existing one. The remaining majors are
coverage that the revision tightened but did not finish: two of 0119's four
obligations, the 0115 no-input stall, `--list` golden provenance, bash-pinned
exit codes, and the split trigger's counting procedure.

### Previously Identified Issues

**Critical**

- 🔴 **Testability + Clarity**: Non-repointable remainder has no defined
  membership — **Resolved**. Now carries enumerated membership, 0167's
  per-branch/per-exit-code depth floor, a fixed extraction domain and a committed
  CI check. Testability lists it among its strengths.

**Major**

- 🟡 Transcript oracle is a document this story rewrites — **Resolved**. Both
  clarity and testability record the circularity as explicitly closed. One new
  minor: byte-for-byte may be unsatisfiable while the transport is still open.
- 🟡 Retirement criteria do not span the retirement inventory — **Resolved** via
  the explicit file-absence list. Two new minors: the residual greps do not fix
  their pattern and corpus in the criterion (0167 hardened exactly this), and
  `scripts/test-interactive-protocol.sh`'s suite floor is still unnamed.
- 🟡 Replacement author-facing API named but never described — **Resolved** as a
  decision (retired with no replacement, recorded in four places). One new minor:
  the documentation criterion's substantive half ("describe writing an in-crate
  Rust migration") has no verifying check, so only the grep can fail.
- 🟡 Subcommand-vs-flag deferred while criteria presuppose flags — **Resolved**;
  scope lists the fixed flag surface as a strength. But see the new `--list`
  scope contradiction below.
- 🟡 "The hook wrapper 0169 establishes" — **Partially resolved**. The *naming*
  is fixed to the bootstrap path, but the replacement text asserts 0169 rewrites
  this hook's registration, which is wrong. See new issues.
- 🟡 "Each decision persisted durably before any artefact mutation" has no
  criterion — **Resolved**. The recording-store-port criterion is now a
  testability strength.
- 🟡 "Not silently reused" names no observable outcome — **Resolved**. Exit code,
  stream and the complementary same-revision reuse case are all stated.
- 🟡 "The configured timeout" undefined — **Partially resolved**. The 30s default
  and test-injectability are stated, but no criterion asserts the default value
  and "plus tolerance" is left undefined.
- 🟡 "Written by exactly one implementation" unverifiable — **Partially
  resolved**. The static post-cutover writer check is a real gain, but the
  bash-log branch added alongside it contradicts the premise. See new issues.
- 🟡 Only the fail-closed branch of the ownership check is asserted — **Partially
  resolved**. The owned-dirty guarded-resume branch was added, but two of 0119's
  four obligations still have no criterion: the `ACCELERATOR_MIGRATE_FORCE`
  bypass, and the four fail-closed manifest states (absent/empty/unreadable/
  stale). The affordance message's content is also unpinned — the refusal
  criterion names the *offending* path, not the owned ones.
- 🟡 Ledger parity: unnamed fixture, uncaptured baseline, undefined basis —
  **Partially resolved**. Goldens, set-and-order basis and the per-migration
  matrix all landed. Three gaps remain: the fixtures are still called "named"
  without being named; `--list` output is absent from the bash-capture set, so
  its golden may be Rust-derived — the same self-authored-oracle defect, on the
  interface the `/migrate` agent flow parses; and error exit codes are "stated"
  rather than pinned to the bash antecedent (`--skip`/`--unskip` are also
  verified by on-disk artefact only, not by behavioural effect).
- 🟡 0167 invocation contract unrecorded, `/migrate` call sites unowned —
  **Resolved**; both the dependency and the owning requirement are now present.
  One new minor about 0167's asserted status.
- 🟡 0180's `outcome=edited` ⇔ `user_value` delegation unrecorded — **Resolved**
  as requirement and criterion. One new minor: the visualiser/0168 consumer claim
  is contradicted by 0168's own record.
- 🟡 Hooks-suite floor missing from the lockstep set — **Resolved**.
- 🟡 ADR-0037 not referenced — **Partially resolved**. Added to References with a
  new Open Question, but no owner, edge or follow-up item tracks the
  reconciliation, and ADR-0023/ADR-0038 are in the same position.
- 🟡 `kind: story` sizing unargued — **Resolved as an argument**; the four-axis
  analysis with mechanism-level rejections is now a scope strength. But the
  contingency it introduces contradicts it. See new issues.
- 🟡 Sizing rests on an unverified assumption — **Resolved** by the
  classification gate, but the gate's own threshold is unmeasurable as written.

**Minor and suggestions** — resolved: single-pending-migration scoping defined;
`<id>` disambiguated; the legacy path's halves separated; the CI-green recording
location bound; fixture disposition stated; the External Claude Code entry added;
"diagnosable message" replaced throughout with stream + exit code + pinned
substring; `accept` and CRLF criteria added; the line total reconciled (clarity
now lists the counts as internally consistent); 0164 and the ADR glosses
corrected; the 0166 edge made two-sided; axis (d) added; the fault-injection seam
added. Still present: the legacy read path's "no second implementation" has no
criterion; the discoverability hook remains an orthogonal bolt-on (not re-flagged
this pass). Withdrawn: `status: draft` is now assessed as appropriate.

### New Issues Introduced

Five majors and three minors are defects in the revision itself, not survivals.

- 🟡 **Dependency**: Hook registration ownership misattributed. The revision
  states "0169 delivers the registration, not the entrypoint", but 0169's
  criteria remove only `vcs-detect.sh`/`vcs-guard.sh` and say
  "`migrate-discoverability.sh` is left in place for 0172", and 0167 records that
  of the four `hooks.json` registrations "`migrate-discoverability` is 0172's".
  No upstream story rewrites this registration — **this story owns the
  `hooks.json` edit**, and as written a planner will scope only the Rust
  entrypoint, leaving the SessionStart hook pointing at a deleted script. Fixing
  the pass-1 naming error introduced a wrong ownership claim.
- 🟡 **Clarity**: `--list` scope now specified two ways. The Command-surface
  criterion says "every pending interactive transformation"; the new
  single-pending-migration definition and the Agent-invocation criterion say
  "the first pending migration's transformations only". The `--list` golden will
  encode one reading — and the "interactive" qualifier adds a third question
  (are non-interactive transformations emitted?).
- 🟡 **Scope**: The split contingency names the axis the same note argues is
  unworkable. The gate says split into "its own work item", Drafting Notes says
  "(b) is the axis to take" — and the preceding paragraph rejects (b) because it
  forces bash driver and Rust engine to coexist over the wire protocol the
  Assumptions rule out. If the gate trips, the story has no workable
  decomposition on record.
- 🟡 **Clarity + Testability**: The bash-log round-trip branch contradicts the
  never-two-writers premise. Accepting and continuing a bash-written log produces
  exactly the mixed-writer file the neighbouring static check exists to forbid.
  Testability adds that the disjunction ("round-trips *or* is discarded") admits
  either outcome and that "never partially consumed" names no observable.
- 🟡 **Completeness**: Technical Notes promises the ledger and skip-list paths
  and record shapes, then defers them ("Record their exact paths and record
  shapes here during planning"). The Requirements cross-reference now points at a
  placeholder, and no criterion pins the locations as unchanged — so the
  "in-flight repo is not stranded" guarantee has no verification.
- 🔵 **Clarity + Scope**: The Summary's "Three surfaces" is now a wrong count.
  Sub-binary registration/distribution and the `allowed-tools` rewrite are also
  outside `skills/config/migrate/`, making it at least five. Widening the Summary
  fixed the omission but introduced a false enumeration.
- 🔵 **Dependency**: 0167 is labelled "(complete)" but its record is
  `status: ready`, and `store` is listed among the crates the 0166/0178/0179/0180
  group landed — 0166's amendment says the `store` crate is 0167's carve-out. Two
  obligations rest on 0167 actually having landed, including the
  permission-coverage check one criterion invokes.
- 🔵 **Clarity**: "store" now names at least three things — the `cli/store`
  crate, `corpus-adapters::store`, `config-adapters::store` — and two Dependencies
  sentences read as contradictory about where the atomic store lives.

### Remaining Coverage Gaps (new, not revision defects)

- 🟡 **Testability**: The 0115 no-input stall is unverified. 0115 requires an
  *immediate* structured stall naming pending decision keys and the resume
  command; only the injected-timeout path is covered, so the port could ship the
  timeout in place of the stall. `ACCELERATOR_MIGRATE_DECISIONS_FILE` — which
  0115 promoted to a documented interface — is also neither enumerated nor
  exercised.
- 🟡 **Testability**: The split trigger's "more than a quarter of assertions" has
  no counting procedure or denominator, so the gate on the largest sizing risk
  can be argued either way at verification time.
- 🟡 **Testability**: Repointed suites are a one-shot gate then deleted, with
  only the remainder mapped to Rust tests — leaving the cluster with thinner
  end-state regression coverage than the bash it replaces, and no stated floor.
- 🟡 **Completeness**: The sub-binary registration requirement — the one the item
  itself says breaks "every installed user at first use" — has no criterion in
  any of the ten groups.
- 🟡 **Dependency**: 0173's deletion of `scripts/validate-corpus-frontmatter.sh`
  is unsequenced against this story's *bash-side* golden capture. The revision
  resolved the post-port direction (Rust 0007 self-validates via
  `corpus`/`document`) but bash 0007 must stay runnable long enough to capture
  its golden, and 0173 is `blocked_by: [0167]` only.
- 🔵 **Completeness**: The seven migrations are never characterised individually —
  only 0007. Planning cannot size the fixture matrix or the interactive share
  without reading 2,632 lines of bash first, which is what the gate exists to
  avoid.
- 🔵 Smaller: "transformation" is load-bearing but undefined; the decisions-file
  "line count" check conflicts with the ignore-blanks-and-comments rule; the
  escalation clauses name no actor; the session log lacks the same-location
  commitment the ledger and skip list have; the `Allow` variant has no stated
  parent type.

### Assessment

Not yet ready for planning, but the distance is much shorter than pass 1's. The
structural work is done and verified by the lenses that flagged it — the
verification strategy is now assessed as "unusually well-specified for a
high-risk port", and the sizing reasoning as a strength.

Two things stand between here and APPROVE. First, **fix the five defects the
revision introduced** — these are cheap (a wrong ownership claim, a scope
contradiction, a count, a status label, an unresolved disjunction) but two of them
are factual errors about sibling items that would mislead a planner, and the hook
ownership one would ship a broken SessionStart hook. Second, **finish the
coverage the revision started**: 0119's force bypass and four manifest states,
the 0115 immediate stall, `--list` in the bash capture set, bash-pinned exit
codes, a criterion for sub-binary registration, and a counting procedure for the
split threshold.

The two open judgement calls worth settling with the author rather than editing
around: what a split actually looks like if the gate trips (the current answer is
self-contradictory), and whether the repointed suites' assertions need mapping to
durable Rust tests or a one-shot green gate is genuinely accepted as the coverage
contract.

## Re-Review (Pass 3) — 2026-07-30

**Verdict:** REVISE

All five lenses re-run against the twice-revised work item. **42 findings: 3
critical, 18 major, 18 minor, 3 suggestions** — up from 37, and the critical count
has gone from 0 back to 3.

This pass is a regression, and the cause is not ambiguous: **all three criticals
were introduced by the pass-2 fix round and the edits that followed it**, and so
were most of the majors. The pattern from pass 2 has repeated at higher severity —
pass 2 found five majors of the reviser's own making, pass 3 finds three
criticals. The structural work established in passes 1–2 still holds and all five
lenses continue to rate the verification design strongly; what is failing is
factual accuracy about referenced documents and sibling work items, in prose
written to close earlier findings.

The specific failure mode: claims about other artefacts were written from lens
summaries rather than from the artefacts themselves. Three of the worst findings
cite documents listed in this item's own References that the reviser never opened
— `skills/config/migrate/SKILL.md`, work item 0119, and work item 0178.

### New Critical Findings

- 🔴 **Clarity**: **"Single-pending-migration scoping" contradicts the documented
  `--list` contract it claims to preserve.**
  **Location**: Requirements (single-pending-migration scoping) / AC: Agent
  invocation / Open Questions
  The item states `--list` "emits the *interactive* transformations of the first
  pending migration only" and pins that in a criterion.
  `skills/config/migrate/SKILL.md` — in this item's own References, and the
  contract the story promises to preserve flag-for-flag against bash-derived
  goldens — documents the opposite: `--list` dry-emits **every** pending
  interactive transformation, segmented with `# migration <id>` headers and
  `<position>` restarting at 1 per migration, with a stderr note when more than
  one is pending. Only the *decisions file* is per-migration ("a single
  multi-migration decisions file is not yet supported"). The item then poses as an
  Open Question something the documentation already answers, while the criterion
  silently pre-decides the "empty output" branch. Four descriptions of one
  contract, and the `--list` golden — the item's own most-important oracle,
  because the `/migrate` skill parses it — would be captured against a mis-stated
  one.

- 🔴 **Testability**: **The parity gate requires all six suites repointed green,
  contradicting the black-box-rewrite criterion beside it.**
  **Location**: AC: Parity and retirement (first) vs Suite classification
  The gate says each of the six suites "is repointed … and observed green in CI at
  a recorded commit … before any script it covers is deleted". The
  Suite-classification group and the Requirements exist precisely because two of
  those suites may prove non-repointable, in which case they are rewritten in Rust
  instead — so the gate is unsatisfiable on the branch the story expects. For a
  *mixed* suite, nothing authorises or records removing the non-repointable
  assertions needed to make it green. 0167, the named exemplar, scopes its parity
  gate to the repointable suites only and discharges the remainder by inventory.
  Introduced by the edit that added the black-box fallback without scoping the
  gate.

- 🔴 **Testability**: **The owned-dirty-path criterion demands both a proceed and
  a refusal for the same precondition, and inverts 0119's contract.**
  **Location**: AC: Agent invocation (0115)
  The first sentence has the run proceed under guarded resume and complete; the
  second requires "a refusal message lists every owned dirty path". Same
  precondition, opposite outcomes. Worse, it inverts what 0119 shipped: there the
  every-owned-path message is a **resume affordance emitted when the pre-flight
  proceeds (exit 0)**, and 0119's refusal criterion asserts that *no* affordance
  message is emitted. The Requirements repeat the inversion ("a refusal names
  every owned dirty path as a resume affordance"). Written in the pass-2 round from
  a lens summary of 0119 rather than from 0119.

### Major Findings — factual errors about referenced artefacts

- 🟡 **Dependency**: **The legacy read path is specified against a mechanism that
  is tested to be refused.** 0178's record states it "deliberately dropped the
  env-var bypass and is tested to ignore it" — `ACCELERATOR_MIGRATION_MODE` "stays
  **unhonoured**" with a retained negative test — and the superseding mechanism is
  a per-invocation `--allow-legacy-layout` flag added by **0167**, passed directly
  by migrations 0001–0006 and via the allowlisted `doc-type-table.sh` for 0007,
  confined by `check-call-site-migration.sh`. This item names none of those three
  artefacts and credits the capability to a completed story. The seven ported
  migrations could satisfy the legacy criterion while being unable to read legacy
  config at all.
- 🟡 **Dependency**: **0167 does not carry the reciprocal edge this item claims it
  does.** 0167's `blocks` is `[0169, 0173, 0174]`, and its record says 0170–0172
  "are deliberately excluded… recorded in prose only" to avoid the asymmetry its
  own criterion exists to remove. Describing that prose as "the reciprocal edge"
  is wrong, and the two records now disagree about whether 0167 blocks 0172.
- 🟡 **Dependency**: **0180's stricter record validation is uncaptured.** 0180
  ported validation "to the documented spec, not the bash source's looser
  behaviour": its AC-8 rejects records whose `proposed_value` is empty *or absent*.
  Bash-written logs can therefore hold records that are readable but that 0180's
  composer refuses — exactly the case the new session-log cutover requirement does
  not address, defeating its stated goal that an in-flight repo "is picked up by
  the Rust one rather than stranded".
- 🟡 **Dependency**: **`scripts/jsonl-common.sh` is unowned across the epic.** Per
  0180, the JSONL primitives have one production caller — `interactive-lib.sh`,
  which this story deletes. The library appears in no retirement list, guard list
  or Dependencies here, and no other 0136 story claims it. 0167 set the precedent
  of stating such a disposition explicitly for `config-common.sh`.
- 🟡 **Dependency**: **The 0173 ordering constraint is still only prose.** The item
  discharges it with "Record that constraint on 0173 as well", but 0173 remains
  `blocked_by: [0167]` only, mentions 0172 nowhere, and its criteria already
  require deleting `validate-corpus-frontmatter.sh` — so it is unblocked the moment
  0167 lands and can destroy the oracle for `test-migrate-0007.sh` and the 0007
  golden with no signal from its own record.

### Major Findings — internal drift and unverifiable criteria

- 🟡 **Scope**: The whole-cluster assertion-mapping obligation — which Drafting
  Notes itself calls "the largest single addition to this story" — appears only in
  the Acceptance Criteria and Drafting Notes. The Requirements still describe "the
  pattern 0167 set … with one addition", and 0167's pattern is
  remainder-only-inventory with the green run as the gate for the bulk. Summary,
  Requirements and Criteria now describe three different amounts of work.
- 🟡 **Scope**: The sizing note reaches "kept as one story" from two premises that
  both argue for decomposition (it exceeds 0167; the epic decomposed 0166) without
  reconciling either, and its two-option rejection of the primary axis omits the
  option the 0166 precedent supplies — children in sequence under 0172 with the
  bash driver intact and deletions only in the final child. The item's own
  same-locations requirement is what would make that staged cutover safe, and
  since 0007 is the only interactive migration a first child over 0001–0006 would
  never write a session log.
- 🟡 **Testability**: The golden-capture criterion states no comparison semantics
  or normalisation. Byte comparison is impossible by construction for the session
  log (0180's canonical record carries a `timestamp`; byte parity is scoped out),
  and plausibly for the manifest and corpus state too. Only the ledger criterion
  states granularity. The capture set also omits success-path exit codes, which a
  sibling criterion compares against.
- 🟡 **Testability**: The four fail-closed manifest cases are pinned only to
  "non-zero, empty stdout, mutates nothing" — which any unrelated crash satisfies,
  so the guard is not actually verified. What makes a manifest "stale" is also
  undefined here (0119 defines it as carrying a different run's identity).
- 🟡 **Testability**: The no-input stall criterion omits the interactive-pending
  precondition — with only 0007 interactive, a repo with only 0001–0006 pending
  should complete, not stall — and asserts empty stdout at run start, whereas the
  0115 behaviour raises the stall at the first `PROMPT`, after pending
  non-interactive migrations have run and mutated the tree.
- 🟡 **Testability**: The multi-pending `--list` criterion cannot be exercised as
  intended. Since 0007 is both the only interactive migration and the last, any
  multi-pending scenario has a non-interactive first pending migration, so the
  criterion either asserts empty stdout or asserts something with nothing to
  consume, depending on the unresolved Open Question.
- 🟡 **Testability**: "Single atomic replacement before appending" has no stated
  observation mechanism — end-state inspection cannot distinguish it from
  truncate-and-rewrite, or rewrite-then-append from the reverse — and "decisions
  survive unchanged" has no comparison basis, since the point of the rewrite is to
  change the encoding.
- 🟡 **Testability**: The transcript golden's deviation list is unbounded, so a
  substantially drifted transcript can satisfy a byte-for-byte criterion by
  enumerating each difference as justified. No harness is specified for driving
  the four scripted decisions on either side.
- 🟡 **Completeness**: The ADR-reconciliation follow-up is promised in Open
  Questions but appears in no Requirement or criterion — and the retired-symbol
  grep excludes `meta/`, so the ADRs' references to the retired harness survive
  the cutover unchallenged.
- 🟡 **Completeness**: The bash-golden list includes the session log, but no later
  criterion says what that golden asserts: the JSON group pins records against
  0180's Rust-canonical golden instead, and byte parity is scoped out. The
  session-log golden has no consumer and no comparison basis.

### Minor and Suggestions

Recurring themes: `store` still listed among the "landed" crates though the item
says elsewhere it arrives with the open blocker 0167 (and `document` omitted from
that list); the "four-step `--list` → decisions-file → run → verify" flow does not
match the documented `list → decide → write → resume`; "the shares"/"the
denominator" have no antecedent (vocabulary left behind when the threshold was
removed); "the same record shapes as `run-migrations.sh` used" conflicts with the
canonical-Rust rewrite; `ACCELERATOR_MIGRATE_FORCE` "set" versus the documented
`=1`; the six suites' own deletion and the retiring fixture subtree are absent
from the file-absence criterion, and four suites are never given paths; the
manifest's own write correctness has no criterion despite Dependencies claiming
one; the retired-symbol grep runs over `.` including `target`/`node_modules`; the
`session[-_]log` grep is a name-based proxy that can fail on a legitimate mention
or pass on an indirect write; the "in-process concurrency model" requirement has
no criterion, so a Rust reimplementation of the same IPC would pass; several
delicate criteria run against fixtures absent from the enumerated set; and the
two per-assertion tables (classification and inventory) duplicate bookkeeping over
the same corpus. 0164 is absent from Dependencies though the story consumes its
bootstrap path and fetch-verify-cache model.

### Assessment

Not approved, and not close in the way pass 2 suggested it was. The item's
architecture, verification strategy and boundary-setting are genuinely strong —
every lens says so, and the whole-cluster inventory, bash-derived goldens, injected
seams and known-positive grep floors are all sound. But it now carries three
criticals and a cluster of false claims about the very documents and sibling items
it cites, and every one of those was introduced while closing earlier findings.

The remedy is not another fix round of the same kind. Three consecutive rounds of
editing from lens summaries have each injected new defects, at rising severity.
Before the affected sections are rewritten again, the primary sources have to be
read directly: `skills/config/migrate/SKILL.md` for the real `--list` and
decisions-file contract and the documented four steps, 0119 for the
affordance-on-proceed semantics and the manifest's staleness definition, 0178 and
0167 for what actually happened to `ACCELERATOR_MIGRATION_MODE` and what
`--allow-legacy-layout` replaced it with, 0180 for AC-8's stricter validation and
the `jsonl-common.sh` caller question, and 0167's own frontmatter for the edge it
does and does not carry. Those five reads would resolve all three criticals and
five of the majors.

Two items also need author decisions rather than edits: whether the sizing
conclusion survives contact with the parent-with-children option the epic has
already used (scope raises it with the staged-cutover mechanism spelled out), and
whether the whole-cluster mapping obligation is promoted into the Requirements as
a stated departure from 0167's pattern or scaled back to match them.

## Re-Review (Pass 4) — 2026-07-30

**Verdict:** REVISE

All five lenses re-run after a revision written from the primary sources.
**45 findings: 2 critical, 24 major, 17 minor, 2 suggestions.**

The primary-source reading worked for what it targeted. **All three pass-3
criticals are resolved**, and the lenses confirm it independently: clarity
verified the `--list`, transcript, session-log, 0119, 0166-amendment, 0167-prose
and 0180 AC-7/AC-8 claims as "read as stated"; testability names the split 0119
group and the per-artefact comparison bases as strengths, explicitly retiring
"the goldens with no stated comparison basis defect earlier passes found". The
factual-accuracy failure mode that produced pass 3's regression is gone from the
claims that were checked against sources.

It has been replaced by two other failure modes, and the count went up.

### New Critical Findings

- 🔴 **Testability**: **The transcript golden has no determinate pass/fail.** It
  demands byte-for-byte match while permitting "deviation … for prompt-echo
  framing, named and justified in a list beside the golden" — an exemption list
  the implementer authors after the fact — and then requires the per-decision echo
  to match exactly, overlapping the exempted category. Worse, the documented
  transcript's **first line** is
  `Session log: <SANDBOX>/.accelerator/state/migrations-0099-doc-example-session.jsonl`,
  so any real capture embeds a per-run temp path and no normalisation rule is
  stated — unlike every other golden in the item. A literal byte compare fails on
  the sandbox path alone. This was introduced while reading the very transcript
  that shows the volatile path.
- 🔴 **Testability**: **Byte-for-byte bash `--help` parity is unsatisfiable.** The
  goldens criterion requires "stdout and banners byte-for-byte" and the flag
  criterion requires `--help` to match the bash-derived golden — but the same story
  rewrites every invocation to `accelerator …` and deletes `run-migrations.sh`, so
  the Rust help cannot reproduce bash's usage line or script-path hints. `--list`
  also gets two comparison bases in adjacent criteria ("field-for-field" and
  "byte-for-byte"). Introduced by extending the golden capture set.

### External Couplings No Earlier Pass Found

These are the most consequential findings of the pass, and they were invisible to
the five primary sources the revision consulted.

- 🟡 **Dependency**: **The 0182/0183 cluster is referenced nowhere, from either
  end.** 0182 (plugin-root rename, `ready`, high) renames
  `CLAUDE_PLUGIN_ROOT` → `ACCELERATOR_PLUGIN_ROOT` and carries a criterion that
  **no tracked file under `cli/` contains the string `CLAUDE_`** — which the new
  migrate crate would violate as this item is written. It adds
  `hooks/shim-refresh.sh` as a fourth SessionStart entry that must be appended **at
  index 3**, colliding with this story's `hooks.json` edit. Its
  `CLAUDE_PLUGIN_ROOT` allowlist enumerates `skills/config/migrate/**`, migrations
  `0001`–`0007`, `scripts/interactive-harness.sh` and `hooks/**` — every one of
  which this story deletes. It records that `run-migrations.sh:643` and
  `interactive-lib.sh:433,744` *write* the variable into migration environments, a
  behaviour the Rust engine must replace. And it names the confinement guard this
  item calls `check-call-site-migration.sh` in **Python** form
  (`tasks/lint/skill_permissions.py`), so the artefact a criterion promises to
  update may no longer be a shell script.
- 🟡 **Dependency**: **0183 is an open bug about the exact hook this story ports
  and deletes.** It requires the SessionStart advisory to move off stderr onto a
  top-level `systemMessage` on stdout, and extends the very suite this story
  retires. Whichever lands first invalidates the other. This item's
  discoverability criterion pins no output channel at all.
- 🟡 **Dependency**: **`--allow-legacy-layout` appears nowhere in 0167's own
  record.** The revision correctly relayed 0178's "Notes from 0167" block, but
  never checked 0167 itself: the string occurs in none of its Requirements,
  Acceptance Criteria or Dependencies. 0167 can be marked done without ever
  shipping the flag, at which point migrations `0001`–`0006` cannot read the layout
  they exist to migrate — and 0167's `blocks` deliberately omits 0172, so nothing
  signals the loss. Compounding it: the flag is **CLI-shaped** (`accelerator config
  … --allow-legacy-layout`) while this binary consumes the config crates
  in-process, and no item records a crate-level equivalent.
- 🟡 **Dependency**: **`skills/config/configure/SKILL.md:561` names
  `bash run-migrations.sh --skip`** — a call site outside the `/migrate` skill
  claimed by no story and matched by neither of this item's greps.

### Remaining Majors — internal

- 🟡 **Clarity + Completeness + Testability**: The golden-capture criterion
  anchors to "every fixture named in Technical Notes", where Technical Notes
  carries only the self-referential placeholder "Technical Notes records which
  additionally need a captured bash baseline" — and records nothing. The oracle
  set is therefore undefined, and the capture window closes irreversibly at
  deletion. Several fixture×artefact cells are also inapplicable (`0001`–`0006`
  are non-interactive, so no session log and an empty `--list`).
- 🟡 **Clarity**: Preserved contracts are stated in the vocabulary of the
  mechanism the story deletes — `DONE`, predicate exit 0/1, `RESUMED_APPLIED`,
  `MIGRATION_RESULT` "emitted on stdout … stripped", `migration_session_log_path`
  — with no statement of which are literal observable strings that must survive
  and which are bash-internal shorthand.
- 🟡 **Clarity**: The 0173 bullet says "both directions" and then states 0173
  "mentions 0172 nowhere" — asserting and denying the same edge in one paragraph.
- 🟡 **Clarity**: A criterion guarantees a bash-left repo is "picked up rather
  than stranded" while an Open Question leaves the rule that decides it open, and
  the cutover requirement permits refusal.
- 🟡 **Completeness**: Uncovered obligations — the applied/skipped/pending summary
  counts; the clean-tree pre-flight's three path roots; migration discovery and
  `ACCELERATOR_MIGRATIONS_DIR` (stated only in bash `*.sh` terms that the
  retirement criteria forbid, with no in-crate translation);
  `RESUMED_APPLIED`/`RESUMED_SKIPPED` and the DONE-gated ledger append; the
  abort-on-other-non-zero predicate branch.
- 🟡 **Completeness**: A Drafting Note **over-claims coverage** — it says
  predicate routing, `MECHANICAL_APPLIED` and callback-determinism are "All now in
  Requirements with criteria". `MECHANICAL_APPLIED` appears nowhere else in the
  item and callback-determinism has no criterion. A note that over-claims is worse
  than a silent gap.
- 🟡 **Testability**: "Empty stdout" is asserted for failures that occur *after*
  the mandatory preview banner and prompts have written to stdout — two criteria
  assert an empty stream others require to be non-empty.
- 🟡 **Testability**: The manifest filename is treated as a planning choice
  although 0119 is **done** and its shipped name must be read back for the
  pick-up-not-strand guarantee and the manifest golden.
- 🟡 **Testability**: The normalise-vs-refuse branch for invalid bash-written
  records has no criterion, and the cutover criterion's set-equality assertion
  contradicts normalisation by definition.
- 🟡 **Testability**: Stale-log refusal, manifest refusal and the FORCE bypass have
  overlapping preconditions with different required outcomes and no stated
  precedence; a clean tree at a moved revision has no stated escape, contradicting
  the documented flow.
- 🟡 **Testability**: The timeout criterion's precondition may be unreachable
  depending on the transport still in Open Questions.
- 🟡 **Testability**: The no-gaps check requires one extractor to perform two
  different extractions — assertion counting and control-flow enumeration for the
  depth floor — and the second has no defined procedure.
- 🟡 **Testability**: The whole golden basis depends on an unenforced cross-item
  ordering (0173 deleting the validator bash 0007 needs) with no fallback oracle
  if the window is missed.
- 🟡 **Scope**: The parent-with-children rejection applies a stricter test
  (every child independently *releasable*) than the 0166 precedent satisfied
  (children independently *mergeable*), so the argument does not discriminate —
  and the item concedes a staged cutover is safe here.
- 🟡 **Scope**: The every-assertion mapping is an unmeasured commitment while the
  criterion that would measure it is barred from acting ("it cannot change the
  story's scope"). For calibration, 0167 measured 337 assertions in a 6,289-line
  suite; the row count here is plausibly several hundred and is nowhere estimated.

### Minors of note

`_EXPECTED_MIGRATE_SUITES` is described as an exactly-4 assertion; 0167 records
the same guard as an **at-least floor** (`integration.py:85` compares with `<`).
Four criteria say a grep "returns exactly 0" — grep exits 0 when it *finds*
matches, inverting the intended gate. Extras ordering is given as "receipt order"
where 0180's AC-7 golden pins "declaration order". The retained-and-relocated
`doc-example` fixture would itself match the zero-match harness grep if its bash
migration script is retained. The write-ahead recording port spans two different
write paths (`corpus-adapters` and the `cli/store` `atomic_write`), which one
recorder cannot order. Migrations `0001`–`0006` remain uncharacterised despite a
per-migration fixture-matrix criterion.

### Assessment

Not approved. But the more useful observation is about the trajectory, not this
pass's contents.

Four passes: 48 findings (1 critical) → 37 (0) → 42 (3) → 45 (2). The loop is not
converging. Each round resolves the findings it targeted and surfaces a comparable
number of new ones, because each round adds precision, and added precision creates
new surfaces to contradict. Pass 4's findings are markedly deeper than pass 1's —
they are about normalisation rules, extractor procedures, stream-emptiness
preconditions and cross-item guard ownership — which means the reviews are
tracking a genuinely more precise artefact, not spinning on the same ground.

That points at a diagnosis this pass makes hard to avoid: **the work item is being
asked to carry plan-grade detail.** It now specifies grep corpora, normalisation
rules, extraction procedures, exit-code tables and fixture×artefact matrices — 52
criteria. Those are the things a plan settles, and several pass-4 majors are
precisely complaints that such details are underspecified. The sibling precedent is
explicit: **0119's own review reached APPROVE at pass 3 and deferred
"assertion-grade detail (exact stderr marker token, AC4 condition-splitting,
manifest dedup/ordering) to be settled in `/create-plan`."** 0172 has been held to
a stricter standard than the item whose contract it inherits.

The recommendation is therefore to stop iterating the work item at this
granularity and close it out on a bounded set:

1. **Fix the two criticals** — normalise the transcript's `<SANDBOX>` path and
   close the exemption list; scope `--help` to content parity against a committed
   Rust snapshot rather than bash bytes.
2. **Record the 0182/0183 couplings** and the `--allow-legacy-layout` ownership
   gap, and claim `configure/SKILL.md:561`. These are the findings that would
   otherwise cause real breakage, and two of them need reciprocal records on other
   items.
3. **Fix the mechanical errors** — the at-least floor, the inverted grep phrasing,
   receipt-vs-declaration order, the self-contradictory 0173 sentence, and the
   over-claiming Drafting Note.
4. **Settle the two author decisions** — the sizing shape (the 0166 mergeability
   test, not releasability, is the right comparison) and whether the every-assertion
   mapping gets a measured trigger.
5. **Take the rest to `/create-plan`**, where fixture tables, normalisation rules
   and extraction procedures belong, following 0119's precedent.

Continuing to re-review at this granularity will keep producing majors
indefinitely without the item getting closer to implementable.

## Verdict Change — 2026-08-01

**Verdict:** REVISE → **APPROVE** (set by the author)

Recorded so the frontmatter verdict is not read as a lens outcome. Pass 4's
lens-derived verdict was REVISE; **no fifth pass has run**. The APPROVE reflects
the author's judgement of the work item *after* the bounded close-out that
followed pass 4, which is a state no lens has reviewed.

What the close-out changed, against pass 4's findings:

- **Both criticals fixed.** The transcript golden now uses normalise-then-exact-
  compare (`<SANDBOX>`, `<ID>`) with no exemption list; `--help` is pinned to a
  committed Rust snapshot with content parity rather than unsatisfiable bash-byte
  parity, and `--list` has a single comparison basis.
- **The external couplings are recorded.** 0182 and 0183 added to `relates_to` and
  Dependencies with their operative detail, the `--allow-legacy-layout` ownership
  gap on 0167 stated plainly, and `configure/SKILL.md:561` claimed. Four
  Cross-item-records criteria carry the reciprocal obligations.
- **Shipped-code facts replaced spec paraphrase.** The per-run manifest is a
  sidecar pair at known paths, an empty manifest is valid rather than
  fail-closed, staleness is a base-revision mismatch, session artefacts are owned
  by pattern, and `--list` skips the pre-flight — all read from
  `run-migrations.sh` rather than from 0119's prose, which is looser than the
  implementation.
- **Mechanical errors corrected**: the at-least floor, the inverted grep phrasing,
  declaration-vs-receipt order, the self-contradictory 0173 sentence, and the
  Drafting Note that over-claimed criteria coverage.
- **Both author decisions settled with measurable triggers** — one story unless
  the classification exceeds 400 assertions or planning cannot produce a single
  cutover commit; the exhaustive mapping narrows to the three corruption-risk
  suites above the same threshold. The sizing argument was rebuilt after scope
  correctly showed the original rejection did not discriminate.

What remains open, deliberately, and is now planning work under 0119's precedent:
the fixture × artefact table, exact normalisation tokens, the depth-floor
extraction procedure, stale-log/manifest/FORCE precedence, the stdout-emptiness
preconditions, and the `RESUMED_*`/`DONE` tokens restated in observable terms.
These are recorded in the work item's Drafting Notes.

A reader wanting the lens-assessed state of the item should read the Pass 4
section above; a reader wanting the current state should read the work item.
