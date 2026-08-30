---
type: "plan-validation"
id: "2026-07-23-0168-fold-visualiser-into-cli-workspace-validation"
title: "Validation Report: Fold the Visualiser into the cli/ Workspace"
date: "2026-07-25T23:26:04+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-07-23-0168-fold-visualiser-into-cli-workspace"
target: "plan:2026-07-23-0168-fold-visualiser-into-cli-workspace"
tags: ["rust", "visualiser", "cli", "launcher", "corpus", "workspace"]
last_updated: "2026-07-25T23:26:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Fold the Visualiser into the cli/ Workspace

### Implementation Status

- ✓ **Phase 1: Relocate + Become a Workspace Member** — Fully implemented
- ✓ **Phase 2: Refactor onto the Shared Crates (Parity-Verified)** — Fully
  implemented (one documented deviation: `frontmatter.rs` retained)
- ✓ **Phase 3: Re-home Orchestration into `accelerator visualiser`** — Fully
  implemented (several documented, within-intent deviations)
- ✓ **Phase 4: Producer Wiring + Local-Manifest Verify** — Fully implemented
- ✓ **Phase 5: Distribution Cut-over (Deletions)** — Fully implemented (deviations
  are consequences of the post-plan "unify to one asset" decision)

All five phases are complete and landed across 19 commits (`0e15a0e9685c` →
`ea569e18ef33`). Every automated success criterion is `[x]`; three criteria are
marked `[~]` in the plan, each a documented supersession where the intent is met
by an alternative mechanism (not a failure — see Deviations).

### Automated Verification Results

Run at working-copy revision `d24d70ab1786` (parent `ea569e18`), tree clean.

- ✓ Read-only CI mirror passes: `mise run check` (exit 0) — format + lint +
  types across frontend, server, cli, build-system, and scripts
- ✓ Python task unit suite passes: `mise run test:unit:tasks` (exit 0)
- ✓ Python task integration suite passes: `mise run test:integration:tasks`
  (51 passed, exit 0)
- ✓ CLI workspace unit suite passes: `mise run test:unit:cli`
  (576 tests, 576 passed, 0 skipped)
- ✓ Visualiser unit suite passes: `mise run test:unit:visualiser`
  (334 passed, 0 failed)
- ✓ Visualiser integration suite passes: `mise run test:integration:visualiser`
  (all binaries green — 334/330/23/18/9/7/… , 0 failed)

Not re-run in this validation (see Potential Issues): `test:e2e:visualiser`
(Playwright; the plan notes a known health-port flake) and the full heavy
`mise run` default gate end-to-end.

### Code Review Findings

#### Matches Plan:

- **Phase 1** — `cli/visualiser/server` is the 12th workspace member
  (`cli/Cargo.toml` `members` includes `"visualiser/server"`); the old
  `skills/visualisation/visualise/{server,frontend}` locations are gone; version
  inherits from `[workspace.package]` (`version = "1.24.0-pre.15"`, MSRV `1.90.0`).
- **Phase 2** — the duplicated domain slice is retired: `slug.rs`, `patcher.rs`,
  `typed_ref.rs`, `cluster_key.rs`, and the domain `src/docs.rs` are deleted;
  `gray_matter` and `serde_yml` are absent from the server `Cargo.toml`. The
  server's own wire-view `src/api/docs.rs` correctly stays (Phase 2 §4).
- **Phase 3** — `orchestration/` is decomposed by concern (`process.rs`,
  `lock.rs`, `state.rs`, `mod.rs`); `compose.rs` is internally decomposed
  (`resolve_doc_paths`/`resolve_templates`/`resolve_work_item`/`resolve_kanban`/
  `resolve_idle`). The Origin guard is hardened to an exact-host URI parse
  (`server.rs:658-664`, `matches!(uri.host(), Some("127.0.0.1" | "localhost"))`),
  not `starts_with`. SKILL.md invokes
  `accelerator visualiser --owner-pid $PPID ${ARGUMENTS:-start}`.
- **Phase 4** — `DISPATCHED_SUBBINARIES = ("visualiser",)`
  (`tasks/shared/paths.py:25`); the E2E-insecure byte-scan guard
  (`_assert_no_e2e_insecure`, `tasks/build.py:272`) matches the **full** symbol
  `ACCELERATOR_VISUALISER_E2E_INSECURE` (not the prefix) and is called in staging;
  the launcher manifest fixture is re-keyed on `visualiser`
  (`cli/launcher/tests/fixtures/manifest.example.json`); the coherence check
  `validate_dispatch_coherence` (`tasks/build.py:188`) is gated in
  `manifest.py:138`.
- **Phase 5** — the retired shell surface (`visualiser.sh`, `launch-server.sh`,
  `stop-server.sh`, `status-server.sh`, `write-visualiser-config.sh`),
  `bin/checksums.json`, and the bash CLI `cli/accelerator-visualiser` are all
  absent; the checksums producer/coherence readers (`create_checksums`,
  `update_checksums_json`, `_read_checksums_json_version`, `_read_cargo_toml_version`)
  are gone — only `_read_workspace_version` remains in `validate_version_coherence`;
  `launcher-helpers.sh` is deleted with `start_time_of`/`start_time_matches`
  inlined into the inventory-design Playwright `run.sh`.

#### Deviations from Plan (all documented in the plan's Implementation Progress):

- `frontmatter.rs` is **not** deleted (Phase 2 SC `[~]`): its `serde_yml` engine
  is retired, but its server-only presentation helpers
  (`title_from`/`body_preview_from`/`read_ref_keys`/`FrontmatterState`) remain.
  Within intent — the engine swap, not the file, was the goal.
- `compose.rs` is a single file with internal by-concern functions rather than a
  submodule directory. The plan's "decompose by concern" intent is satisfied
  structurally.
- `config_contract.rs` was retired in Phase 3 (not Phase 5) — Model-1 removal of
  `Config::from_path`/`config.json` forced it early; replaced by
  `compose_contract.rs`.
- The `validate_dispatch_coherence` binding moved Phase 3 → Phase 4 (both sides —
  the SKILL.md invocation and `DISPATCHED_SUBBINARIES` membership — must exist
  together to stay `mise run`-green; no release ships between the two commits, so
  the co-release invariant holds).
- A new `work_item_pattern` module was added (the `id_pattern`→scan-regex
  compiler existed only in bash; ported beside its sole Rust consumer, parity-
  tested against `work-item-pattern.sh`).
- Phase 4/5 "unify to one shared asset" (post-plan decision): the visualiser
  ships as a single `accelerator-visualiser-{platform}` asset referenced by both
  verification paths; the debug archive is repointed to the shared
  `dist/release` binary; `github.py` dropped its checksums-track reverify.
- Cross-upgrade reap (Phase 3 SC `[~]`) and idle-token semantics (Phase 3 SC
  `[~]`) are covered by equivalent current-format tests rather than a shell-era
  reconciliation test, because the shell write path was retired within the phase.

#### Potential Issues:

- **Full end-to-end `mise run` not re-run after Phase 5.** The plan's own Phase 5
  note states the heavy default gate (frontend + Rust builds/tests + E2E) "has
  not been run in full" after the deletions. This validation ran `mise run check`
  plus the Python, CLI, and visualiser unit/integration suites (all green), which
  covers everything Phase 5 touched (shell deletions + Python producer), but the
  Playwright **E2E** suite and the aggregate gate were not re-run here. Residual
  risk is low (Phase 5 is deletions-only in shell + Python, no server/frontend
  code change), but the gate should be run once before the release that carries
  these commits.
- **Phase 5 release-ordering gate is a process invariant, not a code fact.** Phase
  5 must land only after a release carrying Phase 4's producer wiring has shipped
  (so the live manifest resolves `visualiser` before the old fetch path is
  deleted). This cannot be confirmed from the tree — it must be verified against
  the actual release history before/when these deletions merge to a shipping line.

### Manual Testing Required:

Carried forward from the plan's unchecked Manual Verification items:

1. Phase 1 — Relocation integrity:
  - [ ] `jj`/`git` history follows the moved files (blame preserved)
  - [ ] `accelerator-visualiser --version` reports the workspace version
2. Phase 2 — Engine-swap fidelity:
  - [ ] Rendered library / kanban / related views are visually unchanged after
    the frontmatter engine swap (spot-check via the running visualiser)
3. Phase 3 — Lifecycle end-to-end:
  - [ ] `accelerator visualiser start` from a real repo opens a working
    visualiser; `stop` tears it down; `status` reflects each state
  - [ ] The idle window closes a genuinely-idle server after the configured
    timeout; `never`/`0` keeps it up
4. Phase 4 — Release profile:
  - [ ] A dry-run release build stages, signs, and lists the shared
    `accelerator-visualiser-{platform}` asset (manifest key `visualiser`) built
    with default (`embed-dist`) features only — the
    `ACCELERATOR_VISUALISER_E2E_INSECURE` switch is inert in the staged binary
5. Phase 5 — Release ordering:
  - [ ] Confirm a release carrying Phase 4's producer wiring has shipped (live
    manifest resolves `visualiser`) before these deletions merge to a shipping
    release line

### Recommendations:

- Run the full `mise run` (including `test:e2e:visualiser`) once before shipping
  the release that carries these commits, to close the Phase 5 full-gate gap the
  plan flagged.
- Work through the manual checklist above — the two highest-value checks are the
  Phase 2 visual spot-check (the engine swap is the subtlest behaviour change) and
  the Phase 3 live start/stop/status lifecycle from a real repo.
- Before merging Phase 5 to a shipping line, explicitly confirm the release-
  ordering invariant (a manifest-carrying release has shipped) — it is the one
  correctness condition not enforceable in-tree.
