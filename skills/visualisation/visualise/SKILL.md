---
name: visualise
description: Open the accelerator meta visualiser. Launches the companion-window server in the background and returns a URL. Subcommands stop and status manage the running server.
argument-hint: "[stop | status]"
disable-model-invocation: true
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/*)
---

# Visualise Meta Directory

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh visualise`

**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research_codebase`
**Decisions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions`
**PR descriptions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs`
**Validations directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh validations`
**Review plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_plans`
**Review PRs directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs`
**Templates directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh templates`
**Work directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work`
**Work reviews directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_work`
**Notes directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh notes`
**Design gaps directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research_design_gaps`
**Design inventories directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research_design_inventories`
**Tmp directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tmp`

**Visualiser**: !`${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/visualiser.sh "$ARGUMENTS"`

## Server lifecycle

<!--
Context for Claude only — do not relay to the user verbatim:
The visualiser server is a locally-backgrounded Rust process. It
binds a random high port on 127.0.0.1 and exits automatically when
idle for 8 hours, when the process that launched the server
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
  - The server auto-exits after the idle window (8 hours by
    default, configurable via `visualiser.idle_timeout` /
    `ACCELERATOR_VISUALISER_IDLE_TIMEOUT`, and disable-able with
    `never`/`0`), or when the process that launched it exits. To
    stop it explicitly, run `/accelerator:visualise stop`.
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

The idle auto-shutdown window is configurable via the
`visualiser.idle_timeout` config key (a humantime duration like
`"8h"`, `"30m"`, `"1h30m"`; `never`/`0` disables it), with the
`ACCELERATOR_VISUALISER_IDLE_TIMEOUT` environment variable as a
one-shot, shell-scoped override (precedence:
env > `visualiser.idle_timeout` > 8h default).

The detail-page `Open in editor` action is configured via the
`visualiser.editor` config key (with `ACCELERATOR_VISUALISER_EDITOR`
as a one-shot env override; precedence env > config). When unset the
button renders disabled. The value is either a **preset key** or a
**custom URL template**:

- **VS Code family** preset keys — `vscode`, `vscode-insiders`,
  `vscodium`, `cursor`, `windsurf` — each opening
  `{scheme}://file{abs}` (a single slash before the absolute path).
- **JetBrains** preset keys — `idea`, `web-storm`, `pycharm`,
  `php-storm`, `goland`, `rubymine`, `clion`, `rd`, `rustrover` — each
  opening `jetbrains://{tag}/navigate/reference?project={project}&path={rel}`.
  The `{project}` name comes from `visualiser.editor_project`
  (env override `ACCELERATOR_VISUALISER_EDITOR_PROJECT`); when unset it
  defaults to the project directory's basename. Ignored by non-JetBrains
  presets.
- **Custom template** — for editors without a preset (e.g. Zed, Sublime),
  set `visualiser.editor` to a URL template. It **must contain at least
  one** `{abs}` (percent-encoded absolute path) or `{rel}` (percent-encoded
  project-relative path) placeholder — a value that cannot reference the
  file is treated as unresolvable and the button renders disabled. Example:
  `zed://file{abs}`. As a safety guard a resolved link whose scheme is
  `javascript`, `data`, `vbscript`, `blob`, or `file` is rejected (disabled);
  any other editor scheme is allowed.

```yaml
---
visualiser:
  editor: cursor
  # editor: web-storm
  # editor_project: myrepo        # JetBrains project name override
  # editor: "zed://file{abs}"     # custom-template escape hatch
---
```

To run the visualiser from a terminal, symlink the CLI wrapper:

**Install command**: !`printf 'ln -s "%s" "%s"' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/cli/accelerator-visualiser" "$HOME/.local/bin/accelerator-visualiser"`

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh visualise`
