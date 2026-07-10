# Philosophy

Accelerator structures development into discrete phases — research, plan,
implement — where each phase runs with minimal context and communicates with
the next through the filesystem. This design is intentional: by writing
research findings, plans, and other artefacts to disk rather than holding them
in the conversation, each step stays focused and avoids the quality degradation
that comes with large, cluttered context windows.

The result is a development workflow where:

- Each phase has a clear purpose and bounded scope
- The filesystem (specifically the `meta/` directory) serves as persistent
  shared memory between phases and sessions
- Subagents handle exploratory work in isolation, returning only summaries to
  the main context
- Human review happens at the highest-leverage points (research quality and
  plan quality) before implementation begins

For example, a research phase might read 50 files across a codebase, but only
a structured summary is written to disk and passed to the planning phase —
keeping the planner focused and accurate.

---

[Docs home](../README.md#documentation) · [Full Workflow →](workflow.md)
