---
name: init-linear
description: >
  Set up the Linear integration for this project. Verifies a Linear personal
  API key against the real Linear GraphQL API, lets you pick one team, and
  persists that team plus its WorkflowState catalogue under
  `<paths.integrations>/linear/` (default
  `.accelerator/state/integrations/linear/`). `catalogue.json` is team-shared
  and version-controlled; `viewer.json` is per-developer and gitignored.
  Idempotent: safe to re-run after credential or team changes.
argument-hint: "[--team-id <uuid>]"
disable-model-invocation: true
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear *)
---

# Init Linear

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill init-linear --fail-safe`

> **Configuration**: Set `work.integration: linear` in `.accelerator/config.md`
> to enable auto-scoping. See the
> [`### work` section of `configure/SKILL.md`](../../config/configure/SKILL.md#work)
> for the full reference.

You are setting up the Linear integration for this project. Work through the
steps below in order, stopping to prompt the user only when a value is missing
and cannot be derived from existing configuration.

## Step 1: Verify credentials

Run:

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear init verify
```

Run the bare launcher **directly** as an executable; never prefix it with
`bash`/`sh`/`env` (a wrapper prefix escapes the skill's `allowed-tools`
permission and forces an unnecessary prompt). Credential resolution and
verification fold into this one call; the token is never printed.

If it fails naming `E_NO_TOKEN` (no token found), tell the user:

> No Linear API token found. Generate a personal API key at
> <https://linear.app/settings/account/security> (the value starts with
> `lin_api_`), then add it to `.accelerator/config.local.md` (which is
> gitignored):
>
> ```yaml
> ---
> linear:
>   token_cmd: "op read op://Work/Linear/credential"
> ---
> ```
>
> The key must be stored and sent **without** a `Bearer` prefix.
>
> Re-run `/init-linear` once the token is configured.

Then stop. Do not continue until the token is available.

On success the subcommand emits a JSON document with `outcome: "verified"` plus
`{id, name}`, and writes `.accelerator/state/integrations/linear/viewer.json`.
Print: *"Verified as `<name>` (`<id>`)."* On any other non-zero exit, show the
error and stop — a `Bearer`-prefixed token or an invalid key surfaces here as an
authentication failure.

## Step 2: List teams

Run:

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear init list-teams
```

This emits a JSON document with `outcome: "listed"` and a `.teams` array of
`{id, name, key}`. Present the teams to the user as a readable list (key + name)
and ask which one to scope this project to. If the user passed `--team-id
<uuid>`, skip the prompt and use it.

## Step 3: Discover and persist the catalogue

Run, substituting the chosen team's UUID:

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear init discover --team-id <uuid>
```

On success (`outcome: "discovered"`) the subcommand writes `catalogue.json`
atomically, containing the chosen team's `{id, key, name}` and its WorkflowStates
(`{id, name, type, position}`). Only the selected team's states are persisted
(single-team scoping).

## Step 4: Confirm completion

Print a summary:

```
Linear integration initialised:
  Team:   <key> — <name>
  States: <N> WorkflowStates cached (.accelerator/state/integrations/linear/catalogue.json)
  Viewer: <name> (.accelerator/state/integrations/linear/viewer.json — gitignored)
```

Remind the user to commit
`.accelerator/state/integrations/linear/catalogue.json` so teammates pick up the
shared team + state catalogue without re-running `/init-linear`. (`viewer.json`
is gitignored — each developer runs `/init-linear` to record their own viewer
identity and resolve their own credentials.)

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions init-linear --fail-safe`
