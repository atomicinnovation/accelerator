---
type: plan-review
id: "2026-06-18-0051-sync-work-items-skill-review-1"
title: "Plan Review: Sync Work Items Skill"
date: "2026-06-18T13:17:32+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-06-18-0051-sync-work-items-skill"
target: "plan:2026-06-18-0051-sync-work-items-skill"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, correctness, safety, test-coverage, code-quality, usability, portability, performance]
review_number: 1
review_pass: 7
tags: [work, integrations, sync]
last_updated: "2026-06-19T00:21:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Sync Work Items Skill

**Verdict:** REVISE

This is a disciplined, well-researched plan: it sits on a deliberately-prepared
seam, keeps state derivation in a single shared engine consumed by both skills,
routes integration I/O through bridges, splits the functional core (pure,
TDD-first Phases 1–4) from the imperative shell, and phases the work into
genuinely independently-mergeable increments. The architecture instincts are
right and the change-detection invariant ("pre-filter may only short-circuit to
*unchanged*") is sound. However, the review surfaced **five critical issues**
that block implementation as written — most seriously, the core *push* path
relies on an update bridge that does not exist, the *remote-side* change check
has no persisted baseline to compare against, and a remote-ahead pull silently
overwrites local edits with no diff or confirmation. Several of these recur
across multiple lenses, which raises confidence that they are real.

### Cross-Cutting Themes

- **The push / update path is undefined** (flagged by: correctness, architecture,
  safety, test-coverage) — Phase 6 pushes local-ahead items "via
  `work-item-create-remote.sh`-style update path", but that bridge only
  *creates*; no update dispatch (to `jira-update-flow.sh` / `linear-update-flow.sh`)
  is wired anywhere, the read bridge only does `search`/`show`, and the
  `--preview` dry-run it leans on doesn't exist for updates. The dominant write
  operation in bidirectional sync — and the conflict-override push — therefore
  has no implementable, previewable, or testable path.
- **The remote side of the 2×2 has no baseline to compare against** (flagged by:
  correctness, architecture) — `last-sync.json` persists only `remote_updated_at`
  and `local_hash`; there is no remote content or `remote_hash`. So the Phase 4
  contract's `normalise(remote body) == normalise(baseline-equivalent)` branch
  is uncomputable, and the likely fallback (remote-now vs local-now) conflates
  "remote changed since baseline" with "sides currently differ", breaking the
  conflict / remotely-modified / synced distinction.
- **Per-item `show` is an N+1 that the pre-filter cannot save** (flagged by:
  performance, architecture) — the remote pre-filter needs `remote.updated`,
  which on the list path comes *from* a per-item `show`; the timestamp gate
  suppresses the body comparison but not the round-trip, so every tracked item
  costs a network call on every list/sync even in steady state. The bulk
  `search --fields updated,...` already injected in Phase 1 carries enough to
  collapse this to one call.
- **Orchestration logic lives in SKILL prose, not testable scripts** (flagged by:
  test-coverage, code-quality) — the (mode, state) → action matrix, the
  conflict report-and-skip rule, the batch no-resurrect-declines rule, and the
  resumability commit ordering are all SKILL-prose the model executes. The
  codebase already extracts exactly this class of safety-critical logic into a
  pure script (`work-item-push-decide.sh`); without an analogous
  `work-item-sync-decide.sh`, the core correctness surface (and the resumability
  AC for a non-VCS-recoverable operation) cannot be covered in CI.
- **The committed team-shared baseline has multi-machine hazards** (flagged by:
  architecture, safety, correctness, performance, portability) — Decision #2's
  safety argument is analysed only for the local side; a committed
  `remote_updated_at` is the *committer's* last-seen remote state, the
  fresh-checkout "mtime is always newer" assumption isn't guaranteed across
  skewed clocks / mtime-preserving tooling, and cross-machine hash determinism
  additionally depends on byte-stable remote ADF and a fixed locale.
- **Portability of the new primitives** (flagged by: portability, correctness) —
  the engine compares local file mtime against an ISO-8601 string but never says
  how mtime is read (`stat -f` BSD vs `stat -c` GNU; epoch-int vs ISO unit
  mismatch); `diff` output differs across userlands; the inlined `_win_sha256`
  re-implements the audited portable helper.

### Tradeoff Analysis

- **Safety vs. story-locked policy (remote-default-on-conflict)**: Usability and
  Safety both flag that accepting *remote* on a bare Enter destroys local edits —
  the opposite polarity to the safe-default `[y/N]` used everywhere else in the
  suite. This is a deliberate story policy, not a plan error, but the plan should
  make the inversion loud and the consequence explicit inline rather than leaving
  wording to prose.
- **Performance/correctness (bulk search) vs. fidelity (per-item show)**: bulk
  `search` removes the N+1 but may return a lower-fidelity `description` than
  `show`. Recommendation: use bulk search for the `updated` pre-filter for the
  whole corpus, and fall back to `show` only for the genuinely-changed minority
  that need a body — confirm search-`description` is comparison-equivalent first.
- **Committed vs. gitignored baseline**: shareable team baseline (current choice)
  vs. per-machine correctness. The decision is defensible but its multi-writer
  blast radius is under-analysed; worth an explicit note (or ADR) given it also
  governs the byte-pinned gitignore copies.

### Findings

#### Critical

- 🔴 **Correctness / Architecture / Safety / Test-Coverage**: Push (local-ahead)
  path is undefined — the create bridge only creates, no update dispatch is wired
  **Location**: Phase 1 (read bridge) / Phase 6 (core reconciliation) / Phase 7
  (override push)
  Phase 6 routes pushes through a "`work-item-create-remote.sh`-style update
  path", but that bridge only creates (guarded by exit 109 against re-creating a
  synced item), the read bridge wires only `search`/`show`, and the existing
  `jira-update-flow.sh` / `linear-update-flow.sh` update primitives are never
  referenced. The push direction, its `--preview` dry-run, and the
  override-pushes-local resolution all have no implementable write path.

- 🔴 **Correctness / Architecture**: Remote-side comparison has no stored
  baseline to compare against
  **Location**: Phase 4: Change-detection engine (remote-side comparison)
  `last-sync.json` stores only `remote_updated_at` and `local_hash` — no remote
  content or hash. When remote `updated` differs from baseline, there is nothing
  persisted that represents remote content at last sync, so the contract's
  `normalise(remote body) == normalise(baseline-equivalent)` branch cannot be
  evaluated; the data-supported fallback (remote-now vs local-now) misclassifies
  pure local edits as conflicts.

- 🔴 **Safety**: Pull-overwrite silently destroys local edits in default /
  `--pull-only` mode with no diff or confirmation
  **Location**: Phase 6: Reconcile synced items (remote-ahead → overwrite local)
  Only the both-changed "conflict" path shows a diff and prompts. A
  "remotely-modified" verdict overwrites the local file in place with no signal;
  any classification false-negative on the local side (stale shared baseline,
  mtime pre-filter false-negative, edit on another machine) is clobbered with no
  in-flow indication that a loss occurred to make the VCS-revert path actionable.

- 🔴 **Test-Coverage**: The stubbed-bridge test mechanism described doesn't match
  how the codebase actually tests bridges
  **Location**: Phase 1: Tests / Testing Strategy
  The plan says PATH-stubbing integration scripts matches "how
  `test-work-item-scripts.sh` already isolates `work-item-create-remote.sh`" —
  but that suite doesn't test the create bridge at all; the bridge is tested in a
  separate file via a mock HTTP server, and the bridge invokes integration
  scripts by absolute path, so PATH stubs would never be reached. The stated
  mechanism for Phases 1 and 6–8 cannot work as written.

- 🔴 **Test-Coverage / Code-Quality**: Resumability ordering lives in SKILL prose,
  so the kill-recovery AC cannot be exercised automatically
  **Location**: Phase 6: Tests (resumability) / Testing Strategy
  The per-item side-effect-then-baseline-then-global-timestamp ordering — the
  guarantee protecting a non-VCS-recoverable remote write — is SKILL prose the
  model executes. The proposed "simulate a kill after item A's `set`" test can
  only call the baseline-store primitives in isolation; a prose-level ordering
  regression (baseline written before the remote write) would pass every
  automated test.

#### Major

- 🟡 **Safety**: `--preview` no-write guarantee for the push direction rests on a
  non-existent bridge dry-run
  **Location**: Phase 6: Reconcile synced items / `--preview`
  `work-item-create-remote.sh --dry-run` only previews *create* fields; there is
  no update dry-run. If implemented by falling through to a real update,
  `--preview` would perform the very remote write it promises to suppress —
  unbacking the one sanctioned exception to the VCS-recovery convention.

- 🟡 **Correctness**: Per-item local `id` allocation in a loop will collide
  without batch allocation
  **Location**: Phase 8: Untracked remote pull (id allocation)
  `work-item-next-number.sh` returns "highest existing + 1" by scanning the dir.
  The plan builds items in memory before a single Write but never states each
  pulled file is written before the next `id` is allocated; allocating a batch
  before files land yields duplicate `id`s, corrupting the `id`-keyed baseline.

- 🟡 **Correctness**: Sync-side behaviour on a failed per-item remote read is
  unspecified
  **Location**: Phase 4 / Phase 6
  Graceful degradation is defined for `/list-work-items` but not for sync. A
  failed `show` treated as "remote unchanged" would push and silently clobber a
  remote that may be ahead; treated as "changed" makes everything a spurious
  conflict. The indeterminate-remote case must skip the item, writing neither
  side.

- 🟡 **Safety**: Terminal (post-send) push failures mid-batch can duplicate or
  orphan remote state
  **Location**: Phase 6 (resumable persistence) / Phase 8
  `work-item-push-decide.sh` retry/terminal handling is mentioned only for the
  Phase 8 unsynced batch; the synced-push (Phase 6) and override-push (Phase 7)
  paths don't state that 71/terminal codes are never auto-retried, risking a
  re-applied update or a duplicated issue on re-run.

- 🟡 **Safety**: Untracked-pull and per-item reads lack a bound on blast radius
  **Location**: Phase 8 / Performance Considerations
  Individual reads are timeout-bounded, but nothing caps the *count* of items
  fetched/created. A mis-scoped `--all` or automation-flooded project floods
  `meta/work/` with generated files and exhausts IDs in one pass.

- 🟡 **Architecture**: Two-write pull (local file + `external_id`) lacks a defined
  atomic boundary
  **Location**: Phase 8 / Phase 6 resumability ordering
  A crash between "file created" and "`external_id` written" leaves an orphan
  item indistinguishable from a never-pushed one, which a re-run re-pulls
  (duplicate) or offers as an unsynced push. Build full frontmatter (incl.
  `external_id`) in memory and write once.

- 🟡 **Correctness**: Fresh-checkout safety relies on an unguaranteed mtime
  ordering
  **Location**: Decision #2 / Phase 4 local-side pre-filter
  "On a fresh checkout every mtime is newer than the committed timestamp" isn't
  guaranteed (clock skew, mtime-preserving tooling, cross-machine clocks). A file
  mtime ≤ baseline timestamp short-circuits to "unchanged" and a genuinely
  different local file is never hashed → local change silently dropped.

- 🟡 **Performance**: Per-item `show` causes N round-trips even when the
  pre-filter says unchanged
  **Location**: Phase 5 / Performance Considerations
  The pre-filter suppresses the body comparison but not the round-trip that
  supplies `remote.updated`. Steady-state `/list-work-items` makes N serial
  network calls — exactly the case claimed to "stay cheap".

- 🟡 **Performance**: The search response already carries enough to avoid most
  per-item `show`s
  **Location**: Phase 4 / Phase 1
  The mandated `--fields updated,summary,description` means one paginated search
  serves both the `updated` pre-filter and the body comparison; per-item `show`
  should be a rarely-taken fallback, not the default.

- 🟡 **Usability**: Conflict prompt's remote-default destroys local on a bare
  Enter; wording is deferred to prose
  **Location**: Phase 7: Conflict prompt
  The polarity is inverted versus the safe-default `[y/N]` used elsewhere in the
  suite; a reflexive Enter discards local edits. Specify the exact string with
  the data-losing consequence inline.

- 🟡 **Usability**: Section-diff orientation/labelling is unspecified
  **Location**: Phase 7: Section-splitter + diff
  With remote-default-accept, the user must know which side survives; a bare
  `diff a b` with +/- markers is ambiguous. Head each section with LOCAL/REMOTE
  and fix the diff direction.

- 🟡 **Usability**: Unconfigured-integration error lacks message text / how-to-fix
  **Location**: Phase 6: Config gate
  The live repo has no `work:` section, so this is the common first-run path. The
  message must name `work.integration`, its valid values, and the concrete step
  to set it.

- 🟡 **Code-Quality**: Decision logic in SKILL prose has no `work-item-sync-decide.sh`
  analogue to `work-item-push-decide.sh`
  **Location**: Phases 6–8 orchestration
  Extract the (mode, classified-state, decision) → action mapping into a pure,
  unit-tested decision script so the full matrix — including forbidden-write cells
  for directional modes and conflict-skip cells — is asserted in CI.

- 🟡 **Code-Quality**: `_win_sha256` inlining is unjustified — an in-skill shared
  home exists
  **Location**: Phase 2: Normalisation + hashing
  `skills/work/scripts/work-item-common.sh` sits in the same directory and is the
  documented shared home; the cross-skill-`source` objection doesn't apply. Add a
  stdin-reading helper there rather than maintaining a third copy of the
  `sha256sum || shasum` idiom.

- 🟡 **Test-Coverage**: The push-update-to-existing-remote path has no script and
  no tests
  **Location**: Phase 6 / Testing Strategy
  The real remote-write half of bidirectional sync would ship with nothing
  asserting it routes to the update API, passes the right body, or maps
  update-flow exit codes into the push-decide taxonomy.

- 🟡 **Test-Coverage**: Graceful-degradation / timeout paths are manual-only
  **Location**: Phase 5 / Testing Strategy
  A regression that lets a remote failure abort the listing or block on a hung
  read (the explicit AC anti-requirement) wouldn't be caught. Give the fetch
  bridge a deterministic "remote-unavailable" signal and unit-test that
  `work-item-sync-classify.sh` falls back to presence-only on it.

- 🟡 **Test-Coverage**: The (mode, state) action matrix is verified only by manual
  steps
  **Location**: Phases 6/7/8 / Manual Testing Steps
  Forbidden-write enforcement for `--push-only`/`--pull-only` and
  remote-default-on-conflict are the core correctness surface but are checked only
  by a human running seven steps. (Resolved by the `sync-decide.sh` extraction.)

- 🟡 **Portability**: The engine compares local mtime against an ISO-8601 string
  with no portable read specified
  **Location**: Phase 4: local-side pre-filter
  The repo idiom (`stat -f '%m' || stat -c '%Y'`) yields epoch *seconds* — a
  GNU/BSD-divergent invocation *and* a unit/format mismatch versus the ISO
  `timestamp`. Reconcile units (store epoch, or convert mtime to ISO) and use the
  dual-`stat` fallback.

#### Minor

- 🔵 **Architecture**: Read bridge invents a parallel 70/71/72/73 taxonomy instead
  of sharing one dispatcher namespace — the 70/71 retryable-vs-terminal split is
  mutation-specific and largely meaningless for reads (Phase 1).
- 🔵 **Architecture**: Normalisation ignored-key set is a hardcoded list, not
  derived from the work-item schema — a drift hazard like the byte-pinned copies
  (Phase 2 / Decision #3).
- 🔵 **Architecture / Safety**: Committing a per-machine baseline couples team
  members through a shared mutable artifact; the safety argument covers only the
  local side (Decision #2).
- 🔵 **Architecture**: Per-item `show` reads don't scale with corpus size on a
  cold (no-baseline) run (Phase 5/6).
- 🔵 **Correctness**: Ignoring "any field absent from the local schema" is
  asymmetric between local and remote normalisation; define how a remote body maps
  to the comparable form (Phase 2).
- 🔵 **Correctness**: `IGNORE_KEYS` may be missing `revision` (research Open
  Question 2); if tooling stamps it on save, a bare re-save misclassifies as a
  local change (Phase 2).
- 🔵 **Correctness**: `--preview` must suppress the global `set-timestamp`, not
  only per-item baseline writes, or it poisons the next real run's pre-filter
  (Phase 6).
- 🔵 **Safety**: Pull-overwrite local write should explicitly route through
  `atomic_write`, and `external_id` writeback should persist before the baseline
  entry (Phase 6/8).
- 🔵 **Safety**: A committed shared baseline can mask divergence after
  merges/rebases (Decision #2).
- 🔵 **Performance**: Serial per-item reads multiply timeout cost (N ×
  per-item-timeout) on a slow-but-reachable remote (Phase 5).
- 🔵 **Performance**: The mtime pre-filter mis-fires on fresh checkout / branch
  switch, forcing a full normalise+hash of the whole corpus on the first run
  (Decision #2/#3).
- 🔵 **Usability**: Summary and override-log output formats are unspecified; the
  override log is the only audit trail for irreversible push-local decisions
  (Phase 6/7).
- 🔵 **Usability**: The batch accept-all/decline-all grammar is cited by reference
  to two precedents that use *different* grammars (numbered menu vs y/n); pin one
  (Phase 8).
- 🔵 **Usability**: Bidirectional is the unnamed default; no symmetric
  `--bidirectional` and no `--push-only --preview` composition example (Phase 6).
- 🔵 **Usability**: No incremental progress feedback during long per-item reads —
  a long silent pause reads as a hang (Phase 5/6).
- 🔵 **Code-Quality**: The normaliser's three entry points (`--hash` vs
  `--hash-stdin`) differ in whether normalisation runs — a readability/footgun
  trap; rename or collapse (Phase 2).
- 🔵 **Code-Quality**: The Phase 1 bridge sketch uses bare top-level `case`/`return`
  rather than the create bridge's `_wicr_main` + `BASH_SOURCE` guard skeleton
  (Phase 1).
- 🔵 **Code-Quality**: Read-bridge 70/71 carries write-shaped semantics into a
  read; document or narrow the taxonomy (Phase 1/6).
- 🔵 **Code-Quality**: Note that any future reversal of the committed-baseline
  decision should collapse the byte-pinned duplication rather than re-pin a third
  copy (Phase 3 / Migration Notes).
- 🔵 **Test-Coverage**: The engine state-table test omits the 5th (presence-only,
  baseline-present-but-no-`external_id`) branch and a realistic lexicographic
  ISO-8601 comparison edge (Phase 4).
- 🔵 **Test-Coverage**: Baseline "atomicity" is asserted as idempotency, not
  crash-safety; verify no partial temp survives and output always parses (Phase 3).
- 🔵 **Portability**: `_win_sha256` re-implements rather than reuses the audited
  portable helper (Phase 2).
- 🔵 **Portability**: `diff` output differs across GNU/BSD; restrict to `diff -u`,
  determine byte-equality via the normaliser+hash, assert on a stable subset
  (Phase 7).
- 🔵 **Portability**: Cross-machine determinism depends on byte-stable remote ADF;
  canonicalise the remote payload (e.g. `jq -S`) before hashing (Decision #2 /
  Phase 1).
- 🔵 **Portability**: Per-line whitespace trim is locale-sensitive under BSD
  awk/sed; force `LANG=C`/`LC_ALL=C` for the normalisation pass (Phase 2/4).

### Strengths

- ✅ Single source of truth for state derivation: `work-item-sync-classify.sh` is
  the one engine both skills call, extending the existing one-script
  classification pattern rather than duplicating it.
- ✅ Strong functional-core / imperative-shell split: Phases 1–4 are pure,
  TDD-first, dependency-stubbable scripts with no skill wiring.
- ✅ Bridge-mediated dispatch keeps gate and route from diverging; the caller
  passes the config-resolved `--integration`.
- ✅ The two-stage pre-filter invariant ("may only short-circuit to *unchanged*")
  is correctly specified so the cheap and authoritative checks cannot disagree.
- ✅ Resumability ordering follows the house side-effect-then-commit-ledger-last
  pattern (`run-migrations.sh`); re-runs are idempotent by design.
- ✅ Graceful degradation is designed into `/list-work-items`: timeout-bounded
  per-item reads, presence-only fallback, exit 0, no retry storm.
- ✅ `--preview` is a well-justified, correctly-scoped exception to the
  VCS-recovery convention (remote writes escape VCS).
- ✅ Lexicographic ISO-8601 comparison avoids fragile cross-platform date parsing;
  the bash-3.2 floor is pinned with a per-phase `lint-bashisms.sh` check.
- ✅ Phases are genuinely independently-mergeable and each leaves `mise run` green.
- ✅ Five-state label distinctness is concretely operationalised by extending the
  existing pairwise glyph-AND-text and no-ANSI tests.

### Recommended Changes

1. **Specify a write/update bridge and route the push path through it**
   (addresses: push-path-undefined, `--preview`-dry-run-unbacked,
   push-update-no-tests). Add `work-item-update-remote.sh` (or an `update`
   subcommand) mirroring the create bridge — dispatching to
   `jira-update-flow.sh` / `linear-update-flow.sh`, mapping their exit codes into
   the shared retryable/terminal taxonomy, and exposing a real dry-run
   (`--print-payload`). Have Phases 6 and 7 depend on it; add mock-server tests
   for success/retryable/terminal and a `--preview` test asserting zero remote
   writes.

2. **Give the remote side a real baseline** (addresses:
   remote-baseline-uncomputable, normalisation-asymmetry). Either add a
   `remote_hash` (digest of normalised remote content at last sync) to each
   baseline entry, or redefine the remote side as "unchanged iff `remote_updated_at`
   matches" with the body fetch used only to refresh content on pull. Update the
   2×2 table and the "baseline-equivalent" wording, and define precisely how a
   remote payload is projected into the comparable normalised form.

3. **Make remote-ahead pulls visible and confirm-safe** (addresses:
   pull-overwrite-silent, atomic-overwrite, failed-read-during-sync). Emit a
   per-item summary line for every pull-overwrite (`id` + "local replaced from
   remote"), route the overwrite through `atomic_write`, and specify that an
   indeterminate (failed/timed-out) remote read during sync skips the item
   (neither side written) rather than assuming unchanged/changed.

4. **Collapse the N+1 to a bulk read** (addresses: per-item-show-N-round-trips,
   search-carries-enough, serial-timeout-multiplier). Source `remote.updated`
   (and, where fidelity-equivalent, the body) from one bulk `search --fields
   updated,summary,description` keyed by `external_id`; reserve per-item `show`
   for the genuinely-changed minority. Confirm search-`description` matches
   `show` for the normalised comparison.

5. **Extract the decision matrix into a tested script** (addresses:
   prose-resident-decision-logic, mode-state-matrix-manual-only,
   resumability-untestable). Add `work-item-sync-decide.sh` mirroring
   `work-item-push-decide.sh`: inputs (mode, classified state, user decision) →
   action (push/pull/skip-conflict/prompt), with the full (mode × state) matrix
   unit-tested including forbidden-write and conflict-skip cells. Where possible,
   move the per-item commit sequence into a thin, fault-injectable apply helper so
   the kill-recovery AC can be tested in CI; otherwise state explicitly it is
   manual-only.

6. **Fix the bridge test mechanism** (addresses: PATH-stub-false-claim). Rebase
   Phase 1/6–8 tests on the existing mock-HTTP-server harness
   (`test-work-item-create-remote.sh` / `mock-jira-server.py`), not PATH stubs,
   since bridges invoke integration scripts by absolute path. Assert `--fields`
   injection and exit taxonomy against captured requests.

7. **Bound the blast radius and harden id allocation** (addresses:
   untracked-pull-unbounded, id-allocation-collision). Add a count-and-confirm
   gate when the untracked-pull set exceeds a threshold, and allocate the whole
   pull batch up front with `work-item-next-number.sh --count N` (or guarantee
   each file is written before the next allocation).

8. **Specify the portable mtime read and normalisation locale** (addresses:
   mtime-portability, fresh-checkout-mtime, locale-trim, ADF-determinism). Use the
   dual-`stat` fallback with reconciled units, force `LANG=C`/`LC_ALL=C` for the
   normalisation pass, canonicalise remote ADF (`jq -S`) before hashing, and treat
   the mtime gate as advisory (always fall through to the hash on any uncertainty).

9. **Pin the user-facing wording and outputs** (addresses:
   conflict-prompt-wording, diff-orientation, unconfigured-error,
   summary/override-log, batch-grammar). Specify the conflict prompt string with
   the data-losing consequence inline and the inversion made loud; head section
   diffs with LOCAL/REMOTE; give the unconfigured error a what/why/how-to-fix
   message naming `work.integration`; pin the summary (per-item, not just counts)
   and override-log formats; and reproduce one concrete batch-push grammar.

10. **Lower-priority cleanups** (addresses: remaining minors). Source the sha256
    idiom from `work-item-common.sh`; reproduce the create-bridge skeleton in the
    fetch bridge and reconcile the read-side exit taxonomy; add `revision` to
    `IGNORE_KEYS` if tooling stamps it; ensure `--preview` suppresses
    `set-timestamp`; add the presence-only and lexicographic-edge engine test
    cells; and note the byte-pinned-copy maintenance trap in Migration Notes.

## Per-Lens Results

### Architecture

**Summary**: Architecturally disciplined in its big moves (single shared
classification engine, bridge-mediated I/O, functional-core/imperative-shell
split, independently-mergeable phases). The most significant structural gap is
the push-existing-item path, which routes through a create-only bridge with no
update dispatch to the existing `jira-update-flow.sh`/`linear-update-flow.sh`
primitives. Secondary: asymmetric read/create bridge taxonomy, an under-specified
two-write pull boundary, and a remote-side equivalence comparison with no
persisted referent.

**Strengths**: single source of truth for state derivation; functional-core /
imperative-shell separation; bridge dispatch keeps gate and route aligned;
independently-mergeable, releasable phases; designed-in graceful degradation;
house resumability pattern with the short-circuit-only pre-filter invariant.

**Findings**:
- 🟡 (high) Push-of-existing-item path has no write/update bridge — breaks the
  bridge architecture (Phase 6/8). Routes through a create-only bridge; update
  primitives exist but are unwired; either reaches past the bridge boundary or
  conflates update with create.
- 🟡 (medium) Remote-side equivalence compares against an ill-defined
  "baseline-equivalent" (Phase 4) — no remote content/hash is persisted.
- 🟡 (medium) Two-write pull (local file + `external_id`) lacks a defined atomic
  boundary (Phase 8/6) — a crash mid-pair leaves an orphan re-pulled on re-run.
- 🔵 (high) Read bridge invents a parallel exit taxonomy instead of reusing the
  dispatcher namespace (Phase 1).
- 🔵 (medium) Normalisation ignored-key set is a hardcoded list, not derived from
  the schema (Phase 2 / Decision #3).
- 🔵 (medium) Committing a per-machine baseline couples team members through a
  shared mutable artifact; safety analysed only for the local side (Decision #2).
- 🔵 (low) Per-item `show()` reads do not scale with corpus size on a cold run
  (Phase 5/6).

### Correctness

**Summary**: Unusually rigorous about the change-detection contract and
resumability ordering, and the two-stage pre-filter invariant is sound. But the
remote-side comparison is logically under-determined (no persisted remote
baseline), the push path has no write dispatch, and several state-transition /
idempotency details (fresh-checkout mtime, batch id allocation, failed-read
verdict, `--preview` timestamp suppression) need tightening.

**Strengths**: correct short-circuit-only pre-filter invariant; internally
consistent resumability ordering; correct lexicographic ISO-8601 comparison;
fully-grounded local-side detection (local_hash is persisted).

**Findings**:
- 🔴 (high) Remote-side "baseline-equivalent" comparison has no stored baseline
  (Phase 4).
- 🔴 (high) Push (local-ahead) path is undefined — create bridge only creates
  (Phase 1/6).
- 🟡 (medium) Fresh-checkout safety relies on an unguaranteed mtime ordering
  (Decision #2/Phase 4).
- 🟡 (high) Per-item id allocation in a loop will collide without batch allocation
  (Phase 8).
- 🟡 (medium) Sync-side behaviour on a failed per-item remote read is unspecified
  (Phase 4/6).
- 🔵 (medium) Ignoring "absent from local schema" is asymmetric between local and
  remote normalisation (Phase 2).
- 🔵 (high) Inconsistent ignored-field set — `revision` may be auto-stamped (Phase 2).
- 🔵 (medium) `--preview` must also suppress the global `set-timestamp` (Phase 6).

### Safety

**Summary**: Handles the central concern (irreversible remote writes,
non-VCS-recoverable local overwrites) with genuine care — `--preview` is well
justified, the resumability ordering is sound, local writes route through
`atomic_write`. The most serious gaps: pull-overwrite silently destroys local
edits with no diff/confirm in directional/default mode, and the `--preview`
no-write guarantee for push rests on a non-existent update dry-run.

**Strengths**: well-justified `--preview` exception; crash-safe resumability
ordering; `atomic_write` for all local/baseline mutations; fail-safe conflict
resolution (no write without confirm; directional report-and-skip); graceful
degradation; short-circuit-only pre-filter.

**Findings**:
- 🔴 (high) Pull-overwrite silently destroys local edits in default/`--pull-only`
  with no diff or confirmation (Phase 6).
- 🟡 (high) `--preview` no-write guarantee for the push direction rests on a
  non-existent bridge dry-run (Phase 6).
- 🟡 (medium) Terminal (post-send) push failures mid-batch can duplicate/orphan
  remote state (Phase 6/8).
- 🟡 (medium) Untracked-pull and per-item reads lack a bound on blast radius
  (Phase 8 / Performance Considerations).
- 🔵 (medium) A committed shared baseline can mask divergence after merges/rebases
  (Decision #2).
- 🔵 (high) `external_id` writeback / pull-overwrite should be confirmed atomic
  and correctly ordered (Phase 8/6).

### Test Coverage

**Summary**: Strong foundation-layer strategy (Phases 1–4 pure scripts,
TDD-first, real equivalence classes), well-matched to the contract's risk. But
the central testability claim is factually wrong (PATH-stub mechanism doesn't
match the mock-server reality), and the most safety-critical behaviour
(resumability/kill recovery, conflict write ordering) lives in SKILL prose that
the proposed script tests cannot exercise; the plan leans on manual verification
for the riskiest paths.

**Strengths**: pure TDD-first foundation tests over meaningful equivalence
classes; concrete five-state distinctness assertions; contract-direct
normalisation tests; fast, unit-heavy, independently-mergeable phases.

**Findings**:
- 🔴 (high) Stubbed-bridge-on-PATH mechanism doesn't match how the codebase tests
  bridges (mock HTTP server + absolute-path invocation) (Phase 1).
- 🔴 (high) Resumability ordering lives in SKILL prose, so the kill-recovery AC
  cannot be exercised automatically (Phase 6).
- 🟡 (high) Push-update-to-existing-remote path has no script and no tests (Phase 6).
- 🟡 (high) Graceful-degradation / timeout paths are asserted only manually (Phase 5).
- 🟡 (medium) The (mode, state) action matrix is verified only by manual steps
  (Phases 6/7/8).
- 🔵 (medium) Engine state-table test omits the presence-only branch and a
  lexicographic ISO edge (Phase 4).
- 🔵 (medium) Baseline "atomicity" is asserted as idempotency, not crash-safety
  (Phase 3).

### Code Quality

**Summary**: Well-structured for maintainability (pure unit-tested foundation,
single shared engine, close mirroring of the create bridge). Main risks: the
deliberate `_win_sha256` inlining when an in-skill shared home exists, a
normaliser with three overlapping entry points, and meaningful orchestration logic
(mode tables, conflict/push branches, resumability ordering) living in SKILL prose
rather than a testable decision script.

**Strengths**: strong separation of concerns; pure dependency-stubbable Phases
1–4; symmetric bridge mirroring; explicit resumability ordering pinned to a house
pattern; small reviewable phases.

**Findings**:
- 🟡 (high) `_win_sha256` inlining is unjustified — `work-item-common.sh` is the
  in-skill shared home (Phase 2).
- 🟡 (medium) Orchestration decision logic in SKILL prose with no
  `work-item-sync-decide.sh` analogue to `work-item-push-decide.sh` (Phases 6–8).
- 🔵 (high) Normaliser's three entry points (`--hash` vs `--hash-stdin`) differ in
  whether normalisation runs — readability/footgun trap (Phase 2).
- 🔵 (medium) Phase 1 bridge sketch uses bare top-level `case`/`return` rather than
  the create-bridge `_wicr_main` + `BASH_SOURCE` skeleton (Phase 1).
- 🔵 (medium) Read-bridge 70/71 carries write-shaped semantics into a read (Phase 1/6).
- 🔵 (low) Byte-pinned-copy constraints are a maintenance hazard if the
  committed-baseline decision is ever reversed (Phase 3 / Migration Notes).

### Usability

**Summary**: Strong on consistency — mirrors established prompt shapes, routes I/O
through bridges, single state-derivation engine. The main DX risks concentrate in
the conflict-resolution prompt (remote-default-accept is data-destructive yet
wording, diff orientation, and Enter-mapping are unspecified) and in several
deferred output formats (override-log, summary, unconfigured error, batch grammar).

**Strengths**: reuses recognisable prompt shapes; discoverable mutually-exclusive
modes with `--preview` composition; safety-improving `--preview`; well-specified
graceful degradation; readable section diff; consistent five-state mental model
across the two surfaces.

**Findings**:
- 🟡 (high) Conflict prompt's remote-default destroys local on a bare Enter;
  wording deferred to prose (Phase 7).
- 🟡 (medium) Section-diff orientation/labelling unspecified (Phase 7).
- 🟡 (medium) Unconfigured-integration error lacks message text and how-to-fix
  (Phase 6).
- 🔵 (medium) Summary and override-log output formats unspecified (Phase 6/7).
- 🔵 (medium) Batch fast-path grammar cited from two precedents with different
  grammars (Phase 8).
- 🔵 (medium) Bidirectional default discoverability gap; no composition example
  (Phase 6).
- 🔵 (low) No incremental progress feedback during long per-item reads (Phase 5/6).

### Portability

**Summary**: Inherits solid portable primitives (dual sha256 idiom, bash-3.2-safe
ISO8601, `atomic_write`) and commits to the bash-3.2 floor, lexicographic ISO
comparison, and a normalised-hash baseline. But the engine introduces a new,
unspecified dependency on reading local mtime and comparing it to an ISO string —
a `stat` GNU/BSD gap and unit mismatch. Secondary: the inlined sha256
re-implementation, `diff` output divergence, ADF byte-stability, and locale.

**Strengths**: normalised-content basis for cross-machine determinism with the
fresh-checkout fall-through reasoning; lexicographic ISO comparison side-steps
date-parse divergence; bash-3.2 floor pinned with a per-phase check; explicit
awareness of the `sha256sum`-is-GNU-only trap.

**Findings**:
- 🔴 (high) The engine compares local mtime against an ISO string with no portable
  read specified — `stat -f`/`stat -c` divergence + epoch-vs-ISO unit mismatch
  (Phase 4).
- 🔵 (high) `_win_sha256` re-implements rather than reuses the audited portable
  helper (Phase 2).
- 🔵 (medium) `diff` output differs across GNU/BSD; restrict to `diff -u` and
  determine byte-equality via the normaliser (Phase 7).
- 🔵 (medium) Cross-machine determinism depends on byte-stable remote ADF;
  canonicalise (`jq -S`) before hashing (Decision #2 / Phase 1).
- 🔵 (low) Locale-sensitive awk/sed trim; force `LANG=C`/`LC_ALL=C` for the
  normalisation pass (Phase 2/4).

### Performance

**Summary**: Correctly identifies the per-item remote read as the dominant cost
and installs a two-stage pre-filter with per-item timeout-bounding. But the
pre-filter only suppresses the body fetch, not the round-trip: on the list path
`remote.updated` comes *from* a per-item `show`, so the design pays N round-trips
even when nothing changed — a classic N+1 a single bulk `search --fields updated`
could collapse. The local normalise+hash is cheap but runs on every list
invocation and is gated only by a fragile mtime check.

**Strengths**: right two-stage structure; timeout-bounded reads; lexicographic
(no-reparse) timestamp comparison; safe-to-take short-circuit; single shared
normaliser.

**Findings**:
- 🔴 (high) Per-item `show` causes N round-trips even when the pre-filter says
  unchanged (Phase 5 / Performance Considerations).
- 🟡 (medium) The search response already carries enough to avoid most per-item
  `show`s (Phase 4 / Phase 1).
- 🔵 (medium) Serial per-item reads multiply timeout cost on a slow remote (Phase 5).
- 🔵 (medium) The mtime pre-filter mis-fires on committed-baseline / fresh checkout,
  forcing a full normalise+hash of the whole corpus (Decision #2/#3).

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-18T15:46:37+00:00

**Verdict:** REVISE

The revision **resolved all five critical findings** and the large majority of
the majors. The remote side now has a real `remote_hash` baseline; the push path
routes through a new `work-item-update-remote.sh` write bridge wrapping the
verified whole-item update flows; pull-overwrite uses `atomic_write` and emits a
visible summary line; the bridge tests are rebased on the real mock-HTTP-server
harness; and the resumability ordering plus the (mode×state) matrix are extracted
into fault-injectable/CI-testable scripts. The agents independently verified these
claims against the codebase.

However, the re-review surfaced a **new cluster of majors introduced by the N+1
fix**: the "one bulk `search` scoped to the tracked keys" mechanism is not
actually supported by `jira-search-flow.sh` — it has no `key in (...)` primitive
and returns a single page capped at 100 with no auto-pagination. Four lenses
(architecture, correctness, performance, test-coverage) caught this from different
angles. Two further regressions came in with the portability and usability fixes.
The verdict stays **REVISE** for one more focused iteration on the bulk-read
mechanism; the plan is otherwise close.

### Previously Identified Issues

- 🔴 **Correctness/Architecture/Safety/Test**: Push/update path undefined —
  **Resolved**. New `work-item-update-remote.sh` write bridge wraps the verified
  whole-item `jira-update-flow.sh`/`linear-update-flow.sh` (+`--print-payload`).
- 🔴 **Correctness/Architecture**: Remote side had no stored baseline —
  **Resolved**. `remote_hash` added to the schema and the engine contract;
  correctness agent confirms the remote-changed verdict is now authoritative.
- 🔴 **Safety**: Pull-overwrite silently destroys local edits — **Partially
  resolved**. Now `atomic_write` + a visible per-item summary line; but safety
  re-flags that the VCS-recovery model does not cover *uncommitted* local edits,
  and there is no aggregate gate on the *number* of files a pull overwrites.
- 🔴 **Test-Coverage**: PATH-stub test mechanism was wrong — **Resolved**. Rebased
  on `mock-jira-server.py`; the agent verified the harness and the claim now holds.
- 🔴 **Test-Coverage/Code-Quality**: Resumability untestable in prose —
  **Resolved**. `work-item-sync-apply.sh` fault-injection hook makes kill-recovery
  a CI assertion.
- 🟡 **Performance/Architecture**: Per-item `show` N+1 — **Resolved for steady
  state** (bulk pre-filter), but the bulk-read mechanism is under-specified (see
  New Issues).
- 🟡 **Code-Quality**: Decision logic in SKILL prose — **Resolved** via
  `work-item-sync-decide.sh` / `work-item-sync-apply.sh`.
- 🟡 **Portability**: mtime read non-portable — **Partially resolved**. The
  `stat` idiom now matches the repo and the gate is advisory; but the new
  epoch→ISO `date` idiom is novel/unverified and drops the repo-standard
  `LC_ALL=C` (see New Issues).
- 🔵 **Usability** (prompt wording, error, diff orientation, override-log,
  summary), **Portability** (LANG=C, diff -u, jq -S), **Correctness** (`--preview`
  set-timestamp, id `--count`), **Code-Quality** (sha256 home, normaliser
  collapse) — **Resolved** as specified.

### New Issues Introduced

- 🟡 **Architecture/Correctness/Performance/Test (×4)**: The bulk `search` cannot
  be scoped to the tracked keys and is single-page (max 100, no auto-pagination).
  `jira-jql.sh` has no `key in (...)` clause; `jira-search-flow.sh` caps `--limit`
  at 100 and only surfaces `nextPageToken`. So "one round-trip scoped to tracked
  keys" over-fetches the whole project and silently truncates corpora >100 items.
- 🟡 **Correctness**: No engine branch for *tracked-but-remote-absent* — a
  successful fetch where the item's key is missing (deleted / out-of-scope /
  paginated-out) falls through unspecified (risking a push to a non-existent issue
  or a null-lookup crash); distinct from the indeterminate/failed-read branch.
- 🟡 **Portability**: The epoch→ISO `date -u -r || date -u -d @` idiom has no repo
  precedent (the repo uses GNU-first `date -d` / BSD `date -j -f`) and omits the
  repo-mandatory `LC_ALL=C`. Since the gate is advisory, simplest fix is to compare
  raw epoch integers and drop the date formatting entirely.
- 🟡 **Safety**: Pull-overwrite is unrecoverable for *uncommitted* local edits, and
  the blast-radius gate covers only untracked *creates*, not the count of local
  files a single pull *overwrites*.
- 🟡 **Usability**: Two opposite default polarities now coexist — `[Y/n]`
  (destructive default) for conflicts vs `[y/N]` (safe default) for batch push —
  within one command; consider a typed token for the destructive choice.
- 🟡 **Test-Coverage**: The reused `search-200.json`/`issue-200.json` fixtures
  carry no `updated` field — the very thing the bridge injection and remote
  pre-filter depend on; new `updated`-bearing fixtures are needed. Also no tests
  asserted for the blast-radius gate or the `--count N` batch-allocation
  uniqueness, and the Linear post-mutation terminal/double-apply path is
  under-specified.
- 🔵 **Code-Quality/Architecture**: The shared exit taxonomy is only softly
  committed ("or, at minimum, document…") — should be one sourced definition all
  three bridges reference. Plus `wic_` vs the file's established `wip_` prefix.
- 🔵 **Correctness**: The terminal-71-never-retry rationale ("a resent PUT could
  double-apply") is wrong for an idempotent whole-item update — harmless (errs
  safe) but the justification should be corrected to "response-uncertainty".
- 🔵 **Correctness/Safety**: Pre-filter hash-skip ambiguity (does "candidate-
  unchanged" still hash?) creates a concurrent-edit TOCTOU window; and the pull
  branch must hash `local_hash` from the *post-overwrite* file.

### Assessment

The plan is materially stronger and the dangerous issues are gone. It needs one
more focused pass — predominantly on the bulk-read mechanism (key-scoped JQL +
pagination + a tracked-but-remote-absent branch), which is the root of four of the
new majors — plus the small portability (epoch comparison) and safety
(uncommitted-edit overwrite) refinements. None of the remaining items are
structural; they are specification gaps in the new bulk path and a few rationale
corrections.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 4, final) — 2026-06-18T19:20:00+00:00

**Verdict:** COMMENT — implementation-ready, with two recommended pre-implementation fixes.

The plan has converged. Across four passes: 5 criticals → 0 → 1 → **0**. This pass
found **no critical findings**; the performance lens returned **zero findings**;
and most lenses now lead with "no blocking findings — acceptable to conclude." The
agents verified every Pass-3 fix against the actual workspace code (the
`--keys`+`--all-projects` JQL composition, the single sourced taxonomy retrofit,
the mock-server capture flags, the dual-`stat` idiom, the search 100-cap /
no-auto-pagination).

Two **major** findings remain — both narrow, both converged-on by multiple lenses,
both cheap to fix. Neither is structural; the plan is sound to implement once they
are addressed.

### Previously Identified Issues (Pass-3 fixes — all verified resolved)

- 🔴→✅ **jj/git dirty guard** — the Pass-3 CRITICAL is resolved *in principle*
  (a VCS-mode-aware `work-item-file-dirty.sh` now exists for both VCSs); the
  remaining wrinkle is its mode-resolution detail (below).
- 🟡→✅ `--keys` + `--all-projects` (verified no project clause injected),
  cap-overflow → indeterminate, remote-side asymmetry / single body provenance,
  single sourced exit taxonomy (both copies retrofitted), conflict `skip` token +
  no-Enter-default, blast-radius gates pinned + fail-safe, fixtures + capture
  wiring, `revision` unconditional, non-numeric `stat` guard, jq floor.
- ✅ **Performance**: N+1 resolved at realistic scale — `ceil(tracked/50)` chunked
  calls, `--limit 100` avoids the speculative page-2 probe, trusted
  `updated`-equality short-circuit means unchanged items do zero per-item reads.
  **No findings.**

### Remaining Issues (recommended before implementation)

- 🟡 **Safety / Architecture / Portability** (one root cause, flagged by 3 lenses):
  the dirty guard's **VCS-mode resolution** is wrong as written. It cites
  `scripts/vcs-common.sh`, which exposes no git/jj mode accessor (only
  `find_repo_root` + `classify_checkout`, a topology classifier). The repo's real
  idiom is **`.jj`-present-wins** (`vcs-status.sh`, `vcs-detect.sh`,
  `run-migrations.sh`). In a **jj-colocated** checkout (this repo's normal state),
  a `classify_checkout`-based dispatch could route to the git arm, where
  `git status --porcelain` reads the git index — which **lags the jj working-copy
  commit** — so a file with live uncommitted jj edits reads as clean and is
  silently overwritten. Fix: resolve mode as `find_repo_root` then `[ -d
  "$ROOT/.jj" ]` ⇒ jj (colocated included), else git; use `jj --no-pager diff
  --name-only` (the precedent flag, avoids a pager hang in captured contexts);
  decide the git untracked-`^??` policy; and fail-safe-to-dirty when mode is
  indeterminate. (Ideally factor the one-liner into a shared `vcs-common.sh`
  helper so all four copies converge.)
- 🟡 **Test-Coverage / Code-Quality**: the conflict **typed-token → action
  mapping** (`remote`/`local`/`skip`, empty/unknown → re-prompt-once-then-skip) is
  the single most destructive branch (a misparse discards local edits) but is
  described as living "in or validated by a tiny pure helper" with **no test
  listed and no named file**. Fix: fold it into `work-item-sync-decide.sh`'s tested
  vocabulary (or a named helper) and assert the mapping — including the
  empty/unknown → non-destructive `skip` default.

### Minor / suggestion (fold in or defer to implementation)

- Conflict-prompt wording vs behaviour: the string says "pressing Enter re-asks"
  but the rule defaults to `skip` after one re-prompt — reconcile the two.
- Cross-class behaviour on a declined blast-radius gate (does it abort the whole
  run or only the overwrite class?) — state it, with an actionable next-step.
- Batch `--count N` near the id-pattern cap returns < N ids and exits non-zero —
  the pull loop should detect the short allocation and abort with a clear message.
- Precedence: remote-absent (key missing from a successful fetch) should be
  evaluated **before** the first-sync-on-dirty full-contract branch.
- Pull blast-radius count is the **clean** (post-dirty-routing) overwrite set.
- Re-point `EXIT_CODES.md` at the new `work-item-bridge-codes.sh` owner and list
  the fetch/update bridges.
- `git status --porcelain` untracked-`^??` policy; sha256-fallback parity test;
  prerequisite-check-fails-on-jq-without-`-S` test.

### Assessment

The plan is **implementation-ready**. Four passes drove it from 5 criticals to
none; the remaining two majors are a single localised dirty-guard correction (key
on `.jj` presence; use `jj --no-pager diff`) and one test addition for the
conflict token mapping. Recommend applying those two before `/implement-plan`; the
minors can be folded in or handled during implementation against the now-explicit
contracts.

---
*Re-review generated by /accelerator:review-plan*

## Approval — 2026-06-18T20:27:16+00:00

**Verdict:** APPROVE — by Toby Clemson.

The two Pass-4 majors are resolved in the plan: (1) the dirty-working-copy guard
now resolves VCS mode `.jj`-present-wins (jj-colocated → jj arm), uses `jj
--no-pager diff --name-only`, treats git untracked `^??` as dirty, and fails safe to
*dirty* on an indeterminate mode — closing the silent-overwrite-of-uncommitted-jj-
edits path; (2) the conflict typed-token interpretation is now a named, tested entry
point (`work-item-sync-decide.sh resolve-conflict-token`) with empty/unknown → `skip`
asserted, and the prompt wording reconciled.

Across five passes the plan went from 5 criticals to none. Remaining items are
minors backed by explicit contracts, suitable to handle during implementation. The
plan is **approved and marked ready** for `/accelerator:implement-plan`.

---
*Approval recorded by /accelerator:review-plan*

## Post-approval scope change — 2026-06-18T21:39:46+00:00

⚠️ **This APPROVE covered the Jira-first scope.** After approval, the plan was
materially extended to **incorporate Linear as a co-equal supported tracker**
(per-tracker read/update adapters behind the bridge boundary; a different Linear
bulk-read strategy — one team-wide auto-paginated search indexed by identifier,
since Linear has no key-set filter; Markdown-vs-ADF body projection; a required
`updatedAt` GraphQL field addition to the Linear flows; Linear exit-code mapping;
`LINEAR_INNER_GITIGNORE_RULES` parity; Linear mock-server test coverage). The plan
status has been reverted to **draft**.

The prior verdict does **not** cover these additions. A re-review focused on the
Linear adapter sections (Implementation Approach, Phase 1 `search --keys` two-adapter
contract + Linear GraphQL prerequisite, Phase 2 per-tracker projection, Decision
#10, the Phase 1 Linear tests) is recommended before re-approving and re-marking
ready.

---
*Note recorded by /accelerator:review-plan*

## Re-Review (Pass 6, focused delta: Linear + hashing additions) — 2026-06-19T00:07:11+00:00

**Verdict:** REVISE

A focused six-lens delta re-review (architecture, correctness, code-quality,
portability, test-coverage, performance) of only the two additions. The adapter
architecture and the consolidation *placement* are sound (engine/skills never
branch on tracker; `scripts/hash-common.sh` is correctly named/placed; the
launcher wrapper is a clean shim). But verification against the actual Linear code
found **one critical** and several majors — all concrete, code-grounded, and
cheaply fixable.

### Findings — Linear additions

- 🔴 **Critical (correctness; also flagged by architecture + performance)**:
  Linear's team-wide search **truncates at `MAX_PAGES=20` and still exits 0** with
  `truncated:true` in the body (`linear-graphql.sh`). The plan keys "absent from a
  *successful* fetch ⇒ remote-absent" off the exit code, so tracked items in the
  un-fetched tail (a board > ~1000 issues at the default page size of 50) would be
  silently misclassified as **remote-absent** — masking remote changes / treating
  live issues as deleted. **Fix**: the Linear adapter must inspect `truncated` and
  map an incomplete fetch to **indeterminate** (skip + needs-retry), never
  remote-absent; pass `--limit 250` to cut pages ~5×; add a truncated-fixture test.
- 🟡 **Major (correctness; also performance + test-coverage)**: `linear-search-flow.sh`
  selects only `id/identifier/title/state/assignee` — **not `description`**. The
  plan claims "Linear search returns the Markdown body, so `show` is essentially
  never needed," which is false against the code, and the GraphQL prerequisite adds
  only `updatedAt`. **Fix (recommended)**: correct the claim — Linear search yields
  `{updated}` only (add `updatedAt`); the **body comes from `show` for the changed
  minority**, exactly like Jira (avoids fetching every team issue's body). The
  `{updated, body}` contract's body half is populated by `show`, not search.
- 🔵 **Minor (test-coverage)**: assert the **merged distinct-identifier count**
  across the multi-page Linear fixture at the bridge boundary (symmetric to the
  Jira "N distinct external_ids" assertion) so a first-page-only indexing bug is
  caught.

### Findings — hashing consolidation

- 🟡 **Major (architecture; also code-quality)**: the "three duplicate `sha256_of`
  copies" premise is **factually wrong**. The launcher copy emits the full digest;
  the two design playwright copies (`run.sh`, `ensure-playwright.sh`) emit a
  **truncated 8-char** namespace key (`cut -c1-8`), and `run.sh` already sources
  launcher-helpers then *shadows* `sha256_of`. Treating them as drop-in wrappers
  over `hash_sha256_file` would change the cache-namespace hash width and bust
  existing playwright caches. **Fix**: scope the consolidation to the two genuinely
  identical full-digest copies (launcher + new work caller); the playwright copies,
  if consolidated, wrap as `hash_sha256_file "$1" | cut -c1-8` — or take the
  stated standalone-bootstrap exception. Drop the "three duplicates" framing.
- 🟡 **Major (test-coverage; also portability)**: the cross-backend parity test
  can't exercise both backends on one machine (the `sha256sum || shasum`
  short-circuit means each OS runs only one). **Fix**: make it a **golden-digest**
  assertion (hash of a fixed fixture == a hard-coded known SHA-256) **plus** a
  PATH-shadow/wrapper that forces the `shasum` fallback on a host that has
  `sha256sum`, so both branches run in one job.
- 🔵 **Minor (correctness; also portability)**: implement `hash-common.sh` with the
  existing **`command -v sha256sum` detect-and-branch** form (as `launcher-helpers.sh`
  does), not an exit-status `||` fallback — so the consolidation is behaviourally
  identical, not merely digest-equivalent.

### Assessment

The two additions are structurally right but rest on two incorrect assumptions
about the existing code (Linear search truncates-but-exits-0 and omits
`description`; the playwright `sha256_of` copies are truncating, not identical).
The critical (truncation → false remote-absent) is a genuine data-misclassification
path and must be fixed; the rest are small, well-specified corrections. None are
structural — a focused fix pass converges them.

---
*Re-review generated by /accelerator:review-plan*

## Approval after fixes (Pass 7) — 2026-06-19T00:21:42+00:00

**Verdict:** APPROVE — by Toby Clemson. All Pass-6 findings resolved and
**verified against the actual code**.

Code facts re-checked in the workspace before approving:
- `linear-graphql.sh:37` `MAX_PAGES=20`; `truncated=true` set on cap (`:385`) and
  stalled cursor (`:392`) with the flag surfaced in the result — the plan's
  "inspect `truncated`, not the exit code → indeterminate" fix is correctly based.
- `linear-search-flow.sh:159` selects `id identifier title state assignee` — **no
  `description`, no `updatedAt`** — confirming both the `show`-for-body correction
  and the `updatedAt` GraphQL prerequisite.
- `--limit` range `1..250` — `--limit 250` is valid.
- Playwright `run.sh:32` / `ensure-playwright.sh:53` use `cut -c1-8` for a cache
  namespace — confirming they are a *different, truncating* function, correctly
  excluded from the full-digest consolidation.
- Launcher `sha256_of` is full-digest via `command -v` detection
  (`launch-server.sh:142,157` callers) — matching the `hash-common.sh` sketch.

Resolution status:
- 🔴 Linear truncation → indeterminate (not remote-absent), `--limit 250`,
  truncated test fixture — **Resolved**.
- 🟡 Linear `description`/`show` claim corrected (body via `show`, like Jira) —
  **Resolved**.
- 🟡 sha256 "three duplicates" framing corrected (full-digest vs 8-char truncating);
  consolidation scoped to the full-digest copy — **Resolved**.
- 🟡 Parity test → golden-digest + forced-fallback branch in one run — **Resolved**.
- 🔵 `command -v` detect form; Linear merged distinct-count assertion — **Resolved**.

A stale-phrasing sweep confirmed no contradictory text survived ("show never
needed", "three duplicates", Linear `{updatedAt, body}` map all absent). The plan
(Jira + Linear, per-tracker adapters, consolidated hashing) is **approved and
marked ready** for `/accelerator:implement-plan`.

---
*Approval recorded by /accelerator:review-plan*

**Verdict:** REVISE

The agents verified that **every Pass-2 fix landed and holds** against the actual
codebase: the `search --keys` chunked+paginated mechanism is the right shape, the
engine input contract cleanly inverts the dependency (pure classifier over a
pre-fetched record), the epoch-seconds timestamp **fully resolves** the date
portability issue (portability lens now clean bar two suggestions), the single
sourced exit taxonomy is sound, and the test harness/fixtures/`--count` primitives
all exist as the plan assumes.

But Pass 3 surfaced **one new critical** introduced by the Pass-2 safety fix, plus
a tight cluster of majors mostly from the `--keys` fix. The trajectory is
converging (5 criticals → 0 → 1; each fix spawns a smaller, narrower cluster), but
the critical is load-bearing and — aptly — concerns jj.

### Previously Identified Issues (Pass-2 new-issue cluster)

- 🟡 **Bulk-read key-scoping + pagination** — **Resolved**. `search --keys`
  (chunked `key in (…)` + `nextPageToken` to exhaustion) verified plausible against
  `jira-search-flow.sh`; cost honestly restated.
- 🟡 **Tracked-but-remote-absent branch** — **Resolved**. Added as a distinct
  engine state, separate from indeterminate/failed-read.
- 🟡 **epoch→ISO date idiom** — **Resolved**. Idiom removed; epoch-seconds +
  pure-integer mtime compare via the repo-verified dual `stat`.
- 🟡 **Uncommitted-edit overwrite / aggregate gate** — **Partially resolved**.
  Aggregate gate added; the per-file guard itself is now the new critical (below).
- 🟡 **Two prompt polarities** — **Resolved**. Conflict prompt is now a typed
  token, no longer a `[Y/n]` clash.
- 🔵 Post-overwrite hash, finalise/run-start timestamp, exit-taxonomy single
  source, `wip_` prefix, terminal-71 rationale, fixtures, blast-radius/`--count`
  tests — **Resolved** as specified.

### New Issues Introduced

- 🔴 **Safety (CRITICAL)**: The uncommitted-edit guard relies on a per-file
  "dirty" check that **does not exist in the codebase and is conceptually
  unavailable under jj** (the working copy is always a commit — there is no
  uncommitted state). The safeguard meant to protect the only non-VCS-recoverable
  local data may silently no-op under the repo's actual VCS, re-opening the
  silent-loss hole. Fix: define a VCS-mode-aware dirty seam with a real jj
  contract (`jj diff`/`jj status` against the working-copy commit; `git status
  --porcelain` for git mode), or make the aggregate blast-radius gate the
  load-bearing guard and say so.
- 🟡 **Architecture + Correctness**: `--keys` builds `key in (…)` via
  `jira-search-flow.sh --jql`, which **mandates a project clause** (`E_JQL_NO_PROJECT`,
  exit 30) and otherwise AND-narrows to the default project — so cross-project
  tracked keys either error or are dropped and misclassified as remote-absent. Fix:
  `--keys` must pair with `--all-projects` (the key set is the authoritative
  filter).
- 🟡 **Correctness**: The remote pre-filter becomes **load-bearing on the list
  path** — `remote_hash` is a `show`-body digest, but list feeds only `search`
  bodies and reserves `show` for the changed minority, so an unchanged-`updated`
  item can't be hash-confirmed. Resolve by making the remote side explicitly
  asymmetric: `updated`-equality is a *trusted* short-circuit, and any `updated`
  change forces a `show` before a remotely-modified verdict.
- 🟡 **Test-Coverage**: No **update** dropped-response fixture exists (only a
  *create* one), and the create-bridge `start_mock` the plan mirrors does not wire
  `--captured-bodies-file`/`--captured-urls-file` — so the "assert against the
  captured request" checks would have nothing to assert. Both need calling out.
- 🟡 **Usability**: The conflict prompt offers no **skip/abort** token (both
  `remote`/`local` are destructive writes; the natural "resolve later" outcome is
  only reachable via the bad-input fallback), and the two **blast-radius gate**
  answer grammars are left unpinned.
- 🟡 **Safety**: The blast-radius gates must be specified to **fail safe** (abort
  the affected class with zero writes) when run non-interactively / confirmation is
  unavailable, evaluated before any write in that class.
- 🔵 Minors: pin a single authoritative body source for any persisted `remote_hash`
  (always `show`, never `search` description); `--keys` `--limit` should be 100 so
  a 50-key chunk never triggers a speculative page-2 fetch; cap-overflow on the
  keyed fetch must be indeterminate, not silent remote-absent; guard non-numeric
  `stat` output; first-sync-conflict-on-dirty-file masking; resolve the `revision`
  hedge; `E_FETCH_*`/`E_UPDATE_*` sketch names should be the canonical
  `E_DISPATCH_*`; retrofit `work-item-push-decide.sh` onto the shared codes too.

### Assessment

The plan is close and the dangerous structural work is done — but it is not yet
"ready" because Pass 3 found a genuine **critical** (the jj dirty-check no-op) and
two real correctness majors (`--keys` project scoping; remote-side load-bearing
pre-filter) that would cause silent data loss / misclassification if implemented as
written. These are all narrow, well-understood, and cheaply fixable. One more
focused iteration — the jj-aware dirty seam, `--keys --all-projects`, the explicit
remote-side asymmetry, and the four pinning/fixture clean-ups — should converge it.

---
*Re-review generated by /accelerator:review-plan*
