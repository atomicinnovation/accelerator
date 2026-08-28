---
type: plan
id: "2026-08-28-0174-empty-scripts-retire-shell-tooling"
title: "Empty scripts/ and Retire Shell Tooling and CI Guards Implementation Plan"
date: "2026-08-28T07:28:16+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0174"
parent: "work-item:0174"
derived_from: ["codebase-research:2026-08-28-0174-empty-scripts-and-retire-shell-tooling"]
tags: [shell, tooling, ci, cleanup, scripts]
revision: "b1d635d8f391ff188fc0a76508e85bf3a98d8ef0"
repository: "accelerator"
last_updated: "2026-08-28T12:44:34+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Empty scripts/ and Retire Shell Tooling and CI Guards Implementation Plan

## Overview

Empty `scripts/` to the two-file thin-shell floor, retire the build-system and
CI machinery that exists only to police the vanished bash library, and re-home
the guards that still have a job. The migration is behaviourally complete
(0167–0172, 0195–0197, 0211, 0212 all done); what remains is severing a handful
of live couplings, deleting residue in per-commit lockstep, relocating two data
files into their consuming Rust crates, and porting nine authoring guards plus
the bashisms denylist to Python.

## Current State Analysis

The `scripts/` surface is 28 `.sh` files (13 sourced-only libraries + the
`lint-bashisms.sh` entrypoint + 14 `test-*.sh` harnesses), 5 data files, and 3
fixture trees. Two independent lockstep couplings drive the whole plan:

- **The exec-bit stale-entry guard** (`tasks/lint/scripts.py:98-103`). It
  appends an offender for every `SHELL_LIBRARIES` path no longer enumerated by
  `shell_sources()`. Deleting a library without dropping its frozenset entry in
  the same commit fails the lint. `tests/unit/tasks/test_exec_bits.py:243-259`
  mirrors the entire 13-member frozenset (`_RECONCILED_LIBRARIES`) by
  set-equality, so it moves in the same commit.
- **The config suite floor** (`_EXPECTED_CONFIG_SUITES = 14`,
  `tasks/test/integration.py:38`). It is a `scripts/`-wide gauge:
  `run_shell_suites(context, "scripts", …)` glob-discovers every executable
  `scripts/**/test-*.sh` minus `test-helpers.sh`. Every `test-*.sh` deletion —
  domain test *and* authoring-guard port alike — decrements it. `_REQUIRED_CONFIG_SUITES`
  (`:48`) additionally pins `scripts/test-skill-frontmatter-conformance.sh` by
  name, and `tests/unit/tasks/test_integration.py` mirrors these constants.

Only four floor constants survive (`config = 14`, `hooks = 1`, `decisions = 0`,
`github = 0`); `_EXPECTED_WORK_SUITES` and `_EXPECTED_INTEGRATIONS_SUITES` were
already removed by 0211/0212. `SHELL_LIBRARIES` holds thirteen entries (no
`work-common.sh`). The single live bash coupling is one call in the Jira
create-issue SKILL.md that keeps the four-file config source-chain alive.

### Key Discoveries

- **The Jira cutover has no callable target yet.** `link_external_id`
  (`cli/work-cli/src/sync_author.rs:139-161`) is a `LocalAuthor` trait method
  reached only through `accelerator work sync`; its body touches only `path`
  and `external_id`, never `self.config/root/work_dir`, so it lifts cleanly
  into a free function. `work-cli` already depends on every crate the writeback
  needs — **zero new crate dependencies**.
- **`templates-schema.tsv` is `#[cfg(test)]`, not production.** The `include_str!`
  at `cli/corpus/src/frontmatter_validation/schema.rs:277` sits inside a
  `#[cfg(test)] mod tests`. Relocation is still same-commit (test-build
  compile-fail otherwise), but the risk is a test build, not the shipped binary.
- **Two drift-oracle tests are rescope-not-delete.**
  `cli/config/tests/extra_keys_mirror.rs` carries a second test
  (`the_provider_client_keys_are_registered`) that reads no shell;
  `cli/corpus-adapters/tests/parity.rs` carries a pure-Rust regex case
  (`the_compiled_scan_regex_drives_slug_and_id_extraction`). Deleting either
  whole file drops real coverage.
- **`feature = "bash-parity"` gated cases are dropped with the file they read**,
  not deferred, so every commit stays green regardless of whether the default
  lane exercises that feature.
- **The surviving thin shell is two files**, not three: `bin/accelerator` and
  `hooks/launcher-link-refresh.sh`. The Playwright executor is `run.js`
  (JavaScript), outside the shell survivor set.
- **`test.unit.templates` (`tasks/test/unit.py:37-52`) invokes two authoring
  guards directly**, a second lane the config floor does not cover.

## Desired End State

`find scripts -name '*.sh'` returns nothing. The two thin-shell survivors are
homed in `bin/` and `hooks/`, guarded automatically by a Python bashisms task
plus shfmt and ShellCheck over an explicit two-file list asserted equal to a
`tasks/README.md` enumeration. The exec-bit invariant, `SHELL_LIBRARIES`, the
four suite floors, `shell_sources()`, and the `check-scripts` CI job are gone.
The bashisms denylist, nine authoring guards, and two non-detection hook guards
run as Python/pytest. Two data files live under `cli/`. `mise run` (bare
default) exits 0 end-to-end, and every intermediate commit is independently
green.

## What We're NOT Doing

- **Not carrying the work-item or integration clusters.** 0211/0212 already
  deleted `work-item-*.sh`, the jira/linear libraries, `_EXPECTED_WORK_SUITES`,
  and `_EXPECTED_INTEGRATIONS_SUITES`. Do not re-retire them.
- **Not removing shfmt, ShellCheck, `.shellcheckrc`, or the `[*.sh]`
  editorconfig block.** They are retained and rescoped to the two survivors.
- **Not touching the surviving `bin/accelerator` and
  `hooks/launcher-link-refresh.sh` behaviour** beyond keeping them bash-3.2-safe.
- **Not making a live Jira API call** in the cutover verification — the new
  subcommand is verified against the local writeback path only.
- **Not redesigning the relocated data files' consumers** — move the file,
  repoint the `include_str!`, nothing more.

## Implementation Approach

Ten phases, each one or a few independently-`mise run`-green commits. Ordering
is dictated by the two lockstep couplings and three hard constraints: cutovers
precede the deletions they unblock; `include_str!` relocations are same-commit;
and `test-helpers.sh` is deleted last, after every `test-*.sh` and
`hooks/test-vcs-detect.sh` that sources it. Each phase names its **atomic
bundle** — the set of edits that must land in one commit or CI goes green→red.

Granularity is deliberate: the story's entire risk model is per-commit
greenness on a floor/frozenset mismatch, and small commits are the mitigation.
The bashisms Python port and the walk rescope land in separate phases (9 and 10)
so the port is not entangled with retiring the tree-walk it transitionally uses.

Deletion tally across phases keeps both couplings honest:

| Coupling | Start | Drained by |
|---|---|---|
| `SHELL_LIBRARIES` entries | 13 | P2 (4), P4 (1), P6 (5), P7 (2), P10 (1) |
| Config floor `test-*.sh` | 14 | P2 (2), P4 (1), P6 (2), P7 (9) |

---

## Phase 1: Jira external-id cutover

### Overview

Add `accelerator work link-external-id <work-item-path> <external-id>` to
`work-cli` and repoint the Jira create-issue SKILL.md's one bash writeback line
onto it. This severs the sole live consumer of the config source-chain, without
yet deleting anything — the chain is removed in Phase 2.

### Changes Required

#### 1. New subcommand — `cli/work-cli/src/cli.rs`

Model on the two-positional `Diff` variant (`cli.rs:46-51`), not the boxed
`UpdateArgs`:

```rust
/// Write an already-created remote id into a local draft's `external_id`.
LinkExternalId {
    path: PathBuf,
    external_id: String,
},
```

#### 2. Promote the writeback to a shared free function — `cli/work-cli/src/sync_author.rs`

`link_external_id` is today a `LocalAuthor` **trait method**
(`sync_author.rs:139`), so it cannot be called as a free function as written.
Its body touches no `self` field (Key Discovery), so promote it to a
`pub(crate)` free function `link_external_id(path, external_id)` in
`sync_author.rs` — beside the module-private `fn failed` and the
`document`/`corpus`/`corpus_adapters`/`tracker` imports it already depends on —
and have the trait method delegate to it (`pub(crate)` because `main.rs` calls
it across modules; the trait method today is reachable only through the trait).
This keeps one source of truth for the parse/`Mapping::set`/`AtomicWrite`
sequence; the story deletes every drift oracle, so a duplicated copy would have
no guard against divergence.

#### 3. Dispatch — `cli/work-cli/src/main.rs`

Add the match arm (`main.rs:451-467`) and a thin `run_link_external_id` that
delegates to the shared function, mirroring the other `run_*` dispatch
adapters (no writeback-domain logic in `main.rs`). Ensure `main.rs` has
`std::path::Path` and `tracker::ExternalId` in scope (or fully-qualifies them)
for the signature and constructor below:

```rust
Command::LinkExternalId { path, external_id } => {
    run_link_external_id(&path, &external_id)
}
```

```rust
fn run_link_external_id(path: &Path, external_id: &str) -> ExitCode {
    let external_id = tracker::ExternalId::new(external_id.to_owned());
    match sync_author::link_external_id(path, &external_id) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
```

The shared `link_external_id` reads the file, parses frontmatter,
`Mapping::set("external_id", …)` (upsert — covers both insert and overwrite),
renders through `document::render`, and commits via `AtomicWrite`. Every crate
import it needs is already present in `work-cli`.

⚠️ **Frontmatter-safety.** `ExternalId::new` is an unchecked wrapper; the
writeback encodes the value as a YAML scalar through `document::render`, which
is at least as safe as the retired bash string interpolation for realistic Jira
keys (`PP-195`). No new validation is added — the behaviour matches the retired
`config_upsert_frontmatter_field` for the insert and overwrite cases the AC
names.

#### 4. Repoint — `skills/integrations/jira/create-jira-issue/SKILL.md`

Replace the two-line writeback (`:110-112`), keeping WF-4's two-step shape and
the non-atomic caveat (`:124-127`), updating the caveat's wording to name the
new command:

```diff
-  source ${CLAUDE_PLUGIN_ROOT}/scripts/config-common.sh
-  config_upsert_frontmatter_field <work-item-file> external_id <KEY>
+  ${CLAUDE_PLUGIN_ROOT}/bin/accelerator work link-external-id <work-item-file> <KEY>
```

### Atomic bundle

`cli.rs` + `sync_author.rs` (trait method delegating to the promoted free
function) + `main.rs` + the new integration test + the SKILL.md repoint + caveat
wording. Nothing is deleted this phase; the `call_site_migration.py`
`config-common.sh` allowlist stays (it now permits an unreferenced file,
harmless) and is removed in Phase 2.

### Success Criteria

#### Automated Verification

- [x] Workspace builds and clippy passes: `mise run cli:check`
- [x] New integration test passes (model on `cli/work-cli/tests/cli_update.rs`),
      landing the failing cases first: insert (no prior `external_id`) and
      overwrite both write the expected `external_id` scalar **and** leave the
      rest of the file (all other frontmatter fields and the body) byte-identical
      — assert whole-file, minus the changed scalar, not merely the neighbours;
      at least one error case (non-mapping frontmatter or unreadable
      file) yields a non-zero exit with a stderr message: `mise run test:unit:cli`
- [x] No live-Jira coupling in the test — it invokes the built binary against a
      scratch repo only.
- [x] Bare default stays green: `mise run`

#### Manual Verification

- [x] The repointed SKILL.md reads coherently as model-instruction text; the
      non-atomic caveat names `work link-external-id`, not the retired helper.

---

## Phase 2: Config source-chain deletion

### Overview

Delete `config-common.sh` and its `vcs-common.sh` / `config-defaults.sh` /
`atomic-common.sh` source chain, plus their two `test-*.sh` harnesses, and every
drift-oracle coupling that reads them. This is the largest single lockstep
bundle.

### Changes Required

#### 1. Delete the chain and its tests

- `scripts/config-common.sh`, `scripts/vcs-common.sh`,
  `scripts/config-defaults.sh`, `scripts/atomic-common.sh`
- `scripts/test-atomic-common.sh`, `scripts/test-vcs-common.sh`

#### 2. Rescope, don't delete — `cli/config/tests/extra_keys_mirror.rs`

Drop `defaults_script()`, `bash_extra_keys()`, and
`the_bash_registry_lists_exactly_what_the_catalogue_does`. **Keep**
`the_provider_client_keys_are_registered` — it pins provider keys against
`config::catalogue::EXTRA_KEYS` with no shell involvement.

#### 3. Drop the gated case — `cli/corpus-adapters/tests/doc_type_single_source.rs`

Remove `every_config_path_key_exists_in_the_config_schema` (`:61-104`, sources
`config-defaults.sh` `PATH_KEYS`). It is the sole consumer of
`use std::process::Command;` in the file, so prune that import in the same
commit — `test:unit:cli` compiles this `bash-parity` file with `--all-features`
under `warnings = "deny"`, so a leftover unused `Command` reds the bare default
lane (and correspondingly drop `std::process::Command` from Phase 5's prune
list, where it no longer survives). Leave the file
`#![cfg(feature = "bash-parity")]` gated — case C
(`the_type_pair_table_matches_the_tsv`) still needs it until Phase 5.

The deleted case pinned every `DocTypeKey::config_path_key()` against the config
schema; the surviving cases only count registrations. Before dropping it,
confirm a native Rust test already asserts config-path-key resolution as
exhaustively — iterating `DocTypeKey::all()` and, for each `Some(config_path_key())`,
asserting membership in config's canonical path-key set (not a three-key
spot-check). If none exists, add one, so the drift guard survives the bash
oracle's removal at full strength.

#### 4. Guard-machinery lockstep

- `tasks/lint/scripts.py` — drop the four `SHELL_LIBRARIES` entries.
- `tests/unit/tasks/test_exec_bits.py:243-259` — drop the same four from
  `_RECONCILED_LIBRARIES`.
- `tasks/test/integration.py:38` — `_EXPECTED_CONFIG_SUITES` 14 → 12.
- `tests/unit/tasks/test_integration.py` — mirror the new floor.
- `tasks/measure.py:977-986` — drop the `scripts/vcs-common.sh` entry from
  `RECOVERED_FILES` (a baseline-frozen provenance record).
- `tests/unit/tasks/test_measure.py:1293,1744-1755,1796` — update the
  `RECOVERED_FILES` set assertions and the stubbed-layout fixtures.
- `tasks/lint/call_site_migration.py:32` + `tests/unit/tasks/test_call_site_migration.py:35`
  — remove the now-dead `config-common.sh` allowlist entry and its test.

### Atomic bundle

All of the above in one commit. The four `SHELL_LIBRARIES` deletions, the
`_RECONCILED_LIBRARIES` mirror, the floor decrement, the `extra_keys_mirror.rs`
rescope, the `doc_type_single_source.rs` case-B drop, the `RECOVERED_FILES`
edit, and the `call_site_migration.py` allowlist all fail their respective lanes
if split.

### Success Criteria

#### Automated Verification

- [ ] `find scripts -name 'config-common.sh' -o -name 'vcs-common.sh' -o -name 'config-defaults.sh' -o -name 'atomic-common.sh'` returns nothing
- [ ] Config crate tests pass (the surviving provider-key test runs): `mise run test:unit:cli`
- [ ] Exec-bits lint clean (no stale entry): `mise run lint:scripts:exec-bits:check`
- [ ] Bare default green end-to-end: `mise run`

#### Manual Verification

- [ ] No `skills/`/`hooks/`/`templates/` file still sources the deleted chain
      (repo-wide grep for each path resolves to nothing live).

---

## Phase 3: Data-file relocations

### Overview

Move the two Rust-consumed data files into their consuming crates, repointing
each `include_str!` in the same commit. Both couplings are compile-time — a
split commit fails the test build.

### Changes Required

#### 1. `templates-schema.tsv` → `corpus` crate

Move `scripts/templates-schema.tsv` beside its consumer at
`cli/corpus/src/frontmatter_validation/templates-schema.tsv` and repoint the
`#[cfg(test)]` `include_str!` at `cli/corpus/src/frontmatter_validation/schema.rs:277`
to `include_str!("templates-schema.tsv")`. Co-locating in the src tree (not
`tests/`) sits the data with the module that reads it and avoids a
`../../tests/` traversal across the src→root→tests boundary. Update the
doc-comment mentions at `schema.rs:1,6,16` and
`cli/migrate/src/migrations/m0007/schema.rs:1` (prose only).

⚠️ **`test-skill-frontmatter-conformance.sh` also parses this TSV**
(`:61-76`) and is still live until Phase 7. Keep a copy readable by the shell
guard until then, or sequence Phase 7 to consume the relocated path. Default:
the shell guard reads `scripts/templates-schema.tsv`; relocation moves it, so
either (a) leave a `scripts/templates-schema.tsv` in place until Phase 7 and
relocate a *copy* now, or (b) repoint the shell guard's TSV path this phase.
**Chosen: (b)** — repoint the guard's `templates-schema.tsv` read to the
relocated path this phase, since the guard still runs. Same for
`test-template-frontmatter.sh:30`. Have the repointed guards fail-closed on an
unreadable or zero-row TSV (assert the file exists and parses at least one row)
so a wrong relative path errors loudly rather than passing vacuously on empty
parallel arrays through the P3–P6 window before the guards are ported.

#### 2. `extract-work-items-cue-phrases.txt` → `design` crate

Move `scripts/extract-work-items-cue-phrases.txt` under `cli/design/tests/` and
repoint the `include_str!` at `cli/design/tests/cue_phrase_drift.rs:11`.

### Atomic bundle

Each file move + its `include_str!` repoint + (for `templates-schema.tsv`) the
two shell guards' TSV-path repoint, in one commit.

### Success Criteria

#### Automated Verification

- [ ] `corpus` and `design` test builds compile and pass: `mise run test:unit:cli`
- [ ] `templates-schema.tsv` and `extract-work-items-cue-phrases.txt` no longer
      under `scripts/`; both resolve under `cli/`
- [ ] Bare default green: `mise run`

#### Manual Verification

- [ ] No dangling `scripts/templates-schema.tsv` or
      `scripts/extract-work-items-cue-phrases.txt` reference in `cli/`,
      `tasks/`, or the two still-live shell guards.

---

## Phase 4: doc-type-inference cutover

### Overview

Drop the bash-oracle case from `parity.rs`, then delete
`doc-type-inference.sh` and its test. The surviving pure-Rust regex case stays.

⚠️ **Two `infer` behaviours are covered only by the bash-oracle case being
deleted**: the exact-length tie-break (first-listed type wins) and
interior-segment matching (a directory embedded mid-path, not a `starts_with`
prefix). The native `doc_type.rs` unit tests cover longest-wins, whole-segment,
and no-match, but neither a tie nor an interior segment — so a mutation of the
tie comparison or removal of the interior-match branch would pass the suite
after deletion. Before deleting the bash case, add two native `doc_type.rs`
tests (landing them failing first if the behaviour is not yet pinned): an
exact-length tie asserting the first-listed type wins, and an interior-segment
path asserting a mid-path directory matches.

### Changes Required

#### 1. Rescope — `cli/corpus-adapters/tests/parity.rs`

Remove `doc_type_inference_matches_the_bash_matcher` (`:148-215`) and the
`bash_infer` helper (`:86-146`). **Keep**
`the_compiled_scan_regex_drives_slug_and_id_extraction` (`:27-84`) and drop the
file-level `#![cfg(feature = "bash-parity")]` gate (`:9`) — the surviving case
needs no feature.

⚠️ **Prune every orphaned import in the same commit.** `test:unit:cli` compiles
with `--all-features` (`bash-parity` on) under workspace `warnings = "deny"`
(`cli/Cargo.toml:192`), so any import left unused by the deletion is a hard
compile error that reds the bare `mise run` default lane. Removing the bash case
and `bash_infer` orphans not just `tempdir`/`TempDir` but also `std::fs`,
`std::process::Command`, `std::path::{Path, PathBuf}`, `common::require_file`,
and the top-level `use corpus::DocTypeKey;` (the surviving case re-imports
`DocTypeKey` locally). Drop all of them. (The surviving case already ran under
`--all-features`; de-gating only additionally lets it run without the feature.)

#### 2. Delete

- `scripts/doc-type-inference.sh`, `scripts/test-doc-type-inference.sh`

#### 3. Lockstep

- `tasks/lint/scripts.py` + `test_exec_bits.py` — drop the
  `doc-type-inference.sh` `SHELL_LIBRARIES` / `_RECONCILED_LIBRARIES` entry.
- `tasks/test/integration.py:38` + `test_integration.py` —
  `_EXPECTED_CONFIG_SUITES` 12 → 11.

`call_site_migration.py` does **not** reference `doc-type-inference.sh` (verified) —
no edit there.

### Success Criteria

#### Automated Verification

- [ ] `find scripts -name '*doc-type-inference*'` returns nothing
- [ ] The de-gated regex case runs in the default lane: `mise run test:unit:cli`
- [ ] Exec-bits clean; bare default green: `mise run`

#### Manual Verification

- [ ] `parity.rs` reads coherently as a single pure-Rust regex pin.

---

## Phase 5: doc_type_single_source rescope and linkage-type-pairs deletion

### Overview

Drop the last bash-data-oracle case from `doc_type_single_source.rs`, de-gate
the file, and delete `linkage-type-pairs.tsv`. `corpus::linkage::TYPE_PAIRS`
(`cli/corpus/src/linkage.rs:60`) is a hand-maintained const with no reader left
once the drift test is gone.

### Changes Required

#### 1. Rescope — `cli/corpus-adapters/tests/doc_type_single_source.rs`

Remove `the_type_pair_table_matches_the_tsv` (`:109-146`, reads
`linkage-type-pairs.tsv`). **Keep** `every_non_virtual_type_is_registered_exactly_once`
(`:26-53`) and drop the file-level `#![cfg(feature = "bash-parity")]` gate
(`:16`) — it now holds only the bash-free case, which drives the compiled
`config paths --doc-types` resolver. Prune the now-unused `std::fs` and
`require_file` imports (`std::process::Command` was already pruned in Phase 2
with its sole consumer). The same `--all-features` + `warnings = "deny"`
constraint applies — a leftover unused import reds the bare default lane.

⚠️ **The surviving case spawns the compiled launcher** (`common/mod.rs:93`), so
it needs `mise run test:unit:cli` (workspace build with the launcher on the
resolution path), not a bare `cargo test -p corpus-adapters`. It already ran
under that lane's `--all-features`; de-gating only additionally lets it run
without the feature.

#### 2. Delete `scripts/linkage-type-pairs.tsv`

`status-legacy-map.tsv` is a pure orphan deleted in Phase 6.

### Success Criteria

#### Automated Verification

- [ ] `find scripts -name 'linkage-type-pairs.tsv'` returns nothing
- [ ] The de-gated resolver case passes: `mise run test:unit:cli`
- [ ] Bare default green: `mise run`

#### Manual Verification

- [ ] Nothing in `cli/` reads `linkage-type-pairs.tsv`; `TYPE_PAIRS` stands as
      the single source with no drift oracle.

---

## Phase 6: Orphan library deletions

### Overview

Delete the residual libraries and paired tests with no live consumer, each
dropping its `SHELL_LIBRARIES` entry and (for the two `test-*.sh`) decrementing
the config floor.

### Changes Required

#### 1. Delete

- Libraries: `scripts/log-common.sh`, `scripts/accelerator-scaffold.sh`,
  `scripts/doc-type-table.sh`, `scripts/hash-common.sh`, `scripts/fs-common.sh`
- Tests: `scripts/test-hash-common.sh`, `scripts/test-merge-move.sh`
- Data/fixtures: `scripts/status-legacy-map.tsv`,
  `scripts/test-fixtures/config-read-review/`

#### 2. Lockstep

- `tasks/lint/scripts.py` + `test_exec_bits.py` — drop the five library entries
  (`fs-common`, `hash-common`, `log-common`, `doc-type-table`,
  `accelerator-scaffold`).
- `tasks/test/integration.py:38` + `test_integration.py` —
  `_EXPECTED_CONFIG_SUITES` 11 → 9 (two `test-*.sh` gone).
- `tasks/lint/call_site_migration.py:43-63` — remove the `doc-type-table.sh`
  legacy-flag exemption branch (`:55`) now that the file is gone; update the
  docstring and any pinning test.

### Success Criteria

#### Automated Verification

- [ ] `find scripts \( -name 'log-common.sh' -o -name 'accelerator-scaffold.sh' -o -name 'doc-type-table.sh' -o -name 'hash-common.sh' -o -name 'fs-common.sh' -o -name 'status-legacy-map.tsv' \)` returns nothing; `scripts/test-fixtures/config-read-review/` gone
- [ ] `call_site_migration` lint still passes with the exemption removed: `mise run lint:build-system:check`
- [ ] Exec-bits clean; bare default green: `mise run`

#### Manual Verification

- [ ] No `--allow-legacy-layout` reference now lives outside the migration
      engine (the exemption's removal did not orphan a real call site).

---

## Phase 7: Nine-guard Python port

### Overview

Port the nine authoring/evals guards to pytest, carrying `skills-schema.tsv`
and the frontmatter rules/fixtures as pytest fixtures. `test-evals-structure-self`
folds into the two guards it meta-tests, yielding eight standalone guards. This
drains the config floor to zero and removes the config integration task.

The eight pure content-scanning guards home in `tests/unit/tasks/` (their
siblings there are pure-Python scanners). The **conformance guard is the
exception**: it drives the compiled `accelerator corpus frontmatter validate`
through the launcher, needing the `ACCELERATOR_CORPUS_BIN` overlay
(`accelerator_env(corpus_bin=True)`) and the `build:cli:dev` launcher build the
retired `test:integration:config` lane supplied. Home it (and its
design-structure appendix module) in an integration lane that provisions the
launcher — mirroring how Phase 8 routes its binary-driving hook guards to
`tests/integration/hooks/` — not the pure-Python unit-tasks lane.

**Two commits, not one, for the conformance long pole.** Land every pytest port
green while the shell guards are still live and the config floor is unchanged,
then in the next commit delete the nine shell guards and drain the floor. Be
honest about what this buys: it is a positive-tree co-existence checkpoint (both
guard sets pass the same conforming live tree) and a revert-friendly boundary —
**not** a differential negative cross-check, since each guard set scans its own
independent negative corpus. The real defence against a silently dropped
sub-assertion is the parity checklist below. Where a cross-check is cheap, add
one: assert the ported conformance guard's enumerated emitter/discovery counts
(e.g. "17 discovered"/"16 emitters") against the same live-tree greps the shell
guard used, so a dropped count-reconciliation fails loudly. The other guards,
being lower-blast-radius, may port-and-delete in one commit. Each ported guard's
negative-fixture test lands failing first, then the port to green.

### Changes Required

#### 1. Port the eight content guards to `tests/unit/tasks/`; the conformance guard to an integration lane

The eight below (no `__init__.py`; auto-discovered) follow the
`tests/unit/tasks/test_skill_permissions.py` shape — synthetic `tmp_path`
fixtures per branch plus one live-tree assertion. The conformance guard goes to
the launcher-provisioning integration lane per the overview. Before porting each
guard, enumerate its existing shell `assert_*` blocks and their negative
fixtures as a parity checklist, and require the port to reproduce **each** one
(carry every captured negative fixture, not a representative subset) — the
coarse "fail on negatives, pass on conforming" criterion alone lets individual
sub-assertions drop silently. Per guard:

| Guard | Ports to | Live-tree target |
|---|---|---|
| `test-format` | hyphenation regex scan | `skills/ scripts/ templates/ README.md CHANGELOG.md` |
| `test-hierarchy-format` | canonical-fence equality | two work-skill SKILL.md |
| `test-lens-structure` | lens SKILL.md structure | `skills/review/lenses/*-lens/` |
| `test-boundary-evals` | 100%-pass benchmark check | five lens `boundary_benchmark.json` |
| `test-evals-structure` | evals/benchmark pairing + 0.9 floor | `skills/**/evals/` |
| `test-skill-frontmatter-conformance` | validator-driven conformance | 16 emitters + `templates-schema.tsv` |
| `test-skill-frontmatter-population` | populate-instruction detectors | `skills/**/SKILL.md` + `skills-schema.tsv` |
| `test-template-frontmatter` | template-shape contract | `templates/*.md` + `templates-schema.tsv` |

`test-evals-structure-self`'s exit-code assertions become fixture cases inside
the `test-evals-structure` and `test-hierarchy-format` ports (each fixture dir
under `scripts/test-evals-structure-fixtures/` and
`scripts/test-hierarchy-format-fixtures/` carried into `tests/`).

⚠️ **`test-skill-frontmatter-conformance` is the long pole** (768 lines): it
drives the compiled corpus validator (`corpus frontmatter validate`), parses
`templates-schema.tsv` and `frontmatter-emission-rules.sh`, synthesises fixtures
(`frontmatter-fixtures.sh`), and appends a large design-structure block
(`:407-765`: agent tool-allowlists, docs-site, design-script resolution,
`downgrade.rs` `REASONS_EVER`). The appendix is a distinct concern — port it as
its own module (`test_design_structure.py`); both count under the one source
guard. Home by dependency, not by default: assertions that actually invoke the
compiled `accelerator` binary belong in the launcher-provisioning integration
lane beside conformance-proper, but any appendix assertion that is a pure
static content/source scan (agent-md allowlists, docs-site files, the
`REASONS_EVER` const table) belongs in `tests/unit/tasks/` with the other
content scanners — don't gate a pure-Python check on a launcher build. Confirm
which appendix assertions call the binary before homing.

#### 2. Carry fixtures and contract data

- `skills-schema.tsv` → a pytest fixture/resource under `tests/`.
- `frontmatter-emission-rules.sh` constants → re-express once as a named,
  convention-compliant Python module (e.g. `tasks/lint/frontmatter_rules.py`,
  no underscore prefix) that both the conformance and template ports import —
  a module-plus-test split mirroring `skill_permissions.py`, not rules inlined
  into a `test_*.py` file.
- `frontmatter-fixtures.sh` `emit_valid`/`run_validator`/`assert_*` → Python
  helpers driving the built `accelerator` binary.

#### 3. Delete the nine shell guards and their fixture inputs

`scripts/test-format.sh`, `test-hierarchy-format.sh`, `test-lens-structure.sh`,
`test-boundary-evals.sh`, `test-evals-structure.sh`, `test-evals-structure-self.sh`,
`test-skill-frontmatter-conformance.sh`, `test-skill-frontmatter-population.sh`,
`test-template-frontmatter.sh`; `scripts/skills-schema.tsv`,
`frontmatter-emission-rules.sh`, `frontmatter-fixtures.sh`; the two fixture
trees (once copied into `tests/`).

#### 4. Lockstep

- `tasks/lint/scripts.py` + `test_exec_bits.py` — drop the
  `frontmatter-emission-rules.sh` and `frontmatter-fixtures.sh` entries.
- `tasks/test/integration.py` — `_EXPECTED_CONFIG_SUITES` 9 → 0; remove the
  constant, `_REQUIRED_CONFIG_SUITES` (`:48`), and the `config` task
  (`:338-346`); drop `test:integration:config` from `mise.toml` and the
  `test:integration` roll-up. Mirror in `test_integration.py`.
- **Register the replacement lane concretely** — the ported conformance +
  `test_design_structure.py` modules run nothing unless a task invokes them (the
  existing integration tasks are explicit per-subdir runners, not
  auto-discovered). In the same bundle that removes the `config` task, add a new
  `@task` in `tasks/test/integration.py` that runs `uv run pytest` over the
  conformance modules' directory with the `accelerator_env(corpus_bin=True)`
  overlay and a `build:cli:dev` dependency (the overlay + launcher build the old
  `config` lane supplied), and add the task to the `test:integration` roll-up in
  `mise.toml` — which is what the CI `test-integration` job runs (no per-task
  workflow edit needed). Without this the
  conformance guard is an unwired, illusory gate — the frontmatter-conformance
  regression net the plugin depends on.
- `tasks/test/unit.py:37-52` (`test.unit.templates`) — the two pytest ports
  (`test-template-frontmatter`, `test-skill-frontmatter-population`) are
  auto-discovered under `tests/unit/tasks/` and fully subsume this task's two
  guards, so delete the task and its `mise.toml` wiring rather than repoint it
  onto a now-redundant lane.

### Success Criteria

#### Automated Verification

- [ ] Each ported guard fails on every captured negative fixture and passes on
      the conforming ones (the `-self` assertions verified through the two host
      guards): `mise run test:unit:tasks`
- [ ] The live tree passes every ported guard (mirrors the old green state)
- [ ] `find scripts -name 'test-*.sh'` returns nothing (all 14 gone)
- [ ] `_EXPECTED_CONFIG_SUITES` / `config` integration task removed; `mise run test:integration` green
- [ ] Bare default green end-to-end: `mise run`

#### Manual Verification

- [ ] The conformance port's validator-driven assertions still exercise the real
      `corpus frontmatter validate` binary, not a reimplementation.
- [ ] The design-structure appendix module reads as a coherent standalone guard.

---

## Phase 8: Hooks floor to zero

### Overview

Port the two non-detection guards `hooks/test-vcs-detect.sh` uniquely carries,
delete the suite and the dead regenerator, and bring the hooks floor to zero.
VCS-detection coverage is fully mirrored in `cli/vcs-adapters` and `cli/vcs-cli`.

### Changes Required

#### 1. Port to pytest — `tests/integration/hooks/`

The hooks integration task already runs `uv run pytest tests/integration/hooks`.
Add two guards:

- **Launcher-dispatch smoke** (from `test-vcs-detect.sh:210-238`): invoke
  `bin/accelerator vcs detect --format=hook --fail-safe --descriptive` in a
  plain non-repo dir; assert exit 0, **empty stderr**, valid-JSON-or-empty
  stdout, none of the three boundary-prohibition phrases present, and the
  `WORKSPACE BOUNDARY DETECTED` header absent (the shell case asserts both).
- **`hooks.json` registration integrity** (from `:326-343`): select the
  SessionStart entry by its command string; assert empty matcher, exactly one
  hook, `type == command`.

#### 2. Delete

- `scripts/hooks/../hooks/test-vcs-detect.sh` (the suite)
- `hooks/test-fixtures/vcs-detect/regenerate.sh` (dead — invokes the removed
  `hooks/vcs-detect.sh`)

**Keep** `hooks/test-fixtures/vcs-detect/*.json` — read by
`cli/vcs-cli/tests/detect_goldens.rs:24-30`.

#### 3. Lockstep

- `tasks/test/integration.py:57` — remove `_EXPECTED_HOOKS_SUITES` and the
  `run_shell_suites(context, "hooks", …)` call + `_require_suite_floor` in the
  `hooks` task; keep the task's `uv run pytest tests/integration/hooks` line.
- `tests/unit/tasks/test_integration.py` — mirror the floor removal.

### Success Criteria

#### Automated Verification

- [ ] The two ported guards pass: `mise run test:integration:hooks`
- [ ] `find hooks -name 'test-vcs-detect.sh' -o -name 'regenerate.sh'` returns nothing
- [ ] `detect_goldens.rs` still reads its four goldens and passes: `mise run test:unit:cli`
      (the lane compiles with `--all-features`, so its `bash-parity` gate is a
      live gate, not a skip — the goldens run in the bare default)
- [ ] Bare default green: `mise run`

#### Manual Verification

- [ ] The launcher-dispatch guard exercises the real `bin/accelerator` wrapper
      end-to-end (not the sub-binary directly).

---

## Phase 9: Bashisms denylist → Python task

### Overview

Reimplement `lint-bashisms.sh` as in-process Python inside the existing
`bashisms` task, then delete the shell script. No shell tool lints shell.

### Changes Required

#### 1. Replace the task body — `tasks/lint/scripts.py:68-79`

Swap the `context.run("bash scripts/lint-bashisms.sh …")` shell-out for
in-process scanning over `shell_sources()`. Preserve the fail-closed
`_EMPTY_SCOPE` guard already in the wrapper (the shell script fails *open* on
empty scope; the Python task must stay fail-closed).

Reproduce the eight denylist patterns exactly, translating **from the awk
source verbatim, not from the summary table below**. Four correctness hazards,
each of which silently weakens the only guard enforcing the 3.2 floor on the
survivors (shfmt and ShellCheck do not check bash version):

1. **POSIX classes.** The source uses four distinct classes; map each to an
   explicit ASCII range and compile with `re.ASCII` (never `\w`/`\d`/`\s`, which
   admit `_` / Unicode digits / Unicode whitespace the C-locale awk does not):
   `[[:alpha:]]`→`[A-Za-z]` (nameref, letters only), `[[:alnum:]_…]`→
   `[A-Za-z0-9_…]` (the mapfile boundary `[^[:alnum:]_]` and case-modification
   `[[:alnum:]_\[\]@*]` both include `_`), `[[:digit:]]`→`[0-9]`,
   `[[:space:]]`→`[ \t]` (a deliberate narrowing — C-locale `[[:space:]]` also
   matches `\f`/`\v`, but a bash keyword separated from its flag by a form-feed
   is not realistic; document the choice so it does not read as an oversight).
2. **Search mode and granularity.** awk `~` is an unanchored per-record search:
   apply `re.search` (not `re.match`/`fullmatch`) line-by-line, splitting on
   `"\n"` (`text.split("\n")`) to match awk's `RS="\n"` — **not** `splitlines()`,
   which also breaks on `\f`, `\v`, `\x1c-\x1e`, `\x85`, and the U+2028/U+2029
   line/paragraph separators, splitting one awk record into two and shifting
   anchors and line numbers.
   `re.match` anchors at column zero (no pattern anchors there → every line
   passes); whole-file text changes the `(^|…)`/`$` anchors.
3. **File decoding.** Read every source with explicit `encoding="utf-8"` (the
   scripts contain em-dashes); a bare `read_text()` uses the locale default and
   raises `UnicodeDecodeError` under a forced `LANG=C`, aborting the lint on a
   valid file.
4. **Comment-strip and opt-out fidelity.** Reproduce the naive trailing-comment
   strip as `re.sub(r'(^|[ \t])#.*$', '', line, count=1)` (single substitution,
   quote-unaware — do not "improve" it), and test the `# lint-bashisms: ignore`
   opt-out against the raw unstripped line, matching awk's ordering.

Preserve first-match-wins ordering and the
`<file>:<line>: bash-4 construct: <msg>` report format. Add one golden fixture
per pattern — including an indented/mid-line offender (proves position
independence) and a near-miss — rather than relying only on the manual spot-check.

| # | Construct | Message fragment |
|---|---|---|
| 1 | `(declare\|local\|typeset)[ \t]+-A` | associative array |
| 2 | `(declare\|local\|typeset)[ \t]+-[A-Za-z]*n` | nameref |
| 3 | escaped brace in `${…:-…}` default | escaped brace |
| 4 | `mapfile`/`readarray` (word-bounded) | mapfile/readarray |
| 5 | `${var^^ ^ ,, ,}` case-modification | case-modification expansion |
| 6 | `&>>` | append-both redirect |
| 7 | `\|&` | pipe-both |
| 8 | `[-<digit>` | negative array subscript |

#### 2. Delete `scripts/lint-bashisms.sh`

#### 3. Lockstep

- `tests/unit/tasks/test_lint.py:11,59,75-81` — replace the shell-out
  assertions with tests of the Python scanner (fixture corpus of bash-4
  constructs flagged, conforming survivors passed).
- `tests/unit/tasks/test_bootstrap_coverage.py:19,30-32` — replace the
  `_BASHISMS.read_text()` discovery assertion with the Python task's scope check,
  asserting against the bashisms task's exposed scan set (not `shell_sources()`
  directly), so Phase 10's removal of `shell_sources()` does not move this seam a
  second time. The shfmt/shellcheck-discovery assertion (`:27`) still reads
  `shell_sources()` here and is repointed in Phase 10.

`lint-bashisms.sh` is an entrypoint, not a `SHELL_LIBRARIES` member — no
frozenset edit. It is a `.sh` in the walk, so its deletion shrinks
`shell_sources()` naturally.

### Success Criteria

#### Automated Verification

- [ ] The Python bashisms task flags every bash-4 construct on a captured golden
      corpus and passes the conforming survivors: `mise run test:unit:tasks`
- [ ] `mise run lint:scripts:bashisms:check` still guards the surviving shell
- [ ] `find scripts -name 'lint-bashisms.sh'` returns nothing
- [ ] Bare default green: `mise run`

#### Manual Verification

- [ ] The Python patterns match the retired awk denylist construct-for-construct
      (spot-check each of the eight against a known offender).

---

## Phase 10: Final retirement and rescope

### Overview

With `scripts/` holding only `test-helpers.sh`, retire the tree-walk guard
machinery, rescope shfmt/ShellCheck/bashisms to the explicit two-file survivor
list, remove the `check-scripts` CI job, document the survivors, and delete
`test-helpers.sh` last.

### Changes Required

#### 1. Rescope the shared scan set — `tasks/shared/sources.py`

Replace the `shell_sources()` tree-walk with an explicit constant:

```python
SURVIVING_SHELL_SOURCES = (
    "bin/accelerator",
    "hooks/launcher-link-refresh.sh",
)
```

Feed it to shfmt (`format/scripts.py`), ShellCheck and the Python bashisms task
(`lint/scripts.py`). Remove the now caller-less `shell_sources()`, `_keep`, and
`_EXTRA_SHELL_SOURCES`. **Retain `walk_files`** — it is a live shared traversal
(`claude_coupling.py:52` and `test_python_coverage.py:70` both call it), so only
its shell-specific callers go, not the function. Keep the fail-closed guard — a
non-empty two-file list satisfies it.

#### 2. Remove exec-bits and `SHELL_LIBRARIES`

- `tasks/lint/scripts.py` — delete the `exec_bits` task, the `SHELL_LIBRARIES`
  frozenset (now empty), `_FIXTURE_SEGMENT`, and `_sources_args`'s walk
  dependency.
- `mise.toml` — drop `lint:scripts:exec-bits:check` from `lint:scripts:check`.
- `tests/unit/tasks/test_exec_bits.py` — delete (the invariant is gone); update
  `tests/unit/tasks/shared/test_sources.py` to assert the two-file survivor
  list (drop the `shell_sources` cases; keep the `walk_files` cases).
- `tests/unit/tasks/test_format.py` (the four shfmt tests) and
  `tests/unit/tasks/test_lint.py` (**both** `TestShellcheckTask` and
  `TestBashismsTask`'s `shell_sources`-patching cases — e.g.
  `test_raises_on_findings`, `test_raises_on_empty_source_set`) —
  currently `mocker.patch.object(fmt/lint, "shell_sources", …)`; once the
  format/lint modules stop importing `shell_sources`, `patch.object` raises
  `AttributeError` (and a residual `import shell_sources` is an `ImportError` at
  collection). Rewrite every such case to drive the task off
  `SURVIVING_SHELL_SOURCES`. Grep `test_lint.py`/`test_format.py` for
  `patch.object(*, "shell_sources"` to enumerate the full set rather than a
  named subset.
- `tests/unit/tasks/test_bootstrap_coverage.py:13,27` — still imports and calls
  `shell_sources()` in `test_bootstrap_is_in_the_shfmt_and_shellcheck_discovery`
  (Phase 9 rewrote only the bashisms-discovery test). Repoint this assertion
  onto the task's exposed scan set (`SURVIVING_SHELL_SOURCES`, which contains
  `bin/accelerator`) in this atomic bundle, or the commit fails at collection
  time. Ideally Phase 9 already targets the task's scan set rather than
  `shell_sources()` directly, so the seam does not move twice.

#### 3. Remove the remaining floors

`tasks/test/integration.py` — remove `_EXPECTED_DECISIONS_SUITES`,
`_EXPECTED_GITHUB_SUITES`, and the `decisions`/`github` tasks (both guard zero
suites and reference nothing deleted); drop them from `mise.toml` and the
roll-up. Mirror in `test_integration.py`.

#### 4. Re-home the shell lane, don't drop it — `.github/workflows/main.yml`

The rescoped `scripts:check` (shfmt + ShellCheck + Python bashisms over the two
survivors) must keep running in CI — the project is CI-only with no pre-commit
hooks, so deleting `check-scripts` outright would leave `bin/accelerator` (the
launcher every skill invokes) with no bash-4/format/lint enforcement and no
release gate, defeating the ADR-0049 floor this plan claims to keep guarding
automatically.

The fold is **not automatic**: `check-build-system` runs `mise run build-system:check`,
whose `depends` list does not include `scripts:check` (`mise.toml`), and CLAUDE.md
deliberately separates the `scripts` component from `build-system`. Re-home it as
an **explicit additional step** in the `check-build-system` job —
`run: mise run scripts:check`, named (e.g. "Run script checks") so a shell-only
failure still attributes to the shell lane in the log — not a task-level
`depends` edge (which would cross the component boundary). `check-build-system` already provisions
shfmt/ShellCheck/Python via mise and needs no Rust, so it has everything the
rescoped lane requires. Then retire the `check-scripts` job and its `prerelease`
`needs:` edge (`:587`). No other edge references `check-scripts` (verified:
`:147,163,587`).

⚠️ **Branch-protection required-check.** Removing the `check-scripts` job deletes
the "Check scripts" GitHub status name. If it is a required status check in the
repo's branch-protection ruleset (an out-of-repo setting), PRs will hang forever
waiting for a status no job reports. This is a manual handoff step landed in
lockstep with the merge — see Manual Verification.

#### 5. Document the survivors — `tasks/README.md:78-110`

Replace the "Executable-bit invariant" section (its `test-vcs-detect.sh` and
`check-scripts` exemplars are now stale) with a **surviving thin-shell**
enumeration: `bin/accelerator` and `hooks/launcher-link-refresh.sh`, their
bash-3.2 constraint (ADR-0049), and that the Python bashisms task + shfmt +
ShellCheck guard exactly these two. `SURVIVING_SHELL_SOURCES` is the
authoritative single source; the README documents rather than co-defines it. Add
a test that each survivor path in the constant appears as a backticked token in
the README section (tolerating prose formatting), not a strict equality against
parsed prose — otherwise a non-substantive README edit red-fails the guard. In
the same survivor-list test, assert each survivor path is tracked-executable
(`0755`) — the one continuity property the removed exec-bit invariant provided
for `bin/accelerator`, whose executable bit every skill invocation depends on.

#### 6. Delete `scripts/test-helpers.sh` last

All consumers (every `scripts/test-*.sh`, `hooks/test-vcs-detect.sh`) are gone
by now. Drop its final `SHELL_LIBRARIES` entry as part of removing the frozenset.

### Success Criteria

#### Automated Verification

- [ ] `find scripts -name '*.sh'` returns nothing
- [ ] The two survivors are the exact scanned set (from the authoritative
      `SURVIVING_SHELL_SOURCES` constant); a test asserts each appears as a
      backticked token in the `tasks/README.md` section: `mise run test:unit:tasks`
- [ ] `scripts:check` still runs in a surviving CI job (shfmt + ShellCheck +
      bashisms exercised against the survivors in CI, not merely list-asserted)
- [ ] shfmt and ShellCheck each fail on a deliberately malformed copy of a
      survivor (proven able to fail, not merely exit 0)
- [ ] `check-scripts` gone; no dangling `needs:` edge:
      `grep -c check-scripts .github/workflows/main.yml` returns 0
- [ ] Repo-wide grep for every removed `scripts/` path resolves only to
      surviving/relocated locations across `skills/`, `hooks/`, `tasks/`,
      `tests/`, `cli/`, `.github/`, `.editorconfig`
- [ ] Bare default green end-to-end: `mise run`

#### Manual Verification

- [ ] The CI job graph has no dangling dependency (inspect the `prerelease`
      `needs:` list).
- [ ] `tasks/README.md` names exactly the two survivors with their ADR-0049
      constraint; the stale exec-bit exemplars are gone.
- [ ] Branch-protection required-checks updated in lockstep with the merge: drop
      "Check scripts", confirm "Check build system" is required (it now carries
      the shell lane).

---

## Testing Strategy

### Unit Tests

- **Rust**: the rescoped `extra_keys_mirror.rs`, `parity.rs`, and
  `doc_type_single_source.rs` each retain their non-shell case and pass in the
  default lane; `link_external_id` insert/overwrite covered by a new
  `cli/work-cli/tests/` integration test.
- **Python**: the eight ported guards under `tests/unit/tasks/` each carry
  synthetic negative fixtures (fail) + conforming fixtures (pass) + one
  live-tree assertion; the Python bashisms scanner carries a golden corpus of
  bash-4 constructs.

### Integration Tests

- **Hooks**: launcher-dispatch smoke + `hooks.json` integrity under
  `tests/integration/hooks/`.
- **Every phase**: `mise run` (bare default) is the end-to-end gate; run it to
  green before the next phase's commit lands.

### Manual Testing Steps

1. After Phase 1, dry-read the repointed SKILL.md WF-4 flow for coherence.
2. After Phase 7, confirm the conformance port drives the real validator binary.
3. After Phase 10, inspect the `prerelease` `needs:` list and `tasks/README.md`.

## Performance Considerations

None material. The rescope from a whole-tree walk to a two-file constant makes
the shell lint/format lane marginally faster; the Python bashisms port trades a
`bash` subprocess for in-process scanning (negligible).

## Migration Notes

No data migration. Each phase is an independently revertible commit; VCS revert
is the recovery path (destructive-op safety is the working tree's history, not a
dry-run flag).

## References

- Work item: `meta/work/0174-retire-shell-tooling-and-ci-guards.md`
- Grounding research: `meta/research/codebase/2026-08-28-0174-empty-scripts-and-retire-shell-tooling.md`
- ADRs: ADR-0048 (four-toolchain split), ADR-0049 (bash-3.2 floor)
- Parent epic: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Predecessors (all done): 0167, 0168, 0169, 0170, 0171, 0172, 0195, 0196,
  0197, 0211, 0212
- Jira cutover successor: `cli/work-cli/src/sync_author.rs:139-161`
- Retained goldens: `cli/vcs-cli/tests/detect_goldens.rs:24-30`
