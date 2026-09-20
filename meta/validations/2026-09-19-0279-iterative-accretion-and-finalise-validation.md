---
type: "plan-validation"
id: "2026-09-19-0279-iterative-accretion-and-finalise-validation"
title: "Validation Report: Iterative Accretion and Finalise Implementation Plan"
date: "2026-09-20T19:22:02+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-19-0279-iterative-accretion-and-finalise"
tags: ["topic-research", "lifecycle", "finalise"]
last_updated: "2026-09-20T19:22:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Iterative Accretion and Finalise Implementation Plan

All five phases are fully implemented and every runnable automated check is
green. The change surface matches the plan exactly — one engine file, two
sibling test fixtures, and the golden test file — with no drift into the schema,
templates, or Rust source the plan ruled out. The behavioural manual criteria
remain the plan's deliberate carve-out: only the model executing the prose can
exercise them, so they are listed for attended verification rather than failed.

### Implementation Status

- ✅ Phase 1: Multi-round contract fixture and golden — fully implemented
- ✅ Phase 2: `conduct` gap-fill, real round injection, count derivation — fully implemented
- ✅ Phase 3: `outline` next-round appending and pending-round revision — fully implemented
- ✅ Phase 4: `synthesise` multi-round wholesale rewrite — fully implemented
- ✅ Phase 5: Reopen regression, `finalise`, and the `primary` invariant — fully implemented

Commit-to-phase mapping is clean and in the plan's mandated order:

| Change | Phase | Commit |
|---|---|---|
| Multi-round + quarantine fixtures, golden cross-checks | 1 | `pmnmkmyv` |
| `conduct` disk-derived counts, real round | 2 | `ltzynumq` |
| `outline` per-round breadth ceiling, append/revise | 3 | `pqttsoyx` |
| `synthesise` wholesale multi-round rewrite | 4 | `kpwlnlxn` |
| Reopen + `finalise` lifecycle closure | 5 | `tlsssmzq` |
| Docstring comment-policy cleanup (follow-up) | — | `mrzpqlqt` |

### Automated Verification Results

- ✅ Golden suite: `cargo test -p accelerator-corpus --test frontmatter_goldens` — 32 passed, 0 failed
- ✅ Read-only CI mirror: `mise run check` — exit 0
- ✅ Skill-permission census: `mise run lint:skill-permissions:check` — exit 0
- ✅ Bare-invocation guard: `mise run lint:bare-invocation:check` — exit 0
- ✅ Dispatch-coherence: `mise run lint:dispatch-coherence:check` — exit 0
- ✅ Skill `!`-site bootstrap: `mise run test:integration:skill-invocation` — 137 passed
- ⚠️ Full default task `mise run` (Phase 5 criterion) not executed end to end — its read-only lanes (`check`) and the two skill test lanes above are individually green; the untested residue is the frontend build and the network + Chromium docs lane, neither touched by this prose-and-fixture slice.

The four new golden tests the plan required are all present and passing:
`the_committed_topic_research_multiround_set_validates_clean`,
`the_committed_topic_research_multiround_set_counts_agree_with_disk`,
`a_quarantined_invalid_finding_is_excluded_from_round_count`, and
`a_manifest_at_the_complete_lifecycle_end_validates`. The shared
`highest_round(dir, exclude_invalid)` helper backs both drift-guards, as
specified.

### Code Review Findings

#### Matches Plan

- **Change surface is exactly the four artefacts named.** `jj diff` across the six commits touches only `skills/research/research-topic/SKILL.md`, `cli/corpus-cli/tests/frontmatter_goldens.rs`, the two sibling fixture sets, and the plan file. No `schema.rs`, `templates-schema.tsv`, template file, or frontend fixture changed — honouring "What We're NOT Doing".
- **Single-round fixture untouched.** `topic-research-set/` and its negative goldens are absent from the diff; the multi-round and quarantine sets landed as siblings per the fixture-location decision.
- **Fixture shapes are correct and divergent by design.** The multi-round manifest carries `status: synthesised`, `round_count: 2`, `finding_count: 3`, `primary: synthesis.md`; the outline splits three focus areas across two rounds so the file count (3) and the highest round (2) differ, letting the cross-check distinguish a `max(round)` derivation from a file count.
- **Quarantine exclusion is exercised.** The quarantine set stamps `.03-third-focus.md.invalid` with `round: 3` while valid findings sit at rounds 1 and 2; the manifest's `round_count: 2` and `finding_count: 2` prove the marker is excluded, and the test asserts it.
- **`SKILL.md` carries every planned transition.** Widened preconditions (`:71-82`), the generic-refusal enumeration with the `finalise` carve-out (`:84-87`), real-round injection (`:176`), disk-derived counts (`:197-206`), append/revise branching (`:125-153`), wholesale multi-round synthesis with `rounds_covered` stamp (`:215-225`), reopen regression (`:227-255`), and the guarded `finalise` with its read-only freshness gate (`:257-286`) are all present.
- **Count-derivation rule stated once, canonically.** The **Validate every write** section (`:327-333`) is the single source of truth: `conduct`/`synthesise` write both counts, `finalise` reads them for its freshness gate.
- **Five-verb surface is discoverable.** `description` conveys the grow-and-close loop and the reopen edge, `argument-hint` lists `finalise SLUG`, and the dispatch sentence names all five verbs — with no new `allowed-tools` grant.

#### Deviations from Plan

- **None material.** The preconditions in `SKILL.md` already reflect the Phase 5 final set rather than the narrower intermediate widths shown in the Phase 2–4 diffs — expected, since the plan builds these incrementally and Phase 5 supersedes the earlier precondition text. The end state matches Phase 5 verbatim.

#### Potential Issues

- ⚠️ **Behavioural correctness is unverifiable by the test suite.** Extras are presence-checked only, so a `round_count` that disagrees with disk validates clean. The golden cross-checks pin the committed exemplars against drift but cannot exercise `conduct`/`synthesise`/`finalise` execution — that is the model running the prose. This is the plan's recorded accepted risk, not a defect in the implementation.
- ⚠️ **In-place manifest edits remain non-atomic.** A crash mid-write can leave torn YAML as the aggregate root; recovery is a VCS revert. This is a pre-existing 0277 window the plan acknowledges, not new to this slice.

### Manual Testing Required

The plan's Testing Strategy makes these a deliberate carve-out from test-first —
no eval harness hosts them until the deferred `research-topic` Inspect suite
(0161) lands. Most are web-free via a pre-seeded scratch set; one needs attended
live web.

1. Web-free (pre-seeded scratch set from the committed multi-round fixture):
  - [ ] `conduct` reconcile-only run leaves every pre-seeded finding byte-identical (clean VCS diff over finding paths) and only reconciles checkboxes and counts (AC3).
  - [ ] Gap-fill `conduct` within Round 1 leaves `round_count` unchanged; conducting a Round 2 with findings raises it to `2` (AC4).
  - [ ] `outline` appends `## Round N+1` when the highest round has ≥1 finding and revises in place when it has none, reporting the branch taken (AC1, AC2).
  - [ ] `synthesise` rewrites wholesale over both rounds, stamps `rounds_covered: 2`, and is idempotent on a second run (AC5).
  - [ ] `finalise` refuses a non-`synthesised` set naming its status (AC9); sets `complete` on a fresh `synthesised` set; its freshness gate refuses a crash-simulated stale set.
  - [ ] Reopen: `outline`/`conduct` on a `synthesised`/`complete` set regresses to `researching`, prints the reopen notice with a verb-appropriate recovery path, and holds `primary` on `synthesis.md` (AC8, AC10, AC11).

2. Attended live web (never unattended, per the deferred-hardening constraint):
  - [ ] A genuine gap-fill `conduct` stamps the injected `## Round N` into each new finding's `round:` slot and researches only outstanding areas.

### Recommendations

- **Run `mise run` (full default) once before merge** to close the Phase 5 automated criterion end to end, covering the frontend build and docs lane that `check` omits.
- **Walk the web-free manual script** on a scratch set before relying on the loop; the reopen notice and the `finalise` freshness gate are the highest-value paths to confirm by hand, since no test can.
- **Track the deferred `research-topic` eval suite (0161)** as the eventual automation of these behavioural criteria; until it lands, each slice re-verifies manually.
