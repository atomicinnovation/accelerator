---
type: "work-item"
id: "0299"
title: "Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069"
date: "2026-10-05T10:02:05+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "task"
priority: "medium"
relates_to: ["work-item:0226"]
external_id: "PP-881"
tags: ["cli", "architecture", "dependencies", "refactor"]
last_updated: "2026-10-05T20:03:17+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0299: Bring the CLI Workspace's Crate Dependencies into Line with ADR-0069

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Make every crate dependency in the `cli/` workspace satisfy ADR-0069's
dependency rules, and add a lint that keeps them satisfied. The work merges
`consent-adapters` into `config-adapters`, moves the launcher's tracking check
in-process, and replaces each adapter → adapter dependency with a port
injected at a composition root.

## Context

ADR-0069 gives every workspace crate a role (kernel, domain, adapter,
technical library, composition root, test support, bootstrap verifier) and
every context a kind (platform, shared, product). It sets eight rules for
which dependencies are allowed. Today's workspace breaks several of them, and
nothing at the crate-graph level detects it: `cargo-pup` checks `use` paths
and `cargo-deny` checks third-party crates.

Plan 0226 created `consent-adapters` to keep `gix` and `jj-lib` out of
`config-adapters`' dependents. It also made the launcher answer its
SessionStart tracking question by running `vcs tracking --path` as a captured
child process. That needs a process-capture subsystem, and it adds an
`Unchecked` state for a cold first session in which the `vcs` binary cannot be
fetched in time. ADR-0069 rule 7 rules out a crate boundary kept for packaging.
Rule 6 allows the launcher to use platform contexts such as `vcs`.

The violations, by rule:

| Dependency | Rule | Used for |
|---|---|---|
| `corpus-adapters` → `vcs-adapters` | 4 | `facts` for metadata derivation |
| `work-adapters` → `vcs-adapters` | 4 | `InProcessProbe`, `facts` for working-copy status and author |
| `research-adapters` → `corpus-adapters` | 4 | frontmatter validation, `parse`, `FileCorpusStore` |
| `jira-client` → `corpus-adapters` | 4 | `acquire`, `LockOptions` file lock |
| `linear-client` → `corpus-adapters` | 4 | `acquire`, `LockOptions` file lock |
| `migrate-adapters` → `config-adapters` | 4 | `FileConfigStore`, `Composed`, `LegacyPolicy` |
| `migrate-adapters` → `vcs-adapters` | 4 | VCS facts |
| `migrate-adapters` → `corpus-adapters` | 4 | corpus store, walking, frontmatter validation |
| `migrate-adapters` → `work-adapters` | 2, 4 | sync baseline paths, store and digests |
| `consent-adapters` (whole crate) | 4, 7 | packaging boundary; `vcs-adapters` for tracking and repository roots |
| `migrate` → `document` | 1 | frontmatter parse and canonical render in `m0001`, `m0008` |
| `work` → `config` | 1, 3 | nothing: the edge is dead |
| `launcher` → `tracker-support` | 6 | `<tracker>.pull`/`.push` validation in the `config` dump |

## Requirements

1. Each workspace crate declares its ADR-0069 role and context in its
   manifest's package metadata. Each shared context's domain crate also
   declares its closed set of downstream contexts there.
2. A lint over `cargo metadata`, part of `mise run check`, rejects every
   normal or build dependency that ADR-0069 forbids. It names the crate, the
   dependency and the rule broken. It exempts test support and the bootstrap
   verifier. Both registration checklists in
   `tasks/README.md` gain the declaration step.
3. The shipped workspace passes the lint with no exception list.
4. The `vcs` domain publishes the tracking questions (file tracking, and the
   roots of the enclosing repositories) as a port, and `vcs-adapters`
   implements it. The consent adapters move into `config-adapters`, which
   depends on `vcs` and not on `vcs-adapters`. Each composition root injects
   the implementation, and the `consent-adapters` crate is removed.
5. The launcher injects the in-process tracking implementation. The captured
   `vcs tracking --path` child, the `vcs tracking` subcommand,
   `kernel::tracking`, and the `Unchecked` tracking state are removed.
6. The launcher no longer depends on `tracker-support`: `config`'s catalogue
   describes the `<tracker>.pull` and `<tracker>.push` blocks, and
   `tracker-support` derives its parsers from that description. The launcher
   no longer depends on `store`, because `TEMP_PREFIX` moves to `kernel`.
7. Each adapter → adapter dependency is replaced, either by a port of the
   upstream context injected at a composition root or by moving a
   context-free primitive into a technical library.
8. `migrate` reaches frontmatter parsing and canonical rendering through a
   port on `MigrationContext`, implemented in `migrate-adapters` over
   `document`. The dead `work` → `config` dependency and its `pup.ron`
   permission are removed.
9. The remaining domain → domain dependencies (`work` → `corpus`, `work` →
   `tracker`, `migrate` → `corpus`, `collaboration` → `vcs`) are each either
   justified under ADR-0069 rule 3 or removed.
10. The launcher's binary size and warm-dispatch latency are measured before
    and after the change, and the `deny.toml` symbol-count table is
    re-measured.

## Acceptance Criteria

- [ ] Given a fixture workspace with one forbidden dependency per rule (an
  adapter depending on another context's adapter, a domain crate depending on
  a technical library, any crate depending on a product context, the launcher
  depending on a shared context), when the lint runs, then it exits non-zero
  and names the crate, the dependency and the rule for each.
- [ ] Given a fixture shared context with one declared and one undeclared
  downstream, when the lint runs, then the declared downstream's adapter may
  depend on the shared context's adapter and the undeclared downstream's
  adapter may not.
- [ ] Given a crate with no role or context declared, when the lint runs, then
  it exits non-zero and names the crate.
- [ ] Given the shipped workspace, when `mise run check` runs, then the lint
  passes and no exception list exists.
- [ ] Given `cargo metadata` for the workspace, then `consent-adapters` is not
  a member, and no adapter crate depends on another context's adapter crate.
  The launcher's normal dependencies are limited to `kernel`, the crates of
  the `config`, `vcs` and `corpus` contexts, and technical libraries.
- [ ] Given a tracked `config.local.md` and no `vcs` sub-binary in the
  launcher cache, when the SessionStart config summary renders, then it warns
  about the tracked file exactly as today.
- [ ] Given `accelerator vcs tracking --path x`, when it runs, then it fails
  as an unknown subcommand. `TrackingCheck` no longer has an `Unchecked`
  state.
- [ ] Given valid, structurally invalid and unset `<tracker>.pull` and
  `<tracker>.push` blocks, when `accelerator config` dumps them, then the
  output and exit status match the pre-change behaviour.
- [ ] Given the launcher's binary size and warm-dispatch latency measured
  before and after with the same method, then both are recorded in the plan,
  and neither has grown 10× or more.
- [ ] Given the `deny.toml` symbol-count table, then it lists the launcher's
  measured `gix`/`jj-lib`/`uluru` presence and confirms the visualiser's.
- [ ] Given the four remaining domain → domain dependencies, then each is
  either justified under rule 3 or removed, with the outcome recorded in the
  plan.
- [ ] Given the finished change, when `mise run` runs, then it exits 0.

## Open Questions

- Can `config/src/catalogue.rs` describe a structured block with fields and a
  structural validator today? If not, the catalogue change in requirement 6
  may warrant its own work item.

## Dependencies

- Blocked by: acceptance of ADR-0069
- Blocks: none

## Assumptions

- Observable behaviour is preserved, except for removing the `vcs tracking`
  subcommand and the `Unchecked` state.
- `jira-client` and `linear-client` are adapters of the `jira` and `linear`
  product contexts. `tracker-support` is an adapter of the shared `tracker`
  context, which declares `work`, `jira` and `linear` as its downstreams, so
  the clients' dependencies on it are allowed under rule 4's shared-context
  exception.
- `remote-projection` is a technical library.
- `verify` is the bootstrap verifier. It and the `*-test-support` crates are
  exempt from the lint.

## Technical Notes

- **Tracking port:** something like `vcs::tracking::RepositoryTracking`,
  with `file_tracking` and `repository_roots`, implemented by an
  `InProcessTracking` in `vcs-adapters` over the existing `file_tracking`
  and `repository_roots` functions. `config-adapters`' `credential_ports`
  and `repository_roots` take the port as a parameter. The translation from
  `FileTracking` to `Tracking` and the merging of the config root move with
  them. The 8 call sites are `collaboration-cli`, `jira-cli`, `linear-cli`,
  `research-cli`, `work-cli`, `design-cli` (twice) and the launcher.
- **Repository facts:** `corpus-adapters`, `work-adapters` and
  `migrate-adapters` need `vcs_adapters::facts` and `InProcessProbe`. A
  repository-facts port in `vcs`, injected at each composition root, covers
  all three.
- **File lock:** `acquire` and `LockOptions` contain no corpus knowledge.
  Moving them into `store` removes the `jira-client` and `linear-client`
  dependencies without adding a port.
- **Sync baselines in migrations:** `migrate-adapters`' use of
  `work_adapters::sync::{baseline, baseline_store, digest}` becomes a port in
  `migrate` for refreshing sync baselines, which `migrate-cli` wires to
  `work-adapters`. Only the composition root names `work`.
- **Removals:** `cli/launcher/src/launch/outbound/capture.rs` and
  `tracking.rs`, `cli/vcs-cli/src/tracking.rs` with its test,
  `cli/kernel/src/tracking.rs`, and the `consent_adapters_is_zero_spawn` rule
  in `pup.ron`. The placement rule for `consent-adapters` in
  `tasks/README.md:761-764` goes too, along with that crate's licence-audit,
  `cargo-deny` and public-API registrations.
- **Lint precedent:** `tasks/lint/config_test_support.py` already reads
  `cargo metadata` to limit which crates enable a feature.
- **Duplicated field lists:** `dump.rs:85-180` repeats the Jira and Linear
  `pull` field lists that `tracker-support` holds. The catalogue description
  removes the duplicate.

## Drafting Notes

- `launcher` → `store` already satisfies rule 6, because `store` is a
  technical library. Moving `TEMP_PREFIX` to `kernel` is an agreed
  simplification, not a violation fix.
- One work item covers every violation, at the author's request. Planning may
  split it with `/refine-work-item`.
- The 10× threshold reflects the author's tolerance: launcher latency is
  measured, and avoided only if it grows by an order of magnitude.

## References

- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
- `meta/plans/2026-09-25-0226-unify-the-trust-barrier-for-consent-config-keys.md`
- Related: 0226
