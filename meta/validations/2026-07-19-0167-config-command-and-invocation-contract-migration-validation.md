---
type: plan-validation
id: "2026-07-19-0167-config-command-and-invocation-contract-migration-validation"
title: "Validation Report: Built-in config Command and Invocation-Contract Migration"
date: "2026-07-22T16:32:47+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: "partial"
parent: "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
target: "plan:2026-07-19-0167-config-command-and-invocation-contract-migration"
tags: [rust, config, cli, skills, invocation-contract, allowed-tools, hooks, store, migration]
last_updated: "2026-07-22T16:32:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Built-in config Command and Invocation-Contract Migration

### Implementation Status

All eight phases are implemented and committed; every executable check is green.
A defined subset of the plan's own verification criteria remains unwritten (by
the author's own recorded account), and the release-pipeline and live-session
criteria cannot run in this environment. Result: **partial**.

- ✓ Phase 0: Bootstrap override + shim staging + release profile — implemented
  (`rzvpvonlwktp`, `rxpwknyoypst`). `split-debuginfo` dropped and
  `ACCELERATOR_CACHE_DIR` left ungated — both recorded as approved deviations in
  the plan.
- ✓ Phase 1: `store` crate + `atomic_write` consolidation — implemented across
  six commits. `store::current_umask()` hoist recorded as an approved divergence.
- ✓ Phase 2: Read subcommands, goldens, behaviour inventory — implemented across
  the scalar/agents/context/paths/dump/review/summary/template commit series.
- ✓ Phase 3: Write subcommands (`set`, templates, `init`) — implemented
  (`Add the config set write subcommand`, `Add config templates eject/diff/reset
  and config init`).
- ✓ Phase 4: Repoint shell suites at the binary — implemented; superseded suites
  now deleted (Phase 7).
- ✓ Phase 5: Call-site + `allowed-tools` cutover — increments A and B both
  committed and `mise run check`-green. 247 `!` sites + 14 non-`!` sites +
  28 shell consumers + 7 migrations repointed.
- ✓ Phase 6: `config-detect` hook — re-homed. **Approved deviation**:
  `hooks.json` not inlined; a thin `hooks/config-detect.sh` wrapper survives
  because the arg-splitting behaviour needs a live SessionStart to settle
  (deferred to 0169). `vcs-detect` kept at index 0.
- ✓ Phase 7: Deletion + green build — removal set (20 scripts), superseded
  suites, and shims all deleted; four `cli/` dependants repointed; counters
  moved; `PENDING_PHASE7` allowlist empty.

### Automated Verification Results

Run in this session against the workspace at revision `3acb17b6`:

- ✓ `mise run check` exits **0** — full read-only CI mirror green across all four
  components (frontend lint/format/types, server clippy, cli pup +
  store-duplication lint, scripts shellcheck).
- ✓ `mise run` (bare default) exits **0** — the full mirror including the entire
  test suite. The migrate integration suite alone reports `Passed: 536,
  Skipped: 0, Failed: 0`. The in-place `fix` pass produced **no reformatting
  diffs** (working tree clean apart from one pre-existing untracked research
  doc), confirming the committed tree is already format-clean.
- ✓ `bash scripts/check-call-site-migration.sh` exits 0 — Grep A-functional
  clean outside the (now-empty) Phase-7 allowlist; Grep B reports no removal-set
  `config-` script in any SKILL.md; `--allow-legacy-layout` confined to the
  migration engine. (21 Grep A-mention references reported, not gated — expected.)
- ✓ `bash scripts/check-skill-permissions.sh` exits 0.
- ✓ `bash scripts/check-inventory.sh` exits 0 — removal-set floor (20 files),
  22 divergence tests resolve, deletion ledger covers the removal set, member-4
  empty.

### Code Review Findings

#### Matches Plan:

- **Removal set fully deleted** — all 20 scripts (18 `config-*.sh` +
  `config-read-agent-name.sh` + `init.sh`) absent; the three retentions
  (`config-common.sh`, `config-defaults.sh`, `config-read-browser-executor.sh`)
  present, matching the "What We're NOT Doing" scope and Phase 7 §1.
- **Superseded suites and shims deleted** — `test-config.sh`,
  `test-config-read-doc-type-paths.sh`, `test-init.sh` gone;
  `scripts/test-shims/` removed.
- **`store` crate** — present in the workspace `members` list, with
  `atomic_write`/`WriteError`/`NewFileMode`/`WriteBounds` in `lib.rs`, and
  `store-duplication:check` wired into `cli:check` and passing.
- **`config_command` hexagon** — `inbound/`, `core/`, `render/` layers present
  with per-family render modules (scalar/agents/context/paths/dump/review/
  summary/template/instructions), matching Phase 2 §2.
- **Exit-code contract** — `kernel::Error::Refusal(String)` present, mapping the
  subcommand-scoped refusal to exit 2 per divergence 4.
- **Retained parity divergence tests** (10–12) present in
  `config-adapters/tests/parity.rs`
  (`malformed_frontmatter_is_fail_loud_where_bash_degrades`,
  `value_encodings_resolve_to_their_declared_divergent_forms`,
  `inline_and_nested_arrays_resolve_to_typed_sequences`), with the differential
  shell-out oracle (`oracle`/`require_bash`/`scripts_dir`) removed as planned.
- **Inventories present and internally consistent** — behaviour inventory,
  deletion ledger (final-state gate column filled), suite audit, divergences,
  removal-set list, removal-set-references audit + generator.
- **Hook wrapper** matches the approved Phase 6 deviation exactly.
- **`non-mapping-root` refusal** is tested
  (`set_refuses_a_non_mapping_root_and_leaves_it_byte_identical`), matching
  Phase 3.

#### Deviations from Plan (all recorded and approved in the plan text):

- Phase 0: `split-debuginfo` dropped (inert without `debug > 0`);
  `ACCELERATOR_CACHE_DIR` left ungated (always hash-verifies the staged shim);
  content address keys on the source shim's digest, not
  `bin/accelerator-verify.vendored.sha256`.
- Phase 1: umask query hoisted into a shared `store::current_umask()`;
  `libc` replaced by `rustix`.
- Phase 5: the Grep A-functional criterion was satisfied via a `PENDING_PHASE7`
  allowlist during the migration (now empty at the final state).
- Phase 6: `hooks.json` not inlined; thin wrapper retained.

#### Potential Issues:

- **Checkbox bookkeeping drift in the plan.** Several Phase 2 boxes are left
  `[ ]` for features that are in fact implemented and tested elsewhere — e.g.
  the `non-mapping-root` fixture (Phase 2 line 1727 unchecked, but Phase 3 line
  1920 checked and the test exists) and the read-side `--allow-legacy-layout`
  criterion (line 1729 unchecked, but the flag is wired in `cli.rs`/`main.rs`
  and exercised in `config_read.rs`). These are stale checkboxes, not missing
  work, but they make the plan's own status hard to read at a glance.
- **No committed `--help` snapshot test.** `cli/launcher/tests/help.rs` asserts
  help *content* by substring, not against a committed snapshot (Phase 2 line
  1651). The matched-subcommand-help behaviour (line 1714) is separately tested.
- **No `install_crypto_provider` negative test.** The pup module rule
  (`config_command_may_not_import_adapters_or_launch`) plus the lazy resolver
  structurally prevent a built-in from reaching it, but no explicit test asserts
  `config path`/`version` skip it — the author records this as remaining.

### Manual Testing Required:

These are the plan's own criteria that cannot be exercised in this environment.

1. Live-session Claude Code checks (Phase 5 / Phase 6):
  - [ ] `/accelerator:commit` and one integration write skill produce **no new
    permission prompt** (the `vcs/commit` rule addition is the highest-risk edit).
  - [ ] `/accelerator:configure` loads, renders injected blocks, **and completes
    a create-or-edit write** end to end.
  - [ ] Live hook equivalence: `additionalContext` from a real SessionStart is
    byte-identical to `config summary --format=hook`'s parsed field; a
    no-config repo produces no blank context block.
  - [ ] Confirm `hooks.json`'s `command` field arg-splitting behaviour (the
    reason the thin wrapper was retained) — 0169's to resolve before inlining.

2. Release-pipeline gates (Phase 5 / Phase 7):
  - [ ] Signed, checksum-verified launcher artefacts exist for all four triples
    **for a release carrying `config`** (0165 entry precondition).
  - [ ] Pre-release aggregate latency budget (≤25% over bash) against the signed
    release over the post-batching shipped shape, at a fixed working directory.
  - [ ] Standing release gate refusing a version bump without all four triples,
    and the publish reordered so an upload failure cannot half-publish `main`.

3. Remaining automated verification the author records as not-yet-built:
  - [ ] Per-migration failing-stub proofs — a stub failing on the k-th config
    call, for every k, makes the migration exit non-zero AND stay out of
    `migrations-applied`.
  - [ ] Deletion-ledger cross-commit **replay** script with its own
    known-positive floor.
  - [ ] Launcher-size regression check in `mise run check`.
  - [ ] `config-path-customised` fixture pinning the eject vs diff/reset
    resolution asymmetry (Phase 3 §2).

### Recommendations:

- **Reconcile the plan's checkboxes** with the implemented state before merge, or
  add a closing progress note that supersedes the per-line boxes — the current
  mix of `[x]`/`[ ]`/`[~]` understates completeness (features are done, boxes
  lag).
- **Land the remaining low-cost automated gates** the author already scoped: the
  `install_crypto_provider` negative test, the launcher-size regression check,
  and the deletion-ledger replay. These guard the plan's one irreversible risk
  (deletion) and the built-in/HTTP boundary; leaving them unbuilt means those
  properties currently rest on structure rather than a failing test.
- **Sequence the release-gated items behind 0165** as the plan already
  specifies: the signed-artefact entry precondition and latency budget must be
  satisfied before the release that carries the call-site flip to users. The
  mixed bash/`accelerator` state is safe on `main` until then.
- **Re-examine the recorded residual security risk** (Phase 3 §1 / Phase 5 §1):
  `config set` becoming prompt-free in ~25 read-only skills that ingest
  attacker-controllable PR/issue text is an accepted decline, not a closed
  concern. The closing lever — split the grant into a read-only rule enforced by
  `check-skill-permissions.sh` — is worth scheduling rather than leaving open.
