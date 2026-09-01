---
type: "codebase-research"
id: "2026-08-31-0198-vcs-agnostic-status-log-renderer"
title: "Research: VCS-agnostic status/log renderer (0198)"
date: "2026-08-30T23:32:14+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0198"
parent: "work-item:0198"
relates_to: ["codebase-research:2026-08-30-0198-vcs-agnostic-status-log-renderer"]
topic: "VCS-agnostic status/log renderer (0198)"
tags: ["research", "codebase", "vcs", "gix", "jj-lib", "cli", "status", "log"]
revision: "347017dc5c425304214b3b612161a1240001329a"
repository: "accelerator"
last_updated: "2026-08-30T23:32:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: VCS-agnostic status/log renderer (0198)

**Date**: 2026-08-31 (00:32 local / 2026-08-30T23:32:14+00:00 UTC)
**Author**: Toby Clemson
**Git Commit**: 347017dc5c425304214b3b612161a1240001329a
**Branch**: `visualisation-system` jj workspace (working copy, no bookmark)
**Repository**: accelerator

## Research Question

Ground work item 0198 — "Replace `vcs status`/`vcs log` with a VCS-agnostic
library-backed renderer" — against the live tree, so the item can move to
`/create-plan`. Confirm what is being replaced, what the accepted format ADR
fixes, which building blocks the 0188 library adapter already provides versus
what is net-new, and where the acceptance criteria meet gaps in the current
code.

## Summary

The change is well-bounded and every architectural seam already exists. 0198
replaces the last two `vcs` subcommands that spawn a process — `status` and
`log`, both quarantined in `cli/vcs-adapters/src/subprocess.rs` as infallible
`-> String` free functions — with an in-process renderer over `gix` and
`jj-lib`, feeding the single VCS-agnostic format that **ADR-0066 (accepted
2026-08-30)** now fixes. The sole consumer, the `/commit` skill, injects the
output as free-form orientation and parses no field, which is what makes owning
a new format safe.

**The data path is half-built.** Git working-copy status already runs
in-process (`library/dirty_paths.rs`), but three data reads are net-new: a git
revwalk for recent commits, a git tree/blob diff for change-kind + `--stat`
counts, and a jj revset/graph walk for recent changes. The jj change-set walk
itself (`TreeDiffIterator`) already exists and is reusable; only surfacing the
add/modify/delete distinction it currently discards is new.

**One work-item assumption is now falsified in the item's favour.** 0198's
Dependencies section warns the git status/diff surface "may require a `gix`
feature the current selection omits", re-opening deny.toml and licence review.
It does not: `blob-diff` is **already present** in the effective `gix` feature
graph via jj-lib unification, asserted present by
`tests/integration/deny/test_vcs_library_graph.py:59-66`. Declaring it
explicitly on `vcs-adapters` is hygiene, adds no crates, and leaves the licence
closure untouched — provided `default-features = false` holds and
`gix-credentials`/network features stay off.

**Two small, local gaps block AC6.** The `accelerator-vcs` sub-binary never
calls `kernel::logging::init()` (the launcher's init does not survive the
`exec()` process-image swap), so every fallback `warn!` is discarded; and no
warning carries a `gix`/`jj-lib` token (existing ones key on `vcs = "git"`/`"jj"`).
Both are a few lines to fix in `cli/vcs-cli/src/main.rs` and the fallback warn
call.

**The net-new test work is four items:** a conflict fixture (none exists in
either fixture system), a git-vs-jj parity harness (goldens are per-VCS today),
regenerating all 20 `vcs-status-log` goldens to the ADR shape, and extending the
`check-zero-spawn` CI job — currently scoped to detection/facts only — to
exercise status/log.

## Detailed Findings

### The change surface — what is replaced and where it rewires

Backend selection is already in-process; only the text rendering shells out.
Both handlers do the identical dance — `InProcessProbe.discover(start)` →
`probe.kind(root)` → run in the discovered root — then call the subprocess
adapter (`cli/vcs-cli/src/status.rs:14-22`, `cli/vcs-cli/src/log.rs:14-22`). The
swap point is a single line each: `status.rs:21` and `log.rs:21` call
`subprocess::status`/`subprocess::log` and must call the new in-process renderer
instead. The discover/kind/dir derivation above them stays.

`run_vcs_text` (`cli/vcs-adapters/src/subprocess.rs:66-99`) runs exactly four
commands under a scrubbed environment and a 10-second cap:

| Backend + subcommand | Command | Source |
|---|---|---|
| jj status | `jj --color=never --no-pager status` | `subprocess.rs:67-71` |
| jj log | `jj --color=never --no-pager log --limit 5` | `subprocess.rs:72-82` |
| git status | `git -c color.ui=false diff --cached --stat` | `subprocess.rs:83-93` |
| git log | `git -c color.ui=false log --oneline -5` | `subprocess.rs:94-98` |

The current fallback literals are backend-specific — `(jj status unavailable)`,
`(jj log unavailable)`, `(git status unavailable)`, `(git log unavailable)`
(`subprocess.rs:104-113`). ADR-0066 replaces these with two neutral literals
(below). Empty output is explicitly **not** a failure: a clean `git diff
--cached --stat` returns `Some("")` and is preserved (`subprocess.rs:169-175,
209-211`); output is `trim_end`-only because git's `--stat` leading space is
significant.

`status`/`log` are `#[must_use] -> String` and never return `Err`
(`subprocess.rs:36, 43`); the CLI wrappers document "Never fails". Deleting the
subprocess renderer also retires `run_vcs_text`, `run_capped`, `wait_capped`,
`scrub_environment`, the `DEFAULT_CAP`/`POLL_INTERVAL` constants, and the
subprocess-only tests (`subprocess.rs:214-369`).

`--fail-safe` is orthogonal and stays untouched. It is a launcher-consumed flag
(`cli/vcs-cli/src/cli.rs:34-47`), ignored by the status/log handlers
(`main.rs:90-91`, `fail_safe: _`); its only effect is
`swallow_under_fail_safe` in `cli/launcher/src/launch/core.rs:236-241`, which
exits 0 when the sub-binary itself cannot be resolved/exec'd
(`kernel::Error::Failed`). This is a distinct failure domain from the
adapter-failure fallback and must be kept distinct in the plan.

### The accepted format — ADR-0066

`meta/decisions/ADR-0066-vcs-agnostic-status-log-output-format.md` is **accepted**
(frontmatter + body line 21), `parent: work-item:0198`,
`relates_to: [adr:ADR-0053, adr:ADR-0054]`. It selects one VCS-agnostic text
format rendered identically from git and jj (Option 2; byte-parity and JSON
rejected). AC1 is therefore satisfied ahead of planning.

**Status format** (ADR lines 77-97):

```text
Branch: <name>
<N> changed[, <K> conflicted]
  <change-type>  <path>
  ...
```

- `Branch:` is one neutral header line. Value = git branch, or the jj
  working-copy commit's bookmark(s), comma-separated in byte order; literal
  `(none)` when git is detached or the jj commit carries no bookmark (the common
  jj case).
- Summary line `<N> changed`; append `, <K> conflicted` when any file is
  conflicted (`K ⊆ N`, a conflicted file counted once).
- Empty state is the single line `No changes`.
- Change-type markers are **word labels**, closed set of five:
  `added`, `modified`, `deleted`, `untracked`, `conflicted`. Each file line is
  `  <change-type>  <path>` (two-space indent), sorted by repo-relative path in
  byte order.
- Git staging **collapses**: a staged change renders as its change type with no
  staged/worktree distinction (jj has no staging area). This resolves the AC3
  question — staged does not become a literal "modified", it collapses into its
  own type.
- Renames are not a distinct type: surfaced as `deleted` (old) + `added` (new).

**Log format** (ADR lines 103-117): a flat list of up to five most recent
commits, newest first, each `<short-id> <subject>`. No author, no date, no graph
glyphs. The walk is **first-parent ancestry** — from git `HEAD`, and from the jj
working-copy commit's first parent — with jj's working-copy commit (`@`) and the
virtual root **excluded**. Empty subject → `(no description)`; no commits →
single line `No commits`. Abbreviation width is an implementation choice,
normalised by mask in goldens.

**Fallback text** (ADR lines 121-128): on any adapter failure, status returns
`(status unavailable)` and log returns `(log unavailable)` — backend-neutral, no
`git`/`jj` prefix. The failed-adapter identity surfaces only on the
`ACCELERATOR_LOG` path. Functions stay infallible (`-> String`).

**Behaviour changes the ADR records** (lines 145-158): untracked *and* modified
files now appear in status (today's git status is staged-only `diff --cached
--stat`); staged no longer distinguished; jj status reports state as of the last
operation (no snapshot-write, see below). Several goldens empty today become
non-empty under the always-present header.

### The 0188 library adapter — reuse versus net-new

The VCS subdomain is a hexagon: `cli/vcs/` holds ports + value types (no I/O),
`cli/vcs-adapters/` answers them in-process via `gix`/`jj-lib`. Every *probe*
(root, kind, revision, identity, classification, dirty-paths, tracked) is
already library-backed; only status/log text is not. There is **no status/log
port** in the domain crate today — they are concrete functions in the subprocess
adapter, so 0198 introduces a backend-neutral status/log model in `cli/vcs/`
(beside `RepoFacts`, touching the pinned public-api baseline) and a `match kind`
dispatch shaped exactly like `dirty_paths(root, kind)` (`library.rs:391-426`).

| Capability | State today | Location |
|---|---|---|
| git working-copy status (changed paths) | Reusable as-is | `library/dirty_paths.rs:34-62` |
| jj change-set walk (`TreeDiffIterator`) | Reusable; change-kind discarded | `library/dirty_paths.rs:117-160` |
| jj settings-load pipeline (Route B) | Reusable as-is | `dirty_paths.rs:75-116`, `tracked.rs:40-81` |
| repo open helpers, error type, path helpers | Reusable as-is | `library.rs:71-187, 880-926, 933-960` |
| git recent-commit revwalk | **Net-new** (only `head_commit()` today) | `library.rs:582-584` |
| git tree/blob diff (change-kind + `--stat`) | **Net-new** (no gix diff imported) | — |
| jj recent-change walk (revset/graph/dag) | **Net-new** (none imported) | — |
| jj change-kind surfacing (A/M/D per path) | **Net-new** (diff exists, distinction dropped) | `dirty_paths.rs:147` |

The jj log path follows **Route B**, already precedented:
`UserSettings::from_config(StackedConfig::with_defaults())` →
`DefaultWorkspaceLoaderFactory.create(root)` → `loader.load(&settings, …)` →
`repo_loader().load_at_head()` → `view().get_wc_commit_id` → `store().get_commit`
(`dirty_paths.rs:75-116`). The new jj source file must be added to the
`_EXEMPT` set in `tasks/lint/vcs_settings.py` (the guard blocks static
`Workspace::load` by name but not the instance-method `WorkspaceLoader::load`
Route B uses). This closes the item's sole Open Question: jj-lib Route B is
accepted, the item ships whole (git + jj).

⚠️ jj snapshot-on-read divergence (from 0185): Route B `dirty_paths` snapshots
but deliberately drops the lock without `finish()`, so it writes nothing and
reports state **as of the last operation**, unlike the `jj` binary which writes a
fresh working-copy commit. Library-backed `vcs status` therefore reflects the
last operation and stops mutating the repo — ADR-0066 captures this (line 148).

### Dependency policy — the `blob-diff` finding

`gix` is pinned `~0.85.0` with `default-features = false` at the workspace
(`cli/Cargo.toml:137`); `vcs-adapters` declares `features = ["status"]` only
(`cli/vcs-adapters/Cargo.toml:38`). jj-lib is pinned **exact** at `=0.43.0`
(`cli/Cargo.toml:136`) — the asymmetric pair, because a caret on gix 0.x would
cross to 0.86 and put two gix graphs in the lock (`cli/Cargo.toml:122-137`).

The effective resolved gix feature set is larger than `["status"]` through Cargo
unification with jj-lib's own gix selection, and is pinned by
`tests/integration/deny/test_vcs_library_graph.py`:

- **Present** (lines 59-66): `attributes`, `blob-diff`, `index`,
  `max-performance-safe`, `sha1`, `zlib-rs`.
- **Absent** (lines 70-75): `blocking-network-client`, `async-network-client`,
  `credentials`, `blocking-http-transport*`.

So the git diff-stat surface compiles today — `blob-diff` is already on. The
only clean move for 0198 is **hygiene**: declare `"blob-diff"` (and optionally
`"dirwalk"`) explicitly on `cli/vcs-adapters/Cargo.toml:38` so the dependency is
not silently inherited. This adds no crates (licence closure unchanged), stays
green against the feature-graph test, and must keep `default-features = false`
and never admit `gix-credentials` (banned at `cli/deny.toml:125`; the only MPL
exception is `uluru`, `deny.toml:66-102`). A jj-lib bump would move all of
jj-lib, gix, prost, pollster, the Rust toolchain (`1.90.0`), and the mise `jj`
CLI pin (`0.43.0`) in lockstep.

The pup rule `vcs_adapters_library_reads_in_process` (`cli/pup.ron:334-357`)
already permits `^gix(::|$)` and `^jj_lib(::|$)` and denies `^std::process`, so
new diff/revwalk/revset imports need **no pup edit** — they land under
`^vcs_adapters::library` and inherit the rule. Every `use` must stay single-item
(cargo-pup rejects grouped `use a::{b, c}` under an allow-list). Deleting
`subprocess.rs` removes the crate's only sanctioned spawn site, after which the
`std::process` deny effectively covers the whole crate.

### AC6 diagnosability — two gaps

⚠️ **Init gap (blocking).** `kernel::logging::init()`
(`cli/kernel/src/logging.rs:26-36`, reads `ACCELERATOR_LOG`, installs a global
stderr subscriber) is called only in `cli/launcher/src/main.rs:322` and the
visualiser server — **not** in `cli/vcs-cli/src/main.rs:82-101`. The launcher's
init does not carry into the sub-binary: dispatch is `Command::exec()`
(`cli/launcher/src/launch/outbound/exec.rs:15-17`), a process-image replacement,
so `accelerator-vcs` starts with no subscriber and every `warn!` on the fallback
path is discarded (both the subprocess warns and the library warns at
`library.rs:505, 524, 552, 559, 587, 600`). The module doc *promises*
diagnosability the process cannot currently deliver.

⚠️ **Token gap (blocking).** No warning carries the `gix`/`jj-lib` token AC6
requires; existing warnings key on `vcs = "git"`/`"jj"` (`subprocess.rs:104-105`,
`library.rs:506, 523`). The fallback warn must be changed to emit the adapter
token.

Both fixes are small and local: call `kernel::logging::init()` at the top of
`vcs-cli`'s `main` (kernel is already a dependency), and emit the fallback
warning with a `gix`/`jj-lib` field. AC6 is achievable.

### Test infrastructure — goldens, fixtures, masks, CI

The golden test `cli/vcs-cli/tests/status_log_goldens.rs` is gated
`#![cfg(feature = "bash-parity")]`, builds ten real git/jj checkouts in a temp
dir at runtime (`build_states`, lines 183-198), runs the compiled
`accelerator-vcs` binary against each with `--fail-safe`, masks volatile fields,
and does an **exact string compare** on the masked, newline-trimmed output
(lines 211-228). Only the 20 `.txt` goldens are checked in
(`cli/vcs-test-support/fixtures/vcs-status-log/`); the repos are rebuilt each
run. The ten states: `clean-git`, `dirty-git`, `detached-head-git`,
`git-ahead`, `git-behind`, `clean-jj`, `dirty-jj`, `colocated`, `jj-secondary`,
`no-repo`.

Shared apparatus in `cli/vcs-test-support/`:

- `hermetic.rs` — env isolation (`HOME`, `XDG_CONFIG_HOME`, `JJ_CONFIG`, strips
  inherited `GIT_*`), pinned identity, tool-floor guards (git ≥ 2.45, jj CLI/lib
  lockstep). Reusable as-is.
- `masks.rs` + `fixtures/masks.toml` — seven regex patterns (`hex_object_id`,
  `jj_change_id`, timestamps, `relative_age`, `fixture_tempdir_path`,
  `author_identity`), each with positive/negative samples so Rust `regex` and
  Python `re` stay pinned identically, cross-validated in `tests/masks.rs` and
  `tests/unit/vcs/test_masks.py`. ⚠️ Adding a mask is a four-file edit and the
  `masks.toml` header forbids adding/loosening a pattern to rescue a failing
  golden (0169's mask-closure rule, carried into AC3).
- `stubs.rs` — the zero-spawn harness (`Stubs`, `Mode` PathOnly/Strong,
  `assert_shadowing_holds`, `unshadowed_paths` over the six absolute paths,
  `reference_artefact`). Reusable as-is.

**Net-new for 0198:**

1. **Conflict fixture.** No merge-conflict checkout exists in either fixture
   system (`grep -i conflict` empty across `cli/vcs-cli` and
   `cli/vcs-test-support`). AC4 needs a new state builder + git and jj conflict
   goldens.
2. **git-vs-jj parity harness.** Nothing compares the same logical state across
   backends — goldens are per-VCS (`clean-git-*` vs `clean-jj-*`). AC3's "same
   logical state, same field labels/ordering/line structure" comparison is new.
3. **ADR-format goldens.** All 20 goldens hold raw `git`/`jj` output today
   (`dirty-git-status.txt` = raw `--stat`; ahead/behind/detached statuses empty;
   `no-repo-*` = `(git … unavailable)`). All must be regenerated to ADR-0066
   shape, and the renderer rewritten.
4. **Extend `check-zero-spawn`.** The job (`.github/workflows/main.yml:317-371`,
   Linux-only — shadowing fails under macOS SIP) runs `mise run
   test:integration:zero-spawn:strong`, which runs `cargo nextest … -p
   corpus-adapters --features bash-parity -E 'binary(zero_spawn)'`
   (`tasks/test/integration.py:119-179`). It `sudo mv`s every git/jj aside
   (PATH hits + six absolute paths) and asserts `stubs.spawns() == None` — but
   only over the detection/facts path, **not** status/log (which still spawn).
   Covering status/log needs the in-process reimplementation plus a new
   zero-spawn assertion wired into the same privileged job. Scope the shadow to
   `git`/`jj` only — `SystemClock::try_new` spawns `date` unconditionally.

The `status_log_goldens.rs:235-266` test that `--fail-safe` has no effect on a
successful status/log should survive the rewrite unchanged (the handler stays
infallible).

### The consumer — `/commit` skill

`skills/vcs/commit/SKILL.md:13-14` is the sole consumer, injecting both commands
via the `!` preprocessor with `--fail-safe`:

```text
!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs status --fail-safe`
!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs log --fail-safe`
```

The output is free-form orientation: the changed-file list feeds atomic-commit
grouping ("Identify which files belong together", line 27), recent log subjects
set message style (line 45). No field is parsed; no hook reads status/log (the
`SessionStart`/`PreToolUse` hooks consume only `vcs detect`/`vcs guard`,
confirmed by the 0169 review). The skill does no conflict handling today — AC4's
conflict indicator is the one native signal this consumer benefits from, so it
is a hard requirement of the format.

## Code References

- `cli/vcs-adapters/src/subprocess.rs:36-113` — the infallible `status`/`log`
  being replaced; commands `:66-99`, scrub `:118-133`, cap `:23-24, 138-167`,
  fallbacks `:104-113`.
- `cli/vcs-cli/src/status.rs:14-22`, `cli/vcs-cli/src/log.rs:14-22` — the CLI
  handlers; swap point at `:21` of each.
- `cli/vcs-cli/src/main.rs:82-101`, `cli/vcs-cli/src/cli.rs:34-47` — dispatch;
  `--fail-safe` ignored for status/log.
- `cli/launcher/src/launch/core.rs:216-241` — the real home of `--fail-safe`
  (`swallow_under_fail_safe`).
- `cli/vcs/src/lib.rs:20-49, 52-86` — `VcsKind`, `RepoFacts`, the port traits
  (new status/log model lands here, touching public-api).
- `cli/vcs-adapters/src/library.rs:391-426` — the `match kind` dispatch template
  the new renderer follows; `:582-584` single-commit peel (revwalk net-new);
  `:880-926` repo-open helpers.
- `cli/vcs-adapters/src/library/dirty_paths.rs:34-62` (git status, reusable),
  `:75-116` (jj Route B, reusable), `:117-160` (jj `TreeDiffIterator`, reusable;
  change-kind dropped at `:147`).
- `cli/vcs-adapters/Cargo.toml:38` — gix `features = ["status"]`; declare
  `"blob-diff"` here for hygiene.
- `cli/Cargo.toml:122-145` — gix/jj-lib/prost/pollster pins + single-gix
  invariant.
- `cli/deny.toml:66-102, 104-127` — uluru MPL exception, gix-credentials ban.
- `tests/integration/deny/test_vcs_library_graph.py:59-75` — the gix
  feature-present/absent snapshot (`blob-diff` present).
- `cli/pup.ron:334-357` — `vcs_adapters_library_reads_in_process` (permits
  gix/jj_lib, denies std::process).
- `cli/kernel/src/logging.rs:26-36` — `init()`; not called by `vcs-cli`
  (AC6 gap).
- `cli/launcher/src/launch/outbound/exec.rs:15-17` — `exec()` image swap that
  drops the launcher's logging init.
- `cli/vcs-cli/tests/status_log_goldens.rs`, `cli/vcs-test-support/{src,fixtures}`
  — golden harness, masks, hermetic env, zero-spawn stubs.
- `.github/workflows/main.yml:317-371`, `tasks/test/integration.py:119-179` —
  the `check-zero-spawn` job to extend.
- `skills/vcs/commit/SKILL.md:13-14` — the sole consumer.
- `meta/decisions/ADR-0066-vcs-agnostic-status-log-output-format.md` — the
  accepted format contract (AC1).

## Architecture Insights

- **The seam is already cut where it needs to be.** Backend selection is
  in-process; the subprocess boundary is exactly the two text renderings. 0198
  is a substitution behind a stable dispatch shape (`match kind`), not a
  restructuring — the git and jj adapters are the only decomposition seam, and
  the `vcs_adapters::subprocess` deletion gates on the second, which is why the
  item ships whole rather than split.
- **The format ADR de-risked the item before code.** 0169 left status/log on
  subprocess because reimplementing native text "with no byte-parity guarantee"
  was disproportionate. 0198 dissolves that objection by *dropping* byte-parity:
  ADR-0066 owns a single agnostic format, so the original blocker no longer
  applies.
- **Feature unification is load-bearing and invisible.** `blob-diff` reaches
  `vcs-adapters` through jj-lib, not its own manifest. The feature-graph test is
  the only thing making that closure legible — declaring inherited features
  explicitly is the hygiene lesson.
- **Diagnosability is a per-sub-binary responsibility.** Because dispatch is
  `exec()`, each sub-binary owns its own logging init. The AC6 gap is not a
  design flaw so much as an un-wired sub-binary; the fix belongs in `vcs-cli`'s
  `main`.

## Historical Context

- `meta/research/codebase/2026-08-30-0198-vcs-agnostic-status-log-renderer.md` —
  the prior 0198 research (pre-ADR). Its five key findings all hold; this pass
  folds in ADR-0066's acceptance and adds the `blob-diff`-already-present
  finding.
- `meta/decisions/ADR-0066-vcs-agnostic-status-log-output-format.md` — the
  accepted format decision (AC1's first deliverable).
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — Key
  Discovery recording why status/log were left on subprocess ("cannot be
  produced from the six taxonomy queries… no byte-parity guarantee").
- `meta/plans/2026-08-03-0188-library-backed-vcs-adapter.md`,
  `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md` — the
  gix/jj-lib in-process adapter this item extends.
- `meta/work/0185-converge-corpus-adapters-on-library-backed-vcs.md` — removed
  `CommandProbe`, leaving status/log as the subprocess module's only users; the
  source of the jj snapshot-on-read divergence note.
- `meta/reviews/work/0198-vcs-agnostic-status-log-renderer-review-1.md` — the
  work-item review pass that routed the format to ADR-0066 and pinned AC3/AC4/AC6.

## Related Research

- Prior 0198 research (2026-08-30) — superseded-in-part by this pass.
- 0169 research (2026-07-29, 2026-08-05) — the subprocess-choice history.
- 0188 research (2026-08-02) — the library adapter surface.
- 0201 research/plan (2026-08-30) — sibling in-process section-diff, same
  subprocess-retirement pattern.

## Open Questions

- ❓ **jj first-parent walk API choice.** ADR-0066 fixes the *shape* (five
  entries, first-parent, `@`/root excluded) but the jj-lib call path is net-new
  (`revset` vs `graph`/`dag_walk`). The plan should pick one and confirm it
  peels short change-id + subject without loading `@`. Not a blocker — the data
  is reachable (prior art `gulbanana/gg` renders its own graph from jj-lib).
- ❓ **sha256 git repositories.** gix 0.85 returns `Err` on sha256; the
  accept-64-hex-or-record-unsupported policy inherited from 0185 should be
  referenced for the git status/diff path. Confirm the fixture matrix's `S256`
  shape (in the 34-key taxonomy matrix, not the status/log ten) does not need a
  status/log golden.
- ❓ **`--stat` line counts vs change-type only.** ADR-0066's status format
  carries change-type + path, not insertion/deletion counts. Confirm the git
  renderer does not need the `blob-diff` *content* diff at all (change-kind is
  derivable from the tree diff's before/after presence) — if so, the `blob-diff`
  hygiene declaration is optional, and the git diff surface may reduce to tree
  entry-kind, not blob content.
