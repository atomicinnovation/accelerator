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
tags: [rust, config, crates, yaml]
revision: "891d33d751a9446df6f3005564c38ee4b4ffa527"
repository: "accelerator"
last_updated: "2026-07-09T18:25:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0178] Add config crates

## Summary

Builds the native Rust configuration reader that replaces the bash two-level awk reader (`config-read-value.sh`), delivering the `config` (domain + application + ports) and `config-adapters` (serde/YAML + filesystem) crates of the shared-crate layer (0166). It reads accelerator config from YAML frontmatter with arbitrary nesting, resolves personal-over-team precedence per key, and is proven at depth ≤2 against the bash reader by a differential harness. It also flips on the workspace's first architectural domain-boundary gates (cargo-pup + cargo-deny).

## Changes

- **`config` domain crate** — serde-free core: an order-preserving `Node` value tree, dotted `Key`, two `Level`s (team/personal), an error taxonomy mapping into `kernel::Error`, and a `ConfigService` resolving per-key last-writer-wins precedence over arbitrary depth. Resolution projects a scalar leaf or all-scalar sequence to a typed `Value`, and treats a mapping node or mixed sequence as *found-empty* so a present-but-non-addressable value shadows a team value exactly as the bash reader does. Ships the 42-key recognised catalogue with typed defaults and the pure legacy-layout predicate.
- **`config-adapters` crate** — the outbound half of the hexagon: frontmatter splitting, the `serde-saphyr` document boundary mapping YAML to the typed `Node` tree, a filesystem store rooting the two config files at a discovered project directory (stopping at `.accelerator/`, `.git`, or `.jj`), the filesystem legacy guard, and the `render_value`/`render_resolved` projection shared by shipped output and the parity oracle.
- **Composition root + legacy guard** — a single tested `compose` helper that discovers the root once, runs the fail-closed legacy guard, and builds the store and service rooted at the same directory; plus a non-shipped `config-adapters-fixture` bin demonstrating the wiring. The legacy layout exits non-zero with the migrate directive (including under `ACCELERATOR_MIGRATION_MODE` and from git-/jj-rooted subdirectories); a normal layout resolves and exits zero.
- **Architectural enforcement gates (first-mover)** — a whole-crate cargo-pup rule restricting the `config` domain to `std`, `kernel::Error`, and crate-internal imports, and a cargo-deny wrapper ban making `serde-saphyr` reachable only through `config-adapters`. Both are proven by regression tests driving the real gate config (a committed serde-saphyr canary pair for cargo-deny, a probe crate named `config` for cargo-pup).
- **Test corpus** — a committed fixture suite and a differential harness proving depth-≤2 parity with the bash reader, plus direct declared-value assertions at depth ≥3, value-encoding divergences, fail-loud malformed cases, and characterised adversarial inputs (rejected cleanly by serde-saphyr's depth/node budgets).
- Planning trail (research, plan, plan/work reviews) and the parent-epic decomposition of 0166 into 0178/0179/0180.

## Context

Implements work item [0178](../work/0178-config-crates-native-yaml-reader.md) — child of [0166 Shared config/corpus/store crates](../work/0166-shared-config-corpus-store-crates.md). ADR-0047 makes the CLI the native config reader. See the plan at `meta/plans/2026-07-07-0178-config-crates-native-yaml-reader.md` and its codebase research/review companions under `meta/research/` and `meta/reviews/`.

## Testing

- [x] `mise run cli:check` — workspace rustfmt + clippy green (exit 0).
- [x] Full `mise run` verified green end-to-end at implementation time (recorded in the plan-complete commit), including the differential parity harness and the cargo-pup / cargo-deny gate regression tests.
- [x] Legacy-guard black-box test asserts non-zero exit + migrate directive under `ACCELERATOR_MIGRATION_MODE` and from git- and jj-rooted subdirectories.
- [x] bash-availability guard confirmed to fail (not silently pass) under CI with bash off `PATH`.

## Notes for Reviewers

- The core boundary claim to scrutinise: the `config` domain crate must stay serde/YAML/filesystem-free — the cargo-pup whole-crate rule and the cargo-deny wrapper ban exist to enforce exactly that, so the enforcement fixtures (`tests/integration/pup/`, `tests/integration/deny/`) are worth a close read alongside `pup.ron` / `deny.toml`.
- The found-empty semantics in `config/src/service.rs` (a present but non-addressable value shadowing a team value) is a deliberate bash-parity choice, not an oversight — the parity harness pins it.
- This PR delivers the crates and one composition-root *example* only. Wiring each real consumer binary is owned by the respective consumer work item (0167, 0169–0173), not here.
- Sibling crates 0179 (corpus) and 0180 (atomic-store) are scoped in the same epic decomposition but are separate follow-up work.
