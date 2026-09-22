---
type: "plan-validation"
id: "2026-09-20-0282-tunable-depth-and-breadth-validation"
title: "Validation Report: Tunable Depth and Breadth Implementation Plan"
date: "2026-09-22T10:57:58+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-20-0282-tunable-depth-and-breadth"
tags: ["research", "skills", "config"]
last_updated: "2026-09-22T10:57:58+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Tunable Depth and Breadth Implementation Plan

Result: pass. All five phases are implemented as specified, and the plan's
end-state verification lane exits 0 end-to-end. The only outstanding items are
the eval-level manual checks the plan deliberately defers to a human running the
`research-topic` skill.

### Implementation Status

✓ Phase 1: Register the research knobs — Fully implemented
✓ Phase 2: Resolve `config get` from the catalogue — Fully implemented
✓ Phase 3: Breadth as a live outline ceiling — Fully implemented
✓ Phase 4: Depth threaded through conduct, dormant — Fully implemented
✓ Phase 5: Document the knobs in configure help — Fully implemented

### Automated Verification Results

✓ Full read-only lane: `mise run check` (exit 0)
✓ CLI unit + integration tests: `mise run test:unit:cli` (3068 passed, 1 skipped)
✓ Public-API snapshot: `mise run public-api:check` (exit 0)
✓ Skill-invocation conformance: `mise run test:integration:skill-invocation`
  (139 passed)

⚠️ Incidental: `public-api:check` emits a rustdoc warning for an unclosed
`<name>` HTML tag at `corpus/src/frontmatter_validation/template_shape.rs:652`.
It is outside 0282's scope (the `corpus` crate is untouched by this plan) and
does not fail the lane.

### Code Review Findings

#### Matches Plan:

Phase 1 — catalogue plumbing:

- `RESEARCH_KEYS` declares `research.breadth` (`8`) and `research.depth` (`1`)
  beside `REVIEW_KEYS`, wired into `default_for`'s scan array
  (`cli/config/src/catalogue.rs`).
- Key-count test renamed to
  `the_catalogue_holds_sixty_five_keys_across_seven_groups` asserting `65`;
  declared-value test `default_for_the_research_knobs_are_typed_scalars` added.
- Dump loop emits research rows after review, before agents
  (`cli/launcher/src/config_command/core/dump.rs`); the two golden rows appear
  in that position (`dump.golden`) attributed to `default`.
- `RESEARCH_KEYS` inserted alphabetically in `public-api.txt`.
- Parity resolution test added with a `research-knobs` fixture (team
  `breadth:5, depth:3`; personal `breadth:2`) asserting resolved `2`/`3`.

Phase 2 — `config get` from the catalogue:

- Positional `default` becomes `#[arg(long)]` (`launch/inbound/cli.rs`); the
  `Get`/`Path`/`Work` doc comments are reconciled to "built-in default".
- `get.rs` resolves a non-empty `--default`, then the catalogue default
  (cross-level only), then empty, filtering an empty `--default` through to the
  catalogue; both the module `//!` and the `resolve` doc are updated.
- `init-jira` drops the positional `""` on `jira.site`/`jira.email`.
- `ports.rs` port doc re-pointed from the positional-`""` form to the flag form.
- Tests: catalogue-fallback rewrite, precedence chain, single-level
  no-fallback, `--level` + `--default`, uncatalogued empty-default, and an
  extended `--help` assertion; the `:320` get-half now asserts `meta/plans`;
  the fail-safe test is unchanged.
- `CHANGELOG.md` records the breaking grammar change under `### Changed`.

Phases 3–4 — research-topic SKILL prose:

- Argument hint advertises `outline SLUG [--breadth N]` and
  `conduct SLUG [--depth N]`.
- Knob-resolution block injects both
  `!`accelerator config get research.<knob> --fail-safe`` reads, states
  `flag > personal > team > built-in default`, and carries the
  empty / valid / clamp / misplaced-flag rules with the exact warning wording.
- `outline` ceiling rule reads `--breadth` and sizes under the resolved
  ceiling; `conduct` reads `--depth`, spawns one researcher per focus area, and
  gates the `>1` notice — with no breadth re-check introduced.

Phase 5 — configure help:

- `### research` section placed after `### review`, with the knob table
  (`8`/`1`), the resolution order, the plain-language dormancy caveat, the YAML
  example, and the no-comments note.

#### Deviations from Plan:

- The test docstring the plan asked to rewrite on
  `get_prefers_a_non_empty_default_over_the_built_in` (Phase 2 §5) was added by
  the implementation, then removed in a later comment-policy pass (commit
  `8ac60a0b`) as restatement. The test name and body — the behaviour the plan
  specified — are intact and pass. Benign.
- The plan's Phase 3 §2 intro line ("depth stays a fixed 1 until Phase 4 …") is
  correctly absent: Phase 4 §2 broadened it, so the shipped SKILL shows the
  final merged wording. Expected, not a defect.

#### Potential Issues:

- None affecting 0282. The eight manual boxes in Phases 3–4 are runtime
  behaviour of a prompt-resolved skill, verifiable only by invoking
  `research-topic`; they are not implementation gaps.

### Manual Testing Required:

These are the plan's own deferred eval-level checks. Run `research-topic` in a
scratch repo:

1. Breadth (Phase 3):
  - [ ] No config: `outline SLUG` sizes under a ceiling of 8; `--breadth 3`
        sizes to at most 3.
  - [ ] Resolved breadth `0` / `-2` / `2.5` clamps to 1 with the
        `Warning: research.breadth …` message naming the value.
  - [ ] `outline` never writes more focus areas than the ceiling in one round.
  - [ ] `outline SLUG --depth 2` (misplaced flag) is ignored with a note.
  - [ ] A hand-edited over-ceiling `outline.md` then `conduct SLUG` researches
        every focus area without clamping.

2. Depth (Phase 4):
  - [ ] No config: `conduct SLUG --depth 2` fires the notice and still spawns
        one researcher per focus area.
  - [ ] Depth 1 emits no notice; depth greater than 1 emits the notice with no
        recursion.
  - [ ] Depth `0` / negative / non-integer clamps to 1 with the
        `Warning: research.depth …` message.

### Recommendations:

- Merge as-is: the implementation is complete and the automated lane is green.
- Run the eight manual checks above before relying on the runtime behaviour;
  the plan scopes them to a human invocation and no eval suite gates them.
- File the unrelated `corpus` rustdoc `<name>` tag warning separately; it is
  pre-existing and outside this plan.
- For the hand-off to 0283 (recursion engine), the dormant-to-live depth flip is
  a three-site prose edit already catalogued in the plan's "What We're NOT
  Doing".
