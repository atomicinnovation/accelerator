---
type: plan-validation
id: "2026-08-05-0169-vcs-subdomain-and-hooks-migration-validation"
title: "Validation Report: VCS Subdomain and Hooks Migration Implementation Plan"
date: "2026-08-06T00:45:48+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
target: "plan:2026-08-05-0169-vcs-subdomain-and-hooks-migration"
tags: [rust, vcs, hooks, migration]
last_updated: "2026-08-13T16:00:13+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: VCS Subdomain and Hooks Migration Implementation Plan

### Implementation Status

✓ Phase 1: Capture Shell Behaviour as Fixtures — Fully implemented
✓ Phase 2: Move Checkout Types into the `vcs` Domain Crate — Fully implemented
✓ Phase 3: The Checkout Classifier — Fully implemented
✓ Phase 4: Shared Hook Envelope Module — Fully implemented
✓ Phase 5: Launcher Fail-Safe for External Dispatch, and `vcs detect` — Fully implemented
✓ Phase 6: `vcs status` and `vcs log` — Fully implemented
✓ Phase 7: `vcs guard` — Fully implemented
✓ Phase 8: Sub-Binary Registration and Skill Repoint — Fully implemented
✓ Phase 9: `hooks.json` Rewrite and Shell Deletion — Fully implemented
✓ Phase 10: Hand-offs, Documentation, and Validation — Automated portion complete; three manual/release-gated items explicitly deferred (see below)

**Retracted 2026-08-13.** None of the three is release-gated any longer.
`v1.24.0-pre.35` and `v1.24.0-pre.36` both ship `accelerator-vcs-darwin-arm64`
alongside its `.minisig`, and `pre.36`'s signed `manifest.json` carries `vcs`
entries for all four platforms — see the retraction on the "Three
manual/release-gated items" passage below.

All ten phases have a corresponding commit on the current branch, in the
sequence the plan specifies (fixture capture precedes deletion, satisfying
Sequencing Constraint 2):

```
abf654fc8983 Capture VCS shell behaviour as fixtures ahead of the Rust port
678264717b46 Move checkout types from vcs-adapters::library into the vcs domain crate
7635eff087d7 Add the seven-arm checkout classifier to the vcs domain crate
1d4b212b3ae4 Add the shared kernel::hooks envelope module
232b765815c4 Add launcher fail-safe dispatch handling and the vcs detect subcommand
db9b08919005 Add the vcs status and vcs log subcommands
e800dfaa1012 Add the vcs guard subcommand
b35310e3ffdc Register accelerator-vcs as a dispatched sub-binary and repoint the commit skill
db2dc2b0564d Rewrite hooks.json onto accelerator-vcs and delete the retired shell scripts
c50b5e4e8414 Record 0169 hand-offs, spin off two follow-up work items, fix a launcher race
8601642f2a0c Add a follow-up work item for migrating vcs status/log onto library adapters
```

### Automated Verification Results

✓ `mise run` passes end to end: full local CI mirror (frontend + server + cli
  builds, all formatters/lint-fixes, every lint/type-check, and the entire
  test suite including e2e) — exit 0, finished in 719.5s. No test failures;
  the only stderr noise observed (a jsdom "navigation not implemented"
  warning in a frontend test and a deliberately-simulated Jira 500 in a mock
  integration test) is expected output from passing tests, not a failure.
✓ Working tree clean (`jj status`): no uncommitted changes, nothing left
  stray from implementation.
✓ Four independent codebase-analyser passes (one per phase group) read every
  file the plan's "Changes Required" sections name and cross-checked it
  against the corresponding "Success Criteria" — all EXISTS/MATCHES, one
  benign structural note (below), zero MISSING or contradicted items.

### Code Review Findings

#### Matches Plan:

- **Phase 1** — `hooks/test-fixtures/masks.toml` carries exactly the 7 named
  pattern categories with `sample_match`/`sample_no_match` pairs; the
  status/log golden directory has 10 pairs (9 prose states with git
  ahead/behind split, as the plan's own arithmetic note anticipates); the
  guard decision table has exactly 138 rows; both new detect fixtures exist
  with the correct structured-vs-descriptive contract; the cross-engine
  differential test exists on both the Rust (`cli/vcs-test-support/tests/masks.rs`)
  and Python (`tests/integration/hooks/test_masks.py`) sides.
- **Phase 2** — `cli/vcs/src/checkout.rs` holds the four moved types with
  `DualRoots` retyped to `kernel::Error`; `cli/vcs-adapters/src/library.rs`
  imports them and maps its own `Error` at the boundary via an explicit
  `From` impl, mirroring the `ResolutionError` precedent.
- **Phase 3** — `cli/vcs/src/classify.rs`'s `Classification` enum has
  exactly the seven specified variants; cascade order checks `Colocated`
  before both `Nested*` arms; the two-different-kinds-of-`Err` handling
  (hard-fail on `is_bare`/`worktree`/`jj_repository`, graceful `dual_roots`
  degradation) is implemented and tested exactly as specified, including the
  both-sides-`Err`-degrades-to-`Main` case.
- **Phase 4** — `cli/kernel/src/hooks.rs` carries all four envelope
  functions plus `json_escape`; `pre_tool_use_warn` emits a bare
  `{"systemMessage":...}` with no `permissionDecision`; the launcher's
  existing `hook_envelope` now delegates rather than duplicating.
- **Phase 5** — `forwarded_fail_safe`/`swallow_under_fail_safe` implement the
  allowlist-not-exclusion semantics precisely; the `From<ResolutionError>`
  impl is an exhaustive match with no wildcard arm, routing the four
  integrity variants (plus the conditional `CorruptCacheAndRefetchFailed`
  split) to `Refusal` and everything else to `Failed`; `cache_root.rs` is
  split into `candidate()`/`verify_writable()` with the write probe moved
  behind the cache-miss path; `cli/vcs-cli` scaffolds `accelerator-vcs` with
  the `Detect` subcommand behaving as specified.
- **Phase 6** — `subprocess.rs` gains a shared `run_capped` primitive that
  `revision()` now wraps, plus `status()`/`log()` shelling the four exact
  commands with the same cap-and-kill and environment scrubbing;
  `status_log_goldens.rs` masks live output against the same `masks.toml`
  and asserts byte-equality across all ten captured states.
- **Phase 7** — `cli/vcs/src/guard.rs`'s `decide()` is pure, quote-aware, and
  matches the 13-subcommand blocklist plus the `gh`/`rtk` allowance;
  `cli/vcs-cli/src/guard.rs` determines mode first via the same
  mode-determination path `vcs detect` uses (a shared `vcs::mode::determine`
  module, not `Classification` directly, correctly sidestepping the
  fieldless-`Main`-variant problem the plan calls out), then applies
  `decide()` only when jj is present; the 138-row decision table passes end
  to end through the compiled binary.
- **Phase 8** — `DISPATCHED_SUBBINARIES`, `_SUBBINARY_MANIFESTS`,
  `cli/Cargo.toml` workspace members, `.gitignore`, and
  `_CLI_RELEASE_BINARIES` all gain the `vcs`/`accelerator-vcs` entries;
  `skills/vcs/commit/SKILL.md` drops the broad `scripts/*` permission for
  the subcommand-scoped one and repoints its body to the new subcommands.
- **Phase 9** — `hooks/hooks.json` registers the three verbatim command
  strings and no longer references any of the five retired shell files; all
  five files are confirmed deleted from disk; the 42-case parity gate is
  repointed exactly per the plan's five-bucket partition, with
  `test:integration:hooks` now depending on `build:cli:dev` and the stale
  `test_mise.py` pin removed; a doc-reference sweep of `README.md`,
  `docs-site/`, and `tasks/README.md` for the five retired filenames returns
  nothing.
- **Phase 10** — Dated hand-off notes are present in `0172`, `0183`, `0125`,
  and `0189`; the two required follow-up work items (`0199`, `0200`) exist
  and are cross-linked from `0169`'s own `relates_to`; `mise run` (the
  phase's own top-line automated criterion) passes.

#### Deviations from Plan:

- `cli/launcher/src/main.rs`'s fail-safe handling (Phase 5, item 1) is
  implemented via an extracted, independently unit-tested
  `handle_dispatch_error(error, command)` helper rather than inlined
  directly in `main()` as the plan's prose describes. This is a structural
  improvement for testability, not a behavioural deviation — confirmed
  functionally identical by dedicated tests at `cli/launcher/src/main.rs:305-331`.
- `cli/vcs-adapters/src/subprocess.rs`'s `status()`/`log()` route through an
  intermediate private `run_vcs_text` helper rather than calling
  `run_capped` directly as the plan's prose implies — again a minor
  implementation-detail refinement, not a functional gap.

#### Potential Issues:

- **Work item `0169`'s Acceptance Criteria checkboxes were out of sync**
  with the confirmed-complete state (13 of 19 unchecked despite the plan's
  own phase-level Success Criteria all showing `[x]`). **Resolved
  2026-08-06**: synced against the verification evidence above — 16 of 19
  now checked. The `accelerator-vcs` registration criterion was only
  partially checkable: its manifest/description, dispatch-coherence, deny,
  pup, and clippy sub-clauses are confirmed via `mise run`, but its musl
  `_assert_static_elf` sub-clause requires `build:cli:cross-compile`, a
  release-pipeline-only task `mise run` does not invoke — left unchecked
  with an inline note rather than assumed complete.
- Three manual/release-gated items remain genuinely open, and the plan
  itself flags them as deferred rather than done:
  1. **Claude Code floor check end-to-end** (Phase 9 & 10 manual items) — a
     real session against this branch's installed `hooks.json` denying a
     blocked git call and showing the colocated warning. The schema-level
     precheck (Sequencing Constraint 1) is confirmed; the end-to-end
     re-verification through the actual installed hook registration is not,
     because it requires a published release.
  2. **Warm-call latency measurement** (`G ≤ 1.1 × B`, Phase 10) — requires
     the real bootstrap-to-launcher-to-sub-binary dispatch path with no
     `ACCELERATOR_VCS_BIN` override, which in turn requires a published,
     minisign-signed `accelerator-vcs` release asset that does not yet
     exist.
  3. **The release-cut deployment gate** (Sequencing Constraint 4 /
     Dependencies) — scheduling the release that must precede `hooks.json`'s
     rewrite reaching an installed-plugin path is explicitly an owner
     action, not a code change this plan can complete.

  **Retracted 2026-08-13.** The release premise all three rest on is false as
  of `v1.24.0-pre.35`. Both `pre.35` and `pre.36` ship
  `accelerator-vcs-darwin-arm64` with its `.minisig`, and `pre.36`'s signed
  `manifest.json` carries `vcs` entries for all four platforms at
  `schema_version: 1`. Specifically: item 2's "a published, minisign-signed
  `accelerator-vcs` release asset that does not yet exist" is wrong — work item
  0205 dispatched exactly that path with no `ACCELERATOR_VCS_BIN` override and
  measured it; and item 3's release cut has been performed. Item 2's threshold
  `G ≤ 1.1 × B` is separately superseded — it was measured at a ratio of medians
  of 1.2813 and fails; the obligation is discharged on work item 0189 under the
  criterion recorded there, not here.

  All three are pre-declared in the plan and work item as blocked on an
  external release process, not oversights — but they mean the story is not
  yet fully closed out from a user-facing standpoint until that release
  ships and the two measurements are recorded in Validation Results.

### Manual Testing Required:

1. **Release and end-to-end floor check**:
   - [ ] Cut an epic-0136 release whose published manifest lists
     `accelerator-vcs`.
   - [ ] With that release installed, run a real Claude Code session: verify
     a blocked git subcommand in a pure-jj scratch repo is denied, and the
     colocated warning appears in the transcript for a colocated checkout.
     Record the observed client version in the work item's Validation
     Results.

2. **Warm-call latency**:
   - [ ] On a darwin-arm64 host with no build running, capture the median of
     20 `hooks/vcs-guard.sh` invocations (B) and 20 warm, non-overridden
     `accelerator vcs guard` invocations (G) against the same stdin payload
     and pure-jj fixture. Confirm `G ≤ 1.1 × B` and record B, G, the ratio,
     payload, fixture, and host in the work item's Validation Results.

3. **Work item hygiene** (not user-facing, but recommended before closing
   the story):
   - [ ] Reconcile `meta/work/0169-...md`'s Acceptance Criteria checkboxes
     against the confirmed-complete state above.

### Recommendations:

- Treat this validation's `pass` result as covering everything within this
  plan's code scope: all ten phases are fully and correctly implemented,
  `mise run` is green, and no deviation found affects behaviour. The three
  outstanding manual items are release-process prerequisites the plan itself
  scoped out of its own code changes, not implementation gaps — they should
  be tracked to closure separately (the work item's Dependencies section
  already names the release owner) rather than reopening this plan.
- Sync `meta/work/0169-vcs-subdomain-and-hooks-migration.md`'s Acceptance
  Criteria checkboxes to reflect the automated-verification-complete state,
  leaving only the three genuinely-pending items unchecked, so the work item
  and plan agree.
- No code changes are recommended from this validation pass.
