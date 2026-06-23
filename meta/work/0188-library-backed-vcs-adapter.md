---
type: work-item
id: "0188"
title: "Library-Backed VCS Adapter over gix and jj-lib"
date: "2026-07-31T10:41:51+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: story
priority: high
parent: "work-item:0136"
blocked_by: ["work-item:0179"]
blocks: ["work-item:0169", "work-item:0185"]
relates_to: ["work-item:0125", "work-item:0168", "work-item:0187",
  "codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration"]
derived_from: ["work-item:0169"]
tags: [rust, vcs, dependencies]
last_updated: "2026-08-05T17:36:44+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-718
---

# 0188: Library-Backed VCS Adapter over gix and jj-lib

**Kind**: Story
**Status**: Done
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

**Amended 2026-08-03**, on measurement rather than inference — ten changes, each
recorded inline where it applies. Amendments 1-4 came from the planning
measurement pass; 5-6 from plan review; 7-8 from the pin decision and the
`revision` spike; 9-10 from implementation and validation. **Amendment 8 is
withdrawn by amendment 10** — the spike behind it was wrong, and the jj
`revision` mechanism it descoped is delivered here.

1. The **reference-artefact size floor** was mis-calibrated and would have failed
   on the artefact it guards; it is now a ratio floor plus a
   headroom-bearing absolute floor with its unit stated (Acceptance Criteria).
2. **Three walks are required, not one** — queries 4, 5 and the jj half of 6
   need a `.jj`-only walk, because `jj-lib`'s loader does not walk (Delivered
   surface, and the Library trap on `Workspace::load`).
3. The **"colocated" fixture row is two distinct shapes**, the submodule row is
   three, and `jj git init` **colocates by default** at 0.43 so the pure-jj
   builder needs `--no-colocate` (Acceptance Criteria, Test-support
   deliverables).
4. The **single-query-mode rationale was measurably false** — process startup
   dominates, so the aggregate figure is not inflated by an unknown factor
   (Acceptance Criteria).
5. The **size floor needs a mechanism, not just a number.** Amendment 1 fixed the
   threshold but left it a figure recorded in Validation Results, which catches no
   later regression. It is now a committed assertion, with the absolute-byte floor
   scoped to musl triples and a host-native ratio check on the PR path
   (Acceptance Criteria).
6. **The six queries return `Result<Option<T>, Error>`, not `Option<T>`.**
   `Option` alone conflates "no repository of this kind here" with "a repository
   is here and the pinned pre-1.0 library could not parse it", and silently drops
   `CommandProbe`'s time cap, crash isolation and warn-logging when 0185 switches
   the composition root. `dual_roots` is infallible and carries a `Result` per
   side, since a whole-struct `Result` cannot express one-sided failure on the
   field that discriminates `colocated` from `nested-*` (Delivered surface).
7. **The pins are asymmetric and the coupling is four-way, not two-way.**
   `jj-lib` stays exact at `=0.43.0`; `gix` becomes `~0.85.0`, permitting
   `0.85.x` patches so a RustSec fix is a lock update rather than a pin edit — the
   range is anyway identical to jj-lib's own `^0.85.0`, so the single-graph
   property is untouched. The coupling is jj-lib + gix + the Rust toolchain + the
   `mise.toml` jj CLI pin (Dependency policy, Dependencies).
8. ~~**jj `revision` is out of scope; the type implements `VcsProbe` partially.**~~
   **WITHDRAWN 2026-08-03 — superseded by amendment 10.** The spike it rested on
   was wrong. The crate-wide `UserSettings` guard does stay crate-wide, for the
   better reason that the delivered route needs no settings either.
9. **`gix` takes `default-features = false`, not default features.** gix's
   `default` reaches `extras` → `credentials` → `gix-credentials`, the one gix
   subsystem that spawns `git credential-*` helper programs — against a module
   whose whole point is reading git without a subprocess. Nothing is lost:
   `jj-lib`'s own selection still enables `attributes`, `blob-diff`, `index`,
   `max-performance-safe`, `sha1` and `zlib-rs`, and the graph test asserts each
   one present and the network-client family absent. This also makes the
   `gix-credentials` ban a live guard against a later feature widening rather
   than a statement about a crate that could never appear (Dependency policy).
10. **jj `revision` is in scope after all; the type implements `VcsProbe` fully.**
    Amendment 8's spike premise was **false**: `jj_lib::protos` is a public
    module, so the workspace's checkout state (`operation_id` +
    `workspace_name`) decodes through published API rather than a private wire
    format, and `SimpleOpStore::load` takes a path only — no settings anywhere.
    The chain is delivered and verified against the live CLI across pure-jj,
    colocated, commitless, secondary-workspace and multi-workspace shapes, and
    fingerprint-asserted to write nothing. `prost` and `pollster` join the pins
    as jj-lib-coupled direct edges (already in the lock, so two new edges and no
    new packages), making the pin coupling **six-way**. The one divergence from
    the `jj` binary is that the binary snapshots the working copy first — and
    writes — where this route reports the last recorded commit; that is the
    read-only direction and is pinned by a test (Delivered surface, Dependency
    policy, Acceptance Criteria).
9. **`gix` takes `default-features = false`, not default features.** gix's
   `default` reaches `extras` → `credentials` → `gix-credentials`, the one gix
   subsystem that spawns `git credential-*` helper programs — against a module
   whose whole point is reading git without a subprocess. Nothing is lost:
   `jj-lib`'s own selection still enables `attributes`, `blob-diff`, `index`,
   `max-performance-safe`, `sha1` and `zlib-rs`, and the graph test asserts each
   one present and the network-client family absent. This also makes the
   `gix-credentials` ban a live guard against a later feature widening rather
   than a statement about a crate that could never appear (Dependency policy).

Two assumptions also resolved in the same pass: `gix`'s gating feature *is* on
under `jj-lib`'s defaults, so the single-graph reasoning holds; and the
`GIT_DIR` scrub invariant holds for free on **both** library sides, making it a
property to verify rather than implement. A third correction from plan review:
that invariant was verified over `GIT_DIR`/`GIT_COMMON_DIR` only, so "uniformly
immune" overstated it — the verification set is widened to everything
`scrub_environment` touches plus the object-directory and `GIT_CONFIG_COUNT`
families.

Plan: `meta/plans/2026-08-03-0188-library-backed-vcs-adapter.md`, which carries
the full empirical oracle mapping.

### Delivered surface

- **One library-backed type** in `cli/vcs-adapters` implementing *both* the
  `vcs` crate's `RepoRoot` and `VcsProbe` ports over `gix` and `jj-lib`,
  alongside the retained `MarkerWalkRoot`/`CommandProbe` pair. A single type
  avoids having to say which of a pair carries the dual-root query, which needs
  both libraries. **The domain crate `cli/vcs` is untouched: no port is added,
  widened or changed.**

  **Amended 2026-08-03 (validation) — `VcsProbe` is implemented *fully*.** `kind`
  is complete; `revision` answers for both idioms. The git half is
  `gix::discover(root)?.head_commit()?.id()`. The jj half reads the workspace's
  checkout state — `<workspace_root>/.jj/working_copy/checkout`, decoded as
  `jj_lib::protos::local_working_copy::Checkout`, a **public** module — for the
  operation id and the workspace's own name, then reads that operation's view out
  of `SimpleOpStore::load(<repo>/op_store, ..)` and looks the commit up in
  `View.wc_commit_ids` by name. No settings value is constructed anywhere, so the
  crate-wide guard is untouched, and the route writes nothing (fingerprint-asserted
  over the whole `.jj` tree). The name lookup is load-bearing: two workspaces of
  one repository hold different working-copy commits, so taking the view's sole
  entry would answer for the wrong one.

  An earlier amendment (8, now withdrawn) descoped the jj half to 0185 on a spike
  finding that no read-only, settings-free route existed. That finding was wrong —
  it treated the checkout protobuf as a private wire format when `jj_lib::protos`
  is public, and it recorded `UserSettings::from_config` as unbounded panics when
  it returns `Result` and needs exactly five keys. **Nothing transfers to 0185**
  except the recorded divergence: asking the `jj` binary snapshots the working
  copy first — and writes a new commit — where this route reports the commit as of
  the last recorded operation. That is the read-only direction, and after 0185's
  switch metadata derivation stops mutating the user's repository.
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
     truncated by the other's marker, so a consumer can compare them. This
     comparison is **necessary but not sufficient** to separate `colocated` from
     `nested-jj-in-git` and `nested-git-in-jj`; 0169 cannot build those arms
     without it, and cannot build them from it alone.

  **Amended 2026-08-03 (measurement) — dual-root equality is not on its own a
  classification.** An earlier framing implied roots-equal ⇒ `colocated`. It does
  not: a real `jj git init --colocate` main repository has equal roots and
  classifies as `main`, because `classify_checkout`'s `colocated` arm also
  requires `jj_secondary && git_worktree` (`scripts/vcs-common.sh:242-247`).
  Inequality is likewise insufficient — this repo's own `workspaces/<name>`
  layout has *differing* roots and classifies as `jj-secondary`, because the
  `nested-jj-in-git` arm additionally requires
  `jj_main_root != git_main_root` and there they are equal. A consumer reads this
  query together with queries 2 and 5, never alone.

  **Amended 2026-08-03 (plan review) — the queries return
  `Result<Option<T>, Error>`.** `Ok(None)` means "no repository of this kind
  here"; `Err` means "a repository is here and the pinned library could not
  answer". `Option` alone would collapse a corrupt object store, an unreadable
  `.jj/repo` or an unsupported repository format into the same value as genuine
  absence — a regression against `CommandProbe`, which parses in a child process
  with a 10-second cap, kill-on-timeout and a scrubbed environment and warn-logs
  every distinct failure. `InProcessProbe` parses repository-controlled data in
  the caller's address space with no time or memory bound and no crash isolation,
  and after 0185's switch that runs inside `cli/visualiser/server` and on the hook
  path, so the distinction is load-bearing rather than stylistic. The partition
  rule: only the not-found-shaped variant of each library error maps to
  `Ok(None)`; every other variant is `Err`. `dual_roots` is **infallible with a
  `Result` per side** — a whole-struct `Result` cannot say "the git side failed but
  the jj side answered", and flattening that to `None` would reinstate the
  conflation on the one field the nested/colocated distinction rests on. The two
  *port* methods keep their `Option` signatures, since `cli/vcs` is untouched;
  they warn-log instead.

  **Three walks are required, not one** (established 2026-08-03 by measurement;
  amends the single-boundary-walk framing this section previously carried):

  1. The **combined `.jj`-or-`.git` boundary walk** — `RepoRoot::discover` only.
     This is `MarkerWalkRoot`'s existing algorithm.
  2. A **`.jj`-only walk** — queries 4, 5 and the jj half of query 6. This is
     what `jj workspace root` answers, and the combined walk is **wrong** for
     them: `jj_lib`'s `DefaultWorkspaceLoaderFactory::create` performs no upward
     walk of its own, so feeding it the combined boundary makes it return
     `Err(There is no Jujutsu repo in …)` on both nested-git-in-jj shapes —
     where the oracle reports a root. A classifier built on the combined walk
     would therefore report absence where `jj workspace root` reports presence.
  3. **`gix`'s own discovery** (`gix::discover`) — queries 1, 2, 3 and the git
     half of query 6. Its willingness to walk above the boundary is *required*
     here, and is why the boundary rule below scopes to boundary resolution
     only.

  **Invariant over all six**: every query answers identically whether or not
  `GIT_DIR`/`GIT_COMMON_DIR` are set. This matches the one place the shell made
  the decision deliberately (`vcs-common.sh:130-135`, "cannot be poisoned by
  ambient env") and diverges from `classify_checkout`'s inline reads, which
  honour them. The shell is internally inconsistent here — `:206-215` unscrubbed
  against `:130-135` scrubbed — and it is that asymmetry the adapter declines to
  carry forward.

  **The invariant is a property to verify, not to implement** (measured
  2026-08-03, both sides). It holds *for free* across the whole fixture matrix:
  `gix::discover`/`gix::open` do not consult the environment (only
  `discover_with_environment_overrides` does), and `jj_lib`'s workspace loader is
  pure filesystem reads. An earlier draft of this criterion required an explicit
  scrub "for the duration of any detection call"; no such scrub is needed, and
  adding one would be dead code. `discover_with_environment_overrides` is the
  ready-made non-vacuity control the acceptance criteria demand.

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
- **The checkout fixture matrix** — thirteen shapes × their start directories
  (see the criterion; the count rose from ten on 2026-08-03 when the colocated
  and submodule rows were split). Bare, all three submodule shapes,
  linked-worktree, both colocated shapes and both nesting shapes are substantial
  to build; planning should record which already exist in `detection.rs` and
  which are new.
- **A pure-jj benchmark fixture**, committed as a named reusable builder,
  plus the cost measurements taken against it. Handed to 0169 ungated. Built
  with **`jj git init --no-colocate`** — see the colocation-default note under
  the cost criterion.
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

- **Never call `gix::discover` on the boundary path.** `gix::discover` walks
  up *past* a jj workspace boundary (verified 2026-07-29: it returned the parent
  repository's `.git` from inside `workspaces/build-system`). The rule:

  > The checkout boundary is the start path itself, or its nearest ancestor,
  > containing a `.jj` or `.git` marker. `RepoRoot` reports that path and never
  > an ancestor above it, whether or not the environment supplies a ceiling.

  **The mechanism is a marker walk followed by `gix::open` at exactly the
  boundary path** — `gix::open` performs no upward walk and errors cleanly when
  the path is not a git repository, which is precisely the required behaviour.
  `MarkerWalkRoot` already implements the walk, so the boundary rule is
  satisfied by composition rather than by new code.

  **A ceiling cannot enforce this rule and must not be relied on** (spike
  2026-08-03): `gix_discover::upwards::Options::ceiling_dirs` computes the
  ceiling height as `start.strip_prefix(ceiling).components().count()` and
  **discards height 0**, so a ceiling equal to the boundary is silently ignored;
  the walk's bound is then checked as `current_height > max_height` with the
  counter incremented after the test, permitting one further level of ascent. No
  anchor — boundary, boundary's parent, or the filesystem root — confined the
  walk in testing; every configuration either errored uninformatively or
  returned the parent repository. An inherited `GIT_CEILING_DIRECTORIES` is
  irrelevant on this path, because plain `gix::discover` does not read the
  environment at all (only `discover_with_environment_overrides` does).

  **The rule scopes to boundary resolution only** — queries 2, 3 and 6
  legitimately resolve outside the boundary by following a recorded link
  (gitdir, superproject) or by letting each library complete its own walk. An
  earlier draft forbade looking above the boundary at all, which would have made
  0169's `nested-*` arms unimplementable.

  **Consequence for query 1**: a bare repository has neither a `.jj` nor a
  `.git` marker, so the marker walk returns `None` for it (confirmed
  2026-08-03) while `gix::open` on the bare directory reports `is_bare = true`.
  Bare detection therefore cannot take its start path from the boundary walk and
  needs its own entry point. This matches today's behaviour, which
  `cli/vcs-adapters/tests/detection.rs:250-277` pins as `facts(&bare) == None`.
- **Avoid `jj_lib::workspace::Workspace::load`.** It needs a fully-populated
  `UserSettings` whose defaults are private to jj-lib, discovered one panic at a
  time. `DefaultWorkspaceLoaderFactory` is public and
  `WorkspaceLoader::{workspace_root, repo_path}` need no settings. **No code in
  `cli/vcs-adapters` may construct a `UserSettings` at all** — stated
  crate-wide, deliberately wider than the detection paths strictly require, so
  the guard is a simple one.

  **Confirmed 2026-08-03 (validation) — the crate-wide statement holds, and jj
  `revision` is delivered without narrowing it.** The spike that first looked at
  this found `LocalWorkingCopy::load` needs `&UserSettings` for
  `TreeStateSettings`; `CheckoutState` (which holds the `operation_id` +
  `workspace_name` pair) is private; and `SimpleWorkspaceStore::load`, though
  settings-free, **creates `.jj/repo/workspace_store` and writes an `index`
  file** — verified by removing the directory and observing it recreated. All
  three are true, and all three are avoidable: the workspace name those routes
  were reaching for is recorded in the workspace's own `checkout` file, whose
  schema is public (`jj_lib::protos::local_working_copy::Checkout`), and the op
  stores are settings-free (`SimpleOpStore::load(path, root_data)`). The chain
  never needed the blocked links. So the guard costs nothing in scope and needs
  no narrowing — now shown by a delivered mechanism rather than by the absence of
  one.

  **The loader does not walk** (verified 2026-08-03 against jj-lib 0.43
  `src/workspace.rs:564-585`): `create(workspace_root)` demands a path whose
  `.jj` is a directory and errors otherwise, so the caller must supply the
  workspace root — see the `.jj`-only walk in the three-walk requirement above.
  Note also that `repo_path()` comes back already canonicalised
  (`dunce::canonicalize`) while `workspace_root()` is the path passed in, so both
  sides need canonicalising before the secondary-vs-main comparison.

### Dependency policy

- **`jj-lib` pinned exactly at `=0.43.0`.** The loader-internals design and every
  feasibility measurement were validated against it, and the crate declares its
  API unstable. `mise.toml`'s `jj` CLI pin is held in lockstep at 0.43.0 so the
  CLI that writes fixtures and the library that reads them cannot skew. **The
  bump landed 2026-08-02**; what remains for this story is `mise install` in CI
  and a re-run of the jj-fixture shell suites.
- **`gix` pinned to `~0.85.0`** — the version `jj-lib` 0.43 depends on when its
  `gix` feature is enabled (verified 2026-08-02: `jj-lib` 0.43.0 declares
  `gix ^0.85.0`, optional). See the Assumption on feature-gating.

  **Amended 2026-08-03 (plan review) — the two pins are deliberately asymmetric.**
  `gix` takes a tilde range rather than `=`, permitting `0.85.x` patches so a
  RustSec fix is a lock update instead of a pin edit. That matters because
  `cli/deny.toml` sets `unmaintained = "all"` and `yanked = "deny"` over a
  ~60-crate closure no repo code calls, and an exact pin would make an
  `advisories.ignore` entry the cheapest response to an advisory. The
  single-graph property is untouched: it comes from jj-lib's own `^0.85.0` plus
  cargo's range unification, not from exactness, and `Cargo.lock` supplies the
  exactness either way — `~0.85.0` is range-identical to `^0.85.0` for a 0.x
  crate. `jj-lib` keeps `=` because the agility argument does not apply to the
  crate whose declared-unstable internals this design depends on, and because a
  transitive advisory inside *its* closure is adoptable with
  `cargo update -p <crate>` without touching the pin. **The coupling is four-way,
  not two-way**: jj-lib + gix + the Rust toolchain + the `mise.toml` jj CLI pin.
  Load-bearing: `gix`
  0.86.0 exists, and a caret range on a `0.x` crate will not cross it, so
  pinning 0.86 here would produce exactly the two graphs this forbids. The pin
  **must** carry an inline comment saying it tracks `jj-lib`'s `gix` version.
- **A `[[licenses.exceptions]]` entry for `uluru`** (MPL-2.0) in
  `cli/deny.toml` — `gix-pack`'s LRU cache, not feature-gatable. It **must**
  carry an inline comment citing this work item.
- **`gix` with `default-features = false`** (*amendment 9*) — no TLS stack enters
  the graph either way, but gix's `default` reaches `extras` → `credentials` →
  `gix-credentials`, which spawns `git credential-*` helper programs. `jj-lib`'s
  own feature selection supplies everything the adapter calls.
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
      library-backed type**, with the suite's existing **fixed expected values**
      retained — agreement between two implementations is not on its own an
      oracle. The `.git`-as-file worktree case keeps **today's** value
      (`classify_checkout` reports `main`, the git side unseen); 0169 owns
      correcting that to `colocated`. This dual comparison is
      **transitional**: 0185 deletes `CommandProbe`, and collapsing the suite to
      the library-backed type alone is part of that deletion.

      **Amended 2026-08-03 (validation) — parity is full `RepoFacts` equality
      for every `VcsKind`.** An earlier amendment narrowed it per `VcsKind`, on
      the descope that amendment 10 withdraws. With the jj `revision` route
      delivered there is no field either adapter cannot answer, so all 7 cases
      assert whole-struct equality. The one shape where the two legitimately
      differ — an unsnapshotted edit, where asking the `jj` binary snapshots the
      working copy and writes a new commit — is pinned separately in `library.rs`
      rather than relaxed here; every fixture in this suite is built and then read
      with no intervening edit.

      Related, measured 2026-08-03: a **sha256** repository's `HEAD` is 64 hex
      characters, so `is_full_revision_id`'s 40-hex assertion rejects it. Any
      revision validation must accept both widths or record sha256 as
      unsupported — a decision 0185 inherits, since its switch is what exposes
      users to it.
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
      | colocated, **real** (`jj git init --colocate`) | root |
      | colocated, **hand-grafted** (jj secondary + git linked worktree at one path) | root |
      | jj secondary workspace | workspace root, and a subdirectory |
      | plain git | root, and a subdirectory |
      | nested-jj-in-git | inner jj workspace, and the outer git root |
      | nested-git-in-jj | inner git repo, and the outer jj root |
      | linked git worktree | the linked worktree, and the main worktree |
      | git submodule | the submodule, and the superproject root |
      | git submodule, **nested** (depth 2) | the depth-2 submodule |
      | git submodule, **old form** (nested `.git` directory) | the nested repo |
      | bare repository | the bare dir |
      | no repository at all | a dir with no marker at or above it |
      | pure-jj measurement fixture | root |

      **Amended 2026-08-03 (measurement).** The row previously read "colocated |
      root", which is ambiguous between two genuinely different shapes, and both
      are now required. A real `jj git init --colocate` main repository has
      `.jj/repo` as a *directory* and `gix` reports `Kind::Common`, so the shell
      classifies it as **`main`** — the `colocated` arm requires
      `jj_secondary && git_worktree` (`vcs-common.sh:242-247`). The shell suite's
      hand-grafted fixture (`hooks/test-vcs-detect.sh:96-157`) is a jj secondary
      workspace whose path is simultaneously a git linked worktree, reports
      `Kind::LinkedWorkTree`, and is the only shape that classifies as
      **`colocated`**. The two submodule rows are likewise split out: the
      depth-2 shape is what proves the superproject derivation takes the
      *nearest* `modules` component, and the old-form shape reports
      `Kind::Common` — agreeing with git, which also reports no superproject —
      so a `Kind::Submodule`-only implementation would silently miss it with no
      oracle disagreement to reveal the omission.

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
      process-level baseline; it is the figure that corresponds to **G**. The
      reference artefact carries a single-query mode for it. The
      `MarkerWalkRoot`/`CommandProbe` baseline is taken for the port methods only
      (it has no queries and no library to initialise). Host and OS recorded,
      and **darwin-arm64 is the required host** so the figures are comparable
      with 0186's `B = 35.1 ms` — the ~97 ms probe delta is macOS-specific, so a
      cross-host comparison would leave 0169's gate ill-posed.

      **Amended 2026-08-03 (measurement), two corrections.**

      *The single-query rationale was wrong.* This criterion previously said the
      single-query mode was needed because "timing a binary that runs all six
      queries plus both port methods would inflate it by an unknown factor."
      Measured: `all` (six queries + both ports) **3.66 ms** against
      `only <query>` **3.65 ms** on the pure-jj fixture, and 4.49 ms vs 3.65 ms
      on plain git — process startup dominates, so the inflation is 0–23%, not
      unknown. The mode is retained because 0169 will want a per-query figure,
      not because the aggregate figure would be unusable.

      *The fixture needs an explicit flag.* `jj git init` **colocates by default
      at jj 0.43** — a bare `jj git init` in an empty directory produces both
      `.jj` and `.git` — so "no `.git`" is not the default and cannot be assumed.
      The builder must pass **`--no-colocate`** and assert `.git` is absent.
      Consequence for the existing suites: `make_main_jj_workspace`
      (`hooks/test-vcs-detect.sh:47-52`) has been producing *colocated*
      repositories all along and is misnamed; several fixtures built on it are
      not the pure-jj shapes their names suggest.
- [ ] The reference artefact demonstrably links the dependency trees: **built
      with the query calls live** it is at least **3× the size** of the same
      artefact built with those calls replaced by stubs, **and** at least
      **1,500,000 bytes larger** — measured on the **musl-static, stripped**
      artefact, which is the one the release pipeline actually ships, with the
      darwin figures recorded alongside. Both absolute sizes, the delta and the
      ratio are recorded. Without a floor this check cannot fail, and it is the
      one guarding the story's headline false-pass — dead-code elimination
      letting the musl and size checks succeed while linking almost none of
      `gix`/`jj-lib`.

      **Amended 2026-08-03 (measurement).** This criterion previously demanded a
      flat "at least 2 MB larger" and, as written, **would have failed on the
      artefact it exists to guard**. Measured against a prototype linking
      `gix` 0.85 + `jj-lib` 0.43: musl-static stripped delta **2,031,448 B**,
      darwin stripped **1,639,872 B**, darwin unstripped 2,058,656 B. So the old
      floor passed only on a decimal-MB reading of the musl build, by 1.6%, and
      failed outright on the stripped darwin build and on every MiB reading —
      while the trees were unambiguously linked (**ratio 6.19×** on musl, 5.42×
      on darwin). A ratio floor is the robust discriminator; the absolute floor
      is retained at a level with real headroom, with its unit stated, because
      the ratio alone would lose the "these trees are large" signal.

      **Amended again 2026-08-03 (plan review) — the floor needs a mechanism.**
      Amendment 1 fixed the threshold but left it a number written into Validation
      Results, which catches no later regression: a future edit that stops
      printing a query result would let the linker drop the trees again with
      nothing objecting. It is now a **committed assertion** in `tasks/build.py`'s
      fixture staging, with a unit test over the comparison function using
      synthetic sizes. Two scoping rules, because a heuristic threshold that first
      executes in the release pipeline can abort a whole product release: the
      **ratio** floor applies on every triple (wide margin — 5.42× darwin, 6.19×
      musl), while the **absolute-byte** floor applies to **musl triples only**,
      matching how `_assert_static_elf` is already guarded. The darwin stripped
      delta clears 1,500,000 B by just 9.3%, and `[profile.release] strip = true`
      means every triple is stripped, so gating darwin on it would put a
      9%-margin heuristic on `prerelease:prepare`'s critical path. A host-native
      ratio assertion also runs on the PR path, so the guard is not first
      exercised during a release.
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
    change. And because `gix` tracks whatever `jj-lib` depends on, any future
    `jj-lib` **minor** bump is a **coordinated four-pin change** — jj-lib, gix,
    the Rust toolchain (jj-lib's MSRV moved 1.85 → 1.88 → 1.89 across eight
    releases) and the `mise.toml` jj CLI pin, which writes the format jj-lib
    reads. The single-version criterion will otherwise fail it. Both pins carry
    inline comments saying so. *(Amended 2026-08-03: was "two-crate bump"; the
    toolchain and CLI legs were unstated. `gix` patch releases within `~0.85.0`
    are pre-authorised and need none of this ceremony.)*
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

**Both blocking questions were closed during implementation (2026-08-03).** Two
older ones are kept below for the record because their answers carry
consequences.

- **CLOSED 2026-08-03 — who owns the CI job for the strong-form zero-spawn run,
  and does the runner actually permit it?** **0188 owns it.** Delivered as
  `check-zero-spawn`: `ubuntu-latest`, passwordless `sudo`, no container, in
  `main.yml` (actionlint lints nothing else), modelled on `check-architecture`
  so the nightly-isolation invariant still sees exactly `{check-architecture}`,
  and wired into `prerelease.needs`. The shadow list is platform-aware and
  resolved at run time. Original text follows.

  ~~**OPEN — who owns the CI job for the strong-form zero-spawn run, and does the
  runner actually permit it?**~~ The criterion is deliberately non-degradable:
  the strong form (replacing or bind-mounting `/usr/bin/git` and friends) must
  run somewhere, and "somewhere" is currently a Linux CI job that may not exist
  and whose provider may forbid modifying `/usr/bin` without privileged
  containers. This is the one gate in the story resting on an unconfirmed
  infrastructure capability, and it has been raised in three consecutive
  reviews. **Confirm in planning.** *Default if it cannot be arranged*: carve
  the provisioning into its own small item that 0188 declares as `blocked_by`,
  rather than silently weakening the criterion to `PATH`-only everywhere —
  which would leave the property unproven on any platform.
- **CLOSED 2026-08-03 — who changes `vcs_adapters::facts`, 0169 or 0185, and in
  what order?** **0185**, atomically with the `CommandProbe` deletion: the
  composition root cannot switch until nothing else needs the subprocess pair.
  0169 wires its own classifier port without touching `facts`. Recorded on 0185.
  Original text follows.

  ~~**OPEN — who changes `vcs_adapters::facts`, 0169 or 0185, and in what
  order?**~~
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
- **`gix` is an *optional* dependency of `jj-lib` 0.43** (feature-gated), but
  **its gating feature is on under default features** — so the single-graph
  reasoning behind the pin holds as written. **Resolved 2026-08-03** by resolving
  the real graph: `jj-lib` 0.43.0 with default features pulls `gix` 0.85.0 and
  enables `attributes`, `blob-diff`, `index`, `max-performance-safe`, `sha1` and
  `zlib-rs` on it (plus a non-optional `gix-ignore ^0.21`). Two consequences:
  the single-version assertion is **non-vacuous** under our configuration, and
  the `attributes` feature that `Repository::submodules()` needs arrives from
  `jj-lib` itself rather than from `gix`'s own defaults — so narrowing `gix`'s
  features to shrink the binary would not remove it.
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

## Spike Outcome — gix 0.85 API surface (2026-08-03)

Three of the six queries had **no recorded API evidence**: the 2026-07-29
feasibility probe exercised exactly one gix entry point (`gix::discover`), and
the word "submodule" appears nowhere in it. A throwaway prototype against real
git fixtures (git 2.54.0, darwin-arm64, `gix` 0.85.0 with default features)
settled them, together with two riders. Full evidence tables, including the
verbatim per-fixture oracle comparison, are in
`meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md`.

- **Query 1 (bare) — available.** `Repository::is_bare()`
  (`gix-0.85.0/src/repository/worktree.rs:64`) agreed with
  `git rev-parse --is-bare-repository`. It reads `core.bare`, falling back to
  `workdir().is_none()`. See the reachability consequence in Library traps.
- **Query 2 (worktree + common dir) — first-class, and cheaper than assumed.**
  `Repository::kind()` returns a three-variant enum — `Common`, `Submodule`,
  `LinkedWorkTree` (`src/repository/mod.rs:6-16`) — so "is this a linked
  worktree" is a single enum read rather than a path comparison. `git_dir()`
  (`src/repository/location.rs:11`), `common_dir()` (`:33`) and `main_repo()`
  (`src/repository/worktree.rs`) matched `--git-dir`, `--git-common-dir` and the
  main worktree root exactly, from both the linked worktree and the main one.
  `worktrees()` enumerates linked worktrees by id.
- **Query 3 (superproject) — NO library API exists; it must be hand-rolled.**
  Verified by search across `gix` 0.85, `gix-discover` and `gix-submodule`, and
  behaviourally: `main_repo()` on a submodule returns the *submodule*, not the
  superproject. `Repository::submodules()`
  (`src/repository/submodule.rs:93`) resolves only the opposite direction
  (superproject → children) and requires the `attributes` feature. A ~15-line
  derivation from `Kind::Submodule` plus the `git_dir()` path shape
  (`<super-git-dir>/modules/<name>`, nesting as
  `.git/modules/mid/modules/leaf`) matched
  `--show-superproject-working-tree` at depth 1 **and** depth 2. **This is a
  sizing correction**: query 3 is bespoke path logic with its own edge cases,
  not a library call like the other five.
  - Old-form submodules (a nested `.git` *directory*) report `Kind::Common`, and
    git also reports no superproject for them — the two **agree**, so this is
    not an adapter/oracle divergence. The fixture matrix should still carry the
    shape, because `Kind::Submodule` alone would silently miss it.
- **Rider — the boundary rule's mechanism changed.** See Library traps above:
  the ceiling route is structurally incapable of enforcing the rule, and the
  requirement has been rewritten to marker-walk-then-`gix::open`.
- **Rider — the scrub invariant holds for free on the git side.** With
  `GIT_DIR`, `GIT_COMMON_DIR` and `GIT_CONFIG_GLOBAL` (asserting
  `core.bare = true`) all poisoned at another fixture's real `.git`, every gix
  query — `kind()`, `is_bare()`, `git_dir()`, `common_dir()`, `workdir()`,
  `main_repo()` — returned **identical** values, while
  `git rev-parse --git-dir` diverged, proving the poisoning was live. Plain
  `gix::discover`/`gix::open` do not consult the environment;
  only `discover_with_environment_overrides` does, and it diverged as expected.
  Consequences: no explicit scrub is needed for the git-side queries — the
  invariant is a property to *verify*, not to implement — and
  `discover_with_environment_overrides` is a ready-made non-vacuity control for
  the "an unscrubbed control must diverge" clause. **The jj-lib side remains
  untested.**

## Validation Results

**Implemented 2026-08-03.** Host for every figure: **darwin-arm64**, jj 0.43.0,
git 2.54.0, Rust 1.90.0 — chosen to match 0186's `B = 35.1 ms`, since the ~97 ms
probe delta is macOS-specific and a cross-host comparison would leave 0169's
gate ill-posed.

- **Platform the strong-form zero-spawn run held on** — the `check-zero-spawn`
  Linux CI job is written, wired into `prerelease.needs`, and lands with this
  branch; its first execution is on that run. Locally and on macOS the suite
  degrades to `PATH`-only under SIP, and **records** what it could not shadow:
  `/usr/bin/git`, `/opt/homebrew/bin/git`, `/opt/homebrew/bin/jj`, and jj's mise
  install path plus its shim. The harness half of the contract **is** verified:
  it fails closed on a malformed mode and hard-fails when `strong` is claimed
  while a listed path is still executable.
- **`gix` / `gix-*` versions resolved in `Cargo.lock`** — `gix 0.85.0`, 47
  gix-family packages, **no duplicate versions**. Asserted by
  `tests/integration/deny/test_vcs_library_graph.py`.
- **`jj-lib` version resolved in `Cargo.lock`** — `0.43.0`.
- **MSRV of `gix` 0.85 and `jj-lib` 0.43 vs the pinned Rust toolchain** —
  _resolved 2026-08-02_: pinned 1.90.0; jj-lib 1.89, gix 1.85. Both fit. The
  graph test additionally asserts **no** package in the closure declares a
  `rust-version` above the pin (354 packages declare one), catching the
  `kstring` class of trap directly rather than trusting resolver 3's
  *preference*.
- **Installed `jj` CLI version the fixtures were built with** — 0.43.0, matching
  the crate pin. `mise install` done. A committed test now asserts the **binary**
  against the **resolved library** at major.minor, not merely the two
  declarations against each other.
- **Installed `git` CLI version** — 2.54.0. The harness carries a 2.45 floor
  with a named diagnostic (`--ref-format=reftable` landed there).
- **jj `revision` — delivered here, not descoped** (amendment 10, reversing 8).
  Route: the workspace's checkout state
  (`<workspace_root>/.jj/working_copy/checkout`, decoded as the **public**
  `jj_lib::protos::local_working_copy::Checkout`) gives the operation id and the
  workspace's own name; `SimpleOpStore::load(<repo>/op_store, ..)` reads that
  operation's view; `View.wc_commit_ids` is indexed by name. No settings value is
  constructed, so `lint:vcs-settings:check` still passes crate-wide.

  Verified against the live CLI, exact match on every shape tried: this repo,
  pure jj (`--no-colocate`, so there is no git HEAD to have fallen back to),
  colocated, commitless, a secondary workspace, and that workspace's main.
  **The multi-workspace case is the load-bearing one** — the two report
  *different* commits, so indexing by name is doing real work; taking the view's
  sole entry would pass every single-workspace test and answer for the wrong
  workspace. Writes nothing: fingerprint-asserted over the whole `.jj` tree,
  which matters because a sibling loader in the same area does create
  directories on load. Broken checkout state → warn-logged absence, no panic.

  **`detection.rs` now asserts full `RepoFacts` equality for both idioms** — the
  per-`VcsKind` narrowing is gone, and all 7 cases pass.

  **One divergence, recorded deliberately**: asking the `jj` binary snapshots the
  working copy first, so with unsnapshotted edits present it reports *and writes*
  a new commit, while this route reports the commit as of the last recorded
  operation and writes nothing. Measured both directions. After 0185's switch,
  metadata derivation therefore stops mutating the user's repository. Pinned by
  `an_unsnapshotted_edit_is_the_one_documented_divergence`.
- **gix API coverage for the six queries** — all six delivered. Query 3
  (superproject) is the hand-rolled derivation, as the spike predicted, and
  **`gix::open` accepts a linked-worktree gitdir** — the one open question the
  spike left, now settled by the `SM-wt` fixture.
- **`gix` boundary containment mechanism** — `gix::open`/marker-walk, as
  designed. The paired negative assertion is committed: an unbounded
  `gix::discover` on the same fixture escapes to the parent repository.
- **`GIT_DIR` scrub invariant** — _verified 2026-08-03 across all 34 (fixture,
  start directory) pairs_, over a poisoning matrix widened beyond the original
  two variables to everything `scrub_environment` touches **plus**
  `GIT_OBJECT_DIRECTORY`, `GIT_ALTERNATE_OBJECT_DIRECTORIES` and the
  `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` triple, with a populated global config
  asserting a divergent `core.bare`. Every value identical. Non-vacuity control
  confirmed in the same run: `gix::discover_with_environment_overrides` resolves
  the poison target, asserted **inside the poisoned child**.
- **Adapter/oracle divergences recorded** — three, each deliberate:
  1. **sha256 is unsupported by gix 0.85.** Every gix-backed query returns `Err`
     on the `S256` fixture rather than misreading it. Correct under the partition
     rule — a repository *is* here and the pinned library cannot answer — and the
     reason the queries carry an error channel at all. Reftable reads normally.
     **0185 inherits the consequence**, since its switch exposes users to it.
  2. **`D1` returns `Err` where both CLIs report absence.** A `.jj/repo` pointing
     at a deleted directory is a *broken* workspace, not an absent one;
     reporting absence is exactly the failure the error channel exists to
     prevent.
  3. The pre-existing `GIT_DIR` scrub asymmetry in `classify_checkout`, which the
     adapter deliberately does not reproduce.
- **Non-vacuity demonstrations** — all three done, and all three **committed**
  rather than shown by hand: a `std::process::Command` import fails cargo-pup
  with *"Use of module 'std::process::Command' is denied"* naming the rule
  (`tests/integration/pup/test_import_rule.py`, plus a compliant positive
  control and a grouped-import case); a deliberate `UserSettings::from_config`
  fails `lint:vcs-settings:check`; the unscrubbed control diverges under the
  poisoned run.
- **Reference artefact size** — delivered two-binary shape, `--release`
  (stripped by the profile), all four release triples:

  | Triple | linked | stubbed | delta | ratio |
  | --- | --- | --- | --- | --- |
  | `aarch64-apple-darwin` | 2,512,576 | 350,512 | 2,162,064 | **7.17x** |
  | `x86_64-apple-darwin` | 2,560,820 | 347,504 | 2,213,316 | **7.37x** |
  | `aarch64-unknown-linux-musl` | 2,476,312 | 360,864 | 2,115,448 | **6.86x** |
  | `x86_64-unknown-linux-musl` | 2,699,320 | 386,504 | 2,312,816 | **6.98x** |

  All four cross-compile and pass magic-byte checks; both musl builds pass
  `_assert_static_elf`. Every triple clears the 3x ratio floor and both musl
  builds clear the 1,500,000-byte absolute floor. The figures **exceed** the
  prototype's (5.42x darwin / 6.19x musl), so the two-binary shape linked at
  least as much as the feature-gated one.

  **Re-measured after the jj `revision` route landed** (amendment 10), which is
  why these exceed the first pass (6.22x-6.71x): the op-store read and the
  protobuf decode link more of jj-lib. All four triples were re-cross-compiled to
  confirm the new direct `prost`/`pollster` edges break no target — they could
  only have, and did not. Host-native ratio via `build:cli:fixture-size`: 7.16x.

- **Cost, against the pure-jj fixture** (median of 20 with 3 warm-up runs):

  | Measurement | Median | Min | Max |
  | --- | --- | --- | --- |
  | cold per-process, all six queries + both ports, pure-jj | **4.03 ms** | 3.64 | 4.67 |
  | cold per-process, all six queries + both ports, plain git | 4.74 ms | 4.29 | 5.37 |
  | cold per-process, single query (`jj_workspace_root`), pure-jj | **3.58 ms** | 3.33 | 4.75 |
  | cold per-process, single query (`is_bare`), plain git | 3.96 ms | 3.48 | 4.41 |
  | `CommandProbe` jj subprocess (`jj log -r @ -T commit_id`) | **23.84 ms** | 21.52 | 24.75 |
  | `CommandProbe` git subprocess (`git rev-parse HEAD`) | 5.34 ms | 4.60 | 6.06 |

  Warm in-process, same host, median of 20: first-call jj-lib **30.6 us**,
  first-call gix **102.5 us**; per query `jj_workspace_root` 13.7,
  `jj_repository` 29.7, `superproject` 35.3, `worktree` 37.8, `is_bare` 38.6,
  `dual_roots` 50.1 us.

  **Re-measured after the jj `revision` route landed** (amendment 10), on this
  repo's colocated workspace: all six queries + both ports **4.81 ms** (min 4.44,
  max 6.18), single query 2.80 ms, against `jj log -r @ -T commit_id` at
  26.54 ms. So the full surface — now including a real jj revision — still costs
  roughly **a fifth** of one `jj` subprocess.

  Read for 0169's gate: the library-backed cold per-process path costs roughly
  **one sixth** of a single `jj` subprocess. The single-query mode inflation is
  ~12% (3.58 vs 4.03 ms) on pure-jj, confirming amendment 4 — process startup
  dominates and the mode is retained for 0169's convenience, not necessity.

  **Not measured**: an `x86_64-unknown-linux-musl` cold per-process figure. The
  measurement host is darwin-arm64 and cannot execute a musl binary; the
  criterion asked for it so 0169 does not set a threshold with no Linux
  datapoint. **Carried forward** — the `check-zero-spawn` job is the natural
  place to take it.

- **Cold-cache compile cost** (measured 2026-08-03, darwin-arm64, 16 logical /
  12 performance cores, fresh `CARGO_TARGET_DIR`, no `sccache`). This branch
  changes `cli/Cargo.lock`, which invalidates the `Swatinem/rust-cache` key on
  every Rust job, so the first CI run here compiles cold by construction.

  | Cold build | Wall | CPU | Crates |
  | --- | --- | --- | --- |
  | `build:server:dev` (whole closure) | **21.20 s** | 102.97 s | 297 |
  | `-p vcs-adapters` (the two new trees alone) | 16.92 s | 65.80 s | 198 |

  So the two trees are ~64% of the server's cold CPU cost but only ~17 s of wall
  clock here, and 46 gix-family crates. Scaled to a 4-vCPU `ubuntu-latest`
  runner the added cost is on the order of a minute or two — comfortably inside
  `test-visual-regression`'s `timeout-minutes: 20`, whose wall clock is dominated
  by Playwright and Docker rather than by this build. **No budget raise needed**;
  `check-zero-spawn` carries the same 20-minute cap and the same cold first run.

- **Shipped-binary size impact: none, because the trees do not link.** Verified
  2026-08-03 on an unstripped `--release` `accelerator-visualiser`
  (`CARGO_PROFILE_RELEASE_STRIP=false`, darwin-arm64, 10.7 MB): dead-code
  elimination removes the whole `gix`/`jj-lib` closure. Distinctive literals
  (`extensions.objectFormat` from gix, `There is no Jujutsu repo` from jj-lib)
  are present in the linked reference artefact and absent from the shipped
  binary, which carries **zero** symbols from `gix`, `gix-pack`, `gix-odb`,
  `jj_lib`, `clru` or `uluru` against 26,247 total. The cause is broader than
  this story: nothing in the visualiser's reachable call graph enters
  `vcs-adapters` at all — `CommandProbe`'s own `rev-parse` literal is absent
  too. A formal before/after byte comparison against the pre-branch revision was
  therefore not taken; the delta is nil by construction. **This also closes the
  MPL-2.0 §3.2 question** (see below) and **must be re-checked when the
  visualiser starts reaching `vcs_adapters::facts`** — 0185's composition-root
  switch is the expected trigger, and it is recorded there.

- **MPL-2.0 §3.2 — closed 2026-08-03: the notice obligation does not bind.**
  `uluru` is in the normal closure of exactly one shipped binary
  (`accelerator-visualiser`; neither `accelerator` nor `accelerator-verify` has
  it at all), and the dead-code-elimination evidence above shows no `uluru` code
  is distributed in it. §3.1 is satisfied trivially — we ship no modifications —
  so no third-party attribution artefact is required in the release payload, and
  `_release_uploads()` is unchanged. The `cli/deny.toml` exception comment
  records the finding, its evidence and its re-check trigger; an earlier draft of
  that comment asserted the obligation was discharged by a staged licence file
  that does not exist, which is corrected.

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
