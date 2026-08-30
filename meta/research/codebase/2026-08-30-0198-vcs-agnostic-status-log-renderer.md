---
type: "codebase-research"
id: "2026-08-30-0198-vcs-agnostic-status-log-renderer"
title: "Research: VCS-agnostic library-backed status/log renderer (0198)"
date: "2026-08-30T22:40:31+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0198"
parent: "work-item:0198"
topic: "Replacing subprocess vcs status/log with a VCS-agnostic library-backed renderer"
tags: ["research", "codebase", "rust", "vcs", "cli", "gix", "jj-lib", "status", "log"]
revision: "99f7bd8d2f2a503778c1ddeadcddcdd9921511fa"
repository: "accelerator"
last_updated: "2026-08-30T22:40:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: VCS-agnostic library-backed status/log renderer (0198)

**Date**: 2026-08-30T22:40:31+00:00
**Author**: Toby Clemson
**Git Commit**: 99f7bd8d2f2a503778c1ddeadcddcdd9921511fa
**Branch**: (jj working copy; visualisation-system workspace)
**Repository**: accelerator

## Research Question

Ground the implementation of work item 0198 — replace the subprocess-backed
`vcs status` and `vcs log` with a single VCS-agnostic renderer computed
in-process from `gix` (git) and `jj-lib` (jj). What is the exact contract to
preserve, where does the new code live, what data is already reachable, what
enforcement and test surfaces move, and what constraints do the 0169/0188/0185
precedents impose?

## Summary

The change is well-bounded and every seam it needs already exists. `status`/`log`
are the last two `vcs` subcommands still spawning a process; they live in one
quarantined module (`cli/vcs-adapters/src/subprocess.rs`) as two free functions
returning `String` with no error channel, and their only consumer — the
`/commit` skill — treats the output as free-form human orientation and parses no
fields. That makes owning a new format safe.

Five findings shape the plan:

- **The renderer is net-new; the data path is half-built.** `gix`'s working-copy
  status is already called (`dirty_paths.rs`), but no revwalk exists (git `log`
  is new gix surface) and no revset/graph walk exists (jj `log` is new jj-lib
  surface). No structured status/log model exists anywhere — today's functions
  return pre-formatted `String`.
- **The jj Open Question has a concrete answer.** A jj `log` needs a loaded
  `Repo`, which needs `UserSettings::from_config(StackedConfig::with_defaults())`
  — already used by `dirty_paths.rs`/`tracked.rs` and exempted by name in
  `tasks/lint/vcs_settings.py`. The new jj file must be added to that `_EXEMPT`
  set. Precedented, not a wall.
- ⚠️ **AC6's `ACCELERATOR_LOG` diagnostic does not work today.** `accelerator-vcs`
  never calls `kernel::logging::init()`, so the `tracing::warn!` fallback events
  are discarded. The replacement must add that wiring or AC6 cannot pass.
- **The AC1 format ADR is ADR-0066** (highest committed is ADR-0064). It is
  unwritten and is the first deliverable.
- **Several status goldens are legitimately empty** (`clean-git`, both
  ahead/behind, `detached-head-git`). An agnostic status that always prints a
  header turns all of them non-empty — a real, intended behaviour change that
  the golden rewrite must absorb, alongside the fact that today's git `status` is
  `diff --cached --stat` (staged-only), so untracked/modified files are new
  output.

The item ships whole (git + jj) unless jj-lib is judged unacceptable, in which
case it re-scopes git-only and the jj adapter plus `subprocess` deletion move to
a follow-up — a deliberate re-scope, not a degraded pass.

## Detailed Findings

### 1. Current subprocess implementation — the contract to preserve

The dispatch chain is thin. `vcs-cli`'s `status::run`/`log::run` resolve
`(dir, kind)` through the `RepoRoot`/`VcsProbe` ports on `InProcessProbe`, then
call the concrete `subprocess::status`/`log` free functions, and `main` prints
the returned `String` with `println!`.

| Surface | Signature | Notes |
|---|---|---|
| `status::run` / `log::run` | `fn(&Path) -> String` | `#[must_use]`, infallible |
| `subprocess::status` / `log` | `fn(&Path, VcsKind) -> String` | `#[must_use]`, sole `std::process` owner |
| Backend select | `probe.kind(root)` | `.jj` wins colocated; no repo → `Git` |

The four commands run under a scrubbed environment (`GIT_DIR`, `GIT_WORK_TREE`,
`JJ_CONFIG`, … removed; `GIT_CONFIG_NOSYSTEM=1`, `/dev/null` global/system),
colour disabled by argv, and a 10 s cap (`subprocess.rs:66-99`,
`scrub_environment` `:118-133`, `DEFAULT_CAP` `:24`):

```text
(Jj, Status)   jj --color=never --no-pager status
(Jj, Log)      jj --color=never --no-pager log --limit 5
(Git|None, Status)  git -c color.ui=false diff --cached --stat
(Git|None, Log)     git -c color.ui=false log --oneline -5
```

**Never-fail is structural, not defensive.** `run_capped` returns
`Option<String>`; any spawn failure, non-zero exit, timeout, or read error folds
to `None`, and `run_vcs_text` maps that to the exact literal fallback
(`subprocess.rs:104-113`). The four literals are load-bearing contract:

```text
(jj status unavailable)   (jj log unavailable)
(git status unavailable)  (git log unavailable)
```

Empty output is **not** failure — a clean `git diff --cached --stat` returns
`Some("")`. Trailing-only trim preserves the significant leading space in
`--stat` output (`subprocess.rs:210-211`).

**`--fail-safe` is a different failure domain.** It is parsed per-subcommand
(`cli.rs:35-47`) but discarded in the handler (`main.rs:90-91`); its only effect
is at the **launcher**, where `swallow_under_fail_safe` exits 0 when an external
dispatch fails with `kernel::Error::Failed` (`launcher/src/launch/core.rs:216-241`,
`main.rs:357-368`). The golden `fail_safe_has_no_effect_on_a_successful_status_or_log`
pins byte-identity with and without the flag. AC6's internal adapter-failure
fallback is a distinct domain the ADR must name separately.

⚠️ **AC6 diagnostic gap.** The fallback sites already emit `tracing::warn!` with
a `vcs` field (`subprocess.rs:149, 157-161, 189, 197, 203`), but
`accelerator-vcs` **never installs a subscriber** — `main` does not call
`kernel::logging::init()` (which reads `ACCELERATOR_LOG`, `kernel/src/logging.rs:26-36`).
The events are discarded today. AC6 requires an `ACCELERATOR_LOG` run to emit a
line naming the failed adapter (`gix`/`jj-lib`), so the replacement must add the
`logging::init()` call and switch the warn fields from `"git"`/`"jj"` to the
adapter token the ADR defines.

### 2. Library-backed adapter — where the new code slots in

All library code lives in `cli/vcs-adapters/src/library.rs` behind
`InProcessProbe`, with one capability per submodule file
(`library/dirty_paths.rs`, `library/tracked.rs`), each exposing
`pub(super) fn git_*` / `jj_*` taking `(root: &Path, …)`. Capability methods on
`InProcessProbe` match on `VcsKind` and delegate to the pair. Backend selection
is marker-driven (`markers.rs:42-50`), jj winning colocated.

**The new capability follows the established shape:** add `library/status.rs` and
`library/log.rs` with `git_*`/`jj_*` fns; add `InProcessProbe::status`/`log`;
define backend-neutral data structs in the `vcs` domain crate alongside
`RepoFacts` (`vcs/src/lib.rs:43-49`); render those structs in `vcs-cli`,
replacing the `subprocess` pass-through.

**gix data already reachable** — the git status half is a short extension:

```rust
// dirty_paths.rs:34-46 — the only user of the "status" feature today
let status = repository
    .status(gix::progress::Discard)?
    .untracked_files(gix::status::UntrackedFiles::Files);
// reaches untracked-file PATHS only; no rename/mode/staged-vs-worktree split yet
```

`head_commit()` (`library.rs:582-590`) is the **only** commit peel — there is no
`rev_walk`. A git `log` is new gix surface (a revwalk over recent ancestors), and
a richer status (change-type markers, staged vs worktree) means extracting more
than untracked paths from `repository.status(...)`.

**jj data — two loading routes, and the settings question:**

- **Route A (settings-free, no write):** loader + `SimpleOpStore::load(path)` +
  proto-decode of `.jj/working_copy/checkout` (`library.rs:631-665`). Yields the
  working-copy commit **id** only. Enough for a bare revision, not for ancestry
  or trees.
- **Route B (loaded `Repo`, needs settings):** `UserSettings::from_config(StackedConfig::with_defaults())`
  + `loader.load(&settings, …)` + `repo_loader().load_at_head()`
  (`dirty_paths.rs:76-103`). A jj `log` (revset + graph walk over recent commits)
  reuses this to get a `Repo`, then adds new `revset`/`graph`/`dag_walk` surface
  that **does not exist anywhere today**.

The `UserSettings` guard (`tasks/lint/vcs_settings.py`) forbids `UserSettings`
and `Workspace::load` across `cli/vcs-adapters` **except** a hard-coded
`_EXEMPT` set — currently `library/dirty_paths.rs` and `library/tracked.rs`. The
exemption reasoning: snapshotting and tree reads genuinely need bundled-defaults
settings, and jj-lib's private defaults are "discovered one panic at a time". A
jj `log` file needing a loaded `Repo` must be **added to `_EXEMPT`**. Note the
guard blocks the static `Workspace::load` associate fn by name; the instance
method `WorkspaceLoader::load(&settings, …)` used in Route B is not matched and
stays allowed.

⚠️ **jj snapshot-on-read divergence (inherited from 0185).** Route B's
`dirty_paths` snapshots the working copy but deliberately drops the lock without
`finish()`, so it writes nothing and reports state as of the last recorded
operation — unlike the `jj` binary, which snapshots and *writes* a new commit.
A library-backed `vcs status` therefore reflects the last operation, not a fresh
snapshot, and stops mutating the user's repo. The ADR should note this as a
behavioural change from the subprocess output.

### 3. Test infrastructure — goldens, masks, fixture matrix

The suite (`cli/vcs-cli/tests/status_log_goldens.rs`, gated
`#![cfg(feature = "bash-parity")]`) builds ten checkout states at test time by
shelling to real `git`/`jj` through a hermetic wrapper (`vcs-test-support`
`hermetic.rs`), runs the compiled binary, applies regex masks, and asserts exact
string equality against 20 committed `.txt` goldens. **Fixtures spawning real
binaries is fine and orthogonal** to the code-under-test's no-spawn rule.

Fixture matrix (1:1 with a single backend each — no cross-backend pairing):

| State | Backend | Status golden today |
|---|---|---|
| `clean-git` | git | empty |
| `dirty-git` | git | staged file only (`diff --cached --stat`) |
| `detached-head-git` | git | empty |
| `git-ahead` / `git-behind` | git | empty (divergence only in log count) |
| `clean-jj` / `dirty-jj` | jj | native jj status text |
| `colocated` | jj-wins | native jj text (`main` bookmark) |
| `jj-secondary` | jj | native jj graph, two heads |
| `no-repo` | none→git | `(git … unavailable)` fallback |

Implications for the rewrite:

- ⚠️ **Empty status goldens are load-bearing.** `clean-git`, both ahead/behind,
  and `detached-head-git` are empty because `diff --cached --stat` reports
  nothing. An agnostic status with an always-on header makes them non-empty — a
  deliberate change the rewrite absorbs.
- **Staging is git-only** (jj has no staging area); AC1 decides whether staged
  collapses into "modified" in the agnostic format.
- **No conflict fixture and no cross-backend parity harness exist** — both are
  net-new. AC4's conflict recipe (a merge with conflicting edits) and AC3's
  "same logical state in git and jj" harness must be built.
- **Masks are dual-validated in Rust and Python.** Seven patterns in
  `masks.toml`, each with `sample_match` + `sample_no_match` (the "unmasked
  control"). `EXPECTED_PATTERN_NAMES` is asserted in both
  `cli/vcs-test-support/tests/masks.rs` and `tests/unit/vcs/test_masks.py`, so
  adding a mask is a **four-file edit** (toml + two name lists + the golden). The
  `masks.toml` header forbids adding/loosening a pattern to rescue a failing
  golden — 0169's mask-closure rule, carried into AC3.
- **The `--fail-safe` neutrality test and the `(… unavailable)` fallback are
  behavioural invariants** the new format must keep.

### 4. Dependency policy and enforcement surfaces

| Surface | File | What moves for 0198 |
|---|---|---|
| No-spawn import rule | `cli/pup.ron:316-338` | Widen `vcs_adapters_library_reads_in_process` to cover new `library` submodules; single-item `use` only |
| gix features | `cli/vcs-adapters/Cargo.toml:38` | Already `features = ["status"]`; a richer status/diff may need more, re-opening deny + licence review |
| gix pin | `cli/Cargo.toml:117-147` | `~0.85.0`, `default-features = false` — must stay; never re-admit `gix-credentials` |
| MPL exception | `cli/deny.toml:100-102` | `uluru` MPL-2.0; any new exception cites this item |
| jj `UserSettings` guard | `tasks/lint/vcs_settings.py:46-51` | Add the new jj file to `_EXEMPT` if it loads a `Repo` |
| Zero-spawn CI | `.github/workflows/main.yml:317-361` | `check-zero-spawn` (Linux-only, strong absolute-path shadow) must exercise `vcs status`/`log` |
| public-api | `tasks/public_api.py` | `vcs` **is** pinned (`vcs/tests/fixtures/public-api.txt`); `vcs-adapters` is **not** — new domain structs touch the pinned baseline |

The pup rule is path-scoped to `^vcs_adapters::library($|::)` and pairs a permit
list (`gix`, `jj_lib`, `pollster`, `prost`, `tracing`, `vcs`, `crate`, std) with
an explicit `^std::process` deny — this is what makes zero-spawn structural, not
merely tested. The `subprocess` sibling module is deliberately outside it and may
spawn.

**The zero-spawn strong assertion** (`tasks/test/integration.py:120-179`,
`vcs-test-support/src/stubs.rs`) moves the real `git`/`jj` binaries out of every
absolute path (`/usr/bin`, `/usr/local/bin`, `/opt/homebrew/bin`, and jj's mise
install path on CI) and asserts no marker is written *and every value matches an
unrestricted run* — because an adapter degrading to `None` also writes no marker.
It runs only on privileged ephemeral Ubuntu (SIP blocks it on macOS). Today it
drives the corpus-adapters fixture; 0198 extends it to `status`/`log`. Mind the
carve-out: scope the assertion to `git`/`jj`, since `SystemClock::try_new` spawns
`date` unconditionally.

### 5. The consumer — `/commit` skill

`skills/vcs/commit/SKILL.md:13-14` injects both via the `!` preprocessor with
only `--fail-safe`. The status output is wired to atomic-commit grouping
("Review the VCS status and diff above to see what files changed", lines 22, 27);
the **log output is injected but referenced by no instruction**; there is **no
conflict handling** anywhere in the skill. It parses no fields and relies on no
labels or ordering. A grep confirms this is the only consumer — no hook, agent,
or other skill reads the text. A format that keeps changed-file names readable is
non-degrading; the conflict marker AC4 mandates is new information the skill does
not yet use but the developer benefits from.

## Code References

- `cli/vcs-adapters/src/subprocess.rs:36-133` — `status`/`log`/`run_vcs_text`, scrub, cap, fallback literals
- `cli/vcs-cli/src/status.rs:14-22`, `cli/vcs-cli/src/log.rs:14-22` — thin pass-through entry points
- `cli/vcs-cli/src/main.rs:82-101` — dispatch; `run_status`/`run_log`; no `logging::init()`
- `cli/vcs-cli/src/cli.rs:35-47` — `--fail-safe` per-subcommand parse (discarded)
- `cli/launcher/src/launch/core.rs:216-241`, `main.rs:357-368` — launcher `--fail-safe` swallow
- `cli/kernel/src/logging.rs:26-36` — `ACCELERATOR_LOG` subscriber init (not called by vcs-cli)
- `cli/vcs-adapters/src/library.rs:199-540` — `InProcessProbe`, port impls, gix/jj helpers
- `cli/vcs-adapters/src/library/dirty_paths.rs:34-160` — gix `status`; jj Route-B loaded `Repo` + tree diff
- `cli/vcs-adapters/src/library/tracked.rs:27-93` — jj tree read needing a loaded `Repo`
- `cli/vcs/src/lib.rs:43-77` — `RepoFacts`, `RepoRoot`, `VcsProbe` ports (home for new structs)
- `cli/vcs-cli/tests/status_log_goldens.rs:52-266` — fixture builders, golden compare, fail-safe neutrality
- `cli/vcs-test-support/fixtures/masks.toml`, `cli/vcs-test-support/tests/masks.rs`, `tests/unit/vcs/test_masks.py` — dual-validated masks
- `cli/pup.ron:316-338` — `vcs_adapters_library_reads_in_process` no-spawn rule
- `cli/deny.toml:100-127` — uluru MPL exception; `gix-credentials`/`curl-sys` bans
- `cli/Cargo.toml:117-147`, `cli/vcs-adapters/Cargo.toml:38-39` — gix/jj-lib pins and `status` feature
- `tasks/lint/vcs_settings.py:44-98` — `UserSettings`/`Workspace::load` guard and `_EXEMPT`
- `.github/workflows/main.yml:317-361` — `check-zero-spawn` job
- `tasks/test/integration.py:16-179`, `cli/vcs-test-support/src/stubs.rs` — absolute-path shadow harness
- `skills/vcs/commit/SKILL.md:13-14, 22, 27` — sole consumer

## Architecture Insights

- **The decomposition seam is the two adapters feeding one renderer.** The item
  is deliberately whole: git and jj adapters populate one backend-neutral struct;
  a shared renderer turns it into text; the `subprocess` deletion gates on the jj
  half. Splitting leaves a half-migrated state with no standalone value.
- **New structured model in `vcs`, adapters in `vcs-adapters`.** Following
  `RepoFacts`, the status/log data types belong in the pinned `vcs` domain crate
  (touching its public-api baseline); the `git_*`/`jj_*` populators belong in new
  `library/` submodules under the unpinned `vcs-adapters` (whose surface may grow
  freely). The renderer replaces the `vcs-cli` pass-through.
- **Zero-spawn is enforced by a path-scoped import rule, not discipline.** New
  code must sit inside `vcs_adapters::library`, import one item per `use`, and
  never name `std::process`. `subprocess` is the one module allowed to spawn, and
  it is being deleted.
- **Two failure domains stay separate.** Internal adapter failure → ADR fallback
  text (never-fail, diagnosable via `ACCELERATOR_LOG`); launcher fetch/dispatch
  failure → `--fail-safe` exit-0 swallow. The ADR names both.

## Historical Context

- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — chose subprocess for
  status/log because they "cannot be produced from the six taxonomy queries …
  with no byte-parity guarantee"; explicitly names 0198 as the follow-up that
  owns that choice. 0198 dissolves the objection by dropping byte-parity.
- `meta/work/0188-library-backed-vcs-adapter.md` (+ plan
  `meta/plans/2026-08-03-…`) — the in-process precedent: the six-way pin, the
  `uluru` MPL exception, `gix default-features = false`, the pup rule 0198
  widens, and the `check-zero-spawn` strong-form job 0198 extends. Records the
  sha256 limitation (gix 0.85 returns `Err` on sha256 repos; reftable reads fine)
  and the unbounded-containment reality (in-process parsing has no cap/isolation).
- `meta/work/0185-converge-corpus-adapters-on-library-backed-vcs.md` — removed
  `CommandProbe`, the completed precondition for full-module deletion; warns the
  capped-stdout helper and env scrub are `run_vcs_text`'s, for 0198 to delete;
  carries the jj snapshot-on-read divergence.
- `meta/work/0125-converge-vcs-detection-on-probe-layer.md` — the
  no-PATH-dependency and ~3.6-4.7 ms-vs-~23.8 ms latency evidence 0198 borrows.
- `meta/reviews/work/0198-…-review-1.md` — three passes → APPROVE (reviewer
  override). Sequenced the format ADR first (AC1); mandated absolute-path
  shadowing over PATH-only; made git-only a deliberate re-scope; corrected the
  staged fixture to git-only. Deferred verbatim ADR strings, commit-order
  verification, and an indicative output example to AC1's ADR and `/create-plan`.
- **Next ADR number is 0065** (highest committed is
  `meta/decisions/ADR-0064-…`). Relevant existing ADRs: `ADR-0053`
  (hexagonal ports/adapters), `ADR-0054` (git-style modular CLI), `ADR-0030`
  (ADR template), `ADR-0029` (sequential identifiers).

## Related Research

- `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md`
- `meta/research/codebase/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`
- `meta/research/codebase/2026-08-10-0185-converge-corpus-adapters-library-backed-vcs.md`
- `meta/research/codebase/2026-03-16-jujutsu-integration-and-vcs-autodetection.md`

No prior 0198-specific research or plan exists — this is the first.

## Open Questions

- **jj-lib maintenance cost (0198's sole Open Question) — resolution path
  identified, decision still owed.** A jj `log`/richer `status` needs a loaded
  `Repo` via bundled-defaults `UserSettings` (Route B), reachable and precedented
  (add the new file to `vcs_settings.py`'s `_EXEMPT`). The open decision is
  whether taking on more of jj-lib's unstable surface (`revset`/`graph`/
  `dag_walk`, not just the checkout-state read) is acceptable, or whether 0198
  ships git-only. Resolve alongside AC1's ADR, before the jj adapter is built.
- **sha256 repositories.** gix 0.85 returns `Err` on sha256 (`extensions.objectFormat`);
  the accept-64-hex-or-record-unsupported policy inherited from 0185 must be
  referenced by AC1's ADR for the git status/diff path.
- **Change-type markers and staging collapse.** AC1's ADR fixes the exact markers
  (added/modified/deleted/untracked), whether git staged collapses into
  "modified", the conflict marker text, per-subcommand fallback strings, and log
  depth (five) — the verbatim strings the review deferred.
- **AC6 logging wiring.** Confirm whether `kernel::logging::init()` in
  `accelerator-vcs` is in 0198's scope (it must be, for AC6) or a separate fix,
  and settle the adapter-name token (`gix`/`jj-lib`) the warn fields must carry.
