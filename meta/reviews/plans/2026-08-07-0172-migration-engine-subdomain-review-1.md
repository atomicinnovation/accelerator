---
type: plan-review
id: "2026-08-07-0172-migration-engine-subdomain-review-1"
title: "Plan Review: Migration Engine Subdomain (accelerator-migrate) Implementation Plan"
date: "2026-08-07T10:18:07+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: ""
target: "plan:2026-08-07-0172-migration-engine-subdomain"
relates_to: []
reviewer: Toby Clemson
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, safety, compatibility, usability, documentation]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-07T14:01:48+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Migration Engine Subdomain (accelerator-migrate) Implementation Plan

**Verdict:** REVISE

All eight lenses independently praised the plan's discipline — bash-golden
capture before any deletion, byte-for-byte parity with a fixed normalisation
set, structural enforcement of the write-ahead-log invariant, and an
assertion-inventory approach to test-suite retirement that goes well beyond
typical shell-to-native ports. But the plan also carries a genuine internal
contradiction over a documented env var, several underspecified trait/type
boundaries that will block implementation at Phase 8, a data-race in the
session-log cutover path, and a cluster of unresolved obligations on an
unplanned sibling work item (0195) sitting on the plan's only two blocked
phases — one of which is the sole cutover commit. None of these are fatal to
the plan's overall architecture, but there are more than enough major-severity
findings to warrant a revision pass before implementation starts.

### Cross-Cutting Themes

- **The 0195 cross-item dependency is a compounding risk, not a single one**
  (flagged by: Architecture, Compatibility, Safety) — Phase 8's interface
  design (`extract_linkages`), Phase 8's self-validation runtime fallback
  (`validate-corpus-frontmatter.sh`), and Phase 0's golden-capture window all
  independently depend on how and when work item 0195 lands, and 0195 has not
  yet committed to any of the three. Each lens found a different facet of the
  same underlying coupling; together they suggest this dependency needs to be
  resolved as a precondition of starting Phase 8, not worked out reactively
  once 0195 is planned.
- **The plan contradicts itself on `ACCELERATOR_MIGRATIONS_DIR`** (flagged
  by: Compatibility, Usability) — Key Discoveries says it's one of three env
  vars "the ported surface honours"; Phase 2 says it "becomes moot" and is
  dropped entirely. Two independent lenses caught the same textual
  contradiction from different angles (compatibility-contract risk vs.
  least-surprise risk), which is a strong signal it needs resolving before
  implementation, not during it.
- **The interactive timeout/TTY adapter is under-specified and
  under-tested** (flagged by: Test Coverage, Correctness, Safety) — the
  concrete `TtyDecisionSource` (thread + `recv_timeout`) has no described
  automated test, the "timeout never armed" structural guarantee (Phase 5,
  point 4) contradicts the single generic engine call site described in point
  2, and the detached-thread-on-timeout design assumes a short-lived process
  with no structural guard against future reuse. This is the single riskiest
  piece of genuinely new concurrent code in the plan and currently has the
  thinnest verification story.
- **Trait/context boundaries between the mechanical and interactive engines
  are underspecified** (flagged by: Architecture, Code Quality) — the
  relationship between `Migration::apply`'s `&dyn MechanicalContext` and
  `InteractiveMigration`'s `&dyn MigrationContext` is never stated, and the
  `verify_applied` hook is declared but never called anywhere the plan
  describes. Both stem from Phase 2/5 not fully specifying the context-trait
  shape before Phase 8 needs to use it.

### Findings

#### Critical

None.

#### Major

- 🔴 **Architecture, Compatibility**: Phase 8/10 depend on an interface
  (0195's `extract_linkages`) that work item 0195 has not committed to
  providing in the required shape
  **Location**: Phase 8: Migration 0007 Port; Cross-item obligation on 0195
  0195's own text commits only to a CLI subcommand inside `accelerator-corpus`'s
  binary crate, not the library-level, infrastructure-free export Phase 8
  requires — and no edge recording this obligation exists on 0195 yet.

- 🔴 **Compatibility, Usability**: The plan internally contradicts itself on
  whether `ACCELERATOR_MIGRATIONS_DIR` is honoured
  **Location**: Key Discoveries vs. Phase 2, Core Lifecycle Engine, point 1
  Key Discoveries lists it as one of three honoured env vars; Phase 2 says
  it "becomes moot" and describes dropping it as a deliberate narrowing.

- 🔴 **Correctness**: The session-log cutover rewrite bypasses the mkdir-lock
  that ordinary record writes acquire
  **Location**: Phase 4: JSON Record Model, Changes Required #3
  `AtomicWrite::write` (used for the one-time cutover rewrite) doesn't
  acquire the lock `RecordStore::append_record`/`remove_by_key` require,
  letting the highest-stakes session-log mutation race a concurrent locked
  writer.

- 🔴 **Correctness, Code Quality**: Two non-equivalent, unresolved detection
  strategies are offered for identifying a non-canonical session-log record
  **Location**: Phase 4: JSON Record Model, Changes Required #3
  The "field-order check" and the "parse-then-validate" alternatives aren't
  equivalent — the second can't actually detect "needs cutover" — and the
  first silently depends on `serde_json`'s `preserve_order` feature, which
  isn't mentioned.

- 🔴 **Correctness**: The "timeout never armed" structural guarantee
  contradicts the single generic engine call site described two sections
  earlier
  **Location**: Phase 5: Interactive Framework, points 2 and 4
  Point 2 describes one generic `next_decision(t, timeout)` call site; point
  4 asserts `NoInputDecisionSource` is never called with a non-zero timeout —
  these can't both be true without an undescribed per-source special case.

- 🔴 **Correctness**: The VCS staleness comparison doesn't address
  `vcs::RepoFacts.revision` being `None`
  **Location**: Phase 3: Guarded Resume & Path Manifest, Changes Required #4
  A fresh repo with no resolvable revision (a real, tested case in `cli/vcs`)
  has no stated staleness rule, risking a silent violation of the
  guarded-resume fail-closed guarantee for new users.

- 🔴 **Safety**: The golden-capture race with 0195 is acknowledged but not
  mitigated until Phase 10
  **Location**: Phase 0: Fixture Matrix & Bash Golden Capture
  If 0195 deletes `scripts/validate-corpus-frontmatter.sh` before Phase 0
  runs, the bash oracle for 0007's self-validation is destroyed before it can
  be captured — the reciprocal edge closing this race isn't recorded until
  Phase 10.

- 🔴 **Safety**: No run-level lock guards the ledger/manifest against
  concurrent `accelerator migrate` invocations
  **Location**: Phase 2: Core Lifecycle Engine; Phase 3: Guarded Resume
  A pre-existing bash TOCTOU gap is carried forward unchanged, even though
  the codebase already has the mkdir-lock primitive (`corpus-adapters`) that
  could close it cheaply during this rewrite.

- 🔴 **Compatibility**: Self-validation's bash fallback conflicts with
  0195's own planned deletion of the same script
  **Location**: Phase 8: Migration 0007 Port, Changes Required #3
  Phase 8's documented fallback (shell out to `validate-corpus-frontmatter.sh`
  if 0195's Rust equivalent isn't ready) assumes the script still exists —
  but 0195's Requirements explicitly retire and delete it.

- 🔴 **Code Quality**: Unreconciled context types between the mechanical
  apply loop and the interactive engine
  **Location**: Phase 2, Changes Required #1; Phase 5, Changes Required #1–2
  `Migration::apply` takes `&dyn MechanicalContext`; `InteractiveMigration`'s
  callbacks take `&dyn MigrationContext`. Migration 0007 implements both, and
  the plan never states how one `apply()` bridges the two.

- 🔴 **Test Coverage**: The concrete `TtyDecisionSource` adapter has no
  described automated test
  **Location**: Phase 5, Changes Required #3; Success Criteria
  The timeout test substitutes a generic `DecisionSource` test double, not
  the real thread-and-channel adapter; a real terminal only appears in
  Manual Verification.

- 🔴 **Test Coverage**: No described test coverage for the assertion-inventory
  extractor's own correctness
  **Location**: Phase 9, Changes Required #1
  `tasks/lint/migrate_suite_inventory.py` is the sole gate for the
  "no gaps, no duplicates" guarantee underpinning Phase 10's irreversible
  deletion, but its own detection logic has no fixture-based test proposed.

- 🔴 **Documentation**: The documentation deliverable targets a path that
  doesn't exist in the current tree
  **Location**: Phase 10: Documentation, item 3; References
  `docs/migrations.md` was relocated; the current path is
  `docs-site/src/content/docs/migrations.md`. `docs:check`/`docs:build` are
  deliberately excluded from `mise run check`, so a miss here wouldn't be
  caught by CI.

- 🔴 **Documentation**: No CI-enforced check that the rewritten worked
  example stays accurate
  **Location**: Phase 10: Documentation, item 3
  The "doctested in CI" claim (from the work item's own AC) has no named
  test; the bash-era equivalent (`test-migrate-interactive.sh`'s
  `extract_block` scraping) is deleted in the same commit with no successor.

- 🔴 **Architecture**: The mandatory single Phase 10 cutover commit bundles
  six structurally unrelated concerns behind two upstream blockers
  **Location**: Phase 10: Retirement Cutover
  Deletions, call-site rewrites, floor adjustments, docs, and five separate
  cross-item bookkeeping edges are gated on Phase 8 (→0195) and Phase 9;
  Phase 9's own text acknowledges the single-commit constraint may not hold.

#### Minor

- 🟡 **Architecture, Code Quality**: `verify_applied` trait hook is declared
  but never invoked anywhere the plan describes (Phase 5, Changes Required #1)
- 🟡 **Architecture**: `MechanicalContext`/`MigrationContext` port contents
  are never specified, risking an unbounded "god context" (Phase 1, point 2;
  Phase 2, point 1)
- 🟡 **Code Quality**: Interactive callback errors are stringly-typed
  (`String`) rather than following the codebase's typed-error convention
  (Phase 2, point 1; Phase 5, point 1)
- 🟡 **Compatibility, Correctness, Test Coverage**: Bash's argv-order-dependent
  short-circuit among `--skip`/`--unskip`/`--unapply` isn't addressed for
  clap's order-insensitive parsing, and no test covers combined-flag
  invocations (Phase 1, Changes Required #3)
- 🟡 **Compatibility**: clap's default error handling for a missing flag
  value or unknown flag likely diverges from bash's custom usage text and
  exit code (Phase 2, Changes Required #2)
- 🟡 **Compatibility**: `hooks.json`'s plugin-root token is left unresolved
  pending implementation-time verification rather than pinned now (Phase 7,
  Changes Required #3)
- 🟡 **Usability, Architecture**: The discoverability hook's flag-based
  invocation breaks the subcommand convention every sibling sub-binary uses
  (Phase 7, Overview)
- 🟡 **Usability**: "Ordinary Rust, no framework" undersells a real,
  uncompiler-checked authoring contract (determinism, WAL ordering,
  sticky-skip, source-drift) (What We're NOT Doing; Phase 5)
- 🟡 **Usability**: Unrecognised migration IDs are silently accepted with no
  validation on `--skip`/`--unskip`/`--unapply`, deferring the error signal
  (Phase 2, Changes Required #3)
- 🟡 **Test Coverage**: Discoverability hook's negative-path and
  legacy-fallback-chain behaviours aren't named in Phase 7's Success Criteria
- 🟡 **Correctness**: The manifest's "append at time-of-mutation" guarantee
  has no described wiring mechanism ensuring every write call site
  participates uniformly (Phase 3, Success Criteria)
- 🟡 **Safety, Test Coverage**: The detached stdin-reader thread on timeout
  assumes a short-lived, single-invocation process, with no structural
  guard or automated test of its actual exit/lifecycle behaviour (Phase 5,
  point 3)
- 🟡 **Safety**: The single indivisible Phase 10 cutover commit has a
  heavyweight recovery path (VCS revert) that is never itself tested
  (Phase 10, Overview)
- 🟡 **Documentation**: The documentation deliverable is far less specified
  than every other deliverable in the plan — no mapping from bash's ~230
  lines of author-facing contract to a Rust equivalent (Phase 10, item 3)
- 🟡 **Documentation**: Unclear which phase updates the many illustrative
  bash-invocation examples embedded in SKILL.md's prose beyond the primary
  binding (Phase 7, item 4)

#### Suggestions

- 🔵 **Architecture**: The write-ahead-log invariant is enforced by a runtime
  test double, not the type system — consider a `Recorded` token type
  constructible only from a successful append, so a reordering bug fails to
  compile (Phase 5, point 2)
- 🔵 **Code Quality**: `render.rs` is accumulating rendering logic for four
  or more unrelated lifecycle stages across phases — consider per-concern
  submodules as each phase adds to it
- 🔵 **Safety**: Dangerous-path refusal exists only for migration 0005;
  migrations 0001/0003 compute destinations from config without a confirmed
  equivalent guard (Phase 6)
- 🔵 **Safety**: No explicit panic-boundary is described around migration
  `apply()`/callback invocations — clarify whether a panic still preserves
  the typed-error exit-code/ledger-state contract (Phase 2, point 1)
- 🔵 **Usability**: Four distinct manifest-failure causes (absent, empty,
  unreadable, stale) collapse into one indistinguishable refusal message —
  consider a troubleshooting note without touching the pinned golden text
  (Phase 3, point 1)
- 🔵 **Usability**: `ACCELERATOR_MIGRATE_FORCE` has no CLI flag equivalent
  and isn't documented alongside `ACCELERATOR_MIGRATE_DECISIONS_FILE` in
  `--help`
- 🔵 **Usability**: The non-configurable 30s decision timeout may be tight
  for genuine human review of 0007's transformations — low risk, already
  isolated behind one constant (Phase 5, point 3)
- 🔵 **Documentation**: Consider decoupling the documentation rewrite from
  the single indivisible cutover commit — draft it once Phase 5/8 land a
  concrete `InteractiveMigration` implementation to document against

### Strengths

- ✅ Phase 0 captures bash-derived goldens as the first ordered step, before
  any Rust code is written, with a fixed comparison basis per artefact type
  and an explicit "do not loosen a pattern to make a failing golden pass"
  guard — exactly the right foundation for a byte-for-byte port.
- ✅ The write-ahead-log invariant (session-log append before corpus
  mutation) is enforced structurally via a test double that fails the run if
  the calls are observed out of order — genuine mutation-testing-grade
  design, not a documented convention.
- ✅ Idempotency for migrations 0001–0006 is tested two independent ways
  (ledger filter and direct double-invocation of the transform), and every
  migration's own idempotency self-check is preserved as belt-and-suspenders
  per ADR-0023.
- ✅ Phase 9's assertion-inventory approach — classifying every assertion in
  all six retiring suites, checked for gaps/duplicates in CI — is
  materially stronger than typical practice for a shell-to-native port of
  this size.
- ✅ The three-crate `migrate`/`migrate-adapters`/`migrate-cli` split
  precisely mirrors the proven `vcs-cli` template, including reusing its
  cargo-pup enforcement patterns rather than inventing new mechanisms.
- ✅ Moving migrations in-process eliminates two named FIFOs, literal fds, a
  30s watchdog, and a dual hand-rolled JSON escaper in one stroke — a
  genuine, well-justified simplification, not merely a language port.
- ✅ Deliberate behavioural narrowings (detached reader thread, dropped
  `ACCELERATOR_MIGRATIONS_DIR` override in one section, narrower timeout
  scope) are explicitly called out and justified rather than silently
  introduced.
- ✅ Every write funnels through atomic temp+rename primitives with a path
  manifest appended at time-of-mutation, and the session-log cutover is
  fail-closed (refuses and leaves the file byte-unchanged on an invalid
  record) rather than normalising or dropping data.
- ✅ `--help` is deliberately pinned to a committed Rust snapshot with
  content-parity (not byte-for-byte bash) comparison, correctly recognising
  that literal byte parity is structurally unachievable once the invocation
  name changes.

### Recommended Changes

1. **Resolve the `ACCELERATOR_MIGRATIONS_DIR` contradiction in one place**
   (addresses: Compatibility/Usability major finding) — either drop it from
   the "three honoured env vars" list and document the removal explicitly in
   the Phase 10 doc rewrite, or design the compile-time registry's
   test-injection point as its functional replacement and say so.

2. **Treat the 0195 interface as a precondition of starting Phase 8, not a
   parallel task** (addresses: Architecture/Compatibility/Safety findings on
   the 0195 coupling) — land the reciprocal edge and interface amendment on
   0195 itself before Phase 8 begins, and extend it to cover both the
   linkage-extraction signature and the self-validation script's lifetime,
   not just the golden-capture ordering.

3. **Specify the `MechanicalContext`/`MigrationContext` relationship and the
   `verify_applied` call site before Phase 8** (addresses: Code Quality major
   finding, Architecture/Code Quality minor findings) — either declare one
   trait a supertrait of the other or collapse them, and either name
   `verify_applied`'s call site or drop it from the trait.

4. **Route the session-log cutover through the existing lock and commit to
   one detection strategy** (addresses: Correctness major findings on the
   cutover) — acquire the same lock `append_record`/`remove_by_key` require,
   and pick either the field-order check (with `preserve_order` noted
   explicitly) or an unconditional idempotent rewrite, not both as
   alternatives.

5. **Resolve the Phase 5 timeout-armament contradiction** (addresses:
   Correctness major finding) — state explicitly whether the guarantee comes
   from `NoInputDecisionSource` ignoring its timeout argument or from the
   composition root passing a distinguishable value for that source.

6. **Add an explicit `revision: None` staleness rule** (addresses:
   Correctness major finding) — treat it as always-stale, consistent with
   the rest of the manifest-usability gate's fail-closed philosophy, and add
   a fixture for a VCS-present-but-no-commits-yet resume attempt.

7. **Add a run-level advisory lock around the ledger/manifest** (addresses:
   Safety major finding) — reuse the existing mkdir-lock pattern rather than
   carrying the concurrent-invocation race forward unchanged.

8. **Correct the documentation target path and name the doctest mechanism**
   (addresses: Documentation major findings) — point Phase 10 at
   `docs-site/src/content/docs/migrations.md`, and name the specific Rust
   test (e.g. a ported `extract_block`-style scrape) that keeps the worked
   example honest in CI.

9. **Add automated coverage for `TtyDecisionSource` and the assertion-inventory
   extractor** (addresses: Test Coverage major findings) — both currently
   have only manual or no verification for code that gates, respectively, a
   concurrency-sensitive user-facing path and an irreversible deletion.

10. **Consider whether Phase 10's deletion/call-site work can be separated
    from the 0195-gated and cross-item-bookkeeping pieces** (addresses:
    Architecture major finding on the bundled cutover commit) — reduces the
    blast radius of the 0195 gate to only what genuinely needs it.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan mirrors the established `domain`/`adapters`/`cli` three-crate hexagonal split (cli/vcs-cli as template) faithfully and correctly, and the core architectural move — eliminating the FIFO/fd IPC, the 30s process-level watchdog, and the dual hand-rolled JSON escaper by making migrations in-process trait implementations — is a genuine, well-justified simplification that removes a whole class of concurrency and escaping failure modes. The main structural risks are outside the crate design itself: a hard cross-item interface dependency on an unplanned work item (0195) gates two phases including the sole cutover commit, and that cutover commit bundles six largely-unrelated concerns into one indivisible unit. A few interface boundaries (the `MechanicalContext`/`MigrationContext` ports, the `verify_applied` trait method) are introduced without their contract being fully specified.

**Strengths**:
- The `migrate`/`migrate-adapters`/`migrate-cli` split precisely mirrors `vcs-cli`, including reusing its cargo-pup rule patterns.
- Moving migrations in-process eliminates FIFOs, fds, a watchdog, and a dual JSON escaper in one stroke.
- The `DecisionSource` port cleanly isolates the one genuinely unbounded operation behind a single trait boundary.
- The plan reuses already-landed abstractions (`corpus::Record`, `config::paths::doc_type_dirs`, `vcs::RepoFacts`) rather than introducing parallel domain types.
- Trade-offs (narrower timeout scope, abandoned reader thread, compile-time-fixed registry) are surfaced explicitly and justified against the AC's own text.

**Findings**: See merged findings above (Phase 8/0195 interface dependency — major; Phase 10 bundled cutover commit — major; `verify_applied` never invoked — minor; context ports unspecified — minor; WAL invariant test-double-only — suggestion; discoverability hook flag convention — minor).

### Code Quality

**Summary**: The plan is unusually disciplined for a bash-to-Rust port: it consistently reuses existing crate abstractions rather than reinventing them, threads dependencies through injected ports for testability, and explicitly documents every deliberate behavioural narrowing instead of silently introducing them. The main gaps are in the trait/type design sketched across Phase 2 and Phase 5: the relationship between the mechanical `Migration` trait, the `InteractiveMigration` trait, and their two differently-named context types is never reconciled, and error handling in the interactive callbacks reverts to bare `String` rather than the codebase's established typed-error convention.

**Strengths**:
- Consistent dependency injection throughout, mirroring `vcs-cli`/`corpus-adapters`.
- The write-ahead-log invariant is proposed to be verified by a test double that fails the run if the order is violated.
- Deliberate simplifications are explicitly called out and justified.
- Session-log handling reuses `corpus::Record`/`FileCorpusStore` outright instead of building a parallel record type.

**Findings**: See merged findings above (unreconciled context types — major; `verify_applied` never invoked — minor; stringly-typed callback errors — minor; `render.rs` scope creep — suggestion; session-log cutover detection ambiguity — folded into Correctness major).

### Test Coverage

**Summary**: This plan's testing strategy is unusually rigorous for the mechanical, non-concurrent surface — bash-golden byte/substring pinning, table-driven domain tests, write-ahead-log ordering enforced via a fail-fast test double, and a completeness-inventory approach to suite retirement that goes well beyond typical practice. The one genuine soft spot is the concrete `TtyDecisionSource` adapter — the plan's own timeout test explicitly substitutes a generic `DecisionSource` test double rather than exercising the real adapter. A handful of smaller coverage gaps exist around the assertion-inventory extractor's own correctness and a few unlisted negative-path tests.

**Strengths**:
- Phase 0's bash-golden capture, taken first, with a fixed comparison basis per artefact type.
- The write-ahead-log invariant test is genuine mutation-testing-grade design.
- Idempotency for migrations 0001–0006 is tested two ways.
- Phase 9's assertion-inventory approach is materially stronger than typically seen for a port of this size.
- Refusal-path tests consistently assert both the observable failure and a structural invariant.

**Findings**: See merged findings above (`TtyDecisionSource` untested — major; assertion-inventory extractor untested — major; flag-combination test gap — minor; discoverability hook negative-path untested — minor; detached-thread lifecycle untested — folded into Safety minor).

### Correctness

**Summary**: The plan is unusually rigorous about state-machine edge cases (ownership classes, sticky skip, source drift, write-ahead-log ordering enforced via a test double) and correctly eliminates an entire class of process-lifecycle races by moving migrations in-process. However, several concurrency and boundary-condition details are underspecified or internally inconsistent: the session-log cutover's atomic rewrite bypasses the same lock ordinary record appends acquire, the canonical-form detection driving that cutover is described by two non-equivalent algorithms, the "timeout never armed" structural guarantee conflicts with the single generic engine call site described elsewhere in the same phase, and the VCS staleness key's `None` case is never addressed.

**Strengths**:
- The write-ahead-log invariant is enforced not just as a stated rule but as a structurally-tested one.
- The guarded-resume ownership design is explicitly fail-closed by default.
- Retiring the FIFO/fd IPC and watchdog genuinely removes a large surface of concurrency hazards.
- Per-migration idempotency self-checks are explicitly retained as a second guard alongside the ledger filter.

**Findings**: See merged findings above (session-log lock bypass — major; cutover detection ambiguity — major; timeout-armament contradiction — major; staleness `None` case — major; manifest wiring mechanism — minor; argv-order flag handling — minor).

### Safety

**Summary**: The plan is unusually careful about data-corruption prevention where it addresses it directly: the write-ahead-log invariant is enforced structurally, the session-log cutover fails closed, and every write goes through atomic temp+rename primitives plus a path-manifest appended at time-of-mutation. The main gaps are less about the migrations' own mutation logic and more about the surrounding process: an explicitly acknowledged, currently-unguarded race with 0195 that could destroy the bash oracle before Phase 0 captures it, no run-level lock protecting the ledger/manifest against concurrent invocations, and a very large, gated, single-commit cutover whose only recovery mechanism (VCS revert) is never itself tested.

**Strengths**:
- The write-ahead-log invariant is enforced structurally via a failing test double.
- The bash-session-log cutover is explicitly fail-closed, resolving the work item's own open question in favour of the safer option.
- Every migration write funnels through atomic, temp-file+rename primitives with a path manifest appended at time-of-mutation.
- Migration idempotency self-checks are explicitly preserved as belt-and-suspenders beyond the ledger filter.
- The guarded-resume ownership model keeps bash's fail-closed default.

**Findings**: See merged findings above (golden-capture race — major; no concurrent-invocation lock — major; untested cutover-commit revert path — minor; detached-thread assumption — minor; asymmetric dangerous-path refusal — suggestion; no panic boundary — suggestion).

### Compatibility

**Summary**: The plan is unusually disciplined about CLI-contract compatibility — Phase 0's bash-golden capture, the fixed masks.toml normalisation set, and content-vs-byte parity treatment for `--help` all show a mature approach to preserving an external contract. However, it contains a direct internal contradiction over whether `ACCELERATOR_MIGRATIONS_DIR` survives the port, and it makes Phase 8/10 completion depend on an interface that 0195's own work item does not yet commit to providing in the shape 0172 needs. Several smaller CLI-parity gaps (clap's default error/exit-code behaviour, order-dependent flag short-circuiting, an unresolved hooks.json token) round out the risk surface.

**Strengths**:
- Phase 0 captures bash-derived goldens before any deletion and fixes a single, narrow, explicitly-permitted normalisation set.
- `--help` is deliberately pinned to a committed Rust snapshot with content-parity comparison, following 0167's own precedent.
- `ACCELERATOR_MIGRATION_MODE` is kept permanently dead and cross-checked against 0178's existing negative test.
- The "in-process transport" contract is backed by a committed cargo-pup rule, not left as a prose promise.
- Phase 9's assertion-inventory approach is a materially stronger test-parity discipline than typical shell-to-native rewrites.

**Findings**: See merged findings above (`ACCELERATOR_MIGRATIONS_DIR` contradiction — major; 0195 interface not committed — major; self-validation fallback conflicts with 0195's deletion plan — major; clap default error handling — minor; argv-order flag handling — minor; hooks.json token unresolved — minor).

### Usability

**Summary**: The plan is unusually disciplined about preserving the existing CLI's error and stall messages byte-for-byte, which is good for continuity, but it introduces a few real developer-experience frictions of its own: an internal inconsistency about whether `ACCELERATOR_MIGRATIONS_DIR` is honoured, an acknowledged-but-unresolved invocation-shape decision for the discoverability hook that breaks the subcommand convention every sibling sub-binary uses, and a migration-authoring story that markets itself as "ordinary Rust, no framework" while actually asking authors to self-police several invisible, uncompiler-checked contracts.

**Strengths**:
- Phase 7 moves the discoverability advisory from stderr to the `systemMessage` envelope — a concrete, well-motivated fix to a real usability defect.
- Exact string-literal preservation means existing users and tooling see zero behavioural surprise during the cutover.
- Phase 10 requires the worked example be "compiled or doctested in CI", directly addressing docs rotting out of sync.
- The plan explicitly declines scope creep and calls out intentional narrowings rather than silently regressing.

**Findings**: See merged findings above (`ACCELERATOR_MIGRATIONS_DIR` contradiction — major; discoverability hook convention break — minor; "ordinary Rust" undersells authoring contract — minor; unvalidated migration IDs — minor; manifest-failure-cause message collapse — suggestion; `ACCELERATOR_MIGRATE_FORCE` discoverability — suggestion; 30s timeout tightness — suggestion).

### Documentation

**Summary**: The plan is unusually rigorous about reproducing user-facing strings verbatim with file:line citations. However, the actual authoring-documentation deliverable (Phase 10, item 3) is the least-specified change in an otherwise meticulous ten-phase plan: it names a target file that no longer exists in the tree, carries no file-level change breakdown or success-criteria checkbox unlike every other deliverable, and doesn't say what becomes of the author-facing contract SKILL.md currently carries. The work item's own AC requirement that the worked example be "compiled or doctested in CI" has no corresponding CI check named anywhere in the plan.

**Strengths**:
- Exceptional discipline reproducing every bash-era user-facing string verbatim with file:line citations.
- The plan correctly identifies and enforces the registration/skill-binding coupling (points 1 and 7 of the 13-point checklist).
- Phase 7 explicitly folds 0183's abandoned advisory-delivery fix into this item's own discoverability documentation with a reciprocal status update.

**Findings**: See merged findings above (wrong docs target path — major; no doctest CI check — major; documentation deliverable underspecified — minor; unclear scope of SKILL.md prose updates — minor; decouple docs from cutover commit — suggestion).

## Re-Review (Pass 2) — 2026-08-07T12:01:26+00:00

**Verdict:** REVISE

All 10 recommended changes from the initial review were applied. Most held
up well under re-review — the `ACCELERATOR_MIGRATIONS_DIR` contradiction,
the session-log lock bypass, the cutover detection ambiguity, the
timeout-armament contradiction, the `revision: None` staleness gap, the
0195 self-validation conflict, the wrong documentation path, and the
missing doctest mechanism are all confirmed resolved by independent
lenses. However, three of the fixes were themselves incomplete and
introduced new major-severity issues, concentrated in two places: the new
`MigrationContext` trait (Change 3) and the new run-level lock (Change 7).
A pre-existing factual error in the plan's own bash-behaviour claim about
the 30s timeout was also newly surfaced with strong evidence. None of this
is a regression in intent — every gap is a specific, scoped correction to
this pass's own edits — but it means another edit pass is needed before
the plan is ready.

### Previously Identified Issues

- 🟢 **Compatibility, Usability**: `ACCELERATOR_MIGRATIONS_DIR` contradiction — Resolved
- 🟢 **Correctness**: Session-log cutover lock bypass — Resolved
- 🟢 **Correctness, Code Quality**: Cutover detection ambiguity — Resolved (new edge case found: absent-file handling, see below)
- 🟢 **Correctness**: Timeout-armament contradiction (point 2 vs point 4) — Resolved (a deeper, pre-existing bash-parity error was found underneath, see below)
- 🟢 **Correctness**: Staleness `revision: None` case — Resolved
- 🟡 **Safety**: Golden-capture race with 0195 — Partially resolved (mechanism added, but degrades to an unenforced manual note when 0195 doesn't yet exist)
- 🟡 **Safety, Architecture, Correctness**: No run-level lock — Partially resolved (lock added, but doesn't cover `--skip`/`--unskip`/`--unapply`, and its holding span is ambiguous for multi-migration runs)
- 🟢 **Compatibility**: Self-validation fallback vs. 0195's deletion plan — Resolved
- 🟡 **Code Quality, Architecture**: Unreconciled context types — Partially resolved (unified into one trait, but the unified trait now breaks domain-crate isolation, and no dispatch mechanism was specified for routing 0007 through the interactive engine)
- 🟢 **Test Coverage**: `TtyDecisionSource` untested — Resolved (new gap found: the *selection* logic choosing between adapters is still untested, see below)
- 🟢 **Test Coverage**: Assertion-inventory extractor untested — Resolved
- 🟡 **Documentation**: Wrong docs target path — Resolved in the prose (independently verified `docs-site/src/content/docs/migrations.md` exists), but a regression-guard grep check in the same phase still scans the stale `docs/` path
- 🟢 **Documentation**: No CI doctest check — Resolved
- ⚪ **Architecture**: Bundled Phase 10 cutover commit — Still present (left as-is per explicit decision; a note was added to Phase 7 instead)

### New Issues Introduced

- 🔴 **Architecture** (high confidence): `MigrationContext`'s `config()`/`vcs()` methods return sibling-domain-crate types directly, contradicting `migrate`'s stated "corpus + kernel only" dependency footprint and the workspace's established sibling-domain-crate isolation convention (`corpus`/`vcs` depend only on `kernel`, enforced by pup.ron rules) — Phase 1/2
- 🟡 **Architecture** (medium confidence): No mechanism specified for `run_pending()` to route migration 0007 through `run_interactive()` instead of `.apply()` — `Migration` and `InteractiveMigration` as currently typed can't be dispatched between without an undescribed downcast/enum mechanism — Phase 2/5/8
- 🔴 **Architecture, Correctness** (confirmed independently by both lenses, high confidence): The run-level lock never guards `--skip`/`--unskip`/`--unapply`, since those flags short-circuit before pre-flight (where the lock is acquired) — the exact race the lock was introduced to close remains open on that path — Phase 1/3
- 🟡 **Safety, Correctness** (confirmed independently by both lenses): The lock's "pre-flight-to-ledger-append span" wording is ambiguous for a run applying multiple pending migrations — a literal reading could release the lock after the first migration's ledger append, leaving the rest of the run unprotected — Phase 3
- 🔴 **Compatibility** (high confidence): Phase 7's new note ("this rebinding is safe only because 0195 lands first") contradicts the plan's own "independently mergeable now" framing for Phase 7 — nothing structurally prevents Phase 7 landing before Phase 8, which would silently drop migration 0007 coverage from both the rebound skill and the discoverability advisory
- 🟡 **Safety** (medium confidence): The Phase 0 golden-capture-race fix degrades to an unenforced manual note ("must be re-checked immediately before Phase 0 executes") when work item 0195 doesn't yet exist — no automated gate gives it teeth
- 🔴 **Documentation** (high confidence): The Phase 10 regression-guard grep for stale bash vocabulary still targets `docs/` instead of `docs-site/`, so it never actually scans the rewritten documentation file
- 🟡 **Documentation** (high confidence): `mise run docs:check` is listed under Automated Verification but the prose says it's a manual step per the repo's own convention — the two are inconsistent
- 🔴 **Correctness** (high confidence): The plan's claim that the 30s TTY decision timeout narrows an existing bash watchdog is factually wrong — the cited watchdog (`interactive-lib.sh:937-954`) bounds post-decision child teardown, not the decision wait itself; bash's actual TTY read (`interactive-lib.sh:268-269`) has no timeout at all. The 30s timeout is new, stricter behaviour introduced by this port, not a narrowing of existing bash behaviour — Key Discoveries / Phase 5
- 🟡 **Test Coverage**: No test for the session-log cutover when no log file exists yet (the ordinary first-ever-run case) — Phase 4
- 🟡 **Test Coverage**: The production `DecisionSource` selection logic itself (as opposed to the test-injectable override seam) has no test covering each real-world TTY/decisions-file combination — Phase 1/5
- 🟡 **Test Coverage**: The new run-level lock's stale/orphaned-holder reclaim path (crashed prior process) has no fixture test, only the live-concurrent-holder case — Phase 3
- 🔵 **Architecture, Usability, Code Quality** (raised independently by three lenses, minor): The hidden test-only flag/env var for forcing `TtyDecisionSource` selection ships undocumented in the release binary with no `cfg(test)` gating specified — Phase 1
- 🔵 **Correctness** (minor): Ambiguous whether source-drift discarding applies to sticky-skipped records, or only accept/edit records — the two clauses describing this aren't cross-referenced — Phase 5

### Assessment

The core edits from pass 1 are sound in intent and mostly verified correct
in isolation. The two places needing a third pass are the `MigrationContext`
trait (fix the domain-isolation violation and specify the mechanical/
interactive dispatch mechanism) and the run-level lock (extend coverage to
the flag-mutation paths, fix the holding-span wording, add the stale-reclaim
test). The Phase 7 blocking gap and the Phase 0 manual-fallback gap are both
small, mechanical fixes. The timeout-parity correction is a documentation/
framing fix, not a behavioural one — the 30s bound itself was already
correctly justified by the AC directly; it just needs to stop being
described as bash parity. Not yet ready for implementation.

## Re-Review (Pass 3) — 2026-08-07T14:01:48+00:00

**Verdict:** APPROVE

Two further rounds of targeted verification followed Pass 2, each re-running
Architecture, Code Quality, and/or Correctness against the plan's own edits
(not a full 8-lens sweep) and fixing what they found:

**Round A** (verifying Pass 2's edits): confirmed the `MigrationContext`
domain-isolation fix, the `MigrationEntry` dispatch mechanism, the lock's
holding-span/flag-coverage fix, and the Phase 7 blocking annotation all held
up. Found and fixed: `MigrationContext` referenced `corpus::CorpusIndex` and
`FileCorpusStore::replace_with_lock`, neither of which exist — replaced with
a locally-declared `CorpusIndex` port and an explicitly-scoped new
`FileCorpusStore::replace_locked` method; Phase 3's lock/dirty-path-scan
logic bypassed the local-port pattern (would have failed to compile under
Phase 1's own cargo-pup rule) — fixed via new `RunLock`/`DirtyPathScanner`
local ports; the plan's claim that the 30s TTY timeout narrows an existing
bash watchdog was factually wrong (bash's TTY read has no timeout at all,
verified against `interactive-lib.sh:269`) — corrected throughout; and a
significant discovery — `cli/corpus/src/linkage.rs` already implements most
of what Phase 8 assumed 0195 needed to build, narrowing 0195's obligation
down to just the self-validation script's lifetime, removed entirely for
linkage extraction — propagated through Current State Analysis,
Implementation Approach, Desired End State, Phase 7, Phase 8, Phase 9,
Phase 10, and References.

**Round B** (verifying Round A's edits, checked directly against the
`corpus`/`corpus-adapters` source): found two critical bugs, both fixed.
The pup.ron `allowed_only` list for `migrate`'s domain-isolation rule
omitted `corpus` despite `migrate` depending on `corpus::Record`/
`RecordStore`/`linkage` directly throughout the plan — cargo-pup would have
rejected the plan's own required imports; fixed by adding `^corpus(::|$)`
everywhere the rule is specified. `CorpusIndex::target_exists`'s proposed
ID-derivation convention (`corpus::work_item_id`/`corpus::slug`) was wrong
for almost every doc type — verified directly against
`cli/corpus/src/linkage.rs`, `cli/corpus/src/slug.rs`, and
`cli/corpus/src/work_item_id.rs` — and would have silently dropped valid,
existing cross-references during migration 0007's port (a real data-loss
bug), both for non-work-item types and for work-items under a
project-code-configured `id_pattern`; fixed by deriving both sides through
the same function (`resolve_path_target`) so the comparison is correct by
construction, with dedicated tests for both failure modes. Also fixed: the
session-log cutover reached into an adapter type directly from the domain
crate (added a `SessionLogRewriter` port); the `cfg(test)`-gated test seam
would have been invisible to the black-box fixture tests that need it
(`CARGO_BIN_EXE_*` binaries aren't built with `cfg(test)`) — corrected to
the env-var-gated design as primary, not a fallback; the run-level lock
reused `corpus-adapters::lock`'s 5-minute default retry ceiling, contradicting
the plan's own "refuse promptly" intent — fixed with an explicit 2-second
ceiling and a documented fallback for the narrow PID-lookup TOCTOU window
during reclaim; and two stale references to sub-traits removed in an
earlier edit were cleaned up.

### Assessment

Five review passes, each finding progressively fewer and narrower issues —
the last round's findings were two specific, well-isolated technical
corrections rather than structural gaps, indicating convergence. Every
finding across all passes was either fixed directly in the plan or resolved
through an explicit, recorded decision (the Phase 10 cutover-commit
structure, left as-is per the plan author's judgement that 0195 will land
in parallel). The plan's domain-crate isolation, dispatch mechanism,
concurrency safety, 0195 dependency scope, and documentation obligations
are now internally consistent and verified against the actual landed
codebase rather than assumed. Approved for implementation.

---
*Review generated by /accelerator:review-plan*
