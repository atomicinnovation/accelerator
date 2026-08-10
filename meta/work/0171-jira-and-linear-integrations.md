---
type: work-item
id: "0171"
title: "Jira and Linear Integrations"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: medium
parent: "work-item:0136"
blocked_by: ["work-item:0187", "work-item:0203", "work-item:0194"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0170", "work-item:0194", "work-item:0174"]
tags: [rust, jira, linear, integrations, reqwest, sync]
last_updated: "2026-08-10T16:04:28+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-192"
---

# 0171: Jira and Linear Integrations

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Build the `jira-client` and `linear-client` adapter crates (`reqwest`/serde
replacing `jq`/`curl`, each implementing the `RemoteTracker` port) and the thin
`accelerator-jira` / `accelerator-linear` binaries over them, so the standalone
integration skills and the work-item sync engine share one implementation per
provider — and, because these clients are the last thing the Rust sync engine
needs to be usable, perform the work-item cutover: retire the nine migrated
sync and bridge scripts, repoint the work skills at `accelerator work …`, and
give `/sync-work-items` its conversational conflict flow.

## Context

`skills/integrations/jira/scripts/` (22 prod) and `linear/scripts/` (12 prod)
implement create/update/comment/transition/search/show/attach/init flows, ADF↔markdown
conversion, JQL/GraphQL, and auth, shelling out to `jq`/`curl`. Both have Python
mock HTTP servers for tests. Resolved Q2: provider clients are shared crates reused
by both the standalone binaries and the `tracker` sync engine (0194 — split off
from 0170 on 2026-08-05; the `tracker` crate and `RemoteTracker` port now live
there, not in 0170). May be split into separate Jira and Linear stories if finer
granularity is wanted.

## Requirements

- Implement `jira-client` (Jira REST + ADF↔markdown + auth) and `linear-client`
  (Linear GraphQL + auth) as adapter crates over `reqwest` + rustls + serde, each
  `impl RemoteTracker` (the port from 0203's `tracker` crate).
- Implement `accelerator-jira` and `accelerator-linear` as thin inbound CLI adapters
  exposing the user-facing flows (create/update/comment/transition/search/show/
  attach/init).
- Carry over the Python mock servers (`mock-jira-server.py`, `mock-linear-server.py`)
  as integration-test scaffolding for the Rust clients.
- Remove the `jq`/`curl` `allowed-tools` entries from the integration skills once
  migrated; confirm no other skill relies on them.
- Run 0194's shared `RemoteTracker` contract test — the harness it delivers,
  parameterised over implementations — against both real clients under the
  tagged, network-touching filter, so the fake 0194 verified against and the
  real clients are held to one contract.
- Reproduce the existing per-provider projection recipes exactly (jira —
  summary line then the description in Atlassian Document Format through
  key-sorted `jq -S`; linear — title line then Markdown description
  verbatim), verified against the bash-generated baseline corpus 0194
  commits under `test-fixtures/`. A projected body differing by even
  whitespace reclassifies every synced item as `remotely-modified` on the
  first run after the cutover.
- **Perform the work-item cutover**, which 0194 deliberately left undone because
  the flows cannot resolve a real tracker until these clients exist. In one
  change: remove `work-item-sync-{apply,baseline,classify,decide}.sh`,
  `work-item-{fetch,project,create,update}-remote.sh`,
  `work-item-push-decide.sh` and their `test-*.sh` suites plus the superseded
  sections of `test-work-item-scripts.sh`; repoint
  `skills/work/sync-work-items/SKILL.md`,
  `skills/work/create-work-item/SKILL.md`,
  `skills/work/list-work-items/SKILL.md` and `skills/work/scripts/EXIT_CODES.md`
  at `accelerator work …`; and decrement the work suite floor
  (`_EXPECTED_WORK_SUITES`, `tasks/test/integration.py`). Leave
  `work-item-sync-label.sh` and `work-item-normalise.sh` in place — their
  consumers sit outside the sync engine and retire under 0174.
- Add the conversational half of 0194's two-invocation conflict flow to
  `skills/work/sync-work-items/SKILL.md`: invoke `accelerator work sync`, parse
  its machine-parseable conflict report, render each conflict with enough local
  and remote context for the user to judge, collect a choice per item, and
  re-invoke with the matching `--resolve <id>=<remote|local>` orders. The binary
  is non-interactive by 0194's requirement, so until this lands no caller can
  resolve a conflict.
- Take single ownership of the `E_DISPATCH_*` exit-code taxonomy once the bash
  bridges are gone, deleting `work-item-bridge-codes.sh` and the fixture holding
  the two implementations in step.

## Acceptance Criteria

- [ ] `accelerator jira …` and `accelerator linear …` reproduce the standalone flows,
      verified against the repointed integration suites and the mock servers.
- [ ] Both client crates implement `RemoteTracker` and are consumable by
      `accelerator-work`'s sync engine (0194) with no duplication of API logic.
- [ ] No production `jq`/`curl` dependency remains for the migrated integration
      skills; their `allowed-tools` entries are removed.
- [ ] The integration suite floor is decremented in lockstep as the shell scripts
      are removed.
- [ ] 0194's shared `RemoteTracker` contract test passes against both real
      clients under the tagged filter, asserting round-trip `create` → `show`
      and whole-content `update` → `show`; the default
      `cargo test`/`cargo nextest run` invocation still makes no network call.
- [ ] Given the bash-generated baseline corpus 0194 committed, when
      `accelerator work sync` runs against matching remote records through the
      real clients, then every item classifies as `synced` and neither a push
      nor a pull is issued — so the cutover cannot mass-reclassify a user's
      synced items.
- [ ] The nine migrated scripts and their `test-*.sh` suites are removed, the
      superseded sections of `test-work-item-scripts.sh` are deleted, the four
      work-skill callers are repointed at `accelerator work …`, and
      `_EXPECTED_WORK_SUITES` is decremented — all in the same change.
      `work-item-sync-label.sh` and `work-item-normalise.sh` remain in place and
      their consumers still pass.
- [ ] Given a `sync` run reporting two conflicts, then
      `skills/work/sync-work-items/SKILL.md` renders both with their local and
      remote context, collects a choice per item, and re-invokes with
      `--resolve` orders matching those choices — closing the report →
      prompt → resolve loop without the binary reading stdin.
- [ ] After the cutover the `E_DISPATCH_*` taxonomy has one implementation:
      `work-item-bridge-codes.sh` and its parity fixture are gone, and no
      surviving script sources the removed definition.

## Open Questions

- Whether to split into separate Jira and Linear work items — left grouped here;
  split if implementation granularity warrants.

## Dependencies

- Blocked by: 0166 (shared crates), and 0203 (the `RemoteTracker` port) —
  split out of 0194 on 2026-08-10 precisely so the client crates wait on a
  trait, three value types and an error type rather than on a whole sync
  engine. 0203 freezes that signature at its acceptance, so the clients
  build against a contract that will not move. The client work needs
  nothing else from 0194.
- Blocked by: 0194 (the sync engine) for the **cutover half only** — the
  script removal, skill repointing, conversational conflict flow and
  contract-suite run all need the binary that story delivers. The client
  crates and the two thin binaries do not, and can proceed in parallel
  with it once 0203 lands.
- Blocked by: 0187 (generalises the sub-binary registration surface). This story
  adds a dispatch token; it does not generalise the surface. Registration
  follows the checklist 0187 adds at
  `tasks/README.md#registering-a-dispatched-sub-binary`. (2026-08-01)
- Relates to: 0170 (the work-item lifecycle subdomain — no direct
  dependency on these clients or the `RemoteTracker` port itself; 0194
  wires `--push` onto its `create`/`update` commands separately, using
  0194's own port).
- Relates to: 0194 (the sync engine consumes these clients).
- Carries 0194's cutover. 0194 ships the Rust sync engine beside the live bash
  path without retiring it, because `sync` and `create`/`update --push` can only
  resolve fakes until these clients exist — and 0194 blocks this story, so it
  could not have waited. This story therefore inherits four obligations from it:
  the script removal and skill repointing, the sync SKILL's conversational
  conflict flow, running the shared contract suite against real clients, and
  per-provider projection fidelity against 0194's committed corpus. Until this
  story lands, the bash path remains the production path and nothing regresses.
  (2026-08-10)
- Relates to: 0174 (Retire Shell Tooling and CI Guards) — inherits the
  residual `work-item-sync-label.sh` / `work-item-normalise.sh`
  duplication that neither this story nor 0194 removes.
- Parent: epic 0136.

## Assumptions

- The existing Python mock servers port over as Rust integration-test scaffolding
  with minimal change.

## Technical Notes

- Source bash: `skills/integrations/jira/scripts/` (`jira-common`, `jira-auth`,
  `jira-jql`, `jira-body-input`, `jira-custom-fields`, flows) and
  `skills/integrations/linear/scripts/` (`linear-common`, `linear-auth`,
  `linear-graphql.sh`, flows).
- `reqwest` + rustls keeps the clients musl-static-friendly; no native-tls.

## Drafting Notes

- Treated as the Phase 8 story; kept as one grouped item per the user's selection,
  with a noted split option.
- Updated 2026-08-05: 0170 split into 0170 (lifecycle CRUD) and 0194
  (`tracker` crate and remote sync engine) following a work item review. All
  references to "the tracker port from 0170" now point at 0194, which is
  where the `RemoteTracker` port actually lives.
- Updated 2026-08-10, absorbing the cutover from 0194 after review 2 pass 2
  raised it as a critical finding against that item. 0194's original final phase
  retired the bash sync and bridge scripts, but the real clients replacing them
  are this story's deliverable and this story is blocked by 0194 — so the
  user-facing `sync` and `--push` flows would have been dead between the two,
  against epic 0136's rule that the plugin stays functional at every step. The
  cutover moved here, where the clients exist. An interim adapter shelling
  out to the existing bridge scripts was weighed and rejected: throwaway
  work needing its own retirement, for a window this story closes anyway.
  This item grew materially as a result, and its Open Question about
  splitting Jira from Linear is worth re-taking with the cutover in scope —
  a natural third slice.
- Updated 2026-08-10: the `RemoteTracker` port moved out of 0194 into its
  own item, 0203. This story's `blocked_by` now names 0203 for the client
  work and 0194 only for the cutover half, so the client crates and thin
  binaries can start as soon as the port lands rather than waiting on a
  whole sync engine they do not use.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0046, ADR-0053
