# Internals

## The `meta/` Directory

Every project using Accelerator gets a `meta/` directory (by default) that
serves as persistent state for the development workflow. Each skill reads from
and writes to predictable paths within it. Run
[`/init`](configuration.md#init) to create all directories up
front, or let skills create them on first use.
These paths can be overridden via the `paths` configuration section:

`research/` is itself subdivided into four subcategories — codebase
research, issue/RCA research, design inventories, and design gaps:

| Directory                       | Purpose                                                        | Written by                                                   |
|---------------------------------|----------------------------------------------------------------|--------------------------------------------------------------|
| `research/`                     | (parent — see subcategories below)                             | —                                                            |
| `  ├─ codebase/`                | Codebase research findings with YAML frontmatter               | `research-codebase`                                          |
| `  ├─ issues/`                  | Issue / RCA research findings                                  | `research-issue`                                             |
| `  ├─ design-inventories/`      | Per-source design inventory snapshots (markdown + screenshots) | `inventory-design`                                           |
| `  └─ design-gaps/`             | Design-gap analysis artefacts                                  | `analyse-design-gaps`                                        |
| `plans/`                        | Implementation plans with phased changes                       | `create-plan`                                                |
| `decisions/`                    | Architecture decision records (ADRs)                           | `create-adr`, `extract-adrs`, `review-adr`                   |
| `reviews/`                      | Review summaries and per-lens results                          | `review-pr`, `review-plan`                                   |
| `validations/`                  | Plan validation reports                                        | `validate-plan`                                              |
| `prs/`                          | PR descriptions                                                | `describe-pr`                                                |
| `work/`                         | Work item files referenced by planning                         | `create-work-item`, `extract-work-items`, `update-work-item` |
| `notes/`                        | Notes and working documents                                    | `create-note`                                                |

This approach means:

- No skill assumes access to another skill's conversation history
- Work survives session boundaries and context compaction
- Plans can be resumed after interruption (implement-plan picks up from the
  first unchecked item)
- Artefacts are structured and machine-parseable (YAML frontmatter, JSON
  schemas)

## Agents

Accelerator uses specialised subagents to keep the main context lean. Each
agent runs in its own context window with restricted tools, returning only a
focused summary to the parent:

| Agent                       | Role                                                              | Tools                                                                                                                                                                                                                                               |
|-----------------------------|-------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| **codebase-locator**        | Finds files and components by description                         | Grep, Glob, LS                                                                                                                                                                                                                                      |
| **codebase-analyser**       | Analyses implementation details of specific components            | Read, Grep, Glob, LS                                                                                                                                                                                                                                |
| **codebase-pattern-finder** | Finds similar implementations and usage examples                  | Read, Grep, Glob, LS                                                                                                                                                                                                                                |
| **documents-locator**       | Discovers relevant documents in configured directories            | Grep, Glob, LS                                                                                                                                                                                                                                      |
| **documents-analyser**      | Extracts insights from meta documents                             | Read, Grep, Glob, LS                                                                                                                                                                                                                                |
| **reviewer**                | Evaluates code/plans through a specific quality lens              | Read, Grep, Glob, LS                                                                                                                                                                                                                                |
| **web-search-researcher**   | Researches external documentation and resources                   | WebSearch, WebFetch, Read, Grep, Glob, LS                                                                                                                                                                                                           |
| **browser-locator**         | Locates routes/screens/components in a running app via Playwright | `Bash(run.sh navigate)`, `Bash(run.sh snapshot)`                   |
| **browser-analyser**        | Analyses screens, captures state and screenshots via Playwright   | `Bash(run.sh navigate\|snapshot\|screenshot\|evaluate\|click\|type\|wait_for)` |

The separation between locators (find, no Read) and analysers (understand, with
Read) is deliberate: it prevents any single agent from needing to both search
broadly and read deeply, keeping each agent's context bounded.

`browser-*` agents drive Playwright through the skill-shipped executor
(`run.sh`), a Bash wrapper around a Node.js TCP daemon that runs Chromium.
No MCP server is required. See `skills/design/inventory-design/PROTOCOL.md`
for the executor wire protocol.

## VCS Detection

Accelerator automatically detects whether a repository uses git or
[jujutsu (jj)](https://github.com/jj-vcs/jj) and adapts its behaviour
accordingly. A `SessionStart` hook inspects the working directory for `.jj/` and
`.git/` directories, injecting VCS-specific context (command references and
conventions) into the session. Detection also recognises git **linked
worktrees** — where `.git` is a file (a `gitdir:` pointer) rather than a
directory — so worktree-based sessions are detected just like plain checkouts. A
complementary `PreToolUse` guard warns when raw git commands are used in a
jujutsu repository.

This means all VCS-aware skills — `commit`, `respond-to-pr`, and ad-hoc
interactions — use the correct CLI commands without manual configuration. The
detection covers three modes:

| Mode               | Detected when      | VCS commands used |
|--------------------|--------------------|-------------------|
| **git**            | `.git/` only       | `git`             |
| **jj (colocated)** | `.jj/` and `.git/` | `jj`              |
| **jj (pure)**      | `.jj/` only        | `jj`              |

## Terminal Invocation

Accelerator's CLI can be run from an ordinary terminal, not only from inside a
Claude Code session. Because the plugin is installed into a version-scoped
directory that moves on every upgrade, the supported route is a two-hop chain:

```
~/.local/bin/accelerator                # you create this, once
  -> <plugin data>/bin/accelerator      # a SessionStart hook refreshes this
    -> <plugin root>/bin/accelerator    # version-pinned, moves on upgrade
```

The hop you create points at a target that never moves, so it never needs
re-running. The hop that must track the version is owned by
`hooks/launcher-link-refresh.sh`, which refreshes it at every session start.
This requires Claude Code **v2.1.78 or later** (the release that added the
plugin data directory). Below that, link `<plugin root>/bin/accelerator`
directly and re-run the link after each upgrade.

**Supported platforms**: macOS or Linux (including WSL) on x86-64 or arm64. The
CLI refuses anything else, so Git Bash, Cygwin, armv7, riscv64 and ppc64le are
out of scope. The Linux artifacts are statically linked (musl), so there is no
glibc floor. Under WSL the plugin data directory must sit on the Linux
filesystem — a `/mnt/<drive>` DrvFs path cannot hold symlinks.

### Setting it up

First, find the path the hook maintains. It is printed by the hook whenever it
re-points the link, and otherwise discoverable:

```bash
CLAUDE_DIR="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
ls -d "$CLAUDE_DIR"/plugins/data/*accelerator*/bin/accelerator
```

Typical results, one per channel:

| Channel     | Typical path                                                       |
|-------------|--------------------------------------------------------------------|
| Stable      | `~/.claude/plugins/data/accelerator-atomic-innovation/bin/accelerator` |
| Prerelease  | `~/.claude/plugins/data/accelerator-atomic-innovation-prerelease/bin/accelerator` |

These are examples rather than a contract: `~/.claude` moves with
`CLAUDE_CONFIG_DIR`, and the directory name is derived from the plugin
identifier. Prefer the discovery command, or the path the hook printed.

Verify it exists and points somewhere real before linking anything:

```bash
ls -l <discovered path>
```

If it is missing, the hook has not run — start a Claude Code session first, and
check your Claude Code version against the floor above.

Then create your own hop:

```bash
mkdir -p ~/.local/bin
ln -sfn <discovered path> ~/.local/bin/accelerator
```

The `mkdir -p` is not optional; `~/.local/bin` frequently does not exist, and
`ln` then fails with `No such file or directory`. Prefer a directory you own
that is not group-writable: this command fetches and executes signed binaries,
so its `PATH` entry is part of the trust chain, and Homebrew-managed
`/usr/local/bin` is often group-writable.

`~/.local/bin` is not on `PATH` by default on macOS, and some Linux
distributions add it only in a login shell. Open a new shell and check:

```bash
command -v accelerator
```

If that prints nothing, the directory is not on your `PATH`. In a POSIX shell:

```bash
case ":$PATH:" in *":$HOME/.local/bin:"*) echo on ;; *) echo missing ;; esac
```

Add it in your shell profile. In fish the diagnostic above is a syntax error and
the fix is `fish_add_path ~/.local/bin`.

If you track both channels, link the second under a different name — for
example `accelerator-pre` — so the two do not collide.

### Which installation am I running?

The link is opaque, so ask the CLI:

```bash
accelerator version                          # the resolved installation
readlink ~/.local/bin/accelerator            # your hop
readlink "$(readlink ~/.local/bin/accelerator)"   # the plugin-owned hop
```

This matters because the plugin-owned hop is **channel-global with
last-session-wins semantics**. A session started before an upgrade re-points it
to the older installation for every terminal user and every concurrent session,
not just itself — and what it selects is the whole installation: the bootstrap,
the vendored verifier, the release key and the launcher cache. So a stale
session can put your terminal command back on an installation old enough to
predate a security fix, for as long as Claude Code retains the old directory
(observed at roughly two weeks, though that is Claude Code's behaviour rather
than something this plugin controls). `/reload-plugins` corrects it immediately;
the next session corrects it anyway.

### Offline, mirrored and read-only installs

The terminal surface assumes a cache already populated by a Claude Code session.
A first run against an empty cache needs network access to the artifact host and
has no degraded mode. Two environment variables cover the awkward cases:

| Variable                       | Effect                                          |
|--------------------------------|-------------------------------------------------|
| `ACCELERATOR_CACHE_DIR`        | Where the launcher is staged and executed from  |
| `ACCELERATOR_RELEASE_BASE_URL` | Where release artifacts are fetched from        |

Both are trust-root inputs rather than ordinary conveniences. The cache
directory is where the bootstrap writes and *executes* a probe file, so point it
at a directory you own and that is not group-writable. The release base URL
should be a host you trust not to serve an older signed release: the cache key
carries no content hash, so a mirror can hand back an older validly-signed
launcher for the current version.

`ACCELERATOR_PLUGIN_ROOT` is **exported by** the bootstrap for the launcher it
runs; it is never read as an input. Setting it has no effect.

### Removing it

Uninstalling the plugin stops the hook that maintains the second hop, but leaves
your own link behind as a broken `accelerator` on `PATH`. Remove it yourself:

```bash
rm ~/.local/bin/accelerator
```

---

[← Visualiser](visualiser.md) · [Docs home](../README.md#documentation) · [Configuration →](configuration.md)
