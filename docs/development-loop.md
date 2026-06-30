# The Development Loop

The primary workflow is a three-phase loop:

```
research-codebase    →  create-plan  →  implement-plan
        ↓                    ↓                 ↓
meta/research/codebase/ meta/plans/    checked-off plan
```

1. **Research** (`/research-codebase "how does auth work?"`):
   Investigate the codebase using parallel subagents. Produces a structured
   research document in `meta/research/codebase/` with findings, file
   references, and architectural context.

2. **Plan** (`/create-plan ENG-1234`): Build a phased
   implementation plan informed by research. Produces a plan document in
   `meta/plans/` with specific file changes, success criteria, and testing
   strategy. The plan is reviewed by the developer before proceeding.

3. **Implement** (`/implement-plan @meta/plans/plan.md`): Execute
   the plan phase by phase, checking off success criteria as each phase
   completes. The plan file serves as both instructions and progress tracker.

See [Planning](skills/planning.md) for the issue-investigation, note-capture,
and plan-review companions.

## Skill reference

### `/research-codebase [research question]`

Conduct comprehensive codebase research by spawning parallel sub-agents and
synthesising findings into a research document.

*Writes a structured document to `meta/research/codebase/`; run it first so the
plan can cite concrete findings.*

### `/create-plan [work item reference or description]`

Create detailed implementation plans through interactive, iterative
collaboration.

*Produces a phased plan in `meta/plans/` with success criteria; review it
(optionally via `review-plan`) before implementing.*

### `/implement-plan [path to plan file]`

Execute an approved implementation plan from the configured plans directory.

*Works phase by phase, checking off success criteria in the plan file as each
phase completes.*
