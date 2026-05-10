---
name: inventory-design
description: Generate a structured design inventory for a frontend source —
  tokens, components, screens, and features — by crawling it with code analysis,
  live Playwright inspection, or both. Use when you need to capture a snapshot of
  a current or target design surface before running analyse-design-gaps. Produces
  a dated artifact directory with an inventory.md and screenshots/. Re-running
  for the same source-id supersedes the prior snapshot without losing it.
argument-hint: "[source-id] [location] [--crawler code|runtime|hybrid] [--allow-internal] [--allow-insecure-scheme]"
disable-model-invocation: true
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/*)
---

# Inventory Design

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh inventory-design`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher, accelerator:browser-locator,
accelerator:browser-analyser.

**Design inventories directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories`

You are tasked with crawling a design source and producing a structured
`design-inventory` artifact. The artifact captures the design tokens,
components, screens, and features of the source so a downstream
`analyse-design-gaps` run can compute a structured diff between two
snapshots.

## Crawler Modes

| Mode      | Description                                                                                                                                                                                 | Requires Playwright runtime |
|-----------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------------------|
| `code`    | Static analysis of source files only. Reads tokens from config files (Tailwind, CSS custom properties, design-token JSON), components from JSX/TSX/Vue/Svelte, screens from routing config. | No                          |
| `runtime` | Live browser inspection only. Navigates each screen, captures computed styles and state via Playwright.                                                                                     | Yes                         |
| `hybrid`  | Code-static as ground truth for tokens and components; runtime fills in screen states and screenshots. Default when the source is a code repo.                                              | Yes                         |

**Default selection**: if `--crawler` is not specified, the skill selects:
- `hybrid` — when the location is a code-repo path (auto-downgrades to `code` if Playwright is unavailable; see Steps 3–5)
- `runtime` — when the location is an `https://` URL

## Steps

### 1. Validate Arguments

Run:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/validate-source.sh \
  "<location>" ${allow_internal_flag} ${allow_insecure_scheme_flag}
```

where `allow_internal_flag` is `--allow-internal` if the user passed that flag (otherwise
omit it), and `allow_insecure_scheme_flag` is `--allow-insecure-scheme` if the user passed
that flag (otherwise omit it).

If it exits non-zero, report the error to the user and stop. Do not create any
artifact directory.

By default, `https://` URLs to public hosts and `http://localhost` /
`http://127.0.0.1` are accepted without any flag. Other internal hosts
(RFC1918, link-local, other loopback IPs) require `--allow-internal` — on either
scheme. `--allow-internal` subsumes `--allow-insecure-scheme` for internal hosts:
a user accepting internal-host SSRF risk has already accepted the strictly-greater
concern. Plain `http://` to a non-localhost public host requires
`--allow-insecure-scheme` (NOT `--allow-internal`, which would be a misleading flag
name for that case).

**Source-id format**: `source-id` must match `^[a-z0-9][a-z0-9-]*$` (kebab-case,
lowercase, no leading hyphen, no spaces). If it does not, report a clear error
naming the offending characters and stop.

### 2. Resolve Auth Mode

Run:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/resolve-auth.sh
```

Capture the output (`header`, `form`, or `none`). If it exits non-zero, report
the error to the user and stop.

**Auth-header origin allowlist (security-critical)**: if auth mode is `header`,
the `ACCELERATOR_BROWSER_AUTH_HEADER` value is injected **only** on navigations
whose origin (scheme+host+port) matches the resolved `[location]` origin or the
`ACCELERATOR_BROWSER_LOGIN_URL` origin. On any cross-origin navigation (off-site
link, OAuth redirect, or any attacker-controlled target reached during the crawl),
strip the header before the request is issued. Instruct the `{browser analyser agent}`
to enforce this explicitly.

**Auth-walled route handling**: when auth mode is `none` and a route appears to
require authentication, skip it and record it in `Crawl Notes` with the message:

> `inventory-design: skipped <url> (appears auth-walled). Set
> ACCELERATOR_BROWSER_AUTH_HEADER, or
> ACCELERATOR_BROWSER_USERNAME / _PASSWORD / _LOGIN_URL, to crawl
> authenticated routes.`

Do not fabricate observations for auth-walled screens.

### 3. Provisional Crawler-Mode Resolution

Determine the provisional crawler mode from the CLI flag and defaults:

- Explicit `--crawler code` → `code` (skip Steps 4–6)
- Explicit `--crawler runtime` → `runtime` (proceed to Step 4)
- Explicit `--crawler hybrid` → `hybrid` (proceed to Step 4)
- No flag, location is a code-repo path → `hybrid` (proceed to Step 4)
- No flag, location is an `https://` URL → `runtime` (proceed to Step 4)

### 4. Bootstrap Playwright

Only if provisional mode is `runtime` or `hybrid`, run:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/ensure-playwright.sh
```

Capture its stdout, stderr, and exit code.

- **Exit 0** → bootstrap ready; proceed to Step 5.
- **Non-zero exit** → extract the `ACCELERATOR_DOWNGRADE_REASON=<enum>` line from stderr.
  - If provisional mode was `hybrid`: downgrade to `code`. Print the downgrade notice (see below)
    and record it in `Crawl Notes`. Then skip to Step 7 (no ping needed).
  - If provisional mode was `runtime`: hard-fail with the bootstrap stderr and stop.

**Downgrade notice**: run `notify-downgrade.sh --from <mode> --to code --reason <enum>` and
print its stdout **before the crawl starts** (not only in Crawl Notes).

### 5. Confirm Executor Liveness

Only if Step 4 succeeded (bootstrap ready), run:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/playwright/run.sh ping
```

- **Returns `{"ok":true,...}`** → executor is healthy; proceed to Step 6.
- **Returns error JSON or fails** → treat as `executor-ping-failed`.
  - If provisional mode was `hybrid`: downgrade to `code`. Run `notify-downgrade.sh --from hybrid
    --to code --reason executor-ping-failed` and print the result. Record in `Crawl Notes`.
  - If provisional mode was `runtime`: hard-fail with the ping error and stop.

### 6. Finalize Crawler Mode

The crawler mode is now finalised. Use what survived Steps 3–5:
- If Step 3 set `code`, use `code`.
- If Steps 4–5 succeeded for `runtime` or `hybrid`, use that mode.
- If downgrade occurred in Steps 4 or 5, use `code`.

### 7. Compute Next Sequence Number

Scan all `*-{source-id}/inventory.md` files under the design inventories root.
Read each frontmatter `sequence` field. Take `max + 1`, starting at 1 if none
exist. This is the sequence number for the new inventory.

### 8. Spawn Agents in Parallel

Based on the chosen crawler mode:

**`code` mode**: spawn `{codebase locator agent}` and `{codebase analyser agent}`
in parallel to discover and extract:
- Design tokens (Tailwind config, CSS custom properties, design-token JSON, theme files)
- Component inventory (JSX/TSX/Vue/Svelte files, named exports, prop signatures)
- Routing config (React Router, Next.js pages/app, Vue Router)
- Feature flags and conditional blocks

**`runtime` mode**: spawn `{browser locator agent}` to enumerate routes/screens,
then spawn `{browser analyser agent}` for each screen group in parallel.

**`hybrid` mode**: spawn both code and browser agents in parallel. Use code-static
output as ground truth for tokens and component names; use runtime output for
screen states, computed styles, and screenshots.

**Crawl bounds** (enforced regardless of crawler mode):
- **Page cap**: at most 50 distinct routes per crawl. On cap hit, write the
  inventory with `status: incomplete` and list unreached routes in `Crawl Notes`.
- **Wall-clock timeout**: 5 minutes total per crawl. Same handling.
- **Screenshot byte budget**: 50 MB per crawl. When exhausted, skip remaining
  screenshots and record which screens have no visual capture in `Crawl Notes`.
  The crawl continues until another bound fires; the inventory is written with
  `screenshots_incomplete: true` in frontmatter.

**Screenshot masking**: the executor automatically masks `[type=password]`,
`[autocomplete*=token]`, and `[data-secret]` selectors in screenshots. Never
attempt to read or expose the values of masked fields.

**URL scrubbing**: strip query strings from any URL written into the inventory
body (screen routes, references). Document this reduction in `Crawl Notes`.

**Auth-header origin allowlist (security-critical)**: if auth mode is `header`,
the executor's `route()` handler enforces that `ACCELERATOR_BROWSER_AUTH_HEADER`
is injected only on navigations whose origin matches the resolved `[location]`
origin or the `ACCELERATOR_BROWSER_LOGIN_URL` origin. Instruct the
`{browser analyser agent}` to enforce this explicitly for any manual header
injection it performs.

### 9. Synthesise

Compile agent findings into the five inventory categories:

1. **Design tokens** — colour, typography, spacing, radius, shadow (with token names and computed values)
2. **Component catalogue** — name, variants, props summary, usage count
3. **Screen inventory** — route, observed states (loading/empty/error/success), screenshot paths
4. **Feature catalogue** — named features, activation mechanism (route, flag, interaction)
5. **Information architecture** — navigation structure, primary user flows

### 10. Generate Metadata

Run:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/inventory-metadata.sh
```

### 11. Write Artifact (Atomic)

Build the inventory under a sibling temporary directory:
```
<design_inventories>/.YYYY-MM-DD-HHMMSS-{source-id}.tmp/
  inventory.md
  screenshots/
```

Use the `design-inventory` template:
```
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh design-inventory`
```

**Pre-write secret scrubber**: before moving the tmp directory to its final name,
run:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/scrub-secrets.sh \
  "<tmp_dir>/inventory.md"
```
If it exits non-zero, delete the tmp directory and report the error. Do not write
the artifact. Do not print the value of any environment variable in the error
message.

Move the tmp directory to its final name:
```
<design_inventories>/YYYY-MM-DD-HHMMSS-{source-id}/
```

Both the directory glob and the resolver explicitly skip leading-dot directories,
so an in-progress `.tmp/` is invisible to readers.

**Supersede prior inventories**: after the new directory is in place, glob
`*-{source-id}/` under the inventory root (excluding leading-dot names). Exclude
the just-written directory. For each remaining directory where `inventory.md` has
`status: draft` or `status: accepted`, set `status: superseded`. This step is
idempotent; if it fails partway through, the new directory is already
authoritative (the resolver uses `sequence` as its primary tiebreaker).

### 12. Cleanup

If a Playwright daemon was started (Steps 4–5 succeeded), stop it:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/playwright/run.sh daemon-stop
```

This is belt-and-braces — the browser agents also call `run.sh daemon-stop` as their final
action. Running it here ensures cleanup even if an agent exits abnormally.

### 13. Present Summary

Report:
- The artifact path
- Source-id, location, crawler mode used
- Count of tokens, components, screens, and features discovered
- Any crawl bounds that fired (cap, timeout, screenshot budget)
- Any auth-walled routes that were skipped
- Whether a prior inventory was superseded

Suggest next steps:
- Run `/accelerator:inventory-design <target-source-id> <target-location>` for the
  target design surface if not already done
- Run `/accelerator:analyse-design-gaps <current-source-id> <target-source-id>` to
  compute the gap

## Important Guidelines

- Never fabricate observations. Record only what agents actually found.
- If a partial crawl fires a bound, write what was found and mark the inventory
  `status: incomplete` — do not silently drop data.
- Do not expose env-var values in any output, log, or artifact body.
- The `sequence` field is the resolver's primary tiebreaker. Always compute it
  by reading existing inventories before writing.
- The `.tmp/` → final directory rename is atomic on POSIX filesystems. Do not
  write directly to the final directory name.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh inventory-design`
