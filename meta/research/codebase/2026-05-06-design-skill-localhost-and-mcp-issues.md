---
date: 2026-05-06T14:41:55+01:00
researcher: Toby Clemson
git_commit: bf60483a3af7e1519824704cb04cdca995bc2a70
branch: (detached HEAD; main = 45bb6b46 "Bump version to 1.21.0-pre.15")
repository: accelerator
topic: "inventory-design UAT issues — http://localhost rejection and Playwright MCP sub-agent hallucination"
tags: [research, inventory-design, design-skills, playwright, mcp, sub-agents, validation, plugin-dependencies]
status: complete
last_updated: 2026-05-06
last_updated_by: Toby Clemson
last_updated_note: "Added Decisions section recording approach for both issues and the Playwright runtime install mechanism."
---

# Research: inventory-design UAT issues — http://localhost rejection and Playwright MCP sub-agent hallucination

**Date**: 2026-05-06T14:41:55+01:00
**Researcher**: Toby Clemson
**Git Commit**: bf60483a3af7e1519824704cb04cdca995bc2a70
**Branch**: detached HEAD (main at 45bb6b46)
**Repository**: accelerator

## Research Question

`/inventory-design` UAT surfaced two blockers:

1. The skill rejects `http://localhost` URLs even though dev servers commonly run on plain HTTP.
2. The Playwright MCP sub-agents (`browser-locator`, `browser-analyser`) hallucinate — they reference tools they don't have (e.g. `mcp__chrome-devtools__*`), invent routes, and report `tool_uses: 0`. The user observed the workflow only succeeded because they discarded the agent output and crawled manually with their own Playwright tools.

What approach should we take to resolve each?

## Summary

**Issue 1 (http/localhost rejection)** is a small, well-contained change. All blocking happens in a single script (`validate-source.sh`); the rejection messages already advertise an `--allow-internal` flag that was deliberately deferred from v1. Reinstating the flag is mostly mechanical: extend the script, plumb a new arg through `SKILL.md`, and split one eval. The original SSRF rationale still holds and should remain the default — the flag is opt-in.

**Issue 2 (sub-agent MCP hallucination)** is **a known Claude Code bug**, not a design defect in our agents or skill. Plugin-shipped sub-agents that declare `mcp__playwright__*` tools in their frontmatter routinely fail to receive them at runtime (Anthropic issues [#13605](https://github.com/anthropics/claude-code/issues/13605), [#13898](https://github.com/anthropics/claude-code/issues/13898)); project-scoped MCPs (which is what we ship) are the worst case. The user's "fallback" was actually the main-thread MCP working where the sub-agent's didn't.

Adding a plugin dependency on Microsoft's Playwright plugin **does not fix this** — it's the same MCP server with the same `mcp__playwright__*` namespace and the same sub-agent-inheritance bug. Three real options exist (in order of recommendation): (A) **collapse the sub-agent boundary** for the runtime crawler so the main thread invokes the MCP directly; (B) **switch to a Bash/script-based Playwright model** (à la lackeyjb/playwright-skill) so we don't depend on MCP-tool inheritance at all; (C) ship at user scope. (A) is the smallest delta and matches what already works for the user.

## Detailed Findings

### Issue 1 — http:// and localhost rejection

#### Where the blocking happens (single-file surface)

All URL validation lives in [`skills/design/inventory-design/scripts/validate-source.sh`](skills/design/inventory-design/scripts/validate-source.sh:1).

- Scheme dispatch at [lines 29-47](skills/design/inventory-design/scripts/validate-source.sh:29) classifies `http://*` as `SCHEME=http`, then [lines 91-94](skills/design/inventory-design/scripts/validate-source.sh:91) hard-reject it with: `error: http:// URLs are not accepted. Use https:// instead.`
- After the http reject, the host allowlist at [lines 100-147](skills/design/inventory-design/scripts/validate-source.sh:100) is only reachable via https. It blocks `localhost`, `::1`, `127.x`, `10.x`, `172.16-31.x`, `192.168.x`, `169.254.x`, `fe80:` IPv6.
- Every internal-host error message **already names** the flag: `Use --allow-internal to override (not available in v1).` — the placeholder was put there by design.

Nothing else in the skill or in `browser-*` agents performs URL/host validation; the change surface is genuinely just this one script plus its caller.

#### Original rationale (from the plan-review cycle)

From `meta/plans/2026-05-03-design-convergence-workflow.md:1048-1053`:

> "Scheme allowlist: only `https://` accepted by default. `http://` is rejected unless the skill is invoked with an explicit `--allow-insecure` flag (not in v1; flag is reserved for future use)."

From `meta/plans/2026-05-03-design-convergence-workflow.md:1054-1060`:

> "Hosts that resolve to RFC1918 (10/8, 172.16/12, 192.168/16), loopback (127/8, ::1), link-local (169.254/16, fe80::/10), or `localhost` are rejected unless the skill is invoked with an explicit `--allow-internal` flag (not in v1). This prevents accidental SSRF reaching cloud metadata services or internal admin endpoints, particularly in CI contexts."

The driver was a Pass 1 review finding (`meta/reviews/plans/2026-05-03-design-convergence-workflow-review-1.md:120-122`) that the unvalidated `[location]` would expose `file://`, `javascript:`, and SSRF. The flags were named in the plan but explicitly deferred from v1 — UAT has now demonstrated the v1 default is too strict.

#### Eval coverage today

- [`evals.json` id 13](skills/design/inventory-design/evals/evals.json:130) (`http://127.0.0.1:8080`) currently asserts non-zero exit and an error mentioning loopback. Reinstating the flag means splitting it into a "without flag → fail" / "with flag → succeed" pair, plus a sibling test for `http://localhost`.
- No existing eval names `http://localhost` literally.
- The MCP-absent fallback is covered by [eval id 3 `mcp-unavailable-fallback`](skills/design/inventory-design/evals/evals.json:36); it's the only test that asserts no `mcp__playwright__*` tools are invoked.

#### Surface area for the fix

1. `validate-source.sh`: accept a second argument `--allow-internal`; gate the http reject (lines 91-94) and the seven host rejects (lines 100-147) behind it. *Optional but pragmatic*: always allow `http://localhost` and `http://127.0.0.1` even without the flag, treating "I'm pointing at my own dev server" as the common case and leaving the flag for true RFC1918 / cloud-metadata cases. This is a UX call, not a security one — see Open Questions.
2. `SKILL.md`: extend `argument-hint` (line 9) and Step 1 (lines 59-66) to document and pass through the flag.
3. `evals/evals.json` and `evals/benchmark.json`: split id 13; add a `localhost` variant.
4. No change needed to the auth header allowlist (SKILL.md:83-89) — it keys on the resolved origin, so a localhost target matches its own origin trivially.

### Issue 2 — Sub-agent MCP hallucination

#### What's actually happening

The user's symptoms — sub-agent referencing `mcp__chrome-devtools__*` tools that aren't declared in its frontmatter, claiming routes it never visited, `tool_uses: 0` — match a documented Claude Code bug pattern, not a flaw in our agents or skill.

- [Anthropic issue #13605 "Custom plugin subagents cannot access MCP tools (built-in agents can)"](https://github.com/anthropics/claude-code/issues/13605) — exact match: a plugin-shipped agent declaring `tools: mcp__playwright__browser_navigate, …` does not receive those tools at runtime, regardless of how `tools` is specified or omitted; the built-in `general-purpose` agent works. Open, no Anthropic response.
- [Issue #13898 "Subagents cannot access project-scoped MCP servers"](https://github.com/anthropics/claude-code/issues/13898) — sub-agents cannot reach MCP servers declared in a project's `.mcp.json` and **hallucinate plausible results instead**. User-scoped (`~/.claude/mcp.json`) works in the same agents.
- [Issue #19964](https://github.com/anthropics/claude-code/issues/19964) — sub-agent MCP availability docs are themselves contradictory.

We ship the Playwright MCP project-scoped at `.claude-plugin/.mcp.json:3-5` (`@playwright/mcp@0.0.73`), which is precisely the worst case under #13898.

The user's account that the workflow "fell back on the Playwright plugin I had installed" is consistent with this: the **main thread** still has the MCP, the **sub-agents** don't. When they discarded the bogus agent output and crawled directly, they were using the main-thread tools — which work.

#### Microsoft's Playwright plugin doesn't help

[claude.com/plugins/playwright](https://claude.com/plugins/playwright) (Microsoft, 205k+ installs) is a packaging of the same MCP server. Adding it as a `dependencies` entry would change *how the MCP arrives* but not *how sub-agents inherit it* — same `mcp__playwright__*` namespace, same sub-agent inheritance bug.

#### Plugin-to-plugin dependencies (the mechanism, not a fix)

For completeness: Claude Code does support declarative dependencies in `plugin.json` from CLI v2.1.110+. The schema accepts bare strings or `{name, version}` objects, resolves against same-marketplace plugins by default, and auto-installs them. Sources: [Plugin dependencies docs](https://code.claude.com/docs/en/plugin-dependencies), [Plugins reference](https://code.claude.com/docs/en/plugins-reference). Cross-marketplace deps require `allowCrossMarketplaceDependenciesOn` in the root marketplace, and version resolution uses git tags of the form `{plugin-name}--v{version}`.

But this mechanism does not address the underlying bug. It would only matter if there were a Playwright *skill* (not MCP) plugin worth depending on.

#### A genuine alternative: Bash-based Playwright

[lackeyjb/playwright-skill](https://github.com/lackeyjb/playwright-skill) uses a `run.js` "universal executor" invoked via `Bash`, with Claude writing Playwright JS payloads ad-hoc. It does not register any `mcp__*` tools. Sub-agents can use it because `Bash` inheritance is reliable. Trade-off: we'd own more glue (a Node runtime expectation, the executor script, a payload protocol), but we'd be free of the MCP-inheritance class of bug.

#### Re-exporting MCP via plugin.json — also doesn't help here

The `mcpServers` field in `plugin.json` is supported but has its own gotchas:
- [#16143](https://github.com/anthropics/claude-code/issues/16143) — *inline* `mcpServers` in `plugin.json` is silently dropped during manifest parsing (use a separate `.mcp.json` instead, which is what we already do).
- Plugin-shipped agents are explicitly forbidden from declaring `mcpServers` in their own frontmatter ("For security reasons…").

So there's no manifest-level lever to fix the sub-agent inheritance bug from inside the plugin.

#### What did the original plan say about this?

From `meta/research/codebase/2026-05-02-design-convergence-workflow.md:459-466`, the alternatives evaluated were *browser automation tools* (Puppeteer / Selenium / Cypress / Chrome DevTools MCP / WebDriver BiDi). Notably, **Chrome DevTools MCP was rejected** as "lower-level than Playwright; better for performance profiling than app exploration" — relevant context given that the user's hallucinating agent invented `mcp__chrome-devtools__*` tool names. The model has training-data familiarity with the namespace and reaches for it when its real tools aren't wired up.

The plan never compared *integration shapes* (MCP vs Bash-script vs separate plugin); it picked MCP and moved on. There is no historical objection to changing the integration shape if the MCP path proves unreliable.

The plan also never anticipated the broader hallucination class (fabricating tool names from a different namespace, narrating fictional success). Existing safeguards target only *content* fabrication ("never fabricate observations") — they assume the tool calls themselves are real.

### Recommended approaches (ranked)

**A. Collapse the sub-agent boundary for the runtime crawler.** *Smallest delta, matches what already works.*

- Have `inventory-design` invoke `mcp__playwright__*` directly from the skill's main loop for `runtime` and `hybrid` modes, instead of routing through `browser-locator` / `browser-analyser`.
- Code-mode crawl still spawns codebase-locator/analyser sub-agents (filesystem reads work fine in sub-agents).
- The browser-locator/browser-analyser agent files become either deleted or marked deprecated; their behavioural guidance (state matrix, evaluate allowlist, screenshot masking, origin-restricted auth header) needs to migrate into SKILL.md so the main thread enforces it.
- Pro: actually known to work; preserves all existing security posture.
- Con: SKILL.md grows; less parallelism on the runtime side.

**B. Switch to a Bash/script-based Playwright model.** *Most invasive, most robust.*

- Bundle a `run.js`-style executor under the skill's `scripts/`; have agents/the skill invoke it via `Bash`.
- Sub-agents can keep doing their part because they all have `Bash`.
- Requires a Node runtime, vendored or assumed. We already require `mise run deps:install:playwright` per `mise.toml:30-33`, so the Node toolchain is already in scope.
- Pro: removes our hardest external dependency on a Claude Code bug class.
- Con: large rewrite of the runtime/hybrid path; we re-implement what the MCP gave us (navigation, snapshot, evaluate-with-allowlist) ourselves, and own the security properties around `evaluate` payload allowlisting and screenshot masking in script form.

**C. Ship the MCP at user scope and update install instructions.** *Lightest mitigation; least confidence.*

- README change only: tell users to add `@playwright/mcp` to `~/.claude/mcp.json` instead of relying on the project-scoped `.claude-plugin/.mcp.json`. Per #13898, user-scoped MCP servers reach sub-agents.
- Pro: minimal code change.
- Con: Anthropic could change behaviour either way; we'd be relying on a difference between scopes that isn't officially documented as load-bearing. Also creates UX friction (manual install step, versioning drift between users).

**D. Add a plugin dependency on Microsoft's Playwright plugin.** *Does not help.*

- Same MCP, same namespace, same bug. Documenting this as a non-fix is itself useful — it stops the question recurring.

### Why option (A) probably wins on a cost/benefit basis

- The user's manual workaround already proves the main thread can run the MCP correctly today. We're not designing on speculation.
- It avoids us re-implementing browser automation we don't need to own.
- The browser-locator/browser-analyser split was attractive on paper (parallelism, role separation) but has zero current evidence of paying off, and is the exact boundary that the bug bites.
- It's reversible: if Anthropic fixes the sub-agent MCP bug, we can re-introduce the agents without losing the new main-thread path.

## Code References

- `skills/design/inventory-design/SKILL.md:11-20` — `allowed-tools` frontmatter (seven `mcp__playwright__*` entries)
- `skills/design/inventory-design/SKILL.md:53` — default crawler mode keyed on `mcp__playwright__browser_navigate` presence
- `skills/design/inventory-design/SKILL.md:59-66` — Step 1 calls `validate-source.sh`
- `skills/design/inventory-design/SKILL.md:103-104` — MCP detection by LLM self-introspection ("check your own toolbox")
- `skills/design/inventory-design/SKILL.md:118-122` — hard-fail on first `browser_navigate` failure (only programmatic detection point)
- `skills/design/inventory-design/SKILL.md:151-158` — crawl bounds (50 pages / 5 min / 50 MB) — instructional only, no code enforces them
- `skills/design/inventory-design/scripts/validate-source.sh:91-94` — http reject
- `skills/design/inventory-design/scripts/validate-source.sh:100-147` — internal-host rejects, error messages already name `--allow-internal`
- `skills/design/inventory-design/evals/evals.json:130-142` — eval id 13 (the test that has to be split)
- `skills/design/inventory-design/evals/evals.json:36-49` — eval id 3 `mcp-unavailable-fallback`
- `agents/browser-locator.md:7` — sub-agent declares two `mcp__playwright__*` tools
- `agents/browser-analyser.md:7` — sub-agent declares all seven `mcp__playwright__*` tools
- `.claude-plugin/.mcp.json:3-5` — project-scoped Playwright MCP (the scope that triggers #13898)
- `.claude-plugin/plugin.json:1-23` — no `dependencies` field; only `skills` registration
- `scripts/test-design.sh:85,90,133-147` — tests asserting explicit MCP tool names, forbidding wildcards

## Architecture Insights

- **Validation is a single-file affair** (`validate-source.sh`). The skill never re-validates the location elsewhere, which makes the fix surface for issue 1 small and local.
- **MCP detection is LLM-mediated** — there's no script that probes tool availability; the model is instructed to read its own toolbox. This is fragile in the same way the sub-agent bug is fragile: both rely on the model's view of available tools matching reality.
- **Agent role split is theoretical, not load-bearing**. browser-locator (snapshot only) and browser-analyser (full toolset) are conceptually clean, but in practice both broke at the same MCP-inheritance boundary. There's no current value being delivered by keeping them separate that the main thread couldn't deliver.
- **Crawl bounds are unenforced**. The page cap (50), wall-clock (5 min), and screenshot byte budget (50 MB) live as prose in `SKILL.md:151-158`. Whatever path forward we pick should consider whether to make at least one of these programmatic — the wall-clock especially is a hard one to enforce LLM-side.

## Decisions

Recorded after research-and-discussion on 2026-05-06.

### Issue 1 — http/localhost rejection

- **Default-allow `http://localhost` and `http://127.0.0.1`** with no flag required. The common UAT case is a developer pointing at a local dev server; demanding a flag for that case is too strict.
- **RFC1918 (10/8, 172.16-31/12, 192.168/16), link-local (169.254/16, fe80::/10), and `::1`** remain rejected by default. They will be unlocked by an explicit `--allow-internal` flag, reinstating the placeholder already named in the existing error messages at `validate-source.sh:100-147`.
- **Other internal-loopback ranges (`127.0.0.0/8` beyond `127.0.0.1`)**: also flag-gated. `127.0.0.1` is the only loopback address allowed by default; `127.0.0.2`–`127.255.255.254` still require `--allow-internal`. (Practically rare; this keeps the default surface minimal.)
- **Eval split**: existing `evals.json` id 13 (currently `http://127.0.0.1:8080` → fail) becomes `http://10.0.0.1` → fail; add a new eval asserting `http://localhost` and `http://127.0.0.1` succeed without the flag.

### Issue 2 — sub-agent MCP hallucination

- **Approach: option B (Bash-based Playwright executor).** Replace `mcp__playwright__*` tool dependencies entirely. Skill and any sub-agents invoke a vendored `run.js`-style executor via `Bash`. This sidesteps the sub-agent MCP-inheritance bug class (#13605, #13898) and removes us from the namespace where the model can hallucinate adjacent tool families like `mcp__chrome-devtools__*`.
- **Sub-agents stay** in the architecture but talk to the Bash executor instead of the MCP. The browser-locator/browser-analyser role split is retained because Bash inheritance to sub-agents works reliably; the existing security posture (read-only `evaluate` allowlist, screenshot masking, origin-restricted auth header) re-implements as constraints in the executor's payload protocol.
- **No plugin dependency on Microsoft's Playwright plugin.** Confirmed it would not help: it ships an MCP server, not a Playwright runtime our Bash executor can reach (`@playwright/mcp` lives in npx's hash-keyed cache and is not reliably resolvable from another package).
- **`.claude-plugin/.mcp.json` is removed** as part of this change; we no longer need the Playwright MCP server. README install instructions update accordingly.

### Issue 2 install mechanism — Playwright runtime for plugin consumers

- **Approach: lazy self-bootstrap.** A new `scripts/ensure-playwright.sh` is invoked by the skill before any runtime/hybrid crawl. On first run it: checks for `node` / `npm`, installs the `playwright` npm package into the cache dir, runs `npx playwright install chromium`, writes a sentinel file. Subsequent runs skip when the sentinel and binaries are present. First-run latency (1-3 min, dominated by browser download) is acceptable because it's once-per-machine.
- **Cache location: `${CLAUDE_PLUGIN_DATA}`.** This is the documented persistent-data dir per the Plugins reference. It survives plugin version bumps, unlike `${CLAUDE_PLUGIN_ROOT}` which is version-keyed and would force reinstalls on every update.
- **Node requirement: Node ≥ 20.** Aligned with Playwright's current minimum. `ensure-playwright.sh` fails fast with a clear "install Node ≥ 20 from nodejs.org or via your version manager" message if absent or below that floor.
- **`mise run deps:install:playwright` remains** as the developer-facing install path (used during plugin development and tests); it does not satisfy the consumer install path. The consumer path is `ensure-playwright.sh` only.
- **No mid-crawl bootstrap.** The bootstrap runs as a Step-0 prerequisite check before any agent is spawned, so latency and any failure are surfaced cleanly to the user up front rather than after a partial crawl.

### Out of scope for this work

- Programmatic enforcement of crawl bounds (page cap / wall-clock / screenshot byte budget) — deferred to a separate ticket.
- Filing an upstream issue against Anthropic for #13605 / #13898 — option B routes around the bug; an upstream fix isn't on our critical path.
- Verifying #16143 (inline `mcpServers` in plugin.json) — we already use a separate `.mcp.json` and we're removing it entirely as part of this work.

## Historical Context

- `meta/research/codebase/2026-05-02-design-convergence-workflow.md` — the design-convergence research; evaluates browser automation tools (lines 459-466) but not integration shapes; flags Playwright MCP as "first MCP dependency" (line 495).
- `meta/plans/2026-05-03-design-convergence-workflow.md` — the implementation plan; defers `--allow-internal` and `--allow-insecure` (lines 1048-1060); pinning discipline for `@playwright/mcp` (lines 633-638).
- `meta/reviews/plans/2026-05-03-design-convergence-workflow-review-1.md` — Pass 1 Critical Security #1-3, the review pass that introduced the strict URL/host validation.
- Commit `bf60483a3` (HEAD on main = `45bb6b46`) — current revision; design-convergence work is shipped as `meta/research/design-gaps/`, `meta/research/design-inventories/` document types and the two skills.

## Related Research

- `meta/research/codebase/2026-05-02-design-convergence-workflow.md` — original feasibility research for the design-convergence workflow.

## Open Questions

1. **Executor protocol shape.** Option B requires a small payload protocol between the skill (writing JS payloads) and the Node executor (running them under Playwright). What's the smallest viable surface — a fixed set of named operations (`navigate`, `snapshot`, `screenshot`, `evaluate`, `click`, `type`, `wait_for`) mirroring the previous MCP tools, or a freeform-JS-with-allowlist model? The former preserves the existing `evaluate` allowlist enforcement boundary cleanly; the latter is more flexible but harder to lock down.
2. **`${CLAUDE_PLUGIN_DATA}` reliability across CC versions.** The Plugins reference documents this env var, but we should verify it's populated in the Claude Code version range we support before relying on it. Fallback path: `~/.cache/accelerator/playwright/`.
3. **Bootstrap UX during the 1-3 min first run.** Should `ensure-playwright.sh` print a clear "this will take a few minutes; downloading Chromium…" preamble, or just stream npm/playwright output? The skill's main loop is blocked during this — we should make sure the user knows why.
4. **Browser binary scope.** Install Chromium only, or all three (Chromium, Firefox, WebKit)? Chromium-only matches what Playwright MCP defaults to and keeps the download small (~150 MB vs ~500 MB).
