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

Add the marketplace and install the stable plugin:

```bash
/plugin marketplace add atomicinnovation/accelerator
/plugin install accelerator@atomic-innovation
```

Then initialise your project and run the research → plan → implement loop:

```bash
/accelerator:init
/accelerator:research-codebase "how does auth work?"   # 1. research
/accelerator:create-plan                               # 2. plan (optionally pass a work-item key)
/accelerator:implement-plan                            # 3. implement
```

For the prerelease channel (where the newest features land first) and Claude
Code compatibility, see [Releases & Compatibility](docs/releases-and-compatibility.md).

## Documentation

**Concepts**

- [Philosophy](docs/philosophy.md) — the phase model and why development is
  split across the filesystem.
- [Full Workflow](docs/workflow.md) — the map of every skill family and how
  they fit together, so you can pick the parts you need.
- [Development Loop](docs/development-loop.md) — the research → plan →
  implement spine in detail, including the plan review/stress-test cycle.
- [Visualiser](docs/visualiser.md) — the browser-based companion view of `meta/`.
- [Internals](docs/internals.md) — the `meta/` directory deep-dive, the agent
  roster, and VCS detection.
- [Configuration](docs/configuration.md) — config files, templates, per-skill
  customisation, and custom review lenses.
- [Migrations](docs/migrations.md) — upgrading a repo with `/accelerator:migrate`.
- [Releases & Compatibility](docs/releases-and-compatibility.md) — the prerelease
  channel and Claude Code compatibility.

**Skills**

- <img src="https://api.iconify.design/ph/squares-four-bold.svg?color=%23475569" width="16" align="center" alt=""> [All Skills](docs/skills/README.md) — the full index of every skill, grouped by
  family.
- <img src="https://api.iconify.design/ph/arrows-clockwise-bold.svg?color=%236366f1" width="16" align="center" alt=""> [Development Loop](docs/skills/development-loop.md) — research, plan, implement,
  and the plan review/stress-test/validate companions.
- <img src="https://api.iconify.design/ph/strategy-bold.svg?color=%23f59e0b" width="16" align="center" alt=""> [Investigation & Notes](docs/skills/investigation.md) — issue investigation,
  time-boxed spikes, and short-form note capture that feed the loop.
- <img src="https://api.iconify.design/ph/kanban-bold.svg?color=%230d9488" width="16" align="center" alt=""> [Work Items](docs/skills/work-items.md) — capturing features, bugs, and tasks
  that feed into planning.
- <img src="https://api.iconify.design/ph/ticket-bold.svg?color=%232563eb" width="16" align="center" alt=""> [Issue Trackers (Jira & Linear)](docs/skills/issue-trackers.md) — remote
  tracker integration.
- <img src="https://api.iconify.design/ph/scroll-bold.svg?color=%237c3aed" width="16" align="center" alt=""> [Architecture Decision Records (ADRs)](docs/skills/adrs.md) — capturing
  architectural decisions.
- <img src="https://api.iconify.design/ph/git-branch-bold.svg?color=%2316a34a" width="16" align="center" alt=""> [VCS & PR Workflow](docs/skills/vcs-and-pr.md) — commit, describe, review, and
  respond to PRs.
- <img src="https://api.iconify.design/ph/scales-bold.svg?color=%23e11d48" width="16" align="center" alt=""> [Review System](docs/skills/review-system.md) — the multi-lens review system.
- <img src="https://api.iconify.design/ph/palette-bold.svg?color=%23db2777" width="16" align="center" alt=""> [Design Convergence](docs/skills/design-convergence.md) — design inventories
  and gap analysis.

Contributing to Accelerator? See [CONTRIBUTING](CONTRIBUTING.md) for local
development and the CI checks.

## License

MIT — see [LICENSE](LICENSE).
