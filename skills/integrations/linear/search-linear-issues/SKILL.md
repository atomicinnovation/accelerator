---
name: search-linear-issues
description: >
  Use this skill whenever the user wants to search, list, or filter Linear
  issues — by state, assignee, label, or free text — even if they say 'find',
  'show me', 'what's open', 'list my issues', or similar phrasing rather than
  'search Linear'. Composes a Linear IssueFilter from structured flags, executes
  a cursor-paginated search scoped to the configured team, and renders a summary
  table of the results. Prefer this skill over raw GraphQL whenever the user's
  intent maps to a structured flag.
argument-hint: "[--state NAME] [--assignee NAME] [--label NAME] [--text STR] [--limit 1..250] [--quiet]"
disable-model-invocation: false
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator linear *)
---

# Search Linear Issues

!`accelerator config context --skill search-linear-issues --fail-safe`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`.
> The team is fixed at `/init-linear` time (single-team scoping) — there is no
> per-search `--team` flag. See the
> [`### work` section of `configure/SKILL.md`](../../config/configure/SKILL.md#work)
> for the full reference.

Search the configured Linear team using structured flags that compose into a
Linear `IssueFilter`. Work through the steps below in order.

## Step 1: Parse the flag set

Read the argument string and note each flag:

- `--state NAME` — workflow state name.
- `--assignee VALUE` — a member of the init team, matched on their email,
  then their full name, then their display name. An earlier match wins
  outright: a value that is one member's email is never weighed against
  another's name.
- `--label NAME` — a label of the init team, or a workspace label.
- `--text STR` — free-text match on the issue title.
- `--limit N` — page size (1..250, default 50). Pagination follows every page
  regardless; `--limit` only sets the per-request page size.

Every filter value is resolved, case-insensitively, to its id through the
committed `catalogue.json`, and the search is scoped to the init team for
`--state`, `--label` and `--assignee` alike. Within the team, an active state,
label, member or project wins over archived or disabled ones of the same name;
two active ones are ambiguous. When the catalogue lacks the section a flag
needs, the search fetches it from Linear for this run only — it never writes
the catalogue.

With no catalogued team, a `--text`-only search runs workspace-wide; any other
flag refuses with `E_SEARCH_NO_TEAM`.

## Step 2: Run the search

Run the search subcommand, passing the flags through verbatim:

```
accelerator linear search [flags]
```

Run the bare launcher **directly** as an executable; never prefix it with
`bash`/`sh`/`env` and never pipe its output (a wrapper prefix or a pipe escapes
the skill's `allowed-tools` permission and forces an unnecessary prompt).

The subcommand echoes the composed `IssueFilter` to stderr (`INFO:`) for
auditability and emits a single merged JSON document with all pages under
`.data.issues.nodes`, plus a top-level `outcome` keyword. Under `cap-hit` and
`truncated`, `.data.issues.truncated` is `true`: more pages remained than were
fetched. Branch on the keyword:

- **`results`** — render them (Step 3).
- **`empty`** — tell the user no issues matched.
- **`cap-hit`** — the search reached the discovery `max_pages` cap before
  exhausting the results, so they are a lower bound. Tell the user the results
  are incomplete and that they can raise `linear.pull.max_pages` (or its
  `discovery` override), or set it to `unlimited`, then re-run. This is **not**
  a credential failure.
- **`truncated`** — a transient cutoff (a deadline or wire cutoff), not a
  cap-hit; the results are a lower bound, so suggest retrying.

When stdout carries no JSON document, the search failed: a credential,
transport or filter failure names an `E_*` cause on stderr; show it verbatim.
A filter refusal is printed under `pull filters could not be resolved:`, one
indented line per value, each naming its remedy:

| Code | Meaning |
|------|---------|
| `E_SEARCH_UNKNOWN_{STATE,LABEL,ASSIGNEE}` | the init team carries no such value; refresh the catalogue if it was added in Linear |
| `E_SEARCH_AMBIGUOUS_{STATE,LABEL,ASSIGNEE}` | the value matches more than one active record; use an email for an assignee |
| `E_SEARCH_NO_TEAM` | there is no catalogued base team; run `/accelerator:init-linear` |
| `E_SEARCH_CATALOGUE_DAMAGED` | `catalogue.json` cannot be read; restore it from version control |
| `E_SEARCH_TEAM_UNFETCHED` | the init team was in scope but Linear returned no data for it; check the credential's access |

The exit code classifies the whole refusal:

| Exit | Meaning |
|------|---------|
| `77` | every line is a catalogue gap (`E_SEARCH_NO_TEAM`, `E_SEARCH_CATALOGUE_DAMAGED`, `E_SEARCH_TEAM_UNFETCHED`); fix the catalogue, not the values |
| `78` | every line refuses a `--state` value |
| `89` | any other mix of unknown or ambiguous values |

## Step 3: Render the results

Render a Markdown table with one row per issue:

| Identifier | Title | State | Assignee |
|------------|-------|-------|----------|

Read each row from `.data.issues.nodes[]`: `.identifier`, `.title`,
`.state.name`, `.assignee.name` (show `—` for an unassigned issue). Report the
total count and note if the result was truncated.

!`accelerator config instructions search-linear-issues --fail-safe`
