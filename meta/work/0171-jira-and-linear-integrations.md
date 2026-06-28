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
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0170"]
tags: [rust, jira, linear, integrations, reqwest]
last_updated: "2026-06-28T17:01:56+00:00"
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
provider.

## Context

`skills/integrations/jira/scripts/` (22 prod) and `linear/scripts/` (12 prod)
implement create/update/comment/transition/search/show/attach/init flows, ADF↔markdown
conversion, JQL/GraphQL, and auth, shelling out to `jq`/`curl`. Both have Python
mock HTTP servers for tests. Resolved Q2: provider clients are shared crates reused
by both the standalone binaries and the `tracker` sync engine (0170). May be split
into separate Jira and Linear stories if finer granularity is wanted.

## Requirements

- Implement `jira-client` (Jira REST + ADF↔markdown + auth) and `linear-client`
  (Linear GraphQL + auth) as adapter crates over `reqwest` + rustls + serde, each
  `impl RemoteTracker` (the port from 0170's `tracker` crate).
- Implement `accelerator-jira` and `accelerator-linear` as thin inbound CLI adapters
  exposing the user-facing flows (create/update/comment/transition/search/show/
  attach/init).
- Carry over the Python mock servers (`mock-jira-server.py`, `mock-linear-server.py`)
  as integration-test scaffolding for the Rust clients.
- Remove the `jq`/`curl` `allowed-tools` entries from the integration skills once
  migrated; confirm no other skill relies on them.

## Acceptance Criteria

- [ ] `accelerator jira …` and `accelerator linear …` reproduce the standalone flows,
      verified against the repointed integration suites and the mock servers.
- [ ] Both client crates implement `RemoteTracker` and are consumable by
      `accelerator-work`'s sync engine (0170) with no duplication of API logic.
- [ ] No production `jq`/`curl` dependency remains for the migrated integration
      skills; their `allowed-tools` entries are removed.
- [ ] The integration suite floor is decremented in lockstep as the shell scripts
      are removed.

## Open Questions

- Whether to split into separate Jira and Linear work items — left grouped here;
  split if implementation granularity warrants.

## Dependencies

- Blocked by: 0166 (shared crates), and the `tracker` port from 0170.
- Relates to: 0170 (the sync engine consumes these clients).
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

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0046, ADR-0053
