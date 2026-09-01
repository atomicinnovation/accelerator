---
type: "codebase-research"
id: "2026-08-10-0185-converge-corpus-adapters-library-backed-vcs"
title: "Research: Converge corpus-adapters on the Library-Backed VCS Adapter (0185)"
date: "2026-08-10T14:55:36+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0185"
parent: "work-item:0185"
relates_to: ["codebase-research:2026-08-02-0188-library-backed-vcs-adapter", "codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration"]
topic: "Converge corpus-adapters on the Library-Backed VCS Adapter (0185)"
tags: ["research", "codebase", "rust", "vcs", "corpus-adapters", "vcs-adapters", "gix", "jj-lib"]
revision: "63fe1c0a8b6673b026f71c7506943ca50cbec0e9"
repository: "accelerator"
last_updated: "2026-08-10T14:55:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Converge corpus-adapters on the Library-Backed VCS Adapter (0185)

**Date**: 2026-08-10T14:55:36+00:00
**Author**: Toby Clemson
**Git Commit**: 63fe1c0a8b6673b026f71c7506943ca50cbec0e9
**Branch**: (jj workspace, no active bookmark)
**Repository**: accelerator

## Research Question

What does the live codebase look like for work item
[0185](../../work/0185-converge-corpus-adapters-on-library-backed-vcs.md) —
"Converge corpus-adapters on the Library-Backed VCS Adapter" — so a plan can
be written for it? Specifically: what does `vcs_adapters::facts` wire up
today, what exactly is `CommandProbe` and what depends on it, how does
`cli/corpus-adapters` consume `RepoFacts`, what is the shared zero-spawn test
harness and how would it extend to the metadata-read path, what do the
licence/lint gates (`deny.toml`, `pup.ron`) say today, where would the
sha256-repository policy decision be recorded, and does the visualiser server
or a hook path already reach the code this item repoints?

## Summary

0185 is a well-scoped wiring-plus-deletion task with a **completed,
APPROVE-verdict work-item review already on file**
(`meta/reviews/work/0185-converge-corpus-adapters-on-library-backed-vcs-review-1.md`,
Pass 2, 2026-08-10) — the item text itself is internally consistent and ready
for implementation. Its blocker, 0188, is done and delivered a fully
functional `InProcessProbe` with no gaps: `revision` and `kind` are both
completely implemented, not partial. The mechanical core of the task —
repoint `vcs_adapters::facts`, delete `CommandProbe`, collapse the transitional
dual-adapter test comparison — is small and precisely bounded.

Three things the live codebase shows that the work item's own text does not
(all worth carrying into the plan):

1. **The `corpus-adapters` call site has moved behind a new abstraction since
   the work item's Technical Notes were last checked.** It is no longer an
   inline call inside `derive_at`; it is a one-line body inside
   `VcsBackedRepoFactsProbe::facts` (`cli/corpus-adapters/src/metadata.rs:214`),
   a narrower and cleaner blast radius than the work item describes. The line
   numbers the work item cites (`:201`, `:185-186`) no longer apply.
2. **`cli/corpus-adapters/tests/work_item_pattern_parity.rs`, named in AC2,
   does not exist** — the closest thing is a unit-test module inside
   `work_item_pattern.rs` and an unrelated regex test in `parity.rs`, neither
   of which is VCS-related.
3. **The existing zero-spawn test (`corpus-adapters/tests/zero_spawn.rs`)
   does not yet cover the metadata-read path at all.** It proves
   `InProcessProbe`'s individual queries don't spawn (via a reference binary
   in `vcs-adapters`), but that reference binary never calls
   `vcs_adapters::facts` or `VcsBackedRepoFactsProbe`. AC3's "extend the
   zero-spawn assertion to cover a `corpus-adapters` metadata read" is new
   test-authoring work, not an extension of an existing assertion, and the
   `Stubs` harness is `Command`-shaped (it patches a child process's `PATH`),
   so the extension most naturally needs a new small reference-binary entry
   point rather than an in-process call from the test itself.

Everything else lines up cleanly with the work item's own text: `markers.rs`
is genuinely shared and must not be touched; `scrub_environment`/`run_capped`
(the "capped_stdout" helpers) are genuinely shared with 0198's
`run_vcs_text` and must survive; no discovered write path depends on the
jj snapshot-on-read side effect; the `uluru` MPL-2.0 exception, the `gix`/
`jj-lib` pins, and the `vcs_adapters_library_reads_in_process` pup rule are
all exactly as described; `check-zero-spawn` is confirmed to be a standalone
CI job, not part of default `mise run`/`mise run check`; and the visualiser
server does not yet reach `vcs_adapters::facts` at all today (dead-code
elimination strips the whole `gix`/`jj-lib` closure from the shipped
binary), though `InProcessProbe` already runs unbounded, synchronously, on
the hook path for `detect`/`guard` — a pre-existing, uncontained exposure
this item's containment AC should be read against.

## Detailed Findings

### `cli/vcs-adapters` — composition root, `CommandProbe`, `InProcessProbe`

**Composition root** — `cli/vcs-adapters/src/lib.rs:22-26`:
```rust
use crate::subprocess::{CommandProbe, MarkerWalkRoot};

#[must_use]
pub fn facts(start: &Path) -> Option<RepoFacts> {
    vcs::facts(start, &MarkerWalkRoot, &CommandProbe::new())
}
```
This is the entire repoint: swap the import and the two arguments for
`library::InProcessProbe`, which already implements both `RepoRoot` and
`VcsProbe` and can serve both positions as a single value (unit struct,
`Default`). `lib.rs:12-14` declares `pub mod library; mod markers; pub mod
subprocess;` — `markers` is crate-private.

**`CommandProbe`** — `cli/vcs-adapters/src/subprocess.rs:65-139`. Struct +
`cap: Duration`, `impl VcsProbe` (`kind` delegates to
`crate::markers::marker_kind`, shared with `InProcessProbe::kind`; `revision`
spawns `jj log -r @ -T commit_id` / `git rev-parse HEAD` via
`scrub_environment` + `run_capped`) and `impl OriginRemote` (spawns `git
remote get-url origin` via `scrub_environment` + `run_checked`). This whole
span is what 0185 deletes.

**Safe-to-delete alongside it**: `wait_capped_checked`/`run_checked`
(`subprocess.rs:329-407`) — used only by `CommandProbe`'s `OriginRemote` impl,
nothing else calls them; and three `CommandProbe`-flavoured unit tests
(`subprocess.rs:583-632`, `a_configured_origin_is_reported`,
`no_origin_remote_is_reported_as_none`, `a_directory_with_no_repository_is_an_error`)
— redundant, since `library.rs:1009-1107` already carries equivalent
`InProcessProbe`-based versions of all three (plus a fourth,
`an_unreadable_repository_is_an_error`, that `subprocess.rs` has no
counterpart for).

**Must NOT be deleted — shared with 0198's `status`/`log` path** (verifies
the work item's own caution): `scrub_environment` (`subprocess.rs:231-246`)
is called by `CommandProbe::revision` (`:119`), `CommandProbe`'s
`OriginRemote::origin_url` (`:136`), **and** `run_vcs_text` (`:215`, the
function backing `status`/`log`, owned by 0198). `run_capped`/`wait_capped`
(`:248-324`, the actual name of the "capped_stdout" helper — there is no
literal function called `capped_stdout`) are likewise called by both
`CommandProbe::revision` (`:122`) and `run_vcs_text`'s final line (`:226`).
Eight unit tests inside `subprocess.rs` (`:442-568`) exercise these two
helpers directly with generic `sh`/`sleep`/`true`/`false`/`printf` stand-ins,
never touching `CommandProbe` — these belong to the 0198-owned surface and
must be kept as-is.

**`MarkerWalkRoot`** (`subprocess.rs:31-43`) is not on the delete list — it
delegates to `crate::markers` and is still referenced by
`tests/library.rs`'s own parity assertions today; nothing in 0185's scope
requires removing it, only the `facts()` composition that currently wires it
in.

**`InProcessProbe`** (`cli/vcs-adapters/src/library.rs:190` onward, 1108
lines total) — implements `RepoRoot` (`:446-455`), `VcsProbe` (`:457-478`),
`UserIdentityProbe` (`:480-497`), `OriginRemote` (`:499-503`),
`vcs::classify::CheckoutProbe` (`:409-431`) and `vcs::mode::ModeProbe`
(`:433-444`), plus the six inherent taxonomy queries (`is_bare` `:201-204`,
`worktree` `:212-238`, `superproject` `:248-269`, `jj_workspace_root`
`:276-288`, `jj_repository` `:297-330`, `dual_roots` `:334-339`). **Both
`revision` and `kind` are fully implemented with no `todo!`/`unimplemented!`
anywhere in the file.** `revision`'s jj half (`:594-628`) reads the
checkout-state protobuf and op-store directly — no `jj` binary, no snapshot
side effect; the git half (`:533-554`) uses `gix::discover`.

**`markers.rs`** (51 lines) — its own doc comment states its purpose
directly: *"The ancestor walk and marker reading both adapters share. Each
delegates to these rather than to the other, so retiring either strands
nothing."* Both `subprocess.rs` and `library.rs` import from it. Must not be
deleted.

**`tests/detection.rs`** (352 lines, gated `#![cfg(feature = "bash-parity")]`)
— the "dual comparison" is two helper functions plus a shared assertion, not
a table or macro:
```rust
fn facts(start: &Path) -> Option<RepoFacts> {
    facts_via(start, &MarkerWalkRoot, &CommandProbe::new())
}
fn library_facts(start: &Path) -> Option<RepoFacts> {
    facts_via(start, &InProcessProbe, &InProcessProbe)
}
```
plus `assert_implementations_agree(start)` (`:138-154`), called at the end of
each of the seven shape-specific tests. Collapsing it means deleting the
`CommandProbe`/`MarkerWalkRoot` imports and the `facts`/`facts_via`/
`assert_implementations_agree` scaffolding, and having every test call
`vcs::facts(&x, &InProcessProbe, &InProcessProbe)` (or a single renamed
`facts` helper) directly.

**A discrepancy worth flagging to the plan**: the work item's Requirements
describe a ".git-as-file worktree case" that "keeps today's
(`CommandProbe`-oracle) value" as a known, pinned divergence. No such
divergence exists in `detection.rs` today —
`a_worktree_whose_git_marker_is_a_file_is_recognised` (`:284-322`) calls the
same `assert_implementations_agree` as every other test and the two probes
agree on it. **The only documented divergence anywhere in the crate is the
jj-snapshot-on-read case**, pinned in
`cli/vcs-adapters/tests/library.rs:431-474`
(`an_unsnapshotted_edit_is_the_one_documented_divergence`), and
`detection.rs`'s own doc comment on `assert_implementations_agree`
(`:126-132`) explicitly defers to it: *"The one shape where the two can
legitimately differ is out of reach here... `library.rs` pins the divergence
itself."* Treat the work item's worktree-divergence framing as stale; the
real, single divergence is the snapshot one.

**`Cargo.toml`** — `bash-parity` (`:12-16`) is an empty feature flag gating,
via `#![cfg(feature = "bash-parity")]`, every real-binary integration test
across `tests/detection.rs`, `tests/library.rs`, `tests/scrub.rs`,
`tests/classify.rs`, `tests/queries.rs`. It means "needs real `jj`/`git` to
build fixtures," not "shells out in production," and stays relevant after
0185 (fixtures are still built with real binaries).

### `cli/corpus-adapters` — the consumer

**Current call site** — `cli/corpus-adapters/src/metadata.rs:209-220`:
```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct VcsBackedRepoFactsProbe;

impl RepoFactsProbe for VcsBackedRepoFactsProbe {
    fn facts(&self, start: &Path) -> Option<RepositoryFacts> {
        let facts = vcs_adapters::facts(start)?;
        Some(RepositoryFacts {
            name: facts.name,
            revision: facts.revision,
        })
    }
}
```
`derive_at` (`:228-236`) takes `facts_probe: &dyn RepoFactsProbe` by
injection and calls `facts_probe.facts(start)` — the `vcs_adapters` name
never appears there. This is narrower than the work item's described blast
radius (an inline call "inside `derive_at`" at `:201`) — an intermediate
`RepoFactsProbe` seam has been introduced since the work item's Technical
Notes were written, and since `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md`
was researched. **0185's actual repoint target is the single line
`vcs_adapters::facts(start)` at `metadata.rs:214`** — nothing else in
`corpus-adapters` needs to change signature. `derive` (`:196-207`) reads
`.name`/`.revision` a second time from the already-translated
`RepositoryFacts`, at `:204-205`, formatting them via `render()`
(`:241-260`) into `"Repository Name: {name}"`/`"Current Revision: {revision}"`
lines. This module only computes/formats a string block; it writes no file
itself.

**`tests/zero_spawn.rs`** (105 lines, `#![cfg(feature = "bash-parity")]`) —
spawns the compiled `vcs-adapters-fixture` binary (declared in
`cli/vcs-adapters/Cargo.toml:23-25`) as a subprocess, running every fixture
through it twice (unrestricted vs. `Stubs`-applied `PATH`), asserting
identical stdout and no marker written. **This never exercises
`vcs_adapters::facts` or `corpus_adapters::metadata` at all** — the fixture
binary's `main()` calls `vcs_adapters::library::InProcessProbe` directly,
never the composition in `lib.rs`. Extending coverage to the metadata read
(AC3) needs either (a) a new query added to the `vcs-adapters-fixture`
binary that calls `vcs_adapters::facts` itself (that binary already lives in
the `vcs-adapters` crate, so no new dependency edge), or (b) a
`corpus-adapters`-local reference binary calling `VcsBackedRepoFactsProbe`/
`derive_at` directly. Either way, `Stubs::apply` only patches a
`std::process::Command`'s `PATH`, so the harness inherently needs the code
under test to run as a spawned child process, not called in-process from the
test.

**`tests/parity.rs`, `tests/metadata.rs`** — both read in full. Neither
touches `CommandProbe`, subprocess timing, or the jj snapshot-on-read side
effect. `parity.rs` is entirely doc-type-inference/regex-compilation
testing, no VCS import at all. `metadata.rs` (no `bash-parity` gate — always
runs) builds `RepositoryFacts` by hand via a local `facts()` test helper and
never constructs a real `VcsBackedRepoFactsProbe`; its only host-touching
tests call `SystemClock::try_new()`, which spawns `date`, never `jj`/`git`.
Both suites should genuinely "keep passing unchanged" through the
`CommandProbe → InProcessProbe` swap, as AC2 requires.

**`tests/work_item_pattern_parity.rs`, named in AC2, does not exist.** The
crate's actual test files are `zero_spawn.rs`, `metadata.rs`, `parity.rs`,
`doc_type_single_source.rs`, `store.rs`. The logic that name suggests lives
as unit tests inside `src/work_item_pattern.rs`'s own `#[cfg(test)] mod
tests` — pure-Rust, no VCS, no bash oracle. Flag this as a stale AC reference
to correct or drop when the plan revisits acceptance criteria.

**Separately**: a bash-oracle metadata parity test referenced in the 0188
research (`derive_at_agrees_with_the_live_metadata_helper`, comparing
against `scripts/artifact-derive-metadata.sh`) no longer exists in the tree
— both the test and the script are gone, presumably retired when
`corpus-cli` subsumed the bash metadata helpers.

**No write path depends on the snapshot-on-read side effect.** Two
production callers of `derive_at`/`VcsBackedRepoFactsProbe` exist in the
whole `cli/` workspace:
- `cli/corpus-cli/src/main.rs:72-86` (`corpus metadata derive`) — prints the
  rendered block, including revision, to **stdout only**; no file write.
- `cli/work-cli/src/create.rs:258-301` (`work create`, the one Rust write
  path that persists frontmatter via `AtomicWrite::write` at `:301`) — reads
  only `metadata.datetime_utc` (`:285`, from the injected `Clock`).
  `metadata.revision`/`.repository_name` are computed but never referenced
  in the written frontmatter.

This satisfies the AC requiring confirmation that no corpus write path
depends on the snapshot side effect — with the caveat that this research did
not trace into `scripts/`/skill layer beyond the Rust boundary, in case a
shell skill separately consumes the printed revision from `corpus metadata
derive`'s stdout.

**`Cargo.toml`** — `corpus-adapters` already depends on `vcs-adapters`
(`:25`, normal dependency — no manifest change needed for the repoint) and
on `vcs-test-support` (`:37`, dev-only, with an explanatory comment about why
it must stay dev-only: `cli/visualiser/server` depends on `corpus-adapters`,
so fixture-building machinery must not leak into production). It does not
depend on the `vcs` crate directly — only on `vcs_adapters::facts`'s
`Option<vcs::RepoFacts>` return, immediately translated into corpus's own
`RepositoryFacts`.

A separate consumer outside this sweep: `cli/migrate-adapters/src/context.rs:89`
calls `vcs::facts` directly (not `vcs_adapters::facts`) with its own injected
probe — an independent subsystem (the migration engine), out of scope for
0185.

### `cli/vcs-test-support` — the shared zero-spawn harness

Located at `cli/vcs-test-support/` (workspace member). Public API across
four modules (`src/lib.rs:1-11`):

- **`stubs.rs`** — `Stubs::rooted_at(base)` writes marker-writing shell-script
  stubs for `git`/`jj` and computes a synthetic `PATH` (stub dir first, then
  every PATH entry that doesn't itself resolve a real `git`/`jj`).
  `Stubs::apply(&self, command: &mut Command)` sets `PATH` on a `Command`;
  `Stubs::spawns()` reports whether a stub ran. `Mode` (`PathOnly`/`Strong`)
  is read from `ACCELERATOR_ZERO_SPAWN_MODE`; `assert_shadowing_holds(mode)`
  re-verifies the `Strong`-mode shadow list at runtime.
  `unshadowed_paths()` returns the absolute-path shadow list — every `PATH`
  entry with a real `git`/`jj`, plus a hardcoded superset
  (`/usr/bin/git`, `/usr/local/bin/git`, `/opt/homebrew/bin/git` and the `jj`
  equivalents) checked unconditionally via `is_executable()`. "Platform-aware"
  means the *same* fixed list is checked everywhere, with platform
  differences arising from which paths actually resolve on a given host, not
  from separate compiled lists. All privileged shadowing (`sudo mv`) lives in
  `tasks/test/integration.py`, not in this crate — the crate only reports
  paths, never mutates outside its own temp directories.
- **`hermetic.rs`** — `Hermetic::rooted_at(base)` builds an empty `HOME`/
  `XDG_CONFIG_HOME` plus a fixed-identity `jj.toml`; `apply(&self, command)`
  sets `HOME`, `XDG_CONFIG_HOME`, `JJ_CONFIG`, `GIT_CEILING_DIRECTORIES`,
  `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, and strips 8
  ambient `GIT_*` variables. Also exposes `assert_no_repository_ancestor`,
  `assert_jj_matches`, `assert_git_is_recent_enough`.
- **`fixtures.rs`** — `Matrix`/`Fixture`/`matrix_root()`/`pure_jj()`, the full
  checkout-shape fixture matrix builder.
- **`masks.rs`** — golden-output redaction, shared with a Python golden
  generator via `hooks/test-fixtures/masks.toml`.

**Flagship consumer**: `cli/corpus-adapters/tests/zero_spawn.rs` is the one
proving the harness works across a crate boundary — it drives `fixtures`,
`stubs::{Stubs, Mode, assert_shadowing_holds, reference_artefact,
unshadowed_paths}` end to end. Nine other files across `vcs-cli`,
`vcs-adapters`, `corpus-cli`, and `migrate-cli` also depend on it, mostly for
`Hermetic`.

**`check-zero-spawn` CI job** (`.github/workflows/main.yml:323-377`) — its
own `ubuntu-latest` job (macOS can't shadow system binaries under SIP), wired
into `prerelease.needs` (`:436`). Underlying task:
`test:integration:zero-spawn:strong` → `tasks/test/integration.py:181-241`,
refusing to run unless `ACCELERATOR_ZERO_SPAWN_SHADOW=yes`, `sudo mv`-ing
real binaries aside, running the nextest suite, restoring in a `finally`.
**Confirmed: neither the `PathOnly` nor `Strong` form is reachable from
`mise run`, `mise run check`, or any roll-up task** — `test:integration`,
`test`, `check`, and `default` in `mise.toml` all deliberately exclude it
(confirmed by grep and by the tasks' own doc comments: "its own CI job, not
the test roll-up"). This directly answers AC3's open question: the strong
form runs only via `check-zero-spawn` in CI, gated separately from the
default `mise run`/`check` invocation the item's frontmatter otherwise
requires green.

### Licence and lint gates

**`cli/deny.toml:67-81`** — the `uluru` exception, quoted in full:
```toml
# uluru is gix-pack's LRU pack cache, enabled by gix-odb's default features, so
# it cannot be feature-gated out. MPL-2.0 is file-level weak copyleft: we ship
# no modifications, and §3.2's notice obligation does not bind because dead-code
# elimination removes the whole gix/jj-lib closure from the only shipped binary
# that reaches it. Re-check that when anything makes the visualiser reach
# vcs-adapters: once the trees link, distributing the binary requires telling
# recipients how to obtain the source, which the release upload set carries no
# artefact for.
#
# Verify with: an unstripped --release accelerator-visualiser carries neither
# the `extensions.objectFormat` (gix) nor the `There is no Jujutsu repo`
# (jj-lib) literal, both of which the linked reference artefact does carry.
[[licenses.exceptions]]
crate = "uluru"
allow = ["MPL-2.0"]
```
This re-check is a **manual procedure, not an automated test** — confirmed
by a repo-wide search for the two literal strings (`extensions.objectFormat`,
`There is no Jujutsu repo`), which appear only in prose/comments, never as a
grep target inside a `.py`/`.rs` test. `tests/integration/deny/test_vcs_library_graph.py`
(384 lines, 12 tests) covers version pinning, feature presence/absence,
MSRV, and build-script/proc-macro checks — none build or grep an unstripped
visualiser binary. The plan should treat this as a step to perform (build
`--release` unstripped, count `gix`/`jj-lib`/`clru`/`uluru` symbols against a
baseline, grep both literals), matching the procedure already carried out
for 0188 and recorded in `meta/validations/2026-08-03-0188-library-backed-vcs-adapter-validation.md:328-341`.

**`cli/Cargo.toml:72-73`** — `jj-lib = "=0.43.0"`; `gix = { version =
"~0.85.0", default-features = false }`, both with inline coupling comments,
exactly as the work item states.

**`cli/pup.ron:140-163`** — the `vcs_adapters_library_reads_in_process` rule,
quoted in full:
```
Module((
    name: "vcs_adapters_library_reads_in_process",
    matches: Module("^vcs_adapters::library($|::)"),
    rules: [
        RestrictImports(
            allowed_only: Some([
                "^(std|core|alloc)(::|$)", "^dunce(::|$)", "^etcetera(::|$)",
                "^gix(::|$)", "^jj_lib(::|$)", "^pollster(::|$)",
                "^prost(::|$)", "^tracing(::|$)", "^vcs(::|$)", "^crate(::|$)",
            ]),
            denied: Some(["^std::process(::|$)"]),
            severity: Error,
        ),
    ],
)),
```
Slightly broader than the work item's shorthand ("permit `std`, `gix`,
`jj_lib`, `kernel`, `vcs`, `crate::`") — it also permits `dunce`, `etcetera`,
`pollster`, `prost`, `tracing` (jj-lib's own helper crates) and does not
mention `kernel` at all. No equivalent rule exists for `corpus-adapters`.

**`tasks/github.py:231-248`** (not `tasks/build.py` — the work item's file
guess is off by one module) — `_release_uploads()` enumerates debug
archives, the launcher binary + signature, the release manifest + signature,
and per-sub-binary assets. **No licence-file/attribution-artefact machinery
exists anywhere in `tasks/` today** — confirmed by search, and consistent
with 0188's own validation record noting an earlier draft comment falsely
claimed a staged licence file existed. Coverage is enforced by
`tests/unit/tasks/test_workflows.py:207-221`
(`test_attest_globs_cover_every_published_asset`), which asserts every path
`_release_uploads()` returns is matched by a CI attest-step glob — so adding
an attribution artefact (if AC5 requires one) needs a generation step, a
path helper, an `uploads.append(...)` inside `_release_uploads()`, and a
matching CI glob, or that test fails.

### `cli/vcs` — port contracts, `RepoFacts`, and where the sha256 decision belongs

`cli/vcs/src/lib.rs:19-73` — `VcsKind` (`Jj`/`Git`/`None`), `RepoFacts { root,
name, kind, revision }`, `RepoRoot` (infallible, `Option`-returning), and
`VcsProbe`:
```rust
/// Reports a repository's idiom and its working-copy revision.
pub trait VcsProbe {
    fn kind(&self, root: &Path) -> VcsKind;

    /// The full working-copy revision, or `None` when the repository has none
    /// and when the probe cannot answer. A caller cannot distinguish the two;
    /// an adapter is expected to log the failure.
    fn revision(&self, root: &Path, kind: VcsKind) -> Option<String>;
}
```

**No README or markdown file exists under `cli/vcs/` or `cli/vcs-adapters/`**
— the crate's "port-contract documentation" is entirely doc comments on the
traits themselves, and there is clear existing precedent for recording
exactly this kind of policy decision there:
`OriginRemote::origin_url`'s doc comment
(`cli/vcs/src/origin_remote.rs:12-22`) explicitly documents a divergent
error-handling contract ("callers must be able to distinguish 'cleanly
absent' from 'probe malfunctioned', unlike `RepoRoot`/`VcsProbe`'s infallible
fold-to-`None` convention"), and `CheckoutProbe`'s trait doc
(`cli/vcs/src/classify.rs:78-90`) and `checkout::DualRoots`'s struct doc
(`cli/vcs/src/checkout.rs:31-36`) do the same for their own fallibility
rules. **The natural location for the sha256 decision is a doc comment on
`VcsProbe`/`VcsProbe::revision` in `cli/vcs/src/lib.rs:65-73`**, following
that precedent.

`is_full_revision_id` exists **only** in `cli/vcs-adapters/tests/detection.rs:94-96`
— not in production code anywhere — and already carries a doc comment naming
the exact decision this item must record:
```rust
/// Note this rejects a **sha256** repository's 64-hex id. That is deliberate
/// here: no fixture in this suite uses one. Any wider revision validation has to
/// accept both widths or record sha256 as unsupported — a decision the
/// composition-root switch inherits, since it is what exposes users to it.
fn is_full_revision_id(revision: &str) -> bool {
    revision.len() == 40 && revision.chars().all(|c| c.is_ascii_hexdigit())
}
```

The sha256 failure mode itself (`gix` 0.85 returns `Err` rather than
misreading) is already documented and tested in
`cli/vcs-adapters/tests/queries.rs:562-567`
(`an_unsupported_object_format_fails_rather_than_misreads`) and
`cli/vcs-adapters/tests/classify.rs:191-198`. `VcsProbe::revision` is
infallible, so `library.rs`'s `git_revision` already folds any `gix`
failure — sha256 included — to `None` with a `warn!` log
(`library.rs:533-539`), matching `revision`'s documented contract. No new
`Error` variant is strictly required to represent the failure; what's
missing is only the policy decision and where it's written down.
`kernel::Error` (`cli/kernel/src/lib.rs:10-20`) has `LogFilter`/`Failed`/
`Refusal` variants and no unsupported-format-specific one; `vcs_adapters::library::Error`
(`library.rs:69-108`) already carries `Git { path, source }`, which a
sha256-triggered `gix` failure naturally routes through today.

### Visualiser server and hook-path reachability

**No code path inside `cli/visualiser/server/src` calls `derive_at`,
`VcsBackedRepoFactsProbe`, or `vcs_adapters::facts` today.** The only two
production callers of `derive_at` in the whole workspace are
`cli/work-cli/src/create.rs:258-263` and `cli/corpus-cli/src/main.rs:72-86` —
both short-lived, skill-invoked CLI processes, not the server. The
server does depend on `corpus_adapters`/`vcs-adapters` at the Cargo level
(`visualiser/server/Cargo.toml:22-23` → `corpus-adapters/Cargo.toml:25`,
unconditional), but the three `corpus_adapters` surfaces it actually calls
(`frontmatter.rs`, `compose.rs`, `file_driver.rs`) are frontmatter parsing,
the work-item-id regex scanner, and the status-only frontmatter patcher —
none touch `corpus_adapters::metadata`. This matches 0188's own verified
finding (quoted in 0185's Amendment inheritance 5): the whole `gix`/`jj-lib`
closure is currently dead-code-eliminated from the shipped
`accelerator-visualiser` binary. **0185's framing that "this switch is what
first makes `cli/visualiser/server` reachable into `vcs-adapters`'s closure"
appears to describe a server-side call site that does not exist in this
codebase yet** — worth flagging to whoever owns 0185/0168, since the
containment decision may need to be made proactively rather than in
response to an already-live path.

**The hook path already runs `InProcessProbe` synchronously, today, for a
different surface.** `hooks/hooks.json` wires `vcs detect --format=hook
--fail-safe --descriptive` at `SessionStart` and `vcs guard --format=hook
--fail-safe` at `PreToolUse` (matching `Bash`). `cli/vcs-cli/src/main.rs:27-34,59-69`
already constructs `InProcessProbe` for both. So the *type* this item
repoints `facts()` onto is already exercised, unbounded, on the
session-start and every-Bash-call hook path — just not via `facts()`/
`RepoFacts` (which `detect`/`guard` don't use). This is useful context for
the containment-bound AC: the risk class 0185 is being asked to price
already exists in production for `detect`/`guard`, uncontained, and 0185
would be extending the same code paths' reach rather than introducing an
entirely new exposure.

**No timeout/crash-isolation precedent exists in `cli/vcs-adapters` or at
any `InProcessProbe` call site.** `CommandProbe`'s containment
(`DEFAULT_CAP = 10s`, `wait_capped`'s poll-then-kill, `scrub_environment`) is
process-boundary containment being deleted along with the struct — though
its shared helpers survive for 0198. In the server, the closest existing
precedent is `server.rs:34,261`'s `TimeoutLayer` (30s) — cooperative
async-cancellation only, no preemption of blocking synchronous work — and
`file_driver.rs:197-220`'s `spawn_blocking` pattern for isolating blocking
I/O onto a dedicated thread, which has no timeout or reclaim-on-hang
behaviour of its own. **No `catch_unwind` exists anywhere in `cli/`.**
`InProcessProbe`'s `Result<Option<T>, Error>` return shape already
distinguishes failure from absence (the work item's own "containment of
meaning, not blast radius" framing), but no time/memory/crash bound wraps
any of it today.

## Code References

- `cli/vcs-adapters/src/lib.rs:22-26` — `facts()` composition root to repoint.
- `cli/vcs-adapters/src/subprocess.rs:65-139` — `CommandProbe` to delete.
- `cli/vcs-adapters/src/subprocess.rs:231-246,248-324` — `scrub_environment`/`run_capped`, shared with 0198, must survive.
- `cli/vcs-adapters/src/subprocess.rs:329-407` — `run_checked`/`wait_capped_checked`, deletable with `CommandProbe`.
- `cli/vcs-adapters/src/subprocess.rs:583-632` — `CommandProbe`-specific unit tests, redundant with `library.rs`.
- `cli/vcs-adapters/src/library.rs:190-478` — `InProcessProbe` and its port impls.
- `cli/vcs-adapters/src/library.rs:1009-1107` — existing `InProcessProbe` unit-test equivalents to the deleted `CommandProbe` tests.
- `cli/vcs-adapters/src/markers.rs` — shared walk/marker helpers, must not be touched.
- `cli/vcs-adapters/tests/detection.rs:101-154` — dual-comparison scaffolding to collapse.
- `cli/vcs-adapters/tests/library.rs:431-474` — the one documented probe divergence (jj snapshot-on-read).
- `cli/corpus-adapters/src/metadata.rs:189-236` — `derive`/`derive_at`/`VcsBackedRepoFactsProbe`, actual repoint target at `:214`.
- `cli/corpus-adapters/tests/zero_spawn.rs` — existing cross-crate zero-spawn proof, does not yet cover the metadata read.
- `cli/vcs-test-support/src/stubs.rs`, `src/hermetic.rs`, `src/fixtures.rs` — the shared harness.
- `.github/workflows/main.yml:323-377` — `check-zero-spawn` CI job.
- `tasks/test/integration.py:181-241` — `test:integration:zero-spawn:strong` implementation.
- `cli/deny.toml:67-81` — the `uluru` exception and its re-check trigger.
- `cli/Cargo.toml:72-73` — the `jj-lib`/`gix` pins.
- `cli/pup.ron:140-163` — `vcs_adapters_library_reads_in_process`.
- `tasks/github.py:231-248` — `_release_uploads()`.
- `tests/unit/tasks/test_workflows.py:207-221` — attest-glob coverage over release uploads.
- `cli/vcs/src/lib.rs:65-73` — `VcsProbe`, the natural home for the sha256 policy decision.
- `cli/vcs/src/origin_remote.rs:12-22` — precedent doc-comment style for recording a port policy.
- `cli/vcs-adapters/tests/detection.rs:87-96` — `is_full_revision_id`, already names the sha256 decision.
- `hooks/hooks.json` — `vcs detect`/`vcs guard` hook wiring.
- `cli/vcs-cli/src/main.rs:27-69` — `InProcessProbe` already in use on the hook path today.
- `cli/visualiser/server/src/server.rs:34,261` — `TimeoutLayer`, cooperative-only precedent.
- `cli/visualiser/server/src/file_driver.rs:197-220` — `spawn_blocking` precedent.

## Architecture Insights

- **Ports-and-adapters throughout** (ADR-0053): `cli/vcs` is a pure domain
  crate with zero I/O; `cli/vcs-adapters` supplies two adapters
  (`CommandProbe`/`MarkerWalkRoot`, and `InProcessProbe`) behind the same
  ports, exactly the shape `cli/corpus`/`cli/corpus-adapters` mirrors —
  0185 is a same-shape convergence to one this pattern already anticipated.
- **Delegation over duplication for shared walk logic**: both adapters route
  through `markers.rs` rather than each having their own copy, which is
  exactly why deleting one "strands nothing," per that module's own doc
  comment.
- **Injection composition roots stay thin and are the deliberate seam for
  this kind of swap**: `vcs_adapters::facts()` is a two-line function whose
  entire job is choosing which adapter pair to wire in — a pattern this repo
  uses consistently (`corpus_adapters::metadata::VcsBackedRepoFactsProbe` is
  itself another injection point one layer up).
- **Fallibility conventions are deliberate and documented at the port**:
  `RepoRoot`/`VcsProbe` fold to `Option`/`None` on purpose (a caller cannot
  distinguish "no repo" from "probe failed," and an adapter is expected to
  warn-log); `OriginRemote`/`CheckoutProbe` are `Result`-returning because
  that distinction matters for those callers. This crate has existing,
  citable precedent for recording exactly this kind of "why does this port
  behave this way" decision in trait-level doc comments — the natural
  mechanism for 0185's sha256 decision.
- **Zero-spawn is enforced two ways at once**: structurally, via a
  `cargo-pup` import-restriction rule scoped to the `library` module
  (`std::process` denied); and behaviourally, via a black-box test harness
  (`vcs-test-support`) that stubs and shadows real binaries and asserts no
  marker file appears. Neither alone would catch every regression — the pup
  rule can't see a spawn originating inside `gix`/`jj-lib` itself, and the
  black-box test alone wouldn't stop an in-crate `std::process::Command`
  from creeping back in.

## Historical Context

- `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md`
  (Pass 3/4) is the review that split 0169 into 0185/0186/0187/0188, on a
  unanimous scope-lens finding that the `corpus-adapters`/`CommandProbe`
  migration was "orthogonal, own risk profile, nothing in the hooks
  migration needs it."
- `meta/work/0188-library-backed-vcs-adapter.md` (done) delivered
  `InProcessProbe` deliberately unwired, specifically so 0185 could adopt it
  once proven — including reversing an earlier, wrong spike finding
  (amendment 8, withdrawn) that jj `revision` was out of reach; it is fully
  delivered (amendment 10).
- `meta/work/0198-vcs-agnostic-status-log-renderer.md`
  (draft, low priority) owns the separate `status`/`log` subprocess path in
  the same `subprocess.rs` module (`run_vcs_text`) — explicitly out of
  0185's scope, and the reason `scrub_environment`/`run_capped` must survive
  `CommandProbe`'s deletion.
- `meta/reviews/work/0185-converge-corpus-adapters-on-library-backed-vcs-review-1.md`
  (Pass 2, 2026-08-10, **APPROVE**) already reconciled the work item's
  earlier internal contradictions (stale attributions to 0169, an
  "invisible to callers" claim contradicted by the known snapshot
  divergence, ungated containment/sha256/licence decisions) into checkable
  Acceptance Criteria and Dependencies entries. One item was deliberately
  left as an in-line AC rather than split out: the MPL-2.0
  attribution-artefact work, judged cheap enough to track without a separate
  work item.
- `meta/work/0125-converge-vcs-detection-on-probe-layer.md` (draft) — a
  separate, shell-side convergence (`find_repo_root`/`vcs_mode`) whose
  stated rationale (probing costs 1-3 subprocesses, must work with no
  `git`/`jj` on `PATH`) is dissolved *for consumers that reach the Rust
  adapter* by 0188/0185, though its own ~26 shell call sites are unaffected
  until later epic-0136 phases migrate them.

## Related Research

- `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md` —
  the spike/API-surface research behind `InProcessProbe`'s delivery. Note:
  its `corpus-adapters` call-site line references (`derive_at` at `:201`)
  are now stale per this research's findings above.
- `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  and `meta/research/codebase/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`
  — pre- and post-split research for 0169, the parent story this item was
  extracted from.
- `meta/research/codebase/2026-08-06-0195-accelerator-corpus-cli-implementation-surface.md`
  — notes the `vcs`/`vcs-adapters` and `corpus`/`corpus-adapters` crate
  pairs share the same hexagonal port-injection shape.

## Open Questions

- **How should AC3's zero-spawn extension actually be structured?** The
  existing `zero_spawn.rs`/`vcs-adapters-fixture` pairing tests raw
  `InProcessProbe` queries, not the `facts()` composition or
  `VcsBackedRepoFactsProbe`. The plan needs to decide between adding a
  `facts`-calling query to the existing reference binary versus a new
  `corpus-adapters`-local one.
- **Should AC2 be corrected to drop the non-existent
  `work_item_pattern_parity.rs`, or does that suite need to be created as
  part of this item?** The work item's phrasing ("its existing suites pass
  unchanged") suggests the former, but this should be confirmed with
  whoever owns the AC text before the plan finalises it.
- **Is the visualiser-server containment question premature?** No call site
  in `cli/visualiser/server` reaches `vcs_adapters::facts` today, so the
  "this switch is what first makes the server reachable" framing in the
  work item's Dependencies/Amendment does not match the current codebase.
  Confirm with the 0168 owner whether a server-side metadata call site is
  imminent, or whether the containment AC should be reframed as
  precautionary rather than reactive.
- **Where exactly should the sha256 decision doc comment land** — on
  `VcsProbe` itself, on `VcsProbe::revision` specifically, or on both? The
  precedent (`OriginRemote::origin_url`) puts it on the method; `RepoFacts`/
  `VcsKind` have no obvious place to record it since they're plain data
  types.
