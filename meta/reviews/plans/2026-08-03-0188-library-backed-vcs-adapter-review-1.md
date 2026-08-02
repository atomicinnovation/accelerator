---
type: plan-review
id: "2026-08-03-0188-library-backed-vcs-adapter-review-1"
title: "Plan Review: Library-Backed VCS Adapter over gix and jj-lib Implementation Plan"
date: "2026-08-03T11:42:20+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: "work-item:0188"
target: "plan:2026-08-03-0188-library-backed-vcs-adapter"
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: [architecture, code-quality, test-coverage, correctness, compatibility, portability, safety, security]
review_number: 1
review_pass: 4
tags: [rust, vcs, dependencies, gix, jj-lib]
last_updated: "2026-08-03T15:30:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Library-Backed VCS Adapter over gix and jj-lib

**Verdict:** REVISE

This is an exceptionally well-evidenced plan — the oracle mapping is empirical
rather than inferred, every guard carries a non-vacuity control, the phases are
genuinely independently mergeable, and the risk-isolation framing (unwired
adapter, deletion-only rollback) is the right response to adopting two pre-1.0
dependency trees. The findings cluster in three places rather than being spread
thin: the strong-form CI job's shadow window is drawn far too wide and would
prevent its own suite from running; `VcsProbe::revision` is the one method with
no design and its jj half appears unreachable under the plan's own crate-wide
`UserSettings` ban while Phase 3 demands byte-identical `RepoFacts`; and the
oracle mapping — the plan's central artefact — has incomplete tables, one cell
that contradicts its siblings, and a verdict column stating an inference its own
amendment refutes. Beneath those, a consistent thread runs through six lenses:
the plan's *declared* surfaces (pins, licences, graph invariants) are rigorously
policed, while its *runtime and environmental* surfaces (error channels,
canonicalisation, environment immunity, cross-target verification) are asserted
rather than constructed.

### Cross-Cutting Themes

- **The shadowed CI window encloses everything, not just the code under test**
  (flagged by: test-coverage, architecture, portability, safety) — moving `git`
  and `jj` aside before a step that compiles the workspace *and* builds 24
  fixtures with those same binaries breaks the suite, risks a vergen/cargo
  build-script failure, and can let `mise` silently reinstall `jj` and make the
  assertion vacuous.

- **The pup `allowed_only` list contradicts the crate's own contract**
  (flagged by: architecture, code-quality, correctness, compatibility) — the
  permit list omits `tracing`, which `VcsProbe::revision`'s documented contract
  ("an adapter is expected to log the failure") and `CommandProbe`'s six `warn!`
  sites both require; it permits `kernel::Error`, which is not a dependency.

- **`Option`-only signatures with no error channel** (flagged by: architecture,
  compatibility, security, code-quality) — "no repository here" and "the pinned
  pre-1.0 library could not parse this repository" collapse to the same `None`,
  and the plan drops `CommandProbe`'s time cap, crash isolation and warn-logging
  without replacement.

- **Path canonicalisation is specified per-query, not at the boundary**
  (flagged by: correctness, code-quality, portability) — `dual_roots` equality
  is the colocated-vs-nested discriminator, yet its git side comes from gix's
  own reconstruction and its jj side from a pass-through `workspace_root()`;
  on macOS the `/var` → `/private/var` split can make them differ spuriously.

- **The two new trees enter the shipped visualiser's closure, verified on one
  triple of four** (flagged by: architecture, compatibility, portability,
  safety) — `vcs-adapters` → `corpus-adapters` → `visualiser/server` is a
  normal-dependency chain, so gix and jj-lib must cross-compile for all four
  release targets, and no `check-*` job cross-compiles at all.

- **Test-fixture binaries acquire release-artefact shape** (flagged by:
  architecture, safety, security, compatibility, portability) — adding them to
  `_CLI_RELEASE_BINARIES` stages them into `dist/release/` and puts fixture
  build failures on the release critical path.

- **Environment immunity and structural zero-spawn are both narrower than
  claimed** (flagged by: security, correctness) — the scrub invariant was tested
  against 2 of the 10+ variables `CommandProbe` scrubs, and cargo-pup's
  `RestrictImports` constrains first-party `use` paths only, saying nothing
  about gix's `attributes`/`blob-diff` filter machinery.

### Tradeoff Analysis

- **Structural enforcement vs. contract compliance**: the pup `allowed_only`
  list buys a closed-world zero-spawn property, but as drafted it forbids the
  logging the port contract mandates and imposes an invisible single-item-import
  rule the repo already rejected once (`cli/pup.ron:93-98`). Recommendation:
  add `tracing`, and consider expressing the rule as `denied`-only — the
  zero-spawn guarantee comes from the deny clause, not the permit list.

- **Exact pins vs. patch agility**: `=0.85.0`/`=0.43.0` buy a reproducible
  single graph, but under `unmaintained = "all"` + `yanked = "deny"` they make
  `advisories.ignore` the cheapest response to an upstream advisory in a
  56-package closure that no code calls. Recommendation: keep the exactness in
  `Cargo.lock`, relax the manifest to patch-permitting, and document the
  break-glass procedure.

- **Test rigour vs. CI cost**: one table-driven test per query rebuilds the full
  ~19-fixture matrix six times per leg, on a repo with a known fixture-flake
  history. Recommendation: invert to one test per (fixture, start directory)
  pair — same per-cell traceability, roughly a sixth of the fixture builds.

### Findings

#### Critical

- 🔴 **Test Coverage + Architecture + Portability + Safety**: The strong-form CI
  shadow window encloses fixture construction and compilation
  **Location**: Phase 4 §3 (The strong-form CI job)
  The job moves `git` and `jj` aside, then runs a step that must build 24
  fixtures *using those binaries* and compile the `cli` workspace (whose
  `vergen-gitcl` build script shells out to `git`). As written the suite cannot
  construct a single fixture, and `mise run` may reinstall the shadowed `jj`
  and make the assertion vacuous.

- 🔴 **Correctness + Code Quality + Test Coverage**: `VcsProbe::revision` has no
  specified mechanism and its jj half conflicts with the `UserSettings` ban
  **Location**: Phase 1 §4 (`InProcessProbe`); Phase 3 §2 (injection seam)
  `revision` is sketched as `/* gix / jj-lib */` and never returned to, yet
  Phase 3 requires identical `RepoFacts` against `CommandProbe`'s
  `jj log -r @ -T commit_id`. Reading jj's `@` commit id in-process needs a
  `UserSettings`, which Phase 2's guard forbids crate-wide; git HEAD is not an
  escape (pure-jj has none, colocated exports `@-`). Phase 1 also ships `kind`,
  `revision` and `repository_root` with no oracle rows and no test.

- 🔴 **Safety + Portability**: The `stubs` shadow list is specified as if the
  test crate itself mutates absolute system binaries
  **Location**: Phase 3 §1 (`stubs` module); Phase 4 §3
  The crate is described as resolving and shadowing absolute paths "recording
  which paths it could not shadow", including `/opt/homebrew/bin/git` — a
  user-writable location — while the actual privileged move lives in workflow
  YAML. `test:integration:zero-spawn` is reachable from the bare `mise run`
  every contributor is told to run before pushing.

#### Major

- 🟡 **Architecture + Code Quality + Correctness + Compatibility**: The pup
  `allowed_only` list omits `tracing` and permits an inert `kernel::Error`
  **Location**: Phase 1 §3 (The two-clause import rule)
  The first `use tracing::warn;` in `library.rs` fails `pup:check`, forcing the
  module to be the crate's only silently-failing adapter.

- 🟡 **Architecture + Compatibility + Security**: No error channel anywhere in
  the new surface
  **Location**: Phase 2 §1 (The query surface)
  All six queries return `Option` with no error arm, so a corrupt object store,
  a locked jj repo or an unparseable repository reads identically to "no
  repository here" — and the plan drops `CommandProbe`'s 10s cap, crash
  isolation and warn-logging with nothing in their place.

- 🟡 **Correctness**: Query 6's "roots equal → colocated" verdict contradicts
  Amendment 3 and the `classify_checkout` table
  **Location**: Oracle Mapping — Query 6 (`CR`, `NGJ-o` rows)
  Both `CR` and `NGJ-o` have equal dual roots yet classify as `main`. A
  classifier built from this column would misclassify every real colocated main
  repository — the commonest shape in the wild.

- 🟡 **Correctness**: Three mapping tables have missing cells
  **Location**: Oracle Mapping — Queries 5 and 6, `classify_checkout` records
  Query 6 covers 17 of 24 pairs, Query 5 omits `NGPJ-i` (which is a jj fixture,
  so the catch-all does not reach it), and the `classify_checkout` table omits
  five fixtures — breaking the traceability criterion the whole empirical
  deferral exists to serve.

- 🟡 **Correctness**: `PJG-i`'s recorded `--git-common-dir` contradicts its
  structurally identical siblings
  **Location**: Oracle Mapping — Query 2 (`PJG-i` row)
  `.git` where `NJG-i` records `../.git`. Under the plan's own absolutisation
  rule this makes the shell oracle take the `colocated` arm rather than
  `nested-jj-in-git`, so "no other divergence was observed" cannot stand.

- 🟡 **Correctness + Code Quality + Portability**: Canonicalisation specified for
  two queries only; `dual_roots` compares two different regimes
  **Location**: Phase 2 §1 (implementation notes); Oracle Mapping — Query 6
  `git_dir`, `main_worktree_root`, `superproject`, `jj_workspace_root` and both
  `DualRoots` arms are silent. The recorded `CG` equality is protected only by
  the fixture-construction rule, which does not hold in production.

- 🟡 **Test Coverage + Correctness**: Every gix-based absence cell rests on an
  unstated precondition
  **Location**: Phase 2 §2 (fixture matrix); Oracle Mapping preamble
  `gix::discover` reads no environment, so `GIT_CEILING_DIRECTORIES` cannot fence
  it — the `None` expectations hold only because no ancestor of `$TMPDIR`
  carried a `.git` on the measuring host.

- 🟡 **Correctness**: `rposition` on `modules` components misresolves a submodule
  whose own path contains `modules`
  **Location**: Phase 2 §1 (`superproject`)
  A submodule at `modules/foo` yields `git_dir() == $super/.git/modules/modules/foo`;
  `rposition` selects the inner one and `gix::open` on its parent fails, silently
  returning `None` where git returns `$super`.

- 🟡 **Correctness**: No fixture for a jj secondary workspace nested inside its
  own main repository
  **Location**: Fixture keys table
  This is the shape this repository uses daily (`workspaces/<name>`), and it is
  precisely where differing dual roots point at the wrong `classify_checkout`
  arm — `jj-secondary`, not `nested-jj-in-git`.

- 🟡 **Correctness**: `kind() == Kind::LinkedWorkTree` cannot represent a
  repository that is both a submodule and a linked worktree
  **Location**: Phase 2 §1 (`worktree.linked`)
  `Kind` is mutually exclusive; `git worktree add` from within a submodule has
  unequal `--git-dir`/`--git-common-dir` but `kind()` can report only one fact.
  No fixture combines them.

- 🟡 **Correctness**: Query 2's `main_worktree_root` column has no matching
  oracle for the bare and submodule fixtures
  **Location**: Oracle Mapping — Query 2 (`BARE`, `SM-1`, `SM-2`)
  `realpath $(dirname <common-dir>)` gives `$BASE/super/.git/modules` for `SM-1`
  where the library records `$BASE/super/mid`, yet every Verdict cell reads
  "agree" — one verdict is carried for five independent columns.

- 🟡 **Architecture + Compatibility + Portability + Safety**: The trees enter the
  shipped visualiser's closure; one of four release triples verified
  **Location**: Phase 1 §1 (pins); Phase 4 §2 (musl staging)
  `vcs-adapters` → `corpus-adapters` → `visualiser/server` is a normal-dependency
  chain, `cli_cross_compile` iterates four triples, and no `check-*` job
  cross-compiles — so a break lands on `main` and first fails in the release job.

- 🟡 **Architecture + Safety + Security**: Test-fixture binaries added to
  `_CLI_RELEASE_BINARIES`
  **Location**: Phase 4 §2 (musl staging)
  This stages them into `dist/release/`, puts fixture build failures on the
  release critical path, and places the guard against the story's headline
  false-pass (dead-code elimination) in the one stage that never runs on a PR.

- 🟡 **Test Coverage**: The cargo-pup rule's non-vacuity is a manual
  add-then-revert, though a committed harness exists
  **Location**: Phase 1 §3 Success Criteria
  `tests/integration/pup/test_import_rule.py` already drives the shipped
  `pup.ron` against synthetic workspaces with a positive control for exactly the
  "rule matched nothing" failure — and 0169 is expected to rename the module.

- 🟡 **Test Coverage**: The size-floor criterion is listed as automated with no
  named mechanism
  **Location**: Phase 4 §4 Success Criteria
  No test file, no `tasks/` check, no mise leaf — so the guard against dead-code
  elimination becomes a number written once into Validation Results.

- 🟡 **Test Coverage**: The scrub-invariant comparison runs its two arms through
  different execution paths
  **Location**: Phase 2 §3
  The poisoned arm runs in a child binary, the clean arm presumably in-process,
  so a formatting mismatch fails 144 cells and a shared absence rendering passes
  vacuously. The stated rationale ("because nextest is process-per-test") also
  reads backwards — that property makes `set_var` safer, not less safe.

- 🟡 **Compatibility**: The lockstep test compares two declarations, not the `jj`
  binary that builds the fixtures
  **Location**: Phase 1 §5; Manual Testing step 3
  The planning session itself hit this: `jj 0.42.0` from Homebrew was on `PATH`
  because `mise.toml` was untrusted. The mitigation offered is a manual step.

- 🟡 **Compatibility**: The `git` CLI is an unpinned oracle for the whole matrix
  **Location**: Oracle Mapping preamble; Phase 1 §1
  Every cell was recorded against git 2.54.0 and every fixture is built by
  whatever `git` is on `PATH`; `mise.toml` has no `git` entry. The coupling is
  five-way, not the three-way the pin comment describes.

- 🟡 **Compatibility + Safety**: "Regenerate the lock" re-resolves crates the
  workspace deliberately fenced
  **Location**: Migration Notes; Phase 1 §1
  `reqwest = "=0.12.28"` (pinned so a patch cannot re-scope DNS onto
  getaddrinfo), `rustls = "=0.23.41"`, and two RUSTSEC ignores tied to
  `hickory-proto` all sit behind fences a wholesale regeneration floats — and it
  lands disguised as merge cleanup.

- 🟡 **Compatibility**: No non-default git repository formats in the matrix
  **Location**: Phase 2 §2; Fixture keys
  All 24 shapes are files-backend + sha1. Reftable and sha256 repositories —
  which gix 0.85 supports only partially — would silently return `None` from
  `revision` after 0185's switch, where `CommandProbe` succeeds today.

- 🟡 **Safety**: The rollback claim omits the workflow, mise and lock footprint,
  and one revert ordering bricks CI
  **Location**: Migration Notes
  Reverting the job while leaving the `needs: check-zero-spawn` edge is a
  workflow validation error that stops the *whole* Main workflow — not the
  single-module `jj restore` the notes describe.

- 🟡 **Safety**: `unmaintained = "all"` over a 56-package expansion can block all
  releases, with no break-glass documented
  **Location**: Performance Considerations; Phase 1
  One upstream advisory anywhere in a closure no code calls turns
  `check-supply-chain` red for every unrelated PR, and that job is in
  `prerelease.needs`.

- 🟡 **Safety + Portability**: Shadowing inside the cached mise install tree can
  poison the job's own cache
  **Location**: Phase 4 §3
  `$HOME/.local/share/mise/installs/jj/<version>/jj` is inside the tree
  `mise-action` saves on its post step, which runs after the restore — so an
  incomplete restore persists a broken toolchain to `mise-zero-spawn-v1`.

- 🟡 **Safety**: `if: always()` is necessary but not sufficient for the stated
  restore guarantee
  **Location**: Phase 4 Manual Verification
  A partial shadow leaves a straight-line `set -e` restore aborting on the first
  missing source; cancellation gives only a bounded grace window. The claim holds
  only because runners are ephemeral — an assumption never stated.

- 🟡 **Security**: The scrub invariant is far narrower than the "uniformly
  immune" claim
  **Location**: Key Discoveries #6; Phase 2 §3
  Verified over `GIT_DIR`/`GIT_COMMON_DIR` only; `CommandProbe` scrubs seven and
  forces three. `gix::open`'s default permissions do consult
  `GIT_OBJECT_DIRECTORY`/`GIT_ALTERNATE_OBJECT_DIRECTORIES` and system/global
  config, and `is_bare()` reads `core.bare` from config.

- 🟡 **Security**: The pup rule is not a structural zero-spawn guarantee
  **Location**: Phase 1 §3; Key Discoveries #7
  `RestrictImports` sees first-party `use` paths only — an inline
  `std::process::Command::new` is unaffected, `^crate(::|$)` reaches the
  retained `CommandProbe`, and nothing constrains gix's `attributes`/`blob-diff`
  filter and external-command machinery, which is repository-config-driven.

- 🟡 **Security + Compatibility**: Exact `=` pins block patch remediation, and
  the manifest contradicts the graph test
  **Location**: Phase 1 §1 and §5
  `=0.85.0` forbids every `0.85.x` patch while the committed test asserts
  `0.85.\d+`. Under `yanked = "deny"` the cheapest response to an advisory
  becomes an `ignore` entry.

- 🟡 **Security**: The no-TLS assertion is host-target-scoped
  **Location**: Phase 1 §5
  `cargo tree` with no `--target` evaluates only the host triple, whereas
  `cli/deny.toml:11-17` enumerates five deliberately. `[bans].deny` — the
  target-aware, fail-closed mechanism — is not extended at all.

- 🟡 **Security + Compatibility**: The uluru MPL-2.0 justification answers §3.1,
  not §3.2
  **Location**: Phase 1 §2
  "We ship no modifications" addresses modified Source Form. The obligation that
  bites is the Executable Form notice, and the release payload carries no
  third-party licence artefact.

#### Minor

- 🔵 **Architecture**: The three query value types are declared in the adapter
  crate, but ADR-0053 and the `vcs_domain_imports_only_permitted` pup rule mean
  0169's domain port structurally cannot reference them — a guaranteed later
  move that Phase 5's hand-off does not record.
  **Location**: Phase 2 §1; Phase 5 §3

- 🔵 **Architecture + Code Quality**: A dev-dependency cycle
  (`vcs-adapters` tests → `vcs-test-support` → `vcs-adapters`) is created but
  never stated, and it weakens the stated rationale for the separate crate.
  **Location**: Phase 3 §1

- 🔵 **Architecture + Code Quality**: Three marker-walk / marker-kind
  implementations duplicate `MarkerWalkRoot::discover` and `CommandProbe::kind`
  ten lines away in the same crate, including the "never test the filesystem
  root" rule documented only on `MarkerWalkRoot`.
  **Location**: Phase 1 §4

- 🔵 **Code Quality**: The `UserSettings` text guard will flag its own rationale
  comment (the model it copies matches raw lines), and the plan never says why
  the one-line pup `denied` clause was rejected in favour of a new Python lint,
  two `__init__.py` registrations, a mise leaf and two `test_mise.py` constants.
  **Location**: Phase 2 §4

- 🔵 **Code Quality**: The `allowed_only` clause reintroduces a pup pattern
  `cli/pup.ron:93-98` already carries a comment warning against, imposing an
  invisible single-item-import rule enforced by a message naming neither the
  import nor the real cause.
  **Location**: Phase 1 §3

- 🔵 **Code Quality + Test Coverage**: The most intricate fixture builders are
  written in Phase 2 in a location Phase 3 immediately moves them out of, and
  Phase 2's own hermeticity criterion depends on a `hermetic` module that does
  not exist until Phase 3.
  **Location**: Phase 2 §2; Phase 3 §1

- 🔵 **Code Quality**: The Phase 2 interim helper binary's relationship to the
  two Phase 4 `[[bin]]` targets is never stated — potentially three
  near-identical composition roots in one crate.
  **Location**: Phase 2 §3; Phase 4 §1

- 🔵 **Test Coverage**: One table-driven test per query rebuilds the ~19-fixture
  matrix six times per leg, on a repo with a known fixture-flake history;
  inverting to one test per (fixture, start) pair keeps traceability at a sixth
  the cost.
  **Location**: Phase 2 §2

- 🔵 **Test Coverage + Security**: No malformed or adversarial fixture anywhere —
  a `.jj/repo` pointer to a deleted path, a `.git` file whose gitdir target is
  gone, a truncated pack, or a hostile `.git/config` setting `core.pager` /
  `filter.*.clean` / `include.path`.
  **Location**: Testing Strategy; Phase 2 §2

- 🔵 **Test Coverage**: Neither Phase 1 verification list checks that the `gix`
  pin comment and the `uluru` exception comment are present, though the work
  item requires both.
  **Location**: Phase 1 §5

- 🔵 **Test Coverage**: The `.git`-as-file worktree case is required to produce
  identical `RepoFacts` from both implementations *and* to possibly diverge to
  `colocated`; the plan never resolves which assertion governs.
  **Location**: Phase 3 §2

- 🔵 **Correctness**: The `NJG` fixture key says "colocated inner main" but
  Query 5 records `.jj/repo` as a *file* (secondary) and Query 2 shows no inner
  `.git` — a builder written to the key would produce wrong values for every
  `NJG-*` cell.
  **Location**: Fixture keys table

- 🔵 **Correctness**: Pair count is 24 in one place and 25 in another, and
  `--show-superproject-working-tree` is given one absence signal in the table
  while Query 3 records two.
  **Location**: Key Discoveries #6; absence-signal table

- 🔵 **Compatibility**: The new member's manifest omits `edition`/`rust-version`
  inheritance; a missing `rust-version` drops it out of the MSRV-aware fallback
  Key Discovery 11 identifies as load-bearing.
  **Location**: Phase 3 §1

- 🔵 **Portability**: The GitHub-hosted-runner and mise install-layout
  assumptions (passwordless sudo, `/usr/bin/git`, a versioned installs path) are
  encoded rather than derived or documented as coupling.
  **Location**: Phase 4 §3

- 🔵 **Portability**: The `hermetic` module is under-specified relative to the
  environment the mapping was measured in — `GIT_CONFIG_NOSYSTEM` is a boolean,
  not a path, and is the only thing suppressing the system gitconfig, which
  differs between ubuntu and macOS.
  **Location**: Phase 3 §1; Phase 2 Success Criteria

- 🔵 **Portability**: All six cost figures are darwin-arm64 while the shipped
  artefact is static musl, so 0169 inherits a gate with no Linux datapoint.
  **Location**: Phase 4 §4

- 🔵 **Safety**: `accelerator-visualiser`'s musl-static size is never recorded
  before/after, so a future LTO or profile change adding ~2 MB to a fetched,
  hashed artefact has no baseline to fail against.
  **Location**: Phase 1 §1

- 🔵 **Safety**: `test-visual-regression` carries `timeout-minutes: 20` and
  depends on `build:server:dev`, which now compiles both new trees — and this
  plan guarantees a cold cache by changing the lock.
  **Location**: Phase 1 Success Criteria

- 🔵 **Safety**: Phase 5 rewrites 0185's Summary, Context, Assumptions, Technical
  Notes and acceptance criteria in place, and removes an assumption — where 0188
  itself used append-only dated amendment blocks.
  **Location**: Phase 5 §2

- 🔵 **Security**: No build-script or proc-macro policy for the 56-package
  expansion, though the release workflow already documents build-script trust as
  a live concern.
  **Location**: Phase 1 §1 and §5

#### Suggestions

- 🔵 **Architecture + Code Quality**: Split `library.rs` into `library/git.rs`
  and `library/jj.rs` — the pup matcher already covers submodules — so the
  pre-1.0 jj-lib bet the story exists to isolate is isolated inside the module
  too.

- 🔵 **Architecture**: Extract the superproject path derivation as a pure
  function unit-testable against string paths, so the story's only hand-rolled
  edge-case logic is not gated behind its most expensive fixtures.

- 🔵 **Architecture**: Consider `in_process` over `library` for the module name
  before it is baked into `pup.ron` and 0169's hand-off, and document the
  division of labour between cargo-pup (import prohibitions) and the source
  guard (usage prohibitions imports cannot express).

- 🔵 **Code Quality**: Rename `JjRepository` → `JjRepositoryFacts` and replace
  `secondary: bool` with a two-variant `JjWorkspaceRole`, since 0169 builds its
  domain vocabulary directly on these names.

- 🔵 **Compatibility + Portability + Security**: Stage and measure the fixture
  binaries outside `dist/release/` via a separate constant, and assert
  `_release_uploads()` never contains a fixture name.

- 🔵 **Compatibility**: Extend the graph test to assert no package in
  `cli/Cargo.lock` declares a `rust-version` above 1.90.0 — catching the
  `kstring` class of trap directly rather than relying on a resolver preference.

### Strengths

- ✅ The oracle mapping is established by measurement against real oracles, per
  (fixture, start directory) pair, with per-oracle absence signals tabulated
  rather than assumed uniform — and the tests are written before the queries.
- ✅ The three-walk discovery is a genuine insight caught by measurement; an
  earlier framing would have made 0169's nested arms unimplementable.
- ✅ Every guard carries a designed-in non-vacuity control:
  `discover_with_environment_overrides` for the poison, an unbounded
  `gix::discover` for the boundary, and the "no marker written **and** values
  match" rule that closes the degrade-to-`None` false pass.
- ✅ The adapter ships genuinely unwired — `vcs_adapters::facts` keeps naming the
  retained pair — so no consumer can regress at runtime regardless of what the
  new module does.
- ✅ Phase ordering is deliberate and correct: the entire dependency-policy
  surface lands in Phase 1 so an objection is reviewable before any query logic
  exists.
- ✅ Non-obvious library behaviours (`ceiling_dirs` structurally unable to bound
  the walk, unnormalised `common_dir()`, `main_repo()` returning the submodule,
  `repo_path()` pre-canonicalised while `workspace_root()` is not) are recorded
  where an implementer will hit them.
- ✅ The `CR`/`CG` split correctly identifies that the shell's `colocated` arm
  needs `jj_secondary && git_worktree`, which a single fixture row would have
  papered over.
- ✅ Dependency selection is portability-aware: default features chosen so no TLS
  stack enters the graph, no `git2`/`libgit2-sys`, no duplicate gix versions, and
  MSRV-aware resolution identified as load-bearing.
- ✅ The size criterion was recalibrated against measurement rather than left to
  fail at verification time, and Phase 4 is explicitly required to re-measure
  against the delivered two-binary shape.
- ✅ Shared-artefact contention (`Cargo.lock`, `deny.toml`, `pup.ron`,
  `tasks/build.py` vs 0187) is identified with a stated resolution rule rather
  than an implicit ordering assumption.
- ✅ Fixture temp dirs use owned `TempDir` guards with immediate canonicalisation
  and explicitly reject `NamedTempFile`/`.persist`, matching the convention
  adopted after the nextest pid-reuse collisions.
- ✅ The plan declines to reproduce the shell's `GIT_DIR` scrub asymmetry,
  closing an ambient-environment redirection vector rather than porting it.

### Recommended Changes

1. **Redraw the strong-form CI job's shadow window** (addresses: the critical
   shadow-window finding, the mise-cache and `if: always()` findings)
   Build the fixtures and compile the test binaries in steps *before* the shadow
   step; have the post-shadow step execute prebuilt binaries against prebuilt
   fixtures only. Move shadowed binaries to `$RUNNER_TEMP`, not within the cached
   mise tree. Make shadow and restore idempotent per path, add a post-restore
   `git --version && jj --version` assertion, and add `timeout-minutes`.

2. **Resolve `VcsProbe::revision` before Phase 1 starts** (addresses: the
   critical revision finding)
   Either name a settings-free jj-lib path and prove it in a short spike, or
   narrow the `UserSettings` ban to the detection module, or declare jj
   `revision` out of scope for 0188 and narrow Phase 3's parity criterion to
   `root`/`name`/`kind`. Add oracle rows for `kind` and `revision` and test the
   port methods within Phase 1.

3. **State that `vcs-test-support` never writes outside its own temp dirs**
   (addresses: the critical `stubs` finding)
   All privileged mutation lives in the CI workflow step, guarded on
   `GITHUB_ACTIONS`; the crate only *reports* absolute paths it cannot control.
   Give the two halves an explicit contract (`ACCELERATOR_ZERO_SPAWN_MODE=strong`
   plus the shadowed list) so the harness hard-fails when strong mode is claimed
   but a target is still executable.

4. **Add `tracing` to the pup permit list and drop `kernel::Error`** (addresses:
   the pup-list finding, and partly the no-error-channel finding)
   State per query which arms warn-log and which are legitimate absence,
   mirroring `CommandProbe`'s labelled `warn!` sites. Consider expressing the
   whole rule as `denied`-only to avoid the grouped-import tax the repo already
   rejected once.

5. **Decide the error channel explicitly** (addresses: the no-error-channel
   finding, the adversarial-fixture finding, the containment finding)
   Either `Result<Option<T>, _>` or a recorded decision that failures map to
   absence *and* are warn-logged. Record which library error conditions are
   deliberately mapped to `None`, and note the loss of `CommandProbe`'s time cap
   and crash isolation as a hand-off constraint to 0185.

6. **Fix and complete the oracle mapping** (addresses: the Query 6 verdict, the
   missing cells, the `PJG-i` cell, the `NJG` key, the count discrepancy)
   Re-run `--git-common-dir` from `$BASE/PJG/sub`; replace Query 6's verdict
   column with raw equality plus a note that equality is necessary but not
   sufficient; complete Query 5 (`NGPJ-i`), Query 6 (seven fixtures) and the
   `classify_checkout` table; correct the `NJG`/`PJG` keys; reconcile 24 vs 25.

7. **State one canonicalisation contract at the module boundary** (addresses: the
   canonicalisation finding)
   Every `PathBuf` the six queries and both port methods return is canonicalised,
   implemented at a single choke point and documented on the module — then
   re-check whether `CG`'s dual-root equality survives with the fixture built at
   an uncanonicalised path.

8. **Add the missing fixtures** (addresses: the `workspaces/*` shape,
   submodule-with-`modules`-path, submodule+worktree, adversarial, non-default
   format)
   Prioritise the jj-secondary-inside-its-own-main shape — it is what this repo
   runs in daily and it is where differing dual roots point at the wrong arm.

9. **Assert the fixture-matrix preconditions** (addresses: the absence-cell
   finding, the git-pin finding, the jj-lockstep finding)
   Walk from the temp base to `/` and fail if any ancestor carries `.git`/`.jj`;
   assert `jj --version` matches the compiled-in `jj-lib` version; record the
   observed `git --version` and pin the format-relevant knobs via `git -c`.

10. **Take the fixture binaries off the release path** (addresses: the
    `_CLI_RELEASE_BINARIES` finding, the DCE-guard finding)
    Give them their own constant and staging directory, and assert the size ratio
    and absolute floor inside a task that a `check-*` job runs — so the guard
    against the headline false-pass runs on the PR that could break it.

11. **Verify all four release triples in Phase 4** (addresses: the cross-target
    finding) and record the `accelerator-visualiser` musl-static size before and
    after Phase 1.

12. **Tighten the supply-chain surface** (addresses: the host-target TLS
    assertion, the exact-pin remediation path, the MPL §3.2 finding, the
    build-script finding)
    Move the transport prohibition into `[bans].deny` so all five configured
    triples are evaluated; reconcile the manifest pin with the `0.85.\d+`
    assertion and document the yank/advisory break-glass in
    `tasks/README.md`; rewrite the uluru comment to address the Executable Form
    notice; snapshot the build-script-carrying crates in the graph test.

13. **Specify the minimal lock operation on contention** (addresses: the
    lock-regeneration finding) — `cargo update -p gix -p jj-lib`, never
    `generate-lockfile`, with a review step requiring the lock diff to contain
    only the new closure.

14. **Document the revert order in Migration Notes** (addresses: the rollback
    finding) — workflow `needs` edge before the job; `_CLI_RELEASE_BINARIES`
    before the `[[bin]]` targets; mise leaves together with their `test_mise.py`
    assertions; the lock regenerated rather than reverted.

15. **Commit the pup rule's non-vacuity** (addresses: the manual-non-vacuity
    finding) — add probe cases to `tests/integration/pup/test_import_rule.py`
    including a positive control and the grouped-import case.

16. **Create `cli/vcs-test-support` at the start of Phase 2** (addresses: the
    write-then-move finding, Phase 2's unverifiable hermeticity criterion) with
    `fixtures` and `hermetic`; leave `stubs` and the cross-crate proof to Phase 3.

17. **Invert the query test table** (addresses: the fixture-runtime finding) —
    one test per (fixture, start directory) pair asserting all six values, so
    each fixture is built once and failures localise to a shape.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: An unusually well-grounded plan: the ports in `cli/vcs` are left
untouched, the new adapter is unwired by design, every phase is independently
mergeable and revertible, and the risky decisions are deliberately isolated. The
structural weaknesses are concentrated in three places: the query surface's
vocabulary and error channel are frozen in the adapter layer where ADR-0053 says
domain vocabulary does not belong; the heavy dependency trees land as *normal*
dependencies of a crate transitively in the shipped visualiser's build graph;
and the guard against the story's stated headline false-pass is wired into the
release pipeline, the one stage that never runs on a change.

**Strengths**: `cli/vcs` genuinely untouched with dependency direction staying
inward; zero-spawn enforced structurally rather than only by assertion; the
three-walk discovery correctly scoped so containment constrains only
`RepoRoot::discover`; unwired shipping gives a deletion-only rollback; shared
artefact contention identified with a stated resolution rule; the test-support
crate verified across a real crate boundary; quality-attribute tradeoffs
explicitly priced.

**Findings**:
- **major/high** — Query value types declared in the adapter crate while
  ADR-0053 and `vcs_domain_imports_only_permitted` mean 0169's port cannot
  reference them (Phase 2 §1)
- **major/high** — pup `allowed_only` omits `tracing`, permits inert
  `kernel::Error` (Phase 1 §3)
- **major/medium** — gix/jj-lib as normal deps enter `corpus-adapters` →
  `visualiser/server`; compile-time impact unmeasured (Phase 1 §1)
- **major/medium** — fixture binaries in `_CLI_RELEASE_BINARIES` put the DCE
  guard in a stage that never runs on a PR (Phase 4 §2)
- **major/medium** — no error channel; absence signals contractually distinct
  per oracle all collapse to `None` (Phase 2 §1)
- **major/medium** — the strong-form job compiles inside the shadow window with
  `vergen-gitcl` present (Phase 4 §3)
- **minor/medium** — undeclared dev-dependency cycle via `vcs-test-support`
  (Phase 3 §1)
- **minor/medium** — `boundary` reimplements vs delegates to `MarkerWalkRoot`
  is left ambiguous, and the choice determines 0185's deletion cost (Phase 1 §4)
- **suggestion/medium** — split `library.rs` into `git`/`jj` submodules
- **suggestion/medium** — extract the superproject derivation as a pure function
- **suggestion/low** — `in_process` over `library`; document the pup-vs-source-
  guard division

### Code Quality

**Summary**: Unusually well-evidenced — an empirical oracle mapping,
non-vacuity demonstrations for every guard, and a clean extension shape leaving
the `vcs` domain crate and both retained adapters untouched. The weak points are
concentrated in one place: the internals of `InProcessProbe` are sketched rather
than designed.

**Strengths**: Clean extension shape with provable non-modification and a pure
deletion rollback; queries return plain domain value types so no library type
leaks into 0169's port; the `detection.rs` injection seam retains fixed expected
values rather than degrading to implementation-agreement; every guard is paired
with a non-vacuity demonstration; non-obvious library behaviours recorded at the
point an implementer hits them; the dual comparison explicitly labelled
transitional with a named owner.

**Findings**:
- **major/high** — pup permit list forbids `tracing`, so the module cannot
  honour the crate's documented warn-log-on-failure contract (Phase 1 §3/§4)
- **major/medium** — `VcsProbe::revision` entirely undesigned yet Phase 3
  asserts `RepoFacts` parity on it (Phase 1 §4; Phase 3 §2)
- **major/medium** — three marker-walk/marker-kind implementations duplicated
  from code already in the crate (Phase 1 §4; Phase 2 §1)
- **major/medium** — canonicalisation specified per-query, leaving the module's
  output contract inconsistent (Phase 2 §1)
- **minor/high** — the `UserSettings` text guard will flag its own rationale
  comment; the pup alternative is rejected without stated reason (Phase 2 §4)
- **minor/medium** — the `allowed_only` clause re-adopts a pup pattern
  `cli/pup.ron:93-98` warns against (Phase 1 §3)
- **minor/medium** — substantial fixture builders written in Phase 2 in a
  location Phase 3 moves them from (Phase 2 §2; Phase 3 §1)
- **minor/medium** — interim helper binary of unclear identity, justified by a
  rationale that reads backwards (Phase 2 §3; Phase 4 §1)
- **suggestion/medium** — `JjRepositoryFacts` + `JjWorkspaceRole` over
  `JjRepository` + `secondary: bool`
- **suggestion/medium** — `library/mod.rs` + `git.rs` + `jj.rs`

### Test Coverage

**Summary**: Unusually strong on test design: a 24-pair × 6-query oracle mapping
established by measurement, explicit non-vacuity controls for every guard, and a
transitional dual-implementation comparison that retains fixed expected values.
The weaknesses are architectural rather than conceptual — the strong-form job
removes the binaries the fixture matrix needs to be built by; the two `VcsProbe`
port methods land in Phase 1 with no oracle rows and no test until Phase 3; and
two guards the work item calls load-bearing are one-off manual demonstrations.

**Strengths**: Per-pair expected values from actual measurement including
per-oracle absence signals; non-vacuity controls designed in throughout,
including the "no marker **and** values match" rule that defeats the
degrade-to-`None` false pass; the injection seam keeps fixed expectations and
notes that cross-implementation agreement is not an oracle; the cross-crate
proof correctly scopes the spawn assertion to `git`/`jj` so `SystemClock`'s
`date` spawn does not trip it; test-runner realities accounted for; the new
`tasks/` guard ships with unit tests and matching `test_mise.py` updates.

**Findings**:
- **critical/high** — the strong-form job shadows the binaries every fixture is
  built with; the suite cannot construct its own matrix (Phase 4 §3; Phase 3 §3)
- **major/high** — Phase 1 ships `kind`/`revision`/`repository_root` with no
  oracle rows and no test until Phase 3 (Phase 1 §4)
- **major/high** — the pup rule's non-vacuity is manual though
  `tests/integration/pup/test_import_rule.py` exists for exactly this
  (Phase 1 §3)
- **major/medium** — absence cells depend on an unasserted no-`.git`-ancestor
  precondition; `gix::discover` cannot be fenced by the environment (Phase 2 §2)
- **major/medium** — the scrub-invariant arms run through different execution
  paths, risking both false failures and a shared-absence false pass (Phase 2 §3)
- **major/medium** — the size-floor criterion is listed as automated with no
  named mechanism (Phase 4 §4)
- **minor/medium** — one test per query rebuilds the whole matrix six times
  (Phase 2 §2)
- **minor/medium** — infrastructure written twice; Phase 2's hermeticity
  criterion depends on a Phase 3 module (Phase 2/3)
- **minor/medium** — no malformed-repository fixtures despite `Option`-only
  returns (Testing Strategy)
- **minor/high** — no verification that the `gix` pin and `uluru` exception
  comments are present (Phase 1 §5)
- **minor/medium** — the `.git`-file worktree case is required to both match and
  possibly diverge (Phase 3 §2)

### Correctness

**Summary**: Unusually rigorous for a planning document — the oracle mapping is
empirically derived, absence signals are tabulated per oracle, and several
genuine traps were caught by measurement. However the mapping does not hold
together under scrutiny: one query table contains a cell contradicting both its
sibling rows and the plan's own absolutisation rule, Query 6's verdict column
asserts an inference the plan's own amendment refutes, three tables have missing
cells despite a traceability criterion, and canonicalisation is specified for two
of six queries. Most seriously, `VcsProbe::revision` is sketched as
`/* gix / jj-lib */` and its jj half appears unreachable under the crate-wide
`UserSettings` ban.

**Strengths**: The three-walk discovery established by measurement with the
divergent rows bolded; absence signals tabulated per oracle rather than assumed
uniform; the unnormalised `common_dir()` and `repo_path()`-vs-`workspace_root()`
asymmetries recorded; non-vacuity controls well chosen and paired to the property
they guard; the `CR`/`CG` split correctly identified; the `rposition` derivation
validated at the only depth that distinguishes nearest from first.

**Findings**:
- **critical/high** — jj `revision` has no mechanism and conflicts with the
  `UserSettings` ban vs Phase 3's identical-`RepoFacts` criterion
- **major/high** — `PJG-i`'s `--git-common-dir` contradicts `NJG-i`/`PG-s` and
  the plan's own absolutisation rule (Query 2)
- **major/high** — Query 6's "roots equal → colocated" contradicts Amendment 3
  and the `classify_checkout` table (`CR`, `NGJ-o`)
- **major/high** — canonicalisation specified for two queries only;
  `dual_roots` compares two regimes (Phase 2 §1; Query 6)
- **major/medium** — `rposition` misresolves a submodule whose path contains
  `modules` (Phase 2 §1)
- **major/high** — Query 5 omits `NGPJ-i`, Query 6 omits seven fixtures, the
  `classify_checkout` table omits five
- **major/high** — no fixture for a jj secondary inside its own main repository,
  the repo's own `workspaces/*` shape
- **major/medium** — `Kind` cannot represent submodule + linked worktree
  simultaneously (Phase 2 §1)
- **major/medium** — Query 2's `main_worktree_root` has no matching oracle for
  `BARE`/`SM-1`/`SM-2` yet all rows read "agree"
- **major/medium** — every gix-based absent cell rests on an unstated
  no-`.git`-ancestor precondition
- **major/medium** — pup `allowed_only` omits `tracing` while the port contract
  requires logging (Phase 1 §3)
- **minor/high** — the `NJG` fixture key says "colocated inner main" but the
  data says jj secondary (Fixture keys)
- **minor/high** — 24 vs 25 pair count; `--show-superproject-working-tree` has
  two absence signals but one table row

### Compatibility

**Summary**: Unusually rigorous on the dependency-graph side: both pins carry
inline rationale following the workspace's matched-pair precedent, the
single-gix-graph invariant is enforced with an explicit vacuity guard, and the
gix-0.86 caret trap and kstring/MSRV trap were established empirically. The weak
edges are the *runtime* compatibility surfaces: nothing asserts the `jj` binary
building the fixtures matches the pin, the `git` CLI is not pinned anywhere, and
the six query signatures collapse "no repository" and "repository the pinned
library cannot parse" into the same `None`.

**Strengths**: Pins carry rationale in the exact existing style; the gix-0.86
caret reasoning is correct and load-bearing; the single-version invariant is a
committed test with the vacuity trap explicitly guarded; MSRV-aware resolution
established empirically; the jj CLI ↔ jj-lib skew designed out rather than
measured around; the adapter ships unwired so no consumer contract changes;
environment independence verified across all pairs with a live non-vacuity
control; the shadow list correctly handles the CI-vs-developer-macOS divergence;
macOS `/var` canonicalisation and the jj-lib canonicalisation asymmetry recorded.

**Findings**:
- **major/high** — the lockstep test compares two declarations, not the `jj`
  binary that builds the fixtures (Phase 1 §5; Manual Testing step 3)
- **major/high** — the `git` CLI is an unpinned oracle for the entire matrix and
  is absent from the stated coupling (Oracle Mapping preamble; Phase 1 §1)
- **major/high** — all six signatures conflate absent with unparseable
  (Phase 2 §1; Phase 3 §3)
- **major/high** — gix and jj-lib become non-optional transitive dependencies of
  the shipped visualiser; only one of four triples verified (Migration Notes;
  Phase 4 §2)
- **major/medium** — "regenerate the lock" re-resolves deliberately fenced
  crates (`reqwest`, `rustls`, the hickory RUSTSEC ignores) (Migration Notes)
- **major/medium** — no non-default git format fixtures (reftable, sha256) where
  gix 0.85 support is partial (Phase 2 §2)
- **minor/medium** — the exact `=0.85.0` pin blocks patch remediation under
  `yanked = "deny"` and contradicts the test's `0.85.\d+` assertion (Phase 1 §1/§5)
- **minor/medium** — the new member's manifest omits `edition`/`rust-version`
  inheritance (Phase 3 §1)
- **minor/high** — the pup permit list disagrees with the declared dependency set
  in both directions (Phase 1 §1/§3)
- **suggestion/medium** — fixture binaries added to the release-binary contract
  constant (Phase 4 §2)
- **suggestion/medium** — the MPL-2.0 discharge rationale is untested against a
  publicly distributed binary (Phase 1 §2)

### Portability

**Summary**: Unusually strong on environment portability: it correctly
identifies that the work item's absolute shadow list was half-vacuous on
GitHub-hosted Linux, that macOS must degrade to PATH-only under SIP, and that
macOS `$TMPDIR` canonicalisation has to be designed into every fixture. The two
weak points are both about *proof of environment*: the strong-form job splits
the shadowing mechanism between workflow YAML and a Rust module with no defined
contract and no automated assertion that the run was actually strong; and the
two new fixture binaries are wired into a four-target cross-compile that only one
target has been verified against.

**Strengths**: The OS-matrix placement decision is conscious and correct; the
closed shadow list is rewritten as runtime resolution after discovering it was a
no-op on Linux; macOS `/var` → `/private/var` designed into fixture construction
with the jj-lib asymmetry called out; the size criterion measured on the artefact
the pipeline actually ships and recalibrated after measurement; dependency
selection portability-aware (no TLS, no `git2`, no duplicate gix, MSRV-aware);
`if: always()` restore and a documented rollback.

**Findings**:
- **major/high** — strong-form shadowing split between YAML and the harness with
  no contract; nothing automated proves the run was strong (Phase 3 §1; Phase 4 §3)
- **major/medium** — cargo compilation and mise tool resolution happen inside
  the shadow window; `mise run` may reinstall `jj` (Phase 4 §3)
- **major/high** — only one of four release targets verified, and no CI job
  cross-compiles (Phase 4 §2)
- **minor/high** — GitHub-hosted-runner and mise install-layout assumptions
  encoded rather than derived or documented (Phase 4 §3)
- **minor/medium** — shadowing inside mise's cached install tree risks poisoning
  the job's own cache (Phase 4 §3)
- **minor/medium** — cost figures are darwin-only while the shipped artefact is
  static musl (Phase 4 §4)
- **minor/medium** — only `common_dir()` canonicalised, but every returned path
  crosses the macOS `/var` symlink boundary (Phase 2 §1)
- **minor/medium** — the `hermetic` module under-specified vs the measured
  environment; `GIT_CONFIG_NOSYSTEM` is a boolean, not a path (Phase 3 §1)
- **suggestion/medium** — assert magic bytes and static ELF in place rather than
  staging fixtures into `dist/release/` (Phase 4 §2)

### Safety

**Summary**: The core product-safety posture is strong: the adapter ships
unwired, `vcs_adapters::facts` stays hard-wired, and no running consumer can be
affected. The safety risk has instead been displaced entirely into the build and
release infrastructure: a test-support crate carrying an absolute-system-binary
shadow list that runs on developer machines, a CI job that `sudo mv`s binaries
out of a *cached* directory while a cargo compile runs inside the shadowed
window, ~60 new crates gaining veto power over every PR and release, and a
release-pipeline edit that cannot be exercised before merge.

**Strengths**: Unwired by construction so consumers cannot regress; the restore
step's `if: always()` and the shadow-took-effect assertion show the hazard was
noticed; the job takes its own cache prefix and `RUSTUP_HOME` routing so damage
is contained; fixture temp dirs use owned `TempDir` guards with RAII cleanup on
the panic path; the existing release machinery uses explicit expected sets rather
than directory scans, so a fixture cannot be silently signed or published; phases
individually mergeable with the policy objection surfaced first; the size
criterion recalibrated and Phase 4 required to re-measure.

**Findings**:
- **critical/high** — the `stubs` module is described as attempting to shadow
  absolute system binaries including user-writable `/opt/homebrew/bin`, on a
  path reachable from the bare `mise run` (Phase 3 §1)
- **major/high** — shadowing inside the cached mise tree, with cache-save after
  restore, makes an incomplete restore self-perpetuating (Phase 4 §3)
- **major/high** — a cargo compile runs inside the shadowed window; passes warm,
  fails cold, and this plan guarantees a cold cache (Phase 4 §3)
- **major/medium** — `if: always()` is necessary but not sufficient; the claim
  rests on an unstated ephemeral-runner assumption (Phase 4)
- **major/medium** — `unmaintained = "all"` over the new closure can block all
  releases with no documented break-glass (Performance Considerations)
- **major/high** — fixture binaries on the release critical path, verified on one
  triple, detected only post-merge (Phase 4 §2)
- **major/medium** — "regenerate the lock" is an unaudited whole-graph
  dependency change landing as merge cleanup (Migration Notes)
- **major/high** — the rollback claim omits workflow/mise/lock footprint, and one
  ordering bricks the whole Main workflow (Migration Notes)
- **minor/medium** — no baseline recorded for the shipped visualiser's size
  (Phase 1 §1)
- **minor/medium** — `test-visual-regression`'s 20-minute budget vs a guaranteed
  cold-cache compile of both new trees (Phase 1)
- **minor/medium** — Phase 5 rewrites sibling criteria in place rather than
  appending dated amendment blocks as 0188 did for itself (Phase 5)
- **suggestion/low** — state that no fixture builder writes outside its own
  `TempDir`; keep `protocol.file.allow=always` strictly per-invocation

### Security

**Summary**: Unusually rigorous about *observable* dependency-policy hygiene —
per-crate licence exception, single-graph gix assertion, no-TLS assertion,
non-vacuity demonstrations — and it ships the adapter unwired with a clean
rollback. The gap is on the other side of the trust boundary: it replaces an
isolated, time-capped, environment-scrubbed subprocess with in-process parsing of
repository-controlled data, and neither the fixture matrix nor the success
criteria contain a single adversarial repository. Two headline safety claims are
materially narrower than stated, and the exact pins interact badly with
`unmaintained = "all"` by making `advisories.ignore` the cheapest response.

**Strengths**: The poisoning test points at another fixture's *real* `.git` and
requires a live non-vacuity control — an unusual and correct instinct; every new
guard must be demonstrated non-vacuous before it is trusted, including the "no
marker **and** values match" rule; the licence exception is scoped per-crate with
inline justification, respecting `deny.toml`'s stated convention; Phase 1 lands
the entire policy surface first so an objection is reviewable in isolation; the
unwired adapter bounds the blast radius; the plan declines to reproduce the
shell's `GIT_DIR` scrub asymmetry; default features keep TLS out of the graph.

**Findings**:
- **major/high** — in-process parsing of repository-controlled data drops the
  subprocess time cap, crash isolation and warn-logging with no replacement, and
  the eventual callers include a long-lived HTTP server (Phase 1 §4; Phase 2 §1)
- **major/medium** — the environment-immunity invariant is much narrower than
  the "uniformly immune" claim, and immunity is observed rather than constructed
  (Key Discoveries #6; Phase 2 §3)
- **major/medium** — the pup rule is not a structural zero-spawn guarantee;
  `RestrictImports` sees `use` paths only and nothing constrains gix's
  `attributes`/`blob-diff` filter machinery (Phase 1 §3)
- **major/high** — exact `=` pins block patch remediation under
  `unmaintained = "all"`, and the manifest pin contradicts the test's
  `0.85.\d+` assertion (Phase 1 §1)
- **major/high** — the no-TLS assertion is host-target-scoped while the property
  must hold on five triples; `[bans].deny` is not extended (Phase 1 §5)
- **major/medium** — the uluru justification answers MPL §3.1 (modification),
  not §3.2 (Executable Form notice), and sets the template for every future
  exception (Phase 1 §2)
- **minor/high** — no adversarial fixture in the 24-shape matrix, though
  Queries 2, 3 and 5 all consume repository-supplied paths (Testing Strategy)
- **minor/medium** — the `sudo mv` step mutates the cached mise tree and never
  asserts the restore succeeded; `actions/checkout`'s post step needs `git`
  (Phase 4 §3)
- **minor/medium** — test-fixture binaries added to the release staging constant,
  contrary to the sibling `config-adapters` convention (Phase 4 §2)
- **minor/medium** — no build-script or proc-macro policy for the 56-package
  expansion, though the release workflow documents build-script trust as a live
  concern (Phase 1 §1/§5)

## Re-Review (Pass 2) — 2026-08-03

**Verdict:** REVISE

**Coverage caveat:** 6 of 8 lenses completed. The **correctness** and **safety**
agents both terminated on an API session limit after substantial work and
returned no findings. Their pass-1 findings are therefore **unverified** in this
pass — in particular correctness owned the oracle-mapping audit, which is where
the largest pass-1 cluster sat, and safety owned the CI/release hazards. Neither
absence should be read as resolution.

### What the revisions fixed

All three pass-1 criticals are resolved, and the majority of the majors:

- 🔴 **Shadow window** — Resolved. Compilation and fixture construction now
  precede the shadow step; `$RUNNER_TEMP` rather than in-place rename; idempotent
  shadow/restore; post-restore liveness assertion; `timeout-minutes`; explicit
  mode contract. (Portability and security both confirm the reasoning, with
  residual issues below.)
- 🔴 **`revision`** — Resolved as a blocking pre-Phase-1 gate with three named
  resolutions. Architecture and test-coverage both note the gate is correct but
  Phase 3's criterion carries no matching conditional.
- 🔴 **`stubs` write boundary** — Resolved. Non-destructive, report-only, no
  `sudo`, all privileged mutation in the workflow. Security calls this "the right
  boundary".
- 🟡 **pup `tracing`** — Resolved.
- 🟡 **Oracle mapping completeness** — Resolved via re-measurement; test-coverage
  independently verified all six query tables now reconcile to 24 pairs.
- 🟡 **Error channel, canonicalisation, `_CLI_RELEASE_BINARIES`, four triples,
  lock operation, revert order, MPL §3.2, break-glass** — all resolved in
  substance.

### Two critical defects the revisions INTRODUCED (both now fixed)

- 🔴 **`[bans].deny` on `rustls` would fail `deny:check` outright** — flagged
  independently and at high confidence by **compatibility** and **security**.
  Verified: `cli/Cargo.toml:35` pins `rustls = "=0.23.41"` and
  `cli/launcher/Cargo.toml:31` consumes it directly; the section's own comment
  reads "rustls only", meaning rustls is the *chosen* TLS stack. `deny:check` is
  in `check-supply-chain`, which is in `prerelease.needs`, so this would have
  blocked Phase 1 and all releases. The scope error was substituting a
  whole-graph assertion for the work item's subtree assertion. **Fixed**: named
  gix transport crates only (each verified absent first), `rustls` explicitly
  excluded with the reasoning recorded, and the subtree pytest looped over all
  five configured targets to restore target-awareness.
- 🔴 **`rust-version` does not exist in `Cargo.lock`** — flagged by
  **compatibility**. Verified: zero occurrences in the committed lock; the format
  carries only `name`, `version`, `source`, `checksum`, `dependencies`. A
  lock-parsing implementation would find nothing and pass vacuously. **Fixed**:
  sourced from `cargo metadata --locked`, semver-ordered, with a non-vacuity case.

### Other findings, fixed in this pass

- 🟡 **`superproject` specified two contradictory ways** (code-quality, high) —
  "outermost `modules`" contradicts the plan's own `SM-2` oracle
  (`$super/mid`, not `$super`). **Fixed**: restated as innermost-outward,
  first-whose-parent-opens, with both discriminating cases worked through.
- 🟡 **The "pure function" cannot be pure** (code-quality) — it must probe the
  filesystem to pick the anchor. **Fixed**: parameterised with
  `is_repository: impl Fn(&Path) -> bool`.
- 🟡 **jj version assertion placed in a crate with no jj-lib edge**
  (compatibility, high) — and exact equality would fail the whole matrix on a
  pre-authorised patch skew. **Fixed**: injected from the linking crate via a
  `build.rs` constant, compared at major.minor.
- 🟡 **Hermetic env strips the git identity** (portability, high) — with `HOME`
  at a temp dir and no `user.email`, `git commit` refuses on any host whose
  hostname lacks a domain, i.e. CI runners and containers.
  `hooks/test-vcs-detect.sh:58` already does this correctly. **Fixed**.
- 🟡 **Two of three git format knobs were wrong** (compatibility) —
  `extensions.objectFormat` is a repository-format extension, not an `init` knob,
  and drives a v1-extension-in-v0-repo failure; `init.defaultRefFormat` is not
  documented across the range. **Fixed**: documented `--object-format` /
  `--ref-format` flags, plus a post-construction format assertion.
- 🟡 **Delegation pointed the surviving code at the code 0185 deletes**
  (code-quality) — making `^crate(::|$)` permanently load-bearing and 0185's
  deletion non-mechanical. **Fixed**: direction inverted; helpers extracted in
  Phase 1 into a module that survives, with the retained pair delegating to them.
- 🔵 **Stale `.secondary` and "Phase 3's `hermetic`" references** — **Fixed**.

### Still present — carried forward

Not addressed in this pass; each needs a decision rather than a mechanical edit.

- 🟡 **Phase bookkeeping remains partly split** (architecture, code-quality,
  test-coverage — three lenses). Partially fixed: the crate, dev-dependency,
  `members` entry and `[[bin]]` are now assigned to Phase 2. Still stale:
  Phase 3 §1 heads the manifest file "(new)"; Phase 3's success criterion still
  says "passes with the new member"; Phase 4 §1 still declares the reference
  artefact "(new)" with its clippy prelude though Phase 2 §3 requires it built;
  the Implementation Approach and Phase 4 Overview still say "Phase 4 adds the
  reference artefact".
- 🟡 **Phase 1's boundary-containment criterion needs Phase 2's nesting
  fixtures** (architecture, test-coverage). Either move the criterion to Phase 2
  or move the `fixtures` module to Phase 1.
- 🟡 **The 9 new fixtures have no keys, start directories or query-table cells**
  (test-coverage, high) — so Phase 2's own "no query table is partial" criterion
  is unsatisfiable and their expected values cannot be written test-first. They
  need measuring, exactly as the pass-1 cells were.
- 🟡 **Degenerate/`HOSTILE` expectations are too loose to kill mutations**
  (test-coverage) — "`Err` where applicable" is undefined, and `HOSTILE` is a
  well-formed repository for which `Err` is the wrong expectation entirely.
- 🟡 **`Err` vs `Ok(None)` partition is deferred to implementation**
  (architecture, code-quality) — the half of the new contract the error channel
  exists to express has no oracle, while Phase 2 claims tests come first.
- 🟡 **`Error`'s shape is unspecified** (code-quality) — no variants, no
  `source()` decision, no `thiserror` in the manifest. 0169 is told to map it
  with nothing to map.
- 🟡 **Poisoning matrix omits `HOME`/`XDG_CONFIG_HOME`** (security) — file-based
  `~/.gitconfig` and `/etc/gitconfig` are the paths that apply in production and
  are never exercised; gix's config permissions are granular, so an
  implementation setting only `env: false` passes every specified cell while
  still reading the user's config.
- 🟡 **gix `Permissions`/`Trust` direction unspecified** (security) — "construct
  immunity, don't rely on defaults" could be read as elevating to `Trust::Full`,
  disabling the ownership check that is gitoxide's `safe.directory` equivalent.
- 🟡 **Liveness controls cover one variable family** (test-coverage) — most newly
  added poison cells assert invariance under variables nothing shows would change
  any answer.
- 🟡 **`HOSTILE` will pass trivially** (security) — none of the eight delivered
  calls enters gix's filter/pager/external-diff machinery, so the evidence reads
  as stronger than it is, and 0169 adds exactly the APIs that do reach it.
- 🟡 **gix feature-set assertion** — added this pass at security's suggestion;
  worth confirming the named present/absent sets against the resolved graph.
- 🟡 **Size floor and four-triple buildability run only on the release path**
  (architecture, test-coverage, portability) — no PR-level feedback for either.
- 🟡 **Tilde pins contradict the work item without an amendment**
  (compatibility) — the work item says "pinned exactly at `=0.43`" and "two-crate
  bump"; the plan now says `~0.43.0` and four-pin. Needs a seventh amendment.
  Compatibility also argues the RustSec-agility rationale does not hold for the
  *transitive* closure it invokes, and that `jj-lib` specifically may deserve to
  stay exact.
- 🟡 **Escape-the-boundary queries follow repository-controlled links with no
  hostile-link fixture** (security) — the degenerate fixtures cover missing
  targets, not links pointing at a *valid* foreign repository.
- 🟡 **Strong-form proof is provider-coupled** (portability) — a mount namespace
  (`unshare -m` + bind-mount) or the repo's existing Docker precedent would remove
  the restore/cache/ephemerality reasoning entirely.
- 🟡 **`$MATRIX_DIR` has no location or lifecycle contract** (portability) —
  `TempDir` guards delete on drop and so cannot hand a matrix to a later step, and
  the no-`.git`-ancestor precondition must be asserted against it.
- 🔵 **Dependency-tree placement never weighed against crate-level isolation**
  (architecture) — recorded as a consequence twice, never as a rejected
  alternative, though Phase 3 §3 already relies on the dev-dependency property
  that would deliver it.
- 🔵 Minor: `WorktreeFacts.linked: bool` keeps the shape `JjWorkspaceRole` was
  introduced to reject; per-pair tests abort on first mismatch; `RF`/`S256` are
  record-what-happens with an available oracle unused; warn-logging is unasserted;
  the emulated-vs-native provenance of the Linux musl figure; licence-side
  break-glass; `cargo add` in the lock guidance mutates the manifest.

### Assessment

The plan is substantially stronger than at pass 1 — every critical is closed and
the oracle mapping is now internally complete and empirically grounded. But this
pass found two *new* critical defects introduced by the pass-1 edits, both in
`cli/deny.toml`/graph-test territory and both of which would have failed CI on
Phase 1's first run. That is a signal about the revision process, not just the
content: supply-chain edits made from reasoning rather than from reading the
existing config are the risk area, and both were caught only because two lenses
verified against the actual files.

**Not ready for implementation.** Three things must land first: the seventh
work-item amendment (tilde pins), the `revision` resolution, and the measurement
of the 9 new fixtures. The phase-bookkeeping cleanup is mechanical but should be
done before Phase 1 so the phases really are independently mergeable. A third
pass should re-run **correctness** and **safety** specifically — they did not
complete here, and correctness is the lens that owns the mapping this plan is
built on.

## Re-Review (Pass 3) — 2026-08-03

**Verdict:** REVISE

Scope: the two lenses that failed to complete in pass 2 — **correctness** and
**safety**. Both completed this time. Every claim below that concerns an existing
file was verified against that file before acting.

### Pass-1 findings, now verified

Correctness independently re-derived the row counts and confirms **Queries 1, 2,
4, 5 and 6 each cover exactly 24 pairs with no gaps** — the pass-1 mapping
findings are genuinely fixed. It also confirms the revised `worktree.linked` rule
(canonicalised `git_dir() != common_dir()`) is correct on all 24 pairs plus
`SM-w`, that the revised `superproject` scan produces both worked cases and
survives shapes not in the plan, that the re-measured `PJG-i` common-dir is now
self-consistent with `NJG-i`/`PG-s`, and that the `JS-in` reasoning is exactly
right against `scripts/vcs-common.sh:248-254`.

Safety confirms the containment delta is correctly priced, the `vcs-test-support`
non-destructive boundary is the right call, each shadow-window exclusion is
justified from a real mechanism, revert-order claim 1 is correct and
actionlint-detectable, and that refusing the bare `rustls` ban was right.

### Three more factual errors in my pass-2 edits (all now fixed)

- 🟡 **The lock-contention rationale was wrong.** I cited `reqwest` and `rustls`
  as crates a regeneration would float. Verified: both are **exact** requirements
  (`cli/Cargo.toml:30` `=0.12.28`, `:35` `=0.23.41`) and cannot move whatever
  happens to the lock. A reader who checked would have discounted the whole
  warning. **Fixed**: names the actually-floating caret/tilde set plus the MSRV
  trap. Also fixed the prescribed command — `cargo update -p gix -p jj-lib` fails
  with "package ID specification did not match any packages" in exactly the
  scenario it was prescribed for, pushing the reader toward
  `generate-lockfile`; and `cargo add` rewrites the manifest, dropping the
  lockstep comment a new test asserts.
- 🟡 **The size floor was unscoped.** `_assert_static_elf` is guarded
  `if "musl" in triple` (`tasks/build.py:329-330`); my "next to it" wording left
  the absolute-byte floor applying to darwin, whose stripped delta (1,639,872 B)
  clears 1,500,000 B by only **9.3%** — a 9%-margin heuristic on
  `prerelease:prepare`'s critical path. **Fixed**: ratio floor everywhere,
  absolute floor musl-only, host-native ratio check in `check-zero-spawn` for PR
  feedback, recovery documented.
- 🟡 **`Result<DualRoots, Error>` cannot express per-side failure** (correctness,
  critical). A one-sided failure either discards a valid answer or flattens to
  `None`, reinstating the absence/failure conflation on the *single field* 0169
  discriminates `colocated` from `nested-*` on. **Fixed**: `dual_roots` is
  infallible with `Result` per side; callers must treat `Err` as "not comparable",
  never as inequality. A stale contradicting sentence was also caught and fixed.

### Other findings fixed this pass

- 🟡 **`main_root` dropped the oracle's defensive invariant.** Verified
  `scripts/vcs-common.sh:106-112` carries
  `[ -d "$candidate/.jj/repo" ] || return 1`, commented "so a future jj layout
  change cannot silently produce a wrong-but-non-empty answer". Without it a
  pointer resolving to any existing non-store directory yields a confident wrong
  root that becomes `RepoFacts.name` after 0185. **Fixed**, plus a degenerate
  fixture for an existing-but-not-a-store target.
- 🟡 **Relative `start` splits the three walks.** Verified
  `MarkerWalkRoot::discover` (`cli/vcs-adapters/src/lib.rs:35-44`) is purely
  lexical while `gix::discover` absolutises — so a relative `"sub"` makes a
  colocated checkout read as "git only". Invisible to the matrix, whose paths are
  all absolute. **Fixed**: absolutisation precondition plus a relative-`start`
  test per walk.
- 🟡 **`is_repository -> bool` conflated not-a-repo with unopenable** — my own
  pass-2 edit, and it contradicted the degenerate fixtures' "not a plausible
  wrong path". **Fixed**: fallible probe.
- 🟡 **`$BASE` notation is internally contradictory** — `$BASE/jjmain` is claimed
  both colocated and `--no-colocate`, and `$BASE/gitparent/.git/worktrees/sub` is
  claimed for two fixtures a single parent cannot host (`git worktree add`
  de-duplicates to `sub1`), which the recorded `worktrees()` count of 1
  corroborates. **Fixed**: disambiguation required before the builders are
  written, with the re-measurement's distinct parents named.
- 🟡 **`$MATRIX_DIR` handoff impossible under `TempDir`** — guards delete on drop.
  **Fixed**: workflow-created root, builder writes without owning a guard,
  `TempDir` retained for in-process paths, `TempDir::keep()` explicitly rejected
  (the store-duplication guard misses it and it would leak a 19-fixture tree on
  every developer `mise run`), no `rm -rf "$MATRIX_DIR"`.
- 🟡 **Revert order omitted the dangling-reference edges.** The two
  `[dev-dependencies]` path edges must go before the crate directory or cargo
  cannot load the workspace graph — breaking `cli:check`, `server:check`,
  `test:unit:cli` and `deny:check`; and the `depends` references must go before
  the task definitions. **Fixed** and renumbered.
- 🟡 **Restore rested on a scheduler guarantee** — a job-level timeout cancels the
  job, and a hanging suite is the likeliest unattended failure. **Fixed**:
  `trap restore EXIT` in one step with a step-level timeout; `cache: false`
  recommended, with cache-prefix bump as documented recovery.
- 🟡 **Advisories break-glass had no class scoping** — a pre-blessed five-minute
  suppression covering vulnerability-class advisories in a closure reaching the
  signed binary. **Fixed**: scoped to unmaintained/yanked/notice, escalation
  required for vulnerabilities, review-by dates enforced by a test, licence-side
  case documented.
- 🟡 **Zero-spawn mode contract was fail-open** on absent or misspelled values.
  **Fixed**: exact accepted values, mode reported back and asserted.
- 🟡 **`revision` resolutions were unranked** — option 2 (narrow the
  `UserSettings` ban) would put a chain abandoned after five successive panics
  into a long-lived server with no crash isolation. **Fixed**: ranked, descope
  named the fail-safe, parity narrowed per `VcsKind` rather than wholesale.
- 🔵 **`test:integration:zero-spawn` roll-up membership** was left open; it would
  double fixture-matrix construction on both CI legs and every `mise run`.
  **Fixed**: `_NOT_IN_INTEGRATION_ROLLUP`, per the `test:integration:pup`
  precedent.

### Still open

- 🟡 **`superproject` returns the main worktree, not the containing one**
  (correctness). A submodule inside a linked worktree `$wt` of `$main` anchors on
  `$main/.git` and yields `$main`, where the oracle returns `$wt`. Recorded in the
  plan as unresolved with a measurement required; not fixed, because it needs a
  new fixture measured.
- 🟡 **`SM-w`'s `kind()` was asserted, not measured** (correctness). If it is
  `LinkedWorkTree` the fixture's stated rationale evaporates; if `Submodule`,
  `superproject` has a wrong answer. Needs measuring.
- 🟡 **The `Err`/`Ok(None)` partition remains deferred** (correctness, and
  architecture/code-quality in pass 2 — now three lenses). Recorded divergences
  still says "record the list as Phase 2 establishes it" while the Implementation
  Approach claims values come from the table first. The nine new fixtures'
  expectations are self-fulfilling until an Err set is enumerated per query.
- 🟡 **`classify_checkout` table covers 19 of 24** — missing `PJS`, `NGPJ-i`,
  `NGPJ-o`, `PJG-i`, `PJG-o`, i.e. both nested arms' pure-jj variants.
- 🟡 **Canonicalisation has no defined failure behaviour on the two port
  methods**, which keep non-`Result` signatures (`repository_root -> PathBuf` has
  no channel at all).
- 🟡 **The truncated-pack degenerate fixture proves nothing** — no query reads
  object data; only `revision` does, and it returns `Option`.
- 🟡 **`check-zero-spawn` in `prerelease.needs` has no break-glass** for
  environment-driven failures (safety). Verified `test_workflows.py` does not
  assert that list exhaustively, so the edge adds cleanly.
- 🟡 **The containment hand-off to 0185 is prose on a sibling work item** — safety
  recommends a committed guard asserting `facts` names `CommandProbe`, so 0185
  deletes a guard deliberately rather than failing to read a note.
- 🔵 `Kind::Common`/`gix::open` described three incompatible ways for `discover`;
  `main_worktree_root` holds a value that is neither on submodule shapes;
  Query 2 Verdict vocabulary inconsistent; append-only amendment blocks leave the
  stale text first in reading order.

### Assessment

Correctness's structural verdict is the important result: the oracle mapping's
*coverage* is now genuinely complete and its re-measured cells are
self-consistent. What remains there is a notation collision, five missing
composite rows, and two shapes asserted rather than measured — bounded work, not
a redesign.

The recurring pattern across all three passes is now unmistakable and worth
recording: **six of the defects found in passes 2 and 3 were introduced by my own
revisions, and every one was a claim about an existing file made from reasoning
instead of reading it** — `rustls`'s pin status, `Cargo.lock`'s schema,
`_assert_static_elf`'s guard, `MarkerWalkRoot`'s lexical walk, the shell's
defensive invariant, `TempDir`'s drop semantics. Plan edits that assert
something about the repository must be verified against the repository at the
moment they are written.

**Not ready for implementation.** The blocking set is unchanged in shape but
better specified: resolve `revision` (option 1 spike, else option 2); apply the
work-item amendments (5, 6, and a seventh for the tilde pins); measure the nine
new fixtures, the five missing `classify_checkout` rows, `SM-w`, and the
submodule-in-a-linked-worktree shape; disambiguate the `$BASE` notation; and
enumerate the `Err` set per query. The phase-bookkeeping cleanup from pass 2
remains outstanding and is still mechanical.

## Addendum — `revision` spike and measurement round 2 (2026-08-03)

### Spike: jj `revision` — resolved NO

A throwaway crate (`jj-lib = "=0.43.0"`, Rust 1.90.0, `resolver = "3"`, built
clean) established that **jj-lib 0.43 exposes no read-only, settings-free route
to the working-copy commit id**. The op stores are settings-free
(`SimpleOpHeadsStore::load(dir)`, `SimpleOpStore::load(path, root_data)`), but the
workspace name needed to index `View::get_wc_commit_id` is not:

- `LocalWorkingCopy::load` requires `&UserSettings` (`TreeStateSettings`), and
  `CheckoutState` — which holds exactly the `operation_id` + `workspace_name`
  pair — is a private struct with a private `load`.
- `SimpleWorkspaceStore::load(repo_path)` is settings-free but **mutates the
  repository**. Verified empirically: with `.jj/repo/workspace_store` removed,
  the call recreated the directory and wrote an `index` file. Unusable in a
  read-only probe. Its trait also exposes only `get_workspace_path(name)` with no
  listing API, so root→name inversion is unavailable regardless.

**Outcome**: jj `revision` descoped to 0185; the crate-wide `UserSettings` guard
stays crate-wide (the spike is the evidence nothing in scope needs it); Phase 3
parity narrows per `VcsKind`. Recorded as work-item amendment 8.

The spike also independently confirmed **Key Discovery 11**: without
`resolver = "3"` the graph selected `kstring v2.0.4 (requires Rust 1.96.0)`; with
it, `kstring v2.0.2`.

### A seventh instance of the same error pattern — in the fix for the sixth

My pass-3 fix for the `rustls` ban said the replacement crates were "each
confirmed absent from the resolved lock before being named". They were not
confirmed, and **`gix-transport` and `gix-protocol` are both present** (verified
in the spike's lock; both compile). Banning either would have failed
`deny:check` on the first run — the identical failure mode, in the edit written
to fix it.

Corrected: `[bans].deny` gains only `gix-credentials` and `curl-sys` (both
verified absent). The enforceable property is the **feature-set** assertion
(`blocking-network-client`, `async-network-client`,
`blocking-http-transport-*` off), not transport-crate absence, which is false.

This is the seventh defect of one kind across three passes: a claim about an
existing file asserted from reasoning rather than read. Every one was caught by
verification, none by re-reading my own prose.

### Measurement round 2

Ten extended fixtures and the five missing `classify_checkout` rows measured;
oracle columns only (gix/jj-lib land in Phase 1). Highlights: `JS-in` confirms
dual roots differ while classification is `jj-secondary`; `SM-wt` **downgrades**
the pass-3 superproject concern (git puts the submodule git dir under
`worktrees/<id>/modules/`, and the plan's scan gets the right answer);
`S256`'s HEAD is 64 hex against `detection.rs`'s 40-hex assertion; `HOSTILE` runs
none of its seven configured commands, so it is a guard for 0169's APIs rather
than evidence for this call set; `D3` proves the `.jj/repo` post-condition is
needed and corrects the blanket "degenerate shapes yield `Err`" to
`D1`/`D2` → `Ok(None)`, `D3` → `Err`. The truncated-pack fixture was dropped as
unreachable by any query.

### Remaining before implementation

**Blocking**: apply work-item amendments 5-8 to
`meta/work/0188-library-backed-vcs-adapter.md`.

**Phase-scoped, not gaps**: `gix::open` on a linked-worktree gitdir (the one
question `SM-wt` leaves); `RF`/`S256` library behaviour; the extended fixtures'
library columns; exact `Err` variant spellings (the partition *rule* is now
fixed). Safety recommendations not yet taken: a `check-zero-spawn` break-glass
and a committed guard asserting `facts` names `CommandProbe`.
