---
name: init-linear
description: >
  Set up the Linear integration for this project. Verifies a Linear personal
  API key against the real Linear GraphQL API, lets you pick one base team,
  and persists the states, labels, members and projects of the base team and
  every synced team, plus the workspace labels, under
  `<paths.integrations>/linear/` (default
  `.accelerator/state/integrations/linear/`). Reports per-team section counts.
  `catalogue.json` is team-shared and version-controlled; `viewer.json` is
  per-developer and gitignored. Idempotent: safe to re-run after credential or
  team changes, and re-running refreshes every synced team.
argument-hint: "[--team-id <uuid>]"
disable-model-invocation: true
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator linear *)
---

# Init Linear

!`accelerator config context --skill init-linear --fail-safe`

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
accelerator linear init verify
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
accelerator linear init list-teams
```

This emits a JSON document with `outcome: "listed"` and a `.teams` array of
`{id, name, key}`. Present the teams to the user as a readable list (key + name)
and ask which one to scope this project to. If the user passed `--team-id
<uuid>`, skip the prompt and use it.

## Step 3: Discover and persist the catalogue

Run, substituting the chosen team's UUID:

```
accelerator linear init discover --team-id <uuid>
```

On success (`outcome: "discovered"`) the subcommand writes `catalogue.json`
atomically. It holds a complete entry (`states`, `labels`, `members`,
`projects`) for the chosen base team and for every **synced team** — each team
already in the catalogue's `teams` array, which sync adds when it imports items
from a team. Other teams the credential can see are never catalogued. Archived
states, labels and projects, and disabled members, are included. The
workspace labels, which belong to no team, are written under the top-level
`labels`. The JSON document reports the base `team` plus each entry's section
counts under `teams`, and the number of `workspaceLabels`.

The catalogue records the name, display name and email of every member of a
synced team, and it is committed, so anyone with read access to the repository
can read them.

States, labels, members or projects added in Linear after init are not picked
up by sync; re-run this step to refresh every synced team.

If the subcommand refuses the existing catalogue as unparseable (for example
an unresolved merge conflict), restore the last good `catalogue.json` from
version control, or resolve the conflict, and re-run. Delete the file only as
a last resort: synced teams are then re-derived from tracked work items on the
next apply sync.

If it prints a `note:` that the file has a `team` but no `teams`, an older
binary may have overwritten the catalogue and erased its synced teams. Restore
the file from version control, or let the next apply sync re-derive them.

The subcommand also writes the discovered team key into `linear.team_key` in
team config (`.accelerator/config.md`), the integration-owned scope key. An
absent key is written automatically. If `linear.team_key` is already set to a
**different** value, the subcommand leaves it intact (it prints
`linear.team_key left intact …`) rather than clobbering a hand-set value. To
adopt the discovered key over an existing one, confirm the change with the user,
then re-run with `--force`:

```
accelerator linear init discover --team-id <uuid> --force
```

## Step 4: Confirm completion

Print a summary:

```
Linear integration initialised:
  Base team: <key> — <name>
  Teams:     one line per entry in `teams`:
             <key>: <states> states, <labels> labels, <members> members, <projects> projects
  Workspace labels: <workspaceLabels>
  Catalogue: .accelerator/state/integrations/linear/catalogue.json
  Viewer: <name> (.accelerator/state/integrations/linear/viewer.json — gitignored)
```

Remind the user that the catalogue is committed and shared. If a teammate has
already refreshed it, pull their change rather than re-running init;
otherwise commit `.accelerator/state/integrations/linear/catalogue.json` so
teammates pick it up. To refresh it later, run
`accelerator linear init discover --team-id <uuid>` and commit the result. (`viewer.json`
is gitignored — each developer runs `/init-linear` to record their own viewer
identity and resolve their own credentials.)

!`accelerator config instructions init-linear --fail-safe`
