# Planning

These skills surround the core [development loop](../development-loop.md):
companions for issue investigation, note capture, and time-boxed spikes, plus
three skills that review, stress-test, and validate a plan.

### `/research-issue [issue description, stacktrace, or error]`

Investigate production issues and bugs through hypothesis-driven debugging.
Accepts stacktraces, logs, or behavioural descriptions and produces a root-cause
analysis in `meta/research/issues/`.

### `/create-note [note topic]`

Interactively capture a short-form note. Writes an observation, insight, or
snippet to `meta/notes/`.

### `/conduct-spike [path to spike work item or brief, or number]`

Interactively conduct a time-boxed spike — collaboratively reduce uncertainty
through discussion mixed with agent-driven research (and small throwaway
prototypes where a question is empirical), then record the outcome on the
spike's work item.

*Reach for it when a work item poses open questions that must be resolved before
planning or implementation can proceed with confidence.*

### `/review-plan [path to plan file]`

Review an implementation plan through multiple quality lenses and collaboratively
iterate based on findings.

*Runs the multi-lens [Review System](review-system.md); see that page for the
lens catalogue. Use before implementation begins.*

### `/stress-test-plan [path to plan file]`

Interactively stress-test an implementation plan by grilling the user on
decisions, edge cases, and assumptions to find issues, inconsistencies, and gaps
before implementation begins.

*Complements `review-plan`: review applies fixed quality lenses, stress-test
interrogates **your** reasoning interactively.*

### `/validate-plan [path to plan file]`

Validate that an implementation plan was correctly executed by verifying success
criteria and identifying deviations.

*Run after `implement-plan` to confirm the code matches the plan and to surface
anything that drifted.*
