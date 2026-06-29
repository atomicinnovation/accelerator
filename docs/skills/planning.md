# Planning

Companion skills handle issue investigation and quick note capture:

- `/accelerator:research-issue "auth timeout on token refresh"` — Investigate
  production issues through hypothesis-driven debugging. Accepts stacktraces,
  logs, or behavioral descriptions and produces an RCA document in
  `meta/research/issues/`.
- `/accelerator:create-note "rate limiter resets on deploy"` — Capture a
  short-form note (observation, insight, snippet) to `meta/notes/`.
- `/accelerator:conduct-spike @meta/work/0042-spike-x.md` — Interactively run a
  time-boxed spike against a spike work item (or brief), recording the outcome
  back on the work item.

Three complementary skills support the core [development
loop](../development-loop.md):

- `/accelerator:review-plan @meta/plans/plan.md` — Review a plan through
  multiple quality lenses before implementation
- `/accelerator:stress-test-plan @meta/plans/plan.md` — Interactively
  stress-test a plan through adversarial questioning to find issues,
  inconsistencies, and gaps
- `/accelerator:validate-plan @meta/plans/plan.md` — Verify after
  implementation that the code matches the plan
