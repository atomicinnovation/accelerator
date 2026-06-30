# Planning

These skills surround the core [development loop](../development-loop.md):
companions for issue investigation, note capture, and time-boxed spikes, plus
three skills that review, stress-test, and validate a plan.

### `/research-issue`

**What it does** — Investigate production issues and bugs through
hypothesis-driven debugging. Accepts stacktraces, logs, or behavioural
descriptions and produces a root-cause analysis in `meta/research/issues/`.

**How to use it** — `/research-issue [issue description, stacktrace, or error message]`

### `/create-note`

**What it does** — Interactively capture a short-form note. Writes an
observation, insight, or snippet to `meta/notes/`.

**How to use it** — `/create-note [note topic]`

### `/conduct-spike`

**What it does** — Interactively conduct a time-boxed spike — collaboratively
reduce uncertainty through discussion mixed with agent-driven research (and
small throwaway prototypes where a question is empirical), then record the
outcome on the spike's work item.

**How to use it** — `/conduct-spike [path to spike work item or brief, or work item number]`

**Advice & guidelines** — Reach for it when a work item poses open questions
that must be resolved before planning or implementation can proceed with
confidence.

### `/review-plan`

**What it does** — Review an implementation plan through multiple quality lenses
and collaboratively iterate based on findings.

**How to use it** — `/review-plan [path to plan file]`

**Advice & guidelines** — Runs the multi-lens [Review System](review-system.md);
see that page for the lens catalogue. Use before implementation begins.

### `/stress-test-plan`

**What it does** — Interactively stress-test an implementation plan by grilling
the user on decisions, edge cases, and assumptions to find issues,
inconsistencies, and gaps before implementation begins.

**How to use it** — `/stress-test-plan [path to plan file]`

**Advice & guidelines** — Complements `review-plan`: review applies fixed
quality lenses, stress-test interrogates *your* reasoning interactively.

### `/validate-plan`

**What it does** — Validate that an implementation plan was correctly executed
by verifying success criteria and identifying deviations.

**How to use it** — `/validate-plan [path to plan file]`

**Advice & guidelines** — Run after `implement-plan` to confirm the code matches
the plan and to surface anything that drifted.
