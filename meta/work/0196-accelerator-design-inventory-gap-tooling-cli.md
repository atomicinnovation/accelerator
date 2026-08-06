---
type: work-item
id: "0196"
title: "accelerator-design: Design Inventory and Gap Tooling CLI"
date: "2026-08-05T19:03:35+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["work-item:0173"]
tags: [rust, design, cli, playwright]
last_updated: "2026-08-06T00:45:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0196: accelerator-design: Design Inventory and Gap Tooling CLI

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Migrate the design inventory/gap tooling (`inventory-design`,
`analyse-design-gaps`) into an `accelerator-design` sub-binary, and resolve
whether the Playwright executor (`run.sh`) stays a thin wrapper the binary
execs or is folded into the binary, per the ADR-0048 thin-wrapper exception
(see Open Questions).

## Context

Split out of work-item:0173 (now abandoned) on 2026-08-05, per that item's
review-1 scope finding: bundling `accelerator-corpus`, `accelerator-design`, and
`accelerator-collaboration` into a single story risked partial-completion
ambiguity and an oversized PR. 0173 also stated the Playwright-executor's fate
inconsistently — as a hedged either/or in Requirements, and separately as an
unresolved Open Question — a clarity finding this item resolves by stating the
choice once, in Open Questions, and requiring the decision be recorded once made.

## Requirements

- `accelerator-design` — design inventory/gap tooling for `skills/design/**`
  maintainers and consumers, per the sub-binary consistency established by
  parent epic work-item:0136 (`inventory-design/scripts/*`,
  `analyse-design-gaps/scripts/*`; the subcommand set is whatever these two
  script directories resolve to at implementation time — record the
  concrete mapping in Drafting Notes once known). The Playwright executor
  (`run.sh`, currently under `inventory-design/scripts/playwright/`) is
  scoped by the Open Questions decision below.
- Rewrite the call sites and `allowed-tools` of every skill under
  `skills/design/**` to call the new `accelerator design` subcommands,
  following the invocation contract established in 0167.

## Acceptance Criteria

- [ ] `accelerator design …` reproduces the inventory/gap behaviours, verified
      against repointed suites (existing tests redirected to invoke the new
      binary instead of the legacy shell scripts) and characterization tests
      where none exist — each covering at least the primary success path and
      one failure path per subcommand in the set recorded in Drafting Notes
      once known (see Requirements).
- [ ] Invoking the `inventory-design` subcommand's Playwright-driven path
      execs `run.sh` (or its folded equivalent) and exits 0, producing a
      report artefact that is byte-identical to the one the current shell
      invocation produces for a fixed fixture input. Restructuring the
      report format is out of scope for this item; if a future need to
      restructure it arises, it is tracked as a separate follow-up item.
- [ ] All skills previously invoking `skills/design/**/scripts/*` now call the
      corresponding `accelerator design` subcommand, with `allowed-tools`
      updated to match, per the 0167 contract.
- [ ] The migrated `skills/design/**` scripts are removed (excepting `run.sh` if
      the thin-wrapper exception is exercised), with the affected suite floors
      decremented in lockstep (see work-item:0174).
- [ ] `accelerator-design` passes every item of the sub-binary registration
      checklist at `tasks/README.md#registering-a-dispatched-sub-binary`.

## Open Questions

- Whether the Playwright executor stays shell (thin-wrapper exception) or is
  folded into `accelerator-design`. Default: stays a thin wrapper, consistent
  with parent epic work-item:0136's resolved expectation of a thin residual
  shell surface — folding it in is a substantially larger rewrite and should
  only be taken as an explicit re-scope, confirmed before implementation
  begins rather than decided mid-implementation. Record the resolution in
  Drafting Notes once made.

## Dependencies

- Blocked by: none currently. Prior blockers are resolved: work-item:0166
  (shared crates, done), work-item:0167 (invocation-contract pattern, done —
  subsumes the earlier launcher/dispatch scaffold), work-item:0187
  (sub-binary registration surface, merged via PR #42).
- Blocks: work-item:0174 (shell/CI-guard retirement — floor decrements from
  this item's script removals feed its lockstep requirement).
- External: a Node/Playwright runtime must be available for the design-gap
  tooling to function (pre-existing coupling, carried forward from the bash
  scripts).
- Coordination: siblings work-item:0195 (corpus) and work-item:0197
  (collaboration) register sub-binaries via the same checklist around the
  same time; if that checklist touches shared state (a central dispatch
  manifest or CI floor config) rather than being purely additive per-binary,
  coordinate to avoid merge contention.
- Parent: work-item:0136 (epic).

## Assumptions

- Repointed suites plus characterization tests where none exist are
  sufficient to establish behavioural parity with the legacy shell scripts.
- A Node/Playwright runtime remains an acceptable external dependency for
  `accelerator-design`, consistent with the pre-existing coupling in the
  bash scripts.

## Technical Notes

- Source bash: `skills/design/**/scripts/*` (`inventory-design`,
  `analyse-design-gaps`), including `run.sh` (confirmed at
  `inventory-design/scripts/playwright/run.sh` — the Playwright executor
  belongs to the `inventory-design` subcommand, not `analyse-design-gaps`).

## Drafting Notes

- Split out of work-item:0173 on 2026-08-05 following that item's review-1
  (verdict REVISE, scope lens): the three sub-binaries it bundled were
  functionally independent and separately deliverable.
- The Playwright-executor either/or was previously stated twice, inconsistently,
  in 0173 (a review-1 clarity finding); here it appears once, in Open Questions.
- review-1 (2026-08-06, verdict REVISE) found the either/or had recurred
  within this item itself — the Summary stated the decision as settled while
  Requirements/Open Questions/AC still hedged it, and the two outcomes are
  not equivalent effort. Resolved by: hedging the Summary consistently,
  removing the restated either/or from Requirements, and adding a default
  (thin wrapper) plus a pre-implementation confirmation gate to Open
  Questions.

## References

- Split from: `meta/work/0173-remaining-subdomains-corpus-design-collaboration.md`
  (abandoned)
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0048, ADR-0053
