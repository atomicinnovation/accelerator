---
type: "pr-description"
id: "127"
title: "[0279] Iterative accretion and finalise for topic research"
date: "2026-09-20T20:02:53+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0279"
parent: "work-item:0279"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/127"
pr_number: 127
tags: ["topic-research", "skills", "lifecycle", "finalise"]
revision: "1359a8797b3865269fa4a56ddc15d6f39b78621e"
repository: "accelerator"
last_updated: "2026-09-20T20:02:53+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0279] Iterative accretion and finalise for topic research

## Summary

Grows a topic-research subject's knowledgebase across multiple rounds without
clobbering earlier findings, and closes it off once researched. This is Slice 2
of the topic-research skillset (0121), extending the single-round engine shipped
in 0277 into a full five-state lifecycle with a new `finalise` verb and reopen
regression. The whole slice is a prose change to one engine file plus committed
multi-round test fixtures — no schema, template, or Rust source change, because
the frontmatter contract already admits every state and field written here (it
landed with 0278).

## Changes

The engine (`skills/research/research-topic/SKILL.md`) gains four behaviours and
one new verb:

- **`outline` accretes rounds.** Appends a `## Round N+1` checklist when the
  highest round has been conducted, revises the highest round in place when it
  is still pending, and scales focus areas under a per-round breadth ceiling of
  8 — so a set may exceed eight focus areas across rounds.
- **`conduct` is multi-round-aware.** Gap-fills outstanding focus areas across
  every round, injects each area's real `## Round N` into its finding's `round:`
  stamp, and derives `round_count` from the highest round on disk instead of a
  hardcoded `1`. Findings stay immutable — an existing finding path is never
  clobbered.
- **`synthesise` rewrites wholesale.** Produces one standalone dossier over all
  findings across every round under anti-changelog discipline (no round
  narration, no per-round headings), stamps `rounds_covered` equal to
  `round_count`, and is idempotent.
- **`finalise` closes the subject.** New verb: refuses any non-`synthesised` set
  naming its current status, applies a read-only freshness gate that refuses a
  stale set (counts or `rounds_covered` behind disk), then sets `status:
  complete` while holding `primary` on `synthesis.md`.
- **Reopen regression.** `outline` or `conduct` on a `synthesised`/`complete`
  set regresses `status` to `researching` on its final manifest edit and reports
  the reopen with a verb-appropriate recovery path — the sole staleness signal,
  with no stored `stale` field.

The count-derivation rule is stated once, canonically, in **Validate every
write**: `finding_count` is the visible `<nn>-*.md` count (excluding
`.invalid` quarantine markers); `round_count` is the highest `round` across those
files, or `0` when none.

Test surface (`cli/corpus-cli/tests/`):

- **Two sibling fixture sets.** A `topic-research-multiround-set` (three findings
  across two rounds, shaped so the file count and highest round deliberately
  differ) and a `topic-research-quarantine-set` (a `.invalid` marker stamped
  `round: 3` excluded from `round_count: 2`). The single-round `topic-research-set`
  is left untouched.
- **Four new goldens.** Clean-validation of the multi-round set, a
  counts-agree-with-disk cross-check, a quarantine-exclusion cross-check, and a
  positive `complete`-status validation — the first anchor for the enum value
  this slice newly writes. A shared `highest_round(dir, exclude_invalid)` helper
  backs both drift-guards.

Meta artefacts: the full 0279 lifecycle — codebase research, plan, plan review,
work-item review, and the validation report — with the plan marked `done`.

## Context

- Work item: `meta/work/0279-iterative-accretion-and-finalise.md`
- Plan: `meta/plans/2026-09-19-0279-iterative-accretion-and-finalise.md`
- Validation: `meta/validations/2026-09-19-0279-iterative-accretion-and-finalise-validation.md`
- Parent epic: `meta/work/0121-topic-research-skillset.md`
- Predecessor slice (single-round engine): 0277; frontmatter contract: 0278

## Testing

Automated checks run green in the implementing session:

- [x] Golden suite: `cargo test -p accelerator-corpus --test frontmatter_goldens` — 32 passed (4 new)
- [x] Read-only CI mirror: `mise run check` — exit 0
- [x] Skill lints: `lint:skill-permissions:check`, `lint:bare-invocation:check`, `lint:dispatch-coherence:check` — all exit 0
- [x] Skill `!`-site bootstrap: `mise run test:integration:skill-invocation` — 137 passed
- [ ] Full default task `mise run` end to end — not yet run; its read-only and skill-test lanes are individually green, leaving only the frontend build and the network + Chromium docs lane
- [ ] Behavioural manual criteria (web-free scratch set) — the plan's deliberate carve-out: no eval harness hosts them until the deferred `research-topic` Inspect suite (0161) lands

## Notes for Reviewers

⚠️ **Behavioural correctness cannot be caught by the test suite.** Extras are
presence-checked only, so a `round_count` that disagrees with disk validates
clean. The golden cross-checks pin the committed exemplars against drift but
cannot exercise verb execution — only the model running the prose does. This is a
recorded, accepted risk of the slice, not an oversight.

- **Focus the review on `SKILL.md`.** It is the product; everything else is
  fixtures and lifecycle docs. The load-bearing invariants are the content-first
  / manifest-last discipline and the single count-derivation rule.
- **Reopen and `finalise` are the highest-value paths to reason about.** The
  freshness gate refuses a stale set rather than repairing it; reopen is
  unconditional on lifecycle position, not on whether work was done.
- **No migration.** Existing sets need none; the vocab and templates are
  unchanged. Depth stays 1 (recursion is 0283); override flags are 0282.
