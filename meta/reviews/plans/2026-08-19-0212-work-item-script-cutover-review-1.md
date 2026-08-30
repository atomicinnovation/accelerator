---
type: "plan-review"
id: "2026-08-19-0212-work-item-script-cutover-review-1"
title: "Plan Review: Work-Item Script Cutover Implementation Plan"
date: "2026-08-19T11:30:07+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-08-19-0212-work-item-script-cutover"
target: "plan:2026-08-19-0212-work-item-script-cutover"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "safety", "compatibility", "documentation", "usability"]
review_number: 1
review_pass: 3
tags: ["rust", "cutover", "work-items", "sync-engine", "tracker-port"]
last_updated: "2026-08-19T12:12:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Work-Item Script Cutover Implementation Plan

**Verdict:** REVISE

The plan is structurally excellent — feature-first / delete-last sequencing with a green build at every boundary, irreversibility quarantined to Phase 6, the dirty-guard fail-safe correctly reasoned and pinned, and TDD with byte-identical parity goldens throughout. But Phase 3's sync-engine extension rests on three defects that would ship broken behaviour: the new create paths bypass the only mass-write safeguard, the unsynced-create fix targets a code branch keyless items provably never reach, and the create-then-write-back sequence duplicates remote issues on partial failure. Three criticals plus a cluster of reinforcing majors on the same engine seams put this at REVISE; the fixes are localised to Phase 3 and the Phase 2 port shapes, not the plan's architecture.

### Cross-Cutting Themes

- **Create paths escape the blast-radius gate** (flagged by: safety, correctness, architecture) — the live gate counts only `Action::Pull`/`Action::Push` (`plan.rs:42-55`, checked `run.rs:174`). Phase 3's create-from-remote and unsynced-create write new files that neither counter sees, so the 25-item bound silently does not apply to the highest-blast operation — and untracked discovery on a multi-team Linear workspace is known to return 900+ issues. `RunError::Refused { pulls, pushes, … }` also has no dimension for a creation count.
- **Non-idempotent create + separate local write-back = orphan/duplicate hazard** (flagged by: correctness, safety, test-coverage) — `create` is documented non-idempotent (`lib.rs:265-269`); "a single `atomic_write`" protects local file atomicity but not the window between a successful remote create and a failed/crashed write-back. The next run re-creates a second remote issue. No test covers the happy or the injected-failure path.
- **`validate_update` is local-only yet replaces a live-tracker dry-run** (flagged by: correctness, architecture, compatibility, test-coverage) — the plan says the bash `--dry-run` "validate[s] every push against the live tracker" (lines 48-49), but the replacement early-returns before transport for both providers and makes "no port call". A purely local composition check cannot detect a tracker-required field the payload omits — the exact behaviour Phase 3/7 verification demands. It also seats a no-I/O operation on the remote port typed with the remote-failure taxonomy.
- **The `work list` replacement surface is under-specified against what the skill does today** (flagged by: usability, architecture, compatibility) — the plan renders four states (`synced/unsynced/ahead/behind`) where the skill shows five including `conflict`; describes flags "mirroring" a skill that has no flags (it parses natural language); and does not state the exit-0 presence-only degradation the skill guarantees under tracker outage.
- **The stated end-state (empty `skills/work/scripts` grep) is unreachable as scoped** (flagged by: documentation) — Phase 5 repoints three skills, but `review-work-item/SKILL.md:11` and `extract-work-items/SKILL.md:13` also declare the `scripts/*` glob and are never touched, so Phase 6's sweep fails.

### Tradeoff Analysis

- **Literal AC compliance vs preview cheapness** (correctness vs compatibility): Jira `preview_create` calling `discover_projects` meets the "unresolvable project key" AC literally but turns a local resolution into a credentialed network round-trip that can fail `70` where bash resolved offline. Recommendation: keep the live check (the AC is explicit) but model the three outcomes distinctly (resolved / unset / unresolvable) and confirm the skill's pre-create gate degrades on transport failure — don't fold "unresolvable" into `project: None`.
- **Port cohesion vs future pre-flight reuse** (architecture vs code-quality): the three new methods widen `RemoteTracker` with heterogeneous I/O profiles. Either document why co-locating them is deliberate, or split the local-only validation off the remote seam. A one-line rationale in the plan suffices — this need not block.

### Findings

#### Critical

- 🔴 **Correctness**: Create-from-local fix (Gap B) targets an unreachable code branch
  **Location**: Phase 3, Section 2
  A keyless item classifies `Unsynced` → `decide()` maps to `Action::Noop` (`decide.rs:80`); `Action::Push` only arises from `LocallyModified`, which returns *after* the `external_id.is_none()` early return. The `else` guard at `run.rs:219` the plan edits is dead — as written the create path never executes and `work create --push` silently does nothing. Route the create through a new `Unsynced`-under-push action in `decide()`/`classify`, mirroring Gap A.

- 🔴 **Correctness / Safety**: Remote create then local write-back is a non-idempotent orphan/duplicate hazard
  **Location**: Phase 3, Section 2 (Gap B)
  `create` is documented non-idempotent (`lib.rs:265-269`). The single-`atomic_write` claim covers local file atomicity, not the create→write ordering: a crash or write failure after the remote issue exists but before `external_id` is persisted leaves the item keyless, so the next run creates a *second* remote issue and orphans the first — unrecoverable by VCS. Specify the recovery invariant (surface a Terminal failure naming the created id) and add a test injecting a write failure between create and persist, asserting no duplicate create on re-run.

- 🔴 **Safety**: New create actions fall outside the pull/push blast-radius gate
  **Location**: Phase 3, Sections 1 & 2
  The gate counts only Pull/Push. Create-from-remote and unsynced-create write new files the gate cannot see; an all-projects discovery returning hundreds of issues could write hundreds of files in one non-interactive run despite the "same 25-item gate" claim. Extend the refusal predicate (a `max_creates` bound / `create_count()`) and evaluate it before the first creation write, with the promised zero-creations-abort test made explicit against the counted total.

#### Major

- 🟡 **Correctness**: `Discovery` carries no title/body needed to assemble a local file
  **Location**: Phase 2 §1 / Phase 3 §1 (Gap A)
  `Discovery.found` is `Vec<(ExternalId, RemoteTimestamp)>` — no title, no body. Gap A's "full frontmatter + body assembly in memory" has nothing to assemble from; it must additionally `show` per discovered id (never stated), which also makes the "one remote round-trip" performance claim wrong (one `search` + one `show` per untracked issue).

- 🟡 **Correctness**: `validate_update` local-only cannot reproduce the live-tracker dry-run it replaces
  **Location**: Phase 2 §2-3 / Phase 3 §3 (Gap C)
  See cross-cutting theme. Decide whether `validate_update` must issue a remote call (adjusting its error contract and the "no port call" claim) or narrow the AC to locally-detectable omissions and record the dropped live validation.

- 🟡 **Correctness**: `CreatePreview` cannot distinguish an unresolvable project from an unset one
  **Location**: Phase 2 §1-2
  `project: Option<String>` folds "no project configured" (benign) and "configured key does not exist remotely" (the AC target) into one `None`. Model unresolvable as a distinct signal — `Err(Retryable)` on a failed existence check, or a three-state result.

- 🟡 **Architecture**: `validate_update` is a no-I/O operation on the remote port with the remote-failure taxonomy
  **Location**: Phase 2 §1&3 / Phase 3 §3
  A purely local validation on `RemoteTracker` (documented as holding "no logic") typed to `TrackerError` (whose classes divide on "could a remote change have happened?") erodes the functional-core/imperative-shell split. Model it as a pure function in `work`, or document why the remote seam is the deliberate home and which variant a local failure maps to.

- 🟡 **Architecture**: Create-from-remote injects remote-only items into a pipeline keyed entirely on local id
  **Location**: Phase 3 §1 (Gap A)
  `GatheredFacts.per_id` is keyed by local id; the apply loop resolves via `items.iter().find(|c| c.id == planned.id)`. A remote-only carrier has no local id/path/digest and would fail the `find`. Decide explicitly whether create-from-remote is a parallel orchestration path (allocate + assemble + write, outside decide/apply) or a first-class action, and if the latter, specify how the carrier acquires id/path/digest.

- 🟡 **Code Quality**: Three new behaviours concentrated into the already-oversized `run()`
  **Location**: Phase 3
  `run.rs`'s `run()` already carries `#[allow(clippy::too_many_lines)]`. Name the decomposition up front (`discover_untracked`, `apply_create`, `validate_pushes` as separate functions) rather than extending the inline match, so complexity stays proportional.

- 🟡 **Test Coverage**: Gap B (unsynced create) has no success criterion
  **Location**: Phase 3 §2
  The highest-risk new behaviour — remote mutation + file write — ships with neither a happy-path assertion (create called once, id written back atomically) nor an error-path assertion (Terminal create leaves file unwritten, reported not swallowed).

- 🟡 **Test Coverage**: `search` truncation contract property unspecified
  **Location**: Phase 2 §1-2
  A happy-path property passes even if `complete` were hard-coded `true` — the exact regression the plan calls out ("destroys the completeness signal"). Specify a property that *induces* truncation and asserts `complete == false`, mirroring `unaccounted_id_is_indeterminate_not_absent`.

- 🟡 **Test Coverage**: No no-mutation property for `preview_create`/`validate_update`; the Jira shim is manual-only
  **Location**: Phase 2 §1-2
  The whole purpose is to surface a problem *before* mutating; a buggy impl that POSTed, or a shim that misclassified an unresolvable key, passes an Ok/Err-shape-only property. Add a no-mutation property and an offline unit test for the `SurfaceError → TrackerError` mapping.

- 🟡 **Test Coverage**: Gap A positive assembly path has no automated verification
  **Location**: Phase 3 §1
  Only the negative (abort) case is named. Add a criterion asserting a discovered issue produces exactly one local file whose id, `external_id`, frontmatter, and body match the remote projection.

- 🟡 **Safety**: Untracked discovery scope offers only refuse-all or flood-all on a multi-team workspace
  **Location**: Phase 3 §1 / Performance Considerations
  The gate is a refusal, not a cap. Bound the discovery at the query level (project/team-scoped `SearchScope`, not `all_projects`) so the untracked set is inherently small, and treat `complete=false` as refusal-with-guidance rather than a limit to raise.

- 🟡 **Usability**: `work list` renames two states and drops `conflict`
  **Location**: Phase 4 §2
  The skill renders five glyph+text states (`🟢 synced`, `⚪ unsynced`, `🔵 locally modified`, `🟣 remotely modified`, `🔴 conflict`); the plan's "synced/unsynced/ahead/behind" renames two and drops the conflict signal — the exact state a user must act on. Preserve the five-state vocabulary or record the deliberate change and glyph migration.

- 🟡 **Usability**: `work list` flag surface is undefined
  **Location**: Phase 4 §1
  "Mirroring `list-work-items`' current surface" — but that skill has no flags; it parses free text (`tagged X`, `under X`, `status X`, `bugs in review`, title search). Enumerate the concrete flags and map each skill filter rule onto one, so the Phase 5 repoint is unambiguous.

- 🟡 **Documentation**: Two work skills the sweep expects clean are never repointed
  **Location**: Phase 5 vs Phase 6 / Desired End State
  `review-work-item/SKILL.md:11` and `extract-work-items/SKILL.md:13` both declare `Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)` and are mirrored to docs-site. Phase 6's empty-grep sweep fails on them. Add their `allowed-tools` removal to Phase 5 or narrow the sweep — the plan cannot have both as written.

- 🟡 **Documentation**: Docs-site mirror regeneration gated on the wrong command
  **Location**: Phase 5 §4
  `docs:check` only verifies a page exists per skill and no orphans — it never compares mirror *content* to source. A stale mirror still referencing `work-item-*.sh` passes it. Name `mise run docs:generate` as the regeneration step, then `docs:check`, and make the empty-reference check a direct grep over the mirror.

- 🟡 **Documentation**: EXIT_CODES.md fold under-specified — carries stale bash content or drops live codes
  **Location**: Phase 5 §3
  `EXIT_CODES.md` documents deleted scripts (push-decide keywords, per-integration native-code tables) and omits the 0–5 codes `exit_codes.rs` already defines. A verbatim fold imports stale behaviour; a partial fold leaves 0–5 undocumented. Specify exactly which numeric taxonomy survives (0–5 and 70–74), dropping the bash-only sections.

- 🟡 **Compatibility**: `work create --dry-run`/`--push` stdout+exit contract to the skill is unpinned
  **Location**: Phase 5 §2
  The create skill parses tab-separated preview fields (`:506-510`) and feeds exit-70/71+attempt into the push-decide retry seam (`:537-554`). Repointed in one terse bullet, verified only manually. Pin a golden reproducing the parsed fields and state how the exit-code+attempt→keyword retry decision is preserved.

#### Minor

- 🔵 **Correctness**: Create-from-remote action must be counted by the write-bound gate — make the counting explicit in Gap A (overlaps the Safety critical).
- 🔵 **Architecture**: `RunError::Refused` has no creation dimension — extend it (or add a sibling variant) so the abort names the tripped dimension.
- 🔵 **Architecture**: Three heterogeneous ops share one "pre-flight" doc banner — state each method's remote-contact profile separately.
- 🔵 **Code Quality**: The `SurfaceError → TrackerError` shim mapping is unspecified against the Retryable/Terminal taxonomy — state it and pin it with a unit test.
- 🔵 **Code Quality**: New `Action` variant ripples to `from_keyword`, `Display`, and the `awaiting_human` match — enumerate all exhaustive sites, not just the apply loop.
- 🔵 **Test Coverage**: `bash_parity_baseline.rs`'s single-root reader (`:61`) must become multi-root for the four relocated corpora, else three trees drift unguarded or it breaks at Phase 6.
- 🔵 **Test Coverage**: Converted `sync_baseline_shellout_parity.rs` loses the independent bash oracle — document the goldens as frozen (never regenerate from Rust) and confirm the set-guard is the tripwire.
- 🔵 **Test Coverage**: Repointed `exit_codes_parity.rs` reading the Rust constants is self-referential — assert against frozen literal integers / a committed golden table instead.
- 🔵 **Safety**: Phase 7 seed creates real remote issues with production token vars, no target guard, no teardown — require a validated scratch-target id and an idempotent-reseed/teardown path.
- 🔵 **Safety**: Batch id allocation + `atomic_write` (whole-file replace) can clobber a colliding on-disk path — use create-new (refuse-on-exists) semantics.
- 🔵 **Documentation**: 0171's *open* register entries (search / create-dry-run / update-dry-run / EXIT_CODES siting / exit-code document-of-record / contract-run route) are decided by this plan but never flipped to *decided* — instruct the transition, not just appended entries.
- 🔵 **Usability**: Phase 5 repoints `list` "for status-rendering flows" but the skill hand-rolls the whole scan/filter/render in-prompt — clarify what moves behind `work list` vs stays in-prompt.
- 🔵 **Usability**: `work list` should reproduce the skill's directory-not-found guidance, per-file malformed-frontmatter warnings, and empty-result messaging — add as a criterion.
- 🔵 **Usability / Documentation**: The folded exit-code table in a module doc is not `--help`-discoverable — surface it in the relevant subcommands' `--help` as `sync` already does.
- 🔵 **Compatibility**: Jira `preview_create` becomes a credentialed round-trip — confirm the skill's pre-create gate degrades on transport failure (distinct from unresolvable key) and record the divergence.
- 🔵 **Compatibility**: Deleting the bash oracle removes the only external anchor for 70–74 — tie `exit_codes.rs` to the folded table (or a skill-facing golden) so a renumber fails rather than self-agrees.

#### Suggestions

- 🔵 **Code Quality**: Factor frontmatter/body assembly (Gap A create, Gap B write-back, existing pull) through one shared helper reusing `digest::remote_body`.
- 🔵 **Architecture**: Route `work list`'s status column through the existing `SyncState`/`decide` classifier so `list` and `sync` cannot drift.
- 🔵 **Usability**: The non-interactive untracked-pull abort should name the count, the limit, and the concrete remedy (scope the search / raise `--max-pulls`).
- 🔵 **Compatibility**: Require the `public-api.txt` diff to contain only the three methods + `SearchScope`/`Discovery`/`CreatePreview` (no leaked `pub` helper/shim type), not accept the regenerate as-is.
- 🔵 **Compatibility**: Add a `work list` test asserting presence-only render + exit 0 under all-`indeterminate`/transport failure, matching the skill's degradation contract.
- 🔵 **Documentation**: Explain or replace the "38 mirrored references" figure with the concrete grep whose empty output is the recorded evidence.

### Strengths

- ✅ Feature-first / delete-last sequencing with a green `mise run check` at every boundary; irreversibility quarantined to Phase 6 with VCS as the recovery path — a partial cutover never leaves the tree broken.
- ✅ TDD throughout: contract properties and parity goldens written red first, `run_all`'s property count asserted to increase, and every relocated golden held byte-identical to the 0210 oracle with a set-guard in lockstep.
- ✅ The dirty-overwrite guard is preserved and hardened (`Pull` only under `Clean`, `Unknown` fails safe as `Dirty`), pinned by a byte-unchanged + refusal-diagnostic test.
- ✅ `Discovery { found, complete }` is deliberately kept distinct from `FetchOutcome`, preserving the truncation signal instead of collapsing a cap-hit to `Err`.
- ✅ cargo-public-api regeneration correctly scoped to `tracker` only; the 0204 frozen-port override recorded in 0171 rather than made silently; the tracker doc-comment claim verified against source (superseding the work item's stale scrub instruction).
- ✅ The engine extensions are deliberately non-interactive, keeping the prompts in 0213 and preserving a clean functional-core / imperative-shell split.

### Recommended Changes

1. **Re-specify the two create paths through `decide()`/`classify`** (addresses: Gap B unreachable branch, create-from-remote pipeline injection) — introduce create-from-local (`Unsynced` under push) and create-from-remote as first-class actions with a defined route for id/path/digest, rather than editing the dead Push guard or bolting a carrier onto an id-keyed pipeline.
2. **Extend the blast-radius gate and `RunError::Refused` to count creations** (addresses: Safety critical, correctness minor, architecture minor) — a `create_count()`/`max_creates` dimension evaluated before the first creation write, with the zero-creations-abort test asserted against the counted total.
3. **Define the create failure/recovery invariant** (addresses: orphan/duplicate critical, Gap B test gap) — on write-back failure after a successful `create`, surface a Terminal diagnostic naming the created `external_id`; add a test injecting the failure and asserting no duplicate create on re-run. Drop the overclaimed "never half-linked" atomicity language.
4. **Resolve `validate_update`'s local-vs-remote contract** (addresses: correctness major, architecture major, compatibility) — either issue a remote validation (adjusting the error contract and the "no port call" claim) or narrow the AC to locally-detectable omissions and record the dropped live validation.
5. **Correct the `Discovery` assembly path** (addresses: Discovery lacks title/body, performance claim) — add a bounded `show`-per-discovered-id step (or extend `Discovery` to carry assembly fields) and fix the round-trip cost note.
6. **Model the unresolvable-project signal distinctly** (addresses: `CreatePreview` conflation) — a three-state result or `Err(Retryable)` on a failed existence check, not `project: None`.
7. **Nail down the `work list` surface** (addresses: state vocabulary, flag surface, degradation, dual-classifier) — preserve the five-state vocabulary including `conflict`, enumerate the flags mapping each skill filter rule, route the column through the existing classifier, and assert the exit-0 presence-only fallback.
8. **Fix the docs/skill-sweep inconsistency** (addresses: two un-repointed skills, docs:generate gating, EXIT_CODES fold, 0171 open entries) — add `review-work-item`/`extract-work-items` `allowed-tools` removal to Phase 5, name `docs:generate` + a content grep, specify which exit-code taxonomy survives the fold, and flip 0171's matching *open* entries to *decided*.
9. **Add the missing Phase 2 contract properties** (addresses: search truncation, no-mutation, shim mapping) — induced-truncation `complete == false`, no-mutation for both pre-flight ops, and an offline `SurfaceError → TrackerError` mapping test.
10. **Name the `run()` decomposition and shared assembly helper up front** (addresses: run.rs concentration, assembly duplication) so complexity stays proportional as the three behaviours land.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Structurally disciplined — feature-first/delete-last with green boundaries, irreversibility isolated, non-interactive engine preserving the core/shell split. Main risks in the port extension: `validate_update` is purely local yet on the remote seam with the remote-failure taxonomy, and create-from-remote injects remote-only items into a pipeline keyed end-to-end on local id.

**Strengths**: Single ordering invariant (no script deleted while referenced); non-interactive engine deferring prompts to 0213; `Discovery` distinct from `FetchOutcome`; `search` preserves the truncation signal; frozen-port override recorded, normalise folded rather than re-exposed.

**Findings**:
- 🟡 major (medium): `validate_update` is purely local yet lives on the remote port with the remote-failure taxonomy (Phase 2 §1&3 / Phase 3 §3).
- 🟡 major (medium): Create-from-remote injects remote-only items into a pipeline keyed entirely on local id (Phase 3 §1).
- 🔵 minor (high): Creation blast-radius has no home in the existing `RunError::Refused` shape.
- 🔵 minor (medium): Three heterogeneous operations lumped under one "pre-flight" banner weaken port cohesion.
- 🔵 suggestion (low): Guard against a second classification path in `work list` — route through `SyncState`/`decide`.

### Code Quality

**Summary**: Well-grounded in existing code — names exact seams, reuses ported modules, models `Discovery` deliberately. Main risks: concentration of three orchestration behaviours into the already-oversized `run()`, trait responsibility growth, and an under-specified error-mapping shim.

**Strengths**: TDD with byte-identical goldens; Phase 4 reuses label/decide/digest/baseline/fetch; `Discovery` modelled to domain; shared compose helper + single `atomic_write`; forced ordering keeps the tree buildable.

**Findings**:
- 🟡 major (high): Phase 3 folds three behaviours into `run.rs`'s `run()`, already at its complexity ceiling.
- 🔵 minor (medium): `RemoteTracker` grows 4→7 methods mixing mutation, bulk read, and pre-flight validation.
- 🔵 minor (medium): `SurfaceError → TrackerError` shim mapping unspecified against the two-class taxonomy.
- 🔵 minor (medium): New `Action` variant ripples to `from_keyword`/`Display`/`awaiting_human`, only the apply loop named.
- 🔵 suggestion (low): Factor frontmatter/body assembly (Gap A / Gap B / existing pull) into one helper.

### Test Coverage

**Summary**: Unusually test-conscious — explicit TDD, byte-for-byte 0210 goldens, red-before-green properties. Strongest gaps on the new mutating engine paths (Gap B and Gap A assembly have no positive/error criteria) and in under-specified contract properties for the three new port methods (truncation, no-mutation).

**Strengths**: Byte-frozen oracle + lockstep set-guard; property-count-increase assertion; sharp Phase 3 edge-case tests named; Phase 7 all-three-gates-required fail-closed design.

**Findings**:
- 🔴 major (high): Gap B (unsynced create) has no success criterion for the non-idempotent create + write-back.
- 🟡 major (high): `search` truncation contract property unspecified — happy-path passes with `complete` hard-coded true.
- 🟡 major (medium): No no-mutation property for the pre-flight ops; the Jira shim is Phase-7-manual-only.
- 🟡 major (medium): Gap A positive assembly path (id/external_id/frontmatter/body) has no automated verification.
- 🔵 minor (medium): `bash_parity_baseline.rs`'s single-root reader must become multi-root for the four corpora.
- 🔵 minor (medium): Converted shellout parity test loses the independent bash oracle — document goldens as frozen.
- 🔵 minor (medium): Repointed `exit_codes_parity.rs` reading the constants is self-referential.
- 🔵 minor (low): Fakes' configurable behaviour (seed/truncate/fail) unspecified; Phase 3 tests need those seams.

### Correctness

**Summary**: Rigorous about state and invariants (dirty-guard, two-tier read, truncation flag), but the two engine-extension gaps rest on incorrect readings of the decision flow and leave two non-idempotent-mutation hazards unaddressed. Most serious: the unsynced-create fix targets a branch keyless items provably never reach.

**Strengths**: Dirty-guard correctly read as fail-safe; `FetchOutcome` totality preserved via truncation flag, untracked set errs toward under-creating; arithmetic self-check (relocated + deletions = 68); deletions ordered strictly last.

**Findings**:
- 🔴 critical (high): Create-from-local fix (Gap B) targets an unreachable code branch.
- 🔴 critical (high): Remote create then local write-back is a non-idempotent orphan hazard.
- 🟡 major (medium): `Discovery` carries no title/body needed to assemble a local file (needs `show`; cost note wrong).
- 🟡 major (medium): Local-only `validate_update` cannot reproduce the live-tracker dry-run it replaces.
- 🟡 major (medium): `CreatePreview` cannot distinguish an unresolvable project from an unset one.
- 🔵 minor (medium): New create-from-remote action must be counted by the write-bound gate.

### Safety

**Summary**: Safety-conscious in structure (deletions last, dirty-guard fail-safe with a test, gate ahead of every write), but material risks concentrate in the new write paths: create actions are not covered by the pull/push-only gate, and unsynced-create pairs a non-idempotent remote mutation with a separate local write-back the single-atomic-write claim does not protect.

**Strengths**: Dirty-guard preserved and hardened; gate before any write in both modes; irreversible deletions quarantined behind five green phases; broad Phase 6 consumer sweep; `read_failure` surfaced not swallowed.

**Findings**:
- 🔴 critical (high): New create actions fall outside the existing pull/push blast-radius gate.
- 🟡 major (high): Cross-boundary half-linked state when remote create succeeds but local write-back fails.
- 🟡 major (medium): Untracked discovery scope offers only refuse-all or flood-all on a multi-team workspace.
- 🔵 minor (medium): Live seed step creates real remote issues with production token vars and no teardown.
- 🔵 minor (low): Batch id allocation + `atomic_write` could clobber an existing path — use refuse-on-exists.

### Compatibility

**Summary**: Central surface is the SKILL↔`accelerator work …` contract (exit codes, stdout formats, frozen port). Machine-checked contracts handled well (byte-identical goldens, 70/71 preserved, public-api scoped to `tracker`, truncation flag). Residual risk in the skill↔CLI output contract, repointed tersely and verified only manually.

**Strengths**: 70/71 preserved as `tracker`-owned constants; byte-identity guarded not trusted; `search` returns a truncation flag; public-api regeneration scoped to `tracker`, override recorded.

**Findings**:
- 🔴 major (medium): `work create --dry-run`/`--push` stdout+exit contract the skill parses is unpinned.
- 🔵 minor (medium): Jira `preview_create` becomes a credentialed round-trip — new failure surface vs bash local resolution.
- 🔵 minor (medium): Deleting the bash oracle leaves the exit-code parity test self-comparing; 72–74 lose their pinned membership.
- 🔵 suggestion (medium): Constrain the `public-api.txt` diff to the three intended additions only.
- 🔵 suggestion (low): Assert `work list` presence-only + exit 0 under tracker outage, matching the skill's degradation contract.

### Documentation

**Summary**: Diligent about documentation obligations (EXIT_CODES re-siting, mirror regeneration, doc-comment accuracy, 0171 recording, each decision paired with an assertion). Three consistency gaps would let stale/contradictory references survive: mirror regeneration gated on the wrong command, two un-repointed skills, and an under-specified EXIT_CODES fold.

**Strengths**: Each recorded decision paired with an independent assertion; tracker doc-comment claim verified against source; dirty-guard doc point preserved and re-pointed; `docs:check` correctly noted absent from aggregate `check`.

**Findings**:
- 🔴 major (high): Docs-site mirror regeneration gated on `docs:check`, which never validates content freshness — name `docs:generate`.
- 🔴 major (high): `review-work-item` and `extract-work-items` still declare the `scripts/*` glob and are never repointed — Phase 6 sweep fails.
- 🟡 major (medium): EXIT_CODES.md fold under-specified — verbatim carries stale bash content; partial leaves 0–5 undocumented.
- 🔵 minor (medium): 0171's *open* register entries this plan decides are never flipped to *decided*.
- 🔵 suggestion (low): The "38 mirrored references" figure is unexplained (a grep returns 21 across 5 files).

### Usability

**Summary**: Careful green-at-every-boundary sequencing (excellent DX) and the `work list` verb fits the surface, but thin on the actual ergonomics: the flag surface "mirrors" a skill that has no flags, the status vocabulary diverges from what users see today, and error/empty-state/exit-code discoverability of the replacements is under-specified.

**Strengths**: `work list` name consistent with existing verbs; Phase 4 manual parity check against current rendering; phased ordering keeps `mise run` green; shrinking `allowed-tools`; folding normalise avoids a drift-prone primitive.

**Findings**:
- 🟡 major (medium): `work list` renames two states and drops `conflict` from the five-state vocabulary.
- 🟡 major (medium): `work list` flag surface undefined — the skill parses natural language, not flags.
- 🔵 minor (medium): Phase 5 repoints `list` "for status-rendering" but the skill hand-rolls the whole scan/filter/render.
- 🔵 minor (medium): Skill's directory-not-found / malformed-frontmatter / empty-result messaging may not survive.
- 🔵 minor (medium): Folded exit-code table in a module doc is not `--help`-discoverable at runtime.
- 🔵 suggestion (low): The non-interactive untracked-pull abort should name count, limit, and remedy.

## Re-Review (Pass 2) — 2026-08-19T11:47:34+00:00

**Verdict:** REVISE

All 22 critical/major findings from the initial review are resolved by the Pass-1
edits — no regressions, and every prior concern is now either handled in the plan
text or confirmed a strength. The deeper second pass, reviewing the revised
design, surfaced a fresh set of second-order majors: the two new port types
(`CreatePreview.issue_type`, `validate_update`'s `Result<(), TrackerError>`)
under-model the outcomes the create/preview flows must render; the untracked-write
path is sequenced ahead of the `--preview` read-only short-circuit; the create
recovery reinvents the existing `pending_push` marker; and the new `work list`
filter surface plus the migrated retry seam lack tests. These are refinements on a
now-sound plan, not structural faults — but there are more than three, so the
threshold holds at REVISE.

Two edits already applied during this pass (self-inconsistencies introduced in
Pass 1): the Testing Strategy "four states" line corrected to five, and the
exit-code fold reworded so only 70/71 map to the `TrackerError` split.

### Previously Identified Issues

- 🔴 **Correctness**: Gap B edits an unreachable branch — **Resolved** (rerouted through `decide()`/`classify`).
- 🔴 **Correctness/Safety**: create-then-write-back orphan — **Resolved** (recovery invariant + inject-failure test; residual crash-window is a new finding below).
- 🔴 **Safety**: creations escape the blast-radius gate — **Resolved** (`creates`/`max_creates` dimension, before-any-write, project-scoped discovery).
- 🟡 **Correctness**: `Discovery` lacks title/body — **Resolved** (bounded `show`-per-id; cost note corrected).
- 🟡 **Correctness/Architecture/Compatibility**: `validate_update` local-only vs live dry-run — **Resolved** (now a remote pre-flight; new return-type/Linear refinements below).
- 🟡 **Correctness**: `CreatePreview` unresolvable-vs-unset — **Resolved** (three-state `ProjectResolution`; issue-type parallel is a new finding below).
- 🟡 **Architecture**: create-from-remote in an id-keyed pipeline — **Resolved** (parallel `discover_untracked` path; producer-asymmetry is a new minor below).
- 🟡 **Code Quality**: three behaviours in `run()` — **Resolved** (named `discover_untracked`/`apply_create`/`validate_pushes` + shared assembly helper).
- 🟡 **Test Coverage** (×4): Gap B test, search truncation, no-mutation, Gap A assembly — **Resolved** (all named as criteria).
- 🟡 **Safety**: untracked discovery refuse-all/flood-all — **Resolved** (project-scoped `SearchScope`, `complete==false` refuses with guidance).
- 🟡 **Usability** (×2): five-state vocabulary, flag surface — **Resolved** (five states via single classifier; flags enumerated).
- 🟡 **Documentation** (×3): two un-repointed skills, `docs:generate`, EXIT_CODES fold — **Resolved** (§2b added; `docs:generate` + grep gate; fold scoped — taxonomy attribution refined this pass).
- 🟡 **Compatibility**: create `--dry-run` stdout contract unpinned — **Resolved** (golden pinning the parsed fields; issue-type five-field gap is a new finding below).
- 🔵 All 16 minors + 6 suggestions — **Resolved** (multi-root reader, frozen-oracle note, non-self-referential exit-code test, refuse-on-exists, Phase 7 guard, 0171 open entries, public-api diff constraint, per-method profiles, `Action` ripple, single classifier, degradation, remedy diagnostic, etc.).

### New Issues Introduced

None are regressions of resolved findings; all are deeper analysis of the revised design.

- 🟡 **Compatibility**: `CreatePreview.issue_type: Option<String>` cannot carry the issue-type *source* (default vs kind-mapped), so the create skill's five-field tab-separated render is not fully recoverable — mirror `ProjectResolution` with a three-state issue-type resolution.
- 🟡 **Code Quality**: `validate_update -> Result<(), TrackerError>` has no home for a tracker-reported *invalid payload* — model `Result<ValidationOutcome, TrackerError>` (`Valid` / `Rejected { reasons }`), reserving `TrackerError` for transport.
- 🟡 **Architecture**: Linear has no non-mutating update-validation surface — the symmetric "remote pre-flight validation" is not uniformly satisfiable; name Linear's actual check and model the asymmetry (weaker guarantee) explicitly.
- 🟡 **Correctness**: `discover_untracked` writes are sequenced before the `RunMode::Preview` short-circuit (`run.rs:183`), so `--preview` would create files — gate the write on `Apply`; in `Preview` do the reads and report `CreateFromRemote` as `NotApplied`.
- 🟡 **Correctness**: the `validate_pushes` loop may drop non-`Push` actions from the frozen preview report — state it reports the full plan, attaching a validation outcome only to `Push` entries; add a mixed-plan preview test.
- 🟡 **Safety**: the create recovery invariant covers graceful write-back failure but not a *crash* (SIGKILL/OOM) between remote `create` and id-persist — needs a durable pre-create marker or provider idempotency key.
- 🟡 **Usability**: the bespoke create recovery diverges from the existing auto-idempotent `pending_push` (`ReuseId`) marker — route create-from-local through it so recovery matches `create --push` (this also discharges the Safety crash-window).
- 🟡 **Usability**: the abort diagnostic names a `--max-creates` remedy the plan never adds to `SyncArgs` — add the flag, decide the create-from-local counting, update the exit-5 `--help`.
- 🟡 **Test Coverage**: no criterion asserts the enumerated `work list` filter flags actually select the right subset (single, multi-flag, empty-match, repeatable `--tag`).
- 🟡 **Test Coverage**: the untracked-set difference (search minus local ids) is not directly tested with a mixed tracked/untracked discovery.
- 🟡 **Test Coverage**: the migrated exit-70-retry / exit-71-terminal decision has no behavioural test and no pinned home.
- 🔵 Minors: create-from-local `from_keyword`/`Display`/`awaiting_human` wiring unenumerated; `ExternalId` set-difference normalisation unstated; mid-batch `show`-failure policy unspecified; create-new write should be temp+rename (`O_EXCL`), not just refuse-on-exists; Phase 7 scratch target needs an allowlist, not just a presence check; new port methods need `# Errors` doc sections; retry-decision authoritative doc home; `RemoteTracker` seven-method sub-trait tradeoff; `Action::CreateFromRemote` out-of-band producer doc-comment; 72–74 attributed to `SelectionError` not `TrackerError`; `--preview` offline-degradation criterion; native-code mapping discoverability; `timed_conformance` evidence assertion; `work sync --help` scope note.

### Assessment

The plan is in materially better shape than at Pass 1 — the three criticals and every major are gone, and the structure (feature-first/delete-last, single classifier, gated writes, honest failure handling) is sound. The remaining work is a bounded set of type-modelling and preview-mode sequencing refinements plus test additions, concentrated in Phase 2 (the two under-modelled port types) and Phase 3 (preview read-only discipline, create recovery via `pending_push`). One item — Linear's lack of a non-mutating validation surface — is a consequence of the remote-validation decision worth an explicit call before implementation. None blocks the overall approach.

## Re-Review (Pass 3) — 2026-08-19T12:07:08+00:00

**Verdict:** COMMENT

All eight lenses ran a third time against the twice-revised plan, after two decisions: `validate_update` reverted to a **local-only** check for both providers, and the pass-2 findings applied. No criticals and no regressions — every pass-2 finding stayed resolved. The pass surfaced a final tranche of refinement-level majors, all now applied in the same session; the strongest signal was a three-lens convergence (Safety + Usability + Test Coverage) that the new `--max-creates` flag was the wrong model. The verdict is COMMENT because the surfaced items were design-refinements on a sound plan rather than blocking faults, and all have been folded in; a confirmatory fourth pass is optional.

### Pass-3 findings and disposition

- 🟡 **Usability / Safety / Test Coverage** — `--max-creates` conflated a VCS-recoverable local-file creation with an irreversible remote-issue creation under one bound with an unspecified default, breaking the `--max-pushes 0 = touch nothing remote` guarantee. **Applied**: folded by blast direction — create-from-remote counts against `--max-pulls`, create-from-local against `--max-pushes`; no new flag; the refusal diagnostic breaks out the creation sub-count per dimension.
- 🔴→🟡 **Correctness** — create-from-remote writes were sequenced *before* the post-`compute_plan` pull/push gate, so a refused run could leave files on disk. **Applied**: discovery now does reads-only first; one combined gate over all three dimensions runs before any write; the out-of-band discovery count is summed into `plan.pull_count()` (it never enters `plan.actions`).
- 🟡 **Correctness** — the claimed canonicalised `ExternalId` comparison had no surface (`ExternalId` derives `Eq` over the raw string; `per_id` keys on the local id). **Applied**: a free canonicalising function over `ExternalId::as_str()` in `work-adapters`, keeping `tracker`'s public API frozen; the folding is defined once and tested.
- 🟡 **Safety** — the untracked-pull write path lacked a `SyncDirection` gate. **Applied**: gated to `PullOnly`/`Bidirectional`, with a `--push-only` no-local-files test.
- 🟡 **Test Coverage** — the differential-oracle removal's replacement tripwire was overstated (only sync-baseline is content-hashed; the set-guard checks names, not bytes). **Applied**: extend a content-hash guard to every relocated golden; co-locate `corpus_hashes.rs` with its relocated corpus.
- 🟡 **Test Coverage** — the `pending_push` recovery test only covered a pre-seeded marker, not the marker-before-`create` ordering. **Applied**: added a test observing marker presence at the point `create` is invoked.
- 🟡 **Architecture / Code Quality** — a purely-local, infallible `validate_update` sits on the remote-contact `RemoteTracker` port. **Applied (documented)**: recorded in 0171 that it stays on the port because payload composition is provider-specific (reuses each client's compose helper), that the non-`Result` shape is deliberate, and that reintroducing remote validation is a signature-breaking change — a conscious, revisitable boundary.
- 🔵 **Correctness / Code Quality** — the `Action` exhaustive-site list was inaccurate (missed `action_keyword` at `sync.rs:152`; `awaiting_human` is a non-exhaustive `matches!`; the apply-loop arm for `CreateFromRemote` is a *mandatory* `unreachable!`). **Applied**: enumeration corrected for both new variants.
- 🔵 Minors/suggestions — exit-code table single-authoritative-source (module doc; `--help` derives), 71's dual create-vs-update rationale, `work list --help` doc text, offline-create degradation criterion, Linear preview-rendering golden, local-only preview contract stated to skills, report golden with create rows, `SearchScope` precedence doc, discovered-item `SyncState` marker excluded from `awaiting_human`, preview over-threshold reports-without-fetch. **All applied.**

### Assessment

Across three passes the plan moved from 3 criticals + 18 majors to a fully-applied set of refinements with no open criticals or regressions. The design is sound and the mechanisms are now specified to implementation depth — port types model their outcomes, all writes sit behind one pre-flight gate, create recovery reuses the durable marker, and the blast-radius model stays directional. The plan is ready for implementation; a fourth verification pass would be confirmatory only.

## Approval — 2026-08-19T12:12:17+00:00

**Verdict: APPROVE.** All findings across three review passes are resolved and applied, with no open criticals, majors, or regressions. The plan is approved for implementation. Reviewer: Toby Clemson.
