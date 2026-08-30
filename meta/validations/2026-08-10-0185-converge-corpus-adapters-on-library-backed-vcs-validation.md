---
type: "plan-validation"
id: "2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs-validation"
title: "Validation Report: Converge corpus-adapters on the Library-Backed VCS Adapter"
date: "2026-08-11T09:24:20+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "work-item:0185"
target: "plan:2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs"
tags: ["rust", "vcs", "cleanup", "tech-debt"]
last_updated: "2026-08-11T09:24:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Converge corpus-adapters on the Library-Backed VCS Adapter

Implemented across five changes on top of `27044ae3b3ce`:

| Change | Phase |
| --- | --- |
| `aec43c3ae941` Record the VCS probe policy decisions the composition switch depends on | 1 |
| `cbc4f0c4b695` Resolve repository facts in-process rather than by spawning jj or git | 2 |
| `3a7dd0b4022f` Delete the subprocess-backed root and revision probes | 3 |
| `4112316793c3` Record where the MPL-2.0 closure actually links across the sub-binaries | 4 |
| `8bc00b20faa5` Strip comments that restate the code or name deleted implementations | (unplanned) |

### Implementation Status

✓ Phase 1: Record the pending policy decisions — fully implemented
✓ Phase 2: Extend zero-spawn coverage, then repoint `facts` — fully implemented
✓ Phase 3: Delete `CommandProbe` and collapse the dual-adapter comparison — fully implemented
✓ Phase 4: Re-run the MPL-2.0 licence check — fully implemented (with a recorded methodology deviation)

⚠️ Validation found one defect introduced by Phase 2 that no phase's
verification set covered: the two new zero-spawn tests broke the
`check-zero-spawn` CI job, a required dependency of the Main CI aggregate.
**Fixed during validation** — see Potential Issues, item 1.

### Automated Verification Results

✓ `mise run check` passes (exit 0, 42.30s)
✓ `cargo nextest run -p vcs-adapters -p corpus-adapters --features bash-parity`
  — 201 tests run, 201 passed, 0 skipped
✓ `cargo nextest run -p corpus-adapters --features bash-parity -E 'binary(zero_spawn)'`
  — 2 passed (the metadata test covers both idioms in one body after the fix)
✓ Simulated strong lane — the compiled `zero_spawn` binary run under
  `env -i` with a `PATH` carrying no `git`/`jj` and the matrix handed over via
  `ACCELERATOR_ZERO_SPAWN_MATRIX` — 2 passed. The same simulation reproduced
  the original break first (both new tests erroring with
  `git: No such file or directory`), so the fix is verified against a proven
  red.
✓ `grep -rn "CommandProbe" cli/ --include="*.rs"` — no matches
✓ `grep -rn "MarkerWalkRoot" cli/ --include="*.rs"` — no matches
✓ Surviving `Command::new` in `cli/vcs-adapters/src/` is confined to
  `subprocess.rs`'s `status`/`log` path (`:68-95`) plus its own generic
  shell-stand-in tests — no `facts`-serving spawn remains
⚠️ `mise run test:integration:zero-spawn:strong` — not runnable locally (it
  `sudo mv`s system `git`/`jj`, so it belongs on an ephemeral runner). Covered
  by the simulation above instead, and by confirming that
  `_compile_zero_spawn_targets`' `--no-run` step does build
  `corpus-adapters-fixture` (checked by deleting it and rebuilding), so
  everything the shadow window needs exists before it opens.
✓ MPL-2.0 finding independently reproduced against the existing release
  artefacts (`nm -a | grep -c`):

  | Binary | `gix_` | `jj_lib` | `uluru` |
  | --- | --- | --- | --- |
  | accelerator-corpus | 546 | 238 | 3 |
  | accelerator-vcs | 467 | 117 | 3 |
  | accelerator-work | 547 | 274 | 3 |
  | accelerator-collaboration | 464 | 0 | 3 |
  | accelerator-migrate | 2163 | 2878 | 3 |
  | accelerator-visualiser | 0 | 0 | 0 |

  This matches `cli/deny.toml`'s recorded finding exactly: five binaries link
  `uluru`, the visualiser links none of the three.

### Code Review Findings

#### Matches Plan:

- `cli/vcs-adapters/src/lib.rs:25` is now
  `vcs::facts(start, &InProcessProbe, &InProcessProbe)`, exactly the shape
  Phase 2 specified.
- All three policy decisions are recorded where the plan placed them:
  sha256 handling on `VcsProbe::revision` (`cli/vcs/src/lib.rs:72-75`), the
  containment bound above `InProcessProbe` (`cli/vcs-adapters/src/library.rs:189-209`),
  and the snapshot-on-read scope on `VcsBackedRepoFactsProbe`
  (`cli/corpus-adapters/src/metadata.rs:205-210`).
- Both backing tests the plan added for those decisions exist and pass: the
  sha256 `revision` case (`cli/vcs-adapters/tests/queries.rs:575-577`) and the
  git-side malformed-ref case
  (`unreadable_git_ref_data_reports_absence_rather_than_a_wrong_commit`,
  `cli/vcs-adapters/tests/library.rs:411-425`). The latter asserts the fixture
  answers *before* it is broken, so it cannot pass vacuously.
- The new reference binary sits at
  `cli/corpus-adapters/tests/fixtures/corpus_adapters_fixture.rs`, declared as
  a `[[bin]]` with no `required-features` gate, mirroring `vcs-adapters-fixture`
  as specified. It is not in `_CLI_RELEASE_BINARIES` (`tasks/build.py:36-44`),
  which is an explicit tuple rather than a glob, so it cannot reach release
  staging.
- Every deletion the plan enumerated landed cleanly, including the easy-to-miss
  ones: the doc comment above `CommandProbe`, `run_checked`/`wait_capped_checked`,
  the orphaned `origin_repo()` helper, the nested test-module imports
  (`std::path::PathBuf` and `CommandProbe` at `subprocess.rs:414,421`), and
  `detection.rs`'s `facts_via` indirection with its `RepoRoot`/`VcsProbe`
  imports.
- `run_capped`'s doc comment no longer intra-doc-links the deleted
  `CommandProbe::revision`; the module-level comments in `subprocess.rs`,
  `lib.rs`, `detection.rs` and `library.rs` were all retitled as the plan
  required, and `library.rs`'s now names all three test groups rather than two.
- `an_unsnapshotted_edit_is_the_one_documented_divergence` was reworked (not
  deleted) to snapshot via `jj_revision_oracle` rather than `CommandProbe`, and
  the new `git_revision_oracle` closes the git-side oracle gap the plan
  identified in `a_plain_git_repository_reports_git_kind`.
- The colocated test was renamed to `a_colocated_checkout_is_driven_as_jj_in_process`,
  avoiding the name collision with `detection.rs` the plan warned about.
- Work item `0203` was filed for the MPL-2.0 attribution artefact, as Phase 4
  required, and carries the methodology caveat for whoever picks it up.

#### Deviations from Plan:

- **Phase 4 verification method changed, and is recorded as such.** The
  plan's `extensions.objectFormat` / `There is no Jujutsu repo` string-literal
  procedure proved unsound as an absence test. The implementation switched to
  `nm -a` symbol counts and recorded the literals' unreliability in
  `cli/deny.toml:91-95` and in `0203` itself. The plan's manual-verification
  checkbox was annotated with the deviation rather than silently ticked. This
  is the right call — my own reproduction confirms the symbol counts.
- **An unplanned fifth commit (`8bc00b20faa5`) strips comments beyond the
  plan's scope.** It rewrites the Phase 1 doc comments it had just added
  (shorter, but the decisions survive) and additionally strips stale bash-port
  references from `cli/corpus-adapters/src/work_item_pattern.rs`,
  `tests/parity.rs` and `src/metadata.rs` — files the plan never named. The
  change is consistent with the repo's comment policy and with the earlier
  `d8776d652f6c` precedent, but it is scope this plan did not authorise and it
  touched a parity suite the plan explicitly listed as "must keep passing
  unchanged".
- **`InProcessProbe` gained `Default`** (`#[derive(Debug, Clone, Copy, Default)]`)
  where the plan's snippet showed a bare `pub struct InProcessProbe;`. Harmless.

#### Potential Issues:

1. **The two new zero-spawn tests failed the `check-zero-spawn` CI job.**
   **Resolved during validation** — resolution at the end of this item.

   `.github/workflows/main.yml:323-367` runs
   `mise run test:integration:zero-spawn:strong`, which physically `sudo mv`s
   every resolved `git` and `jj` aside (`tasks/test/integration.py:214-241`,
   `_resolve_vcs_binaries` at `:279`) and then runs the whole `zero_spawn`
   binary under `-E 'binary(zero_spawn)'`.

   The pre-existing test survives that window because it *adopts* a matrix
   built beforehand (`fixtures::matrix_root()` +
   `Matrix::build_or_adopt`, honouring `ACCELERATOR_ZERO_SPAWN_MATRIX`). The
   two new tests instead build their fixtures inline —
   `environment.git(&["init", "--quiet"], &root)`
   (`cli/corpus-adapters/tests/zero_spawn.rs:165-167`) and
   `fixtures::pure_jj(...)` (`:179`) — both of which resolve their binary
   through `Command::new("git")`/`Command::new("jj")`
   (`cli/vcs-test-support/src/hermetic.rs:94,121`) and return `Err` when it
   cannot be run. With the binaries moved, fixture construction fails and both
   tests error out.

   `check-zero-spawn` is a required dependency of the aggregate at
   `.github/workflows/main.yml:436`, so this breaks Main CI. It is invisible
   locally: the strong lane is deliberately out of every roll-up, so neither
   `mise run check` nor the bare `mise run` exercises it.

   **Resolution**: the two tests were replaced by a single
   `the_metadata_read_resolves_both_idioms_without_spawning_them`, which draws
   its start directories from the shared matrix (`PG-r` for plain git, `PJ` for
   pure jj) exactly as the sibling test does, so the strong lane's existing
   `_build_fixture_matrix` pre-build already covers them and no new CI wiring is
   needed. Both idioms are still exercised, which was the plan's reason for
   asking for two tests. Verified non-vacuous: both shapes resolve real facts
   (`name=PG revision=1cbe1df0…`, `name=c revision=32922826…`) rather than
   matching as "absent" twice.

2. **The new tests did not fail closed on a malformed mode contract.**
   **Resolved during validation.** The pre-existing test opens with
   `Mode::from_environment()?` + `assert_shadowing_holds(mode)?` precisely so a
   dropped export silently downgrades the run rather than passing; the new tests
   skipped that, so they would have reported a strong-form pass in a path-only
   environment. The replacement test now opens the same way.

3. **The canonicalisation behaviour change is documented in the plan but not in
   the code.** Phase 2 noted that `InProcessProbe::repository_root`
   canonicalises where `MarkerWalkRoot`'s did not, and that this can change the
   persisted `Repository Name:` when the repository directory itself is a
   symlink. The plan judged it too narrow for new test infrastructure, which is
   defensible, but nothing in the tree records it — the note lives only in the
   plan document. A line on `VcsBackedRepoFactsProbe` alongside the staleness
   note would keep it findable.

### Manual Testing Required:

1. Zero-spawn strong lane:
  - [ ] Confirm the real lane is green now that issue 1 is fixed — push the
        branch and watch `check-zero-spawn`. Do not run
        `test:integration:zero-spawn:strong` on a developer machine; it
        `sudo mv`s system `git`/`jj` and is meant for ephemeral runners.

2. `corpus metadata derive` behaviour:
  - [ ] Against a jj repository with unsnapshotted edits, confirm
        `Current Revision:` names the last recorded commit (the accepted
        divergence), not a freshly snapshotted one
  - [ ] Against a bare repository, confirm it still reports no facts

3. Hook path:
  - [ ] `vcs detect` / `vcs guard` still behave — unchanged in principle, since
        they already used `InProcessProbe`

### Recommendations:

- Issues 1 and 2 are fixed in `cli/corpus-adapters/tests/zero_spawn.rs`; the
  branch is mergeable once `check-zero-spawn` is confirmed green on CI.
- Consider recording the canonicalisation note in the code rather than only in
  the plan (issue 3).
- Worth a follow-up in its own right: the strong lane is the only place this
  class of defect can surface, and nothing about writing a test in
  `zero_spawn.rs` signals that building a fixture inline is forbidden. A note in
  the module doc, or a helper that is the only sanctioned way to get a start
  directory there, would stop the next person rediscovering this.
- The unplanned comment-stripping commit is fine on its merits but would sit
  more comfortably as its own change with its own justification, given it
  touched a parity suite the plan had ring-fenced.
- `0203` should be prioritised rather than parked: MPL-2.0 §3.2's notice
  obligation is live today for five shipped binaries, and four of those were
  already shipping before this plan.
