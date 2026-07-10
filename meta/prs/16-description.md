---
type: pr-description
id: "16"
title: "[0178] Add config crates"
date: "2026-07-09T18:25:01+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
work_item_id: "0178"
parent: "work-item:0178"
relates_to: ["work-item:0166"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/16"
pr_number: 16
tags: [rust, config, crates, yaml, catalogue]
revision: "e4ee2c163a9ab4ad785e50d49d545235e790e453"
repository: "accelerator"
last_updated: "2026-07-10T12:09:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0178] Add config crates

## Summary

Builds the native Rust configuration reader that replaces the bash two-level awk reader (`config-read-value.sh`), delivering the `config` (domain + application + ports) and `config-adapters` (serde/YAML + filesystem) crates of the shared-crate layer (0166). It reads accelerator config from YAML frontmatter with arbitrary nesting, resolves personal-over-team precedence per key, and is proven at depth ≤2 against the bash reader by a differential harness. It flips on the workspace's first architectural domain-boundary gates (cargo-pup + cargo-deny), and reconciles the recognised-key catalogue with the full plugin config surface and its bash consumers — advertising every template, agent, and defaulted visualiser key — with new drift guards and config-dump visibility (credentials masked) so the catalogue and its consumers can no longer silently diverge.

## Changes

- **`config` domain crate** — serde-free core: an order-preserving `Node` value tree, dotted `Key`, two `Level`s (team/personal), an error taxonomy mapping into `kernel::Error`, and a `ConfigService` resolving per-key last-writer-wins precedence over arbitrary depth. Resolution projects a scalar leaf or all-scalar sequence to a typed `Value`, and treats a mapping node or mixed sequence as *found-empty* so a present-but-non-addressable value shadows a team value exactly as the bash reader does. Ships the recognised-key catalogue with typed defaults and the pure legacy-layout predicate.
- **`config-adapters` crate** — the outbound half of the hexagon: frontmatter splitting, the `serde-saphyr` document boundary mapping YAML to the typed `Node` tree, a filesystem store rooting the two config files at a discovered project directory (stopping at `.accelerator/`, `.git`, or `.jj`), the filesystem legacy guard, and the `render_value`/`render_resolved` projection shared by shipped output and the parity oracle.
- **Composition root + legacy guard** — a single tested `compose` helper that discovers the root once, runs the fail-closed legacy guard, and builds the store and service rooted at the same directory; plus a non-shipped `config-adapters-fixture` bin demonstrating the wiring. The legacy layout exits non-zero with the migrate directive (including under `ACCELERATOR_MIGRATION_MODE` and from git-/jj-rooted subdirectories); a normal layout resolves and exits zero.
- **Architectural enforcement gates (first-mover)** — a whole-crate cargo-pup rule restricting the `config` domain to `std`, `kernel::Error`, and crate-internal imports, and a cargo-deny wrapper ban making `serde-saphyr` reachable only through `config-adapters`. Both are proven by regression tests driving the real gate config (a committed serde-saphyr canary pair for cargo-deny, a probe crate named `config` for cargo-pup).
- **Test corpus** — a committed fixture suite and a differential harness proving depth-≤2 parity with the bash reader, plus direct declared-value assertions at depth ≥3, value-encoding divergences, fail-loud malformed cases, and characterised adversarial inputs (rejected cleanly by serde-saphyr's depth/node budgets).
- **Catalogue reconciliation** — brings the recognised-key catalogue into exact agreement with the shipped plugin and its bash consumers: all thirteen shipped templates are advertised (was six); the two work-item review-verdict keys and the two `browser-*` sub-agents — recognised by their consumers but missing from the catalogue — are added; and the `visualiser.kanban_columns` / `visualiser.idle_timeout` defaults are promoted into a catalogue group as their authoritative declaration (the visualiser server keeps a cross-referenced runtime fallback because it cannot depend on the crate). The catalogue now spans 55 defaulted keys across six groups, drift-tested key-for-key between the Rust catalogue and its bash mirror.
- **Config-surface visibility and drift guards** — `config-dump` now surfaces the previously-invisible `jira`/`linear`/`visualiser` sections in its effective-configuration table, with credential values (`*.token`, `*.token_cmd`) masked to presence-and-source only (never printed). A registry test pins those keys to both the configure docs and the consumer reads, and the agent catalogue is pinned to the `agents/*.md` files, so neither can silently drift again. Also corrects the stale `accelerator.md` / `accelerator.local.md` filenames the jira skill emitted to users (the pre-rename config filenames, which no longer exist), while leaving the legitimate legacy `.claude/accelerator.md` migration references intact; and de-flakes a timing-sensitive dev-server status test (fixed sleep → bounded poll on the reaped-child transition).
- Planning trail (research, plan, plan/work reviews) and the parent-epic decomposition of 0166 into 0178/0179/0180.

## Context

Implements work item [0178](../work/0178-config-crates-native-yaml-reader.md) — child of [0166 Shared config/corpus/store crates](../work/0166-shared-config-corpus-store-crates.md). ADR-0047 makes the CLI the native config reader. See the plan at `meta/plans/2026-07-07-0178-config-crates-native-yaml-reader.md` and its codebase research/review companions under `meta/research/` and `meta/reviews/`.

## Testing

- [x] `mise run check` — full read-only CI mirror green (exit 0) across all four toolchains, including the cargo-pup / cargo-deny domain-boundary gates.
- [x] `mise run test` green across three consecutive full runs, confirming the dev-server status de-flake holds under parallel-suite load.
- [x] `cargo test -p config -p config-adapters` — catalogue count (55) + bash↔Rust catalogue drift + resolution parity (now covering the visualiser default keys) green.
- [x] `scripts/test-config.sh` green, including the new drift guards — registry ↔ docs ↔ consumer reads, and agent catalogue ↔ `agents/*.md` — and the config-dump credential-masking regression (the raw token never appears in output).
- [x] Legacy-guard black-box test asserts non-zero exit + migrate directive under `ACCELERATOR_MIGRATION_MODE` and from git- and jj-rooted subdirectories.
- [x] bash-availability guard confirmed to fail (not silently pass) under CI with bash off `PATH`.

## Notes for Reviewers

- The core boundary claim to scrutinise: the `config` domain crate must stay serde/YAML/filesystem-free — the cargo-pup whole-crate rule and the cargo-deny wrapper ban exist to enforce exactly that, so the enforcement fixtures (`tests/integration/pup/`, `tests/integration/deny/`) are worth a close read alongside `pup.ron` / `deny.toml`.
- The found-empty semantics in `config/src/service.rs` (a present but non-addressable value shadowing a team value) is a deliberate bash-parity choice, not an oversight — the parity harness pins it.
- The catalogue reconciliation is guarded, not just corrected: the new tests in `scripts/test-config.sh` (registry ↔ docs ↔ reads, and agent catalogue ↔ agent files) are the mechanism that keeps the catalogue honest as agents/templates/keys are added — that is where the real regression protection lives.
- `config-dump` credential handling is security-relevant: `*.token` / `*.token_cmd` values are shown as `*(set — hidden)*` with their source, never printed; a test asserts the raw secret is absent from output.
- `visualiser.kanban_columns` / `visualiser.idle_timeout` defaults necessarily live in two places — the config catalogue (authoritative) and the standalone visualiser server crate (runtime fallback, since it cannot depend on `cli/config`) — with reciprocal comments; there is no automated cross-crate guard.
- This PR delivers the crates, one composition-root *example*, and the catalogue reconciliation only. Wiring each real consumer binary is owned by the respective consumer work item (0167, 0169–0173), not here.
- Sibling crates 0179 (corpus) and 0180 (atomic-store) are scoped in the same epic decomposition but are separate follow-up work.
