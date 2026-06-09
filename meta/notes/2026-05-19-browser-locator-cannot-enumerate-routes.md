---
type: note
id: "2026-05-19-browser-locator-cannot-enumerate-routes"
title: "Bug: `browser-locator` agent cannot enumerate routes with its allowed toolset"
date: "2026-05-19T00:00:00+00:00"
author: Toby Clemson
producer: create-note
status: captured
topic: "Bug: `browser-locator` agent cannot enumerate routes with its allowed toolset"
tags: []
revision: "11218123a1e4"
repository: "ticket-management"
last_updated: "2026-05-19T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Bug: `browser-locator` agent cannot enumerate routes with its allowed toolset

## Problem (severity: high — defeats the agent's stated purpose)

`agents/browser-locator.md` declares the agent's job as **"enumerate routes,
screens, and component presence"** and restricts it to two executor commands
(see lines 38–43 and 85, 94):

> **Use only navigate and snapshot** — no screenshots, no evaluate, no clicking
>
> - Do not use `run.sh evaluate` — no JavaScript execution
> - Do not use `run.sh click` or `run.sh type` — no interaction

But `run.sh snapshot` returns the result of Playwright's
`page.accessibility.snapshot()` (see
`scripts/playwright/lib/daemon.js:158–161`), which exposes only ARIA fields:

```json
{ "role": "link", "name": "Work items" }
```

**No `href`. No `data-*`. No URL anywhere in the tree.** The agent sees that
there is a link called "Work items" but has no observable mechanism to
discover where it points.

The two tools the agent is permitted to use:

- `navigate`: requires a URL the agent already knows.
- `snapshot`: returns role+name; never URLs.

The two tools that could surface URLs are explicitly forbidden:

- `evaluate`: could read `document.querySelectorAll('a[href]')`.
- `click`: could follow a link and observe the resulting `page.url()`.

So the agent is **structurally unable to enumerate routes from observed
DOM state**. In practice it falls back to URL-guessing from link text
("Work items" → `/work-items`?, `/define/work-items`?,
`/collections/work-items`?). On the Visualiser specifically the actual
path is `/library/work-items` — verified by driving `evaluate` directly:

```json
[{"text":"Work items60","href":"/library/work-items"},
 {"text":"Decisions26","href":"/library/decisions"},
 {"text":"Plans74","href":"/library/plans"}]
```

## Why this hides itself in practice

The Visualiser is a client-side single-page app. Every URL — `/`,
`/work-items`, `/totally-bogus` — returns the same 1073-byte HTML shell
with HTTP 200. The Playwright `navigate` always succeeds; the resulting
`snapshot` always renders *something* (often the home page tree, because
the SPA router falls back when a path is unknown). So the agent's
"successful" navigation to a hallucinated path looks identical to a
successful navigation to a real one. There is no signal of "this route
doesn't exist". The agent walks deeper, takes more guesses, and either
exhausts its budget or degenerates into garbled output (observed
2026-05-19: a path containing a corrupted segment
`/Users/tobyclemson/.middle.claude/...`).

This failure mode is invisible to:

- Static integration tests using non-SPA fixtures.
- Multi-page apps where unknown routes 404 (navigate would fail loudly).
- Any test where the snapshot includes meaningful link diversity by name.

## Reproduction

1. Start any SPA-style app on localhost.
2. Spawn `browser-locator` with a prompt to enumerate routes.
3. Observe: agent guesses URLs from snapshot link names, every navigate
   "succeeds", snapshot returns home-page tree each time, agent has no
   way to distinguish reality from fabrication.

## Suggested path forward

Pick one (or combine):

1. **Allow `evaluate` for href discovery only** — restrict it to a fixed
   safe expression set (e.g. `__accelerator_collect_links()`) embedded
   as a helper. The agent calls a single command; security stays
   bounded.
2. **Add a `links` command to the executor** — returns an array of
   `{text, href, role}` for every anchor on the page. Cleanest API
   separation; agent definition allows just `navigate`, `snapshot`,
   `links` and stays declarative.
3. **Allow `click` for the locator and have it traverse the link
   graph** — riskier (mutation, hover side-effects, modal dialogs)
   and slower (full page load per link). Not recommended.
4. **Have the orchestrator (skill) pre-extract routes via direct
   executor calls** before spawning the locator agent. Reduces the
   agent's role from "discover" to "describe". Pragmatic.

Option 2 is the cleanest "do once, fix forever" approach. Option 4 is
the fastest near-term unblocker.

## References

- Agent definition: `agents/browser-locator.md`
- Executor command surface: `scripts/playwright/lib/daemon.js:118–215`
- ARIA snapshot API limitation: Playwright `page.accessibility.snapshot()`
  intentionally returns the accessibility tree only — not DOM attributes.
- Sibling notes:
  - [[2026-05-19-browser-agents-self-discover-playwright-executor]]
  - [[2026-05-19-playwright-daemon-owner-pid-ephemeral-shell]]
