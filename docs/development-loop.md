# Development Loop

The primary workflow is a three-phase loop:

```
research-codebase    →  create-plan  →  implement-plan
        ↓                    ↓                 ↓
meta/research/codebase/ meta/plans/    checked-off plan
```

1. **Research** — [`research-codebase`](skills/development-loop.md#research-codebase)
   `"how does auth work?"`: Investigate the codebase using parallel subagents.
   Produces a structured research document in `meta/research/codebase/` with
   findings, file references, and architectural context.

2. **Plan** — [`create-plan`](skills/development-loop.md#create-plan) `ENG-1234`:
   Build a phased implementation plan informed by research. Produces a plan
   document in `meta/plans/` with specific file changes, success criteria, and
   testing strategy. The plan is reviewed by the developer before proceeding.

3. **Implement** — [`implement-plan`](skills/development-loop.md#implement-plan)
   `@meta/plans/plan.md`: Execute the plan phase by phase, checking off success
   criteria as each phase completes. The plan file serves as both instructions
   and progress tracker.

## Companion skills

Three optional skills strengthen the loop around the plan. Use as many or as
few as a change warrants:

- [`review-plan`](skills/development-loop.md#review-plan) and
  [`stress-test-plan`](skills/development-loop.md#stress-test-plan) harden a plan
  before any code is written — review applies fixed quality lenses, stress-test
  interrogates your reasoning interactively. Update the plan in response and
  re-run as needed.
- [`validate-plan`](skills/development-loop.md#validate-plan) runs after
  implementation to confirm the code matches the plan and surface anything that
  drifted.

This loop is the centre of the wider [Full Workflow](workflow.md). For the
per-skill reference — invocation, arguments, and what each writes to `meta/` —
see [Development Loop skills](skills/development-loop.md). For the investigative
and note-capture companions that feed into it, see
[Investigation & Notes](skills/investigation.md).
