---
type: "plan"
id: "2026-10-06-0299-crate-dependencies-adr-0069"
title: "Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069 Implementation Plan"
date: "2026-10-06T11:35:34+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0299"
parent: "work-item:0299"
derived_from: ["codebase-research:2026-10-06-0299-crate-dependencies-adr-0069"]
relates_to: ["adr:ADR-0069", "adr:ADR-0054", "plan:2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys"]
tags: ["cli", "architecture", "dependencies", "refactor", "build-system"]
revision: "2dac05f5ee7d5703185c83438b8a3b0effad12de"
repository: "accelerator"
last_updated: "2026-10-06T16:30:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069 Implementation Plan

## Overview

Every crate in the `cli/` workspace declares its ADR-0069 role, context and
kind. A `cargo metadata` lint in `mise run check` rejects forbidden normal and
build edges. Every edge in 0299's violations table is removed: by a port
injected at a composition root, by moving context-owned logic into the
upstream domain, or by moving a context-free primitive into a technical
library. The launcher answers its SessionStart tracking question in-process,
and the `vcs tracking` subcommand and the `Unchecked` state go.

## Current State Analysis

`cargo metadata --no-deps` shows 43 members, none declaring
`[package.metadata.accelerator]`. The live graph matches 0299's violations
table edge for edge (research, "Current crate graph").

```text
launcher ──> tracker-support (rule 6), store (simplification)
consent-adapters ──> vcs-adapters (rules 4, 7)
corpus-adapters ──> vcs-adapters
work-adapters ──> vcs-adapters
research-adapters ──> corpus-adapters
jira-client, linear-client ──> corpus-adapters (lock only)
migrate-adapters ──> config-adapters, vcs-adapters, corpus-adapters, work-adapters
migrate ──> document (rule 1)
work ──> config (dead)
collaboration ──> vcs (OriginRemote only)
```

Nothing checks crate-graph direction today: `cargo-pup` checks `use` paths
and `cargo-deny` checks third-party crates.

## Desired End State

- Every member declares the role, context, kind and downstreams in 0299's
  Expected declarations table, and a unit test pins the table.
- `invoke lint.crate-dependencies.check` passes over the shipped workspace,
  with no exception list, and is reached from both `cli:check` and
  `lint:check`.
- `consent-adapters` is not a member. The only normal adapter → adapter edges
  across contexts run from `jira-client` and `linear-client` to
  `tracker-support`.
- The launcher's normal workspace dependencies are `kernel`, `config`,
  `config-adapters`, `vcs` and `vcs-adapters`, plus technical libraries other
  than `store`.
- `accelerator vcs tracking --path x` fails as an unknown subcommand.
  SessionStart warns about a tracked `config.local.md` in every case, and
  never emits a "was not checked" note.
- Characterisation tests committed in Phase 2 pass with no test-code change;
  only Phase 7's four reviewed goldens differ.
- `deny.toml`'s symbol-count table and `uluru` record match the re-measured
  binaries. `mise run notices:check` passes.
- Before and after launcher figures, their ratios and the rule 3 outcomes are
  recorded in this plan's Implementation Notes.
- `mise run` exits 0.

### Key Discoveries:

- `corpus-adapters/src/document.rs:69-96` maps `document::Yaml` to
  `corpus::FrontmatterValue` one to one. Both `Mapping`s are ordered `Vec`s,
  the scalar variants match, and `PartialEq` is derived on both. So m0008's
  before/after equality check gives the same answer over
  `to_value(document::parse(..))`.
- `classify` is not `document::parse`. It returns `Absent` for unfenced
  content where `parse` returns an empty mapping, and `Malformed` for a
  sequence or non-null scalar root that `parse` accepts. m0001, m0008 and
  `detect_loss` therefore need `parse`'s exact semantics and raw `split`,
  not `classify`. `classify` itself stays unchanged: it drives the
  visualiser's `frontmatter_state` field.
- `frontmatter_validation.rs:52-58` is the pipeline's only contact with
  `document`. It classifies, then calls `document::split` only for `Parsed`
  content, so the pipeline composes the port's `classify` and raw split.
- Cargo package names differ from directory names for the roots:
  `launcher` is `accelerator`, `verify` is `accelerator-verify`, `<x>-cli` is
  `accelerator-<x>` (except `jira-cli` and `linear-cli`, whose packages keep
  the directory name), and `visualiser/server` is `accelerator-visualiser`.
  Every `-p` and every declaration-table key uses the package name.
- `vcs::OriginRemote::origin_url` returns
  `Result<Option<String>, kernel::Error>`, and `collaboration` tests its
  error path (`collaboration/src/lib.rs:218`).
- `corpus_adapters::lock::acquire` fails three ways: `LockTimeout`,
  `NotWritable` (mkdir `PermissionDenied`) and `Io`. Callers print the error's
  `Display` directly. `LockOptions::default().ceiling_ms` is 300 000 with no
  environment override, so binary-level contention waits five minutes.
- With no `accelerator-vcs` in the cache and no `ACCELERATOR_VCS_BIN`, today's
  launcher fetches it from `ACCELERATOR_RELEASE_BASE_URL` (default: the
  GitHub release for its version) within a 5 s budget, and emits the
  `Unchecked` note when that fails.
- `canonicalise_id` needs `regex` (`work_item_pattern.rs:15,440`), which
  `corpus`'s pup rule bars (`pup.ron:63-77`). It goes behind a `corpus` port.
- `lock::acquire` returns `corpus::StoreError` (`lock.rs:18`), and `store`
  cannot depend on `corpus`. It needs an error type of its own.
- `store` avoids `kernel` (`store/src/lib.rs:8-10`). The reason is error
  taxonomy, not constants, so `store` → `kernel` for `TEMP_PREFIX` is
  consistent with it.
- Dump validation is fail-closed, and its tracker-specific text is pinned
  (`launcher/tests/config_read.rs:985-1104`). Rule 6 rules out keeping
  `tracker-support` behind a port. So structural block validation moves into
  `config`, driven by the catalogue description.
- `CARGO_BIN_EXE_*` exposes only a crate's own binaries. Several existing
  binary tests call Rust APIs the refactor reshapes:
  - `migration_0008.rs:52-60` uses `work_adapters::sync::digest::local`;
  - `work-cli/tests/common/mod.rs:17-28` uses `token_keys()`.

  The characterisation suite is therefore pytest over built binaries.
- `FileSessionLogRewriter` is constructed by no composition root.
- `consent-adapters::command_runner` is a third consumer of repository roots,
  internal to the credential ports.
- `[profile.release] strip = true` (`cli/Cargo.toml:247-252`). Symbol counts
  need `CARGO_PROFILE_RELEASE_STRIP=false`; launcher size uses the shipped
  profile.

## What We're NOT Doing

- Changing any observable behaviour beyond removing `vcs tracking` and the
  `Unchecked` state with its note.
- Linting `use` paths or dev-dependencies. `remote-projection` → `work` and
  `work-adapters` → `jira-client` stay as dev edges.
- Enforcing rule 3 mechanically; it is recorded here and reviewed.
- Moving tracker ceiling interpretation, defaults or `FilterSchema` out of
  `tracker`/`tracker-support`.
- Collapsing the narrow consumer-side VCS ports (`corpus::RepoFactsProbe`,
  `work::create::VcsIdentityProbe`, `migrate::ports::WorkingCopy`); they stay
  and are implemented over the new `vcs` port.
- Restoring a launcher deadline on tracking (0299 Assumptions).
- Splitting 0299 into child work items.
- An interim launcher measurement after Phase 7: prereleases are unstable,
  so the after figures are taken once, in Phase 13.

## Implementation Approach

The work runs from safety net, to enforcement tool, to refactors, to
enforcement, to measurement. Each phase leaves `mise run` green and merges on
its own. The lint exists from Phase 3 but is reached from `check` only in
Phase 12, once the workspace passes it. Inside the refactors, each port or
primitive lands in the same phase as, or before, its consumers' switch.

Every production change is test-first: a failing unit or crate test, the
minimum code, then refactor. A pure move is guarded by the tests that move
with it plus the Phase 2 suite. The goldens stay byte-identical throughout,
except in Phase 7, which changes exactly the four "without `accelerator-vcs`"
summary goldens (tracked and untracked, git and jj) from the `Unchecked` note
to the in-process answer.

Three cross-cutting chores recur in every refactor phase, and each phase's
success criteria name them:

- `cli/pup.ron` rules updated for moved modules, with generated `.pup/*.json`
  files refreshed.
- `public-api.txt` fixtures regenerated through `mise run public-api:update`
  for every pinned crate whose surface changes, with the diff read before it
  is accepted.
- `jj diff --stat --from <phase base> tests/integration/characterisation/goldens`
  is empty, where `<phase base>` is the revision the phase started from
  (Phase 7: only its four named goldens).

---

## Phase 1: Launcher Measurement Harness and Before Figures

### Overview

Add two committed measure tasks and record the before figures, plus a symbol
count of every current binary, before any launcher change.

### Changes Required:

#### 1. Launcher size task

**File**: `tasks/measure.py`, `tests/unit/tasks/test_measure.py`, `mise.toml`
**Changes**:

- `@task(name="launcher-size")` builds
  `cargo build --release --locked --target aarch64-apple-darwin -p accelerator`
  in `CLI_DIR` and prints the byte size of
  `target/aarch64-apple-darwin/release/accelerator`. The shipped profile
  strips.
- A pure `launcher_size_report(path, size, host)` formats the figure with the
  host model and OS. Test-first against a fake path.
- mise leaf `measure:launcher-size`, `depends = ["deps:install:python"]`,
  outside `check` and `default`. The guard
  `test_no_measure_reaching_task_is_in_the_ci_mirror` already covers it.

#### 2. Summary latency task

**File**: `tasks/measure.py`, `tasks/shared/measurement.py` (reuse
`summarise`), `tests/unit/tasks/test_measure.py`, `mise.toml`
**Changes**:

- `@task(name="summary-latency")` builds the launcher and `accelerator-vcs`
  with `cargo build --release -p accelerator -p accelerator-vcs`. It creates
  three fixture repositories under a temporary directory: git, non-colocated
  jj and colocated jj. Each holds a tracked `.accelerator/config.local.md`.
  It then runs `accelerator config summary --format=hook` with
  `ACCELERATOR_CACHE_DIR` pointed at a temporary cache.
- The tracking path is pinned so before and after time the same question:
  - `ACCELERATOR_VCS_BIN` points at the locally built `accelerator-vcs`, so
    the before run dispatches tracking without a fetch;
  - `ACCELERATOR_RELEASE_BASE_URL` points at a refusing loopback address;
  - every run's output must carry the tracked-file warning and no "was not
    checked" note, or the task fails rather than records the figure.
- Each repository gets two modes:
  - **cold**: every run gets a fresh empty launcher cache and a fresh
    repository copy;
  - **warm**: one shared populated cache and repository.

  Each mode takes 3 discarded warm-ups and then at least 20 timed runs, and
  reports the median and p90 via `summarise`.
- A `--repository <path> --revision <rev>` run times the same command
  against a realistically sized repository. The task snapshots that revision
  into a temporary copy, so before and after time the same state. The report
  records the revision, index entry count and, for jj, operation-log length.
  Phase 13 reruns it with the same arguments, and its median ratio falls
  under the same 10× stop.
- The runner takes no stdin envelope, so it is a new
  `summary_measurement_runner` beside `subprocess_measurement_runner`, which
  always sends `STDIN_ENVELOPE`.
- Pure pieces are unit-tested with fake runners, following the
  `MeasurementRunner` Protocol pattern: fixture construction commands, the
  pinned environment, output assertion, mode scheduling (warm-ups discarded,
  cache reset per cold run), and report formatting.
- mise leaf `measure:summary-latency`.

#### 3. README

**File**: `tasks/README.md` "The measure namespace"
**Changes**: document both tasks, their method, the pinned tracking
environment, and that before and after must run on the same host. "Cold"
means a cold launcher cache, not a cold filesystem cache.

#### 4. Before figures

On the measurement host, take these before figures and record them, with the
host model, OS version and load, under Implementation Notes:

- `mise run measure:launcher-size`
- `mise run measure:summary-latency`
- `mise run measure:warm-dispatch` against the current release

Also record a symbol count of every shipped binary: build unstripped with
`CARGO_PROFILE_RELEASE_STRIP=false cargo build --release --target
aarch64-apple-darwin`, then run `nm -a <bin> | grep -c` for `gix_`, `jj_lib`
and `uluru`. This is the true "before" for the licence record, since the
research suspects `design`, `jira`, `linear` and `research` already link
`gix`.

### Success Criteria:

#### Automated Verification:

- [x] Unit tests pass: `uv run pytest tests/unit/tasks/test_measure.py`
- [x] Measure tasks stay out of the CI mirror: `uv run pytest tests/unit/tasks/test_mise.py`
- [x] Build-system checks pass: `mise run build-system:check`
- [x] `mise run` exits 0

#### Manual Verification:

- [x] Before launcher size is recorded with host details
- [x] Before summary latency (cold and warm; git, jj and colocated jj) is recorded with host details, plus the large-repository figure with its repository path and revision
- [x] Before warm-dispatch figure is recorded with host details
- [x] Before symbol counts for every binary are recorded

---

## Phase 2: Characterisation Suite

### Overview

Commit a pytest suite that runs the built binaries in git and jj repositories
and compares stdout, stderr and exit status. It passes against today's
binaries and must pass unchanged at the end.

### Changes Required:

#### 1. Harness

**File**: `tests/integration/support/characterisation.py`,
`tests/integration/characterisation/conftest.py`
**Changes**:

- `binaries` fixture: built on `tasks/test/helpers.py::accelerator_env`,
  which already builds the `ACCELERATOR_BIN`/`ACCELERATOR_<TOKEN>_BIN`
  overlay over `cli/target/debug/`. It is extended to a token-generic form if
  needed rather than reimplemented.
- Hermetic environment, mirroring `vcs-test-support/src/hermetic.rs`:
  isolated `HOME`/`XDG_CONFIG_HOME`/`JJ_CONFIG`, `GIT_CONFIG_NOSYSTEM=1`,
  `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CEILING_DIRECTORIES` from
  `tasks/shared/measurement.py::ceiling_directories`, a fixture identity,
  `LC_ALL=C`, `TZ=UTC`. An ancestor-repository guard, because gix ignores
  `GIT_CEILING_DIRECTORIES`.
- `ACCELERATOR_RELEASE_BASE_URL` and every `ACCELERATOR_*_API_URL` always
  point at one `http://` loopback server that counts the requests it
  receives. The production release fetcher refuses a non-`https` URL before
  any network I/O (`fetcher.rs:236-240`), so release dispatch never reaches
  it; the API overrides need the debug builds' `test-loopback` seam.
- `ACCELERATOR_LOG=off` in every case. The launcher logs INFO tracing with
  timestamps and module targets by default, which a module move would change;
  user-facing text goes through `eprintln!`, which this leaves intact.
- `repository` fixture parametrised over `git` and `jj` (non-colocated
  `jj git init`), with `track(path)` and `untrack(path)`. git uses `git add`;
  jj uses `jj commit`, with `.accelerator/.gitignore` listing the file for
  untracked.
- `Run(code, stdout, stderr, loopback_requests, observations)` and
  `assert_matches_golden(name, run, masks)`. `loopback_requests` is the
  loopback server's request count for the case, so it is golden content, not
  a separate assertion.
  Goldens live under `tests/integration/characterisation/goldens/`, and
  `UPDATE_GOLDEN=1` regenerates them. A missing golden fails, and
  `UPDATE_GOLDEN` is refused when `CI` is set. Masks replace volatile
  substrings (temporary root, revision hashes, timestamps) with `<NAME>`.
- Provider environment variables to scrub are hard-coded, not derived from
  Rust (`ACCELERATOR_JIRA_TOKEN`, `ACCELERATOR_JIRA_TOKEN_CMD`,
  `ACCELERATOR_LINEAR_TOKEN`, `GH_TOKEN`, `GITHUB_TOKEN`,
  `ACCELERATOR_OPENALEX_API_KEY`, ...).
- Network-reaching roots use a minimal Python loopback HTTP server (stdlib
  `http.server` in a thread). It records hits so that refusal is proven by
  zero hits. Base URLs come from `ACCELERATOR_*_API_URL`.

#### 2. Behaviours

**File**: `tests/integration/characterisation/test_*.py`
**Changes**: one module per behaviour group. Each case runs in git and jj,
except cases marked `@pytest.mark.vcs_specific`: the nested git repository,
the corrupted git index, the colocated jj repository and outside any
repository.

| Module | Cases |
|---|---|
| `test_consent_reads.py` | Six keys (`jira.token_cmd`, `jira.allowed_sites`, `linear.token_cmd`, `github.token_cmd`, `openalex.api_key_cmd`, `design.browser_path`), each tracked and untracked, through `jira-cli`, `linear-cli`, `collaboration-cli`, `research-cli`, `work-cli` (`sync --preview`) and `design-cli` (`executor ping`) |
| `test_summary_tracking.py` | `accelerator config summary --format=hook` with `config.local.md` tracked, untracked and outside any repository, plus a nested git repository (`discover_root` with `file_tracking`'s nearest-only walk), a corrupted git index and a colocated jj repository. Tracked and untracked run with `ACCELERATOR_VCS_BIN` set and with no `accelerator-vcs` available. Every case asserts stdout is exactly one JSON line |
| `test_config_dump.py` | `accelerator config` with `jira` and `linear` integration: pull and push valid, unset, empty mapping, non-mapping, `filters` not a mapping, unsupported filter, reserved `any`, `max_pages: 0`, unknown `max_pages.<sub>`, `all` plus `additional`, unknown key, wrong noun, bad push ceiling, `--fail-safe`, and blocks carrying two or three faults at once, including two unknown keys (top-level and `max_pages.<sub>`) and an unsupported filter ahead of a reserved one |
| `test_tracker_blocks.py` | The same blocks parsed by `jira-cli` and `linear-cli` (a command that resolves ceilings), and `work sync --preview` with a structurally invalid pull block, comparing exit status and error text |
| `test_metadata_derive.py` | `accelerator corpus metadata derive`: author, revision and repository name, clean and dirty working copy (masked clock) |
| `test_frontmatter_validate.py` | `corpus frontmatter validate` over a committed fixture corpus with each violation class, plus absent, sequence-root, scalar-root and null-root files |
| `test_work_commands.py` | `work create` (author), `work update` on a dirty working copy, `work sync --preview` and `work list` with a baseline (working-copy status). For create, update and sync-author: a lock dir holding a dead-PID `owner.<nonce>` sentinel (reclaimed, then success) and a lock under a read-only parent (not-writable text) |
| `test_research_commands.py` | A topic verb reaching `validate_path`/`validate_text`/`parse`, ledger read and write, and the arXiv lock reclaimed from a dead-PID sentinel |
| `test_tracker_caches.py` | `jira-cli` and `linear-cli` cache read and write, a cache lock reclaimed from a dead-PID sentinel, and a cache lock under a read-only parent |
| `test_collaboration.py` | `collaboration-cli` resolving the base repository with an `origin` remote present and absent, against the loopback API |
| `test_migrations.py` | Fixture corpora: m0001 with no, empty, null-root, sequence-root, scalar-root, unterminated-fence and invalid-YAML frontmatter; m0008 with a canonical rendering that differs from its source, a meta/ file with no frontmatter, sequence- and scalar-root files, a comment-loss case and sync-baseline refresh (raw `last-sync.json` compared against literal hashes), with `paths.integrations` relative and absolute (outside the repository); a migrate run-lock held by a live PID (refusal text); the full registry |

Lock timeouts are not exercised at binary level: the 300 s default ceiling
would make each case wait five minutes. Timeout text is pinned by the moved
`store::lock` tests, which use short `LockOptions`.

The "no `accelerator-vcs` available" summary cases record today's `Unchecked`
output deterministically: the fetcher refuses the `http://` loopback release
URL before any network I/O. Phase 7 changes exactly these four goldens
(tracked and untracked, git and jj) from the "was not checked" note to the
in-process answer: a warning when tracked, nothing when untracked. The diff is confined to golden files and
is reviewed in Phase 7; no test code changes.

#### 3. Wiring

**File**: `mise.toml`, `tasks/test/integration.py`,
`tests/unit/tasks/test_mise.py`
**Changes**: a `test:integration:characterisation` leaf, added to the
`test:integration` roll-up and to `_LAUNCHER_DEPENDENTS`.

Debug builds are modelled per binary and composed into groups (decided with
the author):

- **Per-binary leaves:** `build:cli:dev` builds the launcher alone.
  `build:cli:<token>:dev` builds one dispatched sub-binary, for every token in
  `DISPATCHED_SUBBINARIES` except `visualiser`, which keeps
  `build:server:dev`.
- **Group leaves:** `build:cli:characterisation:dev` runs one `cargo build`
  over the launcher and every sub-binary, so its members compile in one
  parallel pass instead of queueing on the target lock.
- **Loopback seam:** a binary whose package declares `test-loopback`
  (`jira-cli`, `linear-cli`, `accelerator-research`) is always built with it
  in debug, in every leaf and group. Two leaves building one
  `target/debug/<bin>` with different features would overwrite each other.
  A compile guard already refuses the feature in a release build.
- **Existing suites:** each depends on the leaves for the binaries it runs:
  conformance on the launcher and `corpus`, hooks on the launcher, `vcs` and
  `research`, research on `research`, visualiser on the launcher.

### Success Criteria:

#### Automated Verification:

- [x] Suite passes against today's binaries: `mise run test:integration:characterisation`
- [x] No case reaches the network: every URL override points at the loopback server, and each golden records its request count
- [x] VCS parity: a committed `test_every_case_runs_in_git_and_jj` collection test asserts that every case not marked `vcs_specific` has both a `git` and a `jj` parameter id
- [x] Task wiring pinned: `uv run pytest tests/unit/tasks/test_mise.py`
- [x] `mise run` exits 0

#### Manual Verification:

- [ ] Each 0299 characterisation bullet maps to at least one case (checklist in Implementation Notes)
- [ ] Each rerouted symbol in the violations table is reached by a named case

---

## Phase 3: Role Declarations and the Dependency Lint

### Overview

Declare every crate's role and build the lint against fixtures, without
wiring it into `check`.

### Changes Required:

#### 1. Declarations

**File**: every `cli/**/Cargo.toml` member
**Changes**: add `[package.metadata.accelerator]` exactly as 0299's Expected
declarations table says. The table counts `consent-adapters` (still present
until Phase 6), which declares `role = "adapter"`, `context = "config"`.

```toml
[package.metadata.accelerator]
role = "domain"
context = "tracker"
kind = "shared"
downstreams = ["work", "jira", "linear"]
```

`jira-client` and `linear-client` each declare `kind = "product"`.

#### 2. Lint module

**File**: `tasks/lint/crate_dependencies.py`
**Changes**: modelled on `config_test_support.py`.

- **Domain model:**
  - `Role` enum: kernel, domain, adapter, technical-library,
    composition-root, launcher, test-support, bootstrap-verifier.
  - `Kind` enum: platform, shared, product.
  - `Crate(name, role, context)`, `Context(name, kind, downstreams)`,
    `Edge(source, target, kind)` with kind normal or build.
  - `Finding(crate, dependency, rule)` renders as
    `"{crate} -> {dependency}: {rule}"` (or `"{crate}: {rule}"`).
- `declarations(packages) -> (Workspace, list[Finding])` runs the
  declaration check.
- `violations(workspace, edges) -> list[Finding]` runs rules 1, 2, 4, 5 and
  6, the kernel, technical-library and verifier constraints, and the
  test-support check.
- Sources with role test-support or bootstrap-verifier are exempt.
  Dev edges are dropped on read.
- Rule 2 acyclicity: edges whose source and target share a context are
  dropped, then a strongly-connected-components pass over the platform
  context graph reports every component of two or more contexts.
- `workspace_packages()` moves to `tasks/shared/cargo_metadata.py`, and
  `config_test_support.py` imports it from there.
- `@task check` raises `Exit` with every finding.

#### 3. Tests

**File**: `tests/unit/tasks/test_crate_dependencies.py`
**Changes**: written first, one test per 0299 acceptance bullet, over
synthetic `cargo metadata`-shaped dicts (builders `_crate`, `_dep`):

- every forbidden fixture: rule 1 three ways, rule 2 product target, rule 2
  platform cycle of two and of three contexts, rule 4 normal and build, rule
  4 adapter → root, rule 5 three ways, rule 6 three ways, the kernel,
  technical-library and verifier constraints, and test-support normal and
  build;
- every permitted fixture from the second bullet, plus a platform adapter
  depending on its own domain (no cycle finding);
- shared context with a declared and an undeclared downstream;
- each declaration-check case from the fourth bullet;
- the `check` task over injected metadata with two violations raises `Exit`
  naming both;
- `test_the_shipped_declarations_match_the_expected_table`, a literal table
  keyed by Cargo package name, compared against `workspace_packages()`.

The lint does not yet pass the real tree. So rather than a "shipped
workspace is clean" test, Phase 3 adds
`test_the_shipped_workspace_findings_are_the_known_violations`. It pins the
finding set the lint computes over today's graph, and each later phase
shrinks it. Phase 12 replaces it with `== []`. The set is not 0299's table
verbatim, because two of its rows are rule 3 judgements outside the lint
(`work` → `config`, `collaboration` → `vcs`, Phase 11):

| Source | Target | Rule |
|---|---|---|
| `accelerator` | `tracker-support` | 6 |
| `consent-adapters`, `corpus-adapters`, `work-adapters` | `vcs-adapters` | 4 |
| `research-adapters`, `jira-client`, `linear-client` | `corpus-adapters` | 4 |
| `migrate-adapters` | `config-adapters`, `vcs-adapters`, `corpus-adapters` | 4 |
| `migrate-adapters` | `work-adapters` | 2 and 4, one finding each |
| `migrate` | `document` | 1 |

#### 4. Task wiring (not in `check`)

**File**: `tasks/lint/__init__.py`, `tasks/__init__.py`, `mise.toml`
**Changes**: register the module and add a leaf
`lint:crate-dependencies:check`. Neither roll-up references it yet.

### Success Criteria:

#### Automated Verification:

- [ ] Lint tests pass: `uv run pytest tests/unit/tasks/test_crate_dependencies.py`
- [ ] `config-test-support` still passes after the helper move: `uv run pytest tests/unit/tasks/test_config_test_support.py`
- [ ] `mise run lint:crate-dependencies:check` exits non-zero, naming exactly the known-violation table above
- [ ] `mise run build-system:check` and `mise run cli:check` pass
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] Finding text names the crate, dependency and rule number or check name

---

## Phase 4: Technical-Library Moves (Lock, `TEMP_PREFIX`)

### Overview

Move the file lock into `store` and `TEMP_PREFIX` into `kernel`. This removes
`jira-client`/`linear-client` → `corpus-adapters` and launcher → `store`.

### Changes Required:

#### 1. `kernel::TEMP_PREFIX`

**File**: `cli/kernel/src/lib.rs`, `cli/store/Cargo.toml`,
`cli/store/src/lib.rs`
**Changes**:

- Test-first in `kernel`: `TEMP_PREFIX == ".tmp-"`.
- `store` depends on `kernel` and uses `kernel::TEMP_PREFIX`. The crate doc
  is reworded so that it depends on `kernel` for naming constants only and
  still translates no errors into `kernel::Error`.
- `store::TEMP_PREFIX` is removed. Callers switch to `kernel::TEMP_PREFIX`:
  - `config-adapters/src/store.rs:14,605,815`
  - `work-cli/src/sync.rs:302,2135,2157`
  - `visualiser/server/src/orchestration/state.rs:100,111`
  - `launcher/src/launch/outbound/resolve/cache.rs:11`
- The launcher's private `.tmp-` constant (`resolve/tree/layout.rs:35`)
  becomes `kernel::TEMP_PREFIX`.
- The launcher drops `store` from `Cargo.toml`.

#### 2. Lock into `store`

**File**: `cli/store/src/lock.rs` (moved from
`cli/corpus-adapters/src/lock.rs`), `cli/store/Cargo.toml` (+`rand`)
**Changes**:

- The moved tests are rewritten red first against the new error, pinning the
  variant for each failure class:

  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  #[non_exhaustive]
  pub enum LockError {
      Timeout { path: String },
      NotWritable { path: String },
      Io { path: String, detail: String },
  }
  ```

  It mirrors `WriteError`: the same derives, `#[non_exhaustive]`, and
  hand-written `Display` and `std::error::Error`. `Display` reproduces
  `StoreError`'s three strings byte for byte, pinned by a test per arm.
- `acquire(lockdir, LockOptions) -> Result<LockGuard, LockError>`.
- A new public `pub fn holder_pid(lockdir) -> Option<u32>` replaces the
  duplicated sentinel reader in `migrate-adapters/src/run_lock.rs:80-96`.
  It keeps today's answers: `None` for an absent, duplicate or unparseable
  `owner.<nonce>`, and `None` when only a reclaiming or legacy nonce-less
  `owner` sentinel is present, so migrate's "pid unknown" refusal text does
  not change. A test pins each case.
- `corpus-adapters` maps all three arms into
  `StoreError::{LockTimeout, NotWritable, Io}` in `FileCorpusStore`, with a
  test per arm. `migrate-adapters/src/run_lock.rs` matches
  `LockError::Timeout`.
- `store`'s crate doc names `kernel` and `rand` among its dependencies.
- Callers move to `store::lock::{acquire, LockOptions}`:
  - `jira-client/src/cache.rs`, `linear-client/src/cache.rs` and their tests
  - `work-cli/src/{create,update,sync_author}.rs`
- `jira-client` and `linear-client` drop `corpus-adapters`.
- The `lock.rs` entry in `tasks/lint/store_duplication.py:27` is deleted
  (`cli/store/src/` is skipped).

### Success Criteria:

#### Automated Verification:

- [ ] Lock and constant tests pass: `cargo test --manifest-path cli/Cargo.toml -p store -p kernel`
- [ ] Dependents compile and pass: `cargo test --manifest-path cli/Cargo.toml -p jira-client -p linear-client -p accelerator-work -p migrate-adapters -p corpus-adapters -p accelerator`
- [ ] Characterisation suite unchanged, goldens untouched: `mise run test:integration:characterisation`
- [ ] Goldens untouched since the phase began: `jj diff --stat --from <phase base> tests/integration/characterisation/goldens` is empty
- [ ] Known-violation set shrinks by the two client edges: `uv run pytest tests/unit/tasks/test_crate_dependencies.py`
- [ ] `mise run public-api:check`, `mise run pup:check`, `mise run deny:check` pass
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] `cargo metadata` shows no `store` in the launcher's normal dependencies

---

## Phase 5: `vcs` Repository Ports

### Overview

Publish the tracking and repository-facts questions as `vcs` ports,
implemented in `vcs-adapters`. `corpus-adapters` and `work-adapters` consume
the facts port, injected at their roots.

### Changes Required:

#### 1. Ports in `vcs`

**File**: `cli/vcs/src/tracking.rs`, `cli/vcs/src/lib.rs`
**Changes**:

```rust
pub trait RepositoryTracking {
    fn file_tracking(&self, path: &Path) -> FileTracking;
    fn repository_roots(&self, directory: &Path) -> RootsAnswer;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkingCopyState {
    pub base_commits: Vec<String>,
    pub dirty_paths: Vec<String>,
}

pub trait WorkingCopyStateProbe {
    fn working_copy_state(
        &self,
        root: &Path,
        kind: VcsKind,
    ) -> Result<WorkingCopyState, kernel::Error>;
}

pub trait RepositoryProbe: WorkingCopyStateProbe {
    fn facts_at(&self, start: &Path) -> Option<RepoFacts>;
    fn user_name_at(&self, start: &Path) -> Option<String>;
}

impl<T> RepositoryProbe for T
where
    T: RepoRoot + VcsProbe + UserIdentityProbe + WorkingCopyStateProbe,
{
    fn facts_at(&self, start: &Path) -> Option<RepoFacts> { /* vcs::facts over self */ }
    fn user_name_at(&self, start: &Path) -> Option<String> { /* vcs::user_name over self */ }
}
```

`RepositoryProbe` is the start-relative facade consumers inject. The blanket
impl composes `vcs`'s existing fine-grained ports once, in the domain, so no
adapter re-implements the composition. The `_at` names keep its methods
distinct from `UserIdentityProbe::user_name` and the free `vcs::facts`.
`WorkingCopyState` moves from `vcs-adapters/src/library.rs:194-198` into the
domain. `dirty_paths` is derived from it by consumers. Blanket-impl tests use
stubs of the four fine-grained ports.

#### 2. In-process implementations

**File**: `cli/vcs-adapters/src/{tracking.rs,roots.rs,library.rs,lib.rs}`
**Changes**:

- `InProcessTracking` implements `RepositoryTracking` over the existing
  `file_tracking` and `repository_roots`.
- `InProcessProbe` implements `WorkingCopyStateProbe` by moving its inherent
  `working_copy_state` body into the impl, so one method carries the name.
  It gains `RepositoryProbe` through the blanket impl.
- Contract tests in `vcs-adapters/tests/` exercise the trait objects in git
  and jj:
  - `RepositoryTracking::file_tracking`: tracked, untracked, and corrupted
    index giving `Unknown` (the re-home of `vcs-cli/tests/tracking.rs:92-106`);
  - `RepositoryTracking::repository_roots`: a nested repository;
  - `RepositoryProbe`: `facts_at`, `user_name_at` and `working_copy_state`,
    inside and outside a repository.
- The free functions `file_tracking` and `repository_roots` stay `pub` while
  `consent-adapters` (Phase 6) and `vcs-cli`'s tracking subcommand (Phase 7)
  still call them. Phase 7 makes them private.

#### 3. `corpus-adapters` over the port

**File**: `cli/corpus-adapters/src/metadata.rs`, `Cargo.toml`
**Changes**:

- `VcsBackedRepoFactsProbe<'a> { facts: &'a dyn vcs::RepositoryProbe }`
  implements `corpus::RepoFactsProbe`. Unit tests use a stub
  `RepositoryProbe`.
- `corpus-adapters` drops `vcs-adapters` and depends on `vcs`.
- `corpus-cli/src/main.rs:87` and `work-cli/src/{create,sync_author}.rs`
  construct it over `&vcs_adapters::InProcessProbe`.

#### 4. `work-adapters` over the port

**File**: `cli/work-adapters/src/author.rs`,
`cli/work-adapters/src/sync/working_copy_status.rs`, `Cargo.toml`
**Changes**:

- `VcsBackedIdentityProbe` and `RepositoryIdentityProbe` hold a
  `&dyn RepositoryProbe` and call `user_name_at`. `vcs_user_at` and
  `current_vcs_user` take it as a parameter.
- `VcsWorkingCopyStatus::probed_from(start, &dyn RepositoryProbe)` uses
  `facts_at` then `working_copy_state`.
- `work-adapters` drops `vcs-adapters`.
- `work-cli` (`create.rs`, `sync.rs:1197`, `sync_author.rs`, `list.rs:683-688`)
  injects `InProcessProbe`. Tests are updated to stubs.

### Success Criteria:

#### Automated Verification:

- [ ] Port contract tests pass in git and jj: `cargo test --manifest-path cli/Cargo.toml -p vcs-adapters --all-features`
- [ ] Consumers pass: `cargo test --manifest-path cli/Cargo.toml -p vcs -p corpus-adapters -p work-adapters -p accelerator-corpus -p accelerator-work --all-features`
- [ ] Characterisation suite unchanged: `mise run test:integration:characterisation`
- [ ] Goldens untouched since the phase began: `jj diff --stat --from <phase base> tests/integration/characterisation/goldens` is empty
- [ ] Known-violation set loses `corpus-adapters`/`work-adapters` → `vcs-adapters`: `uv run pytest tests/unit/tasks/test_crate_dependencies.py`
- [ ] `mise run public-api:check`, `mise run pup:check` pass
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] None beyond review

---

## Phase 6: Merge `consent-adapters` into `config-adapters`

### Overview

`config-adapters` gains the consent wiring over `vcs::RepositoryTracking`. The
six non-launcher roots inject `InProcessTracking`, and the crate is deleted.

### Changes Required:

#### 1. Moved wiring

**File**: `cli/config-adapters/src/consent/{tracking.rs,roots.rs,credentials.rs}`,
`cli/config-adapters/Cargo.toml` (+`vcs`)
**Changes**: moved tests are adapted red first to take a stub
`RepositoryTracking`.

```rust
pub struct TrackedConfigFile<T>(pub T);

impl<T: RepositoryTracking> ConfigFileTracking for TrackedConfigFile<T> {
    fn tracking(&self, path: &Path) -> Tracking {
        translated(self.0.file_tracking(path))
    }
}

pub fn repository_roots(
    tracking: &dyn RepositoryTracking,
    config_root: &Path,
    cwd: &Path,
) -> RepositoryRoots;

pub fn credential_ports(
    tracking: impl RepositoryTracking + 'static,
    config_root: &Path,
    cwd: &Path,
) -> CredentialPorts;
```

- `translated` and `beside_config_root` stay on the `config-adapters` side.
- `command_runner` becomes private to the module.
- `TrackedConfigFile` owns its tracker, so `CredentialPorts.tracking` stays a
  `Box<dyn ConfigFileTracking>` with no borrowed lifetime, and tests can pass
  stateful stubs.
- `config-adapters` depends on `vcs` (kernel-only) and never on
  `vcs-adapters`, so the visualiser's closure is unchanged.

#### 2. Roots

**File**:

- `collaboration-cli/src/main.rs:112`
- `jira-cli/src/context.rs:143`
- `linear-cli/src/context.rs:128`
- `research-cli/src/main.rs:138-141`
- `work-cli/src/tracker_registry.rs:185`
- `design-cli/src/config.rs:59-76,264-280`

**Changes**:

- Each call site uses
  `config_adapters::credential_ports(vcs_adapters::InProcessTracking, ...)`.
- `design-cli` builds `TrackedConfigFile(InProcessTracking)` and
  `repository_roots(&InProcessTracking, ...)`, and adds `vcs`.
- Each root drops `consent-adapters`.

#### 3. Deletion and records

**File**:

- `cli/consent-adapters/`
- `cli/Cargo.toml` (members), `Cargo.lock`
- `cli/pup.ron` (`consent_adapters_is_zero_spawn`, the
  `only_the_consent_policy_names_the_command_runner` comment)
- `tests/integration/pup/test_import_rule.py:947-967`
- `tasks/public_api.py:18`
- `tasks/README.md:761-764` and the crate's licence-audit, `cargo-deny` and
  public-API registrations
- the Phase 3 expected-declarations table

**Changes**: delete each. The command-runner pup rule now names
`config_adapters::consent` as its permitted holder.

### Success Criteria:

#### Automated Verification:

- [ ] `cargo test --manifest-path cli/Cargo.toml -p config-adapters -p accelerator-collaboration -p jira-cli -p linear-cli -p accelerator-research -p accelerator-work -p accelerator-design --all-features` passes
- [ ] Characterisation consent cases unchanged: `mise run test:integration:characterisation`
- [ ] Goldens untouched since the phase began: `jj diff --stat --from <phase base> tests/integration/characterisation/goldens` is empty
- [ ] `cargo metadata --manifest-path cli/Cargo.toml --no-deps --format-version 1 | jq -e '[.packages[].name] | index("consent-adapters") | not'`
- [ ] Known-violation set loses `consent-adapters` and the declaration table matches: `uv run pytest tests/unit/tasks/test_crate_dependencies.py`
- [ ] `mise run pup:check`, `mise run public-api:check`, `mise run deny:check`, `uv run pytest tests/integration/pup` pass
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] `cargo tree -p accelerator-visualiser -e normal | grep -E 'gix|jj-lib'` is empty

---

## Phase 7: In-Process Tracking in the Launcher

### Overview

The launcher injects `TrackedConfigFile(InProcessTracking)`. The dispatched
tracking path and its vocabulary are deleted.

### Changes Required:

#### 1. Launcher wiring

**File**: `cli/launcher/src/main.rs`, `cli/launcher/Cargo.toml`
**Changes**:

- `compose_stack` passes
  `Box::new(TrackedConfigFile(vcs_adapters::InProcessTracking))` to
  `compose_stack_with`. The launcher adds `vcs` and `vcs-adapters`.
- Deleted from the launcher:
  - `tracking_resolver`, `FetchBudget` (only `Dispatch` would remain, so it
    collapses), and the pin test `main.rs:787-790`;
  - `a_vcs_binary_that_cannot_spawn_answers_unknown` and the
    unreachable-binary summary test (`main.rs:798-875`);
  - the `rustix` `event` feature.
- Re-check `multiple-versions = "deny"` against `gix`'s `signal-hook`
  (plan 0226).
- `tests/integration/deny/test_vcs_library_graph.py`'s gix feature-absence
  assertions are parametrised over `-p accelerator` as well as
  `-p vcs-adapters`, red first against a deliberately enabled feature.

#### 2. Panic boundary

**File**: `cli/vcs-adapters/src/{panic_fold.rs,tracking.rs}`,
`cli/vcs-cli/src/report.rs`
**Changes**: the fold sits at the `vcs-adapters` edge, where gix and jj-lib
panics originate, beside the boundary `report.rs` already has for the same
libraries.

- `panic_message` moves from `vcs-cli/src/report.rs:37` into `vcs-adapters`,
  and `report.rs` imports it.
- Test-first with a panicking stub `RepositoryTracking`: a
  `PanicFold<T: RepositoryTracking>` decorator answers
  `FileTracking::Unknown` for `file_tracking`, and
  `RootsAnswer { roots: vec![], complete: false }` for `repository_roots`.
  Both fail closed. Each fold emits
  `warn!(adapter, panic = panic_message(..), ..)`, as `report.rs` does, so
  `ACCELERATOR_LOG` can diagnose it.
- `InProcessTracking` delegates through `PanicFold`, so every in-process
  caller is covered, not only the consent path.
- Both rely on `panic = "unwind"`, as `report.rs` already does. The default
  panic hook's message still reaches stderr when a fold happens. That is
  accepted: stdout stays one JSON line, and a fold is a defect worth seeing.
- The process-level isolation the dispatched child gave is replaced by this
  fold for panics. Hangs remain unbounded (0299 Assumptions).

#### 3. Removals

**File**:

- `cli/launcher/src/launch/outbound/{capture.rs,tracking.rs,mod.rs}`
- `cli/launcher/src/launch/core.rs:312-340`
- `cli/launcher/src/config_command/core/summary.rs:95-99`, plus the
  `an_unchecked_personal_file_is_a_context_note_alone` test and
  `SummaryWarnings::context_notes` with its render and CLI consumers if it
  has no other producer
- `cli/config/src/consent.rs` (`TrackingCheck`, `ConfigFileTracking::check`,
  `AuditFinding::PersonalFileUnchecked`), plus `config/tests/consent.rs`
  `FixedTracking.unchecked` and the two `Unchecked` tests
- `cli/kernel/src/tracking.rs` and `lib.rs:9,11`
- `cli/vcs-cli/src/{cli.rs:55-60,main.rs:11,52-53,115,tracking.rs}` and
  `cli/vcs-cli/tests/tracking.rs`
- `cli/launcher/tests/fixtures/accelerator_fixture.rs` (`tracking` branch and
  `TRACKING_ANSWER`)
- `cli/launcher/tests/config_read.rs` (`TRACKING_ANSWER`, `summary_hook`'s
  answer parameter, and the fail, hang and missing-binary tests)
- `tasks/lint/skill_cli_refs.py:26-28` (`"tracking"`)

**Changes**:

- Test-first replacement: `config_read.rs` gains real-repository summary
  tests using `vcs-test-support::Hermetic`. A tracked file in git, jj and
  colocated jj warns in both hook fields. Every case asserts stdout is
  exactly one JSON line and stderr is empty with `ACCELERATOR_LOG` unset.
- `audit` becomes
  `findings.extend(context.tracking.tracking(path).distrust().map(AuditFinding::PersonalFile))`.
- `vcs_adapters::file_tracking` and `repository_roots` become private behind
  `InProcessTracking`. `vcs-adapters/tests/{file_tracking,roots}.rs` are
  folded into the Phase 5 contract tests, which reach the same cases through
  the trait objects.

#### 4. Documentation

**File**:

- `docs-site/src/content/docs/configuration.md:129-136`
- `tasks/README.md:667-700`
- `cli/pup.ron:434-450` comment

**Changes**:

- `configuration.md`: SessionStart checks tracking in-process; drop the
  "noted as skipped" and `ACCELERATOR_VCS_BIN` session-start text.
- `tasks/README.md`: rewrite the worked example around a surviving
  subcommand, e.g. `vcs guard`.
- `pup.ron`: refresh the `vcs_cli_is_free_of_config` comment.

#### 5. Characterisation golden diff

The four "no `accelerator-vcs` available" summary goldens (tracked and
untracked, git and jj) lose the "was not checked" note. The tracked ones gain
the tracked-file warning; the untracked ones carry nothing. Regenerate those four alone and review
the diff. No other golden changes.

### Success Criteria:

#### Automated Verification:

- [ ] `cargo test --manifest-path cli/Cargo.toml -p accelerator -p config -p config-adapters -p kernel -p accelerator-vcs -p vcs-adapters --all-features` passes
- [ ] Launcher gix features guarded: `uv run pytest tests/integration/deny`
- [ ] `accelerator vcs tracking --path x` exits non-zero as an unknown subcommand (characterisation case added this phase)
- [ ] `rg -n 'TrackingCheck|PersonalFileUnchecked|TrackingAnswer|was not checked' cli/` is empty
- [ ] Removal files absent: `test ! -e cli/launcher/src/launch/outbound/capture.rs && test ! -e cli/kernel/src/tracking.rs && test ! -e cli/vcs-cli/src/tracking.rs && test ! -e cli/vcs-cli/tests/tracking.rs && test ! -e cli/launcher/src/launch/outbound/tracking.rs`
- [ ] `uv run pytest tests/unit/tasks/test_skill_cli_refs.py` passes
- [ ] `mise run test:integration:characterisation` passes, and `jj diff --stat --from <phase base> tests/integration/characterisation/goldens` lists exactly the four reviewed goldens, with no change under `tests/integration/characterisation/*.py`
- [ ] `mise run deny:check`, `mise run public-api:check`, `mise run docs:check` pass
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] A fresh Claude Code session in a repository with a tracked `config.local.md` shows the warning, with an empty launcher cache

---

## Phase 8: `corpus` Parsing Port, Validation Pipeline and Canonicaliser Port

### Overview

The `corpus` domain owns frontmatter classification types, the validation
pipeline and two new ports. `research-adapters` switches and drops
`corpus-adapters`.

### Changes Required:

#### 1. Ports and types in `corpus`

**File**: `cli/corpus/src/frontmatter.rs` (new), `cli/corpus/src/work_item_id.rs`
**Changes**:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum FrontmatterState { Parsed(Mapping), Absent, Malformed }

#[derive(Debug, Clone)]
pub struct ParsedDocument { pub state: FrontmatterState, pub body: String }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontmatterError(pub String);

pub trait FrontmatterParser {
    fn classify(&self, raw: &[u8]) -> ParsedDocument;
    fn parse_value(&self, content: &str)
        -> Result<FrontmatterValue, FrontmatterError>;
    fn split_frontmatter(&self, content: &str)
        -> Result<String, FrontmatterError>;
}

pub trait WorkItemIdCanonicaliser {
    fn canonicalise(
        &self,
        input: &str,
        pattern: &str,
        key_value: &str,
    ) -> Result<String, CanonicaliseError>;
}
```

The three parsing operations have distinct, exact contracts:

- `classify`: today's `corpus_adapters::document::parse`, unchanged. A
  non-mapping root is `Malformed`; a null or empty root is `Parsed({})`;
  unfenced content is `Absent`.
- `parse_value`: `document::parse` mapped through `to_value`. Unfenced or
  empty frontmatter is an empty mapping, and any non-tagged YAML root is a
  value. Errors carry `DocumentError`'s `Display` text verbatim.
- `split_frontmatter`: raw `document::split(..).frontmatter`, whatever the
  root. Unfenced content is the empty string, and an unterminated fence is an
  error with `DocumentError`'s text.

`CanonicaliseError` mirrors the `PatternError` arms `canonicalise_id`
returns.

#### 2. Adapters

**File**: `cli/corpus-adapters/src/{document.rs,work_item_pattern.rs,lib.rs}`
**Changes**:

- `YamlFrontmatter` implements `FrontmatterParser` over the existing
  `classify`, `document::parse` with `to_value`, and `document::split`.
  `corpus-adapters` re-exports `FrontmatterState` and `ParsedDocument` from
  `corpus`, and keeps `parse`, for `assemble.rs` and the visualiser server.
  The visualiser's `frontmatter_state` classification does not change.
- A `corpus-adapters` contract test pins each operation over absent,
  empty, null-root, mapping, sequence-root, scalar-root, unterminated-fence,
  invalid-YAML and tagged inputs.
- `PatternCanonicaliser` implements `WorkItemIdCanonicaliser` over
  `canonicalise_id`, with a test per `PatternError` arm.

#### 3. Pipeline move

**File**: `cli/corpus/src/frontmatter_validation/pipeline.rs` (moved from
`corpus-adapters/src/frontmatter_validation.rs`)
**Changes**:

- `build_index`, `validate_path`, `validate_text` and `validate_targets` gain
  a `&dyn FrontmatterParser` parameter. Their `W: CorpusWalker + FileReader`
  bound becomes two parameters, `walker: &dyn CorpusWalker` and
  `reader: &dyn FileReader`, so a context holding separate trait objects can
  call them. `corpus-cli` passes `&RealFs` for both.
- `parsed_frontmatter_text` keeps its gating: `classify` first, and
  `split_frontmatter` only for `Parsed` content, with split errors folded to
  `None` as today.
- `corpus_files`, `validate_templates`, `Checks` and `TargetOutcome` move
  unchanged.
- The moved unit tests use a stub parser. The YAML-dependent cases
  (`frontmatter_validation.rs:400-411`) move to a `corpus-adapters`
  integration test that composes `YamlFrontmatter` with the domain pipeline.
- The module doc at `corpus/src/frontmatter_validation/mod.rs:17-21` is
  rewritten.
- `corpus-cli/src/frontmatter.rs` injects `&YamlFrontmatter`.

#### 4. `research-adapters`

**File**: `cli/research-adapters/src/topic.rs`, `src/conduct/ledger.rs`,
`Cargo.toml`
**Changes**:

- `read_round_inputs`, `available_profiles` and the topic functions take a
  `&dyn FrontmatterParser`. `frontmatter()` uses `parser.classify`.
- Ledger tests swap `FileCorpusStore` for an in-memory `AtomicWrite` and
  `FileRemove` double.
- `research-adapters` drops `corpus-adapters`, and `research-cli` injects
  `&YamlFrontmatter`.

### Success Criteria:

#### Automated Verification:

- [ ] `cargo test --manifest-path cli/Cargo.toml -p corpus -p corpus-adapters -p accelerator-corpus -p research-adapters -p accelerator-research -p accelerator-visualiser --all-features` passes
- [ ] Frontmatter-validate and research characterisation cases unchanged, goldens untouched: `mise run test:integration:characterisation`
- [ ] Goldens untouched since the phase began: `jj diff --stat --from <phase base> tests/integration/characterisation/goldens` is empty
- [ ] Known-violation set loses `research-adapters` → `corpus-adapters`: `uv run pytest tests/unit/tasks/test_crate_dependencies.py`
- [ ] `mise run pup:check` passes with `corpus`'s allow-list unchanged
- [ ] `mise run public-api:check` passes
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] None beyond review

---

## Phase 9: `migrate` and `migrate-adapters` Through Ports

### Overview

`migrate` drops `document`. `migrate-adapters` drops `config-adapters`,
`vcs-adapters`, `corpus-adapters` and `work-adapters`, and `migrate-cli`
wires every capability.

### Changes Required:

#### 1. `migrate` domain

**File**: `cli/migrate/src/ports.rs`, `cli/migrate/src/migrations/{m0001.rs,m0008.rs}`,
`cli/migrate/Cargo.toml`
**Changes**: test-first, each migration's existing unit test double gains the
new capabilities.

- **`MigrationContext` additions**: operations, not collaborators, each with
  a default body returning
  `MigrationError("... is not implemented by this context")` per the
  existing pattern. Each error's message is `DocumentError`'s `Display` text.

  ```rust
  fn parse_frontmatter(&self, content: &str)
      -> Result<corpus::FrontmatterValue, MigrationError>;
  fn frontmatter_text(&self, content: &str) -> Result<String, MigrationError>;
  fn render_canonical(&self, content: &str) -> Result<String, MigrationError>;
  ```

  `parse_frontmatter` and `frontmatter_text` have
  `FrontmatterParser::parse_value`'s and `split_frontmatter`'s contracts
  (Phase 8); `render_canonical` is `document::render` over `document::parse`.
- **m0001**: the check becomes `ctx.parse_frontmatter(&content).is_err()`.
  It refuses exactly what `document::parse` refuses today, so a sequence or
  scalar root still passes.
- **m0008**: the logic stays in the domain, unchanged in shape:
  - `render_canonical(original)` becomes `ctx.render_canonical(original)`;
  - the guard compares `ctx.parse_frontmatter(original)?` with
    `ctx.parse_frontmatter(&rendered)?`. An unfenced original and its
    `{}`-fenced rendering both give an empty mapping, so the guard passes as
    today (Key Discoveries);
  - `document::split` in `canonicalise` and `detect_loss` becomes
    `ctx.frontmatter_text(..)`;
  - each failure keeps today's `at(path, &error.to_string())` text.
- m0008's in-crate unit tests keep only control flow over a stub context
  (abort writes nothing, unchanged files are skipped, loss is reported). The
  YAML-dependent cases move to `migrate-adapters/tests/m0008.rs`, which runs
  `Migration0008` against `FileMigrationContext` composed with
  `YamlFrontmatter`. These cases are `a_value_retyping_re_render_aborts_and_writes_nothing`,
  `re_rendering_is_a_byte_level_fixed_point`,
  `a_block_linkage_sequence_with_colons_reflows_to_quoted_flow` and the
  `detect_loss` cases.
- `SessionLogFactory::for_migration` returns `Box<dyn SessionLog + '_>`, so
  a factory can lend its injected collaborators to the logs it builds.
  Existing test doubles compile unchanged under elision.
- `migrate` drops `document`, and the pup rule drops `^document`.

#### 2. `migrate` sync-baseline port

**File**: `cli/migrate/src/ports.rs`
**Changes**: a `SyncBaselines` port replaces the loop in
`realign_sync_baseline`. `FileMigrationContext` keeps resolving
`paths.integrations` (relative or absolute) and its early `Ok(0)` returns,
then calls the port with the resolved root.

```rust
pub trait SyncBaselines {
    fn realign(
        &self,
        integrations_root: &Path,
        pre_migration: &[(PathBuf, String)],
    ) -> Result<usize, MigrationError>;
}
```

#### 3. `migrate-adapters`

**File**: `cli/migrate-adapters/src/{context.rs,corpus_index.rs,working_copy.rs,session_log_factory.rs,run_lock.rs,lib.rs}`
**Changes**:

- `FileMigrationContext::new` takes the injected capabilities:

  ```rust
  pub struct Capabilities<'a> {
      pub config: &'a dyn config::ConfigAccess,
      pub walker: &'a dyn corpus::scan::CorpusWalker,
      pub reader: &'a dyn corpus::scan::FileReader,
      pub frontmatter: &'a dyn corpus::FrontmatterParser,
      pub canonicaliser: &'a dyn corpus::WorkItemIdCanonicaliser,
      pub sync_baselines: &'a dyn migrate::ports::SyncBaselines,
  }
  ```

  `config::paths::doc_type_dirs` and `effective` read through `config`.
  `report_ignored_personal_file` and `require_readable_personal_file` move to
  `migrate-cli`, which owns the `Composed`.
- `FileMigrationContext` implements `parse_frontmatter` and
  `frontmatter_text` by delegating to the injected parser, and
  `render_canonical` directly over `document::{parse, render}`. That
  satisfies 0299 requirement 8's "implemented in `migrate-adapters` over
  `document`". `migrate-adapters` gains the `document` technical library.
  The two routes meet in one place: `YamlFrontmatter::parse_value` is
  `document::parse` mapped through `to_value`, which lives in
  `corpus-adapters` and so cannot be called here directly. The
  `migrate-adapters/tests/m0008.rs` composition pins that both routes agree.
- `validate_frontmatter` calls the `corpus` pipeline with the injected
  walker, reader and parser. `corpus_index.rs` uses the injected walker and
  reader.
- `VcsWorkingCopy` holds a `&dyn vcs::RepositoryProbe` and calls
  `working_copy_state`.
- `session_log_factory.rs`:
  - takes `&dyn corpus::RecordStore` and `&dyn corpus::Clock`, and returns
    logs that borrow them;
  - `records()` reads through a new `RecordStore::read_records(path) -> Result<Vec<Record>, StoreError>`,
    implemented in `FileCorpusStore` over `jsonl::parse_record`. An absent
    file is an empty vector and empty lines are skipped. An I/O failure is
    `StoreError::Io { detail }` with the `io::Error` text, and a UTF-8 or
    record-parse failure is `StoreError::Validation { detail }` with that
    error's text;
  - the factory maps `Io { detail, .. }` and `Validation { detail }` to
    `MigrationError::new(detail)`, dropping `StoreError`'s prefixes, so each
    message matches today's. Red tests pin each arm's exact text;
  - `check_schema_version` stays in the factory.
- `session_log.rs`'s `FileSessionLogRewriter` and the `SessionLogRewriter`
  port are deleted, since no root constructs them.
- `migrate-adapters` drops `config-adapters`, `vcs-adapters`,
  `corpus-adapters` and `work-adapters`.

#### 4. `migrate-cli` composition

**File**: `cli/migrate-cli/src/main.rs`, `cli/migrate-cli/src/sync_baselines.rs`
(new), `Cargo.toml`
**Changes**:

- `migrate-cli` builds the following and passes them to `FileMigrationContext`
  and the session-log factory:
  - `Composed::over(FileConfigStore::at(&root).with_legacy_policy(LegacyPolicy::Allow))`;
  - `RealFs`, `YamlFrontmatter` and `PatternCanonicaliser`;
  - `FileCorpusStore`, `SystemClock::with_offset(UtcOffset::UTC)` and
    `InProcessProbe`.
- The loop of `context.rs:297-413` and `realign_one_baseline` move to
  `work-adapters`, with their unit tests, so the work context owns what
  realigning a baseline means:

  ```rust
  pub fn realign_baselines(
      integrations_root: &Path,
      pre_migration: &[(PathBuf, String)],
      reader: &dyn FileReader,
      writer_at: &dyn Fn(&Path) -> Box<dyn AtomicWrite>,
  ) -> Result<usize, RealignError>;

  pub enum RealignError {
      Listing(std::io::Error),
      Baseline(corpus::StoreError),
  }
  ```

  `RealignError`'s `Display` is the wrapped error's own text, which is
  exactly what `MigrationError::new(error.to_string())` carries today for a
  directory-listing failure and a baseline load or save failure.

  `writer_at` builds a writer rooted at each baseline's own directory, as
  today's per-integration `FileCorpusStore::new(parent)` does. That keeps an
  absolute `paths.integrations` outside the repository writable, and it
  keeps `work-adapters` free of `corpus-adapters`.
- `WorkSyncBaselines` in `migrate-cli` implements `SyncBaselines` by calling
  `realign_baselines` with `&RealFs` and
  `&|dir| Box::new(FileCorpusStore::new(dir))`, and maps `RealignError` into
  `MigrationError` with today's text. A `migrate-cli` unit test pins that
  mapping over an unreadable and a malformed `last-sync.json`. Only
  `migrate-cli` names both `work` and `migrate`.
- `migrate-cli` depends on `corpus-adapters`, `vcs-adapters`, `work-adapters`,
  `config` and `corpus`; `work-adapters` moves from dev to normal.

### Success Criteria:

#### Automated Verification:

- [ ] `cargo test --manifest-path cli/Cargo.toml -p migrate -p migrate-adapters -p accelerator-migrate -p corpus-adapters -p work-adapters --all-features` passes
- [ ] Migration characterisation cases byte-identical, goldens untouched: `mise run test:integration:characterisation`
- [ ] Goldens untouched since the phase began: `jj diff --stat --from <phase base> tests/integration/characterisation/goldens` is empty
- [ ] Known-violation set loses `migrate` → `document` and the five `migrate-adapters` findings: `uv run pytest tests/unit/tasks/test_crate_dependencies.py`
- [ ] `mise run pup:check`, `mise run public-api:check` pass
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] `/accelerator:migrate` on a scratch copy of an old-schema repository applies cleanly

---

## Phase 10: Catalogue Describes Tracker Blocks

### Overview

`config::catalogue` describes the `<tracker>.pull` and `<tracker>.push`
blocks, and a structural validator in `config` runs off that description.
The launcher's dump uses it and drops `tracker-support`. `tracker-support`
derives its parsers from it.

### Changes Required:

#### 1. Block description in `config`

**File**: `cli/config/src/catalogue.rs`, `cli/config/src/tracker_block.rs` (new)
**Changes**: test-first against the existing `pull.rs`/`push.rs` cases,
ported to `config`.

```rust
pub enum FieldKind {
    EntityList,
    ScopeFlag,
    Ceiling { allow_zero: bool },
    PageCaps { sub_keys: &'static [&'static str] },
    Filters { accepted: &'static [&'static str], reserved: &'static [&'static str] },
}

pub struct Field { pub name: &'static str, pub kind: FieldKind }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockName { Pull, Push }

pub struct TrackerBlock {
    pub scope: &'static str,
    pub label: &'static str,
    pub name: BlockName,
    pub fields: &'static [Field],
    pub mutually_exclusive: &'static [(&'static str, &'static str)],
}

#[derive(Clone, Copy)]
pub struct TrackerCatalogue(pub &'static [TrackerBlock]);

impl TrackerCatalogue {
    pub fn block(self, scope: &str, name: BlockName) -> Option<&'static TrackerBlock>;
    pub fn validate(
        self,
        scope: &str,
        name: BlockName,
        value: &Value,
    ) -> Result<(), BlockError>;
}

pub const TRACKERS: TrackerCatalogue =
    TrackerCatalogue(&[/* jira.pull, linear.pull, jira.push, linear.push */]);
```

The catalogue is a value, so validation looks a block up by
`(scope, BlockName)` in the same table it scans for wrong-noun hints.
Membership holds by construction, and one `'static` lifetime applies
throughout. Tests build their own `TrackerCatalogue` from a `static` table.
`label` is the tracker's display name ("Jira", "Linear") that the wrong-noun
hint prints.

`tracker_block.rs` holds:

- `read_block`, moved from `tracker-support/src/block.rs`;
- `BlockError` with `detail(Level)`, carrying today's exact text from
  `pull.rs:106-157` and `push.rs:60-62`;
- `TrackerCatalogue::validate`, which keeps today's first-fault order, parse
  stage included:
  1. the block is not a mapping;
  2. `filters` is not a mapping;
  3. wrong-tracker noun with hint;
  4. the first unknown key in block order, with top-level and
     `max_pages.<sub>` keys interleaved as they appear;
  5. exclusivity;
  6. filters field by field in block order, each checked for reserved and
     then unsupported before the next field;
  7. ceilings: a positive integer, zero where allowed, or `unlimited`.

Wrong-noun hints come from scanning the other blocks' `EntityList` and
`ScopeFlag` fields and their blocks' `label`s, replacing `NOUN_OWNERS`. A
unit test pins the hint text ("`all_teams` is a Linear key; this integration
is Jira — did you mean `all_projects`?"). A test pins that every block has
exactly one `EntityList` and one `ScopeFlag`, the invariant the pairing relies
on. Field order is the dump and "accepted:" order.

`config` owns the structural syntax of tracker blocks, so per-tracker data
sits in the platform catalogue. That is the accepted cost of the
catalogue-driven approach. `tracker-support` owns their meaning: ceiling
interpretation, defaults and folding.

#### 2. `tracker-support` derives from it

**File**: `cli/tracker-support/src/{pull.rs,push.rs,block.rs}`
**Changes**:

- `parse_with(catalogue, scope, value)` dispatches on `FieldKind` over the
  block's fields; `parse` binds `TRACKERS`. `_with` follows the codebase's
  name for an injectable variant (`compose_stack_with`, `resolve_with`). Known fields fold into
  `PullConfig`'s typed fields as today. A field the code has no typed slot
  for lands in `PullConfig::extensions: Vec<(&'static str, Value)>`, in
  field order.
- `validate` delegates structure to `config::tracker_block::validate` and
  wraps it as one variant, `PullConfigError::Structure(BlockError)` (and the
  same on `PushConfigError`). Its `Display` and `detail` delegate to
  `BlockError`, so refusal text lives in one place. Only semantic variants,
  such as ceiling resolution, stay in `tracker-support`.
- `Tracker::{accepted_top_level_keys, noun}`, `NOUN_OWNERS`, the hard-coded
  `"(accepted: max_items)"` and `block.rs` go.
- `JIRA_FILTERS` and `LINEAR_FILTERS` (`pull.rs:264-269`) are derived from
  the catalogue's `FieldKind::Filters`, so the filter lists exist once.
- Kept here: `CeilingToken` to `Ceiling`, `DEFAULT_MAX_ITEMS`/`DEFAULT_MAX_PAGES`,
  page-cap precedence, `resolve_ceilings`, `PullConfig` folding.
- `work-cli/src/sync.rs:742-774` collapses to `pull::read` plus `validate`,
  matching `validate_push_config`.

#### 3. Launcher dump

**File**: `cli/launcher/src/config_command/core/dump.rs`, `cli/launcher/Cargo.toml`
**Changes**:

- `pull_fields`, the push placeholder and `block_level` go.
- The dump's core takes a `TrackerCatalogue`; the imperative shell binds
  `TRACKERS`. Placeholders come from the block's `fields`. Validation is
  `TrackerCatalogue::validate` mapped to
  `ConfigError::Invalid { detail: error.detail(level) }`.
- No pull or push field name remains as a string literal.
- The launcher drops `tracker-support`.

#### 4. Catalogue-extension tests

**File**: `cli/config/tests/tracker_block.rs`,
`cli/tracker-support/tests/derived.rs`,
`cli/launcher/src/config_command/core/dump.rs` (unit tests)
**Changes**: a test-only catalogue holding an extra field in `pull`, and
separately in `push`, drives all three consumers through a test
`TrackerCatalogue`: its own `block`/`validate`, `parse_with`, and the dump
core's catalogue parameter:

- set valid: the dump prints it and the parser returns it in `extensions`;
- set invalid: the validator, the parser and the dump each report it as
  structurally invalid.

### Success Criteria:

#### Automated Verification:

- [ ] `cargo test --manifest-path cli/Cargo.toml -p config -p tracker-support -p accelerator -p accelerator-work -p jira-cli -p linear-cli -p linear-client --all-features` passes
- [ ] Dump and tracker-block characterisation cases unchanged, multi-fault blocks included, goldens untouched: `mise run test:integration:characterisation`
- [ ] Goldens untouched since the phase began: `jj diff --stat --from <phase base> tests/integration/characterisation/goldens` is empty
- [ ] `rg -n '"(additional_|all_|filters|max_items|max_pages)' cli/launcher/src/config_command/core/dump.rs` is empty
- [ ] Known-violation set loses launcher → `tracker-support`: `uv run pytest tests/unit/tasks/test_crate_dependencies.py`
- [ ] `mise run pup:check`, `mise run public-api:check` pass
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] `accelerator config` in a repository with a real Jira integration renders as before

---

## Phase 11: Domain → Domain Dependencies (Rule 3)

### Overview

Remove the dead `work` → `config` edge and `collaboration` → `vcs`. Record
the two justified edges.

### Changes Required:

#### 1. `work` → `config`

**File**: `cli/work/Cargo.toml:13`, `cli/pup.ron:163`
**Changes**: delete both.

#### 2. `collaboration` → `vcs`

**File**: `cli/collaboration/src/{lib.rs,base_repo.rs,update_body.rs}`,
`cli/collaboration/Cargo.toml`, `cli/collaboration-cli/src/main.rs`
**Changes**:

- `collaboration` declares its own port, named for its role rather than
  after `vcs::OriginRemote`, with today's signature:

  ```rust
  pub trait RepositoryOrigin {
      fn origin_url(&self, root: &Path) -> Result<Option<String>, kernel::Error>;
  }
  ```

  Domain tests use stubs. The existing `StubOriginRemote(Err(..))` error-path
  tests (`collaboration/src/lib.rs:218`) move across unchanged in behaviour.
- `collaboration-cli` adds `struct VcsOrigin<'a>(&'a dyn vcs::OriginRemote)`,
  implementing `collaboration::RepositoryOrigin` by one-to-one delegation over
  `InProcessProbe`.
- `collaboration` drops `vcs`, and its pup allow-list drops `^vcs`.

#### 3. Recorded justifications

Under Implementation Notes:

- **`work` → `corpus`**: justified. `work`'s public model is expressed in
  `corpus::WorkItemIdScheme` and the `corpus::IdScanner` port
  (`filter.rs`, `next_number.rs`, `resolve.rs`).
- **`work` → `tracker`**: justified. `work`'s sync model is expressed in
  `tracker::ExternalId` and `tracker::RemoteTimestamp`
  (`sync/classify.rs`, `sync/plan.rs`, `sync/push_precondition.rs`).
- **`collaboration` → `vcs`**: removed.

### Success Criteria:

#### Automated Verification:

- [ ] `cargo test --manifest-path cli/Cargo.toml -p work -p collaboration -p accelerator-collaboration --all-features` passes
- [ ] Collaboration characterisation cases, origin present and absent included, unchanged and goldens untouched: `mise run test:integration:characterisation`
- [ ] Goldens untouched since the phase began: `jj diff --stat --from <phase base> tests/integration/characterisation/goldens` is empty
- [ ] `cargo metadata` shows neither edge; `rg -n 'config' cli/pup.ron` shows no `work` permission on `config`
- [ ] `mise run pup:check`, `mise run public-api:check` pass
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] Rule 3 outcomes recorded with upstream types named

---

## Phase 12: Wire the Lint into `check`

### Overview

The workspace now passes the lint, so it becomes a CI gate. The registration
checklists gain the declaration step.

### Changes Required:

#### 1. Gate

**File**: `mise.toml` (`cli:check`, `lint:check`),
`tests/unit/tasks/test_mise.py`,
`tests/unit/tasks/test_crate_dependencies.py`, `tasks/README.md:34-44`
**Changes**:

- `lint:crate-dependencies:check` joins both roll-ups.
- `_CLI_CHECK_GATES` gains it, and the missing
  `lint:config-test-support:check`.
- The known-violations test becomes `test_the_shipped_workspace_is_clean`
  (`== []`).
- The README roll-up prose lists every Python guard in `cli:check`.

#### 2. Checklists

**File**: `tasks/README.md` (sub-binary checklist, library checklist),
`tests/unit/tasks/test_registration_docs.py`, `tasks/CLAUDE.md:12`,
`CLAUDE.md`, `tasks/public_api.py:78`
**Changes**:

- **Sub-binary checklist**: a fourteenth point, "14. **Add** the crate's
  `[package.metadata.accelerator]` declaration, and its row in
  `tests/unit/tasks/test_crate_dependencies.py`'s expected-declarations
  table **[PR]**", naming role `composition-root` and linking the lint. Only
  the leading verb is bold, like its siblings.
- **Library checklist**: a sixth bullet, the same declaration and table row,
  with role and context and kind where the role carries one.
- Every "thirteen" becomes "fourteen": `test_the_checklist_has_thirteen_points`
  is renamed, its line 118 comment and assertions (`:110,114,123`) updated,
  and `tasks/public_api.py:78`, `tasks/CLAUDE.md` and the root `CLAUDE.md`
  follow. The library count goes from "five" to "six".
- `[package.metadata.accelerator]` joins `_NAMED`, and a new test pins that
  the library checklist names it.

### Success Criteria:

#### Automated Verification:

- [ ] `uv run pytest tests/unit/tasks/test_crate_dependencies.py tests/unit/tasks/test_mise.py tests/unit/tasks/test_registration_docs.py` passes
- [ ] `rg -n -i 'thirteen' tasks/ tests/unit/tasks/ CLAUDE.md` is empty
- [ ] `mise run check` passes
- [ ] Adding `corpus-adapters = { path = "../corpus-adapters" }` to `cli/jira-client/Cargo.toml` makes `mise run check` exit non-zero naming `jira-client -> corpus-adapters: rule 4` (reverted after)
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] Both checklists read naturally with the new step

---

## Phase 13: Re-measure, Licence Record and After Figures

### Overview

Re-measure every binary, rewrite `deny.toml` and the notices, cut the signed
prerelease, take the after figures and apply the gates.

### Changes Required:

#### 1. Symbol counts and `deny.toml`

**File**: `cli/deny.toml:67-104`
**Changes**:

- Rebuild every binary unstripped
  (`CARGO_PROFILE_RELEASE_STRIP=false cargo build --release --target aarch64-apple-darwin`)
  and count `gix_`, `jj_lib` and `uluru` with `nm -a`.
- Rewrite the table with one row per binary, including `accelerator`
  (launcher) and `accelerator-research`.
- The "notice obligation is live for" sentence names exactly the binaries
  that link `uluru`.
- 🔴 If `accelerator-visualiser` links any of the three, stop for the
  author's decision, recorded here.

#### 2. Notices

`mise run notices:update`, then `mise run notices:check`. `uluru` stays in
the workspace closure, so the artefact should not change. If it does, review
the diff.

#### 3. After figures

- Run `mise run measure:launcher-size` and `mise run measure:summary-latency`
  on the Phase 1 host, including the large-repository run with Phase 1's
  arguments: `--repository . --revision
  22786dde6161e025e85a642e5ca701dcb389cb20`. Compare release-build figures
  only: on this host, process-spawning runs are bimodal at ~11 ms and ~34 ms,
  and the release build sat at ~32 ms in Phase 1.
- A holder of `ACCELERATOR_RELEASE_SECRET_KEY` runs `mise run prerelease` for
  the finished tree.
- Then run `mise run measure:warm-dispatch` against that prerelease. It
  refuses while any other Claude Code session runs on the host, and while
  the launcher cache is cold for the tree's version: warm it first with
  `bin/accelerator vcs detect`. Compare its C1 cell with Phase 1's 44.00 ms.
- Record raw figures and after/before ratios under Implementation Notes.
- 🔴 A ratio of 10 or more in size, or in any latency median (the
  large-repository run included), stops for the author's decision, recorded
  here.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run deny:check` and `mise run notices:check` pass
- [ ] `uv run pytest tests/unit/tasks/test_vcs_pin_lockstep.py` passes
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] Symbol-count table matches the measured counts for every binary
- [ ] Binaries named as carrying the MPL-2.0 obligation are exactly those linking `uluru`
- [ ] After figures and ratios recorded; no ratio ≥ 10, or a recorded decision
- [ ] Visualiser links none of `gix`, `jj-lib`, `uluru`, or a recorded decision

---

## Testing Strategy

### Unit Tests:

- **Lint**: every forbidden and permitted fixture and every declaration
  case, over synthetic metadata.
- **Measure tasks**: pure scheduling, fixture commands and report formatting,
  with fake runners.
- **New ports**: stub-driven domain tests (`RepositoryTracking`,
  `RepositoryProbe`, `FrontmatterParser`, `WorkItemIdCanonicaliser`,
  `SyncBaselines`, `collaboration::RepositoryOrigin`), and the new
  `MigrationContext` operations through m0001's and m0008's doubles.
- **`config::tracker_block`**: every refusal text, the full validation order,
  the one-`EntityList`-one-`ScopeFlag` invariant, and the extension field
  through an injected table in all three consumers.
- **`store::lock`**: the moved lock tests against `LockError`, one per arm and
  its text, plus `holder_pid`.
- **Panic boundary**: `PanicFold` over a panicking `RepositoryTracking`
  stub folds `file_tracking` to `Unknown` and `repository_roots` to an
  incomplete empty answer, each with a `warn!`.
- **`RepositoryProbe` blanket impl**: composed over stubs of the four
  fine-grained `vcs` ports.

### Integration Tests:

- `tests/integration/characterisation/`: built binaries in git and jj,
  compared against goldens, for every behaviour the refactor reroutes. No
  case reaches the network.
- `vcs-adapters` port contract tests in git and jj for every
  `RepositoryTracking` and `RepositoryProbe` method, including the corrupted
  index and a nested repository.
- `corpus-adapters` contract tests for `YamlFrontmatter` over every root class
  and for `PatternCanonicaliser` over every `PatternError` arm, plus the
  composition test with the domain pipeline.
- Launcher real-repository summary tests in `config_read.rs`, asserting
  single-line stdout.
- `tests/integration/deny`: gix feature absence for the launcher's graph.

### Manual Testing Steps:

1. Start a Claude Code session in a git repository with a tracked
   `config.local.md` and an empty launcher cache, and confirm the warning.
2. Repeat in a jj repository, and with the file untracked, and confirm there
   is no warning and no "was not checked" note.
3. Run `accelerator config` with a Jira integration carrying pull and push
   blocks, and compare with the pre-change output.
4. Run `/accelerator:migrate` on a scratch old-schema repository.

## Performance Considerations

- ⏱️ The launcher now links `gix` and `jj-lib`. Its size, warm-dispatch
  latency and summary latency are measured in Phase 1 and Phase 13 by
  committed tasks, and 10× is the stop threshold.
- In-process tracking has no deadline. A slow repository delays SessionStart.
  The fixture figures cannot show this, so a pinned large-repository run
  measures it at realistic scale and falls under the 10× stop. A panic folds to `Unknown` (Phase 7); a
  hang does not.
- Colocated jj is the most expensive tracking path (a jj workspace load plus
  two git index loads), and it is measured alongside git and plain jj.
- `gix`'s optional `signal-hook` must match the launcher's pin, or
  `multiple-versions = "deny"` fails (plan 0226).

## Migration Notes

The launcher and sub-binaries move together (ADR-0054). A new launcher never
dispatches `vcs tracking`, and an old launcher is never paired with a `vcs`
binary from this change, because sub-binaries re-fetch on a manifest-hash
change. No user data migrates.

## Implementation Notes

### Phase 1 before figures

Host: Mac16,5, macOS 26.3 (arm64), revision `22786dde` (the plan's parent).

- Launcher size, stripped `--release`, `aarch64-apple-darwin`:
  **8 065 488 bytes**. Taken at load 137.60 / 88.71 / 58.01; size does not
  depend on load.
- Summary latency, `mise run measure:summary-latency -- --repository .
  --revision 22786dde6161e025e85a642e5ca701dcb389cb20`, at load
  8.20 / 19.96 / 47.13 (the host was settling after heavy builds; Chrome and
  IntelliJ running):

  | Repository | Mode | Median | p90 |
  |---|---|---|---|
  | git | cold | 31.98 ms | 33.92 ms |
  | git | warm | 32.10 ms | 33.91 ms |
  | jj | cold | 31.66 ms | 34.63 ms |
  | jj | warm | 31.20 ms | 34.67 ms |
  | colocated jj | cold | 33.43 ms | 35.35 ms |
  | colocated jj | warm | 33.03 ms | 35.16 ms |
  | large jj | cold | 34.76 ms | 36.42 ms |
  | large jj | warm | 36.99 ms | 39.82 ms |

  The large repository is this workspace at
  `22786dde6161e025e85a642e5ca701dcb389cb20`: jj, 4587 index entries, 7
  operations in the snapshot. A run a minute earlier at load 15.12 gave
  medians within 3 ms of these. On this host, process-spawning runs are
  bimodal at ~11 ms and ~34 ms: the debug build landed in either mode, while
  the release build sat at ~32 ms in both runs. Take the after figures with
  the release build and compare like for like.
- Warm dispatch, `mise run measure:warm-dispatch` against the
  `1.24.0-pre.74` release, with no other Claude Code session running. Record:
  `meta/measurements/warm-dispatch-7.json` and its samples file. Load at start
  28.86 / 16.82 / 35.77 over 16 CPUs (the harness flagged the host as
  oversubscribed).

  | Cell | Point | 95 % interval |
  |---|---|---|
  | C1 median(G), fast backend | 44.00 ms | 43.91–44.10 |
  | C2 p90(G), fast backend | 52.59 ms | 52.10–52.91 |
  | C3 median(G), fallback backend | 54.12 ms | 53.82–54.36 |
  | C4 p90(G), fallback backend | 62.81 ms | 62.24–63.72 |
  | C5 median(G)/median(B), fast | 1.560 | 1.557–1.564 |
  | C6 median(G)/median(B), fallback | 1.919 | 1.908–1.929 |

  The run is `invalid-post-run` (branch 5b in every cell). Its drift
  diagnostic failed: the G/B ratio moved −0.0085 between the first and last
  thirds against a 0.0077 band (significance 0.036). That makes it no verdict
  on the warm-dispatch criterion. It is still adequate as the before figure
  for 0299's 10× gate, which a 0.5 % drift cannot move. The figure compared
  in Phase 13 is C1. An earlier attempt aborted on a single 200.67 ms outlier
  in B against a 34.57 ms running median.

Before symbol counts, unstripped `--release`, `aarch64-apple-darwin`,
`nm -a | grep -c`:

| Binary | `gix_` | `jj_lib` | `uluru` |
|---|---|---|---|
| `accelerator` | 0 | 0 | 0 |
| `accelerator-verify` | 0 | 0 | 0 |
| `accelerator-vcs` | 2273 | 2976 | 3 |
| `accelerator-work` | 2182 | 3000 | 3 |
| `accelerator-corpus` | 546 | 238 | 3 |
| `accelerator-collaboration` | 1679 | 2896 | 3 |
| `accelerator-migrate` | 2167 | 2890 | 3 |
| `accelerator-design` | 1688 | 2909 | 3 |
| `accelerator-linear` | 1678 | 2896 | 3 |
| `accelerator-jira` | 1678 | 2896 | 3 |
| `accelerator-research` | 1678 | 2896 | 3 |
| `accelerator-visualiser` | 0 | 0 | 0 |

`deny.toml` is already stale: it records `design`, `linear` and `jira` as
linking none of the three and omits `research`, but all four link the closure
today through `consent-adapters` → `vcs-adapters`.

### Progress

- Phase 1 is complete, before figures included.
- `mise run` first failed on `docs:audit:check`: a new high advisory,
  `GHSA-68fv-2mgg-jv7q`, against `source-map-js@1.2.1` in `docs-site/`. It
  is unrelated to 0299. `docs:audit:fix` also bumped about a dozen unrelated
  packages, so only `source-map-js` went to 1.2.2, in a commit of its own
  ahead of Phase 1.
- Phase 2 is complete. Debug builds were split into per-binary and group
  leaves with the author (Wiring section).

### Phase 2 characterisation coverage

534 cases across 11 behaviour modules, each run in git and jj unless marked
`vcs_specific`.

| 0299 bullet or rerouted edge | Module and case |
|---|---|
| Six consent keys, each non-launcher root, tracked and untracked | `test_consent_reads.py`: `jira`, `linear`, `collaboration`, `research`, `work sync --preview` (jira and linear), `design executor ping` |
| Launcher root resolution, nested repository | `test_summary_tracking.py::test_summary_in_a_nested_git_repository` |
| SessionStart tracking: tracked, untracked, outside, corrupt index, colocated, with and without `accelerator-vcs` | `test_summary_tracking.py` |
| `accelerator config` dump of tracker blocks, multi-fault order | `test_config_dump.py` |
| `jira-cli`/`linear-cli` block parsing; `work sync` validation | `test_tracker_blocks.py` |
| `corpus metadata derive`, clean, dirty, unborn, outside | `test_metadata_derive.py` |
| `frontmatter validate` over every violation class and root class | `test_frontmatter_validate.py` |
| `work-cli` author, create/update locks, `sync --preview`, `list` | `test_work_commands.py` |
| `research-adapters` → `corpus-adapters`: `parse`, `validate_path`, `validate_text`, ledger store | `test_research_commands.py` |
| `jira-client`/`linear-client` → `corpus-adapters` (lock) | `test_tracker_caches.py` |
| `collaboration` → `vcs` (`OriginRemote`) | `test_collaboration.py` |
| `migrate` → `document`; `migrate-adapters` → config, VCS, corpus and work adapters | `test_migrations.py`: m0001 root classes, m0008 re-render, comment loss, baseline realignment (relative and absolute `paths.integrations`), run-lock refusal, full registry |

Deviations from the Phase 2 text, found while writing the cases:

- **Release fetches:** the production fetcher refuses `http://` before any
  I/O, so the loopback never sees a release request. `release_fetches`
  became `loopback_requests`, counting every overridden URL.
- **arXiv lock:** it is a kernel `flock`, with no PID sentinel to reclaim.
  The case holds the lock and pins the contention answer instead.
- **`sync_author` lock:** taken only when a sync creates an item from the
  remote, which needs a tracker response `work-cli` cannot be pointed at
  offline. `work create` takes the same lockdir through the same primitive.
- ⚠️ **Working-copy status:** `work sync` consults it only for pulls, and
  `work list` only once a tracker resolves and answers. `work-cli` has no
  API-URL override, so neither is reachable offline without real network.
  `VcsWorkingCopyStatus` stays covered by its unit tests and, from Phase 5,
  by the `RepositoryProbe` contract tests in git and jj. Migrate's preflight,
  which reads `working_copy_state` through `VcsWorkingCopy`, is characterised.
- **Launcher `TEMP_PREFIX`:** used only when a fetched binary is installed,
  which no offline case reaches. The Phase 4 constant tests guard it.
- **Golden findings worth knowing:** `jira-cli` and `linear-cli` resolve
  ceilings without validating, so unknown keys pass them; `work update` keeps
  `last_updated`; a failed migrate run records git's `HEAD` but jj's `@-`.

### Still to record

- Rule 3 outcomes (Phase 11).
- After figures and ratios (Phase 13).
- Author decisions, if any gate tripped.

## References

- Original work item: `meta/work/0299-bring-the-cli-workspace-s-crate-dependencies-into-line-with.md`
- Related codebase research: `meta/research/codebase/2026-10-06-0299-crate-dependencies-adr-0069.md`
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- `meta/plans/2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys.md`
- Lint precedent: `tasks/lint/config_test_support.py:39-100`
- Port-with-default-body precedent: `cli/migrate/src/ports.rs:212-274`
