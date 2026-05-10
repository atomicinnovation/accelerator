---
name: paths
description: Resolves all configured document-discovery paths for the current
  project. Preloaded by agent definitions that need config-driven directory
  locations; not intended for direct user invocation.
user-invocable: false
allowed-tools: 
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

<!--
Maintainer note: this skill uses `user-invocable: false` (hide from the /
menu) rather than `disable-model-invocation: true`, because the latter blocks
preload via subagent `skills:` frontmatter (per Claude Code docs). Do not
change to disable-model-invocation without re-reading the subagents docs.
-->

These paths are authoritative for all document searches in this project. If
a key is missing from the resolved list below (e.g. bang preprocessing was
disabled or failed), fall back to the plugin default shown in the legend.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-all-paths.sh`

## Path legend

What lives at each path key, with the plugin default if no override is set:

- `plans` — implementation plans for specific work items (default: `meta/plans`)
- `research_codebase` — codebase research outputs on specific work items (default: `meta/research/codebase`)
- `research_issues` — issue / RCA research outputs (default: `meta/research/issues`)
- `research_design_inventories` — design-inventory artifacts, one directory per snapshot (default: `meta/research/design-inventories`)
- `research_design_gaps` — design-gap analysis artifacts (default: `meta/research/design-gaps`)
- `decisions` — architectural decision records for the codebase (default: `meta/decisions`)
- `prs` — PR descriptions for landed changes (default: `meta/prs`)
- `validations` — plan validation reports (default: `meta/validations`)
- `review_plans` — reviews of implementation plans (default: `meta/reviews/plans`)
- `review_prs` — reviews of PR descriptions (default: `meta/reviews/prs`)
- `review_work` — reviews of work items (default: `meta/reviews/work`)
- `work` — work items, often `NNNN-title.md` (default: `meta/work`)
- `notes` — meeting notes, discussions, ad-hoc context (default: `meta/notes`)
- `global` — cross-repo / org-wide information (default: `meta/global`)
