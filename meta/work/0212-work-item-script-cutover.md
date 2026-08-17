---
type: work-item
id: "0212"
title: "Work-Item Script Cutover"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0171"
blocked_by: ["work-item:0210"]
blocks: ["work-item:0211", "work-item:0174"]
relates_to: ["work-item:0194", "work-item:0213"]
tags: [rust, cutover, work-items, fixtures, cli]
last_updated: "2026-08-17T13:43:17+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0212: Work-Item Script Cutover

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Finish the cutover 0194 deliberately left undone: delete all eighteen
`work-item-*.sh` and `test-work-item-*.sh` files, relocate their fixture corpus
into the Rust test tree, convert the eleven parity tests to read those fixtures,
repoint the three work skills at `accelerator work …`, remove the work suite
floor, and consolidate the `E_DISPATCH_*` taxonomy on the `tracker` crate.

## Context

Second of the three sequenced children of 0171, and the first that deletes
anything. It runs **before** 0211 because the coupling between the work scripts
and the integration clusters runs in only one direction: four of the five suites
deleted here reach into `skills/integrations/{jira,linear}/scripts/test-helpers/`
and `.../test-fixtures/` for the Python mock servers and their scenario fixtures
— `test-work-item-create-remote.sh`, `-update-remote.sh`, `-fetch-remote.sh` and
`-sync-apply.sh`. The clusters, by contrast, invoke no `work-item-*.sh` at all;
the two references to `work-item-sync-label.sh` in `linear-create-flow.sh:304`
and `jira-resolve-fields.sh:140` are comments noting that those scripts perform
the same normalisation, not calls.

Deleting the work suites first therefore removes the only consumers of the
clusters' shared test assets, so 0211 can then retire both clusters whole. The
reverse order would break all four suites and the `_EXPECTED_WORK_SUITES` floor
the moment 0211 landed.

All eighteen files: thirteen production scripts (`work-item-bridge-codes`,
`-create-remote`, `-fetch-remote`, `-file-dirty`, `-normalise`,
`-project-remote`, `-push-decide`, `-sync-apply`, `-sync-baseline`,
`-sync-classify`, `-sync-decide`, `-sync-label`, `-update-remote`) and five
suites (`test-work-item-create-remote`, `-fetch-remote`, `-scripts`,
`-sync-apply`, `-update-remote`). `test-work-item-scripts.sh` goes in its
entirety, not merely its superseded sections.

## Requirements

- Delete all eighteen files. By the end of this child no `work-item-*.sh` or
  `test-work-item-*.sh` script remains.
- Repoint `skills/work/sync-work-items/SKILL.md`,
  `skills/work/create-work-item/SKILL.md` and
  `skills/work/list-work-items/SKILL.md` at `accelerator work …`, and re-site
  `skills/work/scripts/EXIT_CODES.md` — either rewritten in place for the Rust
  exit codes, or folded into the CLI's own docs, in which case
  `skills/work/scripts/` disappears entirely. Prefer the latter; see 0171's
  Open Questions.
- Preserve the dirty-work-item overwrite guard. `sync-work-items/SKILL.md:137`
  invokes `work-item-file-dirty.sh` as a precondition and `SKILL.md:312` pipes
  `work-item-project-remote.sh` through `work-item-normalise.sh --stdin`. The
  dirtiness precondition resolves to
  `cli/work-adapters/src/sync/working_copy_status.rs`, which reads dirtiness
  through `cli/vcs-adapters/src/library/dirty_paths.rs`; the normalisation pipe
  resolves to `cli/work/src/normalise.rs`. Deleting the guard without a
  replacement means overwriting a user's uncommitted work item.
- Relocate the whole of `skills/work/scripts/test-fixtures/` into the Rust test
  tree. Each fixture goes beside its consumer —
  `work-item-{push-decide,sync-classify,sync-decide,normalise,sync-label}` into
  `cli/work/tests/fixtures/`, `work-item-{project-remote,sync-baseline}` into
  `cli/work-adapters/tests/fixtures/`, the rest beside whichever Rust test reads
  it — and any fixture with no remaining consumer is deleted with its reason
  recorded. No bash reader survives, so nothing needs repointing.
- Convert all **eleven** Rust tests that resolve paths under
  `skills/work/scripts/` into pure-Rust tests reading their relocated fixtures:
  `cli/work/tests/sync_push_decide.rs`, `sync_classify.rs`, `sync_decide.rs`,
  `normalise_parity.rs`, `sync_label_parity.rs`,
  `cli/work-cli/tests/exit_codes_parity.rs`, `cli_diff_parity.rs`,
  `cli/work-adapters/tests/project_remote_parity.rs`,
  `sync_baseline_corpus.rs`, `sync_baseline_shellout_parity.rs` and
  `diff_shellout_parity.rs`. They are the oracles pinning the Rust engine to the
  bash one; deleting the scripts without converting them breaks the build, and
  deleting the tests instead discards the only guard against a projection or
  classification regression.
- Remove the work suite floor outright. `_EXPECTED_WORK_SUITES = 5`
  (`tasks/test/integration.py:51`) counts exactly the five deleted suites, so
  the floor and its `_require_suite_floor` call are removed rather than
  decremented. Remove `skills/work/scripts/work-item-bridge-codes.sh` from
  `SHELL_LIBRARIES` (`tasks/lint/scripts.py:18`).
- **Discharge the three port-less bridge capabilities** before deleting the
  scripts that carry them. Each has no `RemoteTracker` operation, so none
  survives the deletion by itself, and each is load-bearing today:
  - the unkeyed discovery `search` mode of `work-item-fetch-remote.sh`, which
    `/sync-work-items` uses to list remote issues with no local work item —
    `fetch_all(ids)` is key-scoped and cannot express it. This is **not** the
    provider `search` subcommand 0211 ships;
  - the create bridge's `--dry-run` field-resolution preview, which surfaces an
    unresolvable Jira project *before* the confirm gate;
  - the update bridge's `--dry-run` payload validation, which is what
    `/sync-work-items --preview` uses to validate every push against the live
    tracker today. 0194's `--preview` routes mutations to no-ops and makes no
    port call, so it does not discharge this.

  For each, implement the fate recorded in 0171's `## Decisions`: re-site it
  above the port, or drop it with the replacement outcome stated in observable
  terms. A capability needing a new port operation is out of scope — it becomes a
  new work item blocking this one, since 0204 is done and frozen. All three fates
  must be settled before pickup; see 0171's Open Questions.
- Leave the `tracker` crate as the single owner of the `E_DISPATCH_*` exit-code
  taxonomy. Delete `work-item-bridge-codes.sh` and
  `cli/tracker/tests/fixtures/dispatch-codes.txt` — the fixture exists only to
  pin the bash and Rust definitions to each other, so it has no purpose once one
  is gone.
- Update the doc comments and code comments in `tracker` that name deleted bash
  artefacts: `RemoteIssue.body`'s projection reference, `errors.rs`'s module
  doc, `show`'s `# Errors` note about the read bridge, and
  `RemoteTimestamp::Reported`'s note about the bash-written baseline. The
  contracts they state outlive the scripts; the references do not.

## Acceptance Criteria

- [ ] `ls skills/work/scripts/*.sh` matches nothing: all thirteen production
      scripts and all five `test-work-item-*.sh` suites are deleted, and
      `EXIT_CODES.md` is either rewritten for the Rust surface or re-sited with
      the directory removed. Under the rewritten-in-place branch, a test asserts
      each documented integer equals the value the CLI actually emits for that
      condition.
- [ ] Grepping the whole repository, excluding `meta/`, for each of the eighteen
      deleted basenames and for `skills/work/scripts` returns no hits. The grep
      command and its empty output are the recorded result — this is the
      consumer sweep, and it covers `hooks/`, `templates/`, `docs-site/`,
      `tasks/` and agent definitions, not only `skills/`.
- [ ] `sync-work-items/SKILL.md`, `create-work-item/SKILL.md` and
      `list-work-items/SKILL.md` each invoke `accelerator work …` for every flow
      they previously shelled out to.
- [ ] Given a work item with uncommitted local changes, when a pull would
      overwrite it, then the sync flow still refuses. Verified in two parts:
      statically, `sync-work-items/SKILL.md` invokes the named Rust surface at
      the same precondition point the bash guard occupied; behaviourally, a named
      automated test stages a fixture work item carrying an uncommitted edit
      alongside a remote-modified counterpart whose pull would otherwise apply,
      and asserts the file's bytes are unchanged and the refusal diagnostic
      emitted. Not recorded evidence of a manual run.
- [ ] `skills/work/scripts/test-fixtures/` no longer exists. Every fixture it
      held sits under a `cli/**/tests/fixtures/` directory beside its consumer,
      or is listed in 0171's `## Decisions` with the reason it has none.
      Relocated count plus enumerated deletions equals the pre-change file count
      that 0210's baseline artefact records as a committed number (so the
      comparison survives the directory's removal without VCS archaeology), and grepping `cli/` for `skills/work/scripts`
      returns no hits.
- [ ] All eleven shellout tests are pure Rust — `sync_push_decide`,
      `sync_classify`, `sync_decide`, `normalise_parity`, `sync_label_parity`,
      `exit_codes_parity`, `cli_diff_parity`, `project_remote_parity`,
      `sync_baseline_corpus`, `sync_baseline_shellout_parity` and
      `diff_shellout_parity` — reading committed fixtures from their own crate's
      test tree and invoking no bash script. Each covers at least the same
      fixture cases as 0210's recorded **fixture-case list**, with expected values
      byte-identical to the committed goldens. 0210 also records a pre-conversion
      assertion count per test; that figure is context for the comparison, not the
      bar, since a faithful pure-Rust rewrite may legitimately restructure
      assertions.
- [ ] Given a fixture-driven setup that creates one remote issue per relocated
      corpus record on the credentialed scratch Jira project and Linear team
      named in Dependencies, when `accelerator work
      sync` runs against them through the real clients, then every item
      classifies as `synced` and neither a push nor a pull is issued. The
      exercised set includes at least one item with an absent description per
      provider. (0210 carries the offline half of this guarantee.)
- [ ] Each of the three port-less bridge capabilities — unkeyed discovery
      `search`, the create `--dry-run` field-resolution preview, the update
      `--dry-run` payload validation — has its decided fate implemented, with the
      decision recorded in 0171's `## Decisions` naming the option taken.
- [ ] The behaviour behind each is verified against whichever option was taken,
      not merely recorded: `/sync-work-items` still lists remote issues with no
      matching local work item, and `/sync-work-items --preview` still surfaces an
      unresolvable Jira project key and a payload missing a required field before
      any mutation is issued — each emitting its named diagnostic. Where a
      decision was *drop*, it states the replacement outcome in observable terms
      and that outcome is verified in place of the original behaviour. No branch
      is discharged by prose alone.
- [ ] No work `SKILL.md` declares `jq` or `curl` in `allowed-tools`, and the full
      set of remaining declarers across `skills/` is enumerated and recorded in
      0171's `## Decisions`. The whole-repository **equality** assertion belongs
      to 0211, which lands last — the jira and linear skills still declare both at
      this child's boundary.
- [ ] `_EXPECTED_WORK_SUITES` and its `_require_suite_floor` call are removed
      from `tasks/test/integration.py`, no task references a deleted suite, and
      `work-item-bridge-codes.sh` is gone from `SHELL_LIBRARIES`.
- [ ] The `E_DISPATCH_*` taxonomy has one implementation:
      `work-item-bridge-codes.sh` and `cli/tracker/tests/fixtures/dispatch-codes.txt`
      are gone, and no surviving script or Rust test sources the removed
      definition.
- [ ] The four named doc and code comments in `tracker` no longer reference
      deleted bash artefacts, with the contracts they state preserved.
- [ ] `mise run` exits 0 end-to-end at this child's merge boundary.

## Dependencies

- Blocked by 0210 alone: the clients and the transcribed oracles. Nothing in this
  child needs 0211 — the work skills repoint at `accelerator work …`, not at
  `accelerator jira` or `accelerator linear`.
- **Blocks 0211.** Four of the five suites deleted here are the only consumers of
  `skills/integrations/{jira,linear}/scripts/test-helpers/` and
  `.../test-fixtures/` outside the integration suites themselves. Until they are
  gone, 0211's wholesale cluster deletion would break them and the
  `_EXPECTED_WORK_SUITES` floor. Verified by grep: the clusters invoke no
  `work-item-*.sh`, so the coupling is one-directional and this is the safe
  order.
- **Prerequisite, not discharged by this change**: the credentialed tracker
  target — a scratch Jira project, a Linear team and API tokens, plus repository
  secrets if 0171's secrets Open Question resolves to CI. It gates the corpus
  classification criterion below, exactly as it gates 0210's contract run. It has
  no work item of its own; 0171 names the owner. This child performs the
  irreversible deletions, so an unprovisioned target here strands the cutover
  half-migrated.
- **External systems**: Jira REST and Linear GraphQL. The corpus criterion runs
  through the real clients, so reachability, per-tenant rate limits and Linear's
  query-complexity cap bear on it; a red run is not automatically a defect in
  this change.
- Shares `skills/work/sync-work-items/SKILL.md` with 0213, which adds the
  conflict loop while this child changes the invocations. Different parts of the
  body; whichever lands second rebases onto the other.
- **Blocks 0174**: it cannot remove the bashisms linter, the exec-bit guard or
  the `SHELL_LIBRARIES` frozenset until this child's deletions land, and 0211's
  after them. 0174 owns only the fourteen repo-root `scripts/*.sh` entries. The
  0174 edge is held at child level — 0174's `blocked_by` names 0211 and 0212, and
  0171 records that its own edge is discharged by these children — so a blocker
  lookup from 0174 lands on the increments that do the work rather than on a
  parent that performs none.
- Consumes 0194's baseline corpus at `skills/work/scripts/test-fixtures/` and
  `accelerator work sync`. Confirm both artefacts exist rather than trusting
  0194's status field, which reads `ready` from this workspace: the corpus
  directory is populated, and `accelerator work sync --help` runs. If either is
  absent, add `blocked_by: [work-item:0194]` before planning — this child's
  deletions are irreversible and its corpus criterion depends on both.
- Parent: 0171.

## Assumptions

- ⚠️ **Unconfirmed, and it bounds this child's size**: the Rust surface already
  covers everything the three previously-deferred scripts do (`sync-label` →
  `cli/work/src/sync/label.rs`, `normalise` → `cli/work/src/normalise.rs`,
  `file-dirty` → `working_copy_status.rs` over `dirty_paths.rs`), so blanket
  deletion needs repointing rather than new behaviour. Confirm the dirty guard
  is reachable from the CLI before planning — if a replacement is missing, this
  child grows new behaviour.
- The committed goldens are a sufficient oracle once their bash generators are
  gone — no case in the corpus depends on regenerating it.

## References

- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Blocked by: `meta/work/0210-provider-client-crates-over-the-tracker-port.md`
- Blocks: `meta/work/0211-integration-binaries-and-bash-cluster-retirement.md`
- Related: 0194, 0174
