---
name: paths
description: Resolves all configured document-discovery paths for the current
  project. Preloaded by agent definitions that need config-driven directory
  locations; not intended for direct user invocation.
user-invocable: false
---

# Configured Paths

The following paths are resolved from the project's Accelerator configuration.
When this skill is preloaded into an agent context, the agent should treat these
values as the authoritative directory locations for all document searches,
overriding any hardcoded defaults.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-all-paths.sh`

If a path key is not listed above, use the plugin default for that key.
