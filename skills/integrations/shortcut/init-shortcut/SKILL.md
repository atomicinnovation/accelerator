---
name: init-shortcut
description: >
  Set up the Shortcut integration for this project. Verifies a Shortcut API
  token against the v4 API, lists the workspace's teams by team key, and
  writes every team and workflow to
  `<paths.integrations>/shortcut/catalogue.json` (default
  `.accelerator/state/integrations/shortcut/`), then records the chosen base
  team in `shortcut.team_key`. `catalogue.json` is team-shared and
  version-controlled. Safe to re-run after token or team changes.
argument-hint: "[--team <team-key>]"
disable-model-invocation: true
allowed-tools:
  - Bash(accelerator config *)
  - Bash(accelerator shortcut *)
---

# Init Shortcut

!`accelerator config context --skill init-shortcut --fail-safe`

> **Configuration**: Set `work.integration: shortcut` in
> `.accelerator/config.md` to enable sync. See the
> [`### work` section of `configure/SKILL.md`](../../config/configure/SKILL.md#work)
> for the full reference.

You are setting up the Shortcut integration for this project. Work through the
steps below in order. Stop to ask the user only when a value is missing and no
configuration supplies it.

## Step 1: Verify the token

Run:

```
accelerator shortcut init verify
```

Run the bare launcher **directly** as an executable. Never prefix it with
`bash`, `sh` or `env`: a prefix escapes the skill's `allowed-tools` rule and
forces a permission prompt. The token is never printed.

If it fails naming `E_NO_TOKEN`, tell the user:

> No Shortcut token found. Create an API token at
> <https://app.shortcut.com/settings/account/api-tokens>, then add it to
> `.accelerator/config.local.md`, which is gitignored:
>
> ```yaml
> ---
> shortcut:
>   token_cmd: "op read op://Work/Shortcut/credential"
> ---
> ```
>
> Store the token **without** a `Bearer` prefix. Re-run `/init-shortcut` once
> the token is set.

Then stop.

On success the outcome is `verified`.
The JSON names the `workspace`, the `member` and the token's `scopes`. Print: *"Verified as `<member.mention_name>`
in `<workspace.name>`."* When `read_only` is `true`, also tell the user that a
read-only token supports show, search and pull, but that create, update,
comment, transition and attach will be refused until they use a token with
write scope.

On any other failure, show the `E_*` line from stderr and stop.

## Step 2: List teams

Run:

```
accelerator shortcut init list-teams
```

On the `listed` outcome, `.teams` holds `{key, name, id}` for each team. A
team's **key** is its mention name. Show the teams as a list of key and name,
and ask which one is this project's base team. If the user passed
`--team <team-key>`, use it without asking. When `.complete` is `false`, tell
the user the listing stopped early and some teams may be missing.

## Step 3: Discover and write the catalogue

Run, with the chosen team key:

```
accelerator shortcut init discover --team <team-key>
```

On the `discovered` outcome, `catalogue.json` holds every team and every
workflow with its states. The JSON reports the `workspace`, the base `team`,
and the number of `teams` and `workflows`.

Discover writes nothing when a listing stops before its last page. It names
the listing and says to retry; tell the user that, and stop.

Discover writes the team key into `shortcut.team_key` in
`.accelerator/config.md` when the key is absent. When `shortcut.team_key`
already holds a **different** value, it prints `shortcut.team_key left intact`
and keeps that value. Ask the user whether to replace it. Only if they agree,
run:

```
accelerator shortcut init discover --team <team-key> --force
```

## Step 4: Confirm completion

Print a summary:

```
Shortcut integration initialised:
  Workspace: <workspace.name> (<workspace.slug>)
  Base team: <team.key> — <team.name>
  Teams:     <teams>
  Workflows: <workflows>
  Catalogue: .accelerator/state/integrations/shortcut/catalogue.json
```

Tell the user that the catalogue is committed and shared through version
control. They should commit `catalogue.json`. A teammate should pull it rather
than re-run init. To pick up teams or workflow states added in Shortcut later,
re-run `accelerator shortcut init discover --team <team-key>` and commit the
result.

!`accelerator config instructions init-shortcut --fail-safe`
