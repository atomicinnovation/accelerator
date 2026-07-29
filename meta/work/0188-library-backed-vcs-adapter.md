---
type: work-item
id: "0188"
title: "Library-Backed VCS Adapter over gix and jj-lib"
date: "2026-07-31T10:41:51+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: story
priority: high
parent: "work-item:0136"
blocked_by: ["work-item:0179"]
blocks: ["work-item:0169", "work-item:0185"]
relates_to: ["work-item:0125", "work-item:0168", "work-item:0187",
  "codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration"]
derived_from: ["work-item:0169"]
tags: [rust, vcs, dependencies]
last_updated: "2026-08-02T14:39:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0188: Library-Backed VCS Adapter over gix and jj-lib

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Add a library-backed implementation of the `vcs` crate's outbound ports —
together with six inherent taxonomy queries that 0169's checkout-classification
port will be built over, and the test apparatus that proves the whole thing
reads git and jj **in-process** — using `gix` (gitoxide) and `jj-lib` instead
of spawning `jj`/`git` subprocesses. This is the dependency-adoption half of
the VCS migration: two new dependency trees, a `cli/` workspace-wide
`deny.toml` licence exception, and a pre-1.0 API bet — separated from the
subdomain and hooks work so it can be reviewed and rolled back on its own terms.

The existing `CommandProbe` is **retained**, not replaced: it continues to serve
`cli/corpus-adapters` until 0185 converges that consumer. This story adds an
adapter; it does not remove one.

The adapter ships **unwired** — no caller reaches it until 0169 and 0185 do the
consumer work. That is deliberate: the value delivered here is risk isolation,
landing the dependency trees, the licence exception and the API bet where they
can be reviewed and reverted alone.

## Context

`cli/vcs-adapters` currently drives `jj log -r @ -T commit_id` and `git
rev-parse HEAD` as subprocesses (`cli/vcs-adapters/src/lib.rs:110-125`, spawning
at `:168`). The port abstraction (`cli/vcs/src/lib.rs:48-55`) already permits a
library-backed implementation without touching the domain — a second
implementation of the same ports is an adapter-level change by construction.

Extracted from 0169, on the argument of the scope lens in
`meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md` (pass 4):
dependency adoption and the hooks migration carry very different risk profiles.
A hook-envelope regression is user-visible and reversible in `hooks.json`,
whereas a `jj-lib` API break or a dependency-policy objection is a build-level
failure. Bundled, neither could be accepted or rolled back without the other.

Going in-process also dissolves 0125's stated rationale for keeping the shell
lexical fallback — that detection must work with no `git`/`jj` on `PATH`, and
that probing costs 1-3 subprocesses per call — but only *for consumers that
reach the Rust adapter*. The ~26 shell call sites keep running in bash until
later epic-0136 phases migrate them.

## Requirements

### Delivered surface

- **One library-backed type** in `cli/vcs-adapters` implementing *both* the
  `vcs` crate's `RepoRoot` and `VcsProbe` ports over `gix` and `jj-lib`,
  alongside the retained `MarkerWalkRoot`/`CommandProbe` pair. A single type
  avoids having to say which of a pair carries the dual-root query, which needs
  both libraries. **The domain crate `cli/vcs` is untouched: no port is added,
  widened or changed.**
- **Six taxonomy queries** as *inherent methods* on that type — not port
  methods. `CommandProbe` and `MarkerWalkRoot` gain none of them. 0169 defines
  whatever domain port its classifier needs and implements it over these; that
  port is explicitly out of scope here. The set is a fixed delivery contract:

  1. **Bare-repository check.**
  2. **Worktree detection** — is this a linked worktree, and what is the common
     (main) git directory? (`--git-dir` vs `--git-common-dir`)
  3. **Superproject/submodule resolution** — is this a submodule, and what is
     the superproject's working directory?
  4. **jj workspace-root resolution.**
  5. **jj main-vs-secondary distinction**, and where the main repository is.
  6. **Independent dual-root resolution** — the git repository root and the jj
     workspace root, each resolved by its own library's walk without being
     truncated by the other's marker, so a consumer can compare them. This is
     what separates `colocated` (roots equal) from `nested-jj-in-git` and
     `nested-git-in-jj`; 0169 cannot build those arms without it.

  **Invariant over all six**: `GIT_DIR`/`GIT_COMMON_DIR` are scrubbed for the
  duration of any detection call, so every query answers identically whether or
  not they are set. This matches the one place the shell made the decision
  deliberately (`vcs-common.sh:130-135`, "cannot be poisoned by ambient env")
  and diverges from `classify_checkout`'s inline reads, which honour them. The
  shell is internally inconsistent here — `:206-215` unscrubbed against
  `:130-135` scrubbed — and it is that asymmetry the adapter declines to carry
  forward.

  Anything 0169 needs beyond this list is a change to *this* work item, not
  silent growth in 0169. Two capabilities are explicitly **not** delivered and
  are 0169's own work over the trees this story lands: the `classify_checkout`
  arm cascade, and the `vcs status`/`vcs log` reads. 0169 will need to widen the
  cargo-pup rule below to cover wherever it puts that code.
- **No selection mechanism.** No feature flag, config switch or composition
  helper routes callers to the new type; `vcs_adapters::facts` stays hard-wired
  to `MarkerWalkRoot`/`CommandProbe`. Consumer wiring belongs to 0169 and 0185.
  This story ships an unwired adapter on purpose — its value is risk isolation.

### Test-support deliverables

These are shipped artefacts, not incidental test code, and are sized as such.

- **An injection seam** in `cli/vcs-adapters/tests/detection.rs`, which today
  hard-wires `MarkerWalkRoot` + `CommandProbe::new()` and would otherwise keep
  verifying the subprocess adapter.
- **A reference artefact** — a test-only binary calling every query and
  *printing each result*, so the calls are not eliminable. Without a caller,
  dead-code elimination would let the musl and size checks pass while linking
  none of `gix`/`jj-lib`.
- **A zero-spawn harness** — marker-writing `git`/`jj` stubs, the absolute-path
  shadow list, and the empty-config environment — published as a **shared test
  fixture consumable from another crate**, since 0185 extends it and one shadow
  list must serve both suites.
- **A committed lockfile check** (test or `tasks/` lint) for the version
  invariants below.
- **The checkout fixture matrix** — ten shapes × their start directories (see
  the criterion). Bare, submodule, linked-worktree and both nesting shapes are
  substantial to build; planning should record which already exist in
  `detection.rs` and which are new.
- **A pure-jj benchmark fixture**, committed as a named reusable builder,
  plus the cost measurements taken against it. Handed to 0169 ungated.
- **Dated hand-off notes** on 0125, 0185 and 0169, including a `relates_to`
  edge on 0125.
- **Documentation** of any new `tasks/` leaf task or CI job in
  `tasks/README.md`, alongside the existing cargo-deny/cargo-pup enforcement
  description — undocumented lint gates are the ones later contributors trip
  over or delete.
- **Whatever CI wiring the strong-form zero-spawn run needs** (below), *subject
  to the Open Question on runner capability*. If no existing Linux job can
  shadow absolute system paths, provisioning one is in scope — cross-surface
  work, called out so it is visible at sizing.

### Library traps

- **Bound `gix`'s discovery — for boundary resolution.** `gix::discover` walks
  up *past* a jj workspace boundary (verified 2026-07-29: it returned the parent
  repository's `.git` from inside `workspaces/build-system`). The rule:

  > The checkout boundary is the start path itself, or its nearest ancestor,
  > containing a `.jj` or `.git` marker. `RepoRoot` reports that path and never
  > an ancestor above it, whether or not the environment supplies a ceiling.

  Derive the ceiling from the marker walk rather than trusting the library's
  default; an inherited `GIT_CEILING_DIRECTORIES` may narrow the walk further
  but must never be what makes the rule hold. **The rule scopes to boundary
  resolution only** — queries 2, 3 and 6 legitimately resolve outside the
  boundary by following a recorded link (gitdir, superproject) or by letting
  each library complete its own walk. An earlier draft forbade looking above the
  boundary at all, which would have made 0169's `nested-*` arms unimplementable.
- **Avoid `jj_lib::workspace::Workspace::load`.** It needs a fully-populated
  `UserSettings` whose defaults are private to jj-lib, discovered one panic at a
  time. `DefaultWorkspaceLoaderFactory` is public and
  `WorkspaceLoader::{workspace_root, repo_path}` need no settings. **No code in
  `cli/vcs-adapters` may construct a `UserSettings` at all** — stated
  crate-wide, deliberately wider than the detection paths strictly require, so
  the guard is a simple one.

### Dependency policy

- **`jj-lib` pinned exactly at `=0.43`.** The loader-internals design and every
  feasibility measurement were validated against it, and the crate declares its
  API unstable. `mise.toml`'s `jj` CLI pin is held in lockstep at 0.43.0 so the
  CLI that writes fixtures and the library that reads them cannot skew. **The
  bump landed 2026-08-02**; what remains for this story is `mise install` in CI
  and a re-run of the jj-fixture shell suites.
- **`gix` pinned to the version `jj-lib` 0.43 depends on when its `gix`
  feature is enabled** — 0.85 (verified 2026-08-02: `jj-lib` 0.43.0 declares
  `gix ^0.85.0`, optional). See the Assumption on feature-gating.
  Load-bearing: `gix`
  0.86.0 exists, and a caret range on a `0.x` crate will not cross it, so
  pinning 0.86 here would produce exactly the two graphs this forbids. The pin
  **must** carry an inline comment saying it tracks `jj-lib`'s `gix` version.
- **A `[[licenses.exceptions]]` entry for `uluru`** (MPL-2.0) in
  `cli/deny.toml` — `gix-pack`'s LRU cache, not feature-gatable. It **must**
  carry an inline comment citing this work item.
- **`gix` with default features** — network transports are excluded by default,
  so no TLS stack enters the graph.
- **A `cli/pup.ron` rule scoped to the library-backed module only** — not to
  `vcs_adapters` as a whole, since the retained `CommandProbe` legitimately
  spawns. Two clauses, because a permit-list alone cannot express this
  (`std::process` sits *inside* the permitted `std`): permit `std`, `gix`,
  `jj_lib`, `kernel`, `vcs`, `crate::`; then explicitly deny `std::process`.
  The deny clause is what makes zero-spawn structural rather than only
  test-asserted. This requires the library-backed code to live in its own module
  with a stable path.

## Acceptance Criteria

**On oracles.** The behavioural oracle is `scripts/vcs-common.sh` and the
`git`/`jj` commands it invokes. The exact per-query oracle mapping is
**established empirically during planning** — by running each candidate against
each fixture and recording what it returns — and is **not** to be asserted from
line references. Two attempts to write that mapping by inference were both
wrong: `classify_checkout`'s `BOUNDARY` is documented empty for the `main` and
`none` arms and is set to the *git* root in `nested-git-in-jj`, and
`find_git_main_worktree_root` returns an ordinary root (exit 0) for every
non-submodule checkout. The mapping belongs in the plan, with evidence.

**On path comparison.** Wherever a criterion asserts path equality — adapter to
adapter, or adapter to oracle — compare `realpath`-canonicalised absolute paths.
On macOS a fixture under `$TMPDIR` resolves `/var` → `/private/var`, so an
uncanonicalised comparison fails spuriously. Absence maps to `None`; the exact
absence signal per oracle (non-zero exit, empty stdout, or an empty record
field) is part of the mapping established in planning.

- [ ] `detection.rs` runs every existing case through the injection seam against
      **the retained `MarkerWalkRoot`/`CommandProbe` pair and the single
      library-backed type**, producing identical `RepoFacts`, with the suite's
      existing **fixed expected values** retained — agreement between two
      implementations is not on its own an oracle. The `.git`-as-file worktree
      case keeps **today's** value (`classify_checkout` reports `main`, the git
      side unseen); 0169 owns correcting that to `colocated`, so a
      library-backed answer of `colocated` here is the deferred correction
      arriving early, not a defect to chase. This dual comparison is
      **transitional**: 0185 deletes `CommandProbe`, and collapsing the suite to
      the library-backed type alone is part of that deletion.
- [ ] All six queries are unit-tested against every **(fixture, start
      directory)** pair in the matrix — start directory included because every
      query is start-path-relative and the oracle is directory-parameterised, so
      running all six from each fixture's root would produce a full green matrix
      while never exercising the distinctions the queries exist to make. Each
      pair carries a recorded expected value drawn from the empirically
      established mapping, including explicit not-applicable expectations.
      Matrix, minimum:

      | Fixture | Start directories |
      | --- | --- |
      | colocated | root |
      | jj secondary workspace | workspace root, and a subdirectory |
      | plain git | root, and a subdirectory |
      | nested-jj-in-git | inner jj workspace, and the outer git root |
      | nested-git-in-jj | inner git repo, and the outer jj root |
      | linked git worktree | the linked worktree, and the main worktree |
      | git submodule | the submodule, and the superproject root |
      | bare repository | the bare dir |
      | no repository at all | a dir with no marker at or above it |
      | pure-jj measurement fixture | root |

- [ ] The scrub invariant holds across that whole matrix: every query returns
      the same value with and without `GIT_DIR`/`GIT_COMMON_DIR` set — where
      "set" means **pointed at a real git directory that would produce a
      different answer** (another fixture's `.git`, or the enclosing
      repository's
      for the nested shapes), not an empty or non-existent path, which both git
      and the libraries ignore. An unscrubbed control must diverge under the
      same
      poisoning, or the test proves nothing. The invariant covers the six
      queries
      and `RepoRoot`; `VcsProbe` parity against `CommandProbe` is explicitly
      **out of scope for this criterion**, since `CommandProbe` shells out and
      therefore does honour an ambient `GIT_DIR`.
- [ ] **Zero `jj`/`git` process spawns.** Black-box, over the full query ×
      fixture table, with `HOME`, `GIT_CONFIG_*`, `JJ_CONFIG` and
      `XDG_CONFIG_HOME` at empty temp dirs. The **strong form** — `PATH` stubs
      *plus* `/usr/bin/git`, `/usr/local/bin/git`, `/opt/homebrew/bin/git` and
      the `jj` equivalents replaced or bind-mounted — must actually run in a
      named Linux CI job; failing to achieve it there is a blocking finding, not
      a permitted degradation. Other platforms (SIP-protected macOS) degrade to
      `PATH`-only with unshadowable paths recorded. An in-crate spawn seam is
      **not** an acceptable substitute: the module has no exec port by design,
      so a seam cannot observe a spawn originating inside `gix` or `jj-lib`.
      Assert no marker is written **and** that every value matches the
      unrestricted run — an adapter degrading to `None` also writes no marker.
- [ ] `RepoRoot` cannot report a root above the marker boundary, with
      `GIT_CEILING_DIRECTORIES` unset or set above the parent repository so the
      environment cannot be what stops the walk. Both nesting directions are
      fixtured. A paired negative assertion shows an unbounded `gix::discover`
      on the same fixture *does* escape.
- [ ] No code in `cli/vcs-adapters` references `Workspace::load` or constructs a
      `UserSettings` — enforced by a **committed** check (a `tasks/` lint or an
      additional cargo-pup deny clause, not a one-off inspection), shown
      non-vacuous by a deliberately added construction failing it, plus every jj
      query succeeding under the empty-config environment above. A text guard
      over a crate that never contained those symbols otherwise passes trivially
      and gives no evidence it would catch a reintroduction.
- [ ] The plan records the empirical oracle mapping as a **query × (fixture,
      start directory) table in which every cell carries the exact command
      invoked and its verbatim output**, every expected value in the test suite
      is traceable to a cell, and any adapter/oracle disagreement beyond the
      pre-authorised `GIT_DIR` scrub is listed in Validation Results with a
      justification. Without this, a verifier can confirm each cell has a value
      but not that the value came from observation rather than a third round of
      inference — the failure the deferral exists to prevent.
- [ ] The committed lockfile check asserts: `gix` resolves to **0.85.x** (not
      merely to a single version — because `gix` is optional in `jj-lib`, a
      single-version assertion holds vacuously if that feature is off); no `gix`
      or `gix-*` package at more than one version; `jj-lib` at 0.43;
      `mise.toml`'s
      `jj` pin at the same minor version as the `jj-lib` pin, with its lockstep
      comment present; and no TLS stack (`openssl-sys`, `native-tls`, `rustls`,
      `curl-sys`). Asserted directly because the repo's duplicate-version policy
      is warn-level.
- [ ] `cargo deny` passes with the `uluru` exception, and both it and the `gix`
      pin carry their inline comments; `cargo-pup` passes and the new module
      rule is demonstrably non-vacuous (a deliberately added `std::process`
      import fails it); clippy passes `--locked`; the reference artefact
      cross-compiles to musl and passes `_assert_static_elf`.
- [ ] Nothing existing changed: `CommandProbe`/`MarkerWalkRoot` still exist with
      no new methods, `cli/corpus-adapters` still resolves through them and its
      metadata parity suite passes unchanged, `cli/vcs/src/**` is unmodified,
      and `cli/vcs-adapters` gains no `[features]` entry beyond `bash-parity`
      and no runtime port selection.
- [ ] The shared zero-spawn fixture is proven across a crate boundary **inside
      this story** by a test in `cli/corpus-adapters` — the crate 0185 will
      extend — that runs a full strong-form assertion end to end (stubs *and*
      shadow list *and* empty-config environment) through the fixture's public
      API with no fixture-private helpers. A test consumer that imports only one
      of the three parts would satisfy looser wording while leaving exactly the
      restructuring risk this exists to retire.
- [ ] Cost is **measured and recorded, not gated** — 0169 owns the warm-call
      latency gate (`G ≤ 1.1 × B`, where **B** is the baseline shell
      `hooks/vcs-guard.sh` invocation and **G** the migrated `accelerator vcs
      guard` one, B ≈ 35 ms) and needs comparable numbers. Against a **pure-jj
      fixture defined here** — one main jj workspace, no `.git`, a single
      commit, workspace root three directories below the temp root, committed as
      a **named reusable builder** so 0169 can reconstruct it identically —
      median of 20, reporting library initialisation cost, warm per-call
      in-process cost, and cold per-process cost via the reference artefact.
      The last is required because an in-process microbenchmark yields
      microsecond figures that cannot be compared with 0169's millisecond
      process-level baseline; it is the figure that corresponds to **G**, so the
      reference artefact needs a single-query mode for it — timing a binary that
      runs all six queries plus both port methods would inflate it by an unknown
      factor. The `MarkerWalkRoot`/`CommandProbe` baseline is taken for the port
      methods only (it has no queries and no library to initialise). Host and OS
      recorded.
- [ ] The reference artefact demonstrably links the dependency trees: its size
      **built with the query calls live** is at least **2 MB larger** than the
      same binary built with those calls replaced by stubs, both sizes recorded.
      Without a floor this check cannot fail, and it is the one guarding the
      story's headline false-pass — dead-code elimination letting the musl and
      size checks succeed while linking almost none of `gix`/`jj-lib`.
- [ ] Dated notes are appended to three siblings, **raising** information
      without re-scoping them:
      - **0125** — the adapter dissolves its lexical-fallback rationale *for
        consumers that reach it*, with the ~26 shell call sites still bound
        until later epic-0136 phases migrate them, plus a `relates_to` edge so
        the coupling is visible from both ends.
      - **0185** — the adapter and the zero-spawn harness are 0188's, not
        0169's; **0185's own** Summary, Context, Assumptions, Technical Notes
        and acceptance criteria all currently attribute them to 0169.
        `vcs_adapters::facts` stays hard-wired here by design, so the
        composition-root change falls to a dependant (see Open Questions), and
        the transitional dual-adapter `detection.rs` comparison must be
        collapsed when `CommandProbe` is deleted. Both affect its "wiring plus
        deletion" sizing.
      - **0169** — it inherits the closed six-query contract, must define its
        own port over inherent methods, must widen the pup rule for
        `status`/`log`, must reuse the pure-jj fixture builder, and its 0125
        hand-off sub-clause is now redundant.
- [ ] `mise run` is green end to end, including the shell suites that build jj
      fixtures, which were last green against the pre-bump `jj` 0.36 pin.

## Dependencies

- **Blocked by**: 0179 — delivered the `vcs`/`vcs-adapters` crate pair and the
  ports this story implements (**done**).
- **Blocks**:
  - **0169** — builds the subdomain's classification on these adapters, by
    defining its own domain port over the inherent queries delivered here. 0169
    carries a hard numeric gate (warm-call latency `G ≤ 1.1 × B`, ≈38.6 ms),
    which its own Dependencies already flag as at risk from a ~41 ms warm
    bootstrap. Whether it passes is largely set by this story's in-process
    discovery cost and by the sub-binary size the two new dependency trees
    produce (which also feeds the fetch/verify path) — hence the measurement
    criterion above. This story imposes no threshold on itself; it hands 0169
    the numbers.
  - **0185** — converges `corpus-adapters` onto these adapters and deletes
    `CommandProbe`. It also **consumes the zero-spawn harness built here** (the
    marker-writing stubs, the absolute-path shadow list and the empty-config
    environment), so that harness is exposed as a shared test fixture rather
    than a private helper, keeping one shadow list across both suites. 0185's
    own criteria currently attribute the harness to 0169 — stale since the
    split; a dated correction repointing it to 0188 is part of closing this
    story.
- **External systems**: `gix` (gitoxide) and `jj-lib`, both crates.io. `jj-lib`
  is pre-1.0 with an explicitly unstable API and this design leans on its loader
  internals, so a version bump can break detection — hence the exact `=0.43`
  pin. Adoption requires the `cli/deny.toml` licence exception above, a
  `cli/`-workspace-wide dependency-policy change.
  - **Ongoing cost, not just one-off**: both transitive trees enter `cargo
    deny`'s `advisories` scope, so a future RustSec advisory anywhere in the
    `gix`/`jj-lib` closure fails the workspace-wide check for every unrelated
    change. And because `gix` is pinned to whatever `jj-lib` depends on, any
    future `jj-lib` bump is a **coordinated two-crate bump** — the single-
    version criterion will otherwise fail it. Both pins carry inline comments
    saying so.
- **Upstream toolchain preconditions**:
  - **`jj` CLI ↔ `jj-lib` are now a lockstep pair.** The `bash-parity` fixtures
    are built by the installed `jj` CLI and read by `jj-lib`, so a skew between
    them fails in a way that reads as an adapter defect rather than a pin
    mismatch. `mise.toml` pinned 0.36.0 against this story's `jj-lib` 0.43 — a
    seven-version gap — so **the CLI pin was bumped to 0.43.0 (2026-08-02)**
    with an inline comment tying the two together. Consequences: `mise install`
    is required, and the shell suites that build jj fixtures
    (`hooks/test-vcs-detect.sh`, and the work-item script suite under
    `skills/work/scripts/`) were last green against 0.36 and must be re-run.
    This is a CI-wide change.
  - The symmetric git-side coupling holds too: the bare, linked-worktree and
    submodule fixtures are built by the installed `git` CLI (2.54.0) and read by
    the pinned `gix` 0.85. No format-boundary concern was identified, but both
    CLI versions are recorded in Validation Results.
  - **MSRV: resolved, fits** (2026-08-02). Pinned Rust 1.90.0; `jj-lib` 0.43
    needs 1.89, `gix` 0.85 needs 1.85. One minor version of headroom, and
    `jj-lib`'s MSRV has moved 1.85 → 1.88 → 1.89 over eight releases — so the
    coordinated bump below is really a **three**-pin coupling: `jj-lib`, `gix`,
    and the Rust toolchain.
- **Shared-artefact contention**: this story edits `cli/deny.toml`,
  `cli/pup.ron`, `cli/Cargo.lock` (committed in the same change, because clippy
  runs `--locked`), `mise.toml` (the `jj` pin, above), and **`tasks/build.py`**
  — `_assert_static_elf` and the cross-compile staging the reference artefact
  needs live there. Contending siblings: **0168**, whose workspace
  restructuring has in fact already landed — the residual contention is only
  its possible move of the `cli/visualiser/server/` crate path; and
  **0187**, which rewrites `validate_dispatch_coherence` in that same
  `tasks/build.py`. (0187 does *not* contend on the three `cli/` artefacts — an
  earlier draft listed it for the wrong reason.) More broadly, any epic-0136
  item adding crates under `cli/` contends on the lock. No ordering is imposed,
  but whichever lands second regenerates the lock rather than merging it, and
  must re-verify the single-`gix`-version invariant afterwards.
- **Related**: 0125 — this story dissolves its stated rationale for the shell
  lexical fallback without closing it. Because 0188 lands first and is the
  change that actually dissolves the rationale, **the hand-off note is owned
  here** (see Acceptance Criteria), not by 0169 as originally recorded. Note the
  dissolution is *conditional*: 0125's constraint is about `find_repo_root` and
  `vcs_mode` and their ~26 shell call sites, which keep running in bash and
  cannot reach this adapter until 0169 and the later epic-0136 phases migrate
  them — which is why the required note is worded conditionally.
- **Parent**: epic 0136.

## Open Questions

Two questions are open and gate planning. Two more are closed, kept for the
record because their answers carry consequences.

- **OPEN — who owns the CI job for the strong-form zero-spawn run, and does the
  runner actually permit it?** The criterion is deliberately non-degradable:
  the strong form (replacing or bind-mounting `/usr/bin/git` and friends) must
  run somewhere, and "somewhere" is currently a Linux CI job that may not exist
  and whose provider may forbid modifying `/usr/bin` without privileged
  containers. This is the one gate in the story resting on an unconfirmed
  infrastructure capability, and it has been raised in three consecutive
  reviews. **Confirm in planning.** *Default if it cannot be arranged*: carve
  the provisioning into its own small item that 0188 declares as `blocked_by`,
  rather than silently weakening the criterion to `PATH`-only everywhere —
  which would leave the property unproven on any platform.
- **OPEN — who changes `vcs_adapters::facts`, 0169 or 0185, and in what order?**
  This story deliberately leaves it hard-wired. Requirements say wiring
  "belongs to 0169 and 0185"; the 0185 hand-off says the composition-root change
  is a dependant's own work. Both are currently unblocked by 0188 alone, and
  0185's Technical Notes assume 0169 goes first while its `blocked_by` does not
  say so. **Assign one owner and record the ordering** before either is picked
  up; leaving it implicit reintroduces the hidden ordering the split removed.

- **Does `jj-lib` 0.43's (and `gix` 0.85's) MSRV fit the repo's pinned Rust
  toolchain?** **ANSWERED 2026-08-02 — it fits; no bump needed.** Pinned
  toolchain is Rust **1.90.0** (`mise.toml:8`); `jj-lib` 0.43.0 declares MSRV
  **1.89** and `gix` 0.85.0 declares **1.85** (crates.io). The margin is one
  minor version, and `jj-lib`'s MSRV has moved 1.85 → 1.88 → 1.89 across its
  last eight releases, so a future `jj-lib` bump will likely drag the Rust pin
  with it — recorded as the three-pin coupling in Dependencies.
- **Does the installed `jj` CLI write a repository format `jj-lib` 0.43 can
  read?** **RESOLVED BY ALIGNMENT 2026-08-02** — the question is retired rather
  than answered. `mise.toml:12` pinned `jj = "0.36.0"` against this story's
  `jj-lib` 0.43, a seven-minor-version gap the 2026-07-29 research never
  exercised (it records the crate versions but not which CLI built its probe
  fixtures). Rather than measure the skew, the CLI pin was **bumped to 0.43.0**
  with an inline comment tying it to the crate pin, so CLI and library now match
  and the coherence risk is designed out. Consequences carried into Dependencies
  → Upstream toolchain preconditions: the two pins are now a lockstep pair, and
  the shell suites that build jj fixtures were last green against 0.36.

## Assumptions

- Feasibility is **measured, not assumed** (2026-07-29): `gix 0.85` + `jj-lib
  0.43` pass `cargo deny` for `bans`, `advisories` and `sources` against the
  current `cli/deny.toml`; `jj-lib` no longer depends on `git2`/`libgit2-sys`;
  and a binary calling both cross-compiles to a statically linked musl ELF that
  `_assert_static_elf` accepts. The only licence rejection was `uluru`.
- **Dependency facts re-verified 2026-08-02** against the crates.io index:
  `jj-lib` 0.43.0 requires `gix ^0.85.0`, so "pin `gix` to the version `jj-lib`
  depends on" resolves to 0.85 as stated — and the requirement is load-bearing,
  because `gix` **0.86.0 now exists** and a caret range on a `0.x` crate will
  not cross it, so pinning 0.86 here would produce exactly the two graphs the
  requirement forbids. `jj-lib` 0.43.0 also depends on `gix-ignore ^0.21.0`
  (non-optional), and carries no `git2`/`libgit2-sys` — confirming the
  2026-07-29 finding.
- **`gix` is an *optional* dependency of `jj-lib` 0.43** (feature-gated). The
  single-graph requirement is written as though `jj-lib` always pulls `gix`; if
  the gating feature is off in our configuration, `jj-lib` may pull no `gix` at
  all and this story's direct dependency is the only one. That changes the
  *reasoning* behind the pin, not the pin itself — but the plan should state
  which `jj-lib` features are enabled and re-check the single-version assertion
  under that configuration.
- `jj-lib`'s loader API remains stable across **the pinned `jj-lib` version,
  0.43**, which the exact pin holds fixed. Verified against 0.43; unstable by
  the crate's own declaration.

## Terminology

- **`classify_checkout`** — the bash function in `scripts/vcs-common.sh` that
  produces the current checkout taxonomy, emitting a `KEY=VALUE` record. It is
  the behavioural oracle this story's queries are tested against.
- **`BOUNDARY`** — that record's field for the active workspace root. **Not a
  general-purpose root**: the contract documents it as empty for the `main` and
  `none` arms (`vcs-common.sh:165`), and the `nested-git-in-jj` arm sets it to
  the *git* worktree root (`:259`). It is therefore not a valid oracle for
  "the jj workspace root" outside the `jj-secondary` arm.
- **`JJ_PARENT`** — that record's field for the main jj repository directory,
  which differs from `BOUNDARY` in a secondary workspace.
- **"probe"** — used bare in this document **only** for the 2026-07-29
  feasibility experiment. The identifiers `VcsProbe` (the port) and
  `CommandProbe` (the retained subprocess adapter) are always written in full.

## Technical Notes

- **Starting points for the oracle mapping, not the mapping itself.** Per the
  Acceptance Criteria preamble the mapping is established empirically in
  planning; these are leads to test, not answers to assert. Git side:
  `is_bare` (`vcs-common.sh:206`), the `--git-dir` vs `--git-common-dir`
  worktree comparison (`:217-219`), superproject resolution
  (`find_git_main_worktree_root`, `:127-155`). Note the first two are inline
  locals that never reach the emitted record, and the third returns an ordinary
  root for non-submodules — so in both cases the underlying `git rev-parse`
  invocation, not the shell wrapper, is the likely oracle. jj side: the
  `.jj/repo` dir-vs-file secondary rule (`_jj_workspace_is_secondary`, `:74-81`)
  and the record fields (contract `:164-171`, emitted `:274-279`).
- `jj_lib::workspace::DefaultWorkspaceLoaderFactory` is public, and its loader
  implements the shell's `.jj/repo`-file-means-secondary rule verbatim (jj-lib
  0.43 `src/workspace.rs:564-585`). The 2026-07-29 experiment probed it against
  colocated, secondary, plain and nested fixtures and reported that
  `workspace_root()` equals `BOUNDARY` and `repo_path()` minus `/.jj/repo`
  equals `JJ_PARENT`. **Treat the `BOUNDARY` half as holding only for the
  `jj-secondary` arm** — see the Terminology caveat; the equality cannot hold
  where the shell emits an empty `BOUNDARY`. Re-establish both in planning.
- The existing `bash-parity` feature gate means "needs real `jj`/`git` binaries
  to build fixtures", not "shells out in production" — it stays relevant.
- **`GIT_CEILING_DIRECTORIES` in fixtures: yes, with one exception.** Set it (as
  `hooks/test-vcs-detect.sh:35-40` does) so a stray `.git` above the temp dir
  cannot leak into a probe — *except* in the boundary-containment fixture, where
  it must be unset or set above the parent repository, or the environment rather
  than the adapter would be what stops the walk and the criterion would pass
  vacuously.

## Validation Results

- **Platform the strong-form zero-spawn run held on** — _pending_; absolute
  paths shadowed there — _pending_; paths not shadowable on each other platform
  — _pending_.
- **`gix` / `gix-*` versions resolved in `Cargo.lock`** — _pending_.
- **`jj-lib` version resolved in `Cargo.lock`** — _pending_.
- **MSRV of `gix` 0.85 and `jj-lib` 0.43 vs the pinned Rust toolchain** —
  _resolved 2026-08-02_: pinned Rust 1.90.0; `jj-lib` 0.43.0 MSRV 1.89; `gix`
  0.85.0 MSRV 1.85. Both fit, with one minor version of headroom.
- **Installed `jj` CLI version the fixtures were built with** — _bumped
  2026-08-02_: `mise.toml` now pins **0.43.0**, matching the `jj-lib` crate pin
  (prior state: 0.36.0, seven minor versions behind — the skew this bump
  designs out). Outstanding: `mise install` on each machine and CI, and a
  re-run of the jj-fixture shell suites, which were last green against 0.36 —
  _pending_.
- **Installed `git` CLI version the fixtures were built with** — _recorded
  2026-08-02_: 2.54.0, against `gix` 0.85. Coherence unverified but low-risk;
  no format-boundary concern was identified.
- **Adapter/shell divergences recorded** — the `GIT_DIR` scrub asymmetry in
  `classify_checkout` (`:206-215` unscrubbed vs `:130-135` scrubbed), which the
  adapter deliberately does not reproduce — _confirmed 2026-08-01_; any further
  divergence found in implementation — _pending_.
- **Non-vacuity demonstrations** — the deliberate `std::process` import failing
  cargo-pup — _pending_; the deliberate `UserSettings` construction failing the
  source guard — _pending_; the unscrubbed control diverging under the poisoned
  `GIT_DIR` run — _pending_.
- **Cost, against this story's pure-jj fixture** (reused by 0169; median of 20,
  host and OS): library init — _pending_; warm per-call in-process — _pending_;
  cold per-process via the reference artefact — _pending_; `CommandProbe`
  baseline for the port methods — _pending_.
- **Reference artefact size**, linked vs stubbed builds and the delta —
  _pending_.

## References

- Extracted from: `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
- Split rationale (scope lens, pass 4):
  `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md`
- Feasibility probe, API findings and the two traps:
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §9
- Behavioural oracle for the taxonomy queries: `scripts/vcs-common.sh`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0053 (thin CLI over a hexagonal ports-and-adapters core)
