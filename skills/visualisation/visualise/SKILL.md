---
name: visualise
description: Open the accelerator meta visualiser. Launches the companion-window server in the background and returns a URL. Subcommands stop and status manage the running server.
argument-hint: "[stop | status]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/*)
---

# Visualise Meta Directory

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh visualise`

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research meta/research`
**Decisions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions meta/decisions`
**PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs meta/prs`
**Validations directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh validations meta/validations`
**Review plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_plans meta/reviews/plans`
**Review PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs meta/reviews/prs`
**Templates directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh templates .accelerator/templates`
**Work directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work meta/work`
**Work reviews directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_work meta/reviews/work`
**Notes directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes meta/notes`
**Design gaps directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_gaps meta/design-gaps`
**Design inventories directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories meta/design-inventories`
**Tmp directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp .accelerator/tmp`

**Visualiser**: !`${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/visualiser.sh "$ARGUMENTS"`

## Server lifecycle

<!--
Context for Claude only — do not relay to the user verbatim:
The visualiser server is a locally-backgrounded Rust process. It
binds a random high port on 127.0.0.1 and exits automatically when
idle for 30 minutes, when the process that launched the server
exits, or when `/accelerator:visualise stop` is invoked. Re-running
`/accelerator:visualise` while the server is up reuses the
existing instance. Note: "the process that launched the server"
means different things by invocation mode — for the slash command
it's the Claude Code harness; for the CLI wrapper it's the
terminal shell. Don't assume Claude Code specifically.

The dispatcher routes a single argument: empty/`start` launches
the server, `stop` terminates it, `status` probes its lifecycle
files. Output shape varies by subcommand — read it carefully
before relaying to the user.
-->

The user's argument selects the action. Interpret the
`**Visualiser**:` line above according to which subcommand was
invoked:

**No argument (start)** — the line is either:
- `**Visualiser URL**: <url>` — server is running, relay the URL
  to the user. Tell them:
  - Open the URL in a browser to use the visualiser UI.
  - Re-running `/accelerator:visualise` returns the same URL
    while the server is alive.
  - The server auto-exits after 30 minutes idle, or when the
    process that launched it exits. To stop it explicitly, run
    `/accelerator:visualise stop`.
- A JSON `{"error":...}` object — the server isn't running.
  Read the `hint` field and relay the remediation.

**`stop`** — the line is a JSON object describing the result:
- `{"status":"stopped"}` (optionally `"forced":true`) — the
  server has been terminated.
- `{"status":"not_running"}` — there was no server to stop.
- `{"status":"refused",...}` or `{"status":"failed",...}` —
  relay the reason/error to the user.

**`status`** — the line is a JSON object: `{"status":"running"|"stale"|"not_running","url":...,"pid":...}`.
Relay the status (and URL/PID when present) to the user.

**`{"error":"unknown subcommand",...}`** — the user passed an
unrecognised argument. Tell them the valid subcommands are
`stop` and `status` (no argument starts the server).

### Overrides

By default the plugin downloads a verified per-arch binary from
GitHub Releases on first use. Two overrides exist for dev,
air-gapped, or pinned-binary workflows:

1. **Environment variable** (one-shot, shell-scoped):
   `ACCELERATOR_VISUALISER_BIN=<path>`. Bypasses SHA-256
   verification; use for local dev builds.
2. **Config key** (persistent, per-project):

   ```yaml
   ---
   visualiser:
     binary: <absolute or project-relative path>
   ---
   ```

   in `.accelerator/config.md` (team-committed) or
   `.accelerator/config.local.md` (personal, gitignored).
   Relative paths resolve against the project root.

   The team-committed form is trusted on par with the rest of
   the repo — anyone approving a PR that changes it should
   treat the value as code, not data.

The release-binary mirror URL can be overridden via
`ACCELERATOR_VISUALISER_RELEASES_URL=<base-url>` (air-gapped
or proxy-hosted mirrors).

To run the visualiser from a terminal, symlink the CLI wrapper:

**Install command**: !`printf 'ln -s "%s" "%s"' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/cli/accelerator-visualiser" "$HOME/.local/bin/accelerator-visualiser"`

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh visualise`
