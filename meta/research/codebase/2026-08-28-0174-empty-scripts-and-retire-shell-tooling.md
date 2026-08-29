---
type: codebase-research
id: "2026-08-28-0174-empty-scripts-and-retire-shell-tooling"
title: "Research: Empty scripts/ and retire shell tooling and CI guards (0174)"
date: "2026-08-28T00:37:08+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0174"
parent: "work-item:0174"
relates_to: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture", "codebase-research:2026-06-23-0136-shell-scripts-rust-cli-migration-surface"]
topic: "Empty scripts/ and retire shell tooling and CI guards"
tags: [research, codebase, shell, tooling, ci, cleanup, scripts, bashisms]
revision: "85f919af11c86a39fed31374591796d812713002"
repository: "accelerator"
last_updated: "2026-08-28T00:37:08+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Empty scripts/ and retire shell tooling and CI guards (0174)

**Date**: 2026-08-28T00:37:08+00:00
**Author**: Toby Clemson
**Git Commit**: 85f919af11c86a39fed31374591796d812713002
**Branch**: HEAD (jj working copy, build-system workspace)
**Repository**: accelerator

## Research Question

Ground every claim in work item 0174 against the live codebase: the guard-machinery
anchors, the `scripts/` disposition table, the live couplings that gate deletions, the
CI edges, and the data-file relocations — confirming what still matches and surfacing
where the story has drifted from the code.

## Summary

**The story is directionally correct and its central claims hold — but several of its
anchors, counts, and enumerations have gone stale, and three real gaps would bite an
implementer.** The `scripts/` surface is 28 `.sh` files (14 libraries + 14
`test-*.sh`), 5 data files, and 3 fixture trees; the Jira `config_upsert_frontmatter_field`
call is genuinely the sole live consumer of the four-file config chain; and every Rust
drift-oracle coupling exists as described.

Three findings change the shape of the work:

- ⚠️ **The Jira cutover target is not a callable command.** `link_external_id`
  (`cli/work-cli/src/sync_author.rs:139`) is a trait method reached only internally via
  `accelerator work sync`. The SKILL.md block is model-instruction text that must call an
  invokable subcommand — none exists for a standalone `external_id` writeback today.
- ⚠️ **The story omits `tests/unit/tasks/` from its lockstep and dangling-reference
  scope.** Five Python unit suites mirror the guard machinery (`test_exec_bits.py` alone
  lists every `SHELL_LIBRARIES` entry) and break in the same commits as the guards.
- ⚠️ **A hooks-floor / `test-helpers.sh` tension the audit never covered.** The lone
  hooks suite `hooks/test-vcs-detect.sh` (the `_EXPECTED_HOOKS_SUITES = 1` floor guards
  it) lives outside `scripts/` and sources `test-helpers.sh`, which is slated for
  deletion. Neither file's disposition is stated.

Beyond these, the floor and `SHELL_LIBRARIES` counts the story cites are one-too-high and
its `tasks/test/integration.py` line numbers are stale (0211/0212 already removed the work
and integrations floors). Details and exact line numbers below.

## Detailed Findings

### `scripts/` inventory — 28 `.sh`, 5 data files, 3 fixture trees

`find scripts -name '*.sh'` returns **28**: 14 libraries + 14 `test-*.sh` harnesses.

- **Libraries (14).** `config-common.sh`, `config-defaults.sh`, `atomic-common.sh`,
  `vcs-common.sh`, `fs-common.sh`, `hash-common.sh`, `log-common.sh`, `doc-type-table.sh`,
  `doc-type-inference.sh`, `accelerator-scaffold.sh`, `frontmatter-emission-rules.sh`,
  `frontmatter-fixtures.sh`, `test-helpers.sh`, `lint-bashisms.sh`.
- ⚠️ **`work-common.sh` does not exist under `scripts/`.** The story's Technical Notes
  list it among the fourteen `scripts/*.sh` `SHELL_LIBRARIES` entries; it was already
  removed. The frozenset has 13 entries (below), not 14.
- **Domain `test-*.sh` (5).** `test-atomic-common`, `test-vcs-common`, `test-hash-common`,
  `test-doc-type-inference`, `test-merge-move` — the ones mirror-tested in `cli/`.
- **Authoring/evals `test-*.sh` (9).** `test-format`, `test-hierarchy-format`,
  `test-lens-structure`, `test-boundary-evals`, `test-evals-structure`,
  `test-evals-structure-self`, `test-skill-frontmatter-conformance`,
  `test-skill-frontmatter-population`, `test-template-frontmatter` — the ones to port.
- **Data files (5).** `templates-schema.tsv`, `skills-schema.tsv`, `linkage-type-pairs.tsv`,
  `status-legacy-map.tsv`, `extract-work-items-cue-phrases.txt`.
- **Fixture trees (3).** `test-fixtures/config-read-review/` (3 goldens),
  `test-evals-structure-fixtures/` (5 subdirs, 8 JSON), `test-hierarchy-format-fixtures/`
  (3 subdirs, 6 `.md`). The story names only the first; the ported guards need the other
  two carried as pytest fixtures.

### Thin-shell survivors — two, not three

⚠️ **The "Playwright executor" is not shell.** It is
`skills/design/inventory-design/scripts/playwright/run.js` — JavaScript. `scripts/**/*.sh`
and `skills/**/*.sh` return no matches, and there is no Playwright `.sh` anywhere. The
genuine thin-shell survivors are two:

- `bin/accelerator` — the launcher bootstrap wrapper (`#!/usr/bin/env bash`), enumerated by
  `shell_sources()` via `_EXTRA_SHELL_SOURCES = ("bin/accelerator",)`.
- `hooks/launcher-link-refresh.sh` — the hook wrapper; sources nothing, self-locates its
  plugin root, does only symlink management.

This directly contradicts AC-4/AC-5/AC-6, which enumerate three survivors including a
Playwright executor. The `tasks/README.md` enumeration and the rescoped guard list should
name two shell files (plus, separately, the JS executor if it is to be tracked at all).

### Guard machinery — anchors, with stale counts corrected

`tasks/lint/scripts.py` (cited `:18,86,100` — **all still accurate**):

- **`SHELL_LIBRARIES`** — declared line 18, 13 members at lines 20-32. Of the story's
  targets, `config-common.sh` (24), `vcs-common.sh` (26), `doc-type-table.sh` (27),
  `doc-type-inference.sh` (28) are present.
- **`bashisms`** (68-79) runs `bash scripts/lint-bashisms.sh <shell_sources()>`; fail-closed
  on empty scope.
- **`exec_bits`** (82-130) — the exec-bit invariant. Its **stale-entry guard** (98-103,
  offender text at 100) requires every `SHELL_LIBRARIES` path to still be enumerated by
  `shell_sources()`: delete a library file without editing the frozenset and this lint
  fails. This is the coupling that forces the lockstep.
- **`shellcheck`** (52-65) over the same `shell_sources()` set.

`tasks/test/integration.py` — ⚠️ **story citations stale.** Only **four** floor constants
remain: `_EXPECTED_CONFIG_SUITES = 14` (line 38), `_EXPECTED_HOOKS_SUITES = 1` (57),
`_EXPECTED_DECISIONS_SUITES = 0` (58), `_EXPECTED_GITHUB_SUITES = 0` (59).
`_EXPECTED_WORK_SUITES` and `_EXPECTED_INTEGRATIONS_SUITES` are **already gone** (0212 and
0211 respectively) — the story's re-retirement of them, and its config floor of 15, are
both wrong: config is **14**.

⚠️ **A required-suite coupling the story misses.**
`_REQUIRED_CONFIG_SUITES = ("scripts/test-skill-frontmatter-conformance.sh",)` (line 48) is
passed to `_require_suite_floor` for the config task. That named suite is one of the nine
being ported. Porting it forces an edit to this tuple, not just a floor decrement — else
`_require_suite_floor` (81-87) raises on the missing required suite.

**The config floor is keyed off `scripts/` test discovery, not a "config cluster".** The
`config` task calls `run_shell_suites(context, "scripts", ...)` — discovery over all
`test-*.sh` under `scripts/`. There are 14 such harnesses and the floor is 14. So **every**
`test-*.sh` deletion (domain *or* authoring-guard port) decrements this one floor toward
zero; the story's framing of it as a config-cluster floor understates its reach.

`tasks/format/scripts.py:16` (shfmt), `tasks/shared/sources.py:113` (`shell_sources()`),
and `tasks/lint/scripts.py` (shellcheck, bashisms, exec_bits) are the **only** callers of
`shell_sources()`. Retiring the guards leaves `shell_sources()` caller-less; rescoping means
replacing the walk with an explicit two-file survivor list feeding all three consumers.

### The single live bash coupling — confirmed, but the cutover has no target

`skills/integrations/jira/create-jira-issue/SKILL.md:111-112` (exact) sources
`config-common.sh` and calls `config_upsert_frontmatter_field <work-item-file> external_id
<KEY>`, inside the WF-4 create-and-writeback step (model-instruction text, **not** a
`!`-preprocessor block). This fires only in work-item-file mode.

The chain is exactly four leaf files: `config-common.sh` sources `vcs-common.sh` (8),
`config-defaults.sh` (9), `atomic-common.sh` (10) unconditionally at load; none sources
further. `config_upsert_frontmatter_field` (`config-common.sh:244-310`) is a true upsert
(replace path 252-254, insert path 262-309, both fail-closed, both commit through
`atomic_write` from `atomic-common.sh:16`). A repo-wide grep confirms **no other live
consumer** across `skills/`, `hooks/`, `templates/` — every other hit is a test harness,
lint task, `CHANGELOG.md`, or `meta/` doc.

⚠️ **The Rust successor is not invokable.** `link_external_id`
(`cli/work-cli/src/sync_author.rs:139`) has the right semantics — reads the file, parses
frontmatter, `Mapping::set("external_id", …)` (upsert, 149-152), atomic write (155-159) —
matching AC. But it is a `LocalAuthor` trait method reached only through
`accelerator work sync` (main.rs:465 → run_sync → sync.rs:412 constructs the author). There
is no `accelerator`-binary subcommand that performs a standalone `external_id` writeback.
The SKILL.md cutover therefore cannot simply "call the Rust successor" — it needs a new
invokable command (the natural home is `accelerator jira create`, mirroring the resolver
already called at SKILL.md:60 and :102). This is a design decision the story treats as
settled but is not.

### Rust drift-oracle couplings — ordering matters (two are compile-time)

| Shell file | Consumer | Coupling | Gate | Breaks |
|---|---|---|---|---|
| `templates-schema.tsv` | `schema.rs:277` | `include_str!` | `#[cfg(test)]` mod | corpus **test build** compile-fail |
| `extract-work-items-cue-phrases.txt` | `cue_phrase_drift.rs:11` | `include_str!` | ungated (tests/ target) | design **test target** compile-fail |
| `config-defaults.sh` (EXTRA_KEYS) | `extra_keys_mirror.rs:18` | file read | **ungated** | ungated test at runtime |
| `config-defaults.sh` (PATH_KEYS) | `doc_type_single_source.rs:64` | bash source | `feature=bash-parity` | gated test only |
| `linkage-type-pairs.tsv` | `doc_type_single_source.rs:111` | file read | `feature=bash-parity` | gated test only |
| `doc-type-inference.sh` | `parity.rs:94` | bash spawn | `feature=bash-parity` | gated test only |

⚠️ **The two `include_str!` couplings force ordering:** the file must be relocated and the
path repointed *in the same change* that removes it from `scripts/`, or `cargo test`
compilation breaks for `corpus` / `design`. The runtime couplings are looser, but
`extra_keys_mirror.rs` is **ungated** — deleting `config-defaults.sh` fails a test that runs
on every `cargo test -p config`, so its removal must land with the config-chain cutover, not
before.

`doc_type_single_source.rs` splits as the story says: `every_non_virtual_type_is_registered_exactly_once`
(line 27) drives the compiled `accelerator config paths --doc-types` resolver via
`common/mod.rs:93` — bash-free, retained (de-gate from `feature = "bash-parity"`, file gate
at line 16). The other two cases are bash-data oracles, dropped.

`corpus::linkage::TYPE_PAIRS` (`cli/corpus/src/linkage.rs:60`) is a hand-maintained 16-row
const; nothing reads `linkage-type-pairs.tsv` at build or run time except the one gated
drift test. So the TSV is delete-outright once that test is dropped — confirmed.

### CI edges and config files

`.github/workflows/main.yml` — ⚠️ **`check-scripts` is at lines 147-163** (story's "~99" is
stale). It runs one command, `mise run scripts:check`. Exactly **one** `needs:` edge
references it: the `prerelease` release-gate job at **line 587**. Removing the job means
deleting 147-163 and that one edge. `main.yml` is the only workflow file.

- `.shellcheckrc` (root, 54 lines): `enable=all`, `external-sources=true`,
  `source-path=SCRIPTDIR`, plus eight documented `SCxxxx` disables. Retained and rescoped.
- `.editorconfig`: `[*.sh]` block at lines 33-39 (story cited 36-39; content there),
  `max_line_length = 80` at line 8. Retained.

### The story's dangling-reference scope is incomplete

⚠️ **`tests/unit/tasks/` is not in the story's enumeration but carries heavy references
that break in lockstep:**

- `tests/unit/tasks/test_exec_bits.py:245-257` mirrors the entire `SHELL_LIBRARIES` list.
- `tests/unit/tasks/test_measure.py:1293,1746,1754,1796` references the `vcs-common.sh` pin.
- `tests/unit/tasks/test_lint.py:11,59,85` and `test_bootstrap_coverage.py:19,31` reference
  `lint-bashisms.sh`.
- `tests/unit/tasks/test_call_site_migration.py:35` references `config-common.sh`.

The story's Requirement enumerating "no dangling reference in `skills/`, `hooks/`, `tasks/`,
`cli/`, `.github/` or `.editorconfig`" omits `tests/`. These suites must be updated in the
same commits, or the Python test lane goes red.

Minor inaccuracy: the story says `tasks/lint/call_site_migration.py` holds allowlist entries
for `config-common.sh`, `doc-type-inference.sh`, and `doc-type-table.sh`. Live has only two —
`config-common.sh` (allowed tuple, line 32) and `doc-type-table.sh` (legacy-flag exemption,
line 55). `doc-type-inference.sh` is **not** referenced there.

`tasks/measure.py` `RECOVERED_FILES` (977-986) pins `vcs-common.sh` at the **historical
baseline revision** (`BASELINE_COMMIT`), not the live file — it is a frozen provenance
record, so dropping the entry is safe but must land with `test_measure.py` edits.

## Code References

- `scripts/` — 28 `.sh`, 5 `.tsv`/`.txt`, 3 fixture trees (see inventory above)
- `tasks/lint/scripts.py:18` — `SHELL_LIBRARIES` (13 entries, 20-32); `:82-130` exec_bits,
  stale-entry guard `:98-103`; `:68-79` bashisms; `:52-65` shellcheck
- `tasks/test/integration.py:38,57,58,59` — the four surviving floors; `:48`
  `_REQUIRED_CONFIG_SUITES`; `:62-88` `_require_suite_floor`; config task `run_shell_suites(…,
  "scripts", …)`
- `tasks/format/scripts.py:16` — shfmt over `shell_sources()`
- `tasks/shared/sources.py:113` — `shell_sources()`; `:110` `_EXTRA_SHELL_SOURCES`
- `tasks/measure.py:977-986` — `RECOVERED_FILES` `vcs-common.sh` pin (baseline-frozen)
- `tasks/lint/call_site_migration.py:32,55` — allowlist entries (config-common, doc-type-table)
- `skills/integrations/jira/create-jira-issue/SKILL.md:111-112` — sole live bash coupling
- `scripts/config-common.sh:244-310` — `config_upsert_frontmatter_field`; `:8-10` chain sources
- `cli/work-cli/src/sync_author.rs:139-159` — `link_external_id` (upsert; not a CLI command)
- `cli/corpus/src/frontmatter_validation/schema.rs:277` — `include_str!` templates-schema.tsv
- `cli/design/tests/cue_phrase_drift.rs:11` — `include_str!` cue-phrases.txt
- `cli/config/tests/extra_keys_mirror.rs:18` — ungated read of config-defaults.sh
- `cli/corpus-adapters/tests/doc_type_single_source.rs:27,64,111` — three cases (keep one)
- `cli/corpus-adapters/tests/parity.rs:94` — doc-type-inference.sh bash oracle
- `cli/corpus/src/linkage.rs:60` — hand-maintained `TYPE_PAIRS` (16 rows)
- `.github/workflows/main.yml:147-163` — `check-scripts` job; `:587` sole `needs:` edge
- `hooks/test-vcs-detect.sh:39` — sources `test-helpers.sh` (the hooks-floor suite)
- `tests/unit/tasks/test_exec_bits.py:245-257` — SHELL_LIBRARIES mirror

## Architecture Insights

- **One lockstep coupling drives the whole story: the exec_bits stale-entry guard.** A
  deleted library that stays in `SHELL_LIBRARIES` fails the lint; a floor expecting a
  removed suite fails the test. Both are why the AC insists each deletion + decrement land
  in one independently-green commit.
- **The config floor is a `scripts/`-wide `test-*.sh` gauge, not a domain floor.** Every one
  of the 14 harnesses counts toward it, so the nine-guard port and five-domain-test deletion
  both decrement the same constant — clean seam, but it means the port is not floor-neutral.
- **`shell_sources()` is the single scope shared by shfmt, shellcheck, bashisms, and
  exec_bits.** Rescoping is therefore one edit (walk → explicit survivor list) that all four
  consumers inherit — the design deliberately keeps them from disagreeing.
- **Two `include_str!` couplings are the only hard compile-order constraints;** everything
  else is a runtime test that fails loudly but does not block the build.

## Historical Context

- `meta/decisions/ADR-0048-four-toolchain-split.md` (accepted) — the thin-wrapper floor this
  story drives toward.
- `meta/decisions/ADR-0049-bash-3.2-compatibility-floor.md` (accepted) — why the bashisms
  guard is re-homed rather than dropped.
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md` and
  `2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md` — the epic-level surface this
  cleanup follows.
- `meta/work/0136-migrate-shell-scripts-to-rust-cli.md` (in-progress) — parent epic; 0174 is
  its Phase 11 cleanup.
- `meta/work/0211-*` and `0212-*` (both done) — already removed the integrations and work
  floors and delivered the `sync_author.rs` writeback; their plans
  (`meta/plans/2026-08-19-0211-*`, `2026-08-19-0212-*`) are the closest precedents for the
  floor/guard lockstep discipline.
- `meta/notes/2026-07-13-bash-corpus-script-inconsistencies.md` — the one note on bash-corpus
  inconsistencies.
- **No plan and no dedicated research doc for 0174 existed before this one.**

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
- `meta/research/codebase/2026-08-17-0211-integration-binaries-and-bash-cluster-retirement.md`
- `meta/research/codebase/2026-08-19-0212-work-item-script-cutover.md`

## Open Questions

- ❓ **How does the Jira SKILL.md reach a Rust `external_id` writeback?** `link_external_id`
  is not an invokable command. Add a subcommand to `accelerator jira create`, or a standalone
  writeback command, or expose the `LocalAuthor` path — the story assumes a target that does
  not exist as a callable.
- ❓ **What is the disposition of `hooks/test-vcs-detect.sh` and `test-helpers.sh`?** The lone
  hooks suite (the `_EXPECTED_HOOKS_SUITES = 1` floor) sources `test-helpers.sh`, which is
  slated for deletion, and both sit outside the audited `scripts/` set. Bringing the hooks
  floor to zero implies deleting the suite, but neither file's fate is stated. The fixture
  regenerators `hooks/test-fixtures/vcs-detect/regenerate.sh` also read `vcs-common.sh`.
- ❓ **Two thin-shell survivors, not three.** AC-4/5/6 name a Playwright shell executor that
  does not exist (`run.js` is JavaScript). Confirm the survivor set is `bin/accelerator` +
  `hooks/launcher-link-refresh.sh`, and decide whether `run.js` is tracked separately.
- ❓ **`tests/unit/tasks/` lockstep.** Fold the five affected Python suites into the
  dangling-reference scope and the per-commit-green obligation.
