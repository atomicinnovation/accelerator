---
type: "adr"
id: "ADR-0066"
title: "VCS-agnostic status and log output format"
date: "2026-08-30T23:05:20+00:00"
author: "Toby Clemson"
producer: "create-adr"
status: "accepted"
parent: "work-item:0198"
relates_to: ["adr:ADR-0053", "adr:ADR-0054"]
tags: ["vcs", "cli", "gix", "jj-lib", "status", "log"]
last_updated: "2026-08-30T23:20:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# ADR-0066: VCS-agnostic status and log output format

**Date**: 2026-08-30
**Status**: Accepted
**Author**: Toby Clemson

## Context

`vcs status` and `vcs log` are the last two `vcs` subcommands still shelling out
to the `jj`/`git` binaries (`cli/vcs-adapters/src/subprocess.rs`). Work item 0198
moves them in-process over `gix` and `jj-lib`, removing the runtime dependency on
those binaries being on `PATH`. Neither backend exposes its native CLI rendering
as a library API — jj's template engine, graph-log renderer, and status summary
all live in the `jj-cli` binary crate, not `jj-lib` — so reproducing native text
would mean reimplementing jj's graph engine against library data with no
byte-parity guarantee.

The output has exactly one consumer: the `/commit` skill, which injects `vcs
status`/`vcs log` as free-form orientation for authoring commits. It parses no
fields, relies on no labels or ordering, and never compares git and jj output
(it runs in one repo at a time). The jj-specific richness a neutral format drops
— change-ids, parent/bookmark lines, the graph and working-copy marker,
per-commit author/date — is orientation that consumer does not use. The one
native signal it does benefit from is conflict state, which both backends can
express.

This ADR fixes the single output format both backends render into, before the
adapters are built, because the format changes the text the `/commit` skill
injects and is therefore a cross-cutting decision rather than an implementation
detail.

## Decision Drivers

- The sole consumer is human/LLM-oriented prose, not a machine parser.
- Both `gix` and `jj-lib` must map onto every field from the data they expose.
- Goldens need a deterministic, backend-parity-testable structure.
- Conflict state is the one native signal worth preserving.
- The never-fail contract must survive: `status`/`log` return text on any failure.
- jj `@` (the in-progress working-copy commit) is analogous to git's uncommitted
  tree, not to a point in recorded history.

## Considered Options

1. **Reproduce each backend's native CLI text (byte-parity)** — render `jj
   status`/`jj log` graph output and `git` output faithfully from library data.
2. **A single VCS-agnostic text format** — one renderer over a backend-neutral
   data struct that both adapters populate.
3. **A structured machine format (e.g. JSON)** — emit a parseable document and
   let the consumer format it.

## Decision

We will adopt **option 2**: one VCS-agnostic text format, rendered identically
from git and jj data. Option 1 is rejected — jj's renderer is `jj-cli`-layer, the
effort is disproportionate, and there is no parity guarantee. Option 3 is rejected
— the consumer is a prompt reading prose; no consumer parses the output, so a
structured format is unused ceremony.

### Status format

```
Branch: <name>
<N> changed[, <K> conflicted]
  <change-type>  <path>
  ...
```

- **Header**: one neutral `Branch:` line. The value is the git branch, or the
  bookmark(s) on the jj working-copy commit — when jj carries more than one, they
  are comma-separated in byte order. It is `(none)` when detached (git) or the
  working-copy commit carries no bookmark (jj, the common case).
- **Summary**: `<N> changed`, the total changed-file count. When any file is
  conflicted, append `, <K> conflicted` — the explicit, summary-level conflict
  indicator; `K` counts a subset of `N` (a conflicted file is also a changed
  file, listed once with the `conflicted` type). When `N` is zero, the whole
  body is the single line `No changes`.
- **File list**: one `  <change-type>  <path>` line per changed file, indented
  two spaces, sorted by repo-relative path (byte order). Change types are the
  closed set **`added`, `modified`, `deleted`, `untracked`, `conflicted`**.
- **Staging collapses**: a git staged change renders as its change type, with no
  staged/worktree distinction (jj has no staging area).
- **Renames are not a distinct type**: a rename surfaces as `deleted` (old path)
  plus `added` (new path), keeping both backends aligned and the set at five.

### Log format

```
<short-id> <subject>
... (up to five)
```

- A flat list of up to **five** most recent commits, newest first. No author, no
  date, no graph glyphs (`@`/`○`/`◆`/`│`), no DAG.
- Each line is `<short-id> <subject>`: git renders the abbreviated commit id, jj
  the abbreviated change id. Goldens normalise the id value by mask; the exact
  abbreviation width is an implementation choice, not fixed here.
- The walk is first-parent ancestry from git `HEAD`, and from the jj working-copy
  commit's first parent. jj's working-copy commit (`@`) and virtual root are
  excluded — `@` is in-progress work, not recorded history.
- An empty subject renders as `(no description)`. A repository with no commits
  renders the single line `No commits` (a legitimate empty state, not a failure).

### Fallback and failure

- On any adapter failure, each subcommand returns its backend-neutral literal:
  **`(status unavailable)`** / **`(log unavailable)`**. The functions never fail
  (they return `String`, no error channel), matching the shell's original
  `2>/dev/null || echo` behaviour.
- The failed adapter's identity (`gix`/`jj-lib`) surfaces only on the
  `ACCELERATOR_LOG` diagnostic path, not in the user-facing fallback text.
- This internal adapter-failure domain is distinct from the launcher's
  `--fail-safe` flag, which handles fetch/dispatch failures.

## Consequences

### Positive

- No runtime dependency on `jj`/`git` on `PATH` for these two subcommands.
- One renderer over one neutral struct — simpler to build and test than two
  native-shaped reproductions, and parity-testable across backends.
- Conflict state — the one signal the consumer benefits from — is preserved and
  made summary-level explicit.

### Negative

- Drops jj-native richness (change-ids, operation log, graph, working-copy
  marker, per-commit author/date). A future consumer needing it gets a separate
  structured subcommand, not a re-enrichment of these two.
- Behaviour changes from today's subprocess output: untracked and modified files
  now appear in status (git status was `diff --cached --stat`, staged-only);
  staged is no longer distinguished; jj status no longer snapshots-and-writes the
  working copy, reporting state as of the last operation instead.
- Several status goldens that are empty today (`clean-git`, both ahead/behind,
  `detached-head-git`) become non-empty under an always-present header.

### Neutral

- Volatile id values are normalised by the committed mask set; the format's line
  structure is what goldens pin.
- jj `@`/root exclusion means a fresh jj repo and a fresh git repo with the same
  recorded history produce the same log lines.

## References

- `meta/work/0198-vcs-agnostic-status-log-renderer.md` — the owning work item
- `meta/research/codebase/2026-08-30-0198-vcs-agnostic-status-log-renderer.md` — grounding research
- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md` — the adapter pattern
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md` — the CLI shape
- `meta/work/0169-vcs-subdomain-and-hooks-migration.md` — the original subprocess choice this revisits
