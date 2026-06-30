# Development Loop

The research → plan → implement spine, plus the review companions that harden a
plan before any code is written. For the narrative — how the phases hand off
through the `meta/` directory — see the [Development Loop](../development-loop.md).

### <img src="https://api.iconify.design/ph/magnifying-glass-bold.svg?color=%236366f1" width="18" align="center" alt=""> `/research-codebase [research question]`

Conduct comprehensive codebase research by spawning parallel subagents and
synthesising findings into a research document.

*Writes a structured document to `meta/research/codebase/`; run it first so the
plan can cite concrete findings.*

### <img src="https://api.iconify.design/ph/clipboard-text-bold.svg?color=%236366f1" width="18" align="center" alt=""> `/create-plan [work item reference or description]`

Create detailed implementation plans through interactive, iterative
collaboration.

*Produces a phased plan in `meta/plans/` with success criteria; review it
(optionally via `review-plan`) before implementing.*

### <img src="https://api.iconify.design/ph/hammer-bold.svg?color=%236366f1" width="18" align="center" alt=""> `/implement-plan [path to plan file]`

Execute an approved implementation plan from the configured plans directory.

*Works phase by phase, checking off success criteria in the plan file as each
phase completes.*

### <img src="https://api.iconify.design/ph/binoculars-bold.svg?color=%236366f1" width="18" align="center" alt=""> `/review-plan [path to plan file]`

Review an implementation plan through multiple quality lenses and
collaboratively iterate based on findings.

*Runs the multi-lens [Review System](review-system.md); see that page for
the lens catalogue. Use before implementation begins.*

### <img src="https://api.iconify.design/ph/barbell-bold.svg?color=%236366f1" width="18" align="center" alt=""> `/stress-test-plan [path to plan file]`

Interactively stress-test an implementation plan by grilling the user on
decisions, edge cases, and assumptions to find issues, inconsistencies, and gaps
before implementation begins.

*Complements `review-plan`: review applies fixed quality lenses, stress-test
interrogates **your** reasoning interactively.*

### <img src="https://api.iconify.design/ph/seal-check-bold.svg?color=%236366f1" width="18" align="center" alt=""> `/validate-plan [path to plan file]`

Validate that an implementation plan was correctly executed by verifying success
criteria and identifying deviations.

*Run after `implement-plan` to confirm the code matches the plan and to surface
anything that drifted.*
