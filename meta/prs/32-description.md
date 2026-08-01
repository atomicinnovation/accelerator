---
type: pr-description
id: "32"
title: "Ignore build output under the stale visualiser location"
date: "2026-08-01T13:18:04+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
relates_to: ["work-item:0168"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/32"
pr_number: 32
tags: [build, visualiser, jj, gitignore, hygiene]
revision: "0e08537ac2f797c65904a6a337cd48b8979ba4f0"
repository: "accelerator"
last_updated: "2026-08-01T13:18:04+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Ignore build output under the stale visualiser location

## Why

A build under `skills/visualisation/visualise/` snapshots roughly 119,000 paths
into the jj working copy. Neither existing rule reaches that tree: `cli/target/`
is scoped to the current location, and `/dist/` is root-anchored.

That directory predates the move to `cli/visualiser/`. Only `SKILL.md` and the
four `bin/*.debug.tar.gz` archives under it are tracked; everything generated is
not. Without these rules a bare `jj commit` sweeps ~17 MB of build artefacts into
history — jj's 3 MB snapshot guard refuses only the largest files
(`libaccelerator_visualiser.rlib` at 61 MB, the `dep-graph.bin` files at ~9 MB),
so the tens of thousands of smaller ones go through.

## Change

Two rules, placed with the other build-output ignores:

```
skills/visualisation/visualise/frontend/dist/
skills/visualisation/visualise/server/target/
```

## Notes for reviewers

**Ignore rules alone were not sufficient.** The paths were already snapshotted
into the working-copy commit, and a `.gitignore` entry only prevents future
auto-tracking. `jj file untrack` was needed on both directories to evict them —
which jj permits only once the paths are ignored, so the rules had to land first.
That is a local working-copy operation and produces no diff; this PR is the
`.gitignore` change only.

**The bare `bin/accelerator-visualiser-darwin-arm64` binary is deliberately not
covered.** It is a downloaded artefact rather than build output, jj's snapshot
guard already excludes it at 6.5 MB, and any pattern matching it would also match
the four tracked `.debug.tar.gz` archives — the `git add`-blocking hazard
`.gitignore` already warns about in the comment at its staged-shim rules.

**Untracking those four archives is separate in-flight work** (a
`Untrack the visualiser debug archives` commit exists on another workspace) and is
not touched here. If that lands and removes the stale tree wholesale, these rules
become redundant, harmlessly.
