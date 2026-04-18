---
name: visualise
description: Open the accelerator meta visualiser. Currently prints a placeholder sentinel; the companion-window server is not yet available.
argument-hint: "(no arguments — server not yet available)"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/*)
---

# Visualise Meta Directory

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh visualise`

<!--
The 11 path-key resolutions below are intentional forward-compat
scaffolding for a later phase, where the launcher will take these paths as
inputs to the server's config.json. They are not consumed in this phase;
the only functional payload here is the
`Visualiser URL (not yet running)` line further down. Resolving them
now locks in the preamble shape so the next phase can extend it without
re-editing this block.
-->

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

**Visualiser URL (not yet running)**: !`bash ${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/launch-server.sh`

## Availability

<!--
Context for Claude only — do not relay to the user:
This is a scaffold release. The Rust server that will eventually
back the URL line is added in a later phase. No server is running and no
port is listening.
-->

Tell the user, without referring to phases, sub-phases, or release
numbers: the visualiser UI isn't ready yet — this is a scaffold
release. There's no server to connect to; the `placeholder://` line
above will be replaced by a real URL in a future release. Do not
attempt to open the placeholder in a browser.

To use the same entry point from a terminal (also a placeholder
today), symlink the wrapper onto `$PATH`. Copy the full command
below — the path is pre-resolved for you:

**Install command**: !`printf 'ln -s "%s" "%s"' "${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/cli/accelerator-visualiser" "$HOME/.local/bin/accelerator-visualiser"`

If `accelerator-visualiser` is not found after running that command,
make sure `$HOME/.local/bin` is on your `$PATH` (on macOS you may
need to add `export PATH="$HOME/.local/bin:$PATH"` to your shell rc).

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh visualise`
