---
type: work-item
id: "0113"
title: "Topic Research Skillset"
date: "2026-06-19T01:28:08+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: epic
priority: medium
relates_to: ["work-item:0056"]
tags: [research, skills, deep-research, visualiser, infrastructure]
last_updated: "2026-06-19T01:28:08+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-22
---

# 0113: Topic Research Skillset

**Kind**: Epic
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an Accelerator user, I want a skillset that performs broad-and-deep research on arbitrary external topics — concepts, domains, technical subjects, expert knowledge — so that I can quickly build a thorough, citation-backed knowledgebase on a subject and iteratively deepen or widen it.

This epic delivers a `research-topic` skill (subcommands `brief` / `conduct` / `expand`) implementing an iterative, parallel, multi-agent deep-research loop: author a brief, research a round of topics in parallel, then steer the next round. Findings accrete as immutable per-topic documents plus a wholesale-rewritten synthesis under `meta/research/topics/<slug>/`. Critically, it is built on **reusable infrastructure** — a generic `researcher` agent specialised at spawn time via injectable source/focus profiles (the same pattern as the `reviewer` agent), plus templates and a stable artifact contract — so later, more specific research skills (competitor research, technical-topic research, an ideation skillset) can be layered on top without rebuilding the engine. New artifacts are surfaced in the visualiser library.

## Context

The design is grounded in a survey of existing deep-research systems:

- **Sagan plugin** (`robertbagge/claude-sagan-plugin`) — the closest model: an interactive brief skill + a deep-research skill run one round per invocation, fanning out one parallel agent per topic, with immutable per-topic files, an append-only round log in the brief, and a synthesis rewritten each round. Re-invocation detects the first round with missing files and fills only the gaps. An "anti-changelog" output rule keeps the accreted corpus reading as a single dossier.
- **Anthropic's multi-agent research system** — the orchestrator/sub-agent architecture, the effort-scaling rubric (1 agent for simple, 2–4 for comparisons, 10+ for complex), the four-part sub-agent brief (objective / output format / tools & sources / boundaries) as the main defence against duplicated work, "start wide then narrow", and citation as a dedicated final pass. Caveat: multi-agent runs cost ~15× the tokens of a chat, so depth/breadth must be controllable.
- **dzhng/deep-research** — the cleanest depth × breadth parameterisation (breadth = queries per level; depth = recursion levels; breadth halves each level to narrow the tree).
- **superpowers `/brainstorming`** — a strong convergence/approval-gate pattern, but thin on divergence; informs a later (out-of-scope) ideation skillset rather than this epic.
- Plus academic toolchains (`imbad0202`, `lingzhi227`) for source-verification and citation-tier ideas, and the `DavidZWZ/Awesome-Deep-Research` landscape.

This work conforms to existing Accelerator conventions: artifacts under `meta/research/` with YAML frontmatter and a `type` discriminator; 3-tier template override; per-skill `instructions.md`/`context.md`; agent overrides via config; and the subcommand-dispatch pattern already established by `skills/config/configure/SKILL.md`. The grouping directory is `meta/research/topics/` (sibling to `codebase/`, `issues/`, `design-inventories/`, `design-gaps/`), chosen to read as a subject-typed peer of the existing research categories.

The area is greenfield — no overlapping work items exist; the nearest is `0056` (restructure `meta/research/codebase/` into subject subcategories), which explicitly lists external research as a non-goal.

## Requirements

### High-level goals and themes

1. **Reusable research infrastructure** — a generic `researcher` agent specialised at spawn time via injectable source/focus profiles, research templates, and a stable brief/topic/synthesis artifact contract. This is the foundation that future, more specific research skills consume; `research-topic` is its first consumer.
2. **The `research-topic` skill** — `brief` / `conduct` / `expand` subcommands implementing the iterative deep-research loop, following the `configure` dispatch pattern.
3. **Iterative knowledgebase accretion** — immutable per-topic files, an append-only round log, a wholesale-rewritten synthesis, and gap-detection so re-invocation only researches missing topics.
4. **Multi-source research** — web and academic (and other sensible external) sources, realised purely through injectable profiles with credibility tiers; external sources only (no codebase/internal).
5. **Configuration & override surface** — depth/breadth tunable at project/user config and per-invocation, plus template override and per-skill context.
6. **Visualiser integration** — new research document type(s) registered and rendered in the library so briefs, topic documents, and syntheses are browsable.

### Initial stories (vertical slices)

Each slice is independently shippable, demoable, and testable, cutting through skill → agent → template → artifact → visualiser. Order is negotiable; Slice 1 is foundational, Slices 2–4 are independently valuable and reorderable.

1. **Single-round web research, viewable in the library** *(walking skeleton)*. Commission a brief on a topic, run one round of web research, get immutable topic files plus a synthesis under `meta/research/topics/<slug>/`, and browse them in the visualiser. Establishes: the `paths.research_topics` key and config default (+ `docs.rs` config_path_key); brief/topic/synthesis templates (web-only); the generic `researcher` agent + web source profile; the `research-topic` skill with `brief` + `conduct` (single round) on the `configure` dispatch pattern; sensible hardcoded depth/breadth defaults; visualiser doc-type registration (Rust enum + TS union + glyph + colour tokens + framed background + Discover-phase placement + VR baselines); tests green under `mise run check`.
2. **Iterative accretion + breadth steering**. Grow the knowledgebase across multiple rounds without clobbering, and steer topics between rounds. `conduct` gains gap-detection re-invocation (immutable topic files, append-only round log, synthesis rewritten wholesale, anti-changelog discipline) and proposes + appends the next round; the `expand` subcommand adds new topics; any visualiser grouping needed to show round/topic structure; tests.
3. **Source rigour: academic sources + tiered citations**. Research drawing on academic sources with credible, tiered citations. An academic source profile; the brief declares a source preference the generic `researcher` honours; credibility-tier tagging and citation handling; template + visualiser rendering for citations/source tiers; tests.
4. **Tunable depth & breadth**. Control breadth/depth via config (project + user) and per-invocation overrides. Config-schema extension for numeric `depth` / `breadth` (a new tunable precedent), config-read plumbing, per-invocation override flags wired into `conduct` / `expand`, `configure help` docs; tests.

## Acceptance Criteria

- [ ] Given a topic, `/research-topic brief` produces `meta/research/topics/<slug>/brief.md` with the defined frontmatter and sections, interactively scoped.
- [ ] Given a brief path, `/research-topic conduct <path>` runs a round, spawns parallel `researcher` agents (count scaled by breadth), writes one immutable topic file per topic plus `synthesis.md`, and appends a proposed next round to the brief.
- [ ] Re-invoking `conduct` researches only missing topics, never rewrites existing topic files, and rewrites `synthesis.md` wholesale (idempotent accretion); the corpus reads as a single dossier (no round-by-round narration).
- [ ] `/research-topic expand <path>` appends new topics without clobbering prior brief content.
- [ ] The generic `researcher` agent is specialised for web vs academic purely by injected profile — no per-source agent required.
- [ ] Depth and breadth resolve from config and are overridable per-invocation; defaults are documented.
- [ ] Templates resolve through the 3-tier override; users can override templates and add per-skill `instructions.md`/`context.md`.
- [ ] The `research-topic` skill dispatches `brief`/`conduct`/`expand` following the `configure` pattern with a clear `argument-hint`.
- [ ] Each new research document type appears in the visualiser library under the Discover phase with a glyph and framed background, with passing visual-regression baselines on both darwin and linux.
- [ ] New path key and templates are registered; all checks pass under `mise run check`.

## Open Questions

- Visualiser type granularity: three distinct doc types (brief / topic / synthesis) or one umbrella "Topic research" type? Drives glyph and VR effort.
- Indexer model: how to list/group the nested per-topic directories that mix types in one tree, given the indexer currently assumes one type = one root directory?
- Academic providers: Semantic Scholar / arXiv / OpenAlex / Crossref — do any require API keys or new tooling, and does that expand scope?
- Depth/breadth semantics: dzhng's narrowing tree (breadth halves per level) or flat per-round counts? And what is the per-invocation override syntax (subcommand flags vs args)?
- Citations: a separate final pass (Anthropic) vs inline tiered tagging (Sagan), or both?

## Dependencies

- Blocked by: none.
- Blocks: none.
- Relates to: `0056` (restructure `meta/research/` into subject subcategories) — both touch the `meta/research/` structure and naming.
- Future consumers (out of scope here): competitor-research and technical-topic-research skills, and an ideation/brainstorming skillset, all of which will build on this epic's infrastructure.

## Assumptions

- This epic ships the reusable infrastructure **and** the `research-topic` skill; the ideation skillset and specific consumer skills are deliberately later.
- Sources are external only (web + academic + other sensible external sources); codebase and internal `meta/` docs are out of scope.
- Artifacts use YAML frontmatter (Accelerator convention), not Sagan's bold-markdown headers.
- Subcommand dispatch reuses the existing `configure` precedent — no new pattern.
- Visualiser integration is in-scope and delivered slice-by-slice as each artifact type appears.
- Priority is Medium (no deadline or urgency was specified).

## Technical Notes

- **Subcommand dispatch**: mirror `skills/config/configure/SKILL.md` — inline H3 sections per subcommand in one SKILL.md, with subcommands listed in `argument-hint`. Subcommand bodies may grow large; consider per-mode reference docs only if size warrants.
- **Generic researcher + profiles**: mirror the `reviewer` agent — a single generic agent spawned with a source/focus profile, the per-topic brief, and an output-format contract injected at spawn time. Profiles are the research equivalent of review "lenses" and are the reusable, pluggable seam.
- **Visualiser doc-type checklist** (per new type): `DocTypeKey` variant + `all()` + `config_path_key()` + `label()` + `wire_str()`/`from_wire_str()` and exhaustiveness tests in `server/src/docs.rs`; phase membership in `server/src/api/library.rs` (`PHASES`, under "discover"); TS `DocTypeKey` union + `DOC_TYPE_KEYS` + labels in `frontend/src/api/types.ts`; glyph icon component + `ICON_COMPONENTS` in `frontend/src/components/Glyph/`; colour tokens (light/dark) + framed-background CSS rule; VR baselines (~8 PNGs/type across sizes × themes — mind the linux-baseline drift). No per-type detail renderer is needed; all docs share `LibraryDocView`.
- **Indexer risk**: the file driver lists documents by mapping a type to a single root directory; the per-topic nested layout (brief + synthesis + topic files in one tree) does not fit that assumption and needs a design decision in planning.
- **Config tunables precedent**: the config system has no numeric tunables today (only paths, templates, agents, work integration); Slice 4 establishes that precedent in `scripts/config-defaults.sh` and the config-read plumbing.
- **Cost**: multi-agent rounds are token-heavy (~15× a chat per Anthropic); configurable depth/breadth and the effort-scaling rubric in the orchestrator prompt mitigate over-investment.

## Drafting Notes

- "Infrastructure extraction" interpreted as first-class deliverables *within* this epic (generic researcher + injectable profiles + templates + artifact contract), surfaced through Slice 1 rather than as a standalone horizontal story; reusability is proven later by the out-of-scope consumer skills.
- Re-sliced from layer-wise stories into vertical, INVEST-aligned slices at the user's request; each slice cuts through all layers including the visualiser.
- Terminology `topics` and the `research-topic` subcommand skill chosen by the user; the brief is treated as the `brief` subcommand rather than a separate skill.
- Source model follows the user's steer to reuse the `reviewer` injection pattern.
- Visualiser scope added at the user's request; the indexer tension and VR-baseline cost are flagged as planning risks.
- Priority set to Medium by default — adjust if this is a higher-urgency initiative.

## References

- Source survey (web research conducted during drafting):
  - Sagan plugin — `https://github.com/robertbagge/claude-sagan-plugin`
  - Anthropic multi-agent research system — `https://www.anthropic.com/engineering/multi-agent-research-system`
  - dzhng/deep-research — `https://github.com/dzhng/deep-research`
  - superpowers `/brainstorming` — `https://github.com/obra/superpowers`
  - `https://github.com/199-biotechnologies/claude-deep-research-skill`
  - `https://github.com/Weizhena/Deep-Research-skills`
  - `https://github.com/lingzhi227/agent-research-skills`
  - `https://github.com/imbad0202/academic-research-skills`
  - `https://github.com/DavidZWZ/Awesome-Deep-Research`
- Related: `0056` (restructure `meta/research/` into subject subcategories)
- Internal patterns: `skills/config/configure/SKILL.md` (subcommand dispatch), `agents/reviewer.md` (spawn-time specialisation), `server/src/docs.rs` + `frontend/src/api/types.ts` (visualiser doc-type registries)
