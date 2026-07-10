---
type: work-item
id: "0178"
title: "config and config-adapters Crates with Native YAML Reader"
date: "2026-07-06T22:27:35+00:00"
author: Toby Clemson
producer: refine-work-item
status: done
kind: task
priority: high
parent: "work-item:0166"
external_id: PP-702
tags: [rust, config, corpus, store, crates, dedup]
last_updated: "2026-07-06T23:08:57+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0178: config and config-adapters Crates with Native YAML Reader

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Build the `config` (domain + application + ports) and `config-adapters`
(outbound: native `serde`/YAML frontmatter reader + filesystem) crates — the
native configuration reader that replaces the bash 2-level awk reader with
arbitrary YAML nesting, and the first-mover activation of the workspace's
cargo-deny / cargo-pup domain-boundary enforcement.

## Context

Child of 0166 — Shared config, corpus, and store Crates. ADR-0047 makes the
CLI the native config reader; this task delivers the config half of 0166's
crate layer. The existing bash reader (`config-read-value.sh`) caps nesting at
two levels and the visualiser's `config.rs` only deserialises a pre-baked
`config.json` — neither is reusable, so the YAML reader is built from scratch
against `serde`. The crates follow the hexagonal template the launcher already
establishes (`cli/launcher/src/main.rs` composition root wiring concrete
adapters to port traits).

## Requirements

- `config` crate: domain + application + ports, no outbound dependency. Model
  team→local last-writer-wins precedence and the recognised-key catalogue —
  the five groups (`paths.*`, `templates.*`, `work.*`, `review.*`, `agents.*`)
  plus the doc-type path-key arrays (`DOC_TYPE_NAMES` / `DOC_TYPE_PATH_KEYS`) —
  as domain concepts.
- `config-adapters` crate: native `serde`-based YAML frontmatter reader
  supporting arbitrary nesting (no 2-level cap), plus the filesystem adapter.
  This task delivers the crates plus one composition-root example demonstrating
  **Model 1** wiring — each sub-binary constructs its own `config-adapters` at
  its `main.rs` composition root, as opposed to a shared/global adapter registry.
  Wiring each actual consumer binary is owned by the respective consumer work
  item (0167 and 0169–0173), not this task. That composition-root example is the
  **config-reader entry point** the legacy-guard criteria below exercise.
- Fail-closed legacy guard: reimplement `config_assert_no_legacy_layout` — when
  `.accelerator/config.md` is absent but `.claude/accelerator.md` exists, exit
  non-zero directing the user to `/accelerator:migrate`. Do **not** reproduce the
  migration-internal `ACCELERATOR_MIGRATION_MODE` read fallback.
- Activate the currently-inert cargo-deny infra-out-of-domain ban
  (`cli/deny.toml`) and add the cargo-pup import-restriction rule for the
  `config` domain module (`cli/pup.ron`).

## Acceptance Criteria

- [ ] **Shared fixture suite** (the substrate the parity criteria bind to): a
      committed fixture set with at least one fixture per recognised key across
      the five groups plus the doc-type arrays, a team/local precedence-conflict
      case, and a default-fallback fixture for each key that declares a default
      — the three `WORK_DEFAULTS` keys (incl. `work.id_pattern` → `{number:04d}`)
      and the two inline-array `REVIEW_KEYS` defaults.
- [ ] For keys at nesting depth ≤2, the native reader resolves team→local
      precedence and every recognised key identically to the bash reader
      (`config-read-value.sh` over `config-common.sh` / `config-defaults.sh`),
      verified by a differential test over the shared fixture suite.
- [ ] For nesting depth ≥3 (which the bash reader cannot represent), the reader
      resolves keys to declared expected values on fixtures at a bounded depth,
      covering at least one 3-level scalar, one 4-level scalar, and one nested
      inline-array — each with a declared expected value — verified directly
      rather than by bash parity.
- [ ] Inline-array values (e.g. `review.core_lenses`, `review.disabled_lenses`)
      resolve to a typed sequence with the expected element list for a stated
      fixture, distinct from scalar-string resolution.
- [ ] Given a repo on the legacy `.claude/accelerator.md` layout (no
      `.accelerator/config.md`), when the config-reader entry point (the
      composition-root example) runs, then it exits with code 1 and prints to
      stderr a message containing the `/accelerator:migrate` directive rather
      than reading legacy config — at parity with `config_assert_no_legacy_layout`
      (`config-common.sh`).
- [ ] Given `ACCELERATOR_MIGRATION_MODE` set (=1) and a legacy
      `.claude/accelerator.md` layout, when the config-reader entry point runs,
      then it still exits with code 1 (fails closed, same as the previous
      criterion) — the migration-internal read fallback is deliberately not
      ported.
- [ ] The composition-root example (the config-reader entry point above)
      constructs the reader from concrete adapters, and the `config` domain layer
      carries no outbound dependency (enforced by cargo-pup).
- [ ] The cargo-deny infra-out-of-domain ban bites: a deliberately-violating
      canary — a `config`-domain crate importing the third-party YAML library
      (e.g. `serde_yml`) directly, bypassing the `config-adapters` wrapper — is
      confirmed to make cargo-deny exit non-zero (the ban's presence alone does
      not satisfy this).

## Open Questions

- Which YAML deserialiser crate backs `config-adapters` (`serde_yaml` is
  unmaintained; `serde_yml` is the maintained fork)? The choice fixes the
  cargo-deny `[[bans.deny]]` target.

## Dependencies

- Blocked by: 0166 crate-layer conventions (parent). The launcher
  version-hexagon (0164) and cli/ workspace skeleton (0163) supply the template
  mirrored here — both complete.
- Blocks: 0167 (built-in config command — needs only the config half, so it is
  unblockable the moment this task lands), and the sub-binary consumers whose
  composition roots wire `config-adapters` (0169–0173).
- Enforcement ownership: 0178 owns the first activation of `cli/deny.toml` and
  `cli/pup.ron`; siblings 0179/0180 extend those rules rather than re-activating
  them, so both are ordered after this task (they cannot extend rules that do not
  yet exist).
- Not blocked here: 0168 (visualiser refactor) consumes the corpus crates
  (0179/0180), not the config reader delivered here, so it is deliberately absent
  from Blocks.
- External: introduces one new third-party YAML crate under `config-adapters`
  (`serde_yaml`/`serde_yml` or equivalent) — the crate the cargo-deny
  `[[bans.deny]]` wrapper rule binds to; final choice is an Open Question.
- Parent: 0166.

## Assumptions

- The bash 2-level reader and the visualiser's JSON-reading `config.rs` are not
  reusable; the YAML reader is built from scratch against `serde`.

## Technical Notes

**Size**: M — two crates (`config`, `config-adapters`), a greenfield serde-YAML
reader with arbitrary nesting, porting the ~42-key catalogue across five groups
plus the two doc-type parallel arrays and inline-array parsing, the fail-closed
legacy guard, and first-mover activation of `deny.toml` + `pup.ron`. Well-bounded
and templated by the launcher's version hexagon, hence M rather than L.

- Source bash: `scripts/config-common.sh:55-67` (legacy guard),
  `scripts/config-read-value.sh:33-39` (2-level cap being removed),
  `scripts/config-defaults.sh:26-104` (recognised-key catalogue).
- Hexagon/composition-root template to mirror: `cli/launcher/src/main.rs:86-92`,
  `cli/launcher/src/launch/core.rs:174-202`, `cli/kernel/src/lib.rs:9-15`.
- Enforcement scaffolding to activate: `cli/deny.toml:67-73` (empty `skip`/
  `wrappers` waiting for this split), `cli/pup.ron:10-39` (per-domain
  `RestrictImports` pattern to extend).
- Full recognised-key catalogue spans two files: `config-defaults.sh` holds
  `PATH_KEYS` (17, `:26-44`), the doc-type parallel arrays `DOC_TYPE_NAMES` /
  `DOC_TYPE_PATH_KEYS` (13 each, `:74-83`), `TEMPLATE_KEYS` (6, `:85-92`),
  `WORK_KEYS` / `WORK_DEFAULTS` (3, `:94-104`; `work.id_pattern` default
  `{number:04d}`), and `WORK_INTEGRATION_VALUES` (`:110-115`, a value-domain
  constraint on `work.integration` rather than a separate key — excluded from the
  42-key count). The **review and
  agent keys live in `config-dump.sh`, not `config-defaults.sh`** — `REVIEW_KEYS`
  (9, `config-dump.sh:109-131`, two defaults are inline YAML arrays) and
  `AGENT_KEYS` (7, `config-dump.sh:134-152`, each prefixed `accelerator:`). Total
  42 keys across 5 groups + 2 doc-type arrays.
- Inline-array values (e.g. `review.core_lenses`) are parsed by
  `config_parse_array` (`config-common.sh:318-331`); serde typed sequences
  replace that string-splitting.
- Cleanest concrete hexagon to copy is the **version** slice (not launch):
  `cli/launcher/src/version/core.rs` (the `BuildMetadata` driven-port trait +
  `VersionReporter` service) with its outbound adapter
  `version/outbound/build_metadata.rs` (`VergenBuildMetadata`) — the exact shape
  `config-adapters` (serde/YAML + fs) takes. Map a domain `ConfigError` into
  `kernel::Error::Failed` via `From`, mirroring `launch/core.rs:167-171`.
- pup.ron structural difference: existing rules match modules *inside* the
  launcher crate (`^accelerator::version::core`); the config rule must match a
  **crate path (`^config::…::core`)** because `config` is a separate workspace
  crate. cargo-deny wiring: add a `[[bans.deny]]` for the YAML infra crate with
  `wrappers = ["config-adapters"]`, so a direct `config`-domain import violates.
- `serde` is already a workspace dependency (`cli/Cargo.toml:38-39`); a YAML
  crate (`serde_yaml`/`serde_yml` or equivalent) is **not** yet present — add it
  under `config-adapters` only.
- Legacy-guard exactness: fires only when the **team** file is absent and
  `.claude/accelerator.md` is present (the `.local` file is not part of the
  trigger); prints two stderr lines then `exit 1` (`config-common.sh:55-67`).

## References

- Parent: `meta/work/0166-shared-config-corpus-store-crates.md`
- ADRs: ADR-0047, ADR-0053
