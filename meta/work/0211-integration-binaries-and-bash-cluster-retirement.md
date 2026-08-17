---
type: work-item
id: "0211"
title: "Integration Binaries and Bash Cluster Retirement"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0171"
blocked_by: ["work-item:0210", "work-item:0212"]
blocks: ["work-item:0174"]
relates_to: ["work-item:0165", "work-item:0203"]
tags: [rust, jira, linear, integrations, cli, cutover]
last_updated: "2026-08-17T11:17:18+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0211: Integration Binaries and Bash Cluster Retirement

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Ship `accelerator-jira` and `accelerator-linear` as thin inbound CLI adapters
over 0210's client crates, repoint the jira and linear skill bodies at them, then
delete both bash script clusters, their suites and their Python mock servers.
Register the two dispatch tokens end to end, including the release manifest.

## Context

Last of the three sequenced children of 0171, ordered **after** the work-item
cutover (0212). The clusters this child deletes carry the shared test assets four
of 0212's suites consume — `test-helpers/` holding the Python mock servers, and
`test-fixtures/` holding their scenario fixtures — so those suites must be gone
before the clusters can be retired whole. The coupling is one-directional:
verified by grep, the clusters invoke no `work-item-*.sh`, and the two references
to `work-item-sync-label.sh` in `linear-create-flow.sh:304` and
`jira-resolve-fields.sh:140` are comments about shared normalisation, not calls.
Landing this child first would break `test-work-item-{create,update,fetch}-remote.sh`
and `-sync-apply.sh` along with the `_EXPECTED_WORK_SUITES` floor.

`skills/integrations/jira/scripts/` (22 production scripts plus the
`jira-common`, `jira-auth`, `jira-jql`, `jira-body-input` and
`jira-custom-fields` libraries) and `skills/integrations/linear/scripts/` (12
production scripts plus `linear-common` and `linear-auth`) implement the eight
flows per provider, Atlassian Document Format (ADF)↔markdown conversion, Jira
Query Language (JQL) and GraphQL, and auth.

## Requirements

- Implement `accelerator-jira` and `accelerator-linear` as thin inbound CLI
  adapters over 0210's client crates, each exposing the eight flows its cluster
  implements: `create`, `update`, `comment`, `transition`, `search`, `show`,
  `attach`, `init`. This `search` is the provider-side issue search the bash flow
  exposes; it is **not** the unkeyed discovery `search` mode of
  `work-item-fetch-remote.sh`, which 0212 discharges separately.
- Define and document the exit-code contract for the two binaries. Enumerate the
  failure classes — at minimum auth-invalid, target-not-found, payload-rejected,
  transport-failure and usage-error — and **anchor each integer externally rather
  than choosing it**: it equals the value the retiring bash flow returned for the
  same condition, captured pre-deletion, or `tracker`'s existing `E_DISPATCH_*`
  value where that is genuinely the same taxonomy. Name the document of record
  (the CLI's own exit-code documentation). Choosing the integers freely and then
  writing them down would make the mapping unfalsifiable, and the repointed skill
  bodies branch on these values, so a silent divergence from the bash codes breaks
  skill behaviour.
- Capture, before deleting each bash flow, a golden of its **stdout** as well as
  its request and response. The repointed `SKILL.md` bodies consume that output
  the way they consume the scripts' output today, including any `!`-preprocessor
  interpolation.
- Record the **provenance** of every captured fixture in 0171's `## Decisions`:
  per flow, whether it was captured against the credentialed target or the
  retiring Python mock server, and for each mock-served entry the blocker that
  prevented a real capture. A mock-captured fixture pins the new client to a test
  double this child deletes, so an unrecorded provenance hides that substitution.
- Confirm the shared test assets have no surviving consumer before deleting them.
  `test-helpers/` (both Python mock servers) and `test-fixtures/` (scenarios, and
  Jira's `adf-samples` and `api-responses`) were consumed by four of 0212's
  suites, which that child deletes first. Re-run the sweep at this child's
  boundary — every reference into
  `skills/integrations/{jira,linear}/scripts/{test-helpers,test-fixtures}` from
  outside the clusters themselves — and record the result. It should be empty; if
  anything survives, repoint or remove it here rather than deleting an asset still
  in use.
- Register both dispatch tokens per
  `tasks/README.md#registering-a-dispatched-sub-binary`, add the **two binary
  crates'** `cli/pup.ron` import rules and public-API snapshots — `jira-client`
  and `linear-client` carry theirs in 0210, which creates them — and add both
  binaries to the per-platform release upload set and the minisign-signed
  `manifest.json` 0165 owns. If 0210's copyleft check fired, 0203's attribution artefact is a
  release-path dependency.
- Repoint the **bodies** of every jira and linear `SKILL.md` at
  `accelerator jira …` / `accelerator linear …`, not merely their frontmatter,
  and remove the `jq`/`curl` `allowed-tools` entries.
- Delete both clusters whole: all production scripts, all seven library entries,
  the bash integration suites, and `mock-jira-server.py` /
  `mock-linear-server.py`.
- Enumerate every skill whose frontmatter declares `jq` or `curl` in
  `allowed-tools` after the change, and record the enumeration. This child lands
  last, so the expected survivors are exactly the skills still backed by repo-root
  `scripts/*.sh`, which 0174 owns — no jira, linear or work skill among them (0212
  repointed the work skills before this child began). This is where the
  whole-repository equality assertion is checkable.
- Retire `_EXPECTED_INTEGRATIONS_SUITES = 32` (`tasks/test/integration.py:57`)
  outright rather than decrementing it, and remove the seven jira and linear
  library entries from `SHELL_LIBRARIES` (`tasks/lint/scripts.py:18`):
  `jira-common`, `jira-auth`, `jira-jql`, `jira-body-input`,
  `jira-custom-fields`, `linear-common`, `linear-auth`. Two senses of
  "library" are in play and must not be conflated: a **sourced-only library file**
  on disk (governed by the exec-bit invariant) and a **`SHELL_LIBRARIES` member**
  in `tasks/lint/scripts.py`. Classify `linear-graphql.sh` explicitly as either
  one of Linear's twelve production scripts (executable, no `SHELL_LIBRARIES`
  member, so the expected removal count stays seven) or a sourced-only library
  (an eighth member, so the expected count becomes eight). Record which — an
  orphaned member fails the exec-bit invariant guard with no story left to fix
  it.

## Acceptance Criteria

- [ ] For each of the eight flows per provider, a `wiremock`-backed test asserts
      the outgoing request (method, path or GraphQL document, and body) and the
      parsed response against a fixture captured from the bash flow before
      deletion. Where a flow issues no single provider request — `init` if it
      only validates credentials and writes config — the criterion is replaced by
      a named observable outcome for that flow; for `attach`, the fixture file
      and expected transport shape (multipart or link-based) are named. Every
      fixture's provenance is recorded in 0171's `## Decisions` as
      credentialed-target or mock-served, each mock-served entry naming the
      blocker that prevented a real capture.
- [ ] Each of the sixteen subcommands' stdout is asserted byte-for-byte against
      a golden captured from the corresponding bash flow. Where the shape
      deliberately changed, that branch commits a golden of the **new** stdout
      plus a recorded diff against the bash golden, and a CLI-level test asserts
      the specific tokens the repointed `SKILL.md` body instructs the model to
      read — so the branch is not an escape from the byte-for-byte half.
- [ ] Each binary's eight subcommands parse and dispatch, and every enumerated
      failure class maps to an integer **equal to the value the retiring bash flow
      returned for the same condition** (captured pre-deletion) or to `tracker`'s
      existing `E_DISPATCH_*` value — asserted table-driven at the CLI level, in
      `accelerator-jira` and `accelerator-linear`, the layer the repointed skills
      invoke. The document of record states the same mapping, so it records the
      contract rather than defining it.
- [ ] Every one of the 22 Jira and 12 Linear production scripts maps either to a
      named subcommand of its binary or to a recorded internal-helper
      classification, with the mapping committed and its count reconciling to the
      pre-deletion file list — so a flow present in bash but absent from the
      enumerated eight cannot be deleted unnoticed.
- [ ] The shared-asset sweep is recorded and empty: grepping the repository,
      excluding `meta/`, for
      `skills/integrations/jira/scripts/test-helpers`,
      `.../jira/scripts/test-fixtures`, `.../linear/scripts/test-helpers`,
      `.../linear/scripts/test-fixtures`, `mock-jira-server` and
      `mock-linear-server` returns hits only from inside the clusters being
      deleted. The grep command and its output are the recorded result. Anything
      surviving is repointed or removed here, not deleted from under a live
      consumer.
- [ ] `ls skills/integrations/jira/scripts/*.sh` and
      `ls skills/integrations/linear/scripts/*.sh` each match nothing (or both
      directories are absent); `mock-jira-server.py` and
      `mock-linear-server.py` do not exist; and every jira and linear
      `SKILL.md` body invokes `accelerator jira …` or `accelerator linear …`.
- [ ] The set of `skills/` frontmatter entries declaring `jq` or `curl` in
      `allowed-tools` after this change is **exactly** the set belonging to skills
      still backed by repo-root `scripts/*.sh`, all of which 0174 owns — with no
      jira, linear or work skill among them. This child lands last, so it is the
      one that can assert the whole-repository equality; 0212 asserted only that no
      work skill declares them. The enumerated result is recorded in 0171's
      `## Decisions`.
- [ ] Both dispatch tokens are registered per the sub-binary checklist, both
      binaries appear in the per-platform upload set and the signed
      `manifest.json`, and the two binary crates carry pup rules and public-API
      snapshots — `jira-client` and `linear-client` carried theirs in 0210.
- [ ] `_EXPECTED_INTEGRATIONS_SUITES` is removed from
      `tasks/test/integration.py`, the seven jira and linear entries are gone
      from `SHELL_LIBRARIES` — eight if `linear-graphql.sh` was classified as a
      sourced-only library, seven if as a production script —
      `linear-graphql.sh`'s classification is recorded in 0171's `## Decisions`,
      and `lint:scripts:exec-bits:check` is green.
- [ ] No Python remains in the `cli/` test lane: the mock servers are gone and
      neither client crate's dev-dependencies nor `tasks/` reference them.
- [ ] `mise run` exits 0 end-to-end at this child's merge boundary, not only
      after the whole of 0171.

## Dependencies

- Blocked by 0210: these binaries are thin adapters over its client crates, and
  the per-flow fixtures must be captured while the bash flows still exist (they
  are deleted by this child, so the capture happens inside it).
- Blocked by 0212. The clusters carry `test-helpers/` and `test-fixtures/`, which
  four of 0212's suites consume; those suites must be deleted before these
  directories can be. Verified by grep in both directions: the clusters invoke no
  `work-item-*.sh`, so nothing here waits on 0212 for any other reason.
- **Conditionally requires the credentialed tracker target.** Response shapes
  should be captured from the real Jira project and Linear team wherever
  reachable, since a mock-served fixture pins the new client to a test double this
  child deletes. Where the target is unreachable for a flow, the mock is
  acceptable and the blocker is recorded. The target has no work item; 0171 names
  the owner.
- **Blocks 0174**, as the last of the three: 0174 cannot remove the
  `SHELL_LIBRARIES` frozenset or the exec-bit guard while this child's seven (or
  eight) orphaned
  entries and the live `_EXPECTED_INTEGRATIONS_SUITES` floor still exist. Recorded
  in this child's `blocks` and in 0174's `blocked_by`.
- Blocks 0212: the two `work-item-sync-label.sh` callers in these clusters must
  be gone before 0212 deletes that script.
- 0165 owns the signed `manifest.json` and upload set both binaries join. 0203
  (still `ready`) becomes a release-path dependency if 0210's copyleft check
  fired — a licence failure rather than a build one. **The trigger is explicit**:
  if 0210 records a copyleft component, whoever records it adds
  `work-item:0203` to this child's `blocked_by` and `work-item:0211` to 0203's
  `blocks` before this child leaves `draft`.
- 0174 owns the fourteen repo-root `scripts/*.sh` `SHELL_LIBRARIES` entries and
  the skills backed by them; this child owns only the seven jira and linear
  entries.
- Parent: 0171.

## Assumptions

- ⚠️ **Unconfirmed, and it bounds this child's size**: the eight enumerated
  flows plus ADF↔markdown, JQL and GraphQL construction are the whole
  user-facing surface of both clusters. Confirm against both script directories
  before planning — if the list is short, more flows need migrating, fixturing
  and stdout goldens.

## References

- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Blocked by: `meta/work/0210-provider-client-crates-over-the-tracker-port.md`
- Related: 0165, 0174, 0203
