---
name: init-jira
description: >
  Set up the Jira Cloud integration for this project. Verifies credentials
  against a real Jira Cloud tenant, discovers the tenant's custom-field
  catalogue and project list, and persists the results under
  `<paths.integrations>/jira/` (default `.accelerator/state/integrations/jira/`)
  as team-shared, version-controlled JSON caches. Idempotent: safe to re-run
  after credential or project changes.
argument-hint: "[--site <subdomain>] [--email <addr>] [--refresh-fields] [--list-projects] [--list-fields]"
disable-model-invocation: true
allowed-tools: >
  Bash(${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/*),
  Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*),
  Bash(jq),
  Bash(curl)
---

# Init Jira

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh init-jira`

> **Configuration**: Set `work.integration: jira` and
> `work.default_project_code: <KEY>` in `.accelerator/config.md` to
> enable auto-scoping. See the
> [`### work` section of `configure/SKILL.md`](../../config/configure/SKILL.md#work)
> for the full reference.

You are setting up the Jira Cloud integration for this project. Work through
the steps below in order, stopping to prompt the user only when a value is
missing and cannot be derived from existing configuration.

## Step 0: Parse arguments

Read the argument string (if any) and note:

- `--site <subdomain>` — Jira Cloud subdomain override (e.g. `atomic-innovation`)
- `--email <addr>` — Atlassian account email override
- `--refresh-fields` — skip steps 1–4; re-run field discovery only
- `--list-projects` — print cached projects and exit (no network call)
- `--list-fields` — print cached fields and exit (no network call)

If `--list-projects` was requested, run:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-init-flow.sh list-projects
```

If `--list-fields` was requested, run:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-init-flow.sh list-fields
```

If `--refresh-fields` was requested, skip to Step 5 (field discovery only).

## Step 1: Resolve site

Use the site from `--site` if provided. Otherwise read it from config:

```
${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh jira.site ""
```

If still empty, prompt: *"Enter your Jira Cloud subdomain (the part before
`.atlassian.net`, e.g. `mycompany`):"*

## Step 2: Resolve email

Use `--email` if provided. Otherwise read it from config:

```
${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh jira.email ""
```

If still empty, prompt: *"Enter your Atlassian account email:"*

## Step 3: Resolve token

Run the credential resolver:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-auth-cli.sh
```

If it exits non-zero with `E_NO_TOKEN`, tell the user:

> No Jira API token found. Generate one at
> <https://id.atlassian.com/manage-profile/security/api-tokens>, then add it
> to `.accelerator/config.local.md` (which is gitignored):
>
> ```yaml
> ---
> jira:
>   token_cmd: "op read op://Work/Atlassian/credential"
> ---
> ```
>
> Re-run `/init-jira` once the token is configured.

Then stop. Do not continue with verification until the token is available.

## Step 4: Verify and persist site.json

Run the full initialisation flow (or just verify if re-running mid-setup):

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-init-flow.sh verify
```

On success, `.accelerator/state/integrations/jira/site.json` is written with `{site, accountId}`.
Print: *"Verified as `<accountId>` on `<site>.atlassian.net`."*

On failure, show the error from `jira-request.sh` and stop.

## Step 5: Discover projects and fields

Run:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-init-flow.sh discover
```

This writes `projects.json` (project key/id/name) and `fields.json` (field
id/key/name/slug) atomically. Both files are byte-idempotent — re-running
against an unchanged tenant produces no diff.

If `--refresh-fields` was requested, run only:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-init-flow.sh refresh-fields
```

## Step 6: Default project key

Run:

```
${CLAUDE_PLUGIN_ROOT}/skills/integrations/jira/scripts/jira-init-flow.sh prompt-default
```

If `work.default_project_code` is already set, this is a no-op. Otherwise the
helper prints the available projects and prompts for a selection. Offer to
write the chosen key to `accelerator.md`.

## Step 7: Confirm completion

Print a summary:

```
Jira integration initialised:
  Site:     <site>.atlassian.net
  Fields:   <N> fields cached (.accelerator/state/integrations/jira/fields.json)
  Projects: <M> projects cached (.accelerator/state/integrations/jira/projects.json)
  Default:  <KEY> (work.default_project_code)
```

Remind the user to commit `.accelerator/state/integrations/jira/{fields,projects}.json`
so teammates pick up the shared cache without running `/init-jira` themselves.
(`site.json` is gitignored — each developer runs `/init-jira` to configure their
own credentials.)

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh init-jira`
