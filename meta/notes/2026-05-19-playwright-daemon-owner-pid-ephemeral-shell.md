# Tech Debt: Playwright daemon owner-pid is bash `$$` — dies with ephemeral shells

## Problem

In `skills/design/inventory-design/scripts/playwright/run.sh` (line 110) the
daemon is spawned with:

```bash
nohup node "$SCRIPT_DIR/run.js" daemon \
  --state-dir "$STATE_DIR" \
  --owner-pid "$$" \
  >> "$BOOTSTRAP_LOG" 2>&1 &
```

`$$` is the PID of the `run.sh` bash process. `lib/daemon.js:90–96`
installs an owner-PID watcher that polls every `OWNER_POLL_MS`:

```js
const ownerWatcher = ownerPid > 0 ? setInterval(() => {
  try { process.kill(ownerPid, 0); }
  catch { shutdown('owner-exited', { owner_pid: ownerPid }); }
}, OWNER_POLL_MS) : null;
```

The daemon shuts down with `owner-exited` the moment that PID disappears.

## Why this breaks in an LLM-tool-harness context

The Claude Code Bash tool spawns a fresh shell per invocation; the shell exits
as soon as the command returns. So:

1. `run.sh ping` bootstraps the daemon, registers `--owner-pid $$` where
   `$$` is *itself*.
2. `run.sh` execs into node, ping completes, the process exits.
3. The daemon detects owner death within `OWNER_POLL_MS` and shuts down.
4. The next `run.sh navigate` call finds no daemon, bootstraps a *new* one
   (with its own owner-pid that will also die immediately), and the
   navigate completes — but on a fresh browser context with no shared state.
5. `run.sh snapshot` then either snapshots an empty browser or hangs.

Observed on 2026-05-19 during an `/accelerator:inventory-design` run:
`server-stopped.json` showed reason `owner-exited`, `owner_pid: 9593`. The
PID 9593 was the bash shell that executed `ensure-playwright.sh && run.sh
ping` — an ephemeral process killed seconds after ping returned.

## Why this works in interactive terminals (and tests)

- Interactive shell: `$$` is the long-lived user shell; daemon survives.
- `lib/daemon.test.js`: every test uses `--owner-pid 0`, which is treated
  as "no owner watcher" — daemon stays up until idle/explicit stop.

So the integration tests can't surface this regression because they
explicitly opt out of the mechanism that fails in real use.

## Workaround used today

Bypass `run.sh`'s bootstrap path. Manually spawn the daemon with
`--owner-pid 0`, then let `run.sh`'s reuse short-circuit (`run.sh:49–64`)
pick up the existing daemon for subsequent commands:

```bash
nohup node "$SCRIPT_DIR/run.js" daemon \
  --state-dir "$STATE_DIR" \
  --owner-pid 0 \
  >> "$STATE_DIR/server.bootstrap.log" 2>&1 &
disown
```

This works because:
- `run.sh` writes/checks `server-info.json` + `server.pid` and reuses a
  live daemon when its PID is alive, regardless of how it was started.
- With `--owner-pid 0` the daemon has no owner to watch and lives until
  idle timeout or explicit `daemon-stop`.

## Suggested path forward

Pick one of:

1. **Detect harness context and disable owner-watch** (preferred): if an
   env var like `ACCELERATOR_PLAYWRIGHT_NO_OWNER=1` is set, pass
   `--owner-pid 0`. Skill preambles invoked from the Claude Code harness
   can set the flag explicitly.
2. **Use `$PPID` or harness root**: walk up the process tree to find a
   long-lived parent. Brittle — Claude Code's process model isn't part
   of the contract.
3. **Always `--owner-pid 0`** and rely entirely on the idle timeout for
   cleanup. Removes the interactive-shell convenience of "daemon dies
   when my terminal closes."

Option 1 is the lowest-risk fix and easy to implement.

## References

- Executor: `skills/design/inventory-design/scripts/playwright/run.sh`
- Daemon: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`
- Tests: `skills/design/inventory-design/scripts/playwright/lib/daemon.test.js`
  — note all cases use `--owner-pid 0`
- Sibling note: [[2026-05-19-browser-agents-self-discover-playwright-executor]]
  — same class of "skill-to-agent contract gap" issue
