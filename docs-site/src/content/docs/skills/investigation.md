---
title: 'Investigation & Notes'
---

These skills feed into the core [development loop](../development-loop.md):
companions for issue investigation, time-boxed spikes, and short-form
note capture. They run before or alongside planning to reduce
uncertainty and record what you learn.

- [`research-issue`](../reference/skills/research/research-issue.md)
  investigates production issues and bugs through hypothesis-driven
  debugging. It accepts stacktraces, logs, or vague behavioural
  descriptions and produces a root-cause analysis in
  `meta/research/issues/`. Reach for it when something is broken and
  you don't yet know why.
- [`conduct-spike`](../reference/skills/research/conduct-spike.md) runs
  a time-boxed spike — discussion mixed with agent-driven research and
  small throwaway prototypes where a question is empirical — then
  records the outcome on the spike's work item. Reach for it when a
  work item poses open questions that must be resolved before planning
  can proceed with confidence.
- [`create-note`](../reference/skills/notes/create-note.md) captures a
  short-form observation, insight, or snippet to `meta/notes/`, where
  later research can rediscover it.

The ordering is need-driven rather than sequential: an RCA from
`research-issue` often becomes a bug work item; a spike resolves a work
item's open questions so `create-plan` can start; a note is the cheapest
way to make today's insight available to next month's research.
