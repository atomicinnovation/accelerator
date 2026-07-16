---
title: Configuration Cookbook
description: Ready-made .accelerator/config.md recipes for common
  customisations — paths, agent overrides, per-skill instructions, and
  issue-tracker integrations.
---

Ready-made recipes for `.accelerator/config.md`. Each recipe is a
complete, paste-able example; combine the frontmatter blocks as needed.
For the full key reference and how the files are resolved, see the
[Configuration](../configuration.md) reference, or run
`/accelerator:configure help`.

Two files are read, from the repository root:

- `.accelerator/config.md` — team-shared, committed.
- `.accelerator/config.local.md` — personal, gitignored. For the same
  key, the local value wins; the markdown bodies of both files are
  concatenated (team first).

Config is re-read on every skill invocation, so recipes take effect on
the next skill you run — no session restart needed.

## Add project context for every skill

The markdown body (everything after the frontmatter) is injected into
skills as project context:

```markdown
---
---

## Project Context

This is a Ruby on Rails application using PostgreSQL and Redis.

### Conventions

- Follow StandardRB for code style
- Use service objects for business logic

### Build & Test

- `bundle exec rspec` to run tests
- `bin/dev` to start the development server
```

## Move where documents are written

Skills write to `meta/` by default. Override any output directory under
`paths`:

```yaml
---
paths:
  plans: docs/plans
  research_codebase: docs/research
  decisions: docs/adr
  work: docs/backlog
---
```

Other available keys include `research_issues`, `notes`, `prs`,
`validations`, `review_plans`, `review_prs`, `review_work`, and
`templates` (default `.accelerator/templates`). Anything you do not set
keeps its `meta/…` default.

## Swap in your own agents

Skills spawn named subagents for research and review. Point any of them
at your own agent definitions:

```yaml
---
agents:
  reviewer: my-custom-reviewer
  codebase-locator: my-locator
---
```

Recognised keys: `reviewer`, `codebase-locator`, `codebase-analyser`,
`codebase-pattern-finder`, `documents-locator`, `documents-analyser`,
`web-search-researcher`, `browser-locator`, `browser-analyser`.
Unrecognised keys produce a warning.

## Tune review behaviour

```yaml
---
review:
  core_lenses: [architecture, code-quality, test-coverage, correctness]
  disabled_lenses: [portability, compatibility]
  max_inline_comments: 15
  pr_request_changes_severity: major
---
```

See the [Review a pull request](review-a-pr.md) guide for what the
lenses do, and the [Review System](../skills/review-system.md) page for
the built-in catalogue.

## Give one skill extra instructions

Per-skill customisation lives in files, not in `config.md`. Create a
directory named exactly after the skill under `.accelerator/skills/`:

```
.accelerator/skills/
  review-pr/
    context.md        # extra context, injected after project context
    instructions.md   # extra instructions, appended to the prompt
  create-plan/
    instructions.md
```

For example, `.accelerator/skills/create-plan/instructions.md`:

```markdown
Always include a rollback section in every plan.
Plans must name the feature flag guarding the change.
```

Both files are optional and injected as-is on each invocation. The
directory name must match the part after `/accelerator:` exactly — the
`SessionStart` hook warns about unrecognised names.

## Connect a work-item tracker

Set the active integration and the ID pattern local work items use:

```yaml
---
work:
  integration: jira
  id_pattern: "{project}-{number:04d}"
  default_project_code: "PROJ"
jira:
  site: your-subdomain
---
```

Credentials are personal — put them in `.accelerator/config.local.md`,
never the shared file:

```yaml
---
jira:
  email: you@example.com
  token_cmd: "op read op://Private/jira/token"
---
```

For Linear, set `work.integration: linear`. The `init-jira` /
`init-linear` skills walk through this interactively — see
[Sync work items with Jira or Linear](sync-work-items.md).

## Customise document templates

Point a template key at your own file, or eject the default and edit it:

```yaml
---
templates:
  plan: docs/templates/our-plan-format.md
---
```

Or, without config: `/accelerator:configure templates eject plan`
copies the plugin default into `.accelerator/templates/plan.md` for
editing. Resolution order is `templates.<key>` → `paths.templates`
directory → plugin default.

## Customise the visualiser

```yaml
---
visualiser:
  kanban_columns: [backlog, ready, in-progress, review, done]
  idle_timeout: never
---
```

Visualiser keys are read once at server boot — restart the visualiser
to pick up changes.

## Parser limits

The frontmatter parser is deliberately simple:

- Simple scalar values and inline-flow arrays (`[a, b, c]`) only.
- Two levels of nesting at most (`section.key`).
- No YAML comments — a `#` becomes part of the value.

If a value is not picked up, check these limits first, then see the
[FAQ](faq.md#my-configuration-changes-are-not-picked-up).
