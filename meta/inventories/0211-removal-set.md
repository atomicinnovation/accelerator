---
type: inventory
id: "0211-removal-set"
title: "0211 Integration-Cluster Removal Set"
date: "2026-08-23T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: in-progress
work_item_id: "work-item:0211"
parent: "work-item:0211"
relates_to:
  - "plan:2026-08-19-0211-integration-binaries-and-bash-cluster-retirement"
tags: [rust, jira, linear, integrations, cli, cutover, removal-set, deletion]
last_updated: "2026-08-23T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0211: Integration-Cluster Removal Set

The explicit list of files each provider track deletes once every skill body
invokes its `accelerator <provider>` sub-binary, plus the revival anchor each
independently-mergeable track carries so a digest mismatch in the window before
Phase 5 has a named revision to bisect from rather than an unindexed history
walk. The consumer sweep is folded in here rather than split into a separate
`0211-removal-set-references.md` — `0167-suite-audit.md:31` points at exactly
such a file that never existed, and repeating the shape would repeat the
dangling reference.

## Linear track (Phase 2)

**Deleted**: the whole `skills/integrations/linear/scripts/` subtree — **66
files**:

- **10 flow executables**: `linear-{create,update,show,search,comment,
  transition,attach,init}-flow.sh`, `linear-auth-cli.sh`, `linear-graphql.sh`.
- **2 libraries**: `linear-common.sh`, `linear-auth.sh` (were `SHELL_LIBRARIES`
  entries).
- **`EXIT_CODES.md`** — the prose exit-code doc, superseded by
  `cli/linear-cli/src/exit_codes.rs`'s module doc plus the captured
  `bash-exit-codes.txt` fixture (Decision 6).
- **12 `test-linear-*.sh` suites** (`test-linear-{attach,auth,comment,common,
  create,graphql,init-flow,paths,search,show,transition,update}.sh`).
- **`test-helpers/mock-linear-server.py`** — the deterministic mock server.
- **`test-fixtures/` — 40 scenario files** — reconciled in
  `0211-fixture-reconciliation.md` (15 ported into `cli/linear-cli`, 25
  superseded by an existing `cli/linear-client` crate test).

**Consumer sweep** — nothing outside the deleted subtree still depends on it:

- The read/init skill frontmatters dropped the `linear/scripts/*` glob grant
  (and `jq`/`curl`) for the scoped `accelerator linear *` grant; the write
  skills invoke `accelerator linear` via bare `Bash`. `grep -rn "linear/scripts"
  skills/` returns nothing.
- `cli/linear-cli/tests/fixtures/capture-bash-exit-codes.sh` names the flow
  scripts, but is a committed **provenance** record (it captured
  `bash-exit-codes.txt`), never executed by a test — the parity test reads the
  committed fixture. It survives deletion as the capture recipe, mirroring Phase
  0's `capture-adf-oracle.sh`.
- `cli/collaboration-cli/src/auth.rs` and `cli/linear-cli/src/exit_codes.rs`
  name `linear-auth.sh`/`linear-graphql.sh` in **design-lineage doc comments**
  only; no code path resolves them.

**Guards retired**: two `SHELL_LIBRARIES` members (`tasks/lint/scripts.py`
21→19) and their `_RECONCILED_LIBRARIES` mirror; the `_EXPECTED_INTEGRATIONS_
SUITES` floor 32→20 (the surviving 20 `test-jira-*.sh`); the
`mock-linear-server.py` ruff exclude in `pyproject.toml` and its
`test_python_coverage.py` `MOCK_LINEAR` pin.

**Generator provenance / revival anchor**: the `mock-linear-server.py`, the ten
flow scripts, the two libraries and the 12 suites last existed at revision
**`5ca7dc49d037d70782341ad362c996ca17a7ec16`** ("Register the linear token and
repoint the linear skills"), the commit immediately before the deletion. To
revive a generator, check that revision out and read
`skills/integrations/linear/scripts/`. `bash-exit-codes.txt` was captured by the
committed `cli/linear-cli/tests/fixtures/capture-bash-exit-codes.sh` run against
that revision's cluster; each linear golden is mock-served against
`mock-linear-server.py` at that revision, live-anchored to 0210's 2026-08-21
contract evidence (Decision 8).

## Jira track (Phase 4)

_Pending — recorded at Phase 4's deletion boundary._
