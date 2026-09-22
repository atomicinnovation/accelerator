---
type: "work-item"
id: "0282"
title: "Tunable Depth and Breadth"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0283"]
tags: ["research", "skills", "config"]
last_updated: "2026-09-20T21:33:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-866"
---

# 0282: Tunable Depth and Breadth

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

As an Accelerator user researching a subject, I want to control how wide each
research round goes and to set the depth knob — via team and personal config
and per-invocation flags — so that I can cap the cost of a token-heavy round and
opt into deeper research deliberately.

`breadth` is the hard ceiling on focus areas a round may commission (bounding
`outline`), defaulting to 8; `depth` is the recursion limit within one finding
(bounding `conduct`), defaulting to 1. This knob child is independently shippable
and lands before the sibling recursion engine (0283). `breadth` is live from the
first commit — it bounds `outline`, which already exists. `depth` ships here as a
dormant knob: resolved, documented, and threaded through `conduct`, but with no
behavioural effect until 0283 delivers the recursion engine, and `conduct` emits
a notice when a resolved `depth` exceeds 1.

## Context

The config system already carries numeric tunables — the six `review.*` keys
(`review.max_lenses: 8`, `review.min_lenses: 4`, and peers) — so this work adds
the first *research* tunables by following that precedent, not by establishing a
new one. Defaults live in `cli/config/src/catalogue.rs` and validated numeric
consumption is modelled by `cli/launcher/src/config_command/core/review.rs`;
`breadth: 8` is the exact twin of `review.max_lenses: 8`.

The knob is what makes the recursion engine (0283) affordable to try — `depth >
1` multiplies spend — so it lands adjacent to and before that engine. Because the
knob itself does not token-multiply (config keys, flags, and a dormant `depth`),
it depends only on the round loop 0279 established, not on the 0280
output-quality gate; only 0283 carries the 0280 edge.

## Requirements

- Register `research.topic.breadth` (default `8`) and `research.topic.depth` (default `1`) in
  `cli/config/src/catalogue.rs` as a new `RESEARCH_KEYS` group added to
  `default_for`'s scan list, stored as string scalars parsed by the consumer,
  mirroring the `review.*` numeric tunables. Update the exact key-count test,
  `dump.golden`, `parity.rs`, and `public-api.txt`.
- Per-invocation overrides: `--breadth` on `outline`, `--depth` on `conduct`,
  alongside the slug positional. `outline`/`conduct` are prompt verbs in one
  SKILL.md with no arg parser, so each override is read from the invocation by
  the SKILL.md and resolved in prose.
- Resolution order flag > personal config (`config.local.md`) > team config
  (`config.md`) > hardcoded default (`breadth: 8`, `depth: 1`), read via
  `accelerator config get research.<knob> --default <n>` and applied
  flag-over-config in the SKILL.md.
- Validation in the SKILL.md prose: a resolved value that is zero, negative, or
  non-integer clamps to the floor of 1 with a warning that names the invalid
  value and states it was clamped to 1; only an integer of 1 or more passes
  unchanged — a non-integer clamps regardless of magnitude; no upper cap, since
  `breadth` is itself the cost guard.
- Thread `depth` through `conduct` with no behavioural effect until 0283; when a
  resolved `depth` exceeds 1, `conduct` emits a notice that recursive deepening
  is unavailable until 0283. `breadth` bounds `outline` as a live ceiling.
- `breadth` is enforced at `outline` time only; `conduct` executes the
  outline's outstanding focus areas without re-checking `breadth`, so a
  hand-edited `outline.md` exceeding the ceiling is honoured.
- Replace the hardcoded `breadth`/`depth` prose in
  `skills/research/research-topic/SKILL.md` with the resolved values.
- Document both knobs, their defaults, the resolution order, and depth's
  "no effect until 0283" caveat in `configure help`
  (`skills/config/configure/SKILL.md`).

## Acceptance Criteria

- [ ] Keys visible — given the config catalogue, when `accelerator config dump`
      runs, then `research.topic.breadth` and `research.topic.depth` appear with defaults `8`
      and `1` and source attribution.
- [ ] Default resolution — given neither knob is set at either level, when a knob
      is resolved, then `breadth` resolves to `8` and `depth` to `1`.
- [ ] Config precedence — given `research.topic.breadth` set in `config.md` (team) and
      a different value in `config.local.md` (personal), when resolved with no
      flag, then the personal value wins; given only the team value, the team
      value wins; given neither, the default wins.
- [ ] Flag override — given `research.topic.breadth: 8` in config, when `outline` runs
      with `--breadth 3`, then the round is sized to at most 3 focus areas rather
      than 8; likewise, given `research.topic.depth: 1` in config, when `conduct` runs
      with `--depth 2`, then the depth notice fires, proving the flag resolved to
      2 over the config value (verified at eval level — the SKILL.md carries the
      flag-over-config contract).
- [ ] Validation — given a resolved `breadth` or `depth` that is zero, negative,
      or non-integer, when the knob is resolved, then it clamps to `1` and the
      invocation warns, naming the invalid value and stating it was clamped to
      `1`; only an integer of 1 or more passes unchanged — a non-integer clamps
      regardless of magnitude — with no upper cap.
- [ ] Breadth ceiling — given a resolved `breadth` of N, when `outline` sizes a
      round, then it commissions at most N focus areas; the effort-scaling rubric
      may reduce beneath N but never raise above it.
- [ ] Breadth not re-checked at conduct — given an `outline.md` hand-edited to
      more focus areas than the resolved `breadth`, when `conduct` runs, then it
      researches every outstanding focus area without clamping to `breadth`.
- [ ] Depth dormant with a signal — given a resolved `depth` of 1, when `conduct`
      runs, then it spawns one researcher per focus area and emits no depth
      notice; given a resolved `depth` greater than 1, when `conduct` runs, then
      it still spawns one researcher per focus area (no recursion) and emits a
      notice that recursive deepening is unavailable until 0283.
- [ ] Docs — given `configure help`, when a user reads it, then both knobs are
      documented with their defaults, the resolution order, and depth's
      "no effect until 0283" caveat.
- [ ] All checks pass under `mise run check`.

## Open Questions

- None. The design questions — resolution home, dormant-depth behaviour,
  registration, and validation — were resolved in the 2026-09-20 stress test
  (see Drafting Notes).

## Dependencies

- Blocked by: 0279 (done) — the `outline`/`conduct` round loop this knob wires
  into; recorded on 0279's `blocks`, and now satisfied. The specific artefacts
  this knob makes tunable — the `outline` breadth ceiling, the effort-scaling
  rubric, and the hardcoded `breadth: 8`/`depth: 1` prose — originate in Slice 1
  (0277), transitively upstream via 0279 and already satisfied.
- Blocks: 0283 — the recursion engine needs the `depth` knob; recorded on this
  item's `blocks` (the repo stores forward `blocks` edges only, so there is no
  reciprocal `blocked_by` on 0283; 0283 mirrors the edge in prose).
- Independent of 0280 — unlike 0283, this knob does not token-multiply, so it
  does not depend on the 0280 output-quality gate. Only 0283 carries that edge.
  There is a light build-level coupling, though: 0280 (the academic-sources
  slice) also registers a `research.*` key (`research.contact_email`) touching
  the same key-count test, `dump.golden`, and `public-api.txt`, so whichever of
  0282 and 0280 lands second reconciles those files for the other's key.

## Assumptions

- `breadth` is a hard ceiling, not a target: the effort-scaling rubric operates
  at or beneath it, and a user who wants the rubric's 10+ band raises `--breadth`
  deliberately.
- `breadth` is enforced at `outline` time only; a hand-edited `outline.md`
  exceeding it is a deliberate user override that `conduct` honours without
  re-clamping.
- Numeric tunables are stored as string scalars parsed by the consumer (the
  `review.*` model); no numeric type is added to the config value model.
- Resolution and validation live in SKILL.md prose (prompt-resolved); the
  flag-over-config arithmetic is an eval-level contract, not unit-tested.

## Technical Notes

- Precedent to mirror: `cli/config/src/catalogue.rs` `REVIEW_KEYS` (numeric
  defaults as string scalars) and `cli/launcher/src/config_command/core/review.rs`
  (`positive()`/`non_negative()`/`int()` — parse, validate, clamp, warn).
  `breadth: 8` mirrors `review.max_lenses: 8`.
- Defaults live in `catalogue.rs`, not the deleted `scripts/config-defaults.sh`
  (removed in PR #84 / work item 0174). Registering a key means a new
  `RESEARCH_KEYS` group in `default_for`'s scan list plus updates to the exact
  key-count test, `dump.golden`, `parity.rs`, and `public-api.txt`.
- `outline`/`conduct` are prompt verbs in `skills/research/research-topic/SKILL.md`
  with no arg parser; `--breadth`/`--depth` are read from the invocation text and
  resolved in prose. Today `breadth: 8`/`depth: 1` are hardcoded prose to be
  replaced.
- Config precedence (existing, `cli/config/src/service.rs`): personal
  (`config.local.md`) > team (`config.md`) > catalogue default. Both are in-repo
  files; there is no home-directory level. The flag layer is new and sits in the
  consumer (the SKILL.md), above `config get`, following the flag > config >
  default shape already in `cli/work-cli/src/create.rs` (`--project` over
  `work.key`).
- Testability: the config layers (default fold-in, personal > team, `dump`
  visibility) are unit- and golden-testable through existing config machinery;
  the flag-over-config override, the clamp-and-warn validation, and the
  depth-greater-than-1 notice are prose contracts verified at eval level, not by
  unit tests — a direct consequence of prompt-resolution.
- The two knobs compose rather than contradict: `breadth` bounds focus areas per
  round, `depth` bounds recursion within a finding. Both numbers should be quoted
  together when reasoning about cost. `depth` is dormant until 0283.

## Drafting Notes

- Enriched via an interactive stress test (2026-09-20) against the parent epic
  (0121) and siblings 0279 (done) and 0283, with two codebase agents verifying
  the config plumbing. Decisions: prompt-resolved knobs; accept-and-warn for the
  dormant `depth` window; catalogue-registered keys; positive-integer floor with
  clamp-and-warn; `breadth` as an outline-time ceiling only.
- Corrected three premises carried over from extraction: the deleted
  `scripts/config-defaults.sh`; the "first numeric tunable" claim (six `review.*`
  numeric tunables already exist); and the unstated config precedence (actual
  order is personal > team > default).
- 0280 independence confirmed against the wiring: the 0280 gate edge is recorded
  on 0280's `blocks` (targeting 0283) and mirrored in 0283's prose; this item
  carries no such edge.
- Status kept at `draft` at the author's request; promotion to `ready` is a
  downstream decision.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 5, knob half)
- Parent epic: 0121
- Siblings: `meta/work/0283-recursive-finding-deepening.md` (recursion engine —
  consumes the `depth` knob), `meta/work/0279-iterative-accretion-and-finalise.md`
  (the round loop this wires into; done)
- Precedent: `cli/config/src/catalogue.rs` (`REVIEW_KEYS`),
  `cli/launcher/src/config_command/core/review.rs` (validated numeric tunable),
  `cli/work-cli/src/create.rs` (flag > config > default precedent)
- To modify: `skills/research/research-topic/SKILL.md` (hardcoded breadth/depth),
  `skills/config/configure/SKILL.md` (`configure help` docs)
