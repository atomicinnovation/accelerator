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
blocked_by: ["work-item:0187", "work-item:0194"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0170", "work-item:0194", "work-item:0174"]
tags: [rust, jira, linear, integrations, reqwest, sync]
last_updated: "2026-08-12T00:40:00+00:00"
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
  `impl RemoteTracker` (the port from 0204's `tracker` crate).
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
  real clients are held to one contract. That harness asserts
  `fetch_all`'s partition totality and the read-never-`Terminal` rule as
  well as the create/update round trips; both are port obligations no
  test in `tracker` can hold.
- Port **four** exit-code mapping tables, not two, and derive each
  operation's classification from its own: `_wicr_map_jira` and
  `_wiur_map_jira` for Jira, `_wiur_map_linear` and
  `_linear_map_no_file_failure` (inside `linear-create-flow.sh`) for
  Linear. The two operations' provable sets are **not nested in either
  direction** — Linear code 34 is retryable on `create` and terminal on
  `update`, while codes 18, 23, 25, 27 and 29 run the other way — so a
  single status-to-class table is wrong in both directions.

  The tables are deliberately more conservative than the rule they
  encode: several codes are provably pre-transmission yet mapped
  terminal. Where the two disagree, **the tables win** — port them rather
  than reasoning from `TrackerError`'s doc comment and arriving somewhere
  more precise.
- Carry over the identifier-safety check the create bridge performs
  (`work-item-create-remote.sh:62-87,238-246`): reject a returned
  identifier carrying control characters, a newline, or a leading `---`
  or `#`, because the value is written unquoted into a work item's YAML
  frontmatter. It is the one tracker-agnostic check the dispatcher does,
  and the dispatcher dissolves at the port — `ExternalId::new` is
  infallible by freeze, so the type cannot carry it. An unsafe identifier
  is a `Terminal` failure, not an `Ok`.
- Bound the port's calls from inside. `RemoteTracker` is synchronous with
  no deadline or cancellation, so the per-request timeouts the bridges
  carry today (`curl --max-time 30` for Jira, `--max-time 60` for the
  Linear flows) and the `_WIFR_PAGE_CAP=20` pagination backstop must be
  reproduced behind the seam. A caller has no way to add them, and
  `/list-work-items` relies on the read path not hanging.
- Decide the fate of four bridge capabilities that have **no port
  operation**, each deliberately left above the port by 0204 rather than
  overlooked. For each: re-site it above the port, drop it, or carry it
  as an additive port item.
  - The unkeyed discovery `search` mode of `work-item-fetch-remote.sh` —
    used by `/sync-work-items` to list remote issues with no local work
    item. `fetch_all(ids)` is key-scoped and cannot express it.
  - The create bridge's `--dry-run` field-resolution preview, which
    surfaces an unresolvable Jira project *before* the confirm gate.
  - The update bridge's `--dry-run` payload validation, which is what
    `/sync-work-items --preview` uses to validate every push against the
    live tracker today. 0194's `--preview` routes mutations to no-ops and
    makes no port call, so it does not discharge this.
  - Dropping any of them silently is a user-visible regression at
    cutover.
- Reproduce the existing per-provider projection recipes exactly (jira —
  summary line then the description in Atlassian Document Format through
  key-sorted `jq -S`; linear — title line then Markdown description
  verbatim), verified against the bash-generated baseline corpus 0194
  commits under `test-fixtures/`. Title line, then description, with **no
  blank line between them** and a trailing newline. The value returned is
  the *un-normalised* projection; the caller normalises before hashing.

  The case a JSON deserialiser will get wrong: an **absent** description
  projects as the literal token `null` for Jira (`jq -cS '… // null'`)
  and as an empty line for Linear (`// ""`). Neither is what `serde` would
  naturally produce, and either wrong choice reclassifies every such item
  as `remotely-modified` on the first run after cutover.
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
- [ ] Each of the four bridge capabilities with no port operation — unkeyed
      `search`, both `--dry-run` modes, and the identifier-safety check — is
      re-sited, dropped or carried forward as a recorded decision, and none
      is lost silently at cutover.
- [ ] The doc comments in `tracker` that name bash artefacts are updated in
      the same change that deletes them: `RemoteIssue.body`'s projection
      reference, `errors.rs`'s module doc, `show`'s `# Errors` note about
      the read bridge, and `RemoteTimestamp::Reported`'s note about the
      bash-written baseline. The contracts they state outlive the scripts;
      the references do not.
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

- No longer blocked by 0204 (the `RemoteTracker` port) — split out of
  0194 on 2026-08-10 precisely so the client crates would wait on a
  trait and its vocabulary rather than on a whole sync engine, and
  accepted and implemented on 2026-08-12. The frozen signature is six
  items, not four value types and an error type as this story previously
  understood: the trait, `ExternalId`, `RemoteIssue`, `RemoteTimestamp`,
  `FetchOutcome` and `TrackerError`. One consequence for this story's own
  Requirements: `fetch_all` returns `FetchOutcome.found` as
  `Vec<(ExternalId, RemoteTimestamp)>`, a stamp per key rather than a
  projected issue, so each client's bulk-mode query needs no
  `description`/body field at all — Linear's selection set in particular
  can stay as narrow as `linear-search-flow.sh`'s today, with no need to
  widen it against Linear's complexity cap. The client work needs
  nothing else from 0194.

  One further shape settled on 0204's implementation review (2026-08-12):
  `RemoteTimestamp` is a three-variant enum, not a `String` newtype. A
  client maps a tracker's stamp to `Reported(bytes)` verbatim and a blank
  or null one to `NotReported` — never to `Reported("")`, which means
  nothing. The third variant, `NotRead`, is unreachable through the port:
  a client cannot return it, because `show` and `fetch_all` either answer
  or fail.
- Blocked by: 0194 (the sync engine) for the **cutover half only** — the
  script removal, skill repointing, conversational conflict flow and
  contract-suite run all need the binary that story delivers. The client
  crates and the two thin binaries do not, and can proceed in parallel
  with it once 0204 lands.
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
  own item, 0204. This story's `blocked_by` now names 0204 for the client
  work and 0194 only for the cutover half, so the client crates and thin
  binaries can start as soon as the port lands rather than waiting on a
  whole sync engine they do not use.
- Updated 2026-08-12: 0204 was accepted and implemented, and cleared from
  `blocked_by` — the client work can now start. Its final signature grew
  by one item during plan review (`FetchOutcome`, a total three-way
  partition over a bulk `fetch_all`), which this story's description had
  not caught up with; corrected above, along with the resulting change to
  what a bulk-mode query needs to request. 0204's own review also
  confirmed a real gap this story should be aware of when implementing
  `impl RemoteTracker`: a client that quietly drops an operation once the
  trait gains a default-bodied method is undetectable by either of
  0204's own guards (`cargo public-api` and the signature-probe test),
  so nothing catches that mistake below the level of 0194's shared
  contract test actually exercising all four operations against the real
  client.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0046, ADR-0053
