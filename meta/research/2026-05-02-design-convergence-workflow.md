---
date: 2026-05-02T23:37:50+01:00
researcher: Toby Clemson
git_commit: ae21ddf823b5777728ae649e038feab98d88d5ae
branch: main
repository: accelerator
topic: "Design convergence workflow: inventory-and-diff approach for aligning a current frontend with a Claude Design prototype"
tags: [research, design, convergence, frontend, inventory, agents, skills, mcp, playwright]
status: complete
last_updated: 2026-05-03
last_updated_by: Toby Clemson
last_updated_note: "Revised inventory storage to directory-per-inventory so screenshots are dated alongside the markdown they belong to (§3.1, §5.1, §5.2, §8, §9.9)."
---

# Research: Design Convergence Workflow — Inventory-and-Diff Approach

**Date**: 2026-05-02T23:37:50+01:00
**Researcher**: Toby Clemson
**Git Commit**: ae21ddf823b5777728ae649e038feab98d88d5ae
**Branch**: main
**Repository**: accelerator

## Research Question

How can we use the Accelerator plugin's filesystem-artifact patterns to drive a repeatable workflow that converges a current functional frontend onto a Claude Design prototype — capturing both visual differences and missing functional capabilities, generating an actionable backlog automatically, and remaining stable as both sides independently evolve?

The motivating scenario: a working but basic frontend exists; a more sophisticated prototype has been produced via Claude Design; the engineering work is to converge the former onto the latter, where the gap includes both pure design differences and net-new functional capabilities. The user wants a repeatable process — not a one-shot conversion.

## Summary

The recommended solution is an **inventory-then-diff workflow** built from three filesystem artifact types and two new skills, reusing the existing `codebase-*` agent pair for code-side observation and adding one new `browser-*` agent pair for runtime observation of the rendered application via Playwright MCP.

The workflow produces:

1. Two **`design-inventory`** artifacts — one snapshot per source (current app, prototype). Same schema applied to either; same `source` identifier means newer supersedes older. Each captures the design system, component catalogue, screen inventory, feature catalogue, and information architecture as observed via static code analysis and runtime browser inspection.
2. One **`design-gap`** artifact — the structured comparison of two inventories, organised into Token Drift / Component Drift / Screen Drift / Net-New Features / Removed Features. Body is prose-rich with each gap written in actionable language so the existing `extract-work-items` skill can pick gaps up via its natural-language cue-phrase detection without requiring structural metadata blocks.
3. A natural feeder into the existing chain: `design-gaps → extract-work-items → meta/work/ → research-codebase → create-plan → implement-plan`. The new artifacts plug into the existing workflow rather than competing with it.

The two new skills — `inventory-design` and `analyse-design-gaps` — sit under a new `skills/design/` category. The two new agents — `browser-locator` and `browser-analyser` — handle runtime inspection only; code-side work delegates to the existing `codebase-locator` / `codebase-analyser` pair. Playwright MCP integrates via a sibling `.mcp.json` at the plugin root (inline `mcpServers` in `plugin.json` is unreliable per Claude Code issue #16143).

Screenshots are versioned in the repo under each inventory's directory, providing audit trail and verification value at the cost of some repo size.

The architecture is precedent-setting in two specific ways — first MCP server dependency in the plugin, first `skills/design/` category — but neither requires schema changes. Templates are auto-discovered from `templates/`; path config keys are arbitrary lookups in user config with caller-supplied defaults.

## Detailed Findings

### 1. Problem framing and approach selection

The user proposed a screenshot-diff approach as one option. This was evaluated and rejected as a *primary* mechanism (though it is retained as a verification mechanism downstream). The reasoning:

- Pixel-level diffs catch visual drift but cannot detect interaction differences, hidden states (modals, error/loading/empty), conditional flows, or features that live behind a click.
- A screenshot-diff produces a long list of "this looks different" without distinguishing whether it is a token change, a layout change, or a missing feature — the three of which require very different implementation work.
- It is a discovery mechanism that produces no durable artifact; the next time both sides have evolved, the work starts from scratch.

The selected approach instead builds **structured inventories** of each side and computes a **structural diff** between them. Inventories are durable, version-controlled, regenerable, and team-visible — aligning with the principles in `meta/decisions/ADR-0001-context-isolation-principles.md`. Screenshots are captured per screen during inventory generation as evidence and verification material, but are not the source of truth for difference detection.

The inventory-then-diff approach is more expensive on the first pass (the inventory generator and gap analyser must be built) but converges quickly — second and subsequent runs are a single command per side plus an automatic diff.

### 2. Workflow chain and integration with existing skills

The convergence work plugs into the existing `meta/`-driven workflow rather than introducing a parallel system:

```
design-inventory (current)  ─┐
                             ├──▶ design-gap ──▶ extract-work-items ──▶ meta/work/*
design-inventory (prototype) ┘                                             │
                                                                           ▼
                                                       research-codebase ──▶ meta/research/*
                                                                           │
                                                                           ▼
                                                           create-plan ──▶ meta/plans/*
                                                                           │
                                                                           ▼
                                                       implement-plan ──▶ code changes
```

Critically, **the gap artifact is not a plan**. It is structured input to `extract-work-items`, which is a known skill at `skills/work/extract-work-items/SKILL.md`. The existing extraction logic operates by spawning `documents-analyser` subagents that detect actionable language (cue-phrases like "users need…", "we need to implement…", "the system must…") in document prose, then loops interactively per detected candidate. There is no rigid heading or frontmatter contract on the input — the only structural constraint is that bare headings without descriptive prose are skipped.

This means each gap entry in the gap artifact must be **written as actionable prose**, not as a bullet list of titles. A gap reading "we need to migrate the colour scale from the current 14-hue palette to the prototype's 8-token system, affecting every component that hardcodes hex values" will be picked up; a heading reading "Color drift" with no prose underneath will be skipped.

This integration is a key validation of the dedicated `design-gap` artifact type (rather than emitting a regular `plan` directly): gaps need a structured intermediary that downstream skills can consume independently of the plan/implementation cycle. A team can choose to extract only some gaps as work items, leave others, and re-extract later — without the artifact needing to change.

### 3. Artifact schemas

#### 3.1 `design-inventory`

**Path**: `<paths.design_inventories>/YYYY-MM-DD-{source}/inventory.md` (default `meta/design-inventories/`)

Each inventory is a **directory**, not a flat file. The directory name carries the date and source identity (`YYYY-MM-DD-{source}`); inside it, a fixed-named `inventory.md` holds the markdown, and a `screenshots/` subdirectory holds the per-screen captures. This keeps point-in-time assets travelling with the inventory they belong to: a regenerated snapshot lives in a new dated directory with its own screenshot set, leaving the previous snapshot's screenshots intact for audit and verification (see §8 for rationale).

**Template**: `templates/design-inventory.md` (auto-discovered via `config-common.sh:103-113`)
**Config key**: `paths.design_inventories`

**Frontmatter**:

```yaml
---
date: "2026-05-02T10:00:00+00:00"
type: design-inventory
source: current-app                # kebab-case identifier; the join key
source_kind: code-repo             # code-repo | prototype | running-app
source_location: ../webapp         # path / URL / claude.ai/design link
git_commit: 7f3a2b1c               # if source is a code repo (omit otherwise)
branch: main
crawler: hybrid                    # code-static | playwright-runtime | hybrid
author: Toby Clemson
status: draft                      # draft | accepted | superseded
tags: [design, inventory, current-app]
last_updated: "2026-05-02T10:00:00+00:00"
last_updated_by: Toby Clemson
---
```

The `source` field is the **join key** — two inventories with the same `source` are versions of the same subject, with the newer superseding the older. Two inventories with different `source` values are diff-able (one becomes the "current", the other the "target").

**Body sections**:

```markdown
# Design Inventory: {source}

## Overview
- Scope (which routes/areas covered, which excluded)
- Crawler methodology (static read, runtime crawl, both)
- Known gaps (auth-gated areas, dynamic content not reached, intentional exclusions)

## Design System

### Tokens
Tables of `name: value` for colours, typography, spacing, radii, shadows, motion.
Source file:line refs where available.

### Layout primitives
Grid system, container widths, breakpoints, z-index scale.

## Component Catalogue

### {ComponentName}
- Variants / props
- Used on screens: [screen-id, screen-id]
- Source: file:line (for code) or selector path (for runtime)

(repeat per component)

## Screen Inventory

### {screen-id} — {route or URL}
- Purpose (one line)
- Components used (refs into Component Catalogue)
- States observed: loading | empty | error | success | partial
- Key interactions (click → outcome)
- Screenshot: `screenshots/{screen-id}.png` (if Playwright was used; relative to the inventory directory)

(repeat per screen)

## Feature Catalogue

### {feature-id}
- Capability (one sentence, screen-independent)
- Surfaces on: [screen-id, ...]
- Depends on: [external API name, state slice, ...]

(repeat per feature)

## Information Architecture
Route table or navigation graph (textual, or Mermaid diagram if rendering well).

## Crawl Notes
Anything that surprised the crawler, dead-ends, auth walls, dynamic content gaps.

## References
Source paths, prototype URLs, related ADRs / research / inventories.
```

The structure is **deliberately parallel** between Component Catalogue and Screen Inventory entries — each Screen entry references components from the catalogue; each Feature entry references screens. This creates the cross-link graph that the diff process exploits in the gap artifact.

#### 3.2 `design-gap`

**Path**: `<paths.design_gaps>/YYYY-MM-DD-{slug}.md` (default `meta/design-gaps/`)
**Template**: `templates/design-gap.md` (auto-discovered)
**Config key**: `paths.design_gaps`

**Frontmatter**:

```yaml
---
date: "2026-05-02T10:30:00+00:00"
type: design-gap
current_inventory: meta/design-inventories/2026-05-02-current-app/inventory.md
target_inventory: meta/design-inventories/2026-05-02-prototype/inventory.md
author: Toby Clemson
status: draft                      # draft | accepted | superseded
tags: [design, gap-analysis]
---
```

**Body sections** (prose-rich, designed for `extract-work-items` consumption):

```markdown
# Design Gap Analysis: {current-source} → {target-source}

## Overview
One paragraph framing what was compared, when, and at what fidelity.
Explicitly notes any limitations (one inventory partial, auth-walled
areas excluded, etc.).

## Token Drift
[Intro paragraph framing the category and why the drift matters.]

The colour palette in the current app uses 14 distinct hues that
do not map cleanly onto the prototype's 8-token scale. We need
to migrate the codebase to use the prototype's named tokens
({primary}, {primary-muted}, {surface-1}, …) — this affects every
component that currently hardcodes hex values, primarily under
src/components/ and src/styles/. *Refs: current-app inventory
§Design System / Tokens; prototype inventory §Design System / Tokens.*

The spacing scale in the current app is on a 5px grid; the prototype
uses a 4px grid. We need to update the Tailwind configuration to
align, and audit all components for spacing values that no longer
resolve cleanly. *Refs: current-app §Design System / Tokens / spacing;
prototype §Design System / Tokens / spacing.*

(more token-drift paragraphs)

## Component Drift
[Intro paragraph.]

The Button component in the current app exposes only two variants
(primary, secondary), while the prototype's Button has five
(primary, secondary, ghost, destructive, link). We need to extend
the Button component to support all five variants and migrate
existing usages where the prototype design specifies a different
variant. *Refs: current-app §Component Catalogue / Button;
prototype §Component Catalogue / Button.*

(more component-drift paragraphs)

## Screen Drift
[Intro paragraph — visual/structural differences within screens
that exist on both sides.]

(prose paragraphs per screen-level gap)

## Net-New Features
[Intro paragraph — capabilities present in the prototype but
absent in the current app.]

The prototype includes a search bar in the global header that
supports keyboard-shortcut activation (Cmd+K), recent search
history, and inline result previews. Users need this discovery
surface — it does not exist anywhere in the current app today.
We need to design and build it from scratch, including the
backing search API. *Ref: prototype §Feature Catalogue / search.*

(more net-new-feature paragraphs)

## Removed Features
[Intro paragraph — capabilities in the current app that the
prototype does not include. Each entry should explicitly call
out whether removal is intended or whether the prototype omitted
it by oversight.]

(prose paragraphs per removed-feature gap)

## Suggested Sequencing
A short prose recommendation on phasing: tokens before components
before screens before features. Not prescriptive — sequencing is
a planning concern that emerges in create-plan, not here.

## References
- Current inventory: link
- Target inventory: link
- Related ADRs / research
```

**Key constraint**: every gap paragraph must be written in **actionable language** with cue-phrases. Phrases that work well for `extract-work-items`:

- "We need to…" / "Users need…" / "The system must…"
- "Implement {X} to support {Y}"
- Explicit acceptance criteria where useful

Phrases to avoid: passive descriptions of state without an actionable verb ("the spacing is different"), bare headings, bullet lists of titles.

The body sections are organisational, but extraction operates at paragraph level — categories don't need to be balanced (a gap analysis with 30 token drifts and 2 net-new features is fine). The H2 categories may be omitted if a category has no gaps.

### 4. Agents

#### 4.1 Reused agents (no changes)

The existing `codebase-locator` and `codebase-analyser` agents (`agents/codebase-locator.md`, `agents/codebase-analyser.md`) handle all code-side observation:

- `codebase-locator` (tools: Grep, Glob, LS) — finds candidate files: route definitions, component directories, design-token files (e.g. `tailwind.config.*`, `theme.ts`, `tokens.css`), stylesheets, framework-specific entry points.
- `codebase-analyser` (tools: Read, Grep, Glob, LS) — reads the focused list to extract Design System tokens and Component Catalogue entries.

Per the precedent set by `skills/research/research-codebase/SKILL.md`, the orchestrating skill instructs these agents on what to look for; the agents themselves remain framework-agnostic and reusable.

#### 4.2 New agent pair: `browser-locator` and `browser-analyser`

Two new agents handle runtime observation only. They have no file-search or file-read tools — their entire surface is the rendered application via Playwright MCP.

**`agents/browser-locator.md`**

Frontmatter:

```yaml
---
name: browser-locator
description: Locates routes, screens, and DOM-level component presence in a running web application via the Playwright MCP server. Call browser-locator when you need to enumerate WHERE things appear in the rendered UI, not to extract their detail.
tools: mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot
---
```

Body skeleton (mirroring the canonical locator/analyser pattern):

- Role-stating opener: "You are a specialist at finding WHERE things appear in a running web application. Your job is to enumerate routes, screens, and DOM-level component presence — NOT to extract their detail."
- Core Responsibilities, Search Strategy (numbered), Output Format (literal markdown template with sections like `### Routes`, `### Components on each screen`, `### State indicators`), Important Guidelines, What NOT to Do, "Remember:" closer.
- The "What NOT to Do" list explicitly forbids screenshots, evaluation expressions, interactions (clicks/typing), and reading any source files.

**`agents/browser-analyser.md`**

Frontmatter:

```yaml
---
name: browser-analyser
description: Analyses a focused set of screens in a running web application via the Playwright MCP server. Captures detailed state, screenshots, and computed values. Call browser-analyser when you need to extract HOW a screen behaves, not to enumerate WHERE things are.
tools: mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot, mcp__playwright__browser_take_screenshot, mcp__playwright__browser_evaluate, mcp__playwright__browser_click, mcp__playwright__browser_type, mcp__playwright__browser_wait_for
---
```

Body skeleton: same shape, role contrasts ENUMERATE vs EXTRACT. Output Format mandates per-screen blocks with state matrix (loading / empty / error / success), interaction outcomes, and screenshot paths.

Both agents:
- Have **no file-system tools** — keeping the new pair single-modality.
- Have **no Write tool** — outputs go via the orchestrator.
- Bound their context strictly to the rendered surface.

#### 4.3 Why this split (not a single design-* pair, not extending codebase-*)

The decision rationale:

1. **Single-modality discipline.** The existing `codebase-*` and `documents-*` pairs each operate on a single surface (filesystem code, filesystem documents). Folding code work and runtime work into a single new pair would have given those agents two tool clusters and a forked role statement — a soft violation of the bounded-context invariant from ADR-0001.
2. **Reuse over duplication.** The `codebase-*` agents already know how to find React components, Vue SFCs, route configs, etc. — they don't need to know they are being used for "design inventory" purposes. The orchestrating skill instructs them what to look for.
3. **Future composability.** Naming the runtime pair `browser-*` (rather than `design-*`) frees them for reuse: a future skill needing browser inspection for performance audits, accessibility audits, or e2e exploration can compose `browser-*` without inheriting design-specific output formats.
4. **Tool-list cleanliness.** Each pair has a coherent tool list. `codebase-*` keeps its existing minimal toolset; `browser-*` has nothing but Playwright MCP tools. No agent in the system holds both file-search and browser-inspection capabilities.

### 5. Skills

Both new skills live under a new `skills/design/` category. Existing categories (`config`, `decisions`, `github`, `planning`, `research`, `review`, `vcs`, `visualisation`, `work`) suggest `design/` is a natural fit at the same level.

#### 5.1 `inventory-design`

**Path**: `skills/design/inventory-design/SKILL.md`

**Argument shape**: positional with optional flag — matching every existing skill in the plugin (e.g. `create-adr "[topic] [--supersedes ADR-NNNN]"`):

```
argument-hint: "[source-id] [location] [--crawler code|runtime|hybrid]"
```

The two positional args are required; the `--crawler` flag is optional and defaults to `hybrid` for code-repo sources, `playwright-runtime` for prototype/running-app sources.

**Skeleton** (following `skills/research/research-codebase/SKILL.md` as the canonical orchestration template):

```markdown
---
name: inventory-design
description: Generate a design inventory artifact for a frontend source — code repo, hosted prototype, or running app. Captures design system, components, screens, features, and IA for use as input to design-gap analysis.
argument-hint: "[source-id] [location] [--crawler code|runtime|hybrid]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/*)
---

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh inventory-design`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:codebase-locator, accelerator:codebase-analyser,
accelerator:browser-locator, accelerator:browser-analyser,
accelerator:documents-locator, accelerator:documents-analyser.

**Inventory directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories meta/design-inventories`

# Generate Design Inventory

## Process

### 1. Resolve source
- Parse [source-id], [location], optional --crawler from arguments.
- Validate location reachable.
- Determine source_kind: code-repo (path), prototype (URL), running-app (URL).
- If --crawler not specified, default to hybrid for code-repo, playwright-runtime otherwise.

### 2. Choose crawl strategy
- code-static: spawn {codebase locator agent} for routes/components/tokens; spawn {codebase analyser agent} to extract Design System and Component Catalogue from focused list.
- playwright-runtime: spawn {browser locator agent} to enumerate routes and DOM-level component presence; spawn {browser analyser agent} per screen to capture states and screenshots.
- hybrid: both, with code-static as ground truth for tokens/components and runtime filling in screen states.

### 3. Spawn agents in parallel where independent
Each agent produces one section of the inventory. Use the locator/analyser pattern: locators identify the focused set; analysers extract detail.

### 4. Synthesise
- Merge agent outputs into a single inventory body.
- Cross-link: every Screen entry must reference Components from the catalogue; every Feature must reference Screens.
- Note unreachable areas in Crawl Notes — never fabricate.

### 5. Generate metadata
!`${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/inventory-metadata.sh`

### 6. Write artifact
- Create the per-inventory directory `YYYY-MM-DD-{source-id}/` inside the resolved inventory root (referred to below as `$INV_DIR`).
- Write the markdown to `$INV_DIR/YYYY-MM-DD-{source-id}/inventory.md`.
- Body from template:
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh design-inventory`
- Save screenshots to `$INV_DIR/YYYY-MM-DD-{source-id}/screenshots/{screen-id}.png`.
- If a prior inventory directory with the same `{source-id}` suffix exists, mark its `inventory.md` frontmatter `status: superseded`. Leave its screenshots in place — the superseded directory remains a complete point-in-time record.

### 7. Present summary
- Counts: tokens, components, screens, features.
- Gaps flagged in Crawl Notes.
- Suggest the next command: `/accelerator:analyse-design-gaps {current-id} {target-id}`.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh inventory-design`
```

#### 5.2 `analyse-design-gaps`

**Path**: `skills/design/analyse-design-gaps/SKILL.md`

**Argument shape**:

```
argument-hint: "[current-source-id] [target-source-id]"
```

Both positional, both required. The skill resolves each source ID to its most-recent non-superseded inventory directory (matching `*-{source-id}/`) and reads the `inventory.md` inside.

**Skeleton**: structurally simpler than `inventory-design` because it does not need to spawn locator/analyser agents — it consumes existing inventory artifacts. The body:

1. Resolve the two inventory directories from the source IDs and read each `inventory.md`.
2. Read both inventories fully into context (they are designed to be self-contained).
3. Compute structural diff:
   - Token Drift: tokens present in one but not the other; tokens with diverged values.
   - Component Drift: components present in one only; components with diverged variants/props/usage.
   - Screen Drift: screens present in one only; screens with diverged components or states.
   - Net-New Features (in target, not in current).
   - Removed Features (in current, not in target — flag for explicit confirmation).
4. Convert each diff item into a prose paragraph with cue-phrase language, written for `extract-work-items` consumption.
5. Generate metadata, write artifact to `<paths.design_gaps>/YYYY-MM-DD-{slug}.md` using `templates/design-gap.md`.
6. Present summary; suggest `/accelerator:extract-work-items` as next step.

The prose-generation step is the load-bearing one. The skill's instructions must firmly direct the model to write each gap as a paragraph that includes a verb phrase ("we need to…", "users need…", "the system must…") rather than as a heading or bullet list. This is what makes the gap artifact extraction-ready.

### 6. Browser tooling integration

#### 6.1 Choice of Playwright

A landscape review confirmed Playwright is the dominant headless browser automation tool in 2026. Alternatives evaluated and rejected:

- **Puppeteer**: narrower (Chromium-first historically), MCP support is community-maintained.
- **Selenium**: legacy, slow, broader browser/OS support that this workflow does not need.
- **Cypress**: test-runner, not an inspection tool — wrong shape.
- **Chrome DevTools MCP**: lower-level than Playwright; better for performance profiling than app exploration.
- **WebDriver BiDi**: emerging W3C standard, MCP support immature.

Anthropic ships an official Playwright MCP server with first-class accessibility-tree snapshot support — which is the primary tool the `browser-*` agents need to enumerate components and states without consuming pixel-level data.

#### 6.2 Plugin integration

Claude Code's plugin schema supports MCP server declarations in two forms:

- Inline `mcpServers` field in `.claude-plugin/plugin.json`.
- Sibling `.claude-plugin/.mcp.json` file.

**Recommended form: sibling `.mcp.json`.** Inline `mcpServers` in `plugin.json` is currently broken — the field is stripped during manifest parsing per [Claude Code issue #16143](https://github.com/anthropics/claude-code/issues/16143). The loader code exists but does not fire. Sibling `.mcp.json` works today and is documented in the official Claude Code MCP docs.

A representative `.claude-plugin/.mcp.json` entry:

```json
{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["@playwright/mcp@latest"]
    }
  }
}
```

This makes `mcp__playwright__browser_*` tools available to skills and agents in the plugin.

#### 6.3 Precedent-setting note

Adding `.mcp.json` to this plugin is the **first MCP dependency** in the Accelerator codebase. There is no existing `mcpServers` block, no `.mcp.json` file, and no skill `allowed-tools` line that permits `mcp__*` tool patterns today. This implies:

- README install section needs an addition explaining the Playwright MCP dependency (one-time setup).
- The new agents' frontmatter is the first to use `mcp__*` tool patterns.
- Future MCP-based skills can follow this precedent.

### 7. Infrastructure changes required

The plugin's path/template registration is mostly auto-discovery, with a few explicit lists that need updating.

#### 7.1 Templates (auto-discovered — no wiring)

`scripts/config-common.sh:103-113` (`config_enumerate_templates`) globs `<plugin_root>/templates/*.md` and exposes each basename as a valid template key. The three-tier resolver (`config_resolve_template`, `config-common.sh:153-193`) tries user config → user templates dir → plugin default. There is no allowlist.

**Required**: drop two new template files:

- `templates/design-inventory.md`
- `templates/design-gap.md`

Both become valid template keys immediately. No script edit required.

#### 7.2 Path config keys (no allowlist, but defaults are caller-supplied)

`scripts/config-read-path.sh` is a thin wrapper around `config-read-value.sh` that accepts arbitrary subkeys under `paths.`. Defaults are passed by each caller as the second argument:

```bash
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories meta/design-inventories`
```

**Required**: every consumer skill must pass the explicit default. No script edit required for the path mechanism itself, but the conventional defaults (`meta/design-inventories`, `meta/design-gaps`) must be supplied at every invocation site or the empty default propagates.

#### 7.3 Init skill (explicit list)

`skills/config/init/SKILL.md` (lines 20-31, 40, 105-117) hardcodes the directories `/accelerator:init` creates with `.gitkeep` on a fresh install. This includes a count ("12 directories") and a for-loop body.

**Required**: add `meta/design-inventories/` and `meta/design-gaps/` to the init list, update the count to 14, update the summary block.

#### 7.4 Configure skill (documentation)

`skills/config/configure/SKILL.md` (lines 386-399) contains a documentation table of `paths.*` keys for `/accelerator:configure help`. No validation — but keeping the help output accurate matters.

**Required**: add `paths.design_inventories` and `paths.design_gaps` rows.

#### 7.5 README (documentation)

The README (`README.md` lines 77-88, 213-214) lists meta/ subdirectories and template keys.

**Required**: add `design-inventories/`, `design-gaps/` to the meta/ table; add `design-inventory`, `design-gap` to the template keys list.

#### 7.6 Plugin manifest (MCP dependency)

`.claude-plugin/plugin.json` is unchanged. The new `.claude-plugin/.mcp.json` file is added (as discussed in §6.2). The skill `allowed-tools` lines for `inventory-design` need to permit `mcp__playwright__*` patterns (in addition to the Bash patterns shown above).

### 8. Screenshot storage

Each inventory is a **directory**, not a flat file, so its screenshots travel with it as point-in-time assets:

```
meta/design-inventories/
  2026-05-02-current-app/
    inventory.md
    screenshots/
      home.png
      product-detail.png
  2026-05-02-prototype/
    inventory.md
    screenshots/
      home.png
      product-detail.png
  2026-06-15-current-app/        # next regeneration of the current app
    inventory.md
    screenshots/
      home.png
      product-detail.png
```

Inventories reference their screenshots via paths relative to their own directory (`screenshots/{screen-id}.png`). When a new inventory is generated for the same source, it lands in a new dated directory; the prior directory is marked `status: superseded` in its frontmatter but is **not** deleted — its screenshots remain in place as a complete point-in-time record.

This design has three properties worth calling out:

1. **Dated assets.** Screenshots inherit the date of the inventory directory, so they are inherently versioned without a separate dating scheme. There is no risk of a regenerated inventory overwriting prior screenshots.
2. **Self-contained inventories.** Each `YYYY-MM-DD-{source}/` directory is portable and complete — it can be moved, archived, or referenced independently of its siblings.
3. **Diff-friendly markdown.** The `inventory.md` files diff cleanly across snapshots (`jj diff meta/design-inventories/2026-05-02-current-app/inventory.md meta/design-inventories/2026-06-15-current-app/inventory.md`) without screenshot binaries entering the diff stream unless explicitly requested.

Tradeoffs:

- **Pro**: inventories are self-contained and auditable; visual diffs can be reviewed in PRs; screenshots travel with the artifact across machines and across regenerations.
- **Pro**: directory-per-inventory diverges from the flat-file convention used by `meta/research/`, `meta/plans/`, etc., but the divergence is justified by the unique asset-bearing requirement of inventories.
- **Con**: repo size grows with each regeneration; image diffs are not particularly review-friendly in standard git/jj tooling.
- **Con**: tooling that enumerates inventories must look at directories rather than `.md` files (e.g. `documents-locator` would need updating if it ever surfaces inventories — currently out of scope).

Mitigations: a future enhancement could introduce a `screenshots-prune` script that removes screenshots from superseded inventory directories older than N regenerations, while keeping their `inventory.md` files for textual audit trail.

### 9. Considered and rejected alternatives

#### 9.1 Single combined artifact (current + prototype + diff)

Rejected. Re-generating one side requires re-doing the entire artifact. Inventories as separate documents allow regeneration of just the side that changed.

#### 9.2 Convergence-as-plan (skip dedicated gap type)

Rejected. The gap analysis has structural sections (Token Drift / Component Drift / Screen Drift / Net-New Features / Removed Features) that don't map cleanly onto plan phases. More importantly: the gap is *upstream* of work items, which are *upstream* of plans. Conflating the layers loses the ability to extract a subset of gaps as work items.

#### 9.3 Single new agent pair handling both code and runtime work

Rejected. Folding both surfaces into one pair gives the agent two tool clusters (file-search + browser-MCP) and a forked role statement. The clean separation — reuse `codebase-*`, add `browser-*` — keeps each agent single-modality and frees the runtime pair for future reuse.

#### 9.4 Naming the runtime pair `design-*`

Rejected. The agents are not design-specific — they are generic browser inspectors. Naming them for their first consumer creates a rename burden if a future skill (perf audit, a11y audit, e2e exploration) wants to compose them. `browser-*` names them for what they actually do.

#### 9.5 Inline `mcpServers` in `plugin.json`

Rejected pending fix to issue #16143. Sibling `.mcp.json` is the working alternative today.

#### 9.6 Fully named-flag arguments (`--source <id> --location <path>`)

Rejected. Every existing skill in the plugin uses positional-first arguments with optional flag modifiers. Adopting fully-named flags as a one-off would set a new convention out-of-band. If the plugin should shift to named-flag conventions, that is its own ADR — it should not piggyback on this work.

#### 9.7 H3 + structured-metadata blocks per gap

Rejected. The existing `extract-*` skills (`extract-work-items`, `extract-adrs`) use natural-language pattern detection via `documents-analyser`, not structural parsing. Prose-rich entries with cue-phrase language are picked up automatically; H3 + frontmatter blocks would require modifying the extract skills, which is unnecessary work.

#### 9.8 Nested layout under `meta/research/`

Considered. The user raised the option of restructuring `meta/research/` into subdirectories (e.g. `research/codebase/`, `research/ideas/`, `research/design-inventories/`). This is a reasonable structural improvement but is **a separate refactor**: it changes how every existing research file is organised, affects the `research-codebase` skill's path resolution, and warrants its own ADR. Bundling it with this work would conflate two changes. The flat layout (`meta/design-inventories/`, `meta/design-gaps/`) is recommended for now; the nested-research restructure is noted as a future follow-up (see `meta/notes/2026-05-02-research-directory-subcategory-restructure.md`).

#### 9.9 Flat-file inventories with shared screenshots directory

Initially proposed: each inventory as a flat `YYYY-MM-DD-{source}.md` file alongside a sibling `screenshots/{source}/` directory. Rejected. Screenshots are point-in-time captures and need to share the inventory's date so a regenerated inventory does not overwrite or commingle with the previous snapshot's screenshots. Putting each inventory in its own dated directory (`YYYY-MM-DD-{source}/inventory.md` plus `YYYY-MM-DD-{source}/screenshots/`) makes the per-snapshot bundle self-contained and inherently versioned.

This diverges from the flat-file convention used by `meta/research/`, `meta/plans/`, and `meta/decisions/`, but the divergence is justified by inventories' unique need to bundle non-textual assets with the artifact. Other artifact types in the workflow (`design-gap`, plans derived from extracted work items) remain flat files because they have no such asset-bundling requirement.

## Code References

- `meta/decisions/ADR-0001-context-isolation-principles.md` — bounded-context discipline that informs the agent split.
- `agents/codebase-locator.md`, `agents/codebase-analyser.md` — existing agents reused for code-side observation; canonical locator/analyser skeleton.
- `agents/documents-locator.md`, `agents/documents-analyser.md` — sister pair establishing the single-modality precedent.
- `skills/research/research-codebase/SKILL.md` — canonical orchestration skill structure that `inventory-design` mirrors.
- `skills/work/extract-work-items/SKILL.md:130-138` — cue-phrase patterns the analyser detects in source prose.
- `skills/work/extract-work-items/SKILL.md:496-499` — explicit rule that bare headings are skipped by extraction.
- `skills/decisions/extract-adrs/SKILL.md:80-91` — analogous decision-language patterns; sibling extract skill.
- `scripts/config-read-path.sh:24` — thin wrapper over generic value reader; no allowlist.
- `scripts/config-common.sh:103-113` — `config_enumerate_templates` glob discovery.
- `scripts/config-common.sh:153-193` — `config_resolve_template` three-tier resolver.
- `skills/config/init/SKILL.md:20-31, 40, 105-117` — explicit directory list for `/accelerator:init`.
- `skills/config/configure/SKILL.md:386-399` — paths documentation table.
- `README.md:77-88, 213-214` — meta/ table and template keys list.
- `templates/research.md`, `templates/plan.md`, `templates/work-item.md` — template-file conventions for the new templates to mirror.

## Architecture Insights

The proposed workflow is fully consistent with the architectural principles articulated in ADR-0001 and reinforces them:

1. **Filesystem-as-shared-memory.** Every artifact in the workflow is a structured markdown file in `meta/`, version-controlled, team-visible, and readable across sessions. No state lives only in conversation.

2. **Single-modality agents with bounded context.** The `codebase-*` pair operates on filesystem code only. The `documents-*` pair operates on filesystem documents only. The new `browser-*` pair operates on the rendered application only. Each pair has a tool list narrow enough that a single agent cannot hold both broad search results and deep content simultaneously.

3. **Locator/analyser separation.** The new `browser-*` pair preserves the existing pattern: `browser-locator` (navigate + accessibility-tree snapshot) enumerates without screenshotting or interacting; `browser-analyser` (full Playwright tool set) performs deep per-screen extraction.

4. **Workflow chaining via filesystem handoff.** `design-inventories → design-gaps → extract-work-items → meta/work/* → research-codebase → create-plan → implement-plan` is a chain of filesystem handoffs, each consumable independently. A team can stop at any step, regenerate any artifact, or pick up from any artifact.

5. **Skills orchestrate; agents specialise.** The `inventory-design` skill knows which agents to spawn for what; agents themselves remain framework-agnostic and reusable.

6. **Configuration without ceremony.** Templates auto-discovered. Path keys are arbitrary. The new artifact types add no schema changes.

A new pattern this work introduces — and which may inform future workflows — is **structural inventory-and-diff**. Where existing skills compute things (research, plans, ADRs) from a single document or a code area, this workflow computes a diff between two structurally-equivalent artifacts. If similar workflows emerge in the future (e.g. comparing two API specifications, two database schemas, two architecture diagrams), the inventory-and-diff pattern can be reapplied with new artifact types but the same orchestration shape.

## Historical Context

- `meta/decisions/ADR-0001-context-isolation-principles.md` directly informs the agent decisions: bounded context per agent, locator/analyser separation, filesystem as the inter-phase communication channel. Every architectural decision in this research aligns with the ADR.
- The existing `meta/research/` body shows the prose conventions the gap artifact mirrors: paragraph-rich, action-oriented language is already the norm for research documents, which is partly why `extract-work-items` operates on natural-language patterns rather than structural markers.
- The plugin has not previously declared any MCP server dependencies. This work is precedent-setting in that regard, but the plugin schema supports the addition cleanly.

## Related Research

No prior research documents in `meta/research/` directly address frontend convergence, design inventory, or browser automation in this plugin. This document establishes the problem space.

A related plugin initiative — the ticket-management work captured in the user's auto-memory ([Ticket management initiative](file:///Users/tobyclemson/.claude/projects/-Users-tobyclemson-Code-organisations-atomic-company-accelerator/memory/project_ticket_management.md)) — overlaps in that both work areas feed into `extract-work-items`. The design-convergence work increases the value of the work-item lifecycle by adding a new high-volume feeder.

## Open Questions

1. **Issue #16143 timeline.** When inline `mcpServers` in `plugin.json` is fixed, should we migrate from `.mcp.json` to inline declaration? Probably yes for consistency, but a tracking note (perhaps in the README install section) is enough until then.

2. **Authentication handling for runtime crawls.** The `browser-analyser` agent has `browser_type` and `browser_click` to support login flows, but the inventory skill needs a strategy for credential handling (env vars? prompt the user? skip auth-walled areas?). Worth a small dedicated section in the skill instructions, or a follow-up ADR if patterns emerge.

3. **Framework-specific token discovery.** The `codebase-analyser` agent is generic; framework-specific knowledge (Tailwind config shapes, CSS variable conventions, Material/Chakra/Mantine theme files) lives in the inventory skill's prompt. Whether this knowledge stays inline in the skill or graduates to a separate "framework profile" config file is a question for the second or third real use case, not now.

4. **Screenshot pruning.** Once superseded inventories accumulate, their screenshots stay in the repo. A future enhancement could prune screenshots referenced only by superseded inventories. Not blocking; revisit when repo size becomes a concern.

5. **Mobile vs desktop viewports.** The current design assumes a single viewport per inventory. Real-world prototypes often have distinct mobile/tablet/desktop layouts that should each be inventoried. The simplest path is to treat each viewport as a separate `source` (e.g. `prototype-desktop`, `prototype-mobile`) and run the diff per pair. A more sophisticated approach (multi-viewport entries within a single inventory) can be considered if the pattern recurs.

6. **Future research/ subcategory restructure.** The user raised the idea of nesting `meta/research/` into subdirectories (`research/codebase/`, `research/ideas/`, etc.). Worth its own ADR. The introduction of `design-inventories` could be the forcing function — but as a separate, clearly-scoped change.

## References

- [Connect Claude Code to tools via MCP](https://code.claude.com/docs/en/mcp)
- [Claude Code issue #16143 — Inline `mcpServers` in plugin.json ignored](https://github.com/anthropics/claude-code/issues/16143)
- [Claude Code issue #15308 — `--plugin-dir` does not load MCP servers defined in plugin.json](https://github.com/anthropics/claude-code/issues/15308)
