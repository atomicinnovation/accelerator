---
type: "pr-description"
id: "68"
title: "Decompose the Jira and Linear integrations item into four children"
date: "2026-08-17T13:08:08+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0171"
parent: "work-item:0171"
relates_to: ["work-item:0210", "work-item:0211", "work-item:0212", "work-item:0213", "work-item:0174"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/68"
pr_number: 68
tags: ["work-items", "reviews", "decomposition", "jira", "linear"]
revision: "46d6fdb76f52528138233be1e956fc6d6a3e5d48"
repository: "accelerator"
last_updated: "2026-08-17T13:08:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Decompose the Jira and Linear integrations item into four children

## Summary

0171 (Jira and Linear Integrations) had grown to 28 requirements and 31 acceptance criteria covering two provider client crates, two dispatched binaries, three bash-retirement clusters, a fixture relocation, eleven test conversions and a conversational skill flow — epic-scale for a single story, and three review passes had failed to reduce its major-finding count. This PR turns it into an epic delivering through four children, each with its own status, requirements and acceptance criteria, and adds the five review artefacts that produced the decomposition.

The substantive outcome is not the reshuffling: it is that reviewing the children caught a user-visible regression the parent's own criteria would have shipped, and that verifying the ordering rationale against the repository inverted it.

## Changes

- **0171 becomes an epic** (`kind: epic`, nested under 0136 following the 0145-under-0192 precedent) delivering through 0210 (provider client crates over the `RemoteTracker` port), 0212 (work-item script cutover), 0211 (integration binaries and bash cluster retirement) and 0213 (conversational conflict resolution flow). 0210–0212 are `story`, 0213 is `task`.
- **Requirements and acceptance criteria move into the children, which are normative.** The parent keeps the narrative, the decomposition, the shared `## Decisions` register and the cross-child dependencies. It holds no duplicate copy, because it had one and it drifted within a single revision — losing a per-flow fixture-provenance obligation and the qualifier that a recorded assertion count is context rather than a bar. Both losses surfaced as review findings, which is the evidence for removing the duplication rather than annotating it.
- **The cutover order is 0210 → 0212 → 0211**, with 0213 independent. Two constraints force it: 0210 is the only window in which the bash oracles still exist, so it precedes every deletion; and the coupling between the two deletion sets runs in one direction only, so the work suites must go first.
- **Three port-less bridge capabilities are now owned by 0212** — the unkeyed discovery `search` mode of `work-item-fetch-remote.sh`, the create bridge's `--dry-run` field-resolution preview, and the update bridge's `--dry-run` payload validation. Each has no `RemoteTracker` operation and none survives the deletion by itself.
- **0174's `blocked_by` re-points at 0211 and 0212**, the increments that actually clear its suite floors and `SHELL_LIBRARIES` entries, so a blocker lookup lands on the work rather than on a parent that performs none.
- **Five review artefacts added** under `meta/reviews/work/`: 0171 across three passes and one per child across two, each through the clarity, completeness, dependency, scope and testability lenses. All five accepted with their remaining findings recorded rather than resolved.

## Context

Work item: [0171](../work/0171-jira-and-linear-integrations.md), under epic [0136](../work/0136-migrate-shell-scripts-to-rust-cli.md). Children: [0210](../work/0210-provider-client-crates-over-the-tracker-port.md), [0211](../work/0211-integration-binaries-and-bash-cluster-retirement.md), [0212](../work/0212-work-item-script-cutover.md), [0213](../work/0213-conversational-conflict-resolution-flow.md).

Two findings from the review passes are worth reading in full, because both were latent defects rather than presentation problems.

**The port-less capabilities had fallen through the decomposition.** 0171 spent a requirement, two acceptance criteria, one of three pickup-blocking open questions and three `## Decisions` entries on the fate of three bridge capabilities with no port operation. After the carve-out, no child mentioned any of them — `grep` for `dry-run`, `unkeyed` and `port-less` across all four returned nothing, while 0171 referenced them ten times — and 0212 deletes `work-item-fetch-remote.sh` and both remote bridges wholesale. All four children could have been accepted green while `/sync-work-items` silently lost the ability to list remote issues with no local work item, and `/sync-work-items --preview` lost live push validation against the tracker. Three lenses raised it independently. A full audit (28 requirements, 31 criteria) confirmed it was the only gap.

**The ordering rationale was false, and verifying it inverted the order.** The decomposition originally placed 0211 before 0212 on the belief that `linear-create-flow.sh:304` and `jira-resolve-fields.sh:140` were live callers of `work-item-sync-label.sh`. They are comments noting that those scripts perform the same normalisation; grepping the clusters for invocations of any `work-item-*.sh`, with comment lines excluded, returns nothing. The real coupling runs the other way and is larger: four of the five suites 0212 deletes — `test-work-item-create-remote.sh`, `-update-remote.sh`, `-fetch-remote.sh` and `-sync-apply.sh` — resolve paths into `skills/integrations/{jira,linear}/scripts/test-helpers/` for the Python mock servers and `.../test-fixtures/` for their scenario fixtures. In the original order, 0211's wholesale cluster deletion would have broken all four suites and the `_EXPECTED_WORK_SUITES` floor at its own merge boundary. The 0211 and 0212 reviews carry a `## Correction` section recording that they endorsed that ordering and why the credit they gave it was misplaced.

## Testing

This change is eleven markdown files under `meta/`. No code, no tests, no build inputs.

- [x] `accelerator corpus frontmatter validate` — structural and referential conformance across all eleven changed files: clean.
- [x] Whole-corpus `accelerator corpus frontmatter validate`: one violation, pre-existing and unrelated (`meta/plans/2026-08-11-0196-...md` carries `status: superseded`, not in the plan vocab). None of the touched ids appear.
- [x] The validator caught four dangling `relates_to` refs in the child reviews (`work-item-review:0171` resolves to no artifact; the correct form carries the review's full stem). Fixed and squashed into the commit that introduced them.
- [x] Every touched item parses through `accelerator work show`, with `kind`, `status`, `parent`, `blocked_by` and `blocks` reading back as intended, and frontmatter `**Status**` body labels agreeing with the frontmatter.
- [x] Dependency graph symmetry: 0210 blocks 0211 and 0212; 0212 is blocked by 0210 and blocks 0211 and 0174; 0211 is blocked by 0210 and 0212 and blocks 0174; 0213 free. 0174's `blocked_by` reciprocates from the other side.
- [x] Frontmatter quoting restored to the repository convention on all five work items after `accelerator work update` stripped it (see Notes).
- [ ] `mise run` end to end — not run. Nothing in this diff feeds the frontend, server, `cli/` or `scripts/` lanes, and `docs:check` builds `docs-site/`, which is untouched. CI covers it.

## Notes for Reviewers

The decomposition is the reviewable decision; the review artefacts are the evidence trail behind it and can be skimmed. If you read one thing, read 0171's `## Decomposition` section and the ordering note beneath it.

**Five things must close before 0210, 0211 or 0212 can be picked up**, and they are recorded as such rather than resolved: the three open questions on 0171 (where the credentialed tracker target's secrets live, the fate of the three port-less capabilities, and whether `EXIT_CODES.md` is rewritten in place or folded into the CLI docs) and the two size-bounding assumptions in 0211 and 0212 (that the eight enumerated flows per provider are the whole bash surface, and that the Rust surface already covers the three previously-deferred scripts). Two of the five change what is in scope, so estimates are not meaningful until they are answered. 0213 is gated by none of them and fixes live degradation, so it can and should land first.

**Three findings were accepted rather than fixed**, and are named in each review's `## Acceptance` section: 0210 carries no acceptance criterion for HTTP-status or GraphQL-level error classification or for auth (its four-table criterion covers `curl` transport codes, which the item stresses are not HTTP statuses); the non-port provider surface — `comment`, `transition`, provider `search`, `attach` and `init`, five of the eight flows, none with a port operation — is owned by neither 0210 nor 0211; and 0212 now bundles a mechanical deletion cutover with three capability re-sitings whose fates are still open. The first two are judged more cheaply closed by implementing 0210 than by further specification.

**A CLI defect surfaced and is worth fixing separately.** `accelerator work update` re-serialises frontmatter and strips the repository's quoting convention from `title`, `date`, `parent` and `last_updated`, and unquotes typed-linkage list members. It hit all five items when their status moved to `ready`; I restored the quoting by hand in the first commit, but the next `work update` on any of these items will strip it again. Commit `88d2b760b2` ("Restore frontmatter quoting on the 0185 and 0197 work items") suggests this has bitten before.

**On process, for what it is worth.** Across four review passes the severity ceiling fell — three criticals, then one, then none, then none — while the major count did not converge, and each fix round introduced two or three new majors of a single shape: a requirement updated without its criteria, or a criterion updated without its requirement. That pattern is why this stops at accepted-with-findings rather than at a clean pass.
