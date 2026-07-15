<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/accelerator_logo_dark_bg.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/accelerator_logo_light_bg.png">
    <img alt="Accelerator" src="assets/accelerator_logo_light_bg.png" width="342px">
  </picture>
</p>

A Claude Code plugin for structured, context-efficient software development.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/visualiser_plan_dark.png">
    <source media="(prefers-color-scheme: light)" srcset="assets/visualiser_plan_light.png">
    <img alt="The Accelerator visualiser rendering a plan document" src="assets/visualiser_plan_light.png" width="760px">
  </picture>
</p>

Accelerator splits development into discrete phases — research, plan, implement —
that communicate through the filesystem rather than the conversation. Each phase
runs with minimal context, writing its findings to a persistent `meta/`
directory, so every step stays focused and avoids the quality loss that comes
with large, cluttered context windows.

## Getting Started

```bash
/plugin marketplace add atomicinnovation/accelerator
/plugin install accelerator@atomic-innovation
```

For the full walkthrough — initialising your project
and running the research → plan → implement loop — see [Getting
Started](https://atomicinnovation.github.io/accelerator/getting-started/).

## Documentation

The full documentation site is published at
**<https://atomicinnovation.github.io/accelerator/>**.

**Concepts**

- [Philosophy](https://atomicinnovation.github.io/accelerator/philosophy/) — the
  phase model and why development is split across the filesystem.
- [Full Workflow](https://atomicinnovation.github.io/accelerator/workflow/) — the
  map of every skill family and how they fit together, so you can pick the parts
  you need.
- [Development
  Loop](https://atomicinnovation.github.io/accelerator/development-loop/) — the
  research → plan → implement spine in detail, including the plan
  review/stress-test cycle.
- [Visualiser](https://atomicinnovation.github.io/accelerator/visualiser/) — the
  browser-based companion view of `meta/`.
- [Internals](https://atomicinnovation.github.io/accelerator/internals/) — the
  `meta/` directory deep-dive, the agent roster, and VCS detection.
- [Configuration](https://atomicinnovation.github.io/accelerator/configuration/)
  — config files, templates, per-skill customisation, and custom review lenses.
- [Migrations](https://atomicinnovation.github.io/accelerator/migrations/) —
  upgrading a repo with `/accelerator:migrate`.
- [Releases &
  Compatibility](https://atomicinnovation.github.io/accelerator/releases-and-compatibility/)
  — the prerelease channel and Claude Code compatibility.

**Skills**

- <img src="https://api.iconify.design/ph/squares-four-bold.svg?color=%23475569" width="16" align="center" alt=""> [All Skills](https://atomicinnovation.github.io/accelerator/reference/skills/) — the full index of every skill, grouped by
  family.
- <img src="https://api.iconify.design/ph/arrows-clockwise-bold.svg?color=%236366f1" width="16" align="center" alt=""> [Development Loop](https://atomicinnovation.github.io/accelerator/skills/development-loop/) — research, plan, implement,
  and the plan review/stress-test/validate companions.
- <img src="https://api.iconify.design/ph/strategy-bold.svg?color=%23f59e0b" width="16" align="center" alt=""> [Investigation & Notes](https://atomicinnovation.github.io/accelerator/skills/investigation/) — issue investigation,
  time-boxed spikes, and short-form note capture that feed the loop.
- <img src="https://api.iconify.design/ph/kanban-bold.svg?color=%230d9488" width="16" align="center" alt=""> [Work Items](https://atomicinnovation.github.io/accelerator/skills/work-items/) — capturing features, bugs, and tasks
  that feed into planning.
- <img src="https://api.iconify.design/ph/ticket-bold.svg?color=%232563eb" width="16" align="center" alt=""> [Issue Trackers (Jira & Linear)](https://atomicinnovation.github.io/accelerator/skills/issue-trackers/) — remote
  tracker integration.
- <img src="https://api.iconify.design/ph/scroll-bold.svg?color=%237c3aed" width="16" align="center" alt=""> [Architecture Decision Records (ADRs)](https://atomicinnovation.github.io/accelerator/skills/adrs/) — capturing
  architectural decisions.
- <img src="https://api.iconify.design/ph/git-branch-bold.svg?color=%2316a34a" width="16" align="center" alt=""> [VCS & PR Workflow](https://atomicinnovation.github.io/accelerator/skills/vcs-and-pr/) — commit, describe, review, and
  respond to PRs.
- <img src="https://api.iconify.design/ph/scales-bold.svg?color=%23e11d48" width="16" align="center" alt=""> [Review System](https://atomicinnovation.github.io/accelerator/skills/review-system/) — the multi-lens review system.
- <img src="https://api.iconify.design/ph/palette-bold.svg?color=%23db2777" width="16" align="center" alt=""> [Design Convergence](https://atomicinnovation.github.io/accelerator/skills/design-convergence/) — design inventories
  and gap analysis.

Contributing to Accelerator? See [CONTRIBUTING](CONTRIBUTING.md) for local
development and the CI checks.

## License

MIT — see [LICENSE](LICENSE).
