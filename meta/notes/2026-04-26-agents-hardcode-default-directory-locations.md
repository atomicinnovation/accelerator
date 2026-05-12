# Tech Debt: Agents hardcode default directory locations rather than reading config

## Problem

Agent definition files (e.g. `agents/documents-locator.md`) embed hardcoded
directory paths in their instructions and example output blocks:

```
meta/research/codebase/
meta/plans/
meta/work/
meta/decisions/
...
```

These paths match the plugin defaults but take no account of userspace
configuration, where operators can remap directories via `config.user.yaml` or
`config.team.yaml`. An agent invoked in a repo that has remapped `meta/work/`
to a different path will still emit the default path in its output and steer
its search at the wrong location.

The `agents/documents-locator.md` file is the most visible case — its directory
tree diagram and example output block were updated during the ticket→work-item
rename (commit on 2026-04-26) but remain static strings rather than
config-driven values.

## Why this is not fixed now

Agent definitions are static Markdown files interpreted directly by the LLM at
invocation time; they have no mechanism to interpolate config values at the
point the file is read. Fixing this properly requires either:

1. a pre-processing step that rewrites agent definitions with resolved config
   values before they are injected into context, or
2. instructing agents to read config themselves at the start of each invocation
   and override any directory assumptions in their own instructions.

Neither approach is trivially compatible with the current harness, so this is
deferred until a future initiative that revisits agent configuration awareness
more broadly.

## Suggested path forward (future phase)

- Survey all agent definitions for hardcoded paths and catalogue which config
  keys govern each path.
- Evaluate option 2 first (self-resolving agents): add a preamble to affected
  agents instructing them to read the relevant config key early in their
  execution and substitute the result for the default path.
- If self-resolution is reliable enough in practice, no harness change is
  needed; otherwise pursue option 1 as a harness-level preprocessing hook.

## References

- Agent under discussion: `agents/documents-locator.md`
- Config system: `scripts/config-read-value.sh`, `scripts/config-common.sh`
- Related: `meta/decisions/ADR-0022-work-item-terminology.md` (rename that
  surfaced this gap)
