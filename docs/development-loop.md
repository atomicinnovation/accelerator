# The Development Loop

The primary workflow is a three-phase loop:

```
research-codebase    →  create-plan  →  implement-plan
        ↓                    ↓                 ↓
meta/research/codebase/ meta/plans/    checked-off plan
```

1. **Research** (`/accelerator:research-codebase "how does auth work?"`):
   Investigate the codebase using parallel subagents. Produces a structured
   research document in `meta/research/codebase/` with findings, file
   references, and architectural context.

2. **Plan** (`/accelerator:create-plan ENG-1234`): Build a phased
   implementation plan informed by research. Produces a plan document in
   `meta/plans/` with specific file changes, success criteria, and testing
   strategy. The plan is reviewed by the developer before proceeding.

3. **Implement** (`/accelerator:implement-plan @meta/plans/plan.md`): Execute
   the plan phase by phase, checking off success criteria as each phase
   completes. The plan file serves as both instructions and progress tracker.
