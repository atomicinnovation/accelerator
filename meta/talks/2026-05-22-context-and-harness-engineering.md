---
date: "2026-05-22T00:00:00+00:00"
type: talk
status: draft
author: Toby Clemson
venue: TBD
duration_minutes: 15
demo_minutes: 7
tags: [talk, context-engineering, harness-engineering, accelerator-plugin]
---

# Context & Harness Engineering — The Accelerator Claude Code plugin

**Audience**: Engineers / hands-on devs
**Duration**: 15 minutes (~7 min talk, ~7 min demo)
**Takeaways (weighted)**:
1. The filesystem is the message bus *(heaviest)*
2. Context is the bottleneck, not the model
3. AI-assisted dev is now an engineering discipline

---

## Slide 1 — Title (~10s)

**Context & Harness Engineering**
*The Accelerator Claude Code plugin*
*Toby Clemson, Atomic Innovation*

> **Speaker notes**: 15 minutes, half slides, half demo. Let's get into it.

---

## Slide 2 — Our journey through AI-assisted dev (~1 min)

```
2022                2023-24             June 2025          Oct 2025            Today
We adopted     →    Cursor /       →    Claude Code   →   Read AI That   →   Accelerator
Copilot             Windsurf            "the eureka"      Works podcast      plugin
"autocomplete"      "indexed chat"      "real collaborator"  "doing the easy
                                                              20%"
```

> **Speaker notes**: We started with Copilot — saved keystrokes, didn't change how we worked. Cursor/Windsurf indexed the repo, gave us chat over it — better, but the model still didn't really *know* the code. Last June, Claude Code clicked: ran tests, navigated the repo, planned multi-step work. That was the eureka. We rode it for four months. Then in October, we read the notes from [an AI That Works podcast episode by the humanlayer folks](https://github.com/ai-that-works/ai-that-works/tree/main/2025-08-05-advanced-context-engineering-for-coding-agents) and realised we'd been doing the easy 20%. The model was already good. The leverage was somewhere else entirely. That's where this plugin started.

---

## Slide 3 — The realisation: context is the bottleneck (~1 min)

```
Prompts       <    Tools    <    Model    <    Context
```
*via humanlayer / AI That Works, 5 Aug 2025*

- Context windows are bounded; quality drops above ~120k tokens
- Frontier models follow ~150 instructions reliably; the system prompt eats ~50

> **Speaker notes**: This is the slide I want people to remember. Prompt-craft is mostly noise. Picking Opus over Sonnet helps a bit. The biggest gain — by far — is being deliberate about what's in the context window. ~120k usable tokens before quality degrades, ~100 instruction slots after the system prompt. Not much. Everything that follows is about spending those slots well.

---

## Slide 4 — Principle 1: the filesystem is the message bus *(headline)* (~1.5 min)

```
┌──────────┐    meta/research/    ┌──────────┐    meta/plans/    ┌─────────────┐
│ Research │ ───────────────────► │   Plan   │ ────────────────► │  Implement  │
└──────────┘                      └──────────┘                   └─────────────┘
     │                                 │                                │
     ▼                                 ▼                                ▼
                          meta/decisions/, meta/reviews/, ...
                          (institutional memory, version-controlled)
```

- Every phase starts with a **fresh context** — nothing carried in conversation
- Every skill writes a **structured artifact** to `meta/` (markdown + YAML frontmatter)
- The next phase reads from disk, not chat

> **Speaker notes**: Headline of the talk. Most AI workflows put state in the conversation — the moment you `/clear` or hit token limits, it's gone, and your teammate can't see it either. We invert this completely: the conversation is **disposable**, the artifacts are **permanent**. ADR-0001 establishes context isolation; ADR-0027 says every structured skill output persists to `meta/`. Artifacts are version-controlled — PR review, history, onboarding all come for free. And your future self and teammates inherit the institutional memory.

---

## Slide 5 — Principle 2: bounded agents do bounded work (~45s)

- **Locators** — Grep/Glob/LS, no Read. Find *where* things are. Can't overflow.
- **Analysers** — add Read. Examine a *focused set* of files the locator returned.
- Soft ceiling: ~120k tokens of active context, ~150 CLAUDE.md instructions

> **Speaker notes**: Corollary of the message-bus principle. One agent doing broad search *and* deep file reads runs out of context before it finishes either. So we split them: index first, then fetch. Same pattern as every search system in the last 30 years. Each agent type stays cheap and predictable, and you can compose them.

---

## Slide 6 — Principle 3: structured artifacts are a contract (~45s)

Every meta/ artifact carries the same base frontmatter:

```yaml
---
date: 2026-05-22T12:00:00+00:00
type: plan
skill: create-plan
status: draft
---
```

- Machine-parseable → skills discover and consume each other's output
- Lifecycle transitions are automated (e.g. `validate-plan` flips `status: complete`)
- Headed for **schema validation** (ADR-0033) and **typed linkage** (ADR-0034)

> **Speaker notes**: ADR-0028. Common base schema across every artifact type — plans, ADRs, reviews, research, validations. Once that's in place the meta/ tree stops being a folder of files and becomes a queryable graph. ADRs 0033 and 0034 — in flight right now — tighten this with schema-validated frontmatter and a typed vocabulary for *how* artifacts link to each other. That's what powers the visualiser you'll see in a minute.

---

## Slide 7 — Principle 4: review by perspective, not checklist (~45s)

**Perspective-Based Reading** (Basili & Shull, 1996) — different stakeholders find different defects.

The review system runs 13+ specialist lenses in parallel, each:

- Adopts a focused stakeholder perspective (security, performance, safety, ...)
- Produces a *derivative artifact* — attack scenarios, failure modes, bottlenecks
- Has explicit boundaries (3-4 responsibility groups, "What NOT to Do" section)

> **Speaker notes**: ADR-0003. The naive way to use an LLM as a reviewer is hand it a checklist — turns out PBR research from the 90s shows that's actually the *worst* approach. Checklists encourage passive scanning. Each lens adopts a perspective and *generates* something specific — attacks, failure modes, bottleneck analyses. Explicit boundaries so parallel lenses don't step on each other. 30-year-old software inspection theory, applied to AI.

---

## Slide 8 — Demo (~7 min)

### (a) Tour of `meta/` (~2 min)

- `meta/decisions/` — the ADRs we've been talking about; open ADR-0001 to show the frontmatter
- `meta/plans/` — concrete implementation plans, `status: draft` vs `complete`
- `meta/reviews/plans/` and `meta/reviews/prs/` — persisted multi-lens reviews
- `meta/research/codebase/` — codebase investigations

### (b) Skills overview (~1 min)

*The core loop:*

- `/accelerator:research-codebase` — investigates, writes `meta/research/codebase/`
- `/accelerator:create-plan` — writes `meta/plans/`, links to research
- `/accelerator:review-plan` — multi-lens review, writes `meta/reviews/plans/`
- `/accelerator:implement-plan` — executes the plan, updates lifecycle

*Supporting:*

- `/accelerator:create-work-item`, `/accelerator:create-adr`

### (c) Visualiser (~4 min)

- Launch it; show the meta/ artifacts rendered as a navigable graph
- Walk through typed links: plan → cites → ADRs; review → reviews → plan
- Land it: *"this graph exists in the filesystem regardless — the visualiser just renders it"*

> **Speaker notes**: Move briskly through (a) and (b) — they're orientation. The visualiser is the payoff. Spend most time there, because it makes the meta/ graph tangible in a way the slides can't.

---

## Slide 9 — Credits & resources (~15s)

- **[AI That Works — Advanced Context Engineering (Aug 2025)](https://github.com/ai-that-works/ai-that-works/tree/main/2025-08-05-advanced-context-engineering-for-coding-agents)** — the framing
- **[humanlayer](https://github.com/humanlayer)** — the commands this plugin started from
- Plugin repo: `accelerator`

> **Speaker notes**: One thing to take away: the leverage hierarchy. Two things: the conversation is disposable, the artifacts are permanent.

---

## Slide 10 (Appendix, after credits) — Loops as a harness primitive

**The same journey, one step further:**

```
2022     2023-24    Jun 2025    Oct 2025      ...next?
Copilot  Cursor /   Claude      AI That       Ralph loops
         Windsurf   Code        Works         + cursed
```

**The Ralph loop**

```bash
while :; do cat PROMPT.md | claude-code ; done
```

> *"Deterministically bad in an undeterministic world."* — Geoffrey Huntley

- **One task per loop.** Generate → backpressure (tests, types, compile) → observe → tune the prompt → exit → rerun
- **Don't convince the model to work longer — bound the work instead**

**`cursed`** — what falls out the other end

- A **Gen Z-keyword programming language** built by Claude in a Ralph loop (~3 months)
- **Three editions, built from scratch**: C → Rust → Zig
- Compiles to native binaries via LLVM; the language didn't exist in training data
- Total cost: **~$14k USD in tokens** (Huntley, on Twitter)

**Why it's here**

Same principle as everything else in this talk, taken to its limit:

- The conversation is disposable
- The artifacts — specs, tests, `PROMPT.md` — are permanent
- Ralph just makes the *harness* the whole program

**Credits**

- [ghuntley.com/cursed](https://ghuntley.com/cursed/) · [github.com/ghuntley/cursed](https://github.com/ghuntley/cursed)
- [ghuntley.com/ralph](https://ghuntley.com/ralph/) · [ghuntley.com/loop](https://ghuntley.com/loop/)
- [AI That Works — Ralph Wiggum (28 Oct 2025)](https://github.com/ai-that-works/ai-that-works/tree/main/2025-10-28-ralph-wiggum-coding-agent-power-tools)

> **Speaker notes**: Appendix material — only show if Q&A goes there. The setup: same timeline as slide 2, with one extra waypoint. The point isn't to advocate Ralph for production — it's to show the principle of harness-over-prompt taken to its logical conclusion. Geoffrey Huntley ran Claude in a `while true; cat PROMPT.md | claude-code; done` loop for three months and built a programming language with Gen Z keywords (`slay` for function, `sus` for variable, pointers use the Among Us character `ඞ`). The language didn't exist in training data. He's now rewritten it from scratch three times — C, then Rust, now Zig — total cost ~$14k in tokens. Same idea as our plugin — conversation is disposable, artifacts are permanent — just dialled to 11.

---

## Source ADRs

- [`ADR-0001`](../decisions/ADR-0001-context-isolation-principles.md) — context isolation, filesystem comms, locator/analyser
- [`ADR-0003`](../decisions/ADR-0003-pbr-lens-design-with-structural-invariants.md) — Perspective-Based Reading lens design
- [`ADR-0027`](../decisions/ADR-0027-persist-structured-skill-outputs-to-meta.md) — persist structured skill outputs to meta/
- [`ADR-0028`](../decisions/ADR-0028-common-frontmatter-schema-for-meta-artifacts.md) — common frontmatter schema
- [`ADR-0033`](../decisions/ADR-0033-unified-base-frontmatter-schema.md) — unified base frontmatter (in flight)
- [`ADR-0034`](../decisions/ADR-0034-typed-linkage-vocabulary.md) — typed linkage vocabulary (in flight)
