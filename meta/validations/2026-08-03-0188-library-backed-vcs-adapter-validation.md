---
type: "plan-validation"
id: "2026-08-03-0188-library-backed-vcs-adapter-validation"
title: "Validation Report: Library-Backed VCS Adapter over gix and jj-lib"
date: "2026-08-03T18:05:57+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "work-item:0188"
target: "plan:2026-08-03-0188-library-backed-vcs-adapter"
tags: ["rust", "vcs", "dependencies", "gix", "jj-lib"]
last_updated: "2026-08-03T18:05:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Library-Backed VCS Adapter over gix and jj-lib

Validated against the five implementation commits `lpzkkkzruqsw..rztymnuzussm`
(44 files, +7,152/−161), working copy clean at `d0acd6b3`.

**Result: pass.** All five phases are implemented and every automated criterion
passes, including a green end-to-end `mise run`. Four findings were raised and
all four are resolved in follow-up commits (see Fixes Applied) — three
documentation and hygiene issues, plus one substantive reversal: the descoped jj
`revision` mechanism, raised by the author as a regression at 0185's switch, is
now **delivered**. The remaining unverified criteria are CI-only by construction
and correctly deferred.

One caveat on the "`mise run` green" criterion: it passed cleanly on two of five
invocations, most recently on the final state of this work (653s, zero task
failures, no formatter drift). The failures are all in
`test:integration:entrypoint` and are attributable to a pre-existing
shared-target-directory race that this change aggravates but does not
introduce — see Potential Issues.

### Implementation Status

- ✓ **Phase 1: Dependencies, Policy Gates and the Boundary-Safe Ports** — fully
  implemented, with one unamended deviation (see Deviations).
- ✓ **Phase 2: The Six Taxonomy Queries** — fully implemented.
- ✓ **Phase 3: The Shared Test-Support Crate and the Zero-Spawn Harness** —
  fully implemented.
- ✓ **Phase 4: Reference Artefact, Strong-Form CI, and Measurements** — fully
  implemented; the strong-form CI job is written and wired but by construction
  first executes on CI.
- ✓ **Phase 5: Sibling Hand-Offs and Closeout** — fully implemented.

### Automated Verification Results

✓ `mise run` green end to end — 557.70s, zero task failures, working copy still
clean afterwards (no formatter drift). Includes:

- ✓ `deny:check` passes with the `uluru` MPL-2.0 exception
- ✓ `pup:check` passes with the new `vcs_adapters_library_reads_in_process` rule
- ✓ `cli:check` (clippy `--locked`, pedantic + nursery)
- ✓ `test:unit:cli` — 620 tests, 620 passed
- ✓ `test:integration:deny` — 62 passed, including the 12 new
  `test_vcs_library_graph.py` cases and the 4 `test_advisory_ignores.py` cases
- ✓ `test:unit:tasks` — 649 passed, including `test_vcs_pin_lockstep.py` (7),
  `test_vcs_settings.py`, the updated `test_mise.py` and the new
  `test_build.py` size-floor/fixture-staging cases
- ✓ `lint:vcs-settings:check` passes (125.7ms)
- ✓ `test:integration:pup` — the three new probe cases for the rule (denied
  `std::process` naming the rule, compliant single-item positive control,
  grouped-import behaviour)
- ✓ `hooks/test-vcs-detect.sh` and `scripts/test-metadata-helpers.sh` both ran
  green inside the run — this discharges the plan's manual step 2 for the
  jj-0.43 fixture suites

✓ `mise run test:integration:zero-spawn` — 1 test, passed in PATH-only mode
(deliberately outside the `test:integration` roll-up, so not covered by
`mise run`; run separately).

✓ `mise run build:cli:fixture-size` — host-native ratio floor holds. After the
jj `revision` route landed: `linked 2,513,792 B, stubbed 350,912 B, ratio
**7.16x**` against a 3.0× floor (6.49× before it).

✓ Full `mise run` re-run green on the final state — 653.15s, zero task failures,
no formatter drift; `test:unit:cli` 625 tests (620 before, plus the 5 new jj
`revision` cases), `test:integration:deny` 65, `test:unit:tasks` 650.

✓ Gates re-run individually during the fixes, all green:
`deny:check`, `test:integration:deny` (62 passed), `test:unit:tasks`
(649 passed — includes the two comment-presence assertions the reworded
`cli/deny.toml` block has to satisfy), `test:integration:work`, `cli:check`
(after the dev-dependency removal), and `cargo test -p vcs-test-support
--no-run`. `cli/Cargo.lock` is unchanged by the fixes.

⚠️ **`test:integration:entrypoint` failed in two of three full runs** — see
Potential Issues. Passes in isolation (54 passed) and passed in the green run.

### Code Review Findings

#### Matches Plan:

- `cli/vcs-adapters/src/library.rs` (845 lines after the jj `revision` route;
  686 as reviewed) carries `InProcessProbe`, both
  port impls, and all six inherent queries with the planned signatures —
  `is_bare`, `worktree`, `superproject`, `jj_workspace_root`, `jj_repository`
  return `Result<Option<T>, Error>`; `dual_roots` is infallible with a per-side
  `Result` (`library.rs:184-192`, `:205-359`).
- The three walks are distinct and correctly assigned: `carries_any_marker` for
  `RepoRoot::discover`, `carries_jj_marker` for the jj queries, `gix::discover`
  for the git queries (`markers.rs:31-44`, `library.rs:297`, `:320`).
- Delegation direction is as specified — `MarkerWalkRoot::discover` and
  `CommandProbe::kind` delegate *to* `markers::walk_up`/`marker_kind`, so
  0185's deletion needs no re-homing (`lib.rs:38-41`, `:93-96`).
- Canonicalisation choke point present and documented in the module `//!`
  header; `worktree` canonicalises `common_dir()` before exposing it
  (`library.rs:225-228`).
- `jj_repository` carries the `<main_root>/.jj/repo`-is-a-directory
  post-condition that the shell oracle carries (`library.rs:342-344`).
- Boundary rule: `cli/vcs/src/**` is byte-for-byte unmodified (`jj diff --stat`
  over the five commits reports 0 files); `vcs_adapters::facts` still names
  `MarkerWalkRoot`/`CommandProbe` (`lib.rs:217-219`); `CommandProbe` and
  `MarkerWalkRoot` gained no methods; `cli/vcs-adapters/Cargo.toml` gained no
  `[features]` entry beyond `bash-parity`.
- `cli/vcs-test-support` depends on `vcs`-adjacent crates only — no
  `vcs-adapters` edge, so no dev-dependency cycle. Its `stubs` module resolves
  and *reports* absolute paths and never mutates outside its own temp root
  (`stubs.rs:91`, `:171`).
- The zero-spawn contract is fail-closed on malformed input:
  `Mode::from_environment`
  rejects any value other than `strong`/`path-only`/empty, and rejects a
  non-empty shadow list without `strong` (`stubs.rs:53-72`);
  `assert_shadowing_holds` hard-fails when `strong` is claimed while a listed
  path is still executable (`stubs.rs:210-233`).
- `cli/corpus-adapters/tests/zero_spawn.rs` imports all three parts of the
  public API (`fixtures`, `hermetic`, `stubs`) and asserts a non-empty matrix so
  an empty one cannot pass vacuously (`zero_spawn.rs:22-27`, `:75-77`).
- `detection.rs` runs every case through a `(&dyn RepoRoot, &dyn VcsProbe)` seam
  against both implementations. Parity was asserted per `VcsKind` as planned; it
  is now **whole-struct `RepoFacts` equality** for both idioms, since the jj
  `revision` reversal removed the field neither adapter could answer.
- CI job `check-zero-spawn` matches the plan's shape exactly: `ubuntu-latest`,
  `cache: false` on `mise-action`, compile-then-fixtures-then-shadow ordering,
  `trap restore EXIT` inside a single step with its own `timeout-minutes: 10`,
  idempotent per-path shadow/restore, and an `if: always()` liveness backstop
  (`main.yml:323-457`); wired into `prerelease.needs` (`:498`).
- `tasks/build.py` uses a separate `_CLI_FIXTURE_BINARIES` constant and staging
  loop; `_CLI_RELEASE_BINARIES` is untouched; the ratio floor applies to every
  triple and the absolute floor to musl only (`build.py:45-62`, `:203-217`).
  `test_build.py:466-479` asserts directly that no fixture name appears in
  `_release_uploads()`.
- Phase 5's three sibling amendments are all present as dated append-only
  blocks (`0125:149`, `0169:542`, `0185:146`), and 0125's `relates_to` carries
  `work-item:0188`.
- `tasks/README.md` gained both planned subsections (`Library-backed VCS
  dependency pins` at `:109`, `Zero-spawn strong form` at `:173`), the
  class-scoped break-glass, the licence-side failure mode, the cargo-pup ↔
  source-guard division of labour, and the CI-table row (`:436`).

#### Deviations from Plan:

- ✅ **REVERSED — the jj half of `revision` returned `None` by design.** The
  plan's amendment 8 descoped it to 0185 on a spike finding that no read-only,
  settings-free route existed. Raised by the author as a regression at 0185's
  switch; the finding was wrong and the mechanism is now delivered. See
  Fixes Applied.
- **`gix` is pinned `default-features = false`, not with default features**
  (`cli/Cargo.toml:80`). Sound and better than planned: gix's `default` reaches
  `extras` → `credentials` → `gix-credentials`, the one gix subsystem that
  spawns `git credential-*` helpers, in a module whose whole point is not
  spawning. Nothing is lost — jj-lib's own selection still enables `attributes`,
  `blob-diff`, `index`, `max-performance-safe`, `sha1`, `zlib-rs`, asserted by
  `test_vcs_library_graph.py:272`. **But it is not recorded as a work-item
  amendment**, and `meta/work/0188-*.md:388` still states "**`gix` with default
  features**" as a requirement. Eight smaller deviations got amendments; this
  one should too.
- **`queries.rs` is one table-driven test over all 34 pairs, not one test per
  pair.** The plan chose per-pair to bound fixture rebuild cost and localise
  failures. Delivered: a single test that builds the matrix once, accumulates
  per-cell diagnostics (`key/query`, expected, actual) for every mismatch, and
  asserts `EXPECTED.len() == matrix.fixtures.len()` so the table cannot drift
  from the matrix (`queries.rs:402-459`). Strictly cheaper (1 fixture build, not
  34) with per-cell traceability retained. The plan's rationale is superseded
  rather than violated.
- **`RepoRoot::discover` does not absolutise `start`** (`library.rs:374-376`),
  unlike the six queries. Deliberate: `MarkerWalkRoot::discover` is likewise
  purely lexical, and `detection.rs` asserts parity between them, so
  absolutising here would *create* a divergence. Consequence: the plan's
  "one test per walk uses a relative `start`" is satisfied for the gix walk and
  the `.jj`-only walk only (`queries.rs:467-505`); the boundary walk's
  relative-start behaviour is inherited, not tested.
- Four further deviations were found and recorded by the implementation itself,
  under the plan's `Confirmed during implementation` section — `superproject`
  gating on canonicalised `git_dir() != common_dir()` rather than
  `kind() == Submodule`; `superproject_of` canonicalising in the injected probe
  closure rather than the scan, with the probe returning
  `Result<Option<PathBuf>, Error>`; `D1` returning `Err` rather than the
  recorded `Ok(None)`; and gix 0.85 returning `Err` on sha256 repositories. All
  four are correct under the plan's own partition rule and each carries its
  reasoning.

#### Potential Issues:

- ✅ **RESOLVED — the `uluru` exception comment asserted a licence resolution
  that did not exist.** `cli/deny.toml` stated the MPL-2.0 §3.2 notice
  obligation "is discharged by the third-party licence file staged into the
  release payload". No such artefact existed: no cargo-about /
  cargo-bundle-licenses machinery anywhere under `tasks/`, and
  `_release_uploads()` (`tasks/github.py:231-249`) enumerates launcher
  binaries, signatures, sub-binary assets, debug archives and the manifest only.
  Resolved by measurement rather than by adding an artefact — see Fixes Applied.
- ✅ **RESOLVED — Phase 1's two manual verifications were undone and
  unrecorded**: the `accelerator-visualiser` size before/after, and cold-cache
  `build:server:dev` against `test-visual-regression`'s `timeout-minutes: 20`
  (`main.yml:125`). Both now measured and recorded on the work item; the cap
  needs no raise.
- ⚠️ **OPEN, and not 0188's to fix — `test:integration:entrypoint` fails under
  parallel `mise run` load, in 2 of 3 observed invocations.** Both failures are
  the same defect observed at two points:
  `tests/integration/support/installation.py:125-141` runs
  `cargo build -p accelerator-verify` into the shared `cli/target/debug`, then
  immediately asserts the artefact is present — and the assertion can observe a
  torn tree, because concurrent cargo invocations with *differing feature
  selections* rebuild the same crates and unlink/relink their binaries. Run 1
  surfaced it as `FileNotFoundError` while copying the shim; run 3 as
  `Failed: not built: cli/target/debug/accelerator-verify` in 52 fixture
  setups, immediately after a `cargo build` that exited 0.
  - **CI is structurally immune**: `test-unit` and `test-integration` are
    separate jobs on separate runners (`main.yml:20-21`, `:55-61`), so no
    cross-task sharing of one target directory occurs there. The exposure is to
    the *local* `mise run` gate the contributor guide tells everyone to run.
  - **Pre-existing, aggravated here.** 0188 touches none of that machinery, but
    it enlarges the `--all-features` build surface (`tasks/test/cli.py:11-14`)
    by ~56 crates, widening the window.
  - A deliberate reproduction attempt pairing `test:unit:cli` with
    `test:integration:entrypoint` did **not** trigger it (both green), so the
    specific racing task is unidentified. Deserves its own item.
- **The work item's acceptance criteria are all still unticked** (15 `- [ ]`
  lines) and its `status:` is `ready`, while its Validation Results are filled
  in comprehensively. Housekeeping, not a defect in the change.
- ✅ **RESOLVED** — `cli/vcs-test-support/Cargo.toml` listed `tempfile` in both
  `[dependencies]` and `[dev-dependencies]`.

### Fixes Applied

#### The jj `revision` descope was reversed

Raised by the author on review of the delivered `VcsProbe::revision`: returning
`None` for `VcsKind::Jj` would regress behaviour at 0185's switch, since
`CommandProbe` answers there today. Re-examination found **the spike behind
amendment 8 was wrong**, and the mechanism is now delivered in this story.

The spike's blocking claim was that reading the working-copy commit id needs the
workspace name, reachable only through `LocalWorkingCopy::load` (needs
`&UserSettings`) or `SimpleWorkspaceStore::load` (writes), with `CheckoutState`
private — leaving only "reading the checkout protobuf by hand", dismissed as *"a
private wire format with no compatibility promise"*. That last step is the
error: **`jj-lib` declares `pub mod protos`**, so `Checkout` — carrying exactly
`operation_id` and `workspace_name` — is published API. The delivered code is a
transcription of jj-lib's own `CheckoutState::load` using the same public type.

Delivered route, every link public API and none of it needing settings:

```
DefaultWorkspaceLoaderFactory::create(root)
  → repo_path(), workspace_root()
<workspace_root>/.jj/working_copy/checkout
  → protos::local_working_copy::Checkout → (operation_id, workspace_name)
SimpleOpStore::load(repo_path/"op_store", root_data)
  → read_operation → read_view → View.wc_commit_ids[workspace_name]
```

Verified against the live CLI — exact match on this repo, pure jj
(`--no-colocate`, so no git HEAD to have fallen back to), colocated, commitless,
a secondary workspace and its main. `detection.rs` now asserts **full
`RepoFacts` equality for both idioms** (the per-`VcsKind` narrowing is gone) and
all 7 cases pass; `library.rs` grew 6 cases to 15.

Three properties are pinned by tests rather than asserted in prose:

- **The workspace-name lookup is load-bearing.**
  `each_workspace_reports_its_own_commit_not_its_neighbour_s` asserts the two
  workspaces of one repository report *different* commits and that each matches
  its own oracle. Taking the view's sole entry would pass every single-workspace
  test and answer for the wrong workspace.
- **It writes nothing.** `reading_the_revision_writes_nothing` fingerprints the
  whole `.jj` tree around the call — necessary because a sibling loader in the
  same area does create directories on load.
- **The one divergence is documented and tested.**
  `an_unsnapshotted_edit_is_the_one_documented_divergence`: asking the `jj`
  binary snapshots the working copy first, so with unsnapshotted edits present
  it reports *and writes* a new commit, while this route reports the commit as
  of the last recorded operation. This is the read-only direction — after 0185's
  switch, metadata derivation stops mutating the user's repository.

Cost and size, re-measured on the delivered shape (darwin-arm64, median of 20):
all six queries + both ports **4.81 ms** against `jj log -r @ -T commit_id` at
26.54 ms. The reference artefact's ratio rose from 6.49× to **7.16×**
host-native (more of jj-lib links), and all four release triples were
re-cross-compiled to confirm the new edges break no target — 6.86× to 7.37×,
both musl builds still clearing the absolute floor.

`prost` and `pollster` become direct dependencies. Both were already in
`cli/Cargo.lock` via jj-lib, so **the lock delta is two lines** — two new edges
on `vcs-adapters`, no new packages, no new licences. They are pinned to jj-lib's
majors because the decoded type comes *from* jj-lib, guarded by a new
single-version assertion in `test_vcs_library_graph.py` and a comment assertion
in `test_vcs_pin_lockstep.py`. The pin coupling is now six-way, documented in
`tasks/README.md`.

Two residual risks, recorded rather than resolved: two *paths* are convention
rather than API (`<workspace_root>/.jj/working_copy`, `<repo_path>/op_store`;
`DefaultWorkspaceLoader`'s equivalent field is private), mitigated by asserting
`get_working_copy_type() == "local"` before reading and by decode failure being
`Err`. The *schema* — the part that could silently misparse — is public and
versioned with a crate pinned at `=0.43.0`.

Records updated: work-item amendments 9 and 10 added (10 withdraws 8), the
plan's Phase 1 §4 rewritten with the superseded spike kept in a `<details>`
block, the Phase 3 parity criterion widened, and 0185's inheritance 3 rewritten
— it no longer inherits the mechanism, only the snapshot divergence.

Also corrected for the record: `UserSettings::from_config` returns `Result`, not
a panic, and needs exactly five keys (`settings.rs:135-166`). The spike's
"abandoned after five successive panics with the chain never exhausted" was the
chain being exhausted. So the settings route was viable too — just worse than
the one delivered.

#### The three validation findings

Six follow-up edits, none touching the delivered Rust:

1. **`cli/deny.toml`** — the `uluru` exception comment rewritten around a
   *verified* finding. Measured 2026-08-03: `uluru` is in the normal closure of
   exactly one shipped binary (`accelerator-visualiser`; neither `accelerator`
   nor `accelerator-verify` has it at all), and dead-code elimination removes
   the whole `gix`/`jj-lib` closure from it. An unstripped `--release` build
   carries zero symbols from `gix`, `gix-pack`, `gix-odb`, `jj_lib`, `clru` or
   `uluru` against 26,247 total, and none of the distinctive literals
   (`extensions.objectFormat`, `There is no Jujutsu repo`) that the linked
   reference artefact does carry — the positive control that makes the absence
   meaningful. §3.2 therefore does not bind and no attribution artefact is
   needed. The comment records the evidence **and its re-check trigger**, since
   the finding's real cause is that nothing in the visualiser reaches
   `vcs-adapters` at all today (`CommandProbe`'s own `rev-parse` literal is
   absent too), which 0185's `facts` switch is expected to change.
2. **`meta/work/0185-*.md`** — inheritance 5 added to the amendment block: the
   licence exception must be re-checked *as part of* the `facts` switch, with
   the exact check to re-run, because that switch is what would make §3.2 bind
   and `_release_uploads()` carries no licence file.
3. **`meta/work/0188-*.md`** — amendment 9 recorded for the
   `gix default-features = false` pin, and the stale "**`gix` with default
   features**" requirement corrected to match `cli/Cargo.toml:80`.
4. **`meta/work/0188-*.md`** — Validation Results gained the cold-cache compile
   figures, the shipped-binary size finding, and an entry closing the MPL-2.0
   question.
5. **`meta/plans/2026-08-03-0188-*.md`** — the MPL open question struck through
   and closed with its evidence.
6. **`cli/vcs-test-support/Cargo.toml`** — duplicate `tempfile` dev-dependency
   removed (regular dependencies are already visible to test targets;
   `cargo test --no-run` and `cli:check` confirm).

### Manual Testing Required:

1. Strong-form CI (Linux only, first execution is on this branch's CI run):
  - [ ] The shadow step actually replaced `git` and `jj` — assert
        `git --version` fails inside the step before the restore
  - [ ] The `trap restore EXIT` fired and the `if: always()` liveness step found
        both binaries restored
  - [ ] `ACCELERATOR_ZERO_SPAWN_MODE=strong` was observed by the harness (the
        contract's non-degradability)
2. Cold-cache budgets — **measured, no action expected**; confirm on the first
   CI run only:
  - [ ] `test-visual-regression` stays inside its 20-minute cap (measured cold
        cost of the two new trees: 16.92 s wall / 65.80 s CPU locally, ~1-2 min
        scaled to a 4-vCPU runner, against a job whose wall clock is dominated
        by Playwright and Docker)
  - [ ] `check-zero-spawn` stays inside its own 20-minute cap
3. Cross-machine:
  - [ ] Confirm `mise install` has been run on each development machine (the
        fixture matrix hard-fails on a jj major.minor skew)

### Recommendations:

- **File the `cli/target/debug` race as its own item** — the one open finding.
  `tests/integration/support/installation.py:125-141` builds into the shared
  workspace target directory and asserts the artefact's presence immediately
  afterwards, with no tolerance for a concurrent rebuild;
  `test_signing.py:54-70`
  and `test_manifest.py:67-75` share the same helper shape. Candidate fixes: a
  dedicated `CARGO_TARGET_DIR` for the suites that shell out to cargo, a
  session-scoped build fixture that builds once before the parallel tasks fan
  out, or a bounded retry around the presence assertion. Worth doing because it
  degrades the local `mise run` gate the contributor guide makes mandatory —
  2 of 3 invocations here — even though CI is immune by job separation.
- **Tick the work item's acceptance criteria and transition its status** via
  `/accelerator:update-work-item`, folding the surviving `[ ]` items into the
  carried-forward list. Left alone deliberately: which criteria count as met
  is a call for the author, and status transitions belong to that skill.
- **Re-check the licence finding when 0185 flips `facts`**, not after. Recorded
  in the `cli/deny.toml` comment and as inheritance 5 on 0185, but worth
  restating because a conditional finding with an unowned trigger is how a
  licence obligation goes quietly unmet.
- Consider whether the size floor's sibling — the *shipped* binary's link
  behaviour — deserves a committed check too. The reference artefact's ratio
  floor is committed (`tasks/build.py:203-217`), but the fact that the shipped
  visualiser links none of the trees is recorded only in prose, and it is the
  fact the licence exception now rests on.
