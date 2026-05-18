# Tech Debt: Browser agents self-discover Playwright executor with `find /`

## Problem

The `browser-locator` and `browser-analyser` agent definitions reference the
Playwright executor as a bare `run.sh` command (see
`agents/browser-locator.md` lines 38–43, e.g. `run.sh navigate '{...}'`).
Nothing in the agent prompt, the agent's tool environment, or a documented
env var tells the agent where the script actually lives.

In practice the agent's first action when spawned is to locate the script
itself. Observed behaviour on 2026-05-19 during an `/accelerator:inventory-design`
run:

```
which run.sh || find / -name "run.sh" -type f 2>/dev/null | head -5
```

A full-filesystem `find /` is exactly the kind of scan called out as bad
practice — it can exhaust resources on large home directories, is slow, and
returns false positives from unrelated projects.

## Why this is the wrong contract

The skill that invokes the browser agent (`inventory-design`, and others
under `skills/design/`) knows the executor's absolute path — it is a fixed
location under the plugin cache:

```
<plugin-cache>/skills/design/inventory-design/scripts/playwright/run.sh
```

That path is not being handed over to the agent. The skill-to-agent contract
is implicit: "you'll find it somewhere", which forces every agent invocation
to pay a discovery cost (and to make a guess that may match the wrong
binary in environments where multiple plugin versions are cached
side-by-side).

## Workaround used today

The inventory-design caller now bakes the executor's absolute path into the
agent prompt explicitly. This is a per-caller patch; the underlying agent
definition still tells the agent to invoke a bare `run.sh`.

## Suggested path forward

Pick one of:

1. **Caller-side**: have every skill that spawns a browser agent inject the
   executor path into the agent prompt as a structured field (preferred —
   self-contained, no agent-definition change required, callers already
   know the path).
2. **Agent-side**: amend the agent definition to require the caller to
   supply the executor path and to refuse to self-discover. Combine with a
   prompt-template helper so callers can't forget.
3. **Environment**: standardise on an env var (e.g.
   `ACCELERATOR_PLAYWRIGHT_RUN_SH`) set by the skill's preamble and read by
   the agent. Less explicit than 1 but works if multiple agents need the
   same value.

Option 1 is the lowest-friction fix; option 2 makes the contract explicit
and is worth pairing with it.

## References

- Agent definitions: `agents/browser-locator.md`, `agents/browser-analyser.md`
- Skill that surfaced this: `/accelerator:inventory-design` (skills/design/inventory-design)
- Related: [[2026-04-26-agents-hardcode-default-directory-locations]] — same
  class of problem (agents assuming environment knowledge their caller has)
