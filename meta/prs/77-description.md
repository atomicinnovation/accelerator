---
type: "pr-description"
id: "77"
title: "Migrate the Linear integration to accelerator-linear and retire its bash cluster (0211)"
date: "2026-08-23T14:04:30+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0211"
parent: "work-item:0211"
relates_to: ["work-item:0171"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/77"
pr_number: 77
tags: ["rust", "linear", "integrations", "cli", "cutover", "exit-codes", "registration"]
revision: "86c435d560d2f2de02355adcc2b549ea70513503"
repository: "accelerator"
last_updated: "2026-08-23T14:04:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Migrate the Linear integration to accelerator-linear and retire its bash cluster (0211)

## Summary

Ships `accelerator-linear` — a thin Rust sub-binary over the `linear-client`
crate — and cuts the Linear integration skills over to it, then deletes the
entire Linear bash cluster. This is the **Linear track of 0211** (Phases 0–2);
the Jira track (Phases 3–5) follows in a later PR. Net change is −1,026 lines
across 380 files: more bash retired than Rust added.

## Changes

- **New `accelerator-linear` binary** (`cli/linear-cli`) — create, update, show,
  search, comment, transition, attach, and init, over `linear-client`. Handlers
  return `ExitCode` inline (the `work-cli` shape), branch outcomes onto a typed
  stdout keyword discriminant, and validate the `ACCELERATOR_LINEAR_API_URL`
  base-URL seam before any credential attaches (loopback gated behind a test-only
  `test-loopback` feature, enforced by a compile guard and a staged-binary byte
  scan).
- **`init` owns cache production** — `init verify` writes `viewer.json` and `init
  discover` writes `catalogue.json` (under the advisory lock), so the repointed
  skill needs no `Write` grant. This wires the `LinearCache` that already existed
  in `linear-client` but was unused.
- **Client additions** (`linear-client`) — a structured `LinearFailure` funnel so
  exit codes are read structurally rather than parsed from a string (Decision 9),
  and a read-side search/show projection carrying `state`/`assignee` so the
  rendered table is not degraded (Decision 20).
- **Token registration + skill repoint** — the `linear` dispatch token is
  registered end to end, and all eight Linear `SKILL.md` bodies now invoke
  `accelerator linear …`, branching on keywords and `E_*` stderr names instead of
  exit integers. The three read/init skills drop their `jq`/`curl`/`scripts`
  grants for a scoped `accelerator linear *` grant; the write skills preview
  resolved intent (not wire payloads) and confirm before mutating.
- **Cluster deletion** — the whole `skills/integrations/linear/scripts/` subtree
  (the ten flow scripts, two libraries, `EXIT_CODES.md`, twelve suites, the mock
  server and 40 fixtures — 66 files), with its guards retired: two
  `SHELL_LIBRARIES` members, the integrations suite floor (32→20), and the
  `mock-linear-server.py` coverage exclude.
- **New enforcement guards** (`lint:integration-skills:check`) — keyword parity
  binds every keyword a repointed body branches on to the binary's declared set
  and forbids a reintroduced exit integer; the write gate asserts each write
  skill's confirm step precedes its mutation. Both carry adversarial unit tests.
- **Shared test support** — a new `cli-test-support` crate (scenario→route loader,
  exit-code parser) and an `http-test-support` per-hit request-body log for
  multi-POST assertions.
- **Groundwork** — the plan and its research follow-up, plus Phase 0 (freezing
  the Jira ADF differential against a committed oracle corpus so the Jira cluster
  can be deleted later without redding the test lane).

## Context

- Work item: `meta/work/0211-integration-binaries-and-bash-cluster-retirement.md`
- Plan: `meta/plans/2026-08-19-0211-integration-binaries-and-bash-cluster-retirement.md`
- Parent epic: `meta/work/0171-jira-and-linear-integrations.md`
- Inventories: `meta/inventories/0211-{removal-set,suite-audit,reconciliation,divergences,fixture-reconciliation}.md`

## Testing

- [x] `mise run` (full local CI mirror) exits 0 end to end.
- [x] `cargo nextest run -p linear-cli --features test-loopback` — 33 pass
      (per-flow request/response/stdout goldens, exit-code parity, keyword
      surface, seam rejection, `from_config`, init cache production).
- [x] `mise run cli:check`, `mise run build-system:check`, dispatch coherence,
      the two new integration-skill guards, and the exec-bit/stale-library and
      python-coverage guards.
- [ ] Manual: run `accelerator-linear` against a live Linear team and diff
      against 0210's committed 2026-08-21 contract evidence (not yet performed —
      a disposable/sandbox team is required, since the write flows leave real
      issues that are not VCS-recoverable).

## Notes for Reviewers

- **Independently mergeable.** This is the Linear track only; the Jira cluster is
  untouched and the integrations floor still holds at 20, so it merges without
  waiting on the Jira work.
- **One design divergence to note.** Errors route to exit codes + `E_*` stderr,
  not to keywords (keywords carry success outcomes). The plan's "every error
  class → one keyword" criterion is reframed accordingly, and the parity guard
  checks keyword membership rather than that mapping. Recorded in
  `0211-divergences.md`.
- **Two latent full-run reds** surfaced when the full `mise run` was first
  executed (Phase 1 had only run `mise run check`) and were fixed here: the
  plan's own `status: approved` fell outside the corpus status vocab, and
  `linear-cli` reached `reqwest::Url` past the provider-transport tripwire (now
  re-exported from `linear-client`).
- **The `test-skill-write-gate` and doc-vs-binary guards are build-system Python
  guards** beside `dispatch_coherence`, not a `.sh` suite as the plan text names
  them — the guarantee is identical; recorded in the divergences ledger.
- **Follow-up (not in this PR):** the Jira track (Phases 3–5) — the `jira-cli`
  binary, the Jira cutover and 197-file cluster deletion, and the residue phase
  (including the full 21-decision mirror into 0171).
