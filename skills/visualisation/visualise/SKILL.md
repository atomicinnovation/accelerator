---
name: visualise
description: Open the accelerator meta visualiser. Launches the companion-window server in the background and returns a URL.
argument-hint: "(no arguments)"
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
**Templates directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh templates meta/templates`
**Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`
**Notes directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes meta/notes`
**Tmp directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp meta/tmp`

**Visualiser**: !`${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/launch-server.sh`

## Server lifecycle

<!--
Context for Claude only — do not relay to the user verbatim:
The visualiser server is a locally-backgrounded Rust process. It
binds a random high port on 127.0.0.1 and exits automatically when
idle for 30 minutes, when the process that launched the server
exits, or when `stop-server.sh` is invoked. Re-running
`/accelerator:visualise` while the server is up reuses the
existing instance. Note: "the process that launched the server"
means different things by invocation mode — for the slash command
it's the Claude Code harness; for the CLI wrapper it's the
terminal shell. Don't assume Claude Code specifically.
-->

The server runs in the background on your local machine. The
`**Visualiser**:` line above renders the URL on success, or a
JSON error line on failure. Tell the user:
- Open the URL in a browser — no HTML UI is served yet; only a
  plain-text placeholder response that confirms the server is up.
- Re-running this command returns the same URL if the server is
  already running.
- The server exits on its own after 30 minutes idle, or when the
  process that launched it exits. To stop it explicitly, run the
  command below.
- If the line above contains a JSON `{"error":...}` object, the
  server isn't running; read the `hint` field in the JSON for
  remediation.

**Stop command**: !`printf 'bash "%s"' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/stop-server.sh"`
**Status command**: !`printf 'bash "%s" status' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/stop-server.sh"`

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

   in `.claude/accelerator.md` (team-committed) or
   `.claude/accelerator.local.md` (personal, gitignored).
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
