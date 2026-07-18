---
type: codebase-research
id: "2026-07-18-codex-and-cross-agent-skill-portability"
title: "Research: Making Accelerator skills work with Codex and other coding agents"
date: "2026-07-18T10:02:26+00:00"
author: "Phil"
producer: research-codebase
status: complete
topic: "Codex and cross-agent skill portability"
tags: [research, codebase, skills, portability, codex, agents-md, plugin, hooks]
revision: "49388108dd7a72644350653212a7759f60a7f87b"
repository: "accelerator"
last_updated: "2026-07-18T10:02:26+00:00"
last_updated_by: "Phil"
schema_version: 1
---

# Research: Making Accelerator skills work with Codex and other coding agents

**Date**: 2026-07-18 10:02 UTC
**Author**: Phil
**Git Commit**: 49388108dd7a72644350653212a7759f60a7f87b
**Branch**: claude/skills-codex-compatibility-cmnnzw
**Repository**: accelerator

## Research Question

Make the Accelerator skills work with Codex and other popular coding agents.
Before scoping the work: is there an existing work item for it? Then: which
Claude Code features do the skills actually depend on, and which of those are
supported by Codex (and the other major agents) versus needing to be reworked?

## Summary

**No existing work item.** A search of `meta/work/` (181 items), open/closed
GitHub issues, and PRs found nothing tracking cross-agent portability. The only
existing mentions are incidental notes confirming the current stance — e.g.
`meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md:338`
states plainly that "Accelerator today targets macOS/unix-based systems and
Claude Code only." So this is net-new work.

**The landscape shifted in our favour during 2025–2026.** Two portability
layers now exist and are converging:

1. **AGENTS.md** — the settled cross-agent standard for the *single instruction
   file*. Broad adoption (OpenAI, Google, Cursor, Anthropic, Microsoft, 60k+
   projects; governance under the Linux Foundation's Agentic AI Foundation).
   Deliberately narrow: prose instructions only — no skills, commands, or hooks.
2. **Agent Skills / `SKILL.md`** — Anthropic's skill format, released as an
   **open standard in December 2025** (agentskills.io). This is the real vehicle
   for portable capability folders, and **Claude Code, OpenAI Codex, Cursor,
   GitHub Copilot, and Amp all now load `SKILL.md` skills natively.**

**The good news:** our core product — the `SKILL.md` bundle — is the single most
portable asset we own. The container format ports to five major agents largely
unmodified.

**The catch (our #1 portability hazard):** our skills lean heavily on the
Claude-Code-specific **`!`-preprocessor** (`` !`command` `` inline shell
injection) to pull live context into the prompt at invocation time — used in
**44 of 70 `SKILL.md` files**, up to 6 times per file. No other agent that reads
`SKILL.md` supports it. Only Gemini CLI has an equivalent (`!{...}`), and Gemini
doesn't read `SKILL.md`. Every skill that relies on preprocessor injection must
degrade gracefully — instruct the agent to *run* the bundled script via its own
shell tool, rather than pre-expanding output into the prompt.

**Secondary reworks:** `${CLAUDE_PLUGIN_ROOT}` interpolation (1,411 uses), the
`allowed-tools` permission grammar, `hooks/hooks.json`, `agents/*.md`
(Markdown→TOML for Codex), and the `.claude-plugin/plugin.json` manifest all
have per-agent equivalents but no shared standard, so each needs a mechanical
translation or a shipped-alongside variant.

**Portable already, no change needed:** the `meta/` filesystem protocol, the
`templates/`, the per-skill `.accelerator/` override files, and the backing
`scripts/` bash library (curl/jq, no MCP) — these carry no Claude Code
dependency. The coupling is entirely at the *injection boundary*, not the
storage format.

> **Confidence note.** The internal inventory below is ground truth (verified
> against the repo). The Codex/other-agent capability claims are from official
> 2025–2026 docs, but this space moves weekly — exact Codex version numbers, the
> `.codex-plugin/` marketplace details, and "which experimental field is honoured
> where" must be re-verified against the live CLIs before committing to a design.
> See [Open Questions](#open-questions).

## Detailed Findings

### 1. What Claude-Code-specific mechanisms our skills depend on

Verified over 69 shipped skills (a 70th `SKILL.md` is a migration test fixture)
and 9 agents.

**Hard Claude Code dependencies (must be reworked to run elsewhere):**

| Mechanism | Where / how much | CC-proprietary? |
| --- | --- | --- |
| `` !`command` `` body preprocessor (live-context injection) | 44 `SKILL.md` files, 1–6 uses each; e.g. `skills/vcs/commit/SKILL.md:12-15,66` | Yes — no cross-agent standard |
| `${CLAUDE_PLUGIN_ROOT}` interpolation | 1,411 uses (450 in `SKILL.md`, rest in `scripts/`, `hooks/hooks.json`) | Yes — CC injects it |
| `allowed-tools` frontmatter | 47 skills; scoped-Bash-glob grammar e.g. `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)` (`skills/vcs/commit/SKILL.md:6-7`) | Yes — CC permission syntax |
| `argument-hint` frontmatter | 45 skills | CC slash-command UI hint |
| `disable-model-invocation` | 42 skills | CC-proprietary |
| `user-invocable` | 23 skills | CC-proprietary |
| Slash-command + model-invocation binding of `name`/`description` | all 69 | Behaviour is CC-specific |
| Subagents `agents/*.md` (`name`/`description`/`tools`/`skills`/`color`) | 9 agents; spawned via Task tool `subagent_type` (`skills/github/review-pr/SKILL.md:323-324`) | Yes |
| Subagent **skill-preload** (`skills:` frontmatter on an agent) | 3 agents; gated to **Claude Code ≥ v2.1.144** (`CLAUDE.md:121`) | Yes — version-sensitive |
| Hooks: `hooks/hooks.json` schema + stdin/stdout JSON contract | `SessionStart` (3 cmds) + `PreToolUse` (1, Bash matcher); `hooks/hooks.json:1-44` | Yes |
| Packaging: `.claude-plugin/plugin.json` + `marketplace.json` | `plugin.json:1-29`; agents/hooks discovered by convention, not listed | Yes |

A load-bearing subtlety worth flagging: the `user-invocable: false` vs
`disable-model-invocation: true` distinction is deliberate and version-sensitive
— `disable-model-invocation: true` *blocks preload via subagent `skills:`
frontmatter*, so preload-target skills must use `user-invocable: false` instead
(maintainer note at `skills/config/paths/SKILL.md:11-16`). Any translation layer
has to preserve this.

**Portable already (carry over with little or no change):**

- The `meta/` filesystem message-bus and path-key protocol
  (`skills/config/paths/SKILL.md` — 15 configurable keys, default under `meta/`).
  Skills communicate through the filesystem, not the conversation
  (`CLAUDE.md:67-74`) — this design is agent-agnostic.
- `templates/` (13 Markdown output templates) and the per-skill
  `.accelerator/skills/<name>/instructions.md` + `context.md` override files.
- The `scripts/` bash library (~40 scripts, curl/jq) — plain shell, CC-agnostic
  except where scripts emit the hook JSON contract.

**Not used at all** (so nothing to port): MCP servers (we ship none and declare
none — integrations are curl/jq scripts), `Skill`-tool cross-invocation (cross-
skill references are prose directives to run `/name`), and the `model` field on
agents.

### 2. Cross-agent capability landscape

| Agent | Instruction file | Custom commands | Args | **Shell injection at prompt time** | MCP | Hooks | Subagents | **SKILL.md** |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| **Claude Code** | CLAUDE.md (+AGENTS.md) | `.claude/commands` | Yes | **Yes — `!` preprocessor** | Yes | Yes | Yes | Yes (origin) |
| **OpenAI Codex** | AGENTS.md (layered) | `~/.codex/prompts` (deprecated → skills) | Yes | **No** | Yes | Yes | Yes (`.codex/agents/*.toml`) | Yes (`.agents/skills/`) |
| **Cursor** | `.cursor/rules` +AGENTS.md | `.cursor/commands` | No | **No** | Yes | Yes (richest event set) | Yes | Yes (`.cursor/skills/`) |
| **Gemini CLI** | GEMINI.md +AGENTS.md | TOML `.gemini/commands` | Yes | **Yes — `!{...}`** | Yes | No | No | **No** (uses extensions) |
| **GitHub Copilot / VS Code** | `.github/copilot-instructions.md` | `.github/prompts/*.prompt.md` | Yes | **No** | Yes | No | Partial | Yes (native) |
| **Aider** | CONVENTIONS.md +AGENTS.md | built-in only | No | No | No (bridges only) | No | No | No |
| **Windsurf** | `.windsurf/rules` +AGENTS.md | `.windsurf/workflows` | Partial | No | Yes | Yes (Cascade Hooks) | Partial | No |

The two takeaways: (a) `SKILL.md` is the widest-reaching portable unit — five
agents; (b) *nobody who reads `SKILL.md` supports prompt-time shell injection*.
The overlap of "reads SKILL.md" and "supports `!`" is empty.

### 3. Codex-specific mapping

Codex has converged closely on Claude Code's architecture, which makes it the
best first port target:

- **AGENTS.md** ↔ our `CLAUDE.md`: layered cascade (`~/.codex/` then repo
  root→cwd, nearer wins), `AGENTS.override.md`, `project_doc_max_bytes` cap.
  Direct map.
- **Skills**: native, same `SKILL.md`, progressive disclosure identical to CC.
  Discovered under `.agents/skills/` (repo + `$HOME` + `/etc/codex/skills`), not
  a plugin manifest. Optional `agents/openai.yaml` per skill for tool/invocation
  policy. Our `allowed-tools` has no exact 1:1 — tool scoping is via that YAML or
  the sandbox/approval model, so treat `allowed-tools` as advisory.
- **Custom prompts** (`~/.codex/prompts/`): exist but **deprecated in favour of
  skills**, home-only (not repo-shared), static substitution only. Don't port to
  these — port to skills.
- **Config**: `~/.codex/config.toml` + trusted `.codex/config.toml` (TOML, not
  JSON).
- **MCP**: client supported (`[mcp_servers.*]`). We ship none, so N/A unless we
  add.
- **Hooks**: supported with Claude-parity event names (`SessionStart`,
  `PreToolUse`, `PostToolUse`, …), `hooks.json` / `[hooks]` at `~/.codex/` and
  trusted `.codex/`. **Known gap: `PreToolUse.additionalContext` not yet
  supported** (openai/codex#19385) — so a PreToolUse guard can block/rewrite, but
  can't inject context the way our `config-detect.sh` does at SessionStart. Our
  `SessionStart` hooks map cleanly; our `PreToolUse` git-guard maps to
  block/rewrite.
- **Subagents**: supported, but defined as **TOML** in `.codex/agents/`
  (`name`/`description`/`developer_instructions` + optional `model`,
  `sandbox_mode`, `mcp_servers`, `skills.config`). Our `agents/*.md` bodies would
  become the `developer_instructions`.
- **Dynamic injection**: **no `!` preprocessor.** The supported path is a
  `SessionStart`/`UserPromptSubmit` hook that runs shell and returns stdout /
  `hookSpecificOutput.additionalContext`.
- **Plugin packaging**: reportedly a `.codex-plugin/plugin.json` manifest +
  marketplace deliberately parallel to `.claude-plugin/` (needs live
  verification — see Open Questions).

### 4. What changes we would have to make

Ordered roughly by effort/impact:

1. **Rework the `!`-preprocessor dependency (the big one, 44 files).** For every
   `` !`script` `` injection, the portable form is: keep the logic in `scripts/`,
   and change the skill body to *instruct the agent to run the script via its
   shell tool* and read the output — rather than relying on the harness to
   pre-expand it. On Claude Code we can keep the `!` form (or a shared idiom that
   still works). This is the pattern the cross-agent research explicitly
   recommends. It touches the most files and needs care to preserve behaviour
   (some injections compute values used later in the body, e.g. the dynamic
   `subagent_type` name).
2. **Neutralise `${CLAUDE_PLUGIN_ROOT}` (1,411 uses).** Introduce a resolver that
   maps to each agent's plugin-root convention (Codex plugins expose
   `PLUGIN_ROOT`/`PLUGIN_DATA`; `.agents/skills/` installs resolve differently).
   Likely a build/install step that rewrites paths per target, or a shared env
   shim.
3. **Frontmatter normalisation.** Conform to the Agent Skills spec (`name`
   matching folder, `description`; `allowed-tools` treated as advisory/
   experimental). Decide what to do with `argument-hint`,
   `disable-model-invocation`, `user-invocable` — keep for Claude Code, strip or
   translate for others.
4. **Hooks per target.** Translate `hooks/hooks.json` into Codex `hooks.json`
   (event names largely align) and, where needed, Cursor's `hooks.json` /
   Windsurf Cascade Hooks. Accept that hooks do **not** share a standard — this
   stays a per-agent adapter, and the `PreToolUse.additionalContext` gap on Codex
   means the SessionStart-context pattern has to carry that load.
5. **Agents per target.** Convert `agents/*.md` → Codex `.codex/agents/*.toml`.
   Preserve the locator/analyser tool split via per-agent tool scoping /
   `sandbox_mode`.
6. **Packaging + install layer.** There is no single canonical skills directory
   across vendors, so a portable bundle needs a small installer that places /
   symlinks skills into each agent's expected location and emits the right
   manifest (`.claude-plugin/` for CC, `.codex-plugin/` + `.agents/skills/` for
   Codex, `.cursor/skills/` for Cursor, etc.). Ship `AGENTS.md` alongside
   `CLAUDE.md` (complementary, covers the universal instruction slot).

### 5. Suggested scope / phasing (for a future work item)

- **Phase 0 — verify:** pin down the live Codex CLI behaviour (skills discovery,
  hooks, plugin manifest, `additionalContext` gap). Resolve the Open Questions.
- **Phase 1 — foundations:** ship `AGENTS.md`; conform `SKILL.md` frontmatter to
  the open standard; build the `${CLAUDE_PLUGIN_ROOT}` resolver + install layer.
- **Phase 2 — the injection rework:** convert the 44 preprocessor skills to the
  degrade-gracefully pattern, keeping Claude Code behaviour intact. This is the
  bulk of the effort and the main risk.
- **Phase 3 — Codex target:** hooks → Codex `hooks.json`, agents → TOML, plugin
  manifest, end-to-end test against the live Codex CLI.
- **Phase 4 — broaden:** Cursor / Copilot (skills mostly free once Phase 2 lands;
  hooks/agents are per-agent adapters). Gemini/Aider/Windsurf don't read
  `SKILL.md` — treat as instruction-file + workflow ports only, lower priority.

## Code References

- `skills/vcs/commit/SKILL.md:6-7` — `allowed-tools` scoped-Bash-glob grammar
- `skills/vcs/commit/SKILL.md:12-15,66` — representative `!`-preprocessor
  injections (VCS status/log, config context, per-skill instructions)
- `skills/config/paths/SKILL.md:11-16` — `user-invocable` vs
  `disable-model-invocation` preload subtlety
- `skills/config/paths/SKILL.md` — the `meta/` path-key protocol (15 keys)
- `skills/github/review-pr/SKILL.md:323-324` — dynamic `subagent_type` resolved
  via preprocessor at spawn time
- `hooks/hooks.json:1-44` — hook registration + `SessionStart`/`PreToolUse` events
- `hooks/vcs-guard.sh:96-108` — `PreToolUse` decision JSON contract
- `hooks/config-detect.sh:18-24` — `SessionStart` `additionalContext` injection
- `agents/*.md` — 9 subagents; `agents/browser-locator.md`, `browser-analyser.md`,
  `documents-locator.md` use the `skills:` preload field
- `.claude-plugin/plugin.json:1-29` — plugin manifest (directory-based `skills[]`)
- `CLAUDE.md:121` — minimum Claude Code v2.1.144 (subagent skill-preload)

## Architecture Insights

The single most important architectural fact: Accelerator's coupling to Claude
Code lives almost entirely at the **injection boundary** (the `!` preprocessor,
`${CLAUDE_PLUGIN_ROOT}`, the frontmatter semantics, the hook JSON contract), not
in the *content*. The `SKILL.md` bodies, the `meta/` filesystem protocol, the
templates, and the bash library are all substrate-neutral. That means
portability is primarily a *boundary-adapter* problem, not a rewrite — the bulk
of the value (the skills' actual instructions and the filesystem message-bus)
transfers intact. The dominant cost is mechanical (the 44-file injection rework)
plus a per-agent install/manifest/hook adapter.

## Historical Context

- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md:338-339`
  — confirms current "Claude Code only" targeting.
- `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md:160-164,523`
  — prior survey of cascading-config models across tools including Codex's
  `.codex/` dir-walking; useful precedent for the config/override design.

## Related Research

- `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md`
- `meta/research/codebase/2026-04-17-meta-visualiser-implementation-context.md`

## Open Questions

1. **Codex plugin/marketplace specifics** — is `.codex-plugin/plugin.json` and a
   marketplace really shipping and stable, or partly announced? Verify against
   the live CLI (`codex plugin marketplace list`, `codex --version`).
2. **Codex `PreToolUse.additionalContext`** — still unsupported (issue #19385)?
   This decides whether our SessionStart config-injection pattern is enough.
3. **`allowed-tools` honouring** — which agents actually enforce it vs treat it
   as advisory? Affects whether we keep it, translate it, or drop it.
4. **`${CLAUDE_PLUGIN_ROOT}` equivalents** — confirm each target's plugin-root
   env var / install path so the resolver is correct.
5. **Do we maintain one source tree with an install/transform step, or per-agent
   published bundles?** This is the key architectural decision for a work item —
   it shapes everything downstream.
6. **Which agents are actually in scope?** Codex is the clear first target
   (closest architecture). Cursor/Copilot are cheap follow-ons for skills.
   Gemini/Aider/Windsurf don't read `SKILL.md` and are a different (larger) lift
   — confirm whether they're in scope or explicitly out.
