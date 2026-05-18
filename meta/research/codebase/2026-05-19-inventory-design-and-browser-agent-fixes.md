---
date: 2026-05-19T08:56:33+01:00
researcher: Toby Clemson
git_commit: ee201256c147a4f4e8b8f7427f9292fd79925767
branch: nsvsowxkytlr (unnamed jj change, parent of main)
repository: accelerator
topic: "Fixes for the inventory-design skill and the browser-locator / browser-analyser agents"
tags: [research, codebase, inventory-design, browser-locator, browser-analyser, playwright, config-read-path]
status: complete
last_updated: 2026-05-19
last_updated_by: Toby Clemson
---

# Research: Fixes for the inventory-design skill and the browser-locator / browser-analyser agents

**Date**: 2026-05-19T08:56:33+01:00
**Researcher**: Toby Clemson
**Git Commit**: ee201256c147a4f4e8b8f7427f9292fd79925767
**Branch**: nsvsowxkytlr (unnamed jj change)
**Repository**: accelerator

## Research Question

The 2026-05-19 `/accelerator:inventory-design` run surfaced four distinct
issues, captured as notes under `meta/notes/2026-05-19-*.md`:

1. Browser agents self-discover the Playwright executor with `find /`.
2. `browser-locator` cannot enumerate routes with the toolset it is allowed.
3. `config-read-path.sh` warns "unknown key" for `design_inventories` /
   `design_gaps`.
4. The Playwright daemon dies between Claude Code turns because its
   `--owner-pid` is the ephemeral `run.sh` shell.

For each, what is the right fix — and specifically, how should the "skill
hands paths to agent" contract follow the precedent set by the
`documents-locator` agent in work item 0052?

## Summary

All four issues are variants of the same architectural failure mode:
**implicit contracts between a skill and the agent / process it spawns**.
The skill knows the value (executor path, daemon-ownership posture),
but does not hand it over; the spawned thing then either
self-discovers (badly), guesses (wrongly), or dies (silently).

The precedent already exists in the codebase for fixing this cleanly:
work item 0052 made `documents-locator` config-driven by adding a
`skills: [accelerator:paths]` entry to the agent's frontmatter. At spawn
time the harness bang-preprocesses the `paths` skill — which calls
`scripts/config-read-all-paths.sh` — and injects the resolved values into
the agent's context as a "## Configured Paths" block, *before* the agent
acts. Resolution happens deterministically, not via LLM
instruction-following.

The four fixes are:

1. **Executor path** — give the browser agents a preloaded "skill"
   (analogous to `accelerator:paths`) that emits the absolute path of
   `run.sh`. Caller-side prompt injection is the lower-friction backup.
2. **Route enumeration** — add a `links` command to the Playwright
   executor (returns `{text, href, role}` for every anchor) and allow it
   for the locator. The accessibility snapshot does not surface URLs;
   this is structural, not policy.
3. **`design_inventories` / `design_gaps` keys** — add the missing
   entries to `scripts/config-defaults.sh` (`PATH_KEYS` + `PATH_DEFAULTS`
   index-aligned arrays), then update the four test assertions. There is
   a name-clash to resolve: migration 0004 renamed these keys to
   `research_design_inventories` / `research_design_gaps`. Either alias
   the old names or rename the four skill call sites.
4. **Daemon owner-PID** — pass `--owner-pid 0` when invoked from the
   Claude Code harness (detected by an env var), disabling the
   owner-death watcher. The existing test suite already uses this mode,
   so the production code is the only un-tested path.

## Detailed Findings

### Issue 1 — Browser agents self-discover the Playwright executor

**Note**: `meta/notes/2026-05-19-browser-agents-self-discover-playwright-executor.md`

The agents reference `run.sh` as a bare command. Concrete evidence:

- `agents/browser-locator.md:36-47` —

  ```
  Use the Playwright executor (`run.sh`) as the primary browser interface:

      run.sh navigate '{"url":"<url>"}'
      run.sh snapshot
  ```

  Frontmatter `tools: Bash` (lines 1-8) — no `skills:` key.

- `agents/browser-analyser.md:16-31` — same pattern, bare `run.sh`.

The actual executor is at:

```
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/playwright/run.sh
```

The `inventory-design` skill resolves this path explicitly in its own
Bash blocks (Steps 5 and 12: `skills/design/inventory-design/SKILL.md:138-141`
and `:262-264`) but **never hands it to the agents** at Step 8
(`skills/design/inventory-design/SKILL.md:162-202`). The Step 8 prompt
just says "spawn `{browser locator agent}`" via the agent-name
placeholder.

The agent's first runtime action becomes `which run.sh || find /
-name "run.sh" …` — a full-filesystem scan and a guessing game.

#### Fix: follow the documents-locator precedent

Work item 0052 added a `skills:` frontmatter key to `documents-locator`
(`agents/documents-locator.md:10-11`):

```yaml
skills:
  - accelerator:paths
```

The harness then bang-preprocesses `skills/config/paths/SKILL.md` and
substitutes the output of `scripts/config-read-all-paths.sh` into the
agent's context as a "## Configured Paths" block *before the agent
acts* (work item rationale at
`meta/work/0052-make-documents-locator-paths-config-driven.md:38-90`).

`skills/config/paths/SKILL.md:1-22` is the preloaded shape that needs to
be mirrored — note `user-invocable: false` (not
`disable-model-invocation: true`, which would block preload):

```yaml
---
name: paths
description: Resolves all configured document-discovery paths for the current
  project. Preloaded by agent definitions that need config-driven directory
  locations; not intended for direct user invocation.
user-invocable: false
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
---
…
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-all-paths.sh`
```

Mirror for browser agents:

- Add a new preloaded skill, e.g.
  `skills/config/browser-executor/SKILL.md` (`user-invocable: false`),
  whose body calls a new
  `scripts/config-read-browser-executor.sh` script that prints
  something like:

  ```
  ## Browser Executor

  - run.sh: ${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/playwright/run.sh
  ```

- Amend `agents/browser-locator.md` and `agents/browser-analyser.md`
  frontmatter to add:

  ```yaml
  skills:
    - accelerator:browser-executor
  ```

- Rewrite the agent body's "Tools" sections to reference the resolved
  path from the preloaded block rather than the bare command.

**Pragmatic alternative**: since the executor path is *fixed* under the
plugin cache (it does not depend on user config), caller-side prompt
injection is also acceptable — `skills/design/inventory-design/SKILL.md`
can build a small Step-8 prompt template like:

```
Use this executor: ${PLAYWRIGHT_RUN_SH}
```

But the `skills:` mechanism is the published precedent and works
identically for every caller. Prefer that for consistency with
documents-locator and to avoid the per-caller patch the note already
describes as the current workaround.

#### Code references

- `agents/browser-locator.md:1-8, 36-47`
- `agents/browser-analyser.md:1-8, 16-31`
- `agents/documents-locator.md:10-11, 20-26` (precedent — `skills:`)
- `skills/config/paths/SKILL.md:1-22` (preloaded skill shape)
- `scripts/config-read-all-paths.sh:14, 24-33`
- `skills/design/inventory-design/SKILL.md:138-141, 162-202, 262-264`

---

### Issue 2 — `browser-locator` cannot enumerate routes

**Note**: `meta/notes/2026-05-19-browser-locator-cannot-enumerate-routes.md`

The locator's stated job is "enumerate routes, screens, and component
presence" (`agents/browser-locator.md` description, lines 2-6), but its
two allowed commands cannot do this on a client-side single-page app:

- `run.sh navigate` requires a URL the caller already knows
  (`skills/design/inventory-design/scripts/playwright/lib/daemon.js:152-156`).
- `run.sh snapshot` returns `page.accessibility.snapshot()`
  (`scripts/playwright/lib/daemon.js:158-161`) which exposes only ARIA
  fields — `role` and `name`. No `href`, no `data-*`, no URL.

The two commands that could surface URLs are explicitly forbidden in
the agent body (`agents/browser-locator.md:90-103`):

```
- Do not use `run.sh evaluate` — no JavaScript execution
- Do not use `run.sh click` or `run.sh type` — no interaction
```

On the visualiser specifically every URL returns the same 1073-byte
SPA shell with HTTP 200, so navigation to a hallucinated path is
indistinguishable from a real one. The agent ends up guessing
`/work-items` while the real path is `/library/work-items`.

#### Fix: add a `links` command to the executor

Cleanest option (option 2 in the note). Add a new command to
`scripts/playwright/lib/daemon.js`'s switch (alongside the cases at
lines 113-224):

```javascript
case 'links': {
  const links = await page.evaluate(() =>
    Array.from(document.querySelectorAll('a[href]'))
      .map(a => ({
        text: (a.textContent || '').trim(),
        href: a.getAttribute('href'),
        role: a.getAttribute('role') || 'link'
      }))
  );
  return { protocol: PROTOCOL, links };
}
```

Then loosen the locator's allowlist to permit `links` (and keep
`evaluate`, `click`, `type` forbidden so the bounded-surface posture
is preserved). The agent body update is mechanical: add a "links"
example in the Tools section and add a "discover hrefs via
`run.sh links`" step in the Search Strategy.

This is the "do once, fix forever" option. The pragmatic near-term
unblocker (option 4 in the note) is to have `inventory-design` itself
call `run.sh evaluate` to extract routes *before* spawning the
locator, reducing the agent's role from "discover" to "describe".

#### Code references

- `agents/browser-locator.md:2-6, 36-47, 90-103`
- `scripts/playwright/lib/daemon.js:113-224` (command dispatch)
- `scripts/playwright/lib/daemon.js:158-161` (current snapshot —
  accessibility-tree only)
- `scripts/playwright/lib/daemon.js:178-182` (current evaluate
  implementation — the helper for the new `links` case would mirror
  the pattern)

---

### Issue 3 — `config-read-path.sh` warns "unknown key"

**Note**: `meta/notes/2026-05-19-config-read-path-missing-design-keys.md`

`scripts/config-read-path.sh:17-42` looks up `paths.${key}` in the
`PATH_KEYS` / `PATH_DEFAULTS` index-aligned arrays sourced from
`scripts/config-defaults.sh`. The arrays today have 17 entries
(`config-defaults.sh:26-44, 46-64`). They include
`paths.research_design_inventories` and `paths.research_design_gaps`
(after migration 0004) but **not** the bare names
`paths.design_inventories` / `paths.design_gaps` that four skill
preambles still call:

- `skills/design/inventory-design/SKILL.md:30` —
  `config-read-path.sh design_inventories`
- `skills/design/analyse-design-gaps/SKILL.md:27` —
  `config-read-path.sh design_inventories`
- `skills/design/analyse-design-gaps/SKILL.md:28` —
  `config-read-path.sh design_gaps`
- `skills/config/init/SKILL.md:31-32` — both keys

With no `$2` default at the call site, the script falls into the
"look up centralised default" branch, finds no match, prints the
warning to stderr, sets `default=""`, and execs `config-read-value.sh
paths.<key> ""`. The stderr warning bleeds into the rendered skill
preamble where a path should be.

There is a **name-clash** with migration 0004
(`skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:414-415`)
that explicitly renamed the user-config keys to the
`research_design_*` form. So two fix shapes exist:

**Fix shape A — add the bare keys as new defaults.** The note's
proposal. After `config-defaults.sh:43` (`"paths.research_issues"`) add
`"paths.design_inventories"` and `"paths.design_gaps"`; matching
defaults after `config-defaults.sh:63` (`"meta/research/issues"`).
**Important:** use `meta/research/design-inventories` and
`meta/research/design-gaps` for the defaults, not the
`meta/design-inventories` proposed in the note — the post-migration
canonical layout puts these under `meta/research/`.

This makes the skills' bare-name calls resolve correctly, but it
*aliases* the keys (two keys, same effective default). That muddies
the migration 0004 invariant.

**Fix shape B — rename the four skill call sites.** Change all four
SKILL.md preambles to `config-read-path.sh research_design_inventories`
and `…research_design_gaps`. No `config-defaults.sh` change. This is
the migration-consistent fix: a single canonical key.

`scripts/test-design.sh:13-16, 32-35` `assert_contains` will still
pass under shape B because `research_design_inventories` is a
superstring of `design_inventories`.

**Recommendation**: shape B. The bug exists because the skills weren't
updated when migration 0004 ran; the right fix is to finish that
migration, not to alias around it.

Either shape requires updating `scripts/test-config.sh` length
assertions if `PATH_KEYS` length changes (shape A only: lines 2444,
2446, 2451, 2453; bump `"17"` → `"19"` and extend the expected
key/default strings).

#### Code references

- `scripts/config-read-path.sh:17-42`
- `scripts/config-defaults.sh:26-44, 46-64`
- `scripts/config-read-all-paths.sh:14, 24-33`
- `scripts/config-dump.sh:175-181`
- `scripts/test-config.sh:2443-2455, 3074-3162, 3164-3204, 4026-4045,
  5791-5795`
- `scripts/test-design.sh:13-16, 32-35`
- `skills/design/inventory-design/SKILL.md:30`
- `skills/design/analyse-design-gaps/SKILL.md:27-28`
- `skills/config/init/SKILL.md:31-32`
- `skills/config/init/scripts/init.sh:18-31` (uses the old bare keys
  with explicit defaults — already migration-compatible)
- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:414-415`

---

### Issue 4 — Playwright daemon `--owner-pid $$` dies with ephemeral shells

**Note**: `meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md`

`skills/design/inventory-design/scripts/playwright/run.sh:100-113`
spawns the daemon with the launcher shell's own PID as owner:

```bash
nohup node "$SCRIPT_DIR/run.js" daemon \
  --state-dir "$STATE_DIR" \
  --owner-pid "$$" \
  >> "$BOOTSTRAP_LOG" 2>&1 &
DAEMON_PID=$!
disown "$DAEMON_PID" 2>/dev/null || true
```

`run.sh` then `exec`s into `node run.js "$@"` at line 131; the original
bash `$$` is replaced and disappears.

`scripts/playwright/lib/daemon.js:88-97` polls every
`OWNER_POLL_MS` (default 60 000 ms — daemon.js:21):

```javascript
const ownerWatcher = ownerPid > 0 ? setInterval(() => {
  try { process.kill(ownerPid, 0); }
  catch { shutdown('owner-exited', { owner_pid: ownerPid }); }
}, OWNER_POLL_MS) : null;
```

Under the Claude Code Bash tool (a fresh shell per invocation), the
launcher PID has already exited by the time the watcher's first tick
fires. The daemon shuts down with reason `owner-exited`, and the
*next* `run.sh navigate` call bootstraps a fresh daemon with the same
self-destructing posture — losing any browser context the previous
agent established.

The existing test suite at
`scripts/playwright/lib/daemon.test.js:77, 99, 120, 141, 164` invokes
the daemon with `--owner-pid 0`, which is the disabled-watcher branch
of daemon.js:88 — so the failing path is **untested**.

#### Fix

Option 1 from the note. In `run.sh`, before line 110, detect the
harness via an env var and switch the value:

```bash
owner_pid="$$"
if [ -n "${ACCELERATOR_PLAYWRIGHT_NO_OWNER:-}" ]; then
  owner_pid=0
fi

nohup node "$SCRIPT_DIR/run.js" daemon \
  --state-dir "$STATE_DIR" \
  --owner-pid "$owner_pid" \
  >> "$BOOTSTRAP_LOG" 2>&1 &
```

`inventory-design`'s preamble (or the design-skills preamble more
broadly) sets `ACCELERATOR_PLAYWRIGHT_NO_OWNER=1` before any agent is
spawned. Interactive-terminal users keep the cleanup-on-shell-exit
convenience by default.

Add a test to `daemon.test.js` that spawns with a non-zero
short-lived owner PID and asserts the daemon shuts down with
`reason=owner-exited` — covering the production code path the current
tests opt out of.

#### Code references

- `skills/design/inventory-design/scripts/playwright/run.sh:100-131`
- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:21,
  88-97`
- `skills/design/inventory-design/scripts/playwright/lib/daemon.test.js:77,
  99, 120, 141, 164`

---

## The documents-locator precedent (full mechanism)

This is the pattern the user explicitly asked me to investigate. It
underpins fix 1 above.

### Agent declares a preloaded skill

`agents/documents-locator.md:1-12`:

```yaml
---
name: documents-locator
description: ...
tools: Grep, Glob, LS
skills:
  - accelerator:paths
---
```

Key points:

- No `Bash` tool — the agent never shells out for paths.
- `skills: - accelerator:paths` tells the harness to preload the
  `paths` skill into the agent's context at spawn time.

The body references the preloaded block
(`agents/documents-locator.md:20-26`):

> "Use the resolved paths from the **Configured Paths** block injected
> into your context (provided by the preloaded `paths` skill). … Treat
> those values as authoritative — do not hardcode `meta/` prefixes."

### The preloaded skill

`skills/config/paths/SKILL.md:1-22` is a one-line bang call surrounded
by a human-readable "Path legend". The frontmatter uses
`user-invocable: false` (not `disable-model-invocation: true`, which
per Claude Code docs would block preloading via subagent `skills:`
frontmatter — that constraint is called out in the SKILL maintainer
comment at lines 11-16).

### The bang script

`scripts/config-read-all-paths.sh:1-34` iterates `PATH_KEYS` from
`config-defaults.sh`, excludes `tmp`/`templates`/`integrations`, and
calls `config-read-value.sh paths.<key> <default>` for each. Output:

```
## Configured Paths

- plans: meta/plans
- research_codebase: meta/research/codebase
- research_design_inventories: meta/research/design-inventories
- research_design_gaps: meta/research/design-gaps
- decisions: meta/decisions
- prs: meta/prs
- validations: meta/validations
- review_plans: meta/reviews/plans
- review_prs: meta/reviews/prs
- review_work: meta/reviews/work
- work: meta/work
- notes: meta/notes
- global: meta/global
- research_issues: meta/research/issues
```

Any new key added to `PATH_KEYS` is automatically picked up; the agent
and skill require zero edits to surface a new directory.

### Callers do NOT inject paths

Each calling skill just spawns the agent with a free-text task
description and the agent's preloaded context contains the resolved
paths. Examples:

- `skills/work/extract-work-items/SKILL.md:108-109`
- `skills/work/create-work-item/SKILL.md:210-213`
- `skills/research/research-codebase/SKILL.md:78-82`

The placeholder `{documents locator agent}` in caller bodies is
resolved by `scripts/config-read-agents.sh:32-42, 106-124` — a
parallel mechanism for agent-*name* resolution (separate from path
resolution).

### Why this is the right precedent for the browser agents

- It places the contract in one well-known place (the agent's
  frontmatter), not scattered across every caller.
- Resolution happens before the agent's first tool call (no
  instruction-following race).
- A new value (or a new key) can be added without changing any
  caller.
- The same approach works for the executor path (issue 1).

## Code References

- `agents/browser-locator.md:1-8, 36-47, 90-103` — agent definition,
  bare `run.sh`, forbidden tools that block route discovery
- `agents/browser-analyser.md:1-8, 16-31, 139-144` — agent definition,
  bare `run.sh`
- `agents/documents-locator.md:1-12, 20-26` — precedent agent with
  `skills: - accelerator:paths`
- `skills/config/paths/SKILL.md:1-22` — the preloaded path-resolving
  skill (shape to mirror)
- `scripts/config-read-all-paths.sh:14, 24-33` — emits "## Configured
  Paths" block
- `scripts/config-read-path.sh:17-42` — single-key lookup; warns on
  unknown key
- `scripts/config-defaults.sh:26-44, 46-64` — PATH_KEYS / PATH_DEFAULTS
  arrays — single-edit site for adding new path keys
- `skills/design/inventory-design/SKILL.md:30, 138-141, 162-202,
  262-264` — invokes `config-read-path.sh design_inventories`,
  resolves run.sh path in its own bash, spawns agents without
  injecting the path
- `skills/design/inventory-design/scripts/playwright/run.sh:100-131`
  — `--owner-pid "$$"` issue
- `skills/design/inventory-design/scripts/playwright/lib/daemon.js:21,
  88-97, 113-224, 158-161` — owner-PID watcher, command-dispatch
  surface, accessibility-only snapshot
- `skills/design/inventory-design/scripts/playwright/lib/daemon.test.js:77,
  99, 120, 141, 164` — tests opt out of owner watcher with
  `--owner-pid 0`
- `skills/design/analyse-design-gaps/SKILL.md:21-25, 27-28` — does NOT
  spawn browser agents; same `config-read-path.sh` warning
- `skills/config/init/SKILL.md:31-32`,
  `skills/config/init/scripts/init.sh:18-31` — third call site for
  the missing keys, with internal fallback defaults
- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:414-415`
  — migration that renamed `design_*` → `research_design_*` keys
- `scripts/test-config.sh:2443-2455, 3074-3162, 3164-3204, 4026-4045,
  5791-5795` — allow-list shape tests, per-key default tests, inline
  hardcoded-default audit, all of which a fix must respect

## Architecture Insights

- **Skill-to-agent contract gap is a recurring class.** The originating
  note `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md`
  → work item 0052 → `skills:` preloading mechanism. The browser
  agents are the next instance of the same pattern.
- **Three resolution mechanisms exist** in the codebase, in
  decreasing priority order:
  1. Agent frontmatter `skills:` preloading (resolved before the
     agent's first action; used by `documents-locator`). This is the
     strongest contract.
  2. Caller skill bakes value into spawn prompt (per-caller patch;
     what `inventory-design` would have to do today for the executor
     path if `skills:` is not adopted).
  3. Agent self-discovery (the current `find /` antipattern). Avoid.
- **Tests can hide bugs by opting out of the mechanism under test.**
  `daemon.test.js` exclusively uses `--owner-pid 0`, which is the
  branch that *isn't* broken. The production code path with a real
  owner PID is untested.
- **SPA navigation hides route-resolution bugs.** Successful
  `navigate` + non-empty `snapshot` looks the same for real routes
  and fabricated ones. Any locator working against an SPA needs
  signal beyond "did navigate succeed" — `links` or `evaluate` plus
  explicit reality checking.
- **Two preloaded-skill semantics**:
  - `user-invocable: false` — hidden from the `/` menu but available
    for preload.
  - `disable-model-invocation: true` — blocks preload entirely (per
    Claude Code docs). Critical when authoring a new preloaded
    skill.

## Historical Context

- `meta/work/0052-make-documents-locator-paths-config-driven.md` —
  the precedent. Done. Established the `skills:` agent-frontmatter
  mechanism (Requirement 9, lines 88-90 — the harness extension that
  made `skills:` honour-preload in agent definitions, not just
  SKILL.md files).
- `meta/work/0030-centralise-path-defaults.md` and
  `meta/plans/2026-05-08-0030-remove-inline-path-defaults-from-consumers.md`
  — established the centralised `config-defaults.sh` allow-list and
  removed inline defaults from consumers. The "no inline default at
  call site" rule is enforced by
  `scripts/test-config.sh:3164-3204`.
- `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md`
  — the originating discussion that motivated 0052. Identifies the
  same antipattern the browser agents now exhibit.
- `meta/notes/2026-05-09-design-paths-missing-from-documents-locator.md`
  — a direct follow-up after 0052 noting that newly added design-paths
  keys were still not enumerated by documents-locator. Resolved by the
  research-restructure migration 0004 and the subsequent
  `research_design_*` keys.
- `meta/work/0056-restructure-meta-research-into-subject-subcategories.md`
  and `meta/plans/2026-05-12-…` — introduced the
  `research_design_inventories` / `research_design_gaps` keys.
- `meta/research/codebase/2026-05-06-design-skill-localhost-and-mcp-issues.md`
  and the matching plan — earlier round of Playwright-daemon /
  owner-PID / stale-socket work. Useful background for any larger
  daemon rework.
- `meta/research/design-inventories/2026-05-06-135214-current-app/inventory.md`
  — documents the `browser-locator` fabrication problem in narrative
  form from a real run.

## Related Research

- `meta/research/codebase/2026-05-08-0052-documents-locator-config-driven-paths.md`
  — enumerates which agents/skills do/don't consume config-driven
  path keys (explicitly calls out `browser-locator`,
  `browser-analyser`).
- `meta/research/codebase/2026-05-08-0030-centralise-path-defaults-implementation.md`
  — implementation context for the centralised defaults.
- `meta/research/codebase/2026-05-02-design-convergence-workflow.md`
  — the foundational research that introduced the browser-agent pair
  and the `inventory-design` skill.

## Open Questions

- **Aliased keys (`design_inventories` *and*
  `research_design_inventories`) vs. completing migration 0004**.
  Shape B (rename the four skill call sites) is more architecturally
  honest. Shape A keeps backward compatibility for any user-config
  that still has `paths.design_inventories`. Was migration 0004 a
  hard rename or a soft alias? If hard, shape B is unambiguously
  correct.
- **Should `browser-analyser` also be allowed `links`?** It currently
  has `evaluate`, so it can already get hrefs, but a shared executor
  command keeps the contract symmetric.
- **Should `inventory-design` Step 8 pre-extract routes itself before
  spawning the locator?** Option 4 of issue 2. Reduces the locator's
  role to description and skirts the
  navigate-only-knows-the-URL-you-give-it problem. Could replace the
  locator entirely for this skill, leaving the locator pattern as
  a more general capability for non-SPA apps.
