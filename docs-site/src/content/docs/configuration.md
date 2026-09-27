---
title: Configuration
---

Accelerator supports project-specific configuration through markdown files with
YAML frontmatter. Configuration allows you to provide project context, customise
agent behaviour, and tune review settings. This page is the reference;
for paste-able examples of common customisations, see the
[Configuration cookbook](guides/configuration-cookbook.md).

## Config Files

| File                              | Scope                   | Purpose                             |
|-----------------------------------|-------------------------|-------------------------------------|
| `.accelerator/config.md`          | Team-shared (committed) | Shared project context and settings |
| `.accelerator/config.local.md`    | Personal (gitignored)   | Personal overrides and preferences  |

Local settings override team settings for the same key. Markdown bodies from
both files are concatenated (team context first, then personal).

The personal file (`.accelerator/config.local.md`) must be mode `0600` or
stricter and must not be a symlink. Otherwise it is ignored with an
`E_LOCAL_PERMS_INSECURE` warning: none of its values are used, since a
personal config file is treated as sensitive-by-convention as a whole
(matching how SSH keys or `.netrc` are handled). Team values in `config.md`
and the `ACCELERATOR_*` environment overrides still resolve, and a command
fails with `E_LOCAL_PERMS_INSECURE` only when nothing usable remains.
Commands that write — `migrate`, `work create`/`update`/`sync`, the Jira and
Linear commands that write to the tracker, `config set` and
`config templates eject --force`/`reset --confirm` — refuse outright, because
their writes would outlive the fix.

The remedy is `chmod 600 .accelerator/config.local.md`. On a filesystem that
cannot honour file modes, keep team values in `config.md` and secrets in the
`ACCELERATOR_*` overrides, and move `config.local.md` aside to run a command
that writes. A file created via `accelerator config set` (which always writes
personal-level values at `0600`) already satisfies this — the check can only
trip on external tampering (a manual `chmod`, a tarball restore, a stray umask
elsewhere).

## File Format

Both files use YAML frontmatter for structured settings and a markdown body for
free-form project context:

```yaml
---
agents:
  reviewer: my-custom-reviewer
review:
  disabled_lenses: [ portability, compatibility ]
  max_inline_comments: 15
  pr_request_changes_severity: major
---

## Project Context

  This is a Ruby on Rails application using PostgreSQL and Redis.

### Conventions
- Follow StandardRB for code style
- Use service objects for business logic
- All API endpoints require authentication

### Build & Test
- `bundle exec rspec` to run tests
- `bin/dev` to start the development server
```

The YAML frontmatter supports `agents` (override which agents skills spawn),
`review` (customise review behaviour), `paths` (override where skills write
output documents), `templates` (point to custom document templates), and
`work` (customise work-item ID filename pattern and default project code).
See `/configure help` for the full key reference.

## Template Management

Templates control the structure of documents produced by skills (plans, ADRs,
codebase research, validation reports, PR descriptions). The configure skill
provides subcommands for managing templates without manually locating plugin
internals:

| Command                                        | Description                                            |
|------------------------------------------------|--------------------------------------------------------|
| `/configure templates list`        | List all templates with resolution source and path     |
| `/configure templates show <key>`  | Display the effective template content                 |
| `/configure templates eject <key>` | Copy plugin default to your templates directory        |
| `/configure templates eject --all` | Eject all templates at once                            |
| `/configure templates diff <key>`  | Show differences between your template and the default |
| `/configure templates reset <key>` | Remove your customisation, revert to plugin default    |

Available template keys: `adr`, `codebase-research`, `design-gap`,
`design-inventory`, `note`, `plan`, `plan-review`, `pr-description`,
`pr-review`, `rca`, `validation`, `work-item`, `work-item-review`.

A typical customisation workflow:

1. `templates list` to see what's available and where each resolves from
2. `templates eject plan` to copy the plugin default to your templates directory
3. Edit the ejected file to match your project's conventions
4. `templates diff plan` to review your changes against the default
5. `templates reset plan` if you want to revert to the plugin default

Templates are resolved in order: config path (`templates.<key>`) → templates
directory (`paths.templates`, default `.accelerator/templates/`) → plugin default.

## Managing Configuration

Run `/configure` to create or view your configuration. The skill
walks you through gathering project context and writes the config file for you.

## How It Works

- A `SessionStart` hook detects config files and injects a summary into the
  session context
- The same summary warns about consent keys: a key such as
  `jira.token_cmd` set in `config.md`, and a `config.local.md` that version
  control tracks or whose tracking cannot be determined. Each warning reaches
  both you, as the start-of-session message, and the session context.
- The tracking check asks the `vcs` sub-binary. In the first session after an
  upgrade, if the binary is not yet cached and cannot be fetched within a few
  seconds, because the network is slow or offline, the check is noted as
  skipped rather than warned about. It runs from the next session, and every
  command that reads a consent key still checks tracking itself on every run.
- Setting `ACCELERATOR_VCS_BIN` makes the session-start check run that binary
  without verifying it, as every dispatch does, so an environment that sets
  it can suppress this warning, though not the commands' own refusals
- Skills read project context at invocation time via the `!` preprocessor
- Config changes take effect on the next skill invocation (no session restart
  needed for skills); the SessionStart summary updates on session restart

## Custom Review Lenses

You can add custom review lenses alongside the 13 built-in code review ones. Place them in
`.accelerator/lenses/` following the `[name]-lens/SKILL.md` convention.
Custom lenses are auto-discovered and included in the lens catalogue. See
the [Review System](skills/review-system.md) for the built-in lens catalogue and
`/configure help` for details and a minimal template.

## Per-Skill Customisation

Beyond global context, you can provide context or instructions targeted at
individual skills by placing files in
`.accelerator/skills/<skill-name>/`:

```
.accelerator/skills/
  review-pr/
    context.md          # Context specific to PR review
    instructions.md     # Additional instructions for PR review
  create-plan/
    instructions.md     # Additional instructions for plan creation
```

- **`context.md`** — Injected after global project context. Use for information
  only one skill needs (e.g., review criteria for `review-pr`, architecture
  context for `create-plan`).
- **`instructions.md`** — Appended to the end of the skill's prompt. Use to add
  steps, enforce conventions, or modify output format. Instructions at the end
  of the prompt typically take precedence when they conflict with earlier
  instructions.

Both files are optional. Directory names must match the skill name exactly (the
part after `/`). The SessionStart hook warns about unrecognised
directory names. See `/configure help` for the full reference.

## Skill reference

For invocation and arguments, see the
[`configure`](reference/skills/config/configure.md) and
[`init`](reference/skills/config/init.md) skill references.
