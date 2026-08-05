---
title: 'Releases & Compatibility'
---

The stable install is covered in the [README](https://github.com/atomicinnovation/accelerator/blob/main/README.md). This page covers
the prerelease channel (where the newest features land first) and Claude Code
compatibility.

## Prerelease Versions

Prerelease versions (`X.Y.Z-pre.N`) are published to GitHub Releases on every
push to `main`. A separate marketplace file always points to the latest
prerelease. Add it once:

```bash
/plugin marketplace add https://raw.githubusercontent.com/atomicinnovation/accelerator/main/.claude-plugin/marketplace-prerelease.json
/plugin install accelerator@atomic-innovation-prerelease
```

Re-run `/plugin install accelerator@atomic-innovation-prerelease` to pick up a
newer prerelease as they are published.

To return to the stable channel, uninstall the prerelease plugin and remove its
marketplace:

```bash
/plugin uninstall accelerator@atomic-innovation-prerelease
/plugin marketplace remove atomic-innovation-prerelease
/plugin marketplace add atomicinnovation/accelerator
/plugin install accelerator@atomic-innovation
```

If you have linked the CLI onto your `$PATH`, re-point your own link at the
other channel's plugin data directory — it is per-plugin-id, so switching
channels leaves the old link resolving to the uninstalled channel. The same
applies when uninstalling entirely: remove your link. See
[Terminal Invocation](internals.md#terminal-invocation).

## Claude Code compatibility

This plugin relies on Claude Code's subagent `skills:` preload mechanism
to inject configuration context into agents (e.g. `paths`
into the `documents-*` agents, `browser-executor` into the
`browser-*` agents). **Minimum supported Claude Code: v2.1.144.**
Earlier releases may not support the mechanism; later releases that
change subagent skill-preloading semantics will surface the failure via
the agents' Preload guards.
