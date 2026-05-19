---
name: browser-executor
description: Resolves the absolute path of the Playwright executor (run.sh)
  for browser agents. Preloaded by agent definitions that need to invoke
  the executor without self-discovery; not intended for direct user
  invocation.
user-invocable: false
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---

<!--
Maintainer note: this skill uses `user-invocable: false` (hide from the /
menu) rather than `disable-model-invocation: true`, because the latter
blocks preload via subagent `skills:` frontmatter (per Claude Code docs).
Do not change to disable-model-invocation without re-reading the
subagents docs. The same constraint applies to the sibling `paths` skill.
-->

The path below is authoritative for invoking the Playwright executor.
Reference this resolved value from agent bodies — do not run `which run.sh`
or `find` to discover it.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-browser-executor.sh`
